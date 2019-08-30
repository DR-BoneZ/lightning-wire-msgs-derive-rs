use crate::proc_macro::TokenStream;
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
    let discriminant = data.variants.iter().map(|var| {
        &var.discriminant
            .as_ref()
            .expect("only derivable for enums with discriminants")
            .1
    });
    let variant = data.variants.iter().map(|var| &var.ident);

    let gen = quote! {
        impl std::convert::TryFrom<#repr> for #name {
            type Error = ();

            fn try_from(prim: #repr) -> Result<Self, ()> {
                match prim {
                    #(
                        #discriminant => Ok(#name::#variant),
                    )*
                    _ => Err(())
                }
            }
        }
    };
    gen.into()
}
