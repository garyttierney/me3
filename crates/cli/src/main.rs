use std::{
    io::stderr,
    iter,
    path::{Path, PathBuf},
    slice,
};

use clap::{builder::PossibleValue, ArgAction, Parser, ValueEnum};
use commands::{profile::ProfileCommands, Commands};
use me3_telemetry::TelemetryConfig;
use serde::{Deserialize, Serialize};
use strum::VariantArray;
use tracing::{debug, info};

mod commands;
pub mod db;
pub mod output;

#[derive(Parser)]
#[command(
    name = "me3",
    version,
    about = "Mod loader for FROMSOFTWARE games",
    after_help = "For more information, visit https://me3.help/",
    propagate_version = true,
    flatten_help = true
)]
struct Cli {
    #[clap(flatten)]
    config: Options,

    /// Disable tracing logs and diagnostics.
    #[clap(short, long, action = ArgAction::SetTrue)]
    quiet: bool,

    /// Use a local OTEL collector to visualize development logs.
    #[clap(long, action = ArgAction::SetTrue, hide(true))]
    enable_opentelemetry: bool,

    #[clap(long)]
    config_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

mod config;
pub use self::config::Options;
use crate::{
    config::{Config, KnownDirs},
    db::DbContext,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Game(me3_mod_protocol::Game);

impl ValueEnum for Game {
    fn value_variants<'a>() -> &'a [Self] {
        // SAFETY: slice of a transparent wrapper type of the same length.
        unsafe {
            slice::from_raw_parts(
                me3_mod_protocol::Game::VARIANTS.as_ptr() as *const Self,
                me3_mod_protocol::Game::VARIANTS.len(),
            )
        }
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(PossibleValue::new(self.0.name()).aliases(self.0.aliases()))
    }
}

impl Game {
    fn app_id(self) -> u32 {
        self.0.app_id()
    }

    fn from_app_id(id: u32) -> Option<Self> {
        me3_mod_protocol::Game::from_app_id(id).map(Self)
    }

    fn launcher(self) -> &'static Path {
        self.0.executable()
    }

    fn into_vars(self) -> me3_env::GameVars {
        me3_env::GameVars { launched: self.0 }
    }
}

impl From<Game> for me3_mod_protocol::Game {
    fn from(val: Game) -> Self {
        val.0
    }
}

fn main() {
    me3_telemetry::install_error_handler();

    // Some Windows terminals do not display ANSI escape codes by default.
    #[cfg(target_os = "windows")]
    let _ = crate::commands::windows::enable_ansi();

    let cli = Cli::parse();

    let known_dirs = KnownDirs::default();
    let config_sources = known_dirs.config_dirs().map(|dir| dir.join("me3.toml"));

    let options = config_sources
        .inspect(|path| debug!(?path, "searching for me3.toml in"))
        .flat_map(Options::from_file)
        .chain(iter::once(cli.config))
        .fold(Options::default(), |a, b| a.merge(b));

    let config = Config {
        known_dirs,
        options,
    };

    let log_file = tempfile::tempfile().unwrap();

    let telemetry_config = TelemetryConfig::default()
        .enabled(config.options.crash_reporting.unwrap_or(false))
        .with_console_writer(stderr)
        .with_file_writer(log_file);

    let _telemetry_guard = me3_telemetry::install(telemetry_config);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        commit_id = option_env!("BUILD_COMMIT_ID").unwrap_or("unknown")
    );

    let db = DbContext::new(&config);

    let result = me3_telemetry::with_root_span("me3", "run command", || match cli.command {
        Commands::Info => commands::info::info(config),
        Commands::Launch(args) => commands::launch::launch(db, config, args),
        Commands::Profile(ProfileCommands::Create(args)) => commands::profile::create(config, args),
        Commands::Profile(ProfileCommands::List) => commands::profile::list(db),
        Commands::Profile(ProfileCommands::Show(args)) => commands::profile::show(db, config, args),
        Commands::Profile(ProfileCommands::Upgrade(args)) => {
            commands::profile::upgrade(db, config, args)
        }
        #[cfg(target_os = "windows")]
        Commands::AddToPath => commands::windows::add_to_path(),
        #[cfg(target_os = "windows")]
        Commands::Update => commands::windows::update(),
    });

    if result.is_err() {
        std::process::exit(1);
    }
}
