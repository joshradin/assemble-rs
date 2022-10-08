use crate::derive::{is_prop, Property, PropertyKind};
use crate::strum::{IntoEnumIterator, VariantNames};
use crate::TaskVisitor;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{Field, Generics, ItemStruct, Meta, NestedMeta, PathArguments, Type};

#[derive(Debug)]
pub struct TaskIO<'a> {
    ty: &'a Ident,
    generics: &'a Generics,
    inputs: Vec<Input<'a>>,
    outputs: Vec<Output<'a>>,
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
pub enum InputKind {
    Transparent,
    File,
    Files,
    Directory,
}

impl<'a> Input<'a> {
    pub fn new(field: &'a Field, kind: InputKind) -> Self {
        Self { field, kind }
    }
}

#[derive(Debug)]
struct Input<'a> {
    field: &'a Field,
    kind: InputKind,
}

#[derive(Debug)]
struct Output<'a> {
    field: &'a Field,
    kind: OutputKind,
}

impl<'a> Output<'a> {
    pub fn new(field: &'a Field, kind: OutputKind) -> Self {
        Self { field, kind }
    }
}

#[derive(Debug, Default, EnumString, EnumVariantNames, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum OutputKind {
    #[default]
    Serializable,
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
                    task_io.add_output(property)?;
                }
                PropertyKind::Input(_) => {
                    task_io.add_input(property)?;
                }
                PropertyKind::Internal => {}
            }
        }

        task_io.finish()
    }

    pub fn add_output(&mut self, property: &'a Property) -> syn::Result<()> {
        let output_att = if let PropertyKind::Output(attr) = &property.kind {
            attr
        } else {
            unreachable!()
        };

        let output_kind = match output_att.parse_meta()? {
            Meta::Path(path) => {
                if !path.is_ident("output") {
                    emit_error!(path.span(), "Only output accepted here");
                }

                OutputKind::Serializable
            }
            Meta::List(list) => {
                let nested = &list.nested;
                let nested_span = nested.span();
                let mut metas = nested.into_iter().collect::<Vec<_>>();

                if metas.len() == 0 {
                    OutputKind::Serializable
                } else if metas.len() == 1 {
                    let meta = metas.remove(0);
                    let path = if let NestedMeta::Meta(meta) = &meta {
                        meta.path()
                    } else {
                        abort!(meta.span(), "Only path expected")
                    };

                    let mut out: Option<OutputKind> = None;
                    for kind in OutputKind::iter() {
                        if path.is_ident(kind.as_ref()) {
                            out.replace(kind);
                            break;
                        }
                    }

                    if let Some(out) = out {
                        out
                    } else {
                        abort!(
                            meta.span(),
                            "Only one of these values expected here: {:?}",
                            OutputKind::VARIANTS
                        )
                    }
                } else {
                    abort!(
                        list,
                        "Only one of these values expected here: {:?}",
                        OutputKind::VARIANTS
                    );
                }
            }
            Meta::NameValue(_) => {
                abort!(output_att.span(), "Only list expected here");
            }
        };

        let output = Output::new(&property.field, output_kind);
        self.outputs.push(output);
        Ok(())
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
                            task.work().add_input(stringify!(#field), provider!(|| #field.clone()));
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

        let mut restore_output = quote! {};

        for output in self.outputs {
            let field = &output.field;
            if !is_prop(&field.ty) {
                abort!(
                    field.ty.span(),
                    "Only Prop types are supported currently for outputs"
                )
            }
            let ident = field.ident.as_ref().unwrap();

            match output.kind {
                OutputKind::Serializable => {
                    outputs_quoted = quote! {
                        let #ident = task.#ident.clone();
                        #outputs_quoted
                        task.work().add_serialized_data(stringify!(#ident), #ident);
                    };
                    if let Type::Path(type_path) = &field.ty {
                        let last_segment = type_path.path.segments.last().unwrap();
                        let final_value = &last_segment.ident;
                        let prop_ty = &last_segment.arguments;

                        if final_value == "Prop" {
                            let inner = match prop_ty {
                                PathArguments::AngleBracketed(a) => &a.args,
                                _ => {
                                    unreachable!()
                                }
                            };

                            restore_output = quote! {
                                #restore_output
                                if let Some(value) = map.get(stringify!(#ident)) {
                                    let value: #inner = value.deserialize()?;
                                    self.#ident.set(value)?;
                                }

                            };
                        } else if final_value == "VecProp" {
                            restore_output = quote! {
                                #restore_output
                                if let Some(value) = map.get(stringify!(#ident)) {
                                    let value: Vec #prop_ty = value.deserialize()?;
                                    self.#ident.push_all(value);
                                }

                            };
                        }
                    } else {
                        abort!(ident.span(), "Only type paths accepted")
                    }
                }
                OutputKind::File | OutputKind::Files | OutputKind::Directory => {
                    outputs_quoted = quote! {
                        let #ident = task.#ident.clone();
                        #outputs_quoted
                        task.work().add_output_provider(#ident);
                    };
                }
            }
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

                fn recover_outputs(&mut self, output: &assemble_core::task::work_handler::output::Output) -> assemble_core::__export::ProjectResult {
                    use assemble_core::task::work_handler::output::Output;
                    if let Some(map) = output.serialized_data() {
                        #restore_output
                    }

                    Ok(())
                }
            }
        })
    }
}
