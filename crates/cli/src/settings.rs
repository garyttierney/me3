use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};
use steamlocate::SteamDir;
use tracing::error;

use crate::commands::profile::no_profile_dir;

#[derive(Debug, clap::Args, Serialize, Deserialize, Default)]
#[group(multiple = true)]
pub struct Config {
    /// Enable crash reporting?
    #[clap(long, help_heading = "Configuration")]
    pub(crate) crash_reporting: bool,

    /// Override the path to the me3 profile directory.
    #[clap(long, help_heading = "Configuration", value_hint = clap::ValueHint::DirPath)]
    pub(crate) profile_dir: Option<PathBuf>,

    /// Optional path to a Steam installation, auto-detected if not provided
    #[clap(long, help_heading = "Configuration", value_hint = clap::ValueHint::DirPath)]
    pub(crate) steam_dir: Option<PathBuf>,

    /// Path to PE binaries used by Proton (Linux only)
    #[cfg(target_os = "linux")]
    #[clap(long, help_heading = "Configuration", value_hint = clap::ValueHint::DirPath)]
    pub(crate) windows_binaries_dir: Option<PathBuf>,
}

impl Config {
    pub fn merge(self, other: Self) -> Self {
        Self {
            crash_reporting: other.crash_reporting || self.crash_reporting,
            profile_dir: other.profile_dir.or(self.profile_dir),
            steam_dir: other.steam_dir.or(self.steam_dir),
            #[cfg(target_os = "linux")]
            windows_binaries_dir: other.windows_binaries_dir.or(self.windows_binaries_dir),
        }
    }

    pub fn resolve_steam_dir(&self) -> Result<SteamDir> {
        Ok(self
            .steam_dir
            .as_ref()
            .map(|steam_path| SteamDir::from_dir(steam_path))
            .unwrap_or_else(SteamDir::locate)?)
    }

    pub fn resolve_profile(&self, profile_name: &str) -> Result<PathBuf> {
        if let Ok(true) = std::fs::exists(profile_name) {
            Ok(PathBuf::from(profile_name))
        } else {
            Ok(self
                .profile_dir
                .as_ref()
                .ok_or_else(no_profile_dir)?
                .join(format!("{profile_name}.me3")))
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let encoded_toml = fs::read_to_string(path)?;
        let toml = toml::from_str(&encoded_toml)?;

        Ok(toml)
    }

    pub fn from_files<P: AsRef<Path>>(files: impl IntoIterator<Item = P>) -> Result<Config> {
        let mut config = Config::default();

        for file in files.into_iter() {
            let path = file.as_ref();

            match Config::from_file(path) {
                Ok(item) => config = config.merge(item),
                Err(error) => {
                    error!(?path, ?error, "failed to load configuration")
                }
            }
        }

        Ok(config)
    }
}
