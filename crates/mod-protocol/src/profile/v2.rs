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
    pub(super) fn push_mod_entry<E: Into<ModEntryV2>>(&mut self, mod_entry: E) {
        match mod_entry.into() {
            ModEntryV2::Native(native) => self.natives.push(native),
            ModEntryV2::Package(package) => self.packages.push(package),
            ModEntryV2::Profile(profile) => self.profiles.push(profile),
        }
    }
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
struct ModProfileV2Layout {
    #[serde(default)]
    game: GamePropertiesV2,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    mods: IndexMap<String, ModEntryV2Layout>,
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
struct GamePropertiesV2 {
    launch: Option<Game>,
    savefile: Option<String>,
    start_online: Option<bool>,
    disable_arxan: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum ModEntryV2 {
    Native(Native),
    Package(Package),
    Profile(ModFile),
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind")]
enum ModEntryV2Layout {
    #[serde(rename = "native")]
    Native {
        #[serde(flatten)]
        inner: ModFileV2,

        initializer: Option<NativeInitializerCondition>,

        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        load_before: Vec<Dependent<String>>,

        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        load_after: Vec<Dependent<String>>,
    },

    #[serde(rename = "package")]
    Package {
        #[serde(flatten)]
        inner: ModFileV2,

        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        load_before: Vec<Dependent<String>>,

        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        load_after: Vec<Dependent<String>>,
    },

    #[serde(rename = "profile")]
    Profile(ModFileV2),

    #[serde(untagged)]
    Simple(PathBuf),

    #[serde(untagged)]
    Untagged(UntaggedModEntryV2),
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
struct ModFileV2 {
    path: PathBuf,

    #[serde(
        default = "ModFile::enabled_default",
        skip_serializing_if = "ModFile::enabled_is_default"
    )]
    enabled: bool,

    #[serde(
        default = "ModFile::optional_default",
        skip_serializing_if = "ModFile::optional_is_default"
    )]
    optional: bool,
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
struct UntaggedModEntryV2 {
    #[serde(flatten)]
    inner: ModFileV2,

    initializer: Option<NativeInitializerCondition>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    load_before: Vec<Dependent<String>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    load_after: Vec<Dependent<String>>,
}

impl ModEntryV2 {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        path.as_ref().to_owned().into()
    }
}

impl From<(String, ModEntryV2Layout)> for ModEntryV2 {
    fn from((name, layout): (String, ModEntryV2Layout)) -> Self {
        match layout {
            ModEntryV2Layout::Native {
                inner:
                    ModFileV2 {
                        path,
                        enabled,
                        optional,
                    },
                initializer,
                load_before,
                load_after,
            } => Self::Native(Native {
                inner: ModFile {
                    name,
                    path,
                    enabled,
                    optional,
                },
                initializer,
                load_before,
                load_after,
            }),
            ModEntryV2Layout::Package {
                inner:
                    ModFileV2 {
                        path,
                        enabled,
                        optional,
                    },
                load_before,
                load_after,
            } => Self::Package(Package {
                inner: ModFile {
                    name,
                    path,
                    enabled,
                    optional,
                },
                load_before,
                load_after,
            }),
            ModEntryV2Layout::Profile(ModFileV2 {
                path,
                enabled,
                optional,
            }) => Self::Profile(ModFile {
                name,
                path,
                enabled,
                optional,
            }),
            ModEntryV2Layout::Simple(ref path)
            | ModEntryV2Layout::Untagged(UntaggedModEntryV2 {
                inner: ModFileV2 { ref path, .. },
                ..
            }) => {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();

                let untagged = match layout {
                    ModEntryV2Layout::Simple(path) => UntaggedModEntryV2 {
                        inner: ModFileV2 {
                            path,
                            enabled: ModFile::enabled_default(),
                            optional: ModFile::optional_default(),
                        },
                        initializer: None,
                        load_before: vec![],
                        load_after: vec![],
                    },
                    ModEntryV2Layout::Untagged(untagged) => untagged,
                    _ => unreachable!(),
                };

                if file_name.ends_with(".dll") {
                    Self::Native((name, untagged).into())
                } else if file_name.ends_with(".me3")
                    || file_name.ends_with(".me3.toml")
                    || file_name.ends_with(".me3.json")
                {
                    Self::Profile((name, untagged).into())
                } else {
                    Self::Package((name, untagged).into())
                }
            }
        }
    }
}

