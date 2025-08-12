use color_eyre::owo_colors::OwoColorize;

use crate::{config::Config, output::OutputBuilder};

fn format_path<P: AsRef<std::path::Path>>(path: Option<P>) -> String {
    match path {
        None => "<none>".red().to_string(),
        Some(path) => path.as_ref().to_string_lossy().to_string(),
    }
}

fn format_status(status: bool) -> String {
    if status {
        "Found".green().to_string()
    } else {
        "Not found".red().to_string()
    }
}

pub fn info(config: Config) -> color_eyre::Result<()> {
    let mut output = OutputBuilder::new("Configuration");

    for (game, config) in &config.options.game {
        output.section(game.title(), |builder| {
            if let Some(boot_boost) = config.boot_boost {
                builder.property("Boot Boost", boot_boost);
            }

            if let Some(skip_logos) = config.skip_logos {
                builder.property("Skip startup logos?", skip_logos);
            }

            if let Some(disable_arxan) = config.disable_arxan {
                builder.property("Neutralize Arxan code protection", disable_arxan);
            }

            if let Some(skip_steam_init) = config.skip_steam_init {
                builder.property("Skip Steam init?", format_status(skip_steam_init));
            }

            if let Some(exe) = &config.exe {
                builder.property("Executable", exe.to_string_lossy());
            }
        });
    }

    output.property("Profile directory", format_path(config.profile_dir()));
    output.property("Logs directory", format_path(config.log_dir()));

    output.section("Configuration search paths", |builder| {
        for (index, item) in config.known_dirs.config_dirs().enumerate() {
            builder.property(format!("{index}"), item.join("me3.toml").to_string_lossy());
        }
    });

    #[cfg(target_os = "windows")]
    output.section("Installation", |builder| {
        let installation = config.known_dirs.installation.clone();
        builder.property("Status", format_status(installation.is_some()));

        if let Some(install) = installation {
            builder.property("Installation prefix", install.to_string_lossy());
        }
    });

    let steam = config.steam_dir();

    output.section("Steam", |builder| {
        builder.property("Status", format_status(steam.is_ok()));

        if let Ok(steam) = steam {
            builder.property("Path", steam.path().to_string_lossy());
        }
    });

    print!("{}", output.build());

    Ok(())
}
