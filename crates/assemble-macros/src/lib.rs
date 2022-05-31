extern crate proc_macro;
extern crate syn;
extern crate quote;
extern crate core;

use proc_macro::{TokenStream};
use quote::quote;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput, ItemFn};
use actions::{TaskActionFunction};

mod actions;

#[proc_macro_derive(IntoTask, attributes(input, output, action))]
pub fn derive_into_task(item: TokenStream) -> TokenStream {
    println!("item: \"{}\"", item.to_string());
    TokenStream::new()
}

#[proc_macro_attribute]
pub fn task_action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    let task_action = parse_macro_input!(item as TaskActionFunction);

    TokenStream::from(quote!())
}