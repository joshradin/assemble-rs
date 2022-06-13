//! Handles of task actions

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::visit::{visit_item_fn, Visit};
use syn::{FnArg, Ident, ItemFn, Pat, PatType, Path, ReturnType, Type};

#[derive(Default, Debug)]
pub struct ActionVisitor {
    function_name: Option<Ident>,
    return_type: Option<Type>,
    function_args: Vec<FnArg>,
}

impl ActionVisitor {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn finish(self, function: ItemFn) -> TaskActionTokenizer {
        TaskActionTokenizer {
            function_name: self.function_name.unwrap(),
            return_type: self.return_type.unwrap(),
            function_args: self.function_args,
            original_func: function,
        }
    }
}

impl<'ast> Visit<'ast> for ActionVisitor {
    fn visit_fn_arg(&mut self, arg: &'ast FnArg) {
        self.function_args.push(arg.clone());
    }

    fn visit_item_fn(&mut self, arg: &'ast ItemFn) {
        self.function_name = Some(arg.sig.ident.clone());
        visit_item_fn(self, arg);
    }

    fn visit_return_type(&mut self, i: &'ast ReturnType) {
        self.return_type = match i {
            ReturnType::Default => None,
            ReturnType::Type(_, inner) => Some(*inner.clone()),
        };
    }
}

pub struct TaskActionTokenizer {
    function_name: Ident,
    return_type: Type,
    function_args: Vec<FnArg>,
    original_func: ItemFn,
}

impl ToTokens for TaskActionTokenizer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut fixed_function = self.original_func.clone();
        let helper_ident = format_ident!("__{}__helper", self.function_name);
        let mut helper_function_ident = helper_ident.clone();

        std::mem::swap(&mut fixed_function.sig.ident, &mut helper_function_ident);
        // original name should now be in helper_function_ident

        let mut task_type: Option<Type> = None;

        let mut param_names = self
            .function_args
            .iter()
            .map(|arg| match arg {
                FnArg::Receiver(_) => {
                    panic!("Receiver invalid for args")
                }
                FnArg::Typed(pat) => {
                    if let Pat::Ident(ident) = &*pat.pat {
                        if task_type.is_none() {
                            task_type = Some(*pat.ty.clone())
                        }
                        ident.ident.clone()
                    } else {
                        panic!("Only ident tokens allowed here")
                    }
                }
            })
            .collect::<Vec<_>>();

        if param_names.len() != 2 {
            panic!("Only function with 2 args are allowed for build tasks")
        }

        let task_param = param_names.remove(0);
        let project_param = param_names.remove(0);

        let task_type = match task_type.expect("Need a type for tasks") {
            Type::Reference(type_ref) => *type_ref.elem,
            _ => {
                panic!("task type must be &mut <TYPE>")
            }
        };

        tokens.append_all(quote! {

            fn #helper_function_ident (#task_param: &dyn assemble_core::ExecutableTask, #project_param: &assemble_core::Project) -> assemble_core::BuildResult {
                #fixed_function

                use assemble_core::internal::macro_helpers::*;
                use assemble_core::task::property::FromProperties;

                let mut properties = &mut * #task_param.properties();
                let mut recreated_task: #task_type = FromProperties::from_properties(properties);

                let output = #helper_ident(&mut recreated_task, #project_param);

                recreated_task.write_into_properties(&mut properties);

                output
            }

        })
    }
}