impl From<ModEntryV2> for (String, ModEntryV2Layout) {
    fn from(mod_entry: ModEntryV2) -> Self {
        let path = match &mod_entry {
            ModEntryV2::Native(native) => native.path.as_path(),
            ModEntryV2::Package(package) => package.path.as_path(),
            ModEntryV2::Profile(profile) => profile.path.as_path(),
        };

        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();

        match mod_entry {
            ModEntryV2::Native(native) => {
                if !file_name.ends_with(".dll") {
                    return (
                        native.inner.name,
                        ModEntryV2Layout::Native {
                            inner: ModFileV2 {
                                path: native.inner.path,
                                enabled: native.inner.enabled,
                                optional: native.inner.optional,
                            },
                            initializer: native.initializer,
                            load_before: native.load_before,
                            load_after: native.load_after,
                        },
                    );
                }

                if native.inner.enabled == ModFile::enabled_default()
                    && native.inner.optional == ModFile::optional_default()
                    && native.initializer.is_none()
                    && native.load_before.is_empty()
                    && native.load_after.is_empty()
                {
                    (
                        native.inner.name,
                        ModEntryV2Layout::Simple(native.inner.path),
                    )
                } else {
                    (
                        native.inner.name,
                        ModEntryV2Layout::Untagged(UntaggedModEntryV2 {
                            inner: ModFileV2 {
                                path: native.inner.path,
                                enabled: native.inner.enabled,
                                optional: native.inner.optional,
                            },
                            initializer: native.initializer,
                            load_before: native.load_before,
                            load_after: native.load_after,
                        }),
                    )
                }
            }
            ModEntryV2::Package(package) => {
                if file_name.ends_with(".dll")
                    || file_name.ends_with(".me3")
                    || file_name.ends_with(".me3.toml")
                    || file_name.ends_with(".me3.json")
                {
                    return (
                        package.inner.name,
                        ModEntryV2Layout::Package {
                            inner: ModFileV2 {
                                path: package.inner.path,
                                enabled: package.inner.enabled,
                                optional: package.inner.optional,
                            },
                            load_before: package.load_before,
                            load_after: package.load_after,
                        },
                    );
                }

                if package.inner.enabled == ModFile::enabled_default()
                    && package.inner.optional == ModFile::optional_default()
                    && package.load_before.is_empty()
                    && package.load_after.is_empty()
                {
                    (
                        package.inner.name,
                        ModEntryV2Layout::Simple(package.inner.path),
                    )
                } else {
                    (
                        package.inner.name,
                        ModEntryV2Layout::Untagged(UntaggedModEntryV2 {
                            inner: ModFileV2 {
                                path: package.inner.path,
                                enabled: package.inner.enabled,
                                optional: package.inner.optional,
                            },
                            initializer: None,
                            load_before: package.load_before,
                            load_after: package.load_after,
                        }),
                    )
                }
            }
            ModEntryV2::Profile(profile) => {
                if !(file_name.ends_with(".me3")
                    || file_name.ends_with(".me3.toml")
                    || file_name.ends_with(".me3.json"))
                {
                    return (
                        profile.name,
                        ModEntryV2Layout::Profile(ModFileV2 {
                            path: profile.path,
                            enabled: profile.enabled,
                            optional: profile.optional,
                        }),
                    );
                }

                if profile.enabled == ModFile::enabled_default()
                    && profile.optional == ModFile::optional_default()
                {
                    (profile.name, ModEntryV2Layout::Simple(profile.path))
                } else {
                    (
                        profile.name,
                        ModEntryV2Layout::Untagged(UntaggedModEntryV2 {
                            inner: ModFileV2 {
                                path: profile.path,
                                enabled: profile.enabled,
                                optional: profile.optional,
                            },
                            initializer: None,
                            load_before: vec![],
                            load_after: vec![],
                        }),
                    )
                }
            }
        }
    }
}

impl From<PathBuf> for ModEntryV2 {
    fn from(path: PathBuf) -> Self {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();

        if file_name.ends_with(".dll") {
            Native::from(path).into()
        } else if file_name.ends_with(".me3")
            || file_name.ends_with(".me3.toml")
            || file_name.ends_with(".me3.json")
        {
            ModFile::from(path).into()
        } else {
            Package::from(path).into()
        }
    }
}

impl From<Native> for ModEntryV2 {
    fn from(native: Native) -> Self {
        Self::Native(native)
    }
}

impl From<Package> for ModEntryV2 {
    fn from(package: Package) -> Self {
        Self::Package(package)
    }
}

impl From<ModFile> for ModEntryV2 {
    fn from(profile: ModFile) -> Self {
        Self::Profile(profile)
    }
}

impl From<(String, UntaggedModEntryV2)> for Native {
    fn from((name, mod_entry): (String, UntaggedModEntryV2)) -> Self {
        let UntaggedModEntryV2 {
            inner:
                ModFileV2 {
                    path,
                    enabled,
                    optional,
                },
            initializer,
            load_before,
            load_after,
        } = mod_entry;

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

impl From<(String, UntaggedModEntryV2)> for Package {
    fn from((name, mod_entry): (String, UntaggedModEntryV2)) -> Self {
        let UntaggedModEntryV2 {
            inner:
                ModFileV2 {
                    path,
                    enabled,
                    optional,
                },
            load_before,
            load_after,
            ..
        } = mod_entry;

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

impl From<(String, UntaggedModEntryV2)> for ModFile {
    fn from((name, mod_entry): (String, UntaggedModEntryV2)) -> Self {
        let UntaggedModEntryV2 {
            inner:
                ModFileV2 {
                    path,
                    enabled,
                    optional,
                },
            ..
        } = mod_entry;

        Self {
            name,
            path,
            enabled,
            optional,
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

        for mod_entry in layout.mods {
            profile.push_mod_entry(mod_entry);
        }

        profile
    }
}

impl From<ModProfileV2> for ModProfileV2Layout {
    fn from(profile: ModProfileV2) -> Self {
        let mut mods = IndexMap::new();

        mods.extend(
            profile
                .natives
                .into_iter()
                .map(|native| ModEntryV2::from(native).into()),
        );

        mods.extend(
            profile
                .packages
                .into_iter()
                .map(|package| ModEntryV2::from(package).into()),
        );

        mods.extend(
            profile
                .profiles
                .into_iter()
                .map(|profile| ModEntryV2::from(profile).into()),
        );

        Self {
            game: GamePropertiesV2 {
                launch: profile.supports,
                savefile: profile.savefile,
                start_online: profile.start_online,
                disable_arxan: profile.disable_arxan,
            },
            mods,
        }
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

impl JsonSchema for ModEntryV2 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Profilemod_entry".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schema_for!(ModEntryV2Layout)
    }
}
