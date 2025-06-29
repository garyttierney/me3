use std::{error::Error, fmt::Display, fs::File, io::Read, path::Path, str::FromStr};
use std::path::PathBuf;
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

/// Chronologically sorted list of games supported by me3.
///
/// Feature gates can use [`Ord`] comparisons between game type constants.
#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum Game {
    #[serde(rename = "sekiro")]
    #[serde(alias = "sdt")]
    Sekiro,

    #[serde(rename = "elden-ring")]
    #[serde(alias = "eldenring")]
    EldenRing,

    #[serde(rename = "armoredcore6")]
    #[serde(alias = "ac6")]
    ArmoredCore6,

    #[serde(rename = "nightreign")]
    #[serde(alias = "nightrein")]
    Nightreign,
}

#[derive(Debug)]
pub struct InvalidGame(String);
impl Error for InvalidGame {}
impl Display for InvalidGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a supported game", self.0)
    }
}

impl FromStr for Game {
    type Err = InvalidGame;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.to_ascii_lowercase().as_str() {
            "eldenring" | "elden-ring" => Ok(Game::EldenRing),
            "nightreign" | "nightrein" => Ok(Game::Nightreign),
            "ac6" | "armoredcore6" => Ok(Game::ArmoredCore6),
            _ => Err(InvalidGame(name.to_string())),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Supports {
    #[serde(rename = "game")]
    pub game: Game,
    
    #[serde(rename = "exe")]
    pub exe: Option<PathBuf>,

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
