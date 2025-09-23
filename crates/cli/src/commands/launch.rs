pub mod proton;

use std::{
    error::Error,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use clap::{
    builder::{BoolValueParser, MapValueParser, TypedValueParser},
    ArgAction, Args,
};
use color_eyre::eyre::{eyre, Context, OptionExt};
use me3_env::{CommandExt, LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{
    native::Native,
    package::Package,
    profile::{builder::ModProfileBuilder, Profile},
};
use normpath::PathExt;
use serde::{Deserialize, Serialize};
use steamlocate::{CompatTool, Library, SteamDir};
use tempfile::NamedTempFile;
use tracing::{error, info};

use crate::{
    commands::{launch::proton::CompatTools, profile::ProfileOptions},
    config::Config,
    db::{profile::Profile as DbProfile, DbContext},
    Game,
};

fn remap_slr_path(path: impl AsRef<Path>) -> PathBuf {
    // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/4d85075e6240c839a3464fd97f22aa2253a9cea1/docs/shared-paths.md#never-shared>
    const NON_SHARED_PATHS: [&str; 4] = ["/usr", "/etc", "/bin", "/lib"];

    let path = path.as_ref();

    if NON_SHARED_PATHS
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        Path::new("/run/host").join(path)
    } else {
        path.to_path_buf()
    }
}

#[derive(Debug, clap::Args)]
#[group(multiple = false)]
pub struct Selector {
    /// Detect the game to launch from mod profile.
    #[clap(long, help_heading = "Game selection", action = ArgAction::SetTrue, required = false)]
    auto_detect: bool,

    /// Short name of a game to launch.
    #[clap(
        short('g'),
        long,
        hide_possible_values = false,
        help_heading = "Game selection",
        required = false
    )]
    #[arg(value_enum)]
    game: Option<Game>,

    /// Steam APPID of the game to launch.
    #[clap(
        short('s'),
        long,
        alias("steamid"),
        help_heading = "Game selection",
        required = false
    )]
    #[arg(value_parser = clap::value_parser!(u32))]
    steam_id: Option<u32>,
}

#[derive(Args, Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct GameOptions {
    /// Don't cache decrypted BHD files?
    ///
    /// BHD archives are decrypted every time a game is started, which takes significant time and
    /// CPU. me3 caches the decrypted archives to reduce game startup time.
    #[clap(long("no-boot-boost"), default_missing_value = "true", num_args=0..=1, value_parser = invert_bool())]
    pub(crate) boot_boost: Option<bool>,

    /// Show game intro logos?
    #[clap(long("show-logos"), default_missing_value = "true", num_args=0..=1, value_parser = invert_bool())]
    pub(crate) skip_logos: Option<bool>,

    /// Skip initializing Steam within the launcher?
    #[clap(long("skip-steam-init"), default_missing_value = "true", num_args=0..=1)]
    pub(crate) skip_steam_init: Option<bool>,

    /// Custom path to the game executable.
    #[clap(short('e'), long, help_heading = "Game selection", value_hint = clap::ValueHint::FilePath)]
    pub(crate) exe: Option<PathBuf>,
}

fn invert_bool() -> MapValueParser<BoolValueParser, fn(bool) -> bool> {
    BoolValueParser::new().map(|v| !v)
}

impl GameOptions {
    pub fn merge(self, other: Self) -> Self {
        Self {
            boot_boost: other.boot_boost.or(self.boot_boost),
            skip_logos: other.skip_logos.or(self.skip_logos),
            skip_steam_init: other.skip_steam_init.or(self.skip_steam_init),
            exe: other.exe.or(self.exe),
        }
    }
}

#[derive(Args, Debug)]
pub struct LaunchArgs {
    #[clap(flatten)]
    target_selector: Option<Selector>,

    #[clap(flatten)]
    game_options: GameOptions,

    #[clap(flatten)]
    profile_options: ProfileOptions,

    /// Enable diagnostics for this launch.
    #[clap(short, long, action = ArgAction::SetTrue)]
    diagnostics: bool,

    /// Suspend the game until a debugger is attached.
    #[clap(long, action = ArgAction::SetTrue)]
    suspend: bool,

    /// Name of a profile in the me3 profile dir, or path to a ModProfile (TOML or JSON)
    /// [repeatable option]
    #[arg(
            short,
            long("profile"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    profiles: Vec<String>,

    /// Path to a native DLL, package, file or a profile to use [repeatable option]
    #[clap(
            short,
            long("mod"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::AnyPath,
        )]
    mods: Vec<PathBuf>,

    /// Path to package directory (asset override mod) [repeatable option]
    #[clap(
            long("package"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::DirPath,
        )]
    packages: Vec<PathBuf>,

    /// Path to DLL file (native DLL mod) [repeatable option]
    #[clap(
            short,
            long("native"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    natives: Vec<PathBuf>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[clap(help_heading = "Mod configuration")]
    savefile: Option<String>,
}

