use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use rquickjs::bind;
use assemble_core::identifier::Id as CoreId;

#[bind(public, object)]
mod id {
    use std::fmt::Display;
    use rquickjs::FromJs;

    #[derive(Debug, Clone, FromJs)]
    pub struct IdObject {
        pub(super) parent: Option<Box<IdObject>>,
        pub(super) this: String
    }

    impl IdObject {
        pub fn new(this: String, parent: Option<IdObject>) -> Self {
            Self {
                parent: parent.map(Box::new),
                this,
            }
        }

    }
}

impl Display for IdObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.parent {
            None => {
                write!(f, "{}", self.this)
            }
            Some(parent) => {
                write!(f, "{}::{}", parent, self.this)
            }
        }
    }
}

impl<T : Deref<Target=CoreId>> From<T> for IdObject {
    fn from(value: T) -> Self {
        let id = value.deref().clone();
        Self::new(id.this().to_string(), id.parent().map(|i| IdObject::from(i)))
    }
}

pub use id::*;