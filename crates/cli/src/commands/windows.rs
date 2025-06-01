use color_eyre::eyre::{Context, OptionExt};
use tracing::info;
use winreg::enums::{KEY_READ, KEY_WRITE};

pub fn add_to_path() -> color_eyre::Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hklm = RegKey::predef(HKEY_CURRENT_USER);
    let environment = hklm
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("couldn't find Environment regkey")?;

    let path: String = environment.get_value("Path").ok().unwrap_or_default();
    let mut path_entries: Vec<&str> = path.split(';').collect();

    info!("current path: {path}");

    let current_exe = std::env::current_exe()?;
    let current_exe_dir = current_exe
        .parent()
        .ok_or_eyre("unable to determine binary path")?
        .to_string_lossy();

    if path_entries.contains(&&*current_exe_dir) {
        info!("already on path");
        return Ok(());
    }

    info!("current exe dir: {current_exe_dir}");
    path_entries.push(&current_exe_dir);

    let new_path = path_entries.join(";");
    environment.set_value("Path", &new_path)?;

    Ok(())
}
