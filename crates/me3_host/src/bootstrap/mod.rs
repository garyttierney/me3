use std::error::Error;

use config::Config;
use futures::channel::mpsc::channel;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{bootstrap::settings::SettingsBuilder, widgets::Console};

mod game;
mod settings;

#[derive(Serialize, Deserialize)]
pub struct BootstrapInfo {
    #[serde(default = "Vec::new")]
    config_files: Vec<String>,
}

pub fn setup_and_run() -> Result<(), Box<dyn Error>> {
    let framework = me3_framework::FrameworkBuilder::default()
        .debug_console(cfg!(debug_assertions))
        .build()?;

    log::info!("Setup me3 v{}", env!("CARGO_PKG_VERSION"));

    let bootstrap_info = Config::builder()
        .add_source(
            config::Environment::with_prefix("ME3")
                .try_parsing(true)
                .separator("_")
                .list_separator(";"),
        )
        .build()
        .and_then(|config| config.try_deserialize::<BootstrapInfo>())?;

    log::info!(
        "Loading settings from config files: {:#?}",
        &bootstrap_info.config_files
    );

    let settings_builder = bootstrap_info.config_files.into_iter().fold(
        SettingsBuilder::default(),
        |builder, file| -> SettingsBuilder {
            match builder.add_file(&file, true) {
                Ok(builder) => builder,
                Err((builder, error)) => {
                    log::warn!("failed to load settings from {}: {:#?}", file, error);
                    builder
                }
            }
        },
    );

    let settings = settings_builder.build()?;

    log::info!("Loaded settings: {}", settings);

    let script_host = framework.get_script_host();
    let vfs = framework.get_vfs();

    for (name, config) in settings.mods() {
        if !config.enabled {
            log::info!("skipping disabled mod: {}", name);
        }

        log::info!("initializing mod: {}", name);
        // Get the override folder/scripts and make them absolute based on the
        // path we found this mod config file in.
        let mod_root = settings.mod_path(name).unwrap_or_default();

        if let Some(override_path) = config.file_root {
            let mut override_root = mod_root.clone();
            override_root.push(override_path);

            vfs.add_override_root(override_root);
        }

        for script in &config.scripts {
            let mut script_path = mod_root.clone();
            script_path.push(script);

            match script_host.load_script(script_path) {
                Ok(_) => {
                    log::info!("succesfully loaded script at: {}", script);
                }
                Err(e) => {
                    log::warn!("failed to load script {}, error: {:#?}", script, e)
                }
            }
        }
    }

    let _game = game::bootstrap_game(&framework).expect("unable to determine game");
    let overlay = framework.get_overlay();

    let (mut command_output_tx, command_output_rx) = channel::<String>(1024);
    let (command_tx, mut command_rx) = channel::<String>(1);

    framework.spawn(async move {
        while let Some(command) = command_rx.next().await {
            log::info!("wanted to execute {}", &command);

            let _ = command_output_tx.send(command).await;
        }
    });

    let mut console = Console::new(command_tx, command_output_rx);
    overlay.register_component(move |ctx| console.render(ctx));

    framework.run_until_shutdown();
    Ok(())
}
