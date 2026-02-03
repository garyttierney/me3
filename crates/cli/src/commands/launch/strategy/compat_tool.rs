use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsString,
    fs::File,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::eyre::{bail, eyre, OptionExt};
use normpath::PathExt;
use serde::Deserialize;
use steamlocate::{Library, SteamDir};
use tracing::{debug, info};

use crate::commands::launch::{
    steam::{SteamInputConfig, SteamUserConfig, SteamUsers},
    strategy::LaunchStrategy,
};

pub fn active_mounts() -> std::io::Result<Vec<PathBuf>> {
    let file = File::open("/proc/mounts")?;
    let reader = BufReader::new(file);
    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 2 {
            mounts.push(PathBuf::from(parts[1]));
        }
    }

    mounts.sort_by_key(|a| Reverse(a.as_os_str().len()));

    Ok(mounts)
}

pub struct CompatTools {
    steam: SteamDir,
    search_paths: Vec<Box<Path>>,
}

impl CompatTools {
    pub fn new(steamdir: SteamDir) -> Self {
        let mut search_paths = vec![
            Box::from(Path::new("/usr/share/steam/compatibilitytools.d")),
            Box::from(Path::new("/usr/local/share/steam/compatibilitytools.d")),
        ];

        let extra_search_paths = std::env::var("STEAM_EXTRA_COMPAT_TOOLS_PATHS")
            .into_iter()
            .flat_map(|paths| {
                paths
                    .split(':')
                    .map(|path| Box::from(Path::new(path)))
                    .collect::<Vec<_>>()
            });

        search_paths.extend(extra_search_paths);
        search_paths.push(
            steamdir
                .path()
                .join("compatibilitytools.d")
                .into_boxed_path(),
        );

        Self {
            steam: steamdir,
            search_paths,
        }
    }

    pub fn all(&self) -> impl Iterator<Item = CompatTool> {
        self.search_paths.iter().flat_map(|path| {
            std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    let dir = entry.ok()?;
                    let manifest_path = dir.path().join("compatibilitytool.vdf");

                    manifest_path.exists().then_some(manifest_path)
                })
                .flat_map(CompatTool::load)
        })
    }

    pub fn find_by_id(&self, app_id: u32) -> Option<CompatTool> {
        let (tool_app, tool_library) = self.steam.find_app(app_id).ok().flatten()?;
        let tool_dir = tool_library.resolve_app_dir(&tool_app);

        Some(CompatTool {
            name: tool_app.name.clone().unwrap(),
            display_name: tool_app.name.unwrap(),
            install_path: tool_dir.into(),
        })
    }

    pub fn find(&self, name: impl AsRef<str>) -> Option<CompatTool> {
        let name = name.as_ref();
        let tool_appid = match name {
            "proton_experimental" => 1493710,
            "proton_hotfix" => 2180100,
            "proton_6" => 1580130,
            "proton_7" => 1887720,
            "proton_8" => 2348590,
            "proton_9" => 2805730,
            "proton_10" => 3658110,
            name => {
                let tools = CompatTools::new(self.steam.clone());
                let installations = tools.all();

                return installations
                    .filter(|tool| tool.name == name)
                    .inspect(|tool| info!(path=?tool.install_path, "found compat tool"))
                    .last();
            }
        };

        let (tool_app, tool_library) = self.steam.find_app(tool_appid).ok().flatten()?;
        let tool_dir = tool_library.resolve_app_dir(&tool_app);

        Some(CompatTool {
            name: name.to_string(),
            display_name: name.to_string(),
            install_path: tool_dir.into(),
        })
    }
}

pub struct CompatToolLaunchStrategy {
    pub app_id: u32,
    pub steam: SteamDir,
    pub library: Library,
    pub install_dir: PathBuf,
    pub all_tools: CompatTools,
    pub launch_tool: CompatTool,
    pub base_dirs: Vec<PathBuf>,
}

impl CompatToolLaunchStrategy {
    fn setup_steam_linux_runtime_env(
        command: &mut Command,
        steam: SteamDir,
        install_dir: PathBuf,
        library: Library,
        tool_paths: Vec<String>,
        app_id: u32,
        base_dirs: Vec<PathBuf>,
    ) -> color_eyre::Result<()> {
        // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/docs/steam-compat-tool-interface.md>
        command.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", steam.path());
        command.env("STEAM_COMPAT_INSTALL_PATH", install_dir);
        command.env("STEAM_COMPAT_TOOL_PATH", tool_paths.join(":"));
        command.env("STEAM_COMPAT_LIBRARY_PATHS", library.path());
        let prefix_path = steam
            .library_paths()?
            .into_iter()
            .map(|path| path.join(format!("steamapps/compatdata/{}", app_id)))
            .find(|path| path.exists())
            .unwrap_or_else(|| {
                library
                    .path()
                    .join(format!("steamapps/compatdata/{}", app_id))
            });

        let steam_user_config = SteamUsers::open(steam.path()).ok().and_then(|users| {
            let user = users.active()?;
            SteamUserConfig::open(steam.path(), user).ok()
        });

        let steam_input_status = steam_user_config
            .as_ref()
            .and_then(|config| config.apps.get(&app_id)?.use_steam_controller_config)
            .unwrap_or(SteamInputConfig::Default);

        if steam_input_status != SteamInputConfig::ForceOff {
            const STEAM_INPUT_VIRTUAL_DEV_ID: &str = "0x28DE/0x0000";
            command.env(
                "SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT",
                STEAM_INPUT_VIRTUAL_DEV_ID,
            );
        }

        let active_mount_points = active_mounts()?;
        let mut used_mount_points = HashSet::new();

        for base_dir in base_dirs {
            let mount_point = active_mount_points
                .iter()
                .find(|mount| base_dir.starts_with(mount))
                .cloned();

            used_mount_points.extend(mount_point);
        }

        let compat_mounts = used_mount_points
            .into_iter()
            .map(|path| path.into_os_string())
            .fold(OsString::new(), |mut lhs, rhs| {
                lhs.push(rhs);
                lhs.push(":");
                lhs
            });

        debug!("mounting additional filesystems for SLR: {compat_mounts:?}");

        command.env("STEAM_COMPAT_MOUNTS", compat_mounts);
        command.env("STEAM_COMPAT_DATA_PATH", prefix_path);
        command.env("STEAM_COMPAT_APP_ID", app_id.to_string());

        let mut ld_preload = steam
            .path()
            .join("ubuntu12_64/gameoverlayrenderer.so")
            .into_os_string();

        if let Some(existing_ld_preload) = std::env::var_os("LD_PRELOAD") {
            ld_preload.push(" ");
            ld_preload.push(&existing_ld_preload);
        }

        Ok(())
    }
}

