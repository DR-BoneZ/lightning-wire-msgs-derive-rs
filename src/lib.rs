extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn;

#[proc_macro_derive(WireMessage, attributes(msg_type, tlv_type))]
pub fn wire_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_wire_message(&ast)
}

fn impl_wire_message(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let num = ast
        .attrs
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
    let counter = std::iter::successors(Some(0), |a| Some(a + 1))
        .map(|i| proc_macro2::Literal::usize_suffixed(i));
    let mut tlv = None;
    let field_mapper = |(i, f): (usize, &syn::Field)| -> (syn::Member, Option<syn::Lit>) {
        let mut new_tlv = None;
        let res = (
            f.ident
                .as_ref()
                .map(|id| syn::Member::Named(id.clone()))
                .unwrap_or_else(|| {
                    syn::Member::Unnamed(syn::Index {
                        index: i as u32,
                        span: Span::call_site(),
                    })
                }),
            f.attrs
                .iter()
                .filter_map(|a| match a.parse_meta() {
                    Ok(m) => match m {
                        syn::Meta::NameValue(nv) => {
                            if nv.path.is_ident("tlv_type") {
                                if let syn::Lit::Int(ref lit) = nv.lit {
                                    new_tlv = Some(
                                        lit.base10_parse::<u64>().expect("tlv_type must be a u64"),
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
            _ => (),
        };
        tlv = new_tlv;
        res
    };
    let punc = syn::punctuated::Punctuated::<syn::Field, ()>::new();
    let (field, tlv_type_vec): (Vec<syn::Member>, Vec<Option<syn::Lit>>) = match &ast.data {
        syn::Data::Struct(d) => match &d.fields {
            syn::Fields::Named(n) => n.named.iter(),
            syn::Fields::Unnamed(n) => n.unnamed.iter(),
            syn::Fields::Unit => punc.iter(),
        },
        _ => unimplemented!(),
    }
    .enumerate()
    .map(field_mapper)
    .unzip();
    let tlv_type = tlv_type_vec.into_iter().map(|opt| match opt {
        Some(t) => syn::Expr::Call(syn::ExprCall {
            attrs: Vec::new(),
            func: Box::new(
                syn::ExprPath {
                    attrs: Vec::new(),
                    qself: None,
                    path: syn::Path {
                        leading_colon: None,
                        segments: {
                            let mut seq = syn::punctuated::Punctuated::new();
                            seq.push(syn::PathSegment {
                                ident: syn::Ident::new("Some", Span::call_site()),
                                arguments: syn::PathArguments::None,
                            });
                            seq
                        },
                    },
                }
                .into(),
            ),
            paren_token: syn::token::Paren {
                span: Span::call_site(),
            },
            args: {
                let mut seq = syn::punctuated::Punctuated::new();
                seq.push(syn::Expr::Lit(syn::ExprLit {
                    attrs: Vec::new(),
                    lit: t,
                }));
                seq
            },
        }),
        None => syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: syn::Path {
                leading_colon: None,
                segments: {
                    let mut seq = syn::punctuated::Punctuated::new();
                    seq.push(syn::PathSegment {
                        ident: syn::Ident::new("None", Span::call_site()),
                        arguments: syn::PathArguments::None,
                    });
                    seq
                },
            },
        }
        .into(),
    });
    let gen = quote! {
        pub struct #iter<'a> {
            idx: usize,
            parent: &'a #name,
        }
        impl<'a> Iterator for #iter<'a> {
            type Item = (&'a dyn WireItemBoxedWriter, Option<u64>);

            fn next(&mut self) -> Option<Self::Item> {
                let n = self.idx;
                self.idx += 1;
                match n {
                    #(
                        #counter => Some((&self.parent.#field, #tlv_type)),
                    )*
                    _ => None
                }
            }
        }

        impl<'a> IntoIterator for &'a #name {
            type Item = (&'a dyn WireItemBoxedWriter, Option<u64>);
            type IntoIter = #iter<'a>;
            fn into_iter(self) -> #iter<'a> {
                #iter {
                    idx: 0,
                    parent: self,
                }
            }
        }
        impl WireMessage<'_> for #name {
            fn msg_type(&self) -> u16 {
                #num
            }
        }
    };
    gen.into()
}
