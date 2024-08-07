use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dependency::{Dependency, Dependent};

/// A filesystem path to the contents of a package. May be relative to the [ModProfile] containing
/// it.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PackageSource(pub PathBuf);

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Package {
    /// The unique identifier for this package..
    id: String,

    /// A path to the source of this package.
    pub source: PackageSource,

    /// A list of package IDs that this package should load after.
    #[serde(default)]
    load_after: Vec<Dependent<String>>,

    /// A list of packages that this package should load before.
    #[serde(default)]
    load_before: Vec<Dependent<String>>,
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
