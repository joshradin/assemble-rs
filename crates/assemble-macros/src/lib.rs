#[macro_use]
extern crate proc_macro_error;
#[macro_use]
extern crate strum;

use crate::derive::create_task::CreateTask;
use crate::derive::io_task::TaskIO;

use derive::TaskVisitor;
use proc_macro::TokenStream;
use quote::ToTokens;

use syn::visit::Visit;
use syn::{parse_macro_input, DeriveInput, ItemFn, Lit};

mod actions;
mod derive;

/// Creates tasks using default values. Also creates lazy_evaluation using the name of the field
#[proc_macro_derive(CreateTask)]
#[proc_macro_error]
pub fn derive_create_task(item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as DeriveInput);
    let mut visitor = TaskVisitor::new(&parsed.ident, &parsed.generics, None);
    visitor.visit_derive_input(&parsed);

    TokenStream::from(CreateTask.derive_create_task(&visitor))
}

/// Enables shortcuts for adding inputs and outputs for tasks
#[proc_macro_derive(TaskIO, attributes(input, output, description))]
#[proc_macro_error]
pub fn derive_io_task(item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as DeriveInput);

    let description = parsed
        .attrs
        .iter()
        .find(|att| att.path.is_ident("description"))
        .map(|att| att.parse_args::<Lit>().expect("must be a string"))
        .map(|lit| {
            if let Lit::Str(str) = lit {
                str.value()
            } else {
                panic!("must be a string literal")
            }
        });

    let mut visitor = TaskVisitor::new(&parsed.ident, &parsed.generics, description);
    visitor.visit_derive_input(&parsed);

    TokenStream::from(TaskIO::derive_task_io(&visitor).unwrap())
}

#[proc_macro_attribute]
pub fn plug(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
