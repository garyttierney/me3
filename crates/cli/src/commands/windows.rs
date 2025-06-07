use std::{
    io::{BufRead, BufReader},
    time::Duration,
};

use color_eyre::eyre::{Context, OptionExt};
use tracing::info;
use update_informer::{registry, Check};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

pub fn add_to_path() -> color_eyre::Result<()> {
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

pub fn update() -> color_eyre::Result<()> {
    let informer = update_informer::new(
        registry::GitHub,
        "garyttierney/me3",
        env!("CARGO_PKG_VERSION"),
    )
    .interval(Duration::ZERO);

    if let Some(version) = informer.check_version().ok().flatten() {
        println!("New version is available: {version}");

        let installer_url = format!(
            "https://github.com/garyttierney/me3/releases/download/{version}/me3_installer.exe"
        );
        info!(installer_url, "Downloading installer");

        let mut response = ureq::get(installer_url)
            .header("User-Agent", "me3-cli")
            .call()?;

        let mut installer_file: tempfile::NamedTempFile = tempfile::Builder::new()
            .disable_cleanup(true)
            .suffix(".exe")
            .prefix("me3_installer")
            .rand_bytes(3)
            .tempfile()?;

        info!(
            installer_path = ?installer_file.path(),
            "Saved installer file"
        );

        let mut body_reader = BufReader::new(response.body_mut().as_reader());
        std::io::copy(&mut body_reader, &mut installer_file)?;

        let mut installer_path = installer_file.into_temp_path();
        installer_path.disable_cleanup(true);

        open::that_detached(installer_path)?;
    } else {
        println!("me3 is up-to-date!");
    }

    Ok(())
}
