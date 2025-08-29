use std::path::PathBuf;

use indexmap::IndexMap;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{
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
        let file_path = match &dependency {
            ProfileDependency::Simple(path) => path,
            ProfileDependency::Full { path, .. } => path,
        };

        let file_name = file_path
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
    Full {
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
    },
}

impl ProfileDependency {
    fn into_parts(self) -> (PathBuf, bool, bool, Option<NativeInitializerCondition>) {
        match self {
            Self::Simple(path) => (
                path,
                Self::enabled_default(),
                Self::optional_default(),
                None,
            ),
            Self::Full {
                path,
                enabled,
                optional,
                initializer,
            } => (path, enabled, optional, initializer),
        }
    }

    fn from_parts(
        path: PathBuf,
        enabled: bool,
        optional: bool,
        initializer: Option<NativeInitializerCondition>,
    ) -> Self {
        if enabled == Self::enabled_default()
            && optional == Self::optional_default()
            && initializer.is_none()
        {
            Self::Simple(path)
        } else {
            Self::Full {
                path,
                enabled,
                optional,
                initializer,
            }
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
        let (path, enabled, optional, initializer) = dependency.into_parts();
        Self {
            inner: ModFile {
                name,
                path,
                enabled,
                optional,
            },
            initializer,
        }
    }
}

impl From<Native> for (String, ProfileDependency) {
    fn from(native: Native) -> Self {
        (
            native.inner.name,
            ProfileDependency::from_parts(
                native.inner.path,
                native.inner.enabled,
                native.inner.optional,
                native.initializer,
            ),
        )
    }
}

impl From<(String, ProfileDependency)> for Package {
    fn from(dependency: (String, ProfileDependency)) -> Self {
        ModFile::from(dependency).into()
    }
}

impl From<Package> for (String, ProfileDependency) {
    fn from(package: Package) -> Self {
        package.0.into()
    }
}

impl From<(String, ProfileDependency)> for ModFile {
    fn from((name, dependency): (String, ProfileDependency)) -> Self {
        let (path, enabled, optional, _) = dependency.into_parts();
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
            ProfileDependency::from_parts(item.path, item.enabled, item.optional, None),
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
