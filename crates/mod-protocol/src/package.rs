use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A filesystem path to the contents of a package. May be relative to the [ModProfile] containing it.
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct PackageSource(PathBuf);

/// A package is a source for files that override files within the existing games DVDBND archives.
/// It points to a local path containing assets matching the hierarchy they would be served under in
/// the DVDBND.
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct Package {
    /// The unique identifier for this package..
    id: String,

    /// A path to the source of this package.
    source: PackageSource,

    /// A list of package IDs that this package should load after.
    #[serde(default)]
    load_after: Vec<String>,

    /// A list of packages that this package should load before.
    #[serde(default)]
    load_before: Vec<String>,
}
