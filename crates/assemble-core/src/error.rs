//! An error with a payload

use backtrace::Backtrace;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// An payload with an error
#[derive(Debug)]
pub struct PayloadError<E> {
    kind: E,
    bt: Backtrace,
}

impl<E> PayloadError<E> {
    /// Create a new payloaded error.
    #[inline]
    pub fn new(error: E) -> Self {
        Self::with_backtrace(error, Backtrace::new())
    }

    /// create a new payload error with a backtrace
    pub fn with_backtrace(kind: E, bt: Backtrace) -> Self {
        Self { kind, bt }
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
