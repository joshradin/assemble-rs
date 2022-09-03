//! Extensions that plugins can add

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

use thiserror::Error;

/// A a helper trait that extends the needed traits to add a value as an extension
pub trait Extension: 'static + Send + Sync {}

impl<E: 'static + Send + Sync> Extension for E {}

/// A type that contains extensions
pub trait ExtensionAware {
    /// Gets the extension container
    fn extensions(&self) -> &ExtensionContainer;
    /// Gets a mutable reference to the extension container
    fn extensions_mut(&mut self) -> &mut ExtensionContainer;

    /// If a single extension is registered with a given type, a reference to that value is returned
    /// as `Some(_)`
    fn extension<E: Extension>(&self) -> Option<&E> {
        self.extensions().get_by_type()
    }

    /// If a single extension is registered with a given type, a mutable reference to that value is returned
    /// as `Some(_)`
    fn extension_mut<E: Extension>(&mut self) -> Option<&mut E> {
        self.extensions_mut().get_by_type_mut()
    }
}

type AnyExtension = Box<dyn Any + Send + Sync>;

/// Contains extensions
#[derive(Default)]
pub struct ExtensionContainer {
    ob_map: HashMap<String, AnyExtension>,
}

impl ExtensionContainer {
    /// Adds a new extension to this container
    ///
    /// # Error
    /// Will return an error if `name` is already registered to this container
    pub fn add<E: Extension, S: AsRef<str>>(
        &mut self,
        name: S,
        value: E,
    ) -> Result<(), ExtensionError> {
        let name = name.as_ref();
        if self.ob_map.contains_key(name) {
            return Err(ExtensionError::AlreadyRegistered(name.to_string()));
        }
        let boxed = Box::new(value) as AnyExtension;
        self.ob_map.insert(name.to_string(), boxed);
        Ok(())
    }

    /// Gets a reference to an extension, if it exists
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<&AnyExtension> {
        self.ob_map.get(name.as_ref())
    }

    /// Gets a mutable reference to an extension, if it exists
    pub fn get_mut<S: AsRef<str>>(&mut self, name: S) -> Option<&mut AnyExtension> {
        self.ob_map.get_mut(name.as_ref())
    }

    /// If a single extension is registered with a given type, a reference to that value is returned
    /// as `Some(_)`
    pub fn get_by_type<E: Extension>(&self) -> Option<&E> {
        let mut output: Vec<&E> = vec![];
        for value in self.ob_map.values() {
            if let Some(ext) = value.downcast_ref::<E>() {
                output.push(ext);
            }
        }
        match output.len() {
            1 => Some(output.remove(0)),
            _ => None,
        }
    }

    /// If a single extension is registered with a given type, a mutable reference to that value is returned
    /// as `Some(_)`
    pub fn get_by_type_mut<E: Extension>(&mut self) -> Option<&mut E> {
        let mut output: Vec<String> = vec![];
        for (name, ext) in &self.ob_map {
            if ext.is::<E>() {
                output.push(name.clone());
            }
        }
        match output.len() {
            1 => {
                let index = output.remove(0);
                self.ob_map.get_mut(&index).and_then(|b| b.downcast_mut())
            }
            _ => None,
        }
    }
}

impl Index<&str> for ExtensionContainer {
    type Output = AnyExtension;

    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl IndexMut<&str> for ExtensionContainer {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl Index<String> for ExtensionContainer {
    type Output = AnyExtension;

    fn index(&self, index: String) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl IndexMut<String> for ExtensionContainer {
    fn index_mut(&mut self, index: String) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

impl Debug for ExtensionContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtensionContainer").finish()
    }
}

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Extension with name {0:?} already registered")]
    AlreadyRegistered(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_extensions() {
        let mut ext = ExtensionContainer::default();
        ext.add("test", String::from("Hello, World")).unwrap();

        let value = ext.get("test").unwrap().downcast_ref::<String>().unwrap();
        assert_eq!(value, "Hello, World")
    }

    #[test]
    fn disallow_same_name_extensions() {
        let mut ext = ExtensionContainer::default();
        ext.add("test", String::from("Hello, World")).unwrap();
        assert!(matches!(
            ext.add("test", String::from("Hello, World")),
            Err(ExtensionError::AlreadyRegistered(_))
        ));
    }
}
