use std::{
    error::Error,
    fmt::Debug,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use chrono::Local;
use clap::{ArgAction, Args};
use color_eyre::eyre::{eyre, OptionExt};
use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{native::Native, package::Package};
use normpath::PathExt;
use serde::{Deserialize, Serialize};
use steamlocate::{CompatTool, Library, SteamDir};
use tempfile::NamedTempFile;
use tracing::{error, info};

use crate::{
    config::Config,
    db::{profile::Profile, DbContext},
    Game,
};

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct Selector {
    /// Automatically detect the game to launch from mod profiles.
    #[clap(long, help_heading = "Game selection", action = ArgAction::SetTrue)]
    auto_detect: bool,

    /// Short name of a game to launch. The launcher will look for the the installation in
    /// available Steam libraries.
    #[clap(
        short('g'),
        long,
        hide_possible_values = false,
        help_heading = "Game selection"
    )]
    #[arg(value_enum)]
    game: Option<Game>,

    /// The Steam APPID of the game to launch. The launcher will attempt to find this app installed
    /// in a Steam library and launch the configured command
    #[clap(short('s'), long, alias("steamid"), help_heading = "Game selection")]
    #[arg(value_parser = clap::value_parser!(u32))]
    steam_id: Option<u32>,
}

#[derive(Args, Clone, Debug, Serialize, Deserialize, Default)]
pub struct GameOptions {
    /// Don't cache decrypted BHD files (used to improve game startup speed)?
    #[clap(long("no-boot-boost"), action = ArgAction::SetFalse)]
    pub(crate) boot_boost: Option<bool>,

    /// Skip initializing Steam within the launcher?
    #[clap(long("skip-steam-init"), action = ArgAction::SetTrue)]
    pub(crate) skip_steam_init: Option<bool>,

    /// An optional path to the game executable to launch with mod support. Uses the default
    /// launcher if not present.
    #[clap(short('e'), long, help_heading = "Game selection", value_hint = clap::ValueHint::FilePath)]
    pub(crate) exe: Option<PathBuf>,
}

impl GameOptions {
    pub fn merge(self, other: Self) -> Self {
        Self {
            boot_boost: other.boot_boost.or(self.boot_boost),
            exe: other.exe.or(self.exe),
            skip_steam_init: other.skip_steam_init.or(self.skip_steam_init),
        }
    }
}

#[derive(Args, Debug)]
pub struct LaunchArgs {
    #[clap(flatten)]
    pub target_selector: Selector,

    #[clap(flatten)]
    game_options: GameOptions,

    /// Enable diagnostics for this launch?
    #[clap(short('d'), long("diagnostics"), action = ArgAction::SetTrue)]
    diagnostics: bool,

    /// Suspend the game until a debugger is attached?
    #[clap(long("suspend"), action = ArgAction::SetTrue)]
    suspend: bool,

    /// Path to a ModProfile configuration file (TOML or JSON) or name of a profile
    /// stored in the me3 profile folder ($XDG_CONFIG_HOME/me3).
    #[arg(
            short('p'),
            long("profile"),
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    profile: Option<String>,

    /// Path to package directories that the mod host will use as VFS mount points.
    #[arg(
            long("package"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::DirPath,
        )]
    packages: Vec<PathBuf>,

    /// Path to DLLs to be loaded by the mod host.
    #[arg(
            short('n'),
            long("native"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    natives: Vec<PathBuf>,
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

        let tool_name = self.tool.name.ok_or_eyre("compat tool must have a name")?;
        let custom_tool_path = self
            .steam
            .path()
            .join(format!("compatibilitytools.d/{tool_name}"));

        let proton_path = if std::fs::exists(&custom_tool_path)? {
            custom_tool_path
        } else {
            let proton_id = match tool_name.as_str() {
                "proton_experimental" => 1493710,
                "proton_hotfix" => 2180100,
                "proton_9" => 2805730,
                _ => return Err(eyre!("unrecognised compat tool")),
            };

            let (proton_app, proton_library) = self
                .steam
                .find_app(proton_id)?
                .ok_or_eyre("configured compat tool isn't installed")?;

            proton_library.resolve_app_dir(&proton_app)
        };

        let sniper_path = sniper_library.resolve_app_dir(&sniper_app);

        let mut command = Command::new(sniper_path.join("run"));

        command.args([
            "--batch",
            "--",
            &*proton_path.join("proton").to_string_lossy(),
            "waitforexitandrun",
            &*launcher.to_string_lossy(),
        ]);

        // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/docs/steam-compat-tool-interface.md>
        command.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", self.steam.path());
        command.env(
            "STEAM_COMPAT_DATA_PATH",
            self.library
                .path()
                .join(format!("steamapps/compatdata/{}", self.app_id)),
        );

        // TODO(gtierney): unsure if this works for every scenario, but it shouldn't break anything
        // where it doesn't
        command.env(
            "LD_PRELOAD",
            self.steam.path().join("ubuntu12_64/gameoverlayrenderer.so"),
        );

        Ok(command)
    }
}

