//! An error with a payload


use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use crate::project::ProjectError;

/// An payload with an error
#[derive(Debug)]
pub struct PayloadError<E> {
    kind: E,
    bt: Backtrace,
}

impl<E> PayloadError<E> {
    /// Create a new payloaded error.
    #[inline]
    pub fn new<E2>(error: E2) -> Self
    where
        E2: Into<E>,
    {
        Self::with_backtrace(error, Backtrace::capture())
    }

    /// create a new payload error with a backtrace
    pub fn with_backtrace<E2>(kind: E2, bt: Backtrace) -> Self where
        E2: Into<E>, {
        Self { kind: kind.into(), bt }
    }

    /// Gets the error kind
    pub fn kind(&self) -> &E {
        &self.kind
    }

    /// Gets the backtrace
    pub fn backtrace(&self) -> &Backtrace {
        &self.bt
    }

    /// Convert the error type
    pub fn into<T>(self) -> PayloadError<T>
    where
        E: Into<T>,
    {
        PayloadError {
            kind: self.kind.into(),
            bt: self.bt,
        }
    }

    /// Unwraps the payloaded error
    pub fn into_inner(self) -> E {
        self.kind
    }
}


impl<E> From<E> for PayloadError<E> {
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

impl<E: Display> Display for PayloadError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl<E: Error> Error for PayloadError<E> {}

impl<E> AsRef<E> for PayloadError<E> {
    fn as_ref(&self) -> &E {
        &self.kind
    }
}

/// A result with a pay-loaded error
pub type Result<T, E> = std::result::Result<T, PayloadError<E>>;


impl From<io::Error> for PayloadError<ProjectError> {
    fn from(e: io::Error) -> Self {
        PayloadError::new(e)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::PayloadError;

    #[test]
    fn create_payload() {
        let res = PayloadError::new(());
        let bt = res.backtrace();
        println!("{:?}", bt);
    }
}
