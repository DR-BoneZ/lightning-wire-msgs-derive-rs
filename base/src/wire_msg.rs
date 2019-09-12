use proc_macro2::Span;
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
        Struct(ref s) => impl_trait_struct(&ast.ident, &ast.attrs, s, &ast.generics, Subset::Both),
        _ => unimplemented!(),
    }
}

pub fn impl_writer(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Struct(ref s) => {
            impl_trait_struct(&ast.ident, &ast.attrs, s, &ast.generics, Subset::Writer)
        }
        _ => unimplemented!(),
    }
}

pub fn impl_reader(ast: &syn::DeriveInput) -> TokenStream {
    use syn::Data::*;
    match &ast.data {
        Struct(ref s) => {
            impl_trait_struct(&ast.ident, &ast.attrs, s, &ast.generics, Subset::Reader)
        }
        _ => unimplemented!(),
    }
}

fn def_decode(
    name: &syn::Ident,
    field: &[syn::Member],
    tlv_type: &[Option<syn::Lit>],
) -> proc_macro2::TokenStream {
    let wire_item_read_expr = tlv_type.iter().map(|t| {
        if let Some(t) = t {
            quote! {
                lightning_wire_msgs::TLVWireItemReader::decode_tlv(&mut peek_reader, #t)?
            }
        } else {
            quote! {
                lightning_wire_msgs::WireItemReader::decode(&mut peek_reader)?
            }
        }
    });

    quote! {
        fn decode<R: std::io::Read>(reader: &mut R, check_type: bool) -> std::io::Result<Self> {
            if check_type {
                let mut msg_type = [0_u8; 2];
                reader.read_exact(&mut msg_type)?;
                let msg_type = u16::from_be_bytes(msg_type);
                if msg_type != Self::MSG_TYPE {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                }
            }
            let mut peek_reader = lightning_wire_msgs::PeekReader::from(reader);

            Ok(#name {
                #(
                    #field: #wire_item_read_expr,
                )*
            })
        }
    }
}

fn def_encode(field: &[syn::Member], tlv_type: &[Option<syn::Lit>]) -> proc_macro2::TokenStream {
    let wire_item_write_expr = field.iter().enumerate().map(|(i, field)| {
        if let Some(ref t) = &tlv_type[i] {
            quote! {
                if let Some(ref field) = &self.#field {
                    count += lightning_wire_msgs::TLVWireItemWriter::encode_tlv(field, w, #t)?;
                }
            }
        } else {
            quote! {
                count += lightning_wire_msgs::WireItemWriter::encode(&self.#field, w)?;
            }
        }
    });
    quote! {
        fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<usize> {
            let mut count = 0;
            count += w.write(&u16::to_be_bytes(Self::MSG_TYPE))?;
            #(
                #wire_item_write_expr
            )*
            w.flush()?;
            Ok(count)
        }
    }
}

fn impl_trait_struct(
    name: &syn::Ident,
    attrs: &Vec<syn::Attribute>,
    struct_data: &syn::DataStruct,
    generics: &syn::Generics,
    subset: Subset,
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
            (_, Some(_)) => match &f.ty {
                syn::Type::Path(ref p)
                    if p.path.segments.last().expect("missing type").ident == "Option" =>
                {
                    match &p.path.segments.last().unwrap().arguments {
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
    let (field, tlv_type): (Vec<syn::Member>, Vec<Option<syn::Lit>>) = match &struct_data.fields {
        syn::Fields::Named(n) => n.named.iter(),
        syn::Fields::Unnamed(n) => n.unnamed.iter(),
        syn::Fields::Unit => punc.iter(),
    }
    .enumerate()
    .map(field_mapper)
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
    let decode = def_decode(name, &field, &tlv_type);
    let encode = def_encode(&field, &tlv_type);
    let gen = match subset {
        Subset::Both => quote! {
            impl<#generics_params> lightning_wire_msgs::WireMessage for #name#generics_stripped #generics_where_clause {
                const MSG_TYPE: u16 = #num;
                #encode
                #decode
            }
        },
        Subset::Writer => quote! {
            impl<#generics_params> lightning_wire_msgs::WireMessageWriter for #name#generics_stripped #generics_where_clause {
                const MSG_TYPE: u16 = #num;
                #encode
            }
        },
        Subset::Reader => quote! {
            impl<#generics_params> lightning_wire_msgs::WireMessageReader for #name#generics_stripped #generics_where_clause {
                const MSG_TYPE: u16 = #num;
                #decode
            }
        },
    };
    gen.into()
}
