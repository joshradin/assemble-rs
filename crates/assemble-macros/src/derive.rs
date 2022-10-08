use syn::{
    Attribute, DataEnum, DataUnion, DeriveInput, Field, Generics, Ident, Type,
};
use syn::visit::{Visit, visit_derive_input};

pub mod create_task;
pub mod io_task;

#[derive(Debug)]
pub struct Property {
    kind: PropertyKind,
    field: Field,
}

#[derive(Debug)]
pub enum PropertyKind {
    Output(Attribute),
    Input(Attribute),
    Internal,
}

impl Property {
    pub fn field(&self) -> &Field {
        &self.field
    }
    pub fn new(kind: PropertyKind, field: Field) -> Self {
        Self { kind, field }
    }
}

/// Get whether this type is [`Prop`](assemble_core::lazy_evaluation::Prop)
pub fn is_prop(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            let ident = &path.path;
            let segment = ident.segments.first().unwrap();

            segment.ident == "Prop"
                || segment.ident == "VecProp"
                || segment.ident == "AnonymousProvider"
        }
        _ => false,
    }
}


#[derive(Debug)]
pub struct TaskVisitor {
    ident: Ident,
    generics: Generics,
    properties: Vec<Property>,
    action: Option<Ident>,
    _description: Option<String>,
}

impl TaskVisitor {
    pub fn new(ident: &Ident, generics: &Generics, desc: Option<String>) -> Self {
        Self {
            ident: ident.clone(),
            generics: generics.clone(),
            properties: vec![],
            action: None,
            _description: desc,
        }
    }

    pub fn struct_name(&self) -> &Ident {
        &self.ident
    }

    pub fn struct_generics(&self) -> &Generics {
        &self.generics
    }

    /// Gets the fields found
    pub fn properties(&self) -> &[Property] {
        &self.properties[..]
    }
}

impl Visit<'_> for TaskVisitor {
    fn visit_data_enum(&mut self, _i: &'_ DataEnum) {
        panic!("enums not supported for IntoTask")
    }

    fn visit_data_union(&mut self, _i: &'_ DataUnion) {
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
        let input = i.attrs.iter().find(|att| att.path.is_ident("input"));
        let output = i.attrs.iter().find(|att| att.path.is_ident("output"));

        if input.is_some() && output.is_some() {
            panic!("field can not be marked as both input and output.")
        }

        if let Some(input) = input {
            self.properties
                .push(Property::new(PropertyKind::Input(input.clone()), i.clone()))
        } else if let Some(output) = output {
            self.properties.push(Property::new(
                PropertyKind::Output(output.clone()),
                i.clone(),
            ))
        } else {
            self.properties
                .push(Property::new(PropertyKind::Internal, i.clone()))
        }
    }
}
