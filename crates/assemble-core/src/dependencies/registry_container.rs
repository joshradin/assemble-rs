use crate::dependencies::file_dependency::FileSystem;
use crate::dependencies::DependencyType;
use std::collections::hash_map::IntoValues;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::path::PathBuf;
use url::Url;
use crate::cache::AssembleCache;

/// A registry is some repository that interprets can interpret some [`Url`](url::Url)
pub trait Registry {
    /// Gets the base url this registry is located at.
    fn url(&self) -> Url;

    /// The dependency types that this registry supports
    fn supported(&self) -> Vec<DependencyType>;
}

assert_obj_safe!(Registry);

/// A container of registries
pub struct RegistryContainer {
    type_to_registry_index: HashMap<DependencyType, HashSet<usize>>,
    registries: Vec<Box<dyn Registry>>,
    cache_location: PathBuf,
}

unsafe impl Send for RegistryContainer {}
unsafe impl Sync for RegistryContainer {}

impl Default for RegistryContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryContainer {
    /// Creates a new registry container. Always contains the default `FileSystem`.
    pub fn new() -> Self {
        let mut container = Self {
            type_to_registry_index: Default::default(),
            registries: vec![],
            cache_location: AssembleCache::default().to_path_buf()
        };
        container.add_registry(FileSystem::default());
        container
    }

    /// Add a registry to this container
    pub fn add_registry<R: Registry + 'static>(&mut self, registry: R) {
        let index = self.registries.len();
        let boxed = Box::new(registry) as Box<dyn Registry>;
        let dependency_types = boxed.supported();
        self.registries.push(boxed);

        for dependency_type in dependency_types {
            self.type_to_registry_index
                .entry(dependency_type)
                .or_default()
                .insert(index);
        }
    }

    /// Gets the amount of registries in this container
    pub fn len(&self) -> usize {
        self.registries.len()
    }

    /// Gets the supported registries for a given dependency type
    pub fn supported_registries(&self, ty: &DependencyType) -> RegistrySet {
        if let Some(entry) = self.type_to_registry_index.get(ty) {
            entry
                .into_iter()
                .copied()
                .map(|index| &*self.registries[index])
                .collect()
        } else {
            RegistrySet::default()
        }
    }

    /// Gets the intersection of supported registries for given dependency types
    pub fn supported_registries_intersection<'a, I>(&'a self, ty: I) -> RegistrySet<'a>
    where
        I: IntoIterator<Item = &'a DependencyType>,
    {
        ty.into_iter()
            .map(|ty| self.supported_registries(ty))
            .fold(RegistrySet::default(), RegistrySet::intersection)
    }

    /// Set the location where files should be downloaded to
    pub fn set_cache_location(&mut self, cache_location: PathBuf) {
        self.cache_location = cache_location;
    }

    /// Get where files should be downloaded to
    pub fn cache_location(&self) -> &PathBuf {
        &self.cache_location
    }
}

#[derive(Default)]
pub struct RegistrySet<'a> {
    map: HashMap<Url, &'a dyn Registry>,
}

impl<'a> IntoIterator for RegistrySet<'a> {
    type Item = &'a dyn Registry;
    type IntoIter = IntoValues<Url, &'a dyn Registry>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_values()
    }
}

impl<'a> RegistrySet<'a> {
    /// The intersection of two registry sets
    pub fn intersection(self, other: Self) -> Self {
        let mut map = HashMap::new();
        for (url, reg) in self.map {
            if other.map.contains_key(&url) {
                map.insert(url, reg);
            }
        }
        Self { map }
    }

    /// Gets the registries in this set
    pub fn registries(&self) -> impl IntoIterator<Item = &'a dyn Registry> {
        self.map.values().map(|d| *d).collect::<Vec<_>>()
    }
}

impl<'a> FromIterator<&'a dyn Registry> for RegistrySet<'a> {
    fn from_iter<T: IntoIterator<Item = &'a dyn Registry>>(iter: T) -> Self {
        Self {
            map: iter
                .into_iter()
                .map(|reg: &dyn Registry| (reg.url(), reg))
                .collect(),
        }
    }
}
