use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dependency::{Dependency, Dependent};

pub trait WithPackageSource {
    fn source(&self) -> &ModFile;

    fn source_mut(&mut self) -> &mut ModFile;
}

/// A filesystem path to the contents of a package. May be relative to the [ModProfile] containing
/// it.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct ModFile(pub(crate) PathBuf);

impl Deref for ModFile {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ModFile {
    /// Returns whether or not the package's source description is relative to the mod profile.
    pub fn is_relative(&self) -> bool {
        self.0.is_relative()
    }

    pub fn make_absolute(&mut self, base: &Path) {
        if self.0.is_relative() {
            self.0 = base.join(&self.0);
        }
    }
}

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Package {
    /// The unique identifier for this package..
    pub(crate) id: String,

    /// A path to the source of this package.
    pub(crate) source: ModFile,

    /// A list of package IDs that this package should load after.
    #[serde(default)]
    pub(crate) load_after: Vec<Dependent<String>>,

    /// A list of packages that this package should load before.
    #[serde(default)]
    pub(crate) load_before: Vec<Dependent<String>>,
}

impl Package {
    pub fn new(path: PathBuf) -> Self {
        Self {
            id: path
                .file_name()
                .expect("no name for this package")
                .to_string_lossy()
                .into(),
            source: ModFile(path),
            load_after: vec![],
            load_before: vec![],
        }
    }

    /// Makes the package's source absolute using a given base directory (this is usually the mod
    /// profile's parent path).
    pub fn make_absolute(&mut self, base: &Path) {
        self.source = ModFile(base.join(&self.source.0));
    }
}

impl WithPackageSource for Package {
    fn source(&self) -> &ModFile {
        &self.source
    }

    fn source_mut(&mut self) -> &mut ModFile {
        &mut self.source
    }
}

impl Dependency for Package {
    type UniqueId = String;

    fn id(&self) -> Self::UniqueId {
        self.id.clone()
    }

    fn loads_after(&self) -> &[crate::dependency::Dependent<Self::UniqueId>] {
        &self.load_after
    }

    fn loads_before(&self) -> &[crate::dependency::Dependent<Self::UniqueId>] {
        &self.load_before
    }
}

pub trait AssetOverrideSource {
    fn asset_path(&self) -> &Path;
}

impl AssetOverrideSource for &Package {
    fn asset_path(&self) -> &Path {
        self.source.0.as_path()
    }
}
