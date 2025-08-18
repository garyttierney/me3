use clap::*;
use launch::LaunchArgs;
use profile::ProfileCommands;

pub mod info;
pub mod launch;
pub mod profile;

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Subcommand, Debug)]
#[command(flatten_help = true)]
pub enum Commands {
    /// Launch the selected game with mods.
    #[clap(disable_version_flag = true)]
    Launch(LaunchArgs),

    /// Show information on the me3 installation and search paths.
    #[clap(disable_version_flag = true)]
    Info,

    #[clap(subcommand, disable_version_flag = true)]
    Profile(ProfileCommands),

    #[cfg(target_os = "windows")]
    #[clap(hide = true)]
    AddToPath,

    /// Check for and launch a new version of the me3 installer.
    #[cfg(target_os = "windows")]
    Update,
}
