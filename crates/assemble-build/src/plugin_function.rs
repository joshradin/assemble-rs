//! Create plugin functions

use syn::{custom_punctuation, ExprAssign, Ident, ItemFn, Lit, Signature, Token};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

#[derive(Debug)]
pub struct PluginFunction {
    module: String,
    identifier: String,
    sig: Signature,
    meta: PluginFunctionMetadata
}

#[derive(Debug)]
pub struct PluginFunctionMetadata {
    plugin_id: String
}


impl PluginFunction {
    pub fn try_create(module: String, item: ItemFn) -> Option<Self> {
        None
    }
}

struct Assignment {
    id: Ident,
    eq: Token![=],
    value: Lit
}

impl Parse for Assignment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            id: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?
        })
    }
}

impl Parse for PluginFunctionMetadata {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let split: Punctuated<Assignment, Token![,]> = input.parse_terminated(Assignment::parse)?;

        let mut output = Self { plugin_id: "".to_string() };
        for assignment in split {
            let id = assignment.id.to_string();
            let value = assignment.value;

            match &*id {
                "plugin_id" => {
                    match &value {
                        Lit::Str(s) => {
                            output.plugin_id = s.value();
                        }
                        _ => {
                            return Err(syn::Error::new(value.span(), "plugin_id must be a string"));
                        }
                    };
                }
                _ => {
                    return Err(syn::Error::new(assignment.id.span(), "not a valid setting for Plugins"))
                }
            };
        }

        Ok(output)

    }
}