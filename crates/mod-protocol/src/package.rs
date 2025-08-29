use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::mod_file::{AsModFile, ModFile};

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Package(pub(crate) ModFile);

impl Package {
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ModFile::new(path).into()
    }
}

impl AsRef<Path> for Package {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_mod_file().as_ref()
    }
}

impl Deref for Package {
    type Target = ModFile;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Package {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsModFile for Package {
    #[inline]
    fn as_mod_file(&self) -> &ModFile {
        &self.0
    }

    #[inline]
    fn as_mod_file_mut(&mut self) -> &mut ModFile {
        &mut self.0
    }
}

impl From<ModFile> for Package {
    #[inline]
    fn from(item: ModFile) -> Self {
        Self(item)
    }
}

impl From<PathBuf> for Package {
    #[inline]
    fn from(path: PathBuf) -> Self {
        ModFile::from(path).into()
    }
}
