use std::{fs, path::PathBuf};

use clap::{ArgAction, Args, Subcommand};
use color_eyre::eyre::{eyre, OptionExt};
use me3_mod_protocol::profile::{builder::ModProfileBuilder, ModProfile};
use tracing::{error, info, warn};

use crate::{config::Config, db::DbContext, output::OutputBuilder, Game};

#[derive(Subcommand, Debug)]
#[command(flatten_help = true)]
pub enum ProfileCommands {
    /// Create a new ModProfile.
    Create(ProfileCreateArgs),

    /// List profiles in the profile dir.
    #[clap(disable_help_flag = true)]
    List,

    /// Show information on a profile.
    Show(ProfileNameArgs),

    /// Upgrade a profile to the latest profile version.
    Upgrade(ProfileNameArgs),
}

#[derive(Args, Debug)]
pub struct ProfileCreateArgs {
    #[clap(flatten)]
    name: ProfileNameArgs,

    /// Game to associate with this profile for one-click launches.
    #[clap(
        short('g'),
        long,
        hide_possible_values = false,
        help_heading = "Game selection"
    )]
    #[arg(value_enum)]
    game: Option<Game>,

    /// Path to a native DLL, package, file or profile [repeatable option]
    #[clap(short('u'), long("use"))]
    uses: Vec<PathBuf>,

    /// (DEPRECATED, use "-u") Path to package directory (asset override mod) [repeatable option]
    #[deprecated]
    #[clap(long("native"))]
    natives: Vec<PathBuf>,

    /// (DEPRECATED, use "-u") Path to DLL file (native DLL mod) [repeatable option]
    #[deprecated]
    #[clap(long("package"))]
    packages: Vec<PathBuf>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[clap(long("savefile"))]
    savefile: Option<String>,

    #[clap(flatten)]
    options: ProfileOptions,

    /// Overwrite the profile if it already exists.
    #[clap(long, action = ArgAction::SetTrue)]
    overwrite: bool,
}

#[derive(Args, Clone, Debug, Default, PartialEq)]
pub struct ProfileOptions {
    /// Re-enable online matchmaking (ban risk)?
    ///
    /// Supported games are blocked from matchmaking servers by default to prevent accidental
    /// online play with invalid (modded) data. Setting this option to true disables this
    /// protection.
    #[clap(long("online"), default_missing_value = "true", num_args=0..=1)]
    pub start_online: Option<bool>,

    /// Neutralize Arxan/GuardIT code protection?
    ///
    /// Arxan/GuardIT is a code tampering protection solution applied to most FromSoftware PC
    /// games. Neutralizing it may help with stability of some mods that patch game executables and
    /// allows for debugging the games without crashing.
    #[clap(long("disable-arxan"), default_missing_value = "true", num_args=0..=1)]
    pub disable_arxan: Option<bool>,
}

impl ProfileOptions {
    pub fn merge(self, other: Self) -> Self {
        Self {
            start_online: other.start_online.or(self.start_online),
            disable_arxan: match (other.disable_arxan, self.disable_arxan) {
                (Some(true), _) => Some(true),
                (_, Some(true)) => Some(true),
                (a, b) => a.or(b),
            },
        }
    }
}

#[derive(Args, Debug)]
pub struct ProfileNameArgs {
    /// Name of the profile or its path (use with --file).
    name: String,

    /// Optional flag to treat the input as a filename instead of a profile name.
    #[clap(short, long, action = ArgAction::SetTrue)]
    file: bool,
}

impl ProfileNameArgs {
    fn into_profile_path(self, config: &Config) -> color_eyre::Result<PathBuf> {
        if self.file {
            Ok(PathBuf::from(self.name))
        } else {
            config.resolve_profile(&self.name)
        }
    }
}

#[tracing::instrument(err, skip_all)]
pub fn list(db: DbContext) -> color_eyre::Result<()> {
    for profile_entry in db.profiles.list() {
        let profile_name = profile_entry
            .file_stem()
            .map(|stem| stem.to_owned())
            .expect("must have a filename");

        println!("{}", profile_name.to_string_lossy());
    }

    Ok(())
}

