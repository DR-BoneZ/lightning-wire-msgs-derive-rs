extern crate proc_macro;

use proc_macro::TokenStream;

use lightning_wire_msgs_derive_base as base;

#[proc_macro_derive(TryFromPrimitive, attributes(repr))]
pub fn try_from_derive(input: TokenStream) -> TokenStream {
    base::try_from_derive(input)
}

#[proc_macro_derive(AnyWireMessage)]
pub fn any_wire_message_derive(input: TokenStream) -> TokenStream {
    base::any_wire_message_derive(input)
}

#[proc_macro_derive(AnyWireMessageWriter)]
pub fn any_wire_message_writer_derive(input: TokenStream) -> TokenStream {
    base::any_wire_message_writer_derive(input)
}

#[proc_macro_derive(AnyWireMessageReader)]
pub fn any_wire_message_reader_derive(input: TokenStream) -> TokenStream {
    base::any_wire_message_reader_derive(input)
}

#[proc_macro_derive(WireMessage, attributes(msg_type, tlv_type))]
pub fn wire_message_derive(input: TokenStream) -> TokenStream {
    base::wire_message_derive(input)
}

#[proc_macro_derive(WireMessageWriter, attributes(msg_type, tlv_type))]
pub fn wire_message_writer_derive(input: TokenStream) -> TokenStream {
    base::wire_message_writer_derive(input)
}

#[proc_macro_derive(WireMessageReader, attributes(msg_type, tlv_type))]
pub fn wire_message_reader_derive(input: TokenStream) -> TokenStream {
    base::wire_message_reader_derive(input)
}