pub trait Launcher: Debug {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command>;
}

#[derive(Debug)]
pub struct DirectLauncher;

impl Launcher for DirectLauncher {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command> {
        Ok(Command::new(launcher))
    }
}

#[derive(Debug)]
pub struct CompatToolLauncher {
    tool: CompatTool,
    steam: SteamDir,
    library: Library,
    app_id: u32,
}

impl Launcher for CompatToolLauncher {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command> {
        // TODO: parse this from appcache/appinfo.vcf
        let sniper_id = 1628350;
        let (sniper_app, sniper_library) = self
            .steam
            .find_app(sniper_id)?
            .ok_or_eyre("unable to find Steam Linux Runtime")?;

        let compat_tools = CompatTools::new(&self.steam);
        let compat_tool = compat_tools
            .find(self.tool.name.expect("compat tools must be named"))
            .ok_or_eyre("unable to find compatibility tool installation")?;
        let sniper_path = sniper_library.resolve_app_dir(&sniper_app);
        let mut command = Command::new(sniper_path.join("run"));

        command.args([
            "--batch",
            "--",
            &*compat_tool.install_path.join("proton").to_string_lossy(),
            "waitforexitandrun",
            &*launcher.to_string_lossy(),
        ]);

        // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/docs/steam-compat-tool-interface.md>
        command.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", self.steam.path());

        let prefix_path = self
            .steam
            .library_paths()?
            .into_iter()
            .map(|path| path.join(format!("steamapps/compatdata/{}", self.app_id)))
            .find(|path| path.exists())
            .unwrap_or_else(|| {
                self.library
                    .path()
                    .join(format!("steamapps/compatdata/{}", self.app_id))
            });

        command.env("STEAM_COMPAT_DATA_PATH", prefix_path);

        // TODO(gtierney): unsure if this works for every scenario, but it shouldn't break anything
        // where it doesn't
        let mut ld_preload = self
            .steam
            .path()
            .join("ubuntu12_64/gameoverlayrenderer.so")
            .into_os_string();

        if let Some(existing_ld_preload) = std::env::var_os("LD_PRELOAD") {
            ld_preload.push(" ");
            ld_preload.push(&existing_ld_preload);
        }

        command.env("LD_PRELOAD", ld_preload);

        Ok(command)
    }
}

struct LaunchContext {
    game: Game,
    game_options: GameOptions,
    profile_options: ProfileOptions,
    attach_config: AttachConfig,
}

impl LaunchArgs {
    fn parse_with_context(
        &self,
        db: &DbContext,
        config: &Config,
    ) -> color_eyre::Result<LaunchContext> {
        let profile = if let Some(profile_name) = self.profiles.first() {
            db.profiles.load(profile_name)?
        } else {
            DbProfile::transient()
        };

        let game_from_args = self
            .target_selector
            .as_ref()
            .and_then(|s| s.game.or_else(|| s.steam_id.and_then(Game::from_app_id)))
            .map(Into::into);

        for path in self.mods.iter().chain(&self.natives).chain(&self.packages) {
            if !path.exists() {
                return Err(eyre!("{path:?} does not exist"));
            }
        }

        let other_profiles = self
            .profiles
            .get(1..)
            .into_iter()
            .flatten()
            .map(|profile| config.resolve_profile(profile))
            .collect::<Result<Vec<_>, _>>()?;

        let profile_from_args = ModProfileBuilder::new()
            .with_supported_game(game_from_args)
            .with_mods(self.natives.iter().map(Native::new))
            .with_mods(self.packages.iter().map(Package::new))
            .with_mods(other_profiles.iter().map(Profile::new))
            .with_paths(self.mods.iter().cloned())
            .with_savefile(self.savefile.clone())
            .start_online(self.profile_options.start_online)
            .disable_arxan(self.profile_options.disable_arxan)
            .build();

        let profile = profile.try_merge(&profile_from_args).wrap_err_with(|| {
            eyre!(
                "game ({game_from_args:?}) is not supported by profile ({:?})",
                profile.supported_game()
            )
        })?;

        let game = profile
            .supported_game()
            .map(Game)
            .ok_or_eyre("unable to determine which game to launch")?;

        let game_options = config
            .options
            .game
            .get(&game.0)
            .cloned()
            .unwrap_or_default()
            .merge(self.game_options.clone());

        let profile_options = profile.options().merge(self.profile_options.clone());

        info!(?game, ?game_options, ?profile_options, "resolved game");

        let attach_config = self.generate_attach_config(
            db,
            game,
            &game_options,
            profile,
            &profile_options,
            config.cache_dir(),
        )?;

        Ok(LaunchContext {
            game,
            game_options,
            profile_options,
            attach_config,
        })
    }

