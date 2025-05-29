use clap::*;
use launch::LaunchArgs;
use profile::ProfileCommands;

pub mod info;
pub mod launch;
pub mod profile;

#[derive(Subcommand)]
#[command(flatten_help = true)]
pub enum Commands {
    /// Launch the selected game a collection of mod profiles.
    Launch(LaunchArgs),

    /// Show information on the me3 installation and search paths.
    Info,

    #[clap(subcommand)]
    Profile(ProfileCommands),
}
