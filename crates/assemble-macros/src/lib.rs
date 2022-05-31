extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(IntoTask, attributes(input, output, action))]
pub fn derive_into_task(item: TokenStream) -> TokenStream {
    println!("item: \"{}\"", item.to_string());
    TokenStream::new()
}

#[proc_macro_attribute]
pub fn task_action(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    item
}