    fn generate_attach_config(
        &self,
        db: &DbContext,
        game: Game,
        opts: &GameOptions,
        profile: DbProfile,
        profile_options: &ProfileOptions,
        cache_path: Option<Box<Path>>,
    ) -> color_eyre::Result<AttachConfig> {
        let profile_name = profile.name().to_owned();

        let savefile = profile.savefile();
        if let Some(savefile) = &savefile {
            // https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
            let is_windows_path_reserved_char = |c: char| {
                matches!(
                    c,
                    '\x00'..'\x1f' | '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            };

            if savefile.chars().any(is_windows_path_reserved_char) {
                return Err(eyre!(
                    "savefile name ({savefile:?}) contains reserved file name characters"
                ));
            }
        }

        let (natives, packages) = profile.compile(&db.profiles)?;

        Ok(AttachConfig {
            profile_name,
            game: game.into(),
            natives,
            packages,
            savefile,
            cache_path: cache_path.map(|path| path.into_path_buf()),
            suspend: self.suspend,
            boot_boost: opts.boot_boost.unwrap_or(true),
            skip_logos: opts.skip_logos.unwrap_or(true),
            start_online: profile_options.start_online.unwrap_or(false),
            disable_arxan: profile_options.disable_arxan.unwrap_or(false),
            skip_steam_init: opts.skip_steam_init.unwrap_or(false),
        })
    }
}

#[tracing::instrument(err, skip_all)]
pub fn launch(db: DbContext, config: Config, args: LaunchArgs) -> color_eyre::Result<()> {
    let LaunchContext {
        game,
        game_options,
        profile_options: _profile_options,
        attach_config,
    } = args.parse_with_context(&db, &config)?;

    let bins_dir = config
        .windows_binaries_dir()
        .ok_or_eyre("Can't find location of windows-binaries-dir")?;

    let app_id = game.app_id();
    let launcher_path = if cfg!(target_os = "linux") {
        remap_slr_path(bins_dir.join("me3-launcher.exe"))
    } else {
        bins_dir.join("me3-launcher.exe")
    };

    let dll_path = if cfg!(target_os = "linux") {
        remap_slr_path(bins_dir.join("me3_mod_host.dll"))
    } else {
        bins_dir.join("me3_mod_host.dll")
    };

    let game_exe_path = game_options
        .exe
        .map(color_eyre::eyre::Ok)
        .unwrap_or_else(|| {
            let steam_dir = config.steam_dir()?;
            let (app, library) = steam_dir.find_app(app_id)?.ok_or_eyre(
                "Steam was used to locate the game executable and no game installation was found",
            )?;

            let game_exe = library.resolve_app_dir(&app).join(game.launcher());

            Ok(game_exe)
        })?;

    let mut injector_command = if cfg!(target_os = "linux") {
        let steam_dir = config.steam_dir()?;
        let (_, steam_library) = steam_dir.find_app(app_id)?.ok_or_eyre(
            "Steam was used to locate the WINE configuration and no game installation was found",
        )?;

        let compat_tools = steam_dir.compat_tool_mapping()?;
        let app_compat_tool = compat_tools
            .get(&app_id)
            .or_else(|| compat_tools.get(&0))
            .ok_or_eyre("unable to find compat tool for game")?;

        info!(?app_compat_tool, "found compat tool for appid");
        let launcher = CompatToolLauncher {
            app_id,
            library: steam_library,
            steam: steam_dir,
            tool: app_compat_tool.clone(),
        };

        launcher.into_command(launcher_path)
    } else {
        DirectLauncher.into_command(launcher_path)
    }?;

    let attach_config_dir = config.cache_dir().unwrap_or(Box::from(Path::new(".")));
    std::fs::create_dir_all(&attach_config_dir)?;
    let attach_config_file = NamedTempFile::new_in(&attach_config_dir)?;

    std::fs::write(&attach_config_file, toml::to_string(&attach_config)?)?;
    info!(?attach_config_file, ?attach_config, "wrote attach config");

    let monitor_log_file = NamedTempFile::with_suffix(".log")?;

    let log_file_path = db.logs.create_log_file(&attach_config.profile_name)?;
    // Ensure log file exists so `normalize()` succeeds on Unix
    let log_file = File::create(&log_file_path)?;
    drop(log_file);

    info!(path = ?monitor_log_file.path(), "temporary log file created");

    let launcher_vars = LauncherVars {
        exe: game_exe_path,
        host_dll: dll_path,
        host_config_path: attach_config_file.path().to_path_buf(),
    };

    let telemetry_vars = TelemetryVars {
        enabled: config.options.crash_reporting.unwrap_or_default(),
        log_file_path: log_file_path.normalize()?.into_path_buf(),
        monitor_file_path: monitor_log_file.path().normalize()?.into_path_buf(),
        trace_id: me3_telemetry::trace_id(),
    };

    injector_command
        .with_env_vars(game.into_vars())
        .with_env_vars(launcher_vars)
        .with_env_vars(telemetry_vars)
        .env("SteamAppId", app_id.to_string())
        .env("SteamGameId", app_id.to_string())
        .env("SteamOverlayGameId", app_id.to_string());

    info!(?injector_command, "running injector command");

    let running = Arc::new(AtomicBool::new(true));
    let mut launcher_proc = injector_command.spawn()?;

    let monitor_thread_running = running.clone();

    let monitor_thread = std::thread::spawn(move || {
        let mut log_reader = BufReader::new(monitor_log_file);
        let mut exit_code = None;

        while monitor_thread_running.load(Ordering::SeqCst) {
            exit_code = exit_code.or_else(|| {
                launcher_proc
                    .try_wait()
                    .expect("error while checking status")
            });

            let mut line = String::new();
            let read = log_reader.read_line(&mut line);

            if let Err(e) = read {
                error!(error = &e as &dyn Error, "couldn't read log line from game");
            }

            if !line.is_empty() {
                eprint!("{line}");
            } else if exit_code.is_some() {
                break;
            } else {
                // Back-off the read loop because read_line() returns EOF.
                // This needs replaced with a proper pipe.
                std::thread::sleep(Duration::from_millis(250));
            }

            std::thread::yield_now();
        }

        let _ = launcher_proc.kill();
    });

    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })?;

