use crate::TaskVisitor;
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::Type;

pub struct CreateTask;

impl CreateTask {
    pub fn create_task(&self, visitor: &TaskVisitor) -> TokenStream {
        let struct_type = &visitor.struct_name;

        let mut inner = quote!();

        for prop in &visitor.properties {
            let field = prop.field();

            let id = &field.ident;
            let field_id = id.as_ref().map_or(quote!(), |id| quote! { #id: });
            let ty = &field.ty;
            if let Type::Path(type_path) = ty {
                let last_segment = type_path.path.segments.last().unwrap();
                let final_value = &last_segment.ident;
                let prop_ty = &last_segment.arguments;

                if final_value == "Prop" {
                    inner = quote! {
                        #inner
                        #field_id using_id.prop::#prop_ty(stringify!(#id))?,
                    };
                    continue;
                }
            }

            inner = quote! {
                #inner
                #field_id Default::default(),
            };
        }

        quote! {
            impl assemble_core::__export::CreateTask for #struct_type {
                fn new(using_id: &assemble_core::__export::TaskId, project: &assemble_core::Project) -> assemble_core::project::ProjectResult<Self> {
                    Ok(Self{
                        #inner
                    })
                }
            }
        }
    }
}
