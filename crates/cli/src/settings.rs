use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use steamlocate::SteamDir;

use crate::commands::profile::no_profile_dir;

#[derive(Debug, clap::Args, Serialize, Deserialize, Default)]
#[group(multiple = true)]
pub struct Config {
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
            profile_dir: self.profile_dir.or(other.profile_dir),
            steam_dir: self.steam_dir.or(other.steam_dir),
            #[cfg(target_os = "linux")]
            windows_binaries_dir: self.windows_binaries_dir.or(other.windows_binaries_dir),
        }
    }

    pub fn resolve_steam_dir(&self) -> color_eyre::Result<SteamDir> {
        Ok(self
            .steam_dir
            .as_ref()
            .map(|steam_path| SteamDir::from_dir(steam_path))
            .unwrap_or_else(SteamDir::locate)?)
    }

    pub fn resolve_profile(&self, profile_name: &str) -> color_eyre::Result<PathBuf> {
        if let Ok(true) = std::fs::exists(profile_name) {
            Ok(PathBuf::from(profile_name))
        } else {
            Ok(self
                .profile_dir
                .as_ref()
                .ok_or_else(no_profile_dir)?
                .join(format!("{profile_name}.me3-toml")))
        }
    }
}
