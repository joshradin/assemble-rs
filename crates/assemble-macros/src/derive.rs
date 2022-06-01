use quote::__private::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::visit::{visit_derive_input, Visit};
use syn::{DataEnum, DataUnion, DeriveInput, Field, Ident};

#[derive(Debug)]
pub enum Property {
    Output(Ident),
    Input(Ident),
    Internal(Ident),
}

impl Property {
    pub fn ident(&self) -> &Ident {
        match self {
            Property::Output(v) => v,
            Property::Input(v) => v,
            Property::Internal(v) => v,
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
            self.properties.push(Property::Output(ident))
        } else if is_input {
            self.properties.push(Property::Input(ident))
        } else {
            self.properties.push(Property::Internal(ident))
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

        for property in properties {
            let key = property.ident();
            let getter = quote! { <ToOwned>::to_owned(&self.#key) };
            all_properties.append_all(quote! {
                properties.set(stringify!(#key), #getter);
            });
        }

        tokens.append_all(quote! {

            impl assemble_api::task::IntoTask for #struct_name {
                type Task = assemble_api::defaults::tasks::DefaultTask;
                type Error = ();

                fn create() -> Self {
                    <Self as Default>::default()
                }

                fn default_task() -> Self::Task {
                    DefaultTask::default()
                }

                fn inputs(&self) -> Vec<&str> {
                    vec![]
                }

                fn outputs(&self) -> Vec<&str> {
                    vec![]
                }

                fn set_properties(&self, properties: &mut assemble_api::task::TaskProperties) {
                    #all_properties
                }
            }
        });

        if let Some(action) = action {
            tokens.append_all(quote! {
                impl assemble_api::task::ActionableTask for #struct_name {
                    fn task_action(task: &dyn assemble_api::task::Task, project: &assemble_api::project::Project) -> assemble_api::BuildResult {
                        (#action)(task, project)
                    }
                }
            });
        }
    }
}
