use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    dependency::{Dependency, Dependent},
    mod_file::{AsModFile, ModFile},
};

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Package {
    #[serde(flatten)]
    pub(crate) inner: ModFile,

    #[serde(default)]
    pub(crate) load_before: Vec<Dependent<String>>,

    #[serde(default)]
    pub(crate) load_after: Vec<Dependent<String>>,
}

impl Package {
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ModFile::new(path).into()
    }
}

impl Dependency for Package {
    type UniqueId = String;

    fn id(&self) -> Self::UniqueId {
        self.name.clone()
    }

    fn loads_after(&self) -> &[crate::dependency::Dependent<Self::UniqueId>] {
        &self.load_after
    }

    fn loads_before(&self) -> &[crate::dependency::Dependent<Self::UniqueId>] {
        &self.load_before
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
        &self.inner
    }
}

impl DerefMut for Package {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsModFile for Package {
    #[inline]
    fn as_mod_file(&self) -> &ModFile {
        &self.inner
    }

    #[inline]
    fn as_mod_file_mut(&mut self) -> &mut ModFile {
        &mut self.inner
    }
}

impl From<ModFile> for Package {
    #[inline]
    fn from(item: ModFile) -> Self {
        Self {
            inner: item,
            load_before: vec![],
            load_after: vec![],
        }
    }
}

impl From<PathBuf> for Package {
    #[inline]
    fn from(path: PathBuf) -> Self {
        ModFile::from(path).into()
    }
}
