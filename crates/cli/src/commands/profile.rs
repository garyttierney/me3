use std::{fs, path::PathBuf};

use clap::{ArgAction, Args, Subcommand};
use color_eyre::eyre::{eyre, OptionExt};
use me3_mod_protocol::{
    dependency::Dependency,
    native::Native,
    package::{Package, WithPackageSource},
    ModProfile, Supports,
};
use tracing::{error, warn};

use crate::{config::Config, db::DbContext, output::OutputBuilder, Game};

#[derive(Subcommand, Debug)]
#[command(flatten_help = true)]
pub enum ProfileCommands {
    /// Create a new profile with the given name.
    Create(ProfileCreateArgs),

    /// List all profiles stored in the ME3_PROFILE_DIR.
    List,

    /// Show information on a profile identified by a name.
    #[clap(name = "show")]
    Show { name: String },
}

#[derive(Args, Debug)]
pub struct ProfileCreateArgs {
    /// Name of the profile.
    name: String,

    /// An optional game to associate with this profile for one-click launches.
    #[clap(
        short('g'),
        long,
        hide_possible_values = false,
        help_heading = "Game selection"
    )]
    #[arg(value_enum)]
    game: Option<Game>,

    /// Path to a list of packages to add to the profile.
    #[clap(long("package"))]
    packages: Vec<PathBuf>,

    /// Path to a list of native DLLs to add to the profile.
    #[clap(long("native"))]
    natives: Vec<PathBuf>,

    #[clap(flatten)]
    options: ProfileOptions,

    /// Optional flag to treat the input as a filename instead of a profile ID to store in
    /// ME3_PROFILE_DIR.
    #[clap(short, long, action = ArgAction::SetTrue)]
    file: bool,

    /// Overwrite the profile if it already exists
    #[clap(long, action = ArgAction::SetTrue)]
    overwrite: bool,
}

#[derive(Args, Clone, Debug, Default, PartialEq)]
pub struct ProfileOptions {
    /// Allow the game to connect to the multiplayer server?
    #[clap(long("online"), default_missing_value = "true", num_args=0..=1)]
    pub start_online: Option<bool>,
}

impl ProfileOptions {
    pub fn merge(self, other: Self) -> Self {
        Self {
            start_online: other.start_online.or(self.start_online),
        }
    }
}

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip(config))]
pub fn create(config: Config, args: ProfileCreateArgs) -> color_eyre::Result<()> {
    let profile_path = if args.file {
        PathBuf::from(args.name)
    } else {
        config.resolve_profile(&args.name)?
    };

    if std::fs::exists(&profile_path).is_ok_and(|exists| exists) && !args.overwrite {
        error!("profile already exists, use --overwrite to ignore this error");
        return Ok(());
    }

    let profile_dir = profile_path
        .parent()
        .ok_or_eyre("profile parent path was removed")?;
    fs::create_dir_all(profile_dir)?;

    let mut profile = ModProfile::default();

    if let Some(game) = args.game {
        let supports = profile.supports_mut();

        supports.push(Supports {
            game: game.into(),
            since_version: None,
        });
    }

    let packages = profile.packages_mut();
    for pkg in args.packages {
        let full_path = if pkg.is_absolute() || std::fs::exists(&pkg)? {
            pkg
        } else {
            profile_dir.join(pkg)
        };

        if !std::fs::exists(&full_path)? {
            std::fs::create_dir_all(&full_path)?;
        }

        packages.push(Package::new(full_path));
    }

    let natives = profile.natives_mut();
    for pkg in args.natives {
        natives.push(Native::new(pkg));
    }

    let start_online = profile.start_online_mut();
    *start_online = args.options.start_online;

    let contents = toml::to_string_pretty(&profile)?;

    std::fs::write(profile_path, contents)?;

    Ok(())
}

pub fn show(db: DbContext, name: String) -> color_eyre::Result<()> {
    let profile = db.profiles.load(name)?;
    let mut output = OutputBuilder::new("Mod Profile");

    output.property("Name", profile.name());
    output.property("Path", profile.base_dir().to_string_lossy());

    output.section("Supports", |builder| {
        if let Some(game) = profile.supported_game() {
            builder.property(format!("{game:?}"), "Supported");
        }
    });

    output.section("Natives", |builder| {
        for native in profile.natives() {
            builder.section(native.id(), |builder| {
                builder.indent(2);

                builder.property("Path", native.source().to_string_lossy());
                builder.property("Optional", native.optional.to_string());
                builder.property("Enabled", native.enabled);
            });
        }
    });

    output.section("Packages", |builder| {
        for package in profile.packages() {
            builder.section(package.id(), |builder| {
                builder.indent(2);
                builder.property("Path", package.source().to_string_lossy());
                builder.property("Enabled", package.enabled);
            });
        }
    });

    output.section("Options", |builder| {
        let opt_to_str =
            |o: Option<bool>| o.map(|o| o.to_string()).unwrap_or_else(|| "-".to_owned());

        let options = profile.options();
        builder.property("Start Online", opt_to_str(options.start_online));
    });

    println!("{}", output.build());

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
