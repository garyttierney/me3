use std::{fs::File, io::Read, path::Path};

use native::Native;
use package::Package;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_derive::Serialize;

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
        let mut file = File::open(path)?;

        match path.extension().and_then(|path| path.to_str()) {
            Some("toml") | None => {
                let mut file_contents = String::new();
                let _ = file.read_to_string(&mut file_contents)?;

                toml::from_str(file_contents.as_str()).map_err(std::io::Error::other)
            }
            Some("yml" | "yaml") => serde_yaml::from_reader(file).map_err(std::io::Error::other),
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

#[cfg(test)]
mod tests {
    use std::fmt::format;

    use expect_test::expect_file;

    use super::*;

    fn check(test_case_name: &str) {
        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let test_case = test_data_dir.join(test_case_name);
        let test_snapshot = test_data_dir.join(format!("{}.expected", test_case_name));

        let actual_profile = ModProfile::from_file(&test_case).expect("parse failure");
        let expected_profile = expect_file![test_snapshot];

        expected_profile.assert_debug_eq(&actual_profile);
    }

    #[test]
    fn basic_config_toml() {
        check("basic_config.me3.toml");
    }

    #[test]
    fn basic_config_yaml() {
        check("basic_config.me3.yaml");
    }
}
