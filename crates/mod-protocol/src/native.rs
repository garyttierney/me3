use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    dependency::{Dependency, Dependent},
    mod_file::{AsModFile, ModFile},
};

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct NativeInitializerDelay {
    pub ms: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct NativeInitializerCondition {
    #[serde(default)]
    pub delay: Option<NativeInitializerDelay>,
    #[serde(default)]
    pub function: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Native {
    #[serde(flatten)]
    pub(crate) inner: ModFile,

    pub initializer: Option<NativeInitializerCondition>,

    #[serde(default)]
    pub(crate) load_before: Vec<Dependent<String>>,

    #[serde(default)]
    pub(crate) load_after: Vec<Dependent<String>>,
}

impl Native {
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ModFile::new(path).into()
    }

    #[inline]
    pub fn is_default(&self) -> bool {
        self.inner.is_default() && self.initializer.is_none()
    }
}

impl Dependency for Native {
    type UniqueId = String;

    fn id(&self) -> Self::UniqueId {
        self.path
            .file_name()
            .map(|f| f.to_string_lossy().to_ascii_lowercase())
            .expect("native had no file name")
    }

    fn loads_after(&self) -> &[Dependent<Self::UniqueId>] {
        &self.load_after
    }

    fn loads_before(&self) -> &[Dependent<Self::UniqueId>] {
        &self.load_before
    }
}

impl AsRef<Path> for Native {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_mod_file().as_ref()
    }
}

impl Deref for Native {
    type Target = ModFile;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Native {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsModFile for Native {
    #[inline]
    fn as_mod_file(&self) -> &ModFile {
        &self.inner
    }

    #[inline]
    fn as_mod_file_mut(&mut self) -> &mut ModFile {
        &mut self.inner
    }
}

impl From<ModFile> for Native {
    #[inline]
    fn from(item: ModFile) -> Self {
        Self {
            inner: item,
            initializer: None,
            load_before: vec![],
            load_after: vec![],
        }
    }
}

impl From<PathBuf> for Native {
    #[inline]
    fn from(path: PathBuf) -> Self {
        ModFile::from(path).into()
    }
}
