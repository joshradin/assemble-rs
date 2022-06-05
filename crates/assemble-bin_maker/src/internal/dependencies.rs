use assemble_core::dependencies::{DependencyResolver, DependencyResolverFactory, Source};

pub struct DefaultDependencyResolverFactory {
    sources: Vec<Box<dyn Source>>,
}

impl DefaultDependencyResolverFactory {
    pub fn new() -> Self {
        Self { sources: vec![] }
    }

    pub fn add_source<S: 'static + Source>(&mut self, source: S) {
        self.sources.push(Box::new(source))
    }
}

impl<'a> DependencyResolverFactory<'a, DefaultDependencyResolver<'a>>
    for DefaultDependencyResolverFactory
{
    fn get_resolver(&'a self) -> DefaultDependencyResolver<'a> {
        DefaultDependencyResolver {
            sources: &self.sources,
        }
    }
}

pub struct DefaultDependencyResolver<'s> {
    sources: &'s [Box<dyn Source>],
}

impl<'s> DependencyResolver<'s> for DefaultDependencyResolver<'s> {
    fn sources(&self) -> Vec<&'s dyn Source> {
        self.sources.iter().map(|s| s.as_ref()).collect()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn download_from_git() {}

    #[test]
    fn download_from_crates() {}
}
