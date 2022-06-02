extern crate core;
extern crate proc_macro;
extern crate quote;
extern crate syn;

use derive::IntoTaskVisitor;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{parse_macro_input, DeriveInput, ItemFn};

mod actions;
mod derive;

#[proc_macro_derive(IntoTask, attributes(input, output, action))]
pub fn derive_into_task(item: TokenStream) -> TokenStream {
    println!("item: \"{}\"", item.to_string());
    let parsed = parse_macro_input!(item as DeriveInput);
    let ident = parsed.ident.clone();
    let mut visitor = IntoTaskVisitor::new(ident);
    visitor.visit_derive_input(&parsed);
    println!("Parsed = {:#?}", visitor);
    TokenStream::from(quote! { #visitor })
}

#[proc_macro_attribute]
pub fn task_action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    let task_action = parse_macro_input!(item as ItemFn);

    println!("{:#?}", task_action);

    TokenStream::from(quote!())
}
