use std::path::PathBuf;

use color_eyre::owo_colors::OwoColorize;

use crate::{output::OutputBuilder, AppInstallInfo, AppPaths, Config};

fn format_path(path: Option<PathBuf>) -> String {
    match path {
        None => "<none>".red().to_string(),
        Some(path) => path.to_string_lossy().to_string(),
    }
}

fn format_status(status: bool) -> String {
    let status = if status {
        "Found".green().to_string()
    } else {
        "Not found".red().to_string()
    };

    status
}

pub fn info(
    info: Option<AppInstallInfo>,
    paths: AppPaths,
    config: Config,
) -> color_eyre::Result<()> {
    let mut output = OutputBuilder::new("Configuration");

    output.property("Profile directory", format_path(config.profile_dir.clone()));
    output.property("Logs directory", format_path(paths.logs_path));

    output.section("Search paths", |builder| {
        builder.property(
            "System configuration",
            format_path(paths.system_config_path),
        );

        builder.property("User configuration", format_path(paths.user_config_path));
        builder.property("CLI configuration", format_path(paths.cli_config_path));
    });

    output.section("Installation", |builder| {
        builder.property("Status", format_status(info.is_some()));

        if let Some(install) = info {
            builder.property("Config directory", install.config_path.to_string_lossy());
            builder.property("Installation prefix", install.prefix.to_string_lossy());
        }
    });

    let steam = config.resolve_steam_dir();

    output.section("Steam", |builder| {
        builder.property("Status", format_status(steam.is_ok()));

        if let Ok(steam) = steam {
            builder.property("Path", steam.path().to_string_lossy());
        }
    });

    print!("{}", output.build());

    Ok(())
}
