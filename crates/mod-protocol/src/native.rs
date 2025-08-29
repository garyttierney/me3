use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mod_file::{AsModFile, ModFile};

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

    /// An optional symbol to be called after this native successfully loads.
    pub initializer: Option<NativeInitializerCondition>,
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
        }
    }
}

impl From<PathBuf> for Native {
    #[inline]
    fn from(path: PathBuf) -> Self {
        ModFile::from(path).into()
    }
}
