use crate::derive::{is_prop, Property, PropertyKind};
use crate::TaskVisitor;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::TokenStreamExt;
use syn::spanned::Spanned;
use syn::{Field, Generics, ItemStruct, Meta, NestedMeta, Type};

#[derive(Debug)]
pub struct TaskIO<'a> {
    ty: &'a Ident,
    generics: &'a Generics,
    inputs: Vec<Input<'a>>,
    outputs: Vec<&'a Property>,
}

impl<'a> TaskIO<'a> {
    pub fn new(ty: &'a Ident, generics: &'a Generics) -> Self {
        Self {
            ty,
            generics,
            inputs: vec![],
            outputs: vec![],
        }
    }
}

#[derive(Debug)]
struct Input<'a> {
    field: &'a Field,
    kind: InputKind,
}

impl<'a> Input<'a> {
    pub fn new(field: &'a Field, kind: InputKind) -> Self {
        Self { field, kind }
    }
}

#[derive(Debug)]
pub enum InputKind {
    Transparent,
    File,
    Files,
    Directory,
}

impl<'a> TaskIO<'a> {
    pub fn derive_task_io(visitor: &TaskVisitor) -> syn::Result<TokenStream> {
        let mut task_io = TaskIO::new(visitor.struct_name(), visitor.struct_generics());

        for property in visitor.properties() {
            match &property.kind {
                PropertyKind::Output(_) => {
                    task_io.outputs.push(property);
                }
                PropertyKind::Input(_) => {
                    task_io.add_input(property)?;
                }
                PropertyKind::Internal => {}
            }
        }

        task_io.finish()
    }

    pub fn add_input(&mut self, property: &'a Property) -> syn::Result<()> {
        let input = if let PropertyKind::Input(i) = &property.kind {
            i
        } else {
            unreachable!()
        };
        match input.parse_meta()? {
            Meta::Path(path) => {
                if !path.is_ident("input") {
                    emit_error!(path.span(), "Should not be reachable"; note = "Only input accepted here");
                }
                let input = Input::new(&property.field, InputKind::Transparent);
                self.inputs.push(input);
            }
            Meta::List(l) => {
                if !l.path.is_ident("input") {
                    emit_error!(l.path.span(), "Should not be reachable"; note = "Only input accepted here");
                }
                let nested = l.nested;
                let nested_span = nested.span();
                let mut input: Option<InputKind> = None;

                let mut metas = nested.into_iter().collect::<Vec<_>>();

                if metas.len() == 0 {
                    input = Some(InputKind::Transparent);
                } else if metas.len() == 1 {
                    let meta = metas.remove(0);
                    let path = if let NestedMeta::Meta(meta) = &meta {
                        meta.path()
                    } else {
                        abort!(meta.span(), "Only path expected")
                    };

                    if path.is_ident("file") {
                        input = Some(InputKind::File);
                    } else if path.is_ident("files") {
                        input = Some(InputKind::Files);
                    } else if path.is_ident("directory") {
                        input = Some(InputKind::Directory);
                    }
                }

                if input.is_none() {
                    emit_error!(nested_span, "expected either nothing: file, files, directory";
                        note = "files and directory not implemented yet"
                    )
                }

                let input_kind = input.unwrap();
                let input = Input::new(&property.field, input_kind);
                self.inputs.push(input);
            }
            _ => {
                emit_error!(input.span(), "Only list expected here")
            }
        }
        Ok(())
    }

    pub fn finish(self) -> syn::Result<TokenStream> {
        let inputs = self.inputs;
        let mut inputs_quoted = quote!();

        for input in inputs {
            match input.kind {
                InputKind::Transparent | InputKind::Directory => {
                    let field = input.field.ident.as_ref().unwrap();
                    if is_prop(&input.field.ty) {
                        inputs_quoted = quote! {
                            let #field = task.#field.clone();
                            #inputs_quoted
                            task.work().add_input_prop(&#field);
                        }
                    } else {
                        inputs_quoted = quote! {
                            let #field = task.#field.clone();
                            #inputs_quoted
                            task.work().add_input(stringify!(#field), || #field.clone());
                        }
                    }
                }
                InputKind::Files => {
                    let field = input.field.ident.as_ref().unwrap();
                    inputs_quoted = quote! {
                        let #field = task.#field.clone();
                        #inputs_quoted
                        task.work().add_input_files(stringify!(#field), #field);
                    };
                }
                InputKind::File => {
                    let field = input.field.ident.as_ref().unwrap();
                    inputs_quoted = quote! {
                        let #field = task.#field.clone();
                        #inputs_quoted
                        task.work().add_input_file(stringify!(#field), #field);
                    };
                }
            }
        }

        let mut outputs_quoted = quote!();

        for output in self.outputs {
            let field = &output.field;
            if !is_prop(&field.ty) {
                abort!(
                    field.ty.span(),
                    "Only Prop types are supported currently for outputs"
                )
            }
            let ident = field.ident.as_ref().unwrap();
            outputs_quoted = quote! {
                let #ident = task.#ident.clone();
                #outputs_quoted
                task.work().add_output_provider(#ident);
            };
        }

        let ident = self.ty;
        let (impl_gen, ty_generics, where_clause) = self.generics.split_for_impl();

        Ok(quote! {
            #[automatically_derived]
            impl #impl_gen assemble_core::__export::TaskIO for #ident #ty_generics #where_clause {
                fn configure_io(task: &mut assemble_core::__export::Executable<Self>) -> assemble_core::__export::ProjectResult {
                    #inputs_quoted
                    #outputs_quoted
                    Ok(())
                }
            }
        })
    }
}
