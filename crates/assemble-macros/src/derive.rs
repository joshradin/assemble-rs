use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::visit::{visit_derive_input, Visit};
use syn::{DataEnum, DataUnion, DeriveInput, Field, Ident, Type};

pub mod create_task;

#[derive(Debug)]
pub struct Property {
    kind: PropertyKind,
    field: Field,
}

#[derive(Debug)]
pub enum PropertyKind {
    Output,
    Input,
    Internal
}

impl Property {



    pub fn kind(&self) -> &PropertyKind {
        &self.kind
    }
    pub fn field(&self) -> &Field {
        &self.field
    }
    pub fn new(kind: PropertyKind, field: Field) -> Self {
        Self { kind, field }
    }
}

#[derive(Debug)]
pub struct TaskVisitor {
    struct_name: Ident,
    properties: Vec<Property>,
    action: Option<Ident>,
}

impl TaskVisitor {
    pub fn new(struct_name: Ident) -> Self {
        Self {
            struct_name,
            properties: vec![],
            action: None,
        }
    }
}

impl Visit<'_> for TaskVisitor {
    fn visit_data_enum(&mut self, i: &'_ DataEnum) {
        panic!("enums not supported for IntoTask")
    }

    fn visit_data_union(&mut self, i: &'_ DataUnion) {
        panic!("unions not supported for IntoTask")
    }

    fn visit_derive_input(&mut self, i: &'_ DeriveInput) {
        let attribute = i.attrs.iter().find(|attr| attr.path.is_ident("action"));
        if let Some(attribute) = attribute {
            let action_ident: Ident = attribute.parse_args().expect("expected an identifier");
            self.action = Some(action_ident);
        }
        visit_derive_input(self, i);
    }

    fn visit_field(&mut self, i: &'_ Field) {
        let is_input = i
            .attrs
            .iter()
            .find(|att| att.path.is_ident("input"))
            .is_some();
        let is_output = i
            .attrs
            .iter()
            .find(|att| att.path.is_ident("output"))
            .is_some();

        if is_input && is_output {
            panic!("field can not be marked as both input and output.")
        }

        if is_output {
            self.properties.push(Property::new(PropertyKind::Output, i.clone()))
        } else if is_input {
            self.properties.push(Property::new(PropertyKind::Input, i.clone()))
        } else {
            self.properties.push(Property::new(PropertyKind::Internal, i.clone()))
        }
    }
}

