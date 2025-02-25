use std::{ffi::OsStr, ops::Deref, path::PathBuf};

/// A PathBuf that is absolute, exists and points to a WGSL file
#[derive(Hash, PartialEq, Eq, Clone)]
pub(crate) struct AbsoluteWGSLFilePathBuf {
    path: PathBuf,
}

impl AbsoluteWGSLFilePathBuf {
    /// Creates a new [`AbsoluteWGSLFilePathBuf`], panicking if any requirements aren't met.
    pub(crate) fn new(path: PathBuf) -> Self {
        assert!(
            path.is_file(),
            "`{}` is not a file - expected a `wgsl` file",
            path.display()
        );
        assert!(path.is_absolute(), "`{}` is not absolute", path.display());
        assert_eq!(
            path.extension(),
            Some(OsStr::new("wgsl")),
            "`{}` does not have a `.wgsl` extension",
            path.display()
        );

        Self { path }
    }
}

impl Deref for AbsoluteWGSLFilePathBuf {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl std::fmt::Debug for AbsoluteWGSLFilePathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}
