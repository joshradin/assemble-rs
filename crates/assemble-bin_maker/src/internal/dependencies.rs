use assemble_api::dependencies::{DependencyResolver, DependencyResolverFactory, Source};

pub struct DefaultDependencyResolverFactory {
    sources: Box<dyn Source>,
}

impl DependencyResolverFactory<DefaultDependencyResolver> for DefaultDependencyResolverFactory {
    fn get_resolver(&self) -> DefaultDependencyResolver {
        todo!()
    }
}

pub struct DefaultDependencyResolver {}

impl DependencyResolver for DefaultDependencyResolver {
    fn sources(&self) -> &[&dyn Source] {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn download_from_git() {}

    #[test]
    fn download_from_crates() {}
}
