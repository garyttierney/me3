use std::{error::Error, io::stderr, iter, path::PathBuf, str::FromStr};

use clap::{ArgAction, Parser, ValueEnum};
use color_eyre::eyre::eyre;
use commands::{profile::ProfileCommands, Commands};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize)]
pub enum Game {
    #[serde(alias = "er")]
    #[value(alias("er"))]
    EldenRing,

    #[serde(alias = "nr", alias = "elden-ring-nightreign")]
    #[value(alias("nr"), alias("elden-ring-nightreign"))]
    Nightreign,
}

impl Game {
    pub fn app_id(self) -> u32 {
        match self {
            Self::EldenRing => 1245620,
            Self::Nightreign => 2622380,
        }
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

    color_eyre::config::HookBuilder::default()
        .install()
        .expect("failed to install error handler");

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

    let system_config_source = app_paths.system_config_path.clone();
    let user_config_source = app_paths.user_config_path.clone();
    let cli_config_source = app_paths.cli_config_path.clone();
    let config_sources = [system_config_source, user_config_source, cli_config_source];

    let mut config = config_sources
        .into_iter()
        .flatten()
        .flat_map(Config::from_file)
        .chain(iter::once(cli.config))
        .fold(Config::default(), |a, b| a.merge(b));

    if config.profile_dir.is_none() {
        config.profile_dir = app_project_dirs
            .as_ref()
            .map(|dirs| dirs.config_local_dir().join("profiles"));
    }

    let _guard = me3_telemetry::install(config.crash_reporting, stderr);
    let bins_path = bins_dir(&config);

    let result = match cli.command {
        Commands::Info => commands::info::info(app_install, app_paths, config),
        Commands::Launch(args) => commands::launch::launch(config, app_paths, bins_path, args),
        Commands::Profile(ProfileCommands::Create(args)) => commands::profile::create(config, args),
        Commands::Profile(ProfileCommands::List) => commands::profile::list(config),
        Commands::Profile(ProfileCommands::Show { name }) => commands::profile::show(config, name),
        Commands::Profile(ProfileCommands::Open { name }) => commands::profile::open(config, name),

        #[cfg(target_os = "windows")]
        Commands::AddToPath => commands::windows::add_to_path(),
        #[cfg(target_os = "windows")]
        Commands::Update => commands::windows::update(),
    };

    if let Err(error) = result {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
