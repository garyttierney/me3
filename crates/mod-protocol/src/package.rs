use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dependency::{Dependency, Dependent};

/// A filesystem path to the contents of a package. May be relative to the [ModProfile] containing
/// it.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct PackageSource(pub(crate) PathBuf);

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Package {
    /// The unique identifier for this package..
    pub(crate) id: String,

    /// A path to the source of this package.
    pub(crate) source: PackageSource,

    /// A list of package IDs that this package should load after.
    #[serde(default)]
    pub(crate) load_after: Vec<Dependent<String>>,

    /// A list of packages that this package should load before.
    #[serde(default)]
    pub(crate) load_before: Vec<Dependent<String>>,
}

impl Package {
    /// Returns whether or not the package's source description is relative to the mod profile.
    pub fn is_relative(&self) -> bool {
        self.source.0.is_relative()
    }

    /// Makes the package's source absolute using a given base directory (this is usually the mod
    /// profile's parent path).
    pub fn make_absolute(&mut self, base: &Path) {
        self.source = PackageSource(base.join(&self.source.0));
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
