use std::path::PathBuf;

use schemars::{schema_for, JsonSchema};
use serde::Deserialize;

use crate::{
    dependency::Dependent,
    mod_file::ModFile,
    native::{Native, NativeInitializerCondition, NativeInitializerDelay},
    package::Package,
    Game,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "ModProfileV1Layout")]
pub struct ModProfileV1 {
    /// The games that this profile supports.
    #[serde(default)]
    pub supports: Vec<Supports>,

    /// Native modules (DLLs) that will be loaded.
    #[serde(default)]
    #[serde(alias = "native")]
    pub natives: Vec<Native>,

    /// A collection of packages containing assets that should be considered for loading
    /// before the DVDBND.
    #[serde(default)]
    #[serde(alias = "package")]
    pub packages: Vec<Package>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    #[serde(default)]
    pub savefile: Option<String>,

    /// Starts the game with multiplayer server connectivity enabled.
    #[serde(default)]
    pub start_online: Option<bool>,

    /// Try to neutralize Arxan GuardIT code protection to improve mod stability.
    #[serde(default)]
    pub disable_arxan: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct Supports {
    #[serde(rename = "game")]
    pub game: Game,

    #[serde(rename = "since")]
    pub since_version: Option<String>,
}

#[derive(Default, Deserialize, JsonSchema)]
struct ModProfileV1Layout {
    #[serde(default)]
    pub supports: Vec<Supports>,
    #[serde(default)]
    #[serde(alias = "native")]
    pub natives: Vec<NativeV1>,
    #[serde(default)]
    #[serde(alias = "package")]
    pub packages: Vec<PackageV1>,
    #[serde(default)]
    pub savefile: Option<String>,
    #[serde(default)]
    pub start_online: Option<bool>,
    #[serde(default)]
    pub disable_arxan: Option<bool>,
}

fn on() -> bool {
    true
}

fn off() -> bool {
    false
}

#[derive(Deserialize, JsonSchema)]
enum NativeInitializerConditionV1 {
    #[serde(rename = "delay")]
    Delay { ms: usize },
    #[serde(rename = "function")]
    Function(String),
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema)]
struct NativeV1 {
    path: ModFileV1,
    #[serde(default = "off")]
    optional: bool,
    #[serde(default = "on")]
    enabled: bool,
    #[serde(default)]
    load_before: Vec<Dependent<String>>,
    #[serde(default)]
    load_after: Vec<Dependent<String>>,
    initializer: Option<NativeInitializerConditionV1>,
    finalizer: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema)]
pub struct PackageV1 {
    id: Option<String>,
    #[serde(default = "on")]
    enabled: bool,
    #[serde(alias = "source")]
    path: ModFileV1,
    #[serde(default)]
    load_after: Vec<Dependent<String>>,
    #[serde(default)]
    load_before: Vec<Dependent<String>>,
}

#[derive(Deserialize, JsonSchema)]
struct ModFileV1(PathBuf);

impl From<ModProfileV1Layout> for ModProfileV1 {
    fn from(layout: ModProfileV1Layout) -> Self {
        Self {
            supports: layout.supports,
            natives: layout.natives.into_iter().map(Into::into).collect(),
            packages: layout.packages.into_iter().map(Into::into).collect(),
            savefile: layout.savefile,
            start_online: layout.start_online,
            disable_arxan: layout.disable_arxan,
        }
    }
}

impl From<NativeV1> for Native {
    fn from(value: NativeV1) -> Self {
        let item = ModFile {
            enabled: value.enabled,
            optional: value.optional,
            ..value.path.0.into()
        };

        let initializer = match value.initializer {
            Some(NativeInitializerConditionV1::Delay { ms }) => Some(NativeInitializerCondition {
                delay: Some(NativeInitializerDelay { ms }),
                function: None,
            }),
            Some(NativeInitializerConditionV1::Function(name)) => {
                Some(NativeInitializerCondition {
                    delay: None,
                    function: Some(name),
                })
            }
            None => None,
        };

        Self {
            initializer,
            load_before: value.load_before,
            load_after: value.load_after,
            ..item.into()
        }
    }
}

impl From<PackageV1> for Package {
    fn from(value: PackageV1) -> Self {
        let mut item = ModFile {
            enabled: value.enabled,
            ..value.path.0.into()
        };

        if let Some(id) = value.id {
            item.name = id;
        }

        Self {
            load_before: value.load_before,
            load_after: value.load_after,
            ..item.into()
        }
    }
}

impl JsonSchema for ModProfileV1 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ModProfileV1".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schema_for!(ModProfileV1Layout)
    }
}