    let _ = monitor_thread.join();

    if args.diagnostics {
        open::that_detached(&*log_file_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::{
        commands::{launch::GameOptions, profile::ProfileOptions, Commands},
        Cli,
    };

    #[test]
    fn optional_flags_default_to_none() {
        let cli = Cli::parse_from(&["me3", "launch", "-g", "er"]);

        let Commands::Launch(launch_args) = cli.command else {
            panic!("me3 launch produced incorrect command");
        };

        pretty_assertions::assert_eq!(
            launch_args.game_options,
            GameOptions {
                boot_boost: None,
                skip_logos: None,
                skip_steam_init: None,
                exe: None,
            },
        );

        pretty_assertions::assert_eq!(
            launch_args.profile_options,
            ProfileOptions {
                start_online: None,
                disable_arxan: None,
            },
        );
    }

    #[test]
    fn optional_flags_with_missing_values() {
        let cli = Cli::parse_from(&[
            "me3",
            "launch",
            "-g",
            "er",
            "--no-boot-boost",
            "--show-logos",
            "--disable-arxan",
            "--skip-steam-init",
            "--online",
        ]);

        let Commands::Launch(launch_args) = cli.command else {
            panic!("me3 launch produced incorrect command");
        };

        pretty_assertions::assert_eq!(
            launch_args.game_options,
            GameOptions {
                boot_boost: Some(false),
                skip_logos: Some(false),
                skip_steam_init: Some(true),
                exe: None,
            },
        );

        pretty_assertions::assert_eq!(
            launch_args.profile_options,
            ProfileOptions {
                start_online: Some(true),
                disable_arxan: Some(true),
            },
        );
    }

    #[test]
    fn optional_flags_with_false_values() {
        let cli = Cli::parse_from(&[
            "me3",
            "launch",
            "-g",
            "er",
            "--no-boot-boost=false",
            "--show-logos=false",
            "--disable-arxan=false",
            "--skip-steam-init=false",
            "--online=false",
        ]);

        let Commands::Launch(launch_args) = cli.command else {
            panic!("me3 launch produced incorrect command");
        };

        pretty_assertions::assert_eq!(
            launch_args.game_options,
            GameOptions {
                boot_boost: Some(true),
                skip_logos: Some(true),
                skip_steam_init: Some(false),
                exe: None,
            },
        );

        pretty_assertions::assert_eq!(
            launch_args.profile_options,
            ProfileOptions {
                start_online: Some(false),
                disable_arxan: Some(false),
            },
        );
    }

    #[test]
    fn optional_flags_with_true_values() {
        let cli = Cli::parse_from(&[
            "me3",
            "launch",
            "-g",
            "er",
            "--no-boot-boost=true",
            "--show-logos=true",
            "--disable-arxan=true",
            "--skip-steam-init=true",
            "--online=true",
        ]);

        let Commands::Launch(launch_args) = cli.command else {
            panic!("me3 launch produced incorrect command");
        };

        pretty_assertions::assert_eq!(
            launch_args.game_options,
            GameOptions {
                boot_boost: Some(false),
                skip_logos: Some(false),
                skip_steam_init: Some(true),
                exe: None,
            },
        );

        pretty_assertions::assert_eq!(
            launch_args.profile_options,
            ProfileOptions {
                start_online: Some(true),
                disable_arxan: Some(true),
            },
        );
    }
}
