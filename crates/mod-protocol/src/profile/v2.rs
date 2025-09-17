use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{
    dependency::Dependent,
    mod_file::ModFile,
    native::{Native, NativeInitializerCondition},
    package::Package,
    Game,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(from = "ModProfileV2Layout", into = "ModProfileV2Layout")]
pub struct ModProfileV2 {
    /// The game that this profile supports.
    pub supports: Option<Game>,

    /// Native modules (DLLs) that will be loaded.
    pub natives: Vec<Native>,

    /// A collection of packages containing assets to be added to the virtual file system.
    pub packages: Vec<Package>,

    /// Other profiles listed as dependencies by this profile.
    pub profiles: Vec<ModFile>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    pub savefile: Option<String>,

    /// Starts the game with multiplayer server connectivity enabled.
    pub start_online: Option<bool>,

    /// Try to neutralize Arxan GuardIT code protection to improve mod stability.
    pub disable_arxan: Option<bool>,
}

impl ModProfileV2 {
    pub(super) fn push_dependency(&mut self, (name, dependency): (String, ProfileDependency)) {
        let file_name = dependency
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();

        if file_name.ends_with(".dll") {
            self.natives.push((name, dependency).into());
        } else if file_name.ends_with(".me3")
            || file_name.ends_with(".me3.toml")
            || file_name.ends_with(".me3.json")
        {
            self.profiles.push((name, dependency).into());
        } else {
            self.packages.push((name, dependency).into());
        }
    }
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
struct ModProfileV2Layout {
    #[serde(default)]
    game: ProfileGame,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    dependencies: IndexMap<String, ProfileDependency>,
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
struct ProfileGame {
    #[serde(default)]
    launch: Option<Game>,

    #[serde(default)]
    savefile: Option<String>,

    #[serde(default)]
    start_online: Option<bool>,

    #[serde(default)]
    disable_arxan: Option<bool>,
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ProfileDependency {
    Simple(PathBuf),
    Full(FullProfileDependency),
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub struct FullProfileDependency {
    path: PathBuf,

    #[serde(
        default = "ProfileDependency::enabled_default",
        skip_serializing_if = "ProfileDependency::enabled_is_default"
    )]
    enabled: bool,

    #[serde(
        default = "ProfileDependency::optional_default",
        skip_serializing_if = "ProfileDependency::optional_is_default"
    )]
    optional: bool,

    #[serde(default)]
    initializer: Option<NativeInitializerCondition>,

    #[serde(default)]
    load_before: Vec<Dependent<String>>,

    #[serde(default)]
    load_after: Vec<Dependent<String>>,
}

impl ProfileDependency {
    pub fn path(&self) -> &Path {
        match self {
            Self::Simple(path) => path,
            Self::Full(full) => &full.path,
        }
    }

    fn enabled_default() -> bool {
        true
    }

    fn enabled_is_default(enabled: &bool) -> bool {
        *enabled == Self::enabled_default()
    }

    fn optional_default() -> bool {
        false
    }

    fn optional_is_default(optional: &bool) -> bool {
        *optional == Self::optional_default()
    }
}

impl From<ProfileDependency> for FullProfileDependency {
    fn from(value: ProfileDependency) -> Self {
        match value {
            ProfileDependency::Simple(path) => Self {
                path,
                enabled: ProfileDependency::enabled_default(),
                optional: ProfileDependency::optional_default(),
                initializer: None,
                load_before: vec![],
                load_after: vec![],
            },
            ProfileDependency::Full(full) => full,
        }
    }
}

impl From<FullProfileDependency> for ProfileDependency {
    fn from(value: FullProfileDependency) -> Self {
        if value.enabled == Self::enabled_default()
            && value.optional == Self::optional_default()
            && value.initializer.is_none()
            && value.load_before.is_empty()
            && value.load_after.is_empty()
        {
            Self::Simple(value.path)
        } else {
            Self::Full(value)
        }
    }
}

impl From<ModProfileV2Layout> for ModProfileV2 {
    fn from(layout: ModProfileV2Layout) -> Self {
        let mut profile = Self {
            supports: layout.game.launch,
            savefile: layout.game.savefile,
            start_online: layout.game.start_online,
            disable_arxan: layout.game.disable_arxan,
            ..Default::default()
        };

        for dep in layout.dependencies {
            profile.push_dependency(dep);
        }

        profile
    }
}

impl From<ModProfileV2> for ModProfileV2Layout {
    fn from(profile: ModProfileV2) -> Self {
        let mut dependencies = IndexMap::new();

        dependencies.extend(profile.natives.into_iter().map(Into::into));
        dependencies.extend(profile.packages.into_iter().map(Into::into));
        dependencies.extend(profile.profiles.into_iter().map(Into::into));

        Self {
            game: ProfileGame {
                launch: profile.supports,
                savefile: profile.savefile,
                start_online: profile.start_online,
                disable_arxan: profile.disable_arxan,
            },
            dependencies,
        }
    }
}

impl From<(String, ProfileDependency)> for Native {
    fn from((name, dependency): (String, ProfileDependency)) -> Self {
        let FullProfileDependency {
            path,
            enabled,
            optional,
            initializer,
            load_before,
            load_after,
        } = dependency.into();

        Self {
            inner: ModFile {
                name,
                path,
                enabled,
                optional,
            },
            load_before,
            load_after,
            initializer,
        }
    }
}

impl From<Native> for (String, ProfileDependency) {
    fn from(native: Native) -> Self {
        (
            native.inner.name,
            FullProfileDependency {
                path: native.inner.path,
                enabled: native.inner.enabled,
                optional: native.inner.optional,
                initializer: native.initializer,
                load_before: native.load_before,
                load_after: native.load_after,
            }
            .into(),
        )
    }
}

impl From<(String, ProfileDependency)> for Package {
    fn from((name, dependency): (String, ProfileDependency)) -> Self {
        let FullProfileDependency {
            path,
            enabled,
            optional,
            load_before,
            load_after,
            ..
        } = dependency.into();

        Self {
            inner: ModFile {
                name,
                path,
                enabled,
                optional,
            },
            load_before,
            load_after,
        }
    }
}

impl From<Package> for (String, ProfileDependency) {
    fn from(package: Package) -> Self {
        (
            package.inner.name,
            FullProfileDependency {
                path: package.inner.path,
                enabled: package.inner.enabled,
                optional: package.inner.optional,
                initializer: None,
                load_before: package.load_before,
                load_after: package.load_after,
            }
            .into(),
        )
    }
}

impl From<(String, ProfileDependency)> for ModFile {
    fn from((name, dependency): (String, ProfileDependency)) -> Self {
        let FullProfileDependency {
            path,
            enabled,
            optional,
            ..
        } = dependency.into();

        Self {
            name,
            path,
            enabled,
            optional,
        }
    }
}

impl From<ModFile> for (String, ProfileDependency) {
    fn from(item: ModFile) -> Self {
        (
            item.name,
            FullProfileDependency {
                path: item.path,
                enabled: item.enabled,
                optional: item.optional,
                initializer: None,
                load_before: vec![],
                load_after: vec![],
            }
            .into(),
        )
    }
}

impl JsonSchema for ModProfileV2 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ModProfileV2".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schema_for!(ModProfileV2Layout)
    }
}
