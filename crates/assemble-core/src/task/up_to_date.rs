use crate::{Executable, Task};
use std::fmt::{Debug, Formatter};

/// Can check if this value is up to date.
pub trait UpToDate {
    /// Whether this value is up to date.
    ///
    /// By default, everything is always not up to date.
    fn up_to_date(&self) -> bool {
        true
    }
}

assert_obj_safe!(UpToDate);

pub struct UpToDateHandler<'h, T: Task> {
    container: &'h Vec<Box<dyn Fn(&Executable<T>) -> bool + Send + Sync>>,
    exec: &'h Executable<T>,
}

impl<'h, T: Task> UpToDate for UpToDateHandler<'h, T> {
    fn up_to_date(&self) -> bool {
        for check in self.container {
            if !check(self.exec) {
                return false;
            }
        }
        true
    }
}

/// Contains up to date checks for a task
pub struct UpToDateContainer<T: Task> {
    extra_checks: Vec<Box<dyn Fn(&Executable<T>) -> bool + Send + Sync>>,
}

impl<T: Task> Default for UpToDateContainer<T> {
    fn default() -> Self {
        Self {
            extra_checks: vec![],
        }
    }
}

impl<T: Task> Debug for UpToDateContainer<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpToDateContainer")
            .field("len", &self.extra_checks.len())
            .finish()
    }
}

impl<T: Task> UpToDateContainer<T> {
    /// Add an up to date check
    pub fn up_to_date_if<F>(&mut self, spec: F)
    where
        F: Fn(&Executable<T>) -> bool,
        F: 'static + Send + Sync,
    {
        self.extra_checks.push(Box::new(spec))
    }

    pub fn handler<'h>(&'h self, exec: &'h Executable<T>) -> UpToDateHandler<'h, T> {
        UpToDateHandler {
            container: &self.extra_checks,
            exec,
        }
    }

    pub fn len(&self) -> usize {
        self.extra_checks.len()
    }
}
