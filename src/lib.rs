extern crate proc_macro;

use crate::proc_macro::TokenStream;

mod any_wire_msg;
mod try_from;
mod wire_msg;

#[proc_macro_derive(TryFromPrimitive, attributes(repr))]
pub fn try_from_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    try_from::impl_trait(&ast)
}

#[proc_macro_derive(AnyWireMessage)]
pub fn any_wire_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    any_wire_msg::impl_trait(&ast)
}

#[proc_macro_derive(AnyWireMessageWriter)]
pub fn any_wire_message_writer_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    any_wire_msg::impl_writer(&ast)
}

#[proc_macro_derive(AnyWireMessageReader)]
pub fn any_wire_message_reader_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    any_wire_msg::impl_reader(&ast)
}

#[proc_macro_derive(WireMessage, attributes(msg_type, tlv_type))]
pub fn wire_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    wire_msg::impl_trait(&ast)
}

#[proc_macro_derive(WireMessageWriter, attributes(msg_type, tlv_type))]
pub fn wire_message_writer_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    wire_msg::impl_writer(&ast)
}

#[proc_macro_derive(WireMessageReader, attributes(msg_type, tlv_type))]
pub fn wire_message_reader_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    wire_msg::impl_reader(&ast)
}
