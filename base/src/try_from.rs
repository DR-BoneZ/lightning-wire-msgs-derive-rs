use proc_macro2::TokenStream;
use quote::quote;

pub fn impl_trait(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = if let syn::Data::Enum(ref data) = &ast.data {
        data
    } else {
        panic!("only derivable for enums with discriminants")
    };
    let repr = ast
        .attrs
        .iter()
        .filter_map(|attr| {
            attr.parse_meta().ok().map(|meta| match meta {
                syn::Meta::List(ref ml) if ml.path.is_ident("repr") => {
                    Some(ml.nested.first().expect("invalid repr").clone())
                }
                _ => None,
            })
        })
        .next()
        .expect("missing repr");
    let variant = data
        .variants
        .iter()
        .map(|var| &var.ident)
        .collect::<Vec<_>>();
    let variant_const = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, var)| syn::Ident::new(&format!("C{}", i), var.ident.span()))
        .collect::<Vec<_>>();

    let gen = quote! {
        impl std::convert::TryFrom<#repr> for #name {
            type Error = #repr;

            fn try_from(prim: #repr) -> Result<Self, Self::Error> {
                #(
                    const #variant_const: #repr = #name::#variant as #repr;
                )*
                match prim {
                    #(
                        #variant_const => Ok(#name::#variant),
                    )*
                    _ => Err(prim)
                }
            }
        }
    };
    gen.into()
}
