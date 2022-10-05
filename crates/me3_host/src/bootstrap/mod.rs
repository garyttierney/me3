use std::error::Error;

use self::settings::SettingsBuilder;

mod settings;

pub fn setup_and_run() -> Result<(), Box<dyn Error>> {
    let framework = me3_framework::FrameworkBuilder::default()
        .debug_console(cfg!(debug_assertions))
        .build()?;

    let settings = SettingsBuilder::default()
        .add_file("me3_settings", false)?
        .build()?;

    let vfs = framework.get_vfs();

    for (name, config) in settings.mods() {
        // Get the override folder and make it an absolute path based on the
        // folder the config file was located in.
        if let Some(override_path) = config.file_root {
            let mut mod_root = settings.mod_path(name).unwrap_or_default();
            mod_root.push(override_path);

            vfs.add_override_root(mod_root);
        }
    }

    framework.run_until_shutdown();
    Ok(())
}
