use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::visit::{visit_derive_input, Visit};
use syn::{DataEnum, DataUnion, DeriveInput, Field, Ident, Type};

#[derive(Debug)]
pub enum Property {
    Output(Field),
    Input(Field),
    Internal(Field),
}

impl Property {
    pub fn ident(&self) -> &Ident {
        match self {
            Property::Output(v) => v.ident.as_ref().unwrap(),
            Property::Input(v) => v.ident.as_ref().unwrap(),
            Property::Internal(v) => v.ident.as_ref().unwrap(),
        }
    }

    pub fn field_type(&self) -> &Type {
        match self {
            Property::Output(f) => &f.ty,
            Property::Input(f) => &f.ty,
            Property::Internal(f) => &f.ty,
        }
    }
}

#[derive(Debug)]
pub struct IntoTaskVisitor {
    struct_name: Ident,
    properties: Vec<Property>,
    action: Option<Ident>,
}

impl IntoTaskVisitor {
    pub fn new(struct_name: Ident) -> Self {
        Self {
            struct_name,
            properties: vec![],
            action: None,
        }
    }
}

impl Visit<'_> for IntoTaskVisitor {
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
        let ident = i
            .ident
            .as_ref()
            .cloned()
            .expect("only named identifiers allowed");
        if is_output {
            self.properties.push(Property::Output(i.clone()))
        } else if is_input {
            self.properties.push(Property::Input(i.clone()))
        } else {
            self.properties.push(Property::Internal(i.clone()))
        }
    }
}

impl ToTokens for IntoTaskVisitor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let IntoTaskVisitor {
            struct_name,
            properties,
            action,
        } = self;

        let mut all_properties = TokenStream::new();

        let mut from_properties = TokenStream::new();
        let mut write_into_properties = TokenStream::new();

        let mut keys = vec![];
        let mut keys_types = vec![];

        for property in properties {
            let key = property.ident();
            keys.push(key.clone());

            let field_type = property.field_type().clone();
            keys_types.push(field_type);

            let getter = quote! { self.#key.to_owned() };
            all_properties.append_all(quote! {
                properties.set(stringify!(#key), #getter);
            });
        }

        tokens.append_all(quote! {

            impl assemble_core::task::IntoTask for #struct_name {
                type Task = assemble_core::defaults::task::DefaultTask;
                type Error = ();

                fn create() -> Self {
                    <Self as Default>::default()
                }

                fn default_task() -> Self::Task {
                    assemble_core::defaults::task::DefaultTask::default()
                }

                fn inputs(&self) -> Vec<&str> {
                    vec![]
                }

                fn outputs(&self) -> Vec<&str> {
                    vec![]
                }

                fn set_properties(&self, properties: &mut assemble_core::task::TaskProperties) {
                    #all_properties
                }
            }

            impl assemble_core::internal::macro_helpers::WriteIntoProperties for #struct_name {}
            impl assemble_core::task::property::FromProperties for #struct_name {
                fn from_properties(properties: &mut assemble_core::task::TaskProperties) -> Self {
                    use assemble_core::IntoTask as _;
                    let mut created = Self::create();
                    #(
                        created.#keys = properties.get::<#keys_types, _>(stringify!(#keys))
                                                    .expect(&format!("No property named {} found", stringify!(#keys)))
                                                    .clone();
                    )*
                    created
                }
            }
        });

        if let Some(action) = action {
            tokens.append_all(quote! {
                impl assemble_core::task::GetTaskAction for #struct_name {
                    fn task_action(task: &dyn assemble_core::task::Task, project: &assemble_core::project::Project) -> assemble_core::BuildResult {
                        (#action)(task, project)
                    }
                }
            });
        }
    }
}