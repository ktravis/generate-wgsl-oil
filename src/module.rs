use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
};

use crate::{
    exports,
    files::AbsoluteWGSLFilePathBuf,
    imports::{self, ImportResolutionError},
};

/// A single requested import to a shader.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct Module {
    path: AbsoluteWGSLFilePathBuf,
}

impl Module {
    /// Given a path to a file and the string given to describe an import, tries to resolve the requested import file.
    pub(crate) fn from_path(path: AbsoluteWGSLFilePathBuf) -> Self {
        Self { path }
    }

    /// Given a path to a file and the string given to describe an import, tries to resolve the requested import file.
    pub(crate) fn resolve_module(
        importing: &Module,
        project_root: &PathBuf,
        request_string: &str,
    ) -> Result<Self, ImportResolutionError> {
        let mut searched = HashSet::new();

        // Try interpret as relative to importing file
        let parent = importing
            .path
            .parent()
            .expect("every absolute path to a file has a parent");
        let relative = parent.join(request_string);
        searched.insert(relative.clone());
        if relative.is_file() {
            let path = relative.canonicalize().unwrap();
            return Ok(Self::from_path(AbsoluteWGSLFilePathBuf::new(path)));
        }

        // Try interpret as relative to source root
        let relative = project_root.join(request_string);
        searched.insert(relative.clone());
        if relative.is_file() {
            let path = relative.canonicalize().unwrap();
            return Ok(Self::from_path(AbsoluteWGSLFilePathBuf::new(path)));
        }

        Err(ImportResolutionError::Unresolved {
            requested: request_string.to_string(),
            importer: importing.to_owned(),
            searched,
        })
    }

    pub(crate) fn processed_source(
        &self,
        module_names: &HashMap<Module, String>,
        project_root: &PathBuf,
    ) -> String {
        let source = self.read_to_string();
        // Replace `@export` directives with equivalent whitespace
        let (source, _) = exports::strip_exports(&source);
        // Replace `#import` names with substitutions
        imports::replace_imports_in_source(&source, self, project_root, module_names)
    }

    pub(crate) fn path(&self) -> AbsoluteWGSLFilePathBuf {
        self.path.clone()
    }

    pub(crate) fn read_to_string(&self) -> String {
        std::fs::read_to_string(&*self.path).unwrap_or_else(|_| {
            panic!(
                "file `{}` exists but could not be read",
                self.path.display()
            )
        })
    }

    /// Gets the name of the file, without the `.wgsl` extension.
    pub(crate) fn file_name(&self) -> String {
        self.path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    pub(crate) fn nth_path_component(&self, i: usize) -> Option<Cow<'_, str>> {
        Some(
            self.path
                .components()
                .rev()
                .nth(i)?
                .as_os_str()
                .to_string_lossy(),
        )
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
