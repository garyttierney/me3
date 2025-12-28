mod named_pipe;
pub mod proton;

use std::{
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use clap::{
    builder::{BoolValueParser, MapValueParser, TypedValueParser},
    ArgAction, Args,
};
use color_eyre::eyre::{eyre, OptionExt};
use me3_env::{CommandExt, LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{native::Native, package::Package};
use normpath::PathExt;
use serde::{Deserialize, Serialize};
use steamlocate::{Library, SteamDir};
use tempfile::NamedTempFile;
use tracing::{error, info};

use crate::{
    commands::{
        launch::{named_pipe::NamedPipe, proton::CompatTools},
        profile::ProfileOptions,
    },
    config::Config,
    db::{profile::Profile, DbContext},
    Game,
};

fn remap_slr_path(path: impl AsRef<Path>) -> PathBuf {
    // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/4d85075e6240c839a3464fd97f22aa2253a9cea1/docs/shared-paths.md#never-shared>
    const NON_SHARED_PATHS: [&'static str; 4] = ["/usr", "/etc", "/bin", "/lib"];

    let path = path.as_ref();

    if NON_SHARED_PATHS
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        Path::new("/run/host").join(path.strip_prefix("/").unwrap())
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
    #[clap(short('d'), long("diagnostics"), action = ArgAction::SetTrue)]
    diagnostics: bool,

    /// Suspend the game until a debugger is attached.
    #[clap(long("suspend"), action = ArgAction::SetTrue)]
    suspend: bool,

    /// Name of a profile in the me3 profile dir, or path to a ModProfile (TOML or JSON).
    #[arg(
            short('p'),
            long("profile"),
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    profile: Option<String>,

    /// Path to package directory (asset override mod) [repeatable option]
    #[arg(
            long("package"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::DirPath,
        )]
    packages: Vec<PathBuf>,

    /// Path to DLL file (native DLL mod) [repeatable option]
    #[arg(
            short('n'),
            long("native"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    natives: Vec<PathBuf>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[arg(long("savefile"), help_heading = "Mod configuration")]
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
    tool: proton::CompatTool,
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

        let sniper_path = sniper_library.resolve_app_dir(&sniper_app);
        let mut command = Command::new(sniper_path.join("run"));

        command.args([
            "--batch",
            "--",
            &*self.tool.install_path.join("proton").to_string_lossy(),
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
            .filter(|path| path.exists())
            .next()
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
    profile: Profile,
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
        let profile = if let Some(profile_name) = &self.profile {
            db.profiles.load(profile_name)?
        } else {
            Profile::transient()
        };

        let target_selector = self.target_selector.as_ref().unwrap_or(&Selector {
            auto_detect: true,
            game: None,
            steam_id: None,
        });

        let game = if target_selector.auto_detect {
            profile
                .supported_game()
                .map(crate::Game)
                .ok_or_eyre("unable to determine which game to launch")
        } else {
            target_selector
                .game
                .or_else(|| target_selector.steam_id.and_then(Game::from_app_id))
                .ok_or_eyre("unable to determine game from name or app ID")
        }?;

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
            game,
            &game_options,
            &profile,
            &profile_options,
            config.cache_dir(),
        )?;

        Ok(LaunchContext {
            game,
            profile,
            game_options,
            profile_options,
            attach_config,
        })
    }

    fn generate_attach_config(
        &self,
        game: Game,
        opts: &GameOptions,
        profile: &Profile,
        profile_options: &ProfileOptions,
        cache_path: Option<Box<Path>>,
    ) -> color_eyre::Result<AttachConfig> {
        for path in self.natives.iter().chain(&self.packages) {
            if !path.exists() {
                return Err(eyre!("{path:?} does not exist"));
            }
        }

        let mut packages = self
            .packages
            .iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Package::new(normalized.into_path_buf()))
            .collect::<Vec<_>>();

        let mut natives = self
            .natives
            .iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Native::new(normalized.into_path_buf()))
            .collect::<Vec<_>>();

        let (ordered_natives, early_natives, ordered_packages) = profile.compile()?;

        packages.extend(ordered_packages);
        natives.extend(ordered_natives);

        let savefile = self.savefile.clone().or_else(|| profile.savefile());

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

        Ok(AttachConfig {
            game: game.into(),
            packages,
            natives,
            early_natives,
            savefile,
            cache_path: cache_path.map(|path| path.into_path_buf()),
            suspend: self.suspend,
            boot_boost: opts.boot_boost.unwrap_or(true),
            skip_logos: opts.skip_logos.unwrap_or(true),
            start_online: profile_options.start_online.unwrap_or(false),
            disable_arxan: profile_options.disable_arxan.unwrap_or(false),
            mem_patch: !profile_options.no_mem_patch.unwrap_or(false),
            skip_steam_init: opts.skip_steam_init.unwrap_or(false),
        })
    }
}

#[tracing::instrument(err, skip_all)]
pub fn launch(db: DbContext, config: Config, args: LaunchArgs) -> color_eyre::Result<()> {
    let LaunchContext {
        game,
        profile,
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

        let compat_tool_mapping = steam_dir.compat_tool_mapping()?;
        let compat_tools = CompatTools::new(&steam_dir);

        let app_compat_tool = compat_tool_mapping
            .get(&app_id)
            .or_else(|| compat_tool_mapping.get(&0));

        let compat_tool_name = app_compat_tool
            .and_then(|tool| tool.name.clone())
            .or_else(|| game.0.verified_on_deck_runtime().map(|rt| rt.to_string()))
            .ok_or_eyre("unable to determine Proton runtime to run game with")?;

        let compat_tool = compat_tools.find(&compat_tool_name).ok_or_eyre(format!(
            "unable to find installation of Proton runtime {compat_tool_name}"
        ))?;

        let launcher = CompatToolLauncher {
            app_id,
            library: steam_library,
            steam: steam_dir,
            tool: compat_tool,
        };

        launcher.into_command(launcher_path)
    } else {
        DirectLauncher.into_command(launcher_path)
    }?;

    let attach_config_dir = config.cache_dir().unwrap_or(Box::from(Path::new(".")));
    std::fs::create_dir_all(&attach_config_dir)?;
    let attach_config_file = NamedTempFile::new_in(&attach_config_dir)?;

    std::fs::write(&attach_config_file, toml::to_string_pretty(&attach_config)?)?;
    info!(?attach_config_file, ?attach_config, "wrote attach config");

    let mut monitor_pipe = NamedPipe::create()?;
    info!(path = ?monitor_pipe.path(), "monitor pipe created");

    let log_file_path = db.logs.create_log_file(profile.name())?;
    // Ensure log file exists so `normalize()` succeeds on Unix
    let log_file = File::create(&log_file_path)?;
    drop(log_file);

    let launcher_vars = LauncherVars {
        exe: game_exe_path,
        host_dll: dll_path,
        host_config_path: attach_config_file.path().to_path_buf(),
    };

    let monitor_pipe_path = monitor_pipe.path().normalize()?.into_path_buf();

    let telemetry_vars = TelemetryVars {
        enabled: config.options.crash_reporting.unwrap_or_default(),
        log_file_path: log_file_path.normalize()?.into_path_buf(),
        monitor_pipe_path,
        trace_id: me3_telemetry::trace_id(),
    };

    injector_command
        .with_env_vars(game.into_vars())
        .with_env_vars(launcher_vars)
        .with_env_vars(telemetry_vars)
        .env("SteamAppId", app_id.to_string())
        .env("SteamGameId", app_id.to_string())
        .env("SteamOverlayGameId", app_id.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    info!(?injector_command, "running injector command");
    // Set terminal window title. See console_codes(4)
    print!("\x1B]0;me3 - {}\x07", profile.name());

    let running = Arc::new(AtomicBool::new(true));
    let mut launcher_proc = injector_command.spawn()?;

    let monitor_thread_running = running.clone();

    let monitor_thread = std::thread::spawn(move || {
        monitor_pipe.disable_cleanup(true);

        let monitor_pipe = monitor_pipe
            .into_file()
            .open()
            .expect("failed to open pipe");

        let mut reader = BufReader::new(monitor_pipe);

        let mut exit_code = None;

        while monitor_thread_running.load(Ordering::Relaxed) {
            exit_code = exit_code.or_else(|| {
                launcher_proc
                    .try_wait()
                    .expect("error while checking status")
            });

            let mut line = String::new();

            let read = reader.read_line(&mut line);

            if let Err(error) = read {
                error!(%error, "couldn't read log line from game");
            }

            if !line.is_empty() {
                eprint!("{line}");
            } else if exit_code.is_some() {
                break;
            }
        }

        let _ = launcher_proc.kill();
    });

    ctrlc::set_handler(move || {
        running.store(false, Ordering::Relaxed);
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
                no_mem_patch: None,
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
            "--no-mem-patch",
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
                no_mem_patch: Some(true),
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
            "--no-mem-patch=false",
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
                no_mem_patch: Some(false),
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
            "--no-mem-patch=true",
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
                no_mem_patch: Some(true),
            },
        );
    }
}
