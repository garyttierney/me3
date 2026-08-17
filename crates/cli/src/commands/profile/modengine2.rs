use std::{ffi::OsStr, path::PathBuf};

use me3_mod_protocol::{native::Native, package::Package, ModProfile, ModProfileV1};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ModEngine2Mod {
    pub name: String,
    pub path: PathBuf,
    pub enabled: Option<bool>,
}

#[derive(Default, Deserialize)]
pub struct ModEngine2Global {
    #[serde(default)]
    pub external_dlls: Vec<PathBuf>,
}

#[derive(Default, Deserialize)]
pub struct ModEngine2ModLoader {
    #[serde(default)]
    pub mods: Vec<ModEngine2Mod>,
}

#[derive(Default, Deserialize)]
pub struct ModEngine2ScyllaHide {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Default, Deserialize)]
pub struct ModEngine2Extensions {
    #[serde(default)]
    pub mod_loader: ModEngine2ModLoader,

    #[serde(default)]
    pub scylla_hide: ModEngine2ScyllaHide,
}

#[derive(Default, Deserialize)]
pub struct ModEngine2Config {
    #[serde(default)]
    pub modengine: ModEngine2Global,
    #[serde(default)]
    pub extension: ModEngine2Extensions,
}

impl ModEngine2Config {
    pub fn into_mod_profile(self) -> ModProfile {
        let mut profile = ModProfileV1::default();

        // Loose heuristic to make sure SC gets load_early = true
        fn is_seamless_coop(file_name: &OsStr) -> bool {
            file_name == "ersc.dll" || file_name == "nrsc.dll"
        }

        for external_dll in self.modengine.external_dlls {
            let native_name = external_dll.file_name().unwrap();
            let load_early = is_seamless_coop(native_name);
            let mut native = Native::new(external_dll);
            native.load_early = load_early;

            profile.natives.push(native);
        }

        for me2_mod in self.extension.mod_loader.mods {
            let mut package = Package::new(me2_mod.path.clone()).with_id(me2_mod.name);
            package.enabled = me2_mod.enabled.unwrap_or(true);

            profile.packages.push(package);
        }

        if self.extension.scylla_hide.enabled {
            profile.disable_arxan = Some(true);
        }

        profile.into()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use me3_mod_protocol::{dependency::Dependency, package::WithPackageSource};

    use super::*;

    #[test]
    fn default_config() {
        const DEFAULT_CONFIG: &str = r#"
            [modengine]
            debug = false
            external_dlls = []

            [extension.mod_loader]
            enabled = true
            loose_params = false
            mods = [
                { enabled = true, name = "default", path = "mod" }
            ]

            [extension.scylla_hide]
            enabled = false
        "#;

        let me2: ModEngine2Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        let profile = me2.into_mod_profile();
        assert_eq!(profile.disable_arxan(), None);

        let packages = profile.packages();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id(), "default");
        assert_eq!(packages[0].source().as_path(), Path::new("mod"));
        assert!(packages[0].enabled);
    }

    #[test]
    fn example_config() {
        const EXAMPLE_CONFIG: &str = r#"
            [modengine]
            debug = false
            external_dlls = [
                "mods\\SeamlessCoop\\ersc.dll",
                "elden_ring_practice_tool.dll",
            ]

            [extension.mod_loader]
            enabled = true
            loose_params = false
            mods = [
                { enabled = true, name = "convergence", path = "mods\\convergence" },
                { enabled = false, name = "clever", path = "mods\\clever" },
            ]

            [extension.scylla_hide]
            enabled = true
            "#;

        let me2: ModEngine2Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
        let profile = me2.into_mod_profile();
        assert_eq!(profile.disable_arxan(), Some(true));

        let packages = profile.packages();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id(), "convergence");
        assert_eq!(
            packages[0].source().as_path(),
            Path::new("mods\\convergence")
        );
        assert!(packages[0].enabled);
        assert_eq!(packages[1].id(), "clever");
        assert!(!packages[1].enabled);

        let natives = profile.natives();
        assert_eq!(natives.len(), 2);
        assert_eq!(
            natives[0].path.as_path(),
            Path::new("mods\\SeamlessCoop\\ersc.dll")
        );
        assert!(natives[0].load_early);
        assert_eq!(
            natives[1].path.as_path(),
            Path::new("elden_ring_practice_tool.dll")
        );
        assert!(natives.iter().all(|native| native.enabled));
    }
}
