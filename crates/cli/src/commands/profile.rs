use std::{fs, path::PathBuf};

use clap::{ArgAction, Args, Subcommand};
use color_eyre::eyre::{eyre, OptionExt};
use me3_mod_protocol::{
    dependency::Dependency,
    native::Native,
    package::{Package, WithPackageSource},
    ModProfile, Supports,
};
use tracing::error;

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
    Show(#[clap(flatten)] ProfileNameArgs),
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

    /// Path to package directory (asset override mod) [repeatable option]
    #[clap(long("package"))]
    packages: Vec<PathBuf>,

    /// Path to DLL file (native DLL mod) [repeatable option]
    #[clap(short('n'), long("native"))]
    natives: Vec<PathBuf>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[clap(long("savefile"))]
    savefile: Option<String>,

    #[clap(flatten)]
    options: ProfileOptions,

    /// Overwrite the profile if it already exists.
    #[clap(long, action = ArgAction::SetTrue)]
    overwrite: bool,

    /// Scan a directory of mods to populate packages and natives automatically.
    #[clap(long("from-mods-dir"), short('d'), value_hint = clap::ValueHint::DirPath)]
    from_mods_dir: Option<PathBuf>,

    /// Optional positional path to scan (same as --from-mods-dir PATH)
    #[clap(value_hint = clap::ValueHint::DirPath)]
    mods_dir: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, Default, PartialEq)]
pub struct ProfileOptions {
    /// Re-enable online matchmaking? (ban risk)
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

    /// Do not increase memory limits? (affects game stability)
    ///
    /// Mods may require more system resources than the games are configured to provide by default.
    /// Some supported games (Dark Souls 3, Sekiro and ELDEN RING) include a patch to replace the
    /// memory allocators for more efficient ones, removing upper memory usage bounds and slightly
    /// improving game performance.
    #[clap(long("no-mem-patch"), default_missing_value = "true", num_args=0..=1)]
    pub no_mem_patch: Option<bool>,
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
            no_mem_patch: other.no_mem_patch.or(self.no_mem_patch),
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

    let mut profile = ModProfile::default();

    if let Some(game) = args.game {
        let supports = profile.supports_mut();

        supports.push(Supports {
            game: game.into(),
            since_version: None,
        });
    }

    // Accumulate additions, then apply to avoid overlapping mutable borrows
    let mut add_packages: Vec<Package> = Vec::new();
    let mut add_natives: Vec<Native> = Vec::new();

    // Add from explicit args first
    for pkg in args.packages {
        add_packages.push(Package::new(pkg));
    }
    for dll in args.natives {
        add_natives.push(Native::new(dll));
    }

    // Optionally scan a mods directory
    let scan_dir = args.from_mods_dir.or(args.mods_dir).or_else(|| {
        // Default to current directory if accessible
        std::env::current_dir().ok()
    });

    if let Some(dir) = scan_dir {
        let (scanned_packages, scanned_natives) = scan_mods_dir(&dir)?;
        add_packages.extend(scanned_packages);
        add_natives.extend(scanned_natives);
    }

    {
        let packages = profile.packages_mut();
        packages.extend(add_packages);
    }
    {
        let natives = profile.natives_mut();
        natives.extend(add_natives);
    }

    let start_online = profile.start_online_mut();
    *start_online = args.options.start_online;

    // Serialize to TOML, then remove empty top-level arrays to avoid clutter
    let mut contents = toml::to_string_pretty(&profile)?;
    {
        // Drop lines like `supports = []`, `packages = []`, and `natives = []` at the top level.
        const EMPTY_TOP_LEVEL_ARRAYS: [&str; 3] =
            ["supports = []", "packages = []", "natives = []"];

        let mut changed = false;
        let filtered: Vec<&str> = contents
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                let drop = EMPTY_TOP_LEVEL_ARRAYS
                    .iter()
                    .any(|pat| trimmed.starts_with(pat));
                if drop {
                    changed = true;
                }
                !drop
            })
            .collect();

        if changed {
            contents = filtered.join("\n");
            // Ensure trailing newline for POSIX friendliness
            if !contents.ends_with('\n') {
                contents.push('\n');
            }
        }
    }

    std::fs::write(&profile_path, contents)?;

    // Simple output
    println!("Created profile: {}", profile_path.display());

    Ok(())
}

fn scan_mods_dir(dir: &PathBuf) -> color_eyre::Result<(Vec<Package>, Vec<Native>)> {
    use std::path::Path;

    if !dir.exists() {
        return Err(eyre!("mods directory does not exist: {}", dir.display()));
    }

    // Full list of known FromSoftware mod content directories
    const KNOWN_DIR_MARKERS: &[&str] = &[
        "_backup", "_unknown", "action", "asset", "chr", "cutscene", "event", "font", "map",
        "material", "menu", "movie", "msg", "other", "param", "parts", "script", "sd", "sfx",
        "shader", "sound",
    ];

    const KNOWN_FILE_MARKERS: &[&str] = &["regulation.bin"];

    let mut packages: Vec<Package> = Vec::new();
    let mut natives: Vec<Native> = Vec::new();

    // 1) Any DLL anywhere under the root becomes a native
    fn walk_collect_dlls(root: &Path, out: &mut Vec<Native>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(root)? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                let _ = walk_collect_dlls(&path, out);
            } else if path.is_file()
                && path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
            {
                out.push(Native::new(path));
            }
        }
        Ok(())
    }
    let _ = walk_collect_dlls(dir, &mut natives);

    // Build a set of DLL stems to avoid adding configuration folders with the same name
    let dll_stems: std::collections::HashSet<String> = natives
        .iter()
        .filter_map(|n| n.source().file_stem().and_then(|s| s.to_str()))
        .map(|s| s.to_ascii_lowercase())
        .collect();

    // 2) Immediate subdirectories: include if they contain known markers or are non-empty
    fn dir_contains_known_markers(path: &Path) -> bool {
        for marker in KNOWN_FILE_MARKERS {
            if path.join(marker).exists() {
                return true;
            }
        }
        for marker in KNOWN_DIR_MARKERS {
            if path.join(marker).is_dir() {
                return true;
            }
        }
        false
    }

    fn dir_is_non_empty(path: &Path) -> bool {
        std::fs::read_dir(path)
            .ok()
            .and_then(|mut it| it.next().transpose().ok())
            .flatten()
            .is_some()
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip a package dir if its name matches any DLL stem (common pattern for native config folders)
        let dir_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        let matches_dll = dir_name
            .as_ref()
            .is_some_and(|name| dll_stems.contains(name));

        if !matches_dll && (dir_contains_known_markers(&path) || dir_is_non_empty(&path)) {
            packages.push(Package::new(path));
        }
    }

    // Stable order for deterministic output
    packages.sort_by(|a, b| a.source().cmp(&b.source()));
    natives.sort_by(|a, b| a.source().cmp(&b.source()));

    Ok((packages, natives))
}

#[tracing::instrument(err, skip_all)]
pub fn show(db: DbContext, config: Config, name: ProfileNameArgs) -> color_eyre::Result<()> {
    let profile_path = name.into_profile_path(&config)?;

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

    if let Some(savefile) = profile.savefile() {
        output.property("Savefile", savefile);
    }

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

pub fn no_profile_dir() -> color_eyre::Report {
    eyre!(
        r#"No profile directory was configured and the default profile directory was inaccessible.

        To set a profile directory either provide `--profile-dir` on the command line or set `profile_dir`
        in a me3 configuration file. Use `me3 info` to find out where me3 searches for your configuration files.
    "#
    )
}
