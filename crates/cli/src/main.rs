use std::{error::Error, io::stderr, path::PathBuf, str::FromStr};

use clap::{builder::PossibleValue, ArgAction, Command, Parser, ValueEnum};
use color_eyre::eyre::{self, eyre, DefaultHandler, EyreHandler};
use commands::{profile::ProfileCommands, Commands};
use config::{ConfigError, Environment, File, Map, Source};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::warn;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

mod commands;
pub mod output;

#[derive(Parser)]
#[command(name = "me3", version, about)]
#[command(propagate_version = true)]
#[command(flatten_help = true)]
struct Cli {
    #[clap(flatten)]
    config: Config,

    /// Disable tracing logs and diagnostics.
    #[clap(short, long, action = ArgAction::SetTrue)]
    quiet: bool,

    #[clap(long)]
    config_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

mod settings;
pub use self::settings::Config;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Game(me3_mod_protocol::Game);

impl ValueEnum for Game {
    fn value_variants<'a>() -> &'a [Self] {
        use me3_mod_protocol::Game as G;
        &[
            Game(G::EldenRing),
            Game(G::Nightreign),
            Game(G::ArmoredCore6),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        use me3_mod_protocol::Game as G;
        Some(match self.0 {
            G::EldenRing => PossibleValue::new("eldenring").aliases(["er", "elden-ring"]),
            G::Nightreign => PossibleValue::new("nightreign").aliases(["nr", "nightrein"]),
            G::ArmoredCore6 => PossibleValue::new("armoredcore6").alias("ac6"),
        })
    }
}

impl Game {
    pub fn app_id(self) -> u32 {
        use me3_mod_protocol::Game as G;

        match self.0 {
            G::EldenRing => 1245620,
            G::Nightreign => 2622380,
            G::ArmoredCore6 => 1888160,
        }
    }

    pub fn launcher(&self) -> PathBuf {
        use me3_mod_protocol::Game as G;

        PathBuf::from(match self.0 {
            G::EldenRing => "Game/eldenring.exe",
            G::Nightreign => "Game/nightreign.exe",
            G::ArmoredCore6 => "Game/armoredcore6.exe",
        })
    }

    fn from_app_id(id: u32) -> Option<Self> {
        use me3_mod_protocol::Game as G;

        let game = match id {
            1245620 => G::EldenRing,
            2622380 => G::Nightreign,
            1888160 => G::ArmoredCore6,
            _ => return None,
        };

        Some(Game(game))
    }
}

impl From<Game> for me3_mod_protocol::Game {
    fn from(val: Game) -> Self {
        val.0
    }
}

#[derive(Clone)]
pub struct AppInstallInfo {
    prefix: PathBuf,
    config_path: PathBuf,
}

#[derive(Default)]
pub struct AppPaths {
    system_config_path: Option<PathBuf>,
    user_config_path: Option<PathBuf>,
    cli_config_path: Option<PathBuf>,
    logs_path: Option<PathBuf>,
    cache_path: Option<PathBuf>,
}

impl AppPaths {
    pub fn cache_path<P: Into<PathBuf>>(self, path: Option<P>) -> Self {
        Self {
            cache_path: path.map(|p| p.into()),
            ..self
        }
    }
    pub fn logs_path<P: Into<PathBuf>>(self, path: Option<P>) -> AppPaths {
        Self {
            logs_path: path.map(|p| p.into()),
            ..self
        }
    }

    pub fn cli_config<P: Into<PathBuf>>(self, path: Option<P>) -> AppPaths {
        Self {
            cli_config_path: path.map(|p| p.into()),
            ..self
        }
    }

    pub fn user_config<P: Into<PathBuf>>(self, path: Option<P>) -> AppPaths {
        Self {
            user_config_path: path.map(|p| p.into()),
            ..self
        }
    }

    pub fn system_config<P: Into<PathBuf>>(self, path: Option<P>) -> AppPaths {
        Self {
            system_config_path: path.map(|p| p.into()),
            ..self
        }
    }
}

impl AppInstallInfo {
    #[cfg(target_os = "linux")]
    fn try_from_os() -> Result<Self, Box<dyn Error>> {
        Err(eyre!("unable to detect OS installation on Linux").into())
    }

    #[cfg(target_os = "windows")]
    fn try_from_os() -> Result<Self, Box<dyn Error>> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let me3_reg = hklm.open_subkey(r"Software\garyttierney\me3")?;
        let install_dir_value = me3_reg.get_value::<String, _>("Install_Dir")?;
        let install_dir = PathBuf::from_str(&install_dir_value)?;

        Ok(AppInstallInfo {
            prefix: install_dir.clone(),
            config_path: install_dir.join("config"),
        })
    }