pub fn generate_attach_config(
    game: Game,
    opts: &GameOptions,
    profile: &Profile,
    extra_natives: &[PathBuf],
    extra_packages: &[PathBuf],
    suspend_on_attach: bool,
    cache_path: Option<Box<Path>>,
) -> color_eyre::Result<AttachConfig> {
    for path in extra_natives.iter().chain(extra_packages) {
        if !path.exists() {
            return Err(eyre!("{path:?} does not exist"));
        }
    }

    let mut natives = vec![];
    let mut packages = vec![];

    packages.extend(
        extra_packages
            .iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Package::new(normalized.into_path_buf())),
    );

    natives.extend(
        extra_natives
            .iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Native::new(normalized.into_path_buf())),
    );

    let (ordered_natives, ordered_packages) = profile.compile()?;
    packages.extend(ordered_packages);
    natives.extend(ordered_natives);

    Ok(AttachConfig {
        game: game.into(),
        packages,
        natives,
        skip_steam_init: opts.skip_steam_init.unwrap_or(false),
        suspend: suspend_on_attach,
        boot_boost: opts.boot_boost.unwrap_or(true),
        cache_path: cache_path.map(|path| path.into_path_buf()),
    })
}

#[tracing::instrument(err, skip_all)]
pub fn launch(db: DbContext, config: Config, args: LaunchArgs) -> color_eyre::Result<()> {
    let profile = if let Some(profile_name) = &args.profile {
        db.profiles.load(profile_name)?
    } else {
        Profile::transient()
    };

    let game = if args.target_selector.auto_detect {
        profile
            .supported_game()
            .map(|g| crate::Game(g))
            .ok_or_eyre("me3 profile lists no supported games")
    } else {
        args.target_selector
            .game
            .or_else(|| args.target_selector.steam_id.and_then(Game::from_app_id))
            .ok_or_eyre("unable to determine game from name or app ID")
    }?;

    let game_options = config
        .options
        .game
        .get(&game.0)
        .cloned()
        .unwrap_or_default()
        .merge(args.game_options);

    info!(?game, ?game_options, "resolved game");
    let attach_config = generate_attach_config(
        game,
        &game_options,
        &profile,
        &args.natives,
        &args.packages,
        args.suspend,
        config.cache_dir(),
    )?;

    let bins_dir = config
        .windows_binaries_dir()
        .ok_or_eyre("Can't find location of windows-binaries-dir")?;

    let app_id = game.app_id();
    let launcher_path = bins_dir.join("me3-launcher.exe");
    let dll_path = bins_dir.join("me3_mod_host.dll");
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

    std::fs::write(&attach_config_file, toml::to_string_pretty(&attach_config)?)?;
    info!(?attach_config_file, ?attach_config, "wrote attach config");

    let now = Local::now();
    let log_id = now.format("%Y-%m-%d_%H-%M-%S").to_string();

    let log_folder = config
        .log_dir()
        .unwrap_or_else(|| PathBuf::from(".").into_boxed_path())
        .join(profile.name());

    info!(?log_folder, "creating profile log folder");
    fs::create_dir_all(&log_folder)?;

    let log_files: Vec<(SystemTime, PathBuf)> = fs::read_dir(&log_folder)
        .map(|dir| {
            dir.filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;
                if metadata.is_file() && entry.path().extension().is_some_and(|ext| ext == "log") {
                    Some((metadata.modified().ok()?, entry.path()))
                } else {
                    None
                }
            })
            .collect()
        })
        .unwrap_or_default();

    if log_files.len() >= 5 {
        if let Some((_, path_to_delete)) = log_files.iter().min_by_key(|(time, _)| *time) {
            let _ = fs::remove_file(path_to_delete);
        }
    }

    let log_file_path = log_folder.join(format!("{log_id}.log"));
    let monitor_log_file = NamedTempFile::with_suffix(".log")?;

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

    me3_env::serialize_into_command(launcher_vars, &mut injector_command);
    me3_env::serialize_into_command(telemetry_vars, &mut injector_command);

    injector_command.env("SteamAppId", app_id.to_string());
    injector_command.env("SteamGameId", app_id.to_string());
    injector_command.env("SteamOverlayGameId", app_id.to_string());

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
        open::that_detached(log_file_path)?;
    }

    Ok(())
}
