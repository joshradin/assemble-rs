use std::fs::File as StdFile;
use std::io::Read;
use std::path::{Path, PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::token::Token;
use syn::visit::Visit;
use syn::{parse2, File, Item, ItemFn, ItemMod, Lit, LitStr, Meta, Pat, Token, Visibility};

/// Finds _all_ functions in a project
pub struct FunctionFinder {
    all_functions: Vec<(ModuleData, ItemFn)>,
}

#[derive(Debug, Clone)]
pub struct ModuleData {
    full_path: Vec<String>,
    id: String,
    file_path: PathBuf,
}

impl ModuleData {
    pub fn new(full_path: Vec<String>, id: String, path: PathBuf) -> Self {
        Self {
            full_path,
            id,
            file_path: path,
        }
    }

    fn child_module(&self, id: String, path: PathBuf) -> Self {
        let mut full_path = self.full_path.clone();
        full_path.push(id.clone());
        Self::new(full_path, id, path)
    }

    fn inner_child_module(&self, ids: &[String]) -> Self {
        let mut full_path = self.full_path.clone();
        full_path.extend_from_slice(&ids[..]);
        Self::new(full_path, ids[0].clone(), self.file_path.clone())
    }
}

impl FunctionFinder {
    /// The path is starting file to begin the search.
    pub fn find_all(path: &Path, package_name: String) -> Self {
        let mut module_stack = Vec::new();
        let mut found = Vec::new();

        module_stack.push(ModuleData::new(
            vec![package_name.clone()],
            package_name,
            path.to_path_buf(),
        ));

        while let Some(module) = module_stack.pop() {
            println!("Parsing file: {:?}", module.file_path);

            let module_name = module.id.clone();
            let mut lib_file = StdFile::open(&module.file_path).unwrap();

            let mut content = String::new();
            lib_file.read_to_string(&mut content).unwrap();
            let parsed = syn::parse_file(&content).unwrap();

            let mut visitor = ModuleVisitor::new(module_name.clone());
            visitor.visit_file(&parsed);

            let ModuleVisitor {
                functions, modules, ..
            } = visitor;
            found.extend(functions.into_iter().map(|(modules, fun)| {
                if modules.is_empty() {
                    (module.clone(), fun.clone())
                } else {
                    (module.inner_child_module(modules.as_slice()), fun.clone())
                }
            }));

            for child_module in modules {
                let ident = &child_module.ident;
                let path: Option<String> = child_module
                    .attrs
                    .iter()
                    .find(|attr| attr.path.is_ident("path"))
                    .and_then(|attr| {
                        let meta = parse2::<PathAssign>(attr.tokens.clone()).ok()?;
                        Some(meta.path.value())
                    });

                let parent_dir = module.file_path.parent().unwrap();
                let next_file = if let Some(path) = path {
                    // path specified using #[path = ".."] attribute
                    // see [here](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute)
                    // for how modules from paths are determined
                    parent_dir.join(path)
                } else {
                    // use default path
                    let canonical = if module.file_path.ends_with("mod.rs") {
                        parent_dir.join(ident.to_string()).with_extension("rs")
                    } else {
                        parent_dir
                            .join(&module_name)
                            .join(ident.to_string())
                            .with_extension("rs")
                    };
                    if !canonical.exists() {
                        canonical
                            .with_extension("")
                            .join("mod")
                            .with_extension("rs")
                    } else {
                        canonical
                    }
                };

                module_stack.push(module.child_module(ident.to_string(), next_file));
            }
        }

        Self {
            all_functions: found,
        }
    }

    pub fn found(&self) -> impl Iterator<Item = &(ModuleData, ItemFn)> {
        self.all_functions.iter()
    }

    /// Finds public function ids
    pub fn pub_function_ids(&self) -> impl Iterator<Item = String> + '_ {
        self.found()
            .filter(|(_, fun)| {
                if let Visibility::Public(_) = &fun.vis {
                    true
                } else {
                    false
                }
            })
            .map(|(data, fun)| {
                let module_id = data.full_path.join("::");
                format!("{module_id}::{}", fun.sig.ident)
            })
    }

}

struct ModuleVisitor<'l> {
    module: String,
    inner_modules: Vec<String>,
    /// The found functions
    functions: Vec<(Vec<String>, &'l ItemFn)>,
    /// Non-parsed_modules
    modules: Vec<&'l ItemMod>,
}

impl<'l> ModuleVisitor<'l> {
    pub fn new(module: String) -> Self {
        Self {
            module,
            inner_modules: vec![],
            functions: Default::default(),
            modules: Default::default(),
        }
    }
}

impl<'ast> Visit<'ast> for ModuleVisitor<'ast> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.functions.push((self.inner_modules.clone(), i));
    }

    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        self.inner_modules.push(module.ident.to_string());
        match &module.content {
            None => {
                self.modules.push(module);
            }
            Some((_, items)) => {
                for item in items {
                    self.visit_item(item);
                }
            }
        }
        self.inner_modules.pop();
    }
}

#[derive(Debug)]
struct PathAssign {
    path: LitStr,
}

impl Parse for PathAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![=]>()?;
        let lit = input.parse()?;
        Ok(Self { path: lit })
    }
}
