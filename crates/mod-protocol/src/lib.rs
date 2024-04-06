use std::{fs::File, path::Path};

use native::Native;
use package::Package;
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};

pub mod dependency;
pub mod native;
pub mod package;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "profileVersion")]
pub enum ModProfile {
    #[serde(rename = "v1")]
    V1(ModProfileV1),
}

impl Default for ModProfile {
    fn default() -> Self {
        ModProfile::V1(ModProfileV1::default())
    }
}

impl ModProfile {
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;

        match path.extension().and_then(|path| path.to_str()) {
            Some("yml" | "yaml") | None => {
                serde_yaml::from_reader(file).map_err(std::io::Error::other)
            }
            Some(format) => Err(std::io::Error::other(format!("{format} is unsupported"))),
        }
    }

    pub fn natives(&mut self) -> Vec<Native> {
        match self {
            ModProfile::V1(v1) => v1.natives.drain(..).collect(),
        }
    }

    pub fn packages(&mut self) -> Vec<Package> {
        match self {
            ModProfile::V1(v1) => v1.packages.drain(..).collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ModProfileV1 {
    /// Native modules (DLLs) that will be loaded.
    #[serde(default)]
    natives: Vec<Native>,

    /// A collection of packages containing assets that should be considered for loading
    /// before the DVDBND.
    #[serde(default)]
    packages: Vec<Package>,
}
