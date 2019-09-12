use proc_macro2::TokenStream;
use quote::quote;

enum Subset {
    Both,
    Reader,
    Writer,
}

pub fn impl_trait(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Enum(ref s) => impl_trait_enum(&ast.ident, s, &ast.generics, Subset::Both),
        _ => panic!("only derivable for enums"),
    }
}

pub fn impl_writer(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Enum(ref s) => impl_trait_enum(&ast.ident, s, &ast.generics, Subset::Writer),
        _ => panic!("only derivable for enums"),
    }
}

pub fn impl_reader(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Enum(ref s) => impl_trait_enum(&ast.ident, s, &ast.generics, Subset::Reader),
        _ => panic!("only derivable for enums"),
    }
}

fn def_encode(name: &syn::Ident, variant_name: &[&syn::Ident]) -> proc_macro2::TokenStream {
    quote! {
        fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<usize> {
            match self {
                #(
                    #name::#variant_name(a) => lightning_wire_msgs::WireMessageWriter::encode(a, w),
                )*
            }
        }
    }
}

fn def_decode(
    name: &syn::Ident,
    variant_name: &[&syn::Ident],
    variant_type: &[&syn::Type],
) -> proc_macro2::TokenStream {
    quote! {
        fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
            let mut msg_type = [0_u8; 2];
            r.read_exact(&mut msg_type)?;
            let msg_type = u16::from_be_bytes(msg_type);
            Ok(match msg_type {
                #(
                    <#variant_type as lightning_wire_msgs::WireMessageReader>::MSG_TYPE => #name::#variant_name(<#variant_type as lightning_wire_msgs::WireMessageReader>::decode(r, false)?),
                )*
                _ => return Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
            })
        }
    }
}

fn impl_trait_enum(
    name: &syn::Ident,
    enum_data: &syn::DataEnum,
    generics: &syn::Generics,
    subset: Subset,
) -> TokenStream {
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
    let type_params: Vec<syn::GenericParam> = generics
        .params
        .iter()
        .map(|gparam| match gparam {
            syn::GenericParam::Type(tp) => {
                let mut tp = tp.clone();
                tp.bounds = syn::punctuated::Punctuated::new();
                syn::GenericParam::Type(tp)
            }
            syn::GenericParam::Lifetime(ltp) => {
                let mut ltp = ltp.clone();
                ltp.bounds = syn::punctuated::Punctuated::new();
                syn::GenericParam::Lifetime(ltp)
            }
            a => a.clone(),
        })
        .collect();
    let generics_stripped = {
        let mut generics = generics.clone();
        generics.params = syn::punctuated::Punctuated::new();
        for param in type_params.iter() {
            generics.params.push(param.clone());
        }
        generics.where_clause = None;

        generics
    };
    let generics_params = &generics.params;
    let generics_where_clause = &generics.where_clause;
    let encode = def_encode(name, &variant_name);
    let decode = def_decode(name, &variant_name, &variant_type);
    let gen = match subset {
        Subset::Both => quote! {
            impl<#generics_params> lightning_wire_msgs::AnyWireMessage for #name#generics_stripped #generics_where_clause {
                fn msg_type(&self) -> u16 {
                    match self {
                        #(
                            #name::#variant_name(_) => <#variant_type as lightning_wire_msgs::WireMessage>::MSG_TYPE,
                        )*
                    }
                }
                #encode
                #decode
            }
        },
        Subset::Writer => quote! {
            impl<#generics_params> lightning_wire_msgs::AnyWireMessageWriter for #name#generics_stripped #generics_where_clause {
                fn msg_type(&self) -> u16 {
                    match self {
                        #(
                            #name::#variant_name(_) => <#variant_type as lightning_wire_msgs::WireMessageWriter>::MSG_TYPE,
                        )*
                    }
                }
                #encode
            }
        },
        Subset::Reader => quote! {
            impl<#generics_params> lightning_wire_msgs::AnyWireMessageReader for #name#generics_stripped #generics_where_clause {
                fn msg_type(&self) -> u16 {
                    match self {
                        #(
                            #name::#variant_name(_) => <#variant_type as lightning_wire_msgs::WireMessageReader>::MSG_TYPE,
                        )*
                    }
                }
                #decode
            }
        },
    };
    gen.into()
}