#[tracing::instrument(err, skip_all)]
pub fn create(config: Config, args: ProfileCreateArgs) -> color_eyre::Result<()> {
    let profile_path = args.name.into_profile_path(&config)?;

    if std::fs::exists(&profile_path).is_ok_and(|exists| exists) && !args.overwrite {
        error!("profile already exists, use --overwrite to ignore this error");
        return Ok(());
    }

    let profile_dir = profile_path
        .parent()
        .ok_or_eyre("profile parent path was removed")?;
    fs::create_dir_all(profile_dir)?;

    #[allow(deprecated)]
    if !args.natives.is_empty() {
        warn!("option \"--native\" is deprecated, use \"--use\" instead!");
    }

    #[allow(deprecated)]
    if !args.packages.is_empty() {
        warn!("option \"--package\" is deprecated, use \"--use\" instead!");
    }

    #[allow(deprecated)]
    ModProfileBuilder::new()
        .with_supported_game(args.game.map(Into::into))
        .with_paths(args.uses)
        .with_paths(args.natives)
        .with_paths(args.packages)
        .with_savefile(args.savefile)
        .start_online(args.options.start_online)
        .disable_arxan(args.options.disable_arxan)
        .write(profile_path)?;

    Ok(())
}

#[tracing::instrument(err, skip_all)]
pub fn show(db: DbContext, config: Config, args: ProfileNameArgs) -> color_eyre::Result<()> {
    let profile_path = args.into_profile_path(&config)?;

    let profile = db.profiles.load(profile_path)?;
    let mut output = OutputBuilder::new("Mod Profile");

    output.property("Name", profile.name());

    output.property(
        "Path",
        match profile.base_dir() {
            Some(dir) => dir.to_string_lossy(),
            None => std::borrow::Cow::Borrowed("-"),
        },
    );

    if let Some(savefile) = profile.savefile() {
        output.property("Save", savefile);
    }

    output.section("Supports", |builder| {
        if let Some(game) = profile.supported_game() {
            builder.property(format!("{game:?}"), "Supported");
        }
    });

    output.section("Natives", |builder| {
        for native in profile.natives() {
            builder.section(&native.name, |builder| {
                builder.indent(2);
                builder.property("Path", native.path.to_string_lossy());
                builder.property("Optional", native.optional.to_string());
                builder.property("Enabled", native.enabled);
            });
        }
    });

    output.section("Packages", |builder| {
        for package in profile.packages() {
            builder.section(&package.name, |builder| {
                builder.indent(2);
                builder.property("Path", package.path.to_string_lossy());
                builder.property("Optional", package.optional.to_string());
                builder.property("Enabled", package.enabled);
            });
        }
    });

    output.section("Profiles", |builder| {
        for profile in profile.profiles() {
            builder.section(&profile.name, |builder| {
                builder.indent(2);
                builder.property("Path", profile.path.to_string_lossy());
                builder.property("Optional", profile.optional.to_string());
                builder.property("Enabled", profile.enabled);
            });
        }
    });

    output.section("Options", |builder| {
        let opt_to_str =
            |o: Option<bool>| o.map(|o| o.to_string()).unwrap_or_else(|| "-".to_owned());

        let options = profile.options();
        builder.property("Start Online", opt_to_str(options.start_online));
        builder.property("Neutralize Arxan", opt_to_str(options.disable_arxan));
    });

    println!("{}", output.build());

    Ok(())
}

#[tracing::instrument(err, skip_all)]
pub fn upgrade(db: DbContext, config: Config, args: ProfileNameArgs) -> color_eyre::Result<()> {
    let profile = args
        .into_profile_path(&config)
        .and_then(|path| db.profiles.load(path))?;

    if matches!(profile.as_ref(), ModProfile::V2(_)) {
        info!("Profile is already using the latest profile version.");
        return Ok(());
    }

    let profile_path = profile.path();

    let mut backup_path = profile_path.to_owned();
    backup_path.as_mut_os_string().push(".bak");

    fs::copy(profile.path(), &backup_path)?;

    ModProfileBuilder::new()
        .with_supported_game(profile.supported_game())
        .with_dependencies(profile.natives())
        .with_dependencies(profile.packages())
        .with_dependencies(profile.profiles())
        .with_savefile(profile.savefile())
        .start_online(profile.options().start_online)
        .disable_arxan(profile.options().disable_arxan)
        .write(profile_path)?;

    info!("Successfully upgraded {profile_path:?} (wrote backup to {backup_path:?}).");

    Ok(())
}

pub fn no_profile_dir() -> color_eyre::Report {
    eyre!(
        r#"No profile directory was configured and the default profile directory was inaccessible.

        To set a profile directory either provide `--profile-dir` on the command line or set `profile_dir`
        in a me3 configuration file. Use `me3 info` to find out where me3 searches for your configuration files.
    "#
    )
}
