use std::{fs::File, io::Read, path::Path};

use native::Native;
use package::Package;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod dependency;
pub mod native;
pub mod package;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "profileVersion")]
pub enum ModProfile {
    #[serde(rename = "v1")]
    V1(ModProfileV1),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum Game {
    #[serde(rename = "elden-ring")]
    EldenRing,

    #[serde(rename = "nightrein")]
    Nightreign,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Supports {
    #[serde(rename = "game")]
    pub game: Game,

    #[serde(rename = "since")]
    pub since_version: Option<String>,
}

impl Default for ModProfile {
    fn default() -> Self {
        ModProfile::V1(ModProfileV1::default())
    }
}

impl ModProfile {
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;

        match path.extension().and_then(|path| path.to_str()) {
            Some("toml") | Some("me3-toml") | None => {
                let mut file_contents = String::new();
                let _ = file.read_to_string(&mut file_contents)?;

                toml::from_str(file_contents.as_str()).map_err(std::io::Error::other)
            }
            Some("json") => serde_json::from_reader(file).map_err(std::io::Error::other),
            Some(format) => Err(std::io::Error::other(format!("{format} is unsupported"))),
        }
    }

    pub fn supports_mut(&mut self) -> &mut Vec<Supports> {
        match self {
            ModProfile::V1(v1) => &mut v1.supports,
        }
    }

    pub fn supports(&self) -> Vec<Supports> {
        match self {
            ModProfile::V1(v1) => v1.supports.to_vec(),
        }
    }

    pub fn natives(&self) -> Vec<Native> {
        match self {
            ModProfile::V1(v1) => v1.natives.to_vec(),
        }
    }

    pub fn packages(&self) -> Vec<Package> {
        match self {
            ModProfile::V1(v1) => v1.packages.to_vec(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ModProfileV1 {
    /// The games that this profile supports.
    #[serde(default)]
    supports: Vec<Supports>,

    /// Native modules (DLLs) that will be loaded.
    #[serde(default)]
    natives: Vec<Native>,

    /// A collection of packages containing assets that should be considered for loading
    /// before the DVDBND.
    #[serde(default)]
    packages: Vec<Package>,
}

#[cfg(test)]
mod tests {
    use expect_test::expect_file;

    use super::*;

    fn check(test_case_name: &str) {
        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let test_case = test_data_dir.join(test_case_name);
        let test_snapshot = test_data_dir.join(format!("{test_case_name}.expected"));

        let actual_profile = ModProfile::from_file(&test_case).expect("parse failure");
        let expected_profile = expect_file![test_snapshot];

        expected_profile.assert_debug_eq(&actual_profile);
    }

    #[test]
    fn basic_config_toml() {
        check("basic_config.me3.toml");
    }
}
