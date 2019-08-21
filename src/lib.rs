extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashSet;
use syn;

#[proc_macro_derive(AnyWireMessage)]
pub fn any_wire_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_any_wire_message(&ast)
}

#[proc_macro_derive(WireMessage, attributes(msg_type, tlv_type))]
pub fn wire_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_wire_message(&ast)
}

fn impl_any_wire_message(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Enum(ref s) => impl_any_wire_message_enum(&ast.ident, s),
        _ => panic!("only derivable for enums"),
    }
}

fn impl_any_wire_message_enum(name: &syn::Ident, enum_data: &syn::DataEnum) -> TokenStream {
    let (variant_name, variant_type): (Vec<_>, Vec<_>) = enum_data
        .variants
        .iter()
        .map(|v| {
            (
                &v.ident,
                match &v.fields {
                    syn::Fields::Unnamed(f) => {
                        &f.unnamed
                            .first()
                            .expect("all variants must contain a value")
                            .ty
                    }
                    _ => panic!("all variants must be tuples"),
                },
            )
        })
        .unzip();
    let gen = quote! {
        impl<'a> AnyWireMessage<'a> for #name {
            fn msg_type(&self) -> u16 {
                match self {
                    #(
                        #variant_name(a) => #variant_type::MSG_TYPE,
                    )*
                }
            }

            fn write_to<W: std::io::Write>(&'a self, w: &mut W) -> std::io::Result<usize> {
                match self {
                    #(
                        #variant_name(a) => a.write_to(w),
                    )*
                }
            }
        }
    };
    gen.into()
}

fn impl_wire_message(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Struct(ref s) => impl_wire_message_struct(&ast.ident, &ast.attrs, s),
        _ => unimplemented!(),
    }
}

fn impl_wire_message_struct(
    name: &syn::Ident,
    attrs: &Vec<syn::Attribute>,
    struct_data: &syn::DataStruct,
) -> TokenStream {
    let num = attrs
        .iter()
        .filter_map(|a| match a.parse_meta() {
            Ok(m) => match m {
                syn::Meta::NameValue(nv) => {
                    if nv.path.is_ident("msg_type") {
                        Some(nv.lit)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Err(_) => None,
        })
        .next()
        .expect("missing attribute \"msg_type\"\n\nhelp: add #[msg_type = ...]");
    let iter = syn::Ident::new(&format!("{}Iter", name), Span::call_site());
    let item = syn::Ident::new(&format!("{}Items", name), Span::call_site());
    let counter = std::iter::successors(Some(0), |a| Some(a + 1))
        .map(|i| proc_macro2::Literal::usize_suffixed(i));
    let mut tlv = None;
    let field_mapper =
        |(i, f): (usize, &syn::Field)| -> ((syn::Member, syn::Type), Option<syn::Lit>) {
            let mut new_tlv = None;
            let mut res = (
                (
                    f.ident
                        .as_ref()
                        .map(|id| syn::Member::Named(id.clone()))
                        .unwrap_or_else(|| {
                            syn::Member::Unnamed(syn::Index {
                                index: i as u32,
                                span: Span::call_site(),
                            })
                        }),
                    f.ty.clone(),
                ),
                f.attrs
                    .iter()
                    .filter_map(|a| match a.parse_meta() {
                        Ok(m) => match m {
                            syn::Meta::NameValue(nv) => {
                                if nv.path.is_ident("tlv_type") {
                                    if let syn::Lit::Int(ref lit) = nv.lit {
                                        new_tlv = Some(
                                            lit.base10_parse::<u64>()
                                                .expect("tlv_type must be a u64"),
                                        );
                                        Some(nv.lit)
                                    } else {
                                        panic!("tlv_type must be a u64")
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        },
                        Err(_) => None,
                    })
                    .next(),
            );
            match (tlv, new_tlv) {
                (Some(_), None) => panic!("tlv stream must occur after expected fields"),
                (Some(old), Some(new)) if old > new => {
                    panic!("tlv stream must be monotonically increasing by type")
                }
                (_, Some(_)) => match &f.ty {
                    syn::Type::Path(ref p)
                        if p.path.segments.last().expect("missing type").ident == "Option" =>
                    {
                        (res.0).1 = match &p.path.segments.last().unwrap().arguments {
                            syn::PathArguments::AngleBracketed(a) => {
                                (match a.args.first().expect("tlv value must be Option") {
                                    syn::GenericArgument::Type(t) => t.clone(),
                                    _ => panic!("tlv value must be Option"),
                                })
                            }
                            _ => panic!("tlv value must be Option"),
                        };
                    }
                    _ => panic!("tlv value must be Option"),
                },
                _ => (),
            };
            tlv = new_tlv;
            res
        };
    let punc = syn::punctuated::Punctuated::<syn::Field, ()>::new();
    let (field_tup, tlv_type): (Vec<(syn::Member, syn::Type)>, Vec<Option<syn::Lit>>) =
        match &struct_data.fields {
            syn::Fields::Named(n) => n.named.iter(),
            syn::Fields::Unnamed(n) => n.unnamed.iter(),
            syn::Fields::Unit => punc.iter(),
        }
        .enumerate()
        .map(field_mapper)
        .unzip();
    let (field, field_ty_set): (Vec<syn::Member>, HashSet<syn::Type>) =
        field_tup.into_iter().unzip();
    let field_ty = field_ty_set.iter();
    let field_ty2 = field_ty_set.iter();
    let field_ty_name =
        (0..(field_ty_set.len())).map(|i| syn::Ident::new(&format!("T{}", i), Span::call_site()));
    let field_ty_name2 = field_ty_name.clone();
    let field_ty_name3 = field_ty_name.clone();
    let gen = if field.is_empty() {
        quote! {
            impl<'a> IntoIterator for &'a #name {
                type Item = !;
                type IntoIter = std::iter::Empty;
                fn into_iter(self) -> std::iter::Empty<!> {
                    std::iter::empty()
                }
            }
            impl WireMessage<'_> for #name {
                fn msg_type(&self) -> u16 {
                    #num
                }
            }
        }
    } else {
        quote! {
            pub enum #item<'a> {
                #(
                    #field_ty_name(&'a #field_ty),
                )*
            }
            #(
                impl<'a> From<&'a #field_ty2> for #item<'a> {
                    fn from(t: &'a #field_ty2) -> Self {
                        #item::#field_ty_name2(t)
                    }
                }
            )*
            impl<'a> WireItem for #item<'a> {
                fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<usize> {
                    use #item::*;
                    match self {
                        #(
                            #field_ty_name3(a) => a.encode(w),
                        )*
                    }
                }
            }

            pub struct #iter<'a> {
                idx: usize,
                parent: &'a #name,
            }
            impl<'a> Iterator for #iter<'a> {
                type Item = EncodedItem<#item<'a>>;

                fn next(&mut self) -> Option<Self::Item> {
                    let n = self.idx;
                    self.idx += 1;
                    match n {
                        #(
                            #counter => Some(EncodedItem::from((&self.parent.#field, #tlv_type))),
                        )*
                        _ => None
                    }
                }
            }

            impl<'a> IntoIterator for &'a #name {
                type Item = <#iter<'a> as Iterator>::Item;
                type IntoIter = #iter<'a>;
                fn into_iter(self) -> #iter<'a> {
                    #iter {
                        idx: 0,
                        parent: self,
                    }
                }
            }
            impl<'a> WireMessage<'a> for #name {
                type Item = #item<'a>;

                const MSG_TYPE: u16 = #num;
            }
        }
    };
    gen.into()
}
