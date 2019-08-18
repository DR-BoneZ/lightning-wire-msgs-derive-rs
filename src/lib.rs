extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(WireMessage)]
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
    let iter = syn::Ident::new(&format!("{}Iter", name), proc_macro2::Span::call_site());
    let counter = std::iter::successors(Some(0), |a| Some(a + 1))
        .map(|i| proc_macro2::Literal::usize_suffixed(i));
    let field_mapper = |(i, f): (usize, &syn::Field)| -> syn::Member {
        f.ident
            .as_ref()
            .map(|id| syn::Member::Named(id.clone()))
            .unwrap_or_else(|| {
                syn::Member::Unnamed(syn::Index {
                    index: i as u32,
                    span: proc_macro2::Span::call_site(),
                })
            })
    };
    let punc = syn::punctuated::Punctuated::<syn::Field, ()>::new();
    let fields = match &ast.data {
        syn::Data::Struct(d) => match &d.fields {
            syn::Fields::Named(n) => n.named.iter().enumerate().map(field_mapper),
            syn::Fields::Unnamed(n) => n.unnamed.iter().enumerate().map(field_mapper),
            syn::Fields::Unit => punc.iter().enumerate().map(field_mapper),
        },
        _ => unimplemented!(),
    };
    let gen = quote! {
        pub struct #iter<'a> {
            idx: usize,
            parent: &'a #name,
        }
        impl<'a> Iterator for #iter {
            type Item = &'a dyn WireItemBoxedWriter;

            fn next(&mut self) -> Option<Self::Item> {
                let n = self.idx;
                self.idx += 1;
                match n {
                    #(
                        #counter => Some(&self.parent.#fields),
                    )*
                    _ => None
                }
            }
        }

        impl<'a> IntoIterator for &'a #name {
            type Item = &'a dyn WireItemBoxedWriter;
            type IntoIter = #iter<'a>;
            fn into_iter(self) -> #iter<'a> {
                #iter {
                    idx: 0,
                    parent: self,
                }
            }
        }
        impl WireMessage for #name {
            fn msg_type(&self) -> u16 {
                #num
            }
        }
    };
    gen.into()
}
