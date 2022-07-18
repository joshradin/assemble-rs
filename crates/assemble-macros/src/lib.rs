use actions::ActionVisitor;
use derive::TaskVisitor;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{parse_macro_input, DeriveInput, ItemFn};
use crate::derive::create_task::CreateTask;

mod actions;
mod derive;

/// Creates tasks using default values. Also creates properties using the name of the field
#[proc_macro_derive(CreateTask)]
pub fn derive_create_task(item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as DeriveInput);
    let ident = parsed.ident.clone();
    let mut visitor = TaskVisitor::new(ident);
    visitor.visit_derive_input(&parsed);

    TokenStream::from(CreateTask.create_task(&visitor))
}

// #[proc_macro_derive(Task, attributes(input, output, nested, action))]
// pub fn derive_into_task(item: TokenStream) -> TokenStream {
//     // println!("item: \"{}\"", item.to_string());
//     let parsed = parse_macro_input!(item as DeriveInput);
//     let ident = parsed.ident.clone();
//     let mut visitor = IntoTaskVisitor::new(ident);
//     visitor.visit_derive_input(&parsed);
//     // println!("Parsed = {:#?}", visitor);
//     TokenStream::from(quote! { #visitor })
// }


#[proc_macro_attribute]
pub fn plug(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
