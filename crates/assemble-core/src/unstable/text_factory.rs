//! Text factory adds some useful traits and factories for producing text.

use crate::identifier::{ProjectId, TaskId};
use colored::Colorize;
use std::fmt;
use std::fmt::{Display, Formatter, Write};
use std::time::Duration;

pub mod list;
pub mod graph;

/// Write text to a writer
#[derive(Debug)]
pub struct AssembleFormatter<W: Write> {
    writer: W,
}

impl<W: Write + Display> Display for AssembleFormatter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.writer)
    }
}

impl Default for AssembleFormatter<String> {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl<W: Write> AssembleFormatter<W> {
    /// Create a new text factory wrapper around some writer
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Get a writer to write important information
    pub fn important(&mut self) -> Important<W> {
        Important { factory: self }
    }

    /// Get a writer to write important information
    pub fn less_important(&mut self) -> LessImportant<W> {
        LessImportant { factory: self }
    }

    /// Print some sort of task status
    pub fn project_status<S: ToString>(
        mut self,
        id: &ProjectId,
        status: S,
    ) -> Result<Self, fmt::Error> {
        let formatted = format!("> {} {}", status.to_string(), id)
            .bold()
            .to_string();
        write!(self, "{}", formatted)?;
        Ok(self)
    }

    /// Print some sort of task status
    pub fn task_status<S: ToString>(mut self, id: &TaskId, status: S) -> Result<Self, fmt::Error> {
        let mut formatted = format!("> Task {}", id).bold().to_string();
        let status = status.to_string();
        if !status.trim().is_empty() {
            formatted = format!("{} - {}", formatted, status);
        }
        write!(self, "{}", formatted)?;
        Ok(self)
    }

    /// Finishes the factory, returning the writer that this factory was wrapping
    pub fn finish(self) -> W {
        self.writer
    }
}

impl<W: Write> Write for AssembleFormatter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)
    }
}

/// Print important information
pub struct Important<'f, W: Write> {
    factory: &'f mut AssembleFormatter<W>,
}

impl<'f, W: Write> Write for Important<'f, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.factory, "{}", s.bold())
    }
}

/// Print important information
pub struct LessImportant<'f, W: Write> {
    factory: &'f mut AssembleFormatter<W>,
}

/// Shortcut to format a string with the less_important formatter
pub fn less_important_string<S: ToString>(s: S) -> String {
    let mut formatter = AssembleFormatter::default();
    write!(formatter.less_important(), "{}", s.to_string()).unwrap();
    formatter.finish()
}

impl<'f, W: Write> Write for LessImportant<'f, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.factory, "{}", s.yellow())
    }
}

/// Emit build results
#[derive(Debug)]
pub struct BuildResultString {
    result_good: bool,
    time: Duration,
}

impl BuildResultString {
    /// Construct a new build result
    pub fn new(result_good: bool, time: Duration) -> Self {
        Self { result_good, time }
    }
}

impl Display for BuildResultString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let status = if self.result_good {
            "BUILD SUCCEEDED".bright_green().bold()
        } else {
            "BUILD FAILED".bright_red().bold()
        };
        write!(f, "{} in {:.2} sec", status, self.time.as_secs_f64())
    }
}
