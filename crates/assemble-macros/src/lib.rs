#[macro_use] extern crate proc_macro_error;

use crate::derive::create_task::CreateTask;
use actions::ActionVisitor;
use derive::TaskVisitor;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{parse_macro_input, DeriveInput, ItemFn};
use crate::derive::io_task::TaskIO;

mod actions;
mod derive;



/// Creates tasks using default values. Also creates properties using the name of the field
#[proc_macro_derive(CreateTask)]
pub fn derive_create_task(item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as DeriveInput);
    let ident = parsed.ident.clone();
    let mut visitor = TaskVisitor::new(ident);
    visitor.visit_derive_input(&parsed);

    TokenStream::from(CreateTask.derive_create_task(&visitor))
}

/// Enables shortcuts for adding inputs and outputs for tasks
#[proc_macro_derive(TaskIO, attributes(input, output))]
#[proc_macro_error]
pub fn derive_io_task(item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as DeriveInput);
    let ident = parsed.ident.clone();
    let mut visitor = TaskVisitor::new(ident);
    visitor.visit_derive_input(&parsed);

    TokenStream::from(TaskIO::derive_task_io(&visitor).unwrap())
}



#[proc_macro_attribute]
pub fn plug(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
