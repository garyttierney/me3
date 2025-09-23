use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    mod_file::ModFile,
    native::Native,
    package::Package,
    profile::{
        builder::ModProfileBuilder,
        v1::{ModProfileV1, Supports},
        v2::ModProfileV2,
    },
    Game,
};

pub mod builder;
mod v1;
mod v2;

pub type Profile = ModFile;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "profileVersion")]
pub enum ModProfile {
    #[serde(skip_serializing, rename = "v1")]
    V1(ModProfileV1),
    #[serde(rename = "v2")]
    V2(ModProfileV2),
}

impl Default for ModProfile {
    fn default() -> Self {
        ModProfile::V2(ModProfileV2::default())
    }
}

#[derive(Debug, Error)]
pub enum ProfileMergeError {
    #[error("profiles do not support the same games")]
    MismatchedSupports,
}

impl ModProfile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let path = path.as_ref();
        let mut file = File::open(path)?;

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") | Some("me3") => {
                let mut file_contents = String::new();
                let _ = file.read_to_string(&mut file_contents)?;

                match path
                    .file_stem()
                    .and_then(|stem| Path::new(stem).extension())
                    .and_then(|ext| ext.to_str())
                {
                    Some("json") => serde_json::from_str(&file_contents).map_err(io::Error::other),
                    _ => toml::from_str(&file_contents).map_err(io::Error::other),
                }
            }
            Some("json") => serde_json::from_reader(file).map_err(io::Error::other),
            ext => Err(io::Error::other(format!(
                "\"{}\" is unsupported",
                ext.unwrap_or("no file extension")
            ))),
        }
    }

    pub fn try_merge(&self, other: &Self) -> Result<ModProfile, ProfileMergeError> {
        let my_supports = self.supports();
        let other_supports = other.supports();

        let game = if !my_supports.is_empty() && !other_supports.is_empty() {
            if let Some(supports) = other_supports.iter().find(|s| my_supports.contains(s)) {
                Some(supports)
            } else {
                return Err(ProfileMergeError::MismatchedSupports);
            }
        } else {
            other_supports.first().or(my_supports.first())
        };

        let either = |a: Option<bool>, b: Option<bool>| match (a, b) {
            (Some(true), _) => Some(true),
            (_, Some(true)) => Some(true),
            _ => a.or(b),
        };

        let profile = ModProfileBuilder::new()
            .with_supported_game(game.cloned())
            .with_savefile(self.savefile())
            .with_mods(self.natives().into_iter().chain(other.natives()))
            .with_mods(self.packages().into_iter().chain(other.packages()))
            .with_mods(self.profiles().into_iter().chain(other.profiles()))
            .start_online(either(other.start_online(), self.start_online()))
            .disable_arxan(either(other.disable_arxan(), self.disable_arxan()))
            .build();

        Ok(profile)
    }

    pub fn game(&self) -> Option<Game> {
        match self {
            ModProfile::V1(v1) => match &v1.supports[..] {
                [Supports { game, .. }] => Some(*game),
                _ => None,
            },
            ModProfile::V2(v2) => v2.supports,
        }
    }

    pub fn supports(&self) -> Vec<Game> {
        match self {
            ModProfile::V1(v1) => v1.supports.iter().map(|s| s.game).collect(),
            ModProfile::V2(v2) => v2.supports.iter().cloned().collect(),
        }
    }

    pub fn natives(&self) -> Vec<Native> {
        match self {
            ModProfile::V1(v1) => v1.natives.clone(),
            ModProfile::V2(v2) => v2.natives.clone(),
        }
    }

    pub fn packages(&self) -> Vec<Package> {
        match self {
            ModProfile::V1(v1) => v1.packages.clone(),
            ModProfile::V2(v2) => v2.packages.clone(),
        }
    }

    pub fn profiles(&self) -> Vec<ModFile> {
        match self {
            ModProfile::V1(_) => vec![],
            ModProfile::V2(v2) => v2.profiles.clone(),
        }
    }

    pub fn savefile(&self) -> Option<String> {
        match self {
            ModProfile::V1(v1) => v1.savefile.clone(),
            ModProfile::V2(v2) => v2.savefile.clone(),
        }
    }

    pub fn start_online(&self) -> Option<bool> {
        match self {
            ModProfile::V1(v1) => v1.start_online,
            ModProfile::V2(v2) => v2.start_online,
        }
    }

    pub fn disable_arxan(&self) -> Option<bool> {
        match self {
            ModProfile::V1(v1) => v1.disable_arxan,
            ModProfile::V2(v2) => v2.disable_arxan,
        }
    }
}

impl AsRef<ModProfile> for ModProfile {
    fn as_ref(&self) -> &ModProfile {
        self
    }
}
