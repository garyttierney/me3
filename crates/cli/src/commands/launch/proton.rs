use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use normpath::PathExt;
use serde::Deserialize;
use steamlocate::SteamDir;
use tracing::info;

pub struct CompatTools<'a> {
    steam: &'a SteamDir,
    search_paths: Vec<Box<Path>>,
}

impl<'a> CompatTools<'a> {
    pub fn new(steamdir: &'a SteamDir) -> Self {
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

    pub fn find(&self, name: impl AsRef<str>) -> Option<CompatTool> {
        let name = name.as_ref();
        let tool_appid = match name {
            "proton_experimental" => 1493710,
            "proton_hotfix" => 2180100,
            "proton_9" => 2805730,
            "proton_10" => 3658110,
            name => {
                let tools = CompatTools::new(self.steam);
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
    require_tool_app_id: u32,
    use_sessions: bool,
    compatmanager_layer_name: String,
}