impl LaunchStrategy for CompatToolLaunchStrategy {
    fn build_command(self, exe: &Path, exe_args: Vec<OsString>) -> color_eyre::Result<Command> {
        let mut args = VecDeque::default();
        let Self {
            launch_tool: mut tool,
            library,
            steam,
            app_id,
            install_dir,
            mut base_dirs,
            ..
        } = self;

        const LAUNCH_VERB: &str = "waitforexitandrun";
        let mut tool_paths = vec![];

        loop {
            tool_paths.push(tool.install_path.to_string_lossy().to_string());
            let tool_manifest = tool.manifest()?;
            let tool_command = match tool_manifest.version {
                1 => tool_manifest.commandline.clone(),
                2 | _ => tool_manifest.commandline.replace("%verb%", LAUNCH_VERB),
            };

            let Some(mut tool_args) = shlex::split(&tool_command) else {
                bail!("Couldn't parse compat tool command {tool_command}")
            };

            if tool_args[0].starts_with('/') {
                tool_args[0].insert_str(0, &tool.install_path.to_string_lossy());
            }

            for arg in tool_args.into_iter().rev() {
                args.push_front(arg);
            }

            debug!("Configuring tool {tool_manifest:#?}");

            let Some(parent_tool_id) = tool_manifest.require_tool_appid else {
                break;
            };

            let parent_tool = self.all_tools.find_by_id(parent_tool_id).ok_or_else(|| {
                eyre!(
                    "Required tool with app id {parent_tool_id} for {} couldn't be found",
                    tool.name
                )
            })?;

            tool = parent_tool;
        }

        let mut command = Command::new(
            args.pop_front()
                .ok_or_eyre("Compat Tool produced invalid command")?,
        );
        command.args(args);
        command.arg(exe);
        command.arg("--");
        command.args(exe_args);

        base_dirs.push(exe.parent().unwrap().to_path_buf());

        Self::setup_steam_linux_runtime_env(
            &mut command,
            steam,
            install_dir,
            library,
            tool_paths,
            app_id,
            base_dirs,
        )?;
        Ok(command)
    }
}

#[derive(Debug)]
pub struct CompatTool {
    pub name: String,
    pub display_name: String,
    pub install_path: Box<Path>,
}

impl CompatTool {
    pub fn manifest(&self) -> color_eyre::Result<CompatToolManifest> {
        let path = self.install_path.join("toolmanifest.vdf");
        let file = File::open(path)?;
        let manifest: CompatToolManifest = keyvalues_serde::from_reader(file)?;

        Ok(manifest)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CompatToolLoadError {
    #[error("compatibilitytool.vdf contains no compat tool entry")]
    NoEntry,

    #[error("compatibilitytool.vdf contains multiple entries")]
    TooManyEntries,

    #[error("failed to parse vdf")]
    ParseError(#[from] Box<keyvalues_serde::Error>),

    #[error("unexpected IO error: {inner}")]
    Other {
        #[from]
        inner: std::io::Error,
    },
}
impl CompatTool {
    pub fn load(path: impl AsRef<Path>) -> color_eyre::Result<Self, CompatToolLoadError> {
        let vdf_path = path.as_ref();
        let file = File::open(vdf_path)?;

        let mut vdf: CompatToolVdf = keyvalues_serde::from_reader(file).map_err(Box::from)?;
        if vdf.compat_tools.len() > 1 {
            return Err(CompatToolLoadError::TooManyEntries);
        }

        let (name, info) = vdf
            .compat_tools
            .drain()
            .next()
            .ok_or(CompatToolLoadError::NoEntry)?;

        let display_name = info.display_name;
        let install_path = if info.install_path.is_absolute() {
            info.install_path
        } else {
            vdf_path
                .parent()
                .ok_or(std::io::Error::other("parent folder disappeared"))?
                .normalize()?
                .join(info.install_path)
                .into_path_buf()
        };

        Ok(CompatTool {
            name,
            display_name,
            install_path: install_path.into_boxed_path(),
        })
    }
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
struct CompatToolVdf {
    pub compat_tools: HashMap<String, CompatToolInfo>,
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
struct CompatToolInfo {
    install_path: PathBuf,
    display_name: String,
    from_oslist: String,
    to_oslist: String,
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct CompatToolManifest {
    version: u32,
    commandline: String,
    require_tool_appid: Option<u32>,
    use_sessions: Option<bool>,
    compatmanager_layer_name: String,
}
