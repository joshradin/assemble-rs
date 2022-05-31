//! Handles the parsing of actions

use proc_macro::TokenStream;
use syn::{Block, FnArg, Ident, ItemFn, parse_macro_input, Pat, PatType, Token, token, Type};
use syn::parse::{Parse, ParseStream};

#[derive()]
pub struct TaskActionFunction {
    fn_token: Token![fn],
    pub name: Ident,
    paren_token: token::Paren,
    pub task_param: Ident,
    pub project_param: Ident,
    returns: Token![->],
    pub return_type: Type,
    pub block: Block
}

impl Parse for TaskActionFunction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            fn_token: input.parse()?,
            name: input.parse()?,
            task_param: input.parse()?,
            project_param: input.parse()?,
            block: input.parse()?
        })
    }
}



