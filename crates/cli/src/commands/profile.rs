use std::{
    error::{self, Error},
    fs,
    path::PathBuf,
};

use clap::{ArgAction, Args, Subcommand};
use color_eyre::eyre::{eyre, OptionExt};
use me3_mod_protocol::{
    dependency::Dependency,
    native::Native,
    package::{Package, WithPackageSource},
    ModProfile, Supports,
};
use native_dialog::DialogBuilder;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use tracing::{debug, error, info, warn};

use crate::{output::OutputBuilder, Config, CreateProfileDialog, Game, ModItem};

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

    #[clap(short('i'), long("interactive"), action = ArgAction::SetTrue)]
    interactive: bool,

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

    /// Optional flag to treat the input as a filename instead of a profile ID to store in
    /// ME3_PROFILE_DIR.
    #[clap(short, long, action = ArgAction::SetTrue)]
    file: bool,

    /// Overwrite the profile if it already exists
    #[clap(long, action = ArgAction::SetTrue)]
    overwrite: bool,
}

#[tracing::instrument]
pub fn list(config: Config) -> color_eyre::Result<()> {
    let profile_dir = config.profile_dir.ok_or_else(no_profile_dir)?;

    debug!("searching in {profile_dir:?} for profiles");

    if !fs::exists(&profile_dir)? {
        debug!("profile dir doesn't exist, no profiles");
        return Ok(());
    }

    for profile_entry in std::fs::read_dir(profile_dir)? {
        match profile_entry {
            Ok(profile) => println!("{}", profile.file_name().to_string_lossy()),
            Err(e) => warn!(?e, "unable to read entry"),
        }
    }

    Ok(())
}

#[tracing::instrument]
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

    if args.interactive {
        let mut dialog = CreateProfileDialog::new()?;
        let mut mods = slint::VecModel::default();

        for native in natives {
            mods.push(ModItem {
                is_dll: true,
                path: native.path.to_str().unwrap().into(),
            });
        }

        let model = ModelRc::new(mods);
        dialog.set_mods(model);
        let dialog_handle = dialog.as_weak();

        dialog.on_add_new_mod(move || {
            let Ok(Some(mod_folder)) = DialogBuilder::file()
                .set_title("Select a folder containing modded files or DLLs")
                .open_single_dir()
                .show()
            else {
                return;
            };

            let natives: Vec<PathBuf> = fs::read_dir(&mod_folder)
                .unwrap()
                .filter_map(|path| {
                    let entry = path.ok()?;
                    let path = entry.path();
                    let is_dll = path.extension()? == "dll";

                    is_dll.then_some(path)
                })
                .collect();

            if let Some(model) = dialog_handle
                .upgrade()
                .unwrap()
                .get_mods()
                .as_any()
                .downcast_ref::<VecModel<ModItem>>()
            {
                for native in natives {
                    model.push(ModItem {
                        path: native.to_str().unwrap().into(),
                        is_dll: true,
                    });
                }

                model.push(ModItem {
                    path: mod_folder.to_str().unwrap().into(),
                    is_dll: false,
                });
            }
        });

        let _ = dialog.run()?;
    }

    let contents = toml::to_string_pretty(&profile)?;
    let editor = std::env::var("VISUAL").ok().unwrap_or("edit".into());
    std::fs::write(&profile_path, contents)?;
    open::with_detached(&profile_path, editor)?;
    Ok(())
}

pub fn show(config: Config, name: String) -> color_eyre::Result<()> {
    let profile_path = config.resolve_profile(&name)?;

    if !std::fs::exists(&profile_path)? {
        return Err(eyre!("No profile found with this name"));
    }

    let profile = ModProfile::from_file(&profile_path)?;
    let mut output = OutputBuilder::new("Mod Profile");

    output.property("Name", name.clone());
    output.property("Path", profile_path.to_string_lossy());

    output.section("Supports", |builder| {
        for supports in profile.supports() {
            builder.property(format!("{:?}", supports.game), "Supported");
        }
    });

    output.section("Natives", |builder| {
        for native in profile.natives() {
            builder.section(native.id(), |builder| {
                builder.indent(2);

                builder.property("Path", native.source().to_string_lossy());
                builder.property("Optional", native.optional.to_string())
            });
        }
    });

    output.section("Packages", |builder| {
        for package in profile.packages() {
            builder.section(package.id(), |builder| {
                builder.indent(2);
                builder.property("Path", package.source().to_string_lossy());
            });
        }
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
