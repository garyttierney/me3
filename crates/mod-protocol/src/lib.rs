use std::{fs::File, io::Read, path::Path};

use native::Native;
use package::Package;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_derive::Serialize;

pub mod native;
pub mod package;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "profileVersion")]
pub enum ModProfile {
    #[serde(rename = "v1")]
    V1(ModProfileV1),
}

impl ModProfile {
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;

        match path.extension().and_then(|path| path.to_str()) {
            Some("toml") | None => {
                let mut file_contents = String::new();
                let _ = file.read_to_string(&mut file_contents)?;

                toml::from_str(file_contents.as_str())
                    .map_err(std::io::Error::other)
            }
            Some("yml" | "yaml") => {
                serde_yaml::from_reader(file).map_err(std::io::Error::other)
            }
            Some(format) => Err(std::io::Error::other(format!("{format} is unsupported"))),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ModProfileV1 {
    /// Native modules (DLLs) that will be loaded.
    #[serde(default)]
    natives: Vec<Native>,

    /// A collection of packages containing assets that should be considered for loading
    /// before the DVDBND.
    #[serde(default)]
    packages: Vec<Package>,
}
