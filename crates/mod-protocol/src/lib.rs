use std::{fs::File, io::Read, path::Path};

use native::Native;
use package::Package;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod debug_properties;
pub mod dependency;
pub mod game;
pub mod native;
pub mod package;

use debug_properties::DebugProperties;
pub use game::Game;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "profileVersion")]
pub enum ModProfile {
    #[serde(rename = "v1")]
    V1(ModProfileV1),
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
            Some("toml") | Some("me3") | None => {
                let mut file_contents = String::new();
                let _ = file.read_to_string(&mut file_contents)?;

                toml::from_str(file_contents.as_str()).map_err(std::io::Error::other)
            }
            Some("json") => serde_json::from_reader(file).map_err(std::io::Error::other),
            Some(format) => Err(std::io::Error::other(format!("{format} is unsupported"))),
        }
    }

    pub fn natives_mut(&mut self) -> &mut Vec<Native> {
        match self {
            ModProfile::V1(v1) => &mut v1.natives,
        }
    }

    pub fn packages_mut(&mut self) -> &mut Vec<Package> {
        match self {
            ModProfile::V1(v1) => &mut v1.packages,
        }
    }

    pub fn supports_mut(&mut self) -> &mut Vec<Supports> {
        match self {
            ModProfile::V1(v1) => &mut v1.supports,
        }
    }

    pub fn start_online_mut(&mut self) -> &mut Option<bool> {
        match self {
            ModProfile::V1(v1) => &mut v1.start_online,
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

    pub fn savefile(&self) -> Option<String> {
        match self {
            ModProfile::V1(v1) => v1.savefile.clone(),
        }
    }

    pub fn start_online(&self) -> Option<bool> {
        match self {
            ModProfile::V1(v1) => v1.start_online,
        }
    }

    pub fn disable_arxan(&self) -> Option<bool> {
        match self {
            ModProfile::V1(v1) => v1.disable_arxan,
        }
    }

    pub fn mem_patch(&self) -> Option<bool> {
        match self {
            ModProfile::V1(v1) => v1.mem_patch,
        }
    }

    pub fn mem_patch_heap_size(&self) -> Option<u32> {
        match self {
            ModProfile::V1(v1) => v1.mem_patch_heap_size,
        }
    }

    pub fn debug_properties(&self) -> Vec<(String, String)> {
        match self {
            ModProfile::V1(v1) => v1
                .debug_properties
                .props
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
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
    #[serde(alias = "native")]
    natives: Vec<Native>,

    /// A collection of packages containing assets that should be considered for loading
    /// before the DVDBND.
    #[serde(default)]
    #[serde(alias = "package")]
    packages: Vec<Package>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[serde(default)]
    savefile: Option<String>,

    /// Starts the game with multiplayer server connectivity enabled.
    #[serde(default)]
    start_online: Option<bool>,

    /// Try to neutralize Arxan GuardIT code protection to improve mod stability.
    #[serde(default)]
    disable_arxan: Option<bool>,

    /// Patch memory limits for supported games to improve mod stability.
    #[serde(default)]
    #[serde(alias = "patch_mem")]
    mem_patch: Option<bool>,

    /// Override how many megabytes of memory the supported game should allocate
    /// (with `mem_patch = true`).
    #[serde(default)]
    mem_patch_heap_size: Option<u32>,

    /// Debug game property overrides.
    #[serde(default)]
    debug_properties: DebugProperties,
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

    #[test]
    fn plural_packages_name() {
        check("plural_packages.me3");
    }

    #[test]
    fn singular_packages_name() {
        check("singular_package.me3");
    }

    #[test]
    fn debug_properties() {
        check("debug_properties.me3");
    }
}