    // When running under `cargo run ...`
    fn try_from_cargo() -> Result<Self, Box<dyn Error>> {
        if std::env::var("NO_CARGO_DETECTION").is_ok() {
            return Err(eyre!("Cargo detection was disabled via NO_CARGO_DETECTION=").into());
        }

        let ws_dir = std::env::var("CARGO_MANIFEST_DIR")?;

        Ok(Self {
            prefix: ws_dir.clone().into(),
            config_path: ws_dir.clone().into(),
        })
    }

    fn system_config(&self) -> PathBuf {
        self.config_path.join("me3.toml")
    }
}

#[derive(Debug, Clone)]
pub struct OptionalConfigSource<T: Source + Clone + Send + Sync>(Option<T>);

impl<T: Source + Clone + Send + Sync + 'static> Source for OptionalConfigSource<T> {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<config::Map<String, config::Value>, ConfigError> {
        match &self.0 {
            None => Ok(Map::new()),
            Some(source) => Ok(source.collect().unwrap_or_default()),
        }
    }
}

pub struct AppContext {
    config: Config,
    installation: Option<AppInstallInfo>,
    paths: AppPaths,
}

#[cfg(target_os = "linux")]
fn bins_dir(config: &Config) -> PathBuf {
    const DEBUG_TARGET_DIR: &str = "x86_64-unknown-linux-gnu/debug";

    config.windows_binaries_dir.clone().unwrap_or_else(|| {
        let cli_exe_path = std::env::current_exe().expect("can't find current exe");
        let cli_exe_dir = cli_exe_path.parent().expect("can't find current exe dir");

        if cli_exe_path.is_symlink() {
            std::fs::read_link(&cli_exe_path).unwrap()
        } else if cfg!(debug_assertions) && cli_exe_dir.ends_with(DEBUG_TARGET_DIR) {
            let target_dir = cli_exe_dir
                .ancestors()
                .nth(2)
                .expect("found cargo workspace, but no target dir");

            target_dir.join("x86_64-pc-windows-msvc/debug")
        } else {
            cli_exe_dir.to_path_buf()
        }
    })
}

#[cfg(target_os = "windows")]
fn bins_dir(_config: &Config) -> PathBuf {
    let cli_exe_path = std::env::current_exe().expect("can't find current exe");
    let cli_exe_dir = cli_exe_path.parent().expect("can't find current exe dir");

    cli_exe_dir.to_path_buf()
}

fn main() {
    let cli = Cli::parse();

    let app_install = AppInstallInfo::try_from_cargo()
        .or_else(|_| AppInstallInfo::try_from_os())
        .ok();

    let app_project_dirs = ProjectDirs::from("com.github", "garyttierney", "me3");
    let app_paths = AppPaths::default()
        .system_config(app_install.as_ref().map(|info| info.system_config()))
        .user_config(
            app_project_dirs
                .as_ref()
                .map(|dirs| dirs.config_local_dir().join("me3.toml")),
        )
        .cli_config(cli.config_file)
        .logs_path(
            app_project_dirs
                .as_ref()
                .map(|dirs| dirs.data_local_dir().join("logs"))
                .or_else(|| std::env::current_dir().ok()),
        )
        .cache_path(app_project_dirs.as_ref().map(|dirs| dirs.cache_dir()));

    let system_config_source = app_paths.system_config_path.clone().map(File::from);
    let user_config_source = app_paths.user_config_path.clone().map(File::from);
    let cli_config_source = app_paths.cli_config_path.clone().map(File::from);

    let mut config = config::Config::builder()
        .add_source(OptionalConfigSource(system_config_source))
        .add_source(OptionalConfigSource(user_config_source))
        .add_source(OptionalConfigSource(cli_config_source))
        .add_source(Environment::with_prefix("ME3"));

    let mut config = config
        .build()
        .and_then(|cfg| cfg.try_deserialize::<Config>())
        .inspect_err(|err| warn!("Failed to load configuration: {err:?}"))
        .unwrap_or_default()
        .merge(cli.config);

    if config.profile_dir.is_none() {
        config.profile_dir = app_project_dirs
            .as_ref()
            .map(|dirs| dirs.config_local_dir().join("profiles"));
    }

    let _guard = me3_telemetry::install(
        config.crash_reporting,
        None,
        Some(BoxMakeWriter::new(stderr)),
    );

    let bins_path = bins_dir(&config);

    let result = match cli.command {
        Commands::Info => commands::info::info(app_install, app_paths, config),
        Commands::Launch(args) => commands::launch::launch(config, app_paths, bins_path, args),
        Commands::Profile(ProfileCommands::Create(args)) => commands::profile::create(config, args),
        Commands::Profile(ProfileCommands::List) => commands::profile::list(config),
        Commands::Profile(ProfileCommands::Show { name }) => commands::profile::show(config, name),
        #[cfg(target_os = "windows")]
        Commands::AddToPath => commands::windows::add_to_path(),
        #[cfg(target_os = "windows")]
        Commands::Update => commands::windows::update(),
    };

    result.expect("command failed");
}
