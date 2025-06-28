use std::{io::BufReader, str::FromStr};

use color_eyre::eyre::{Context, OptionExt};
use semver::Version;
use tracing::{error, info};
use windows::Win32::System::Console::{
    GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_PROCESSED_OUTPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
};
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
    const RELEASE_URI: &str = "https://api.github.com/repos/garyttierney/me3/releases/latest";
    let response = ureq::get(RELEASE_URI)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "me3-cli")
        .call()?;

    if !response.status().is_success() {
        error!(
            "unable to check latest version, check https://github.com/garyttierney/me3/releases/latest"
        );
        return Ok(());
    }

    let body = response.into_body().into_reader();
    let release: serde_json::Value = serde_json::from_reader(body)?;

    let current_version =
        Version::from_str(env!("CARGO_PKG_VERSION")).expect("cargo version is incorrect");

    let latest_version = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .and_then(|tag_name| Version::from_str(tag_name.strip_prefix('v').unwrap_or(tag_name)).ok())
        .ok_or_eyre("no tag_name in latest GitHub release")?;

    if latest_version > current_version {
        println!("New version is available: {latest_version}");

        let installer_url = format!(
            "https://github.com/garyttierney/me3/releases/download/v{latest_version}/me3_installer.exe"
        );

        info!(installer_url, "Downloading installer");

        let mut response = ureq::get(&installer_url)
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

        let mut body_reader = BufReader::new(response.into_body().into_reader());
        std::io::copy(&mut body_reader, &mut installer_file)?;

        let mut installer_path = installer_file.into_temp_path();
        installer_path.disable_cleanup(true);

        open::that_detached(installer_path)?;
    } else {
        println!("me3 is up-to-date!");
    }

    Ok(())
}

pub fn enable_ansi() -> color_eyre::Result<()> {
    unsafe {
        let console = GetStdHandle(STD_OUTPUT_HANDLE)?;

        let mut mode = ENABLE_PROCESSED_OUTPUT;
        GetConsoleMode(console, &mut mode)?;

        SetConsoleMode(
            console,
            mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        )?;

        Ok(())
    }
}
