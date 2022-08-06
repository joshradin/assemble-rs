//! Define the DupeSpec

use assemble_core::file_collection::FileSet;
use assemble_core::project::VisitProject;
use assemble_core::utilities::{AndSpec, Spec, Work};
use assemble_core::Project;
use std::path::{Path, PathBuf};

/// A dupe spec is used for copying files around. All values (besides children) are inherited
/// by children specs unless specified otherwise. Include and Excludes are always inherited
pub struct DupeSpec {
    /// The files to copy from. If not set, uses the parent spec
    from: Option<FileSet>,
    /// The target directory to copy files in. If not set, uses the parent spec
    into: Option<PathBuf>,
    /// Filters which files to include
    include: Box<dyn Spec<Path>>,
    /// Filters which files should be excluded.
    ///
    /// Only files which pass the included filter are tested to be excluded.
    exclude: Box<dyn Spec<Path>>,
    parent: Option<Box<DupeSpec>>,
}

impl DupeSpec {
    fn get_from(&self) -> Option<&FileSet> {
        self.from
            .as_ref()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_from()))
    }

    fn get_into(&self) -> Option<&Path> {
        self.into
            .as_ref()
            .map(|p| p.as_path())
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_into()))
    }

    fn is_included(&self, path: &Path) -> bool {
        if let Some(parent) = &self.parent {
            if !parent.is_included(path) {
                return false;
            }
        }
        self.include.accept(path) && !self.exclude.accept(path)
    }

    // fn copy(&self, from: ) -> Work {
    //     todo!()
    // }
}

impl VisitProject<Work> for DupeSpec {
    fn visit(&mut self, project: &Project) -> Work {
        todo!()
    }
}
