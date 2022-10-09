use std::{collections::HashMap, path::PathBuf, str::FromStr};

use config::{builder::DefaultState, Config, ConfigBuilder, ConfigError, FileFormat, Value};
use serde::{Deserialize, Serialize};

pub type SettingsError = config::ConfigError;

#[derive(Default, Debug)]
pub struct SettingsBuilder {
    config_builder: ConfigBuilder<DefaultState>,
    mod_paths: HashMap<String, PathBuf>,
}

impl SettingsBuilder {
    #[allow(clippy::result_large_err)]
    pub fn add_file<T: AsRef<str>>(
        mut self,
        filename: T,
        required: bool,
    ) -> Result<Self, (Self, SettingsError)> {
        let source = config::File::new(filename.as_ref(), FileFormat::Toml).required(required);
        let config = match Config::builder().add_source(source).build() {
            Ok(config) => config,
            Err(e) => return Err((self, e)),
        };
        let mods: HashMap<String, Value> = config.get("mod").unwrap_or_default();

        for (name, _) in mods {
            self.mod_paths.insert(
                name,
                PathBuf::from_str(filename.as_ref())
                    .unwrap() // path must be valid, IO has already happened above
                    .parent()
                    .unwrap() // not possible for a file to be without a parent
                    .to_owned(),
            );
        }

        Ok(Self {
            config_builder: self.config_builder.add_source(config),
            mod_paths: self.mod_paths,
        })
    }

    #[allow(dead_code)]
    pub fn add_string<T: AsRef<str>>(self, string: T) -> Self {
        Self {
            config_builder: self
                .config_builder
                .add_source(config::File::from_str(string.as_ref(), FileFormat::Toml)),
            mod_paths: self.mod_paths,
        }
    }

    pub fn build(self) -> Result<Settings, ConfigError> {
        Ok(Settings {
            config: self.config_builder.build()?,
            mod_paths: self.mod_paths,
        })
    }
}

fn mod_default_enablement() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct Mod {
    pub file_root: Option<String>,

    #[serde(default = "mod_default_enablement")]
    pub enabled: bool,

    /// A list of Lua scripts that will be executed when this mod loads.
    #[serde(default = "Vec::new")]
    pub scripts: Vec<String>,
}

pub struct Settings {
    config: Config,
    mod_paths: HashMap<String, PathBuf>,
}

impl Settings {
    pub fn mods(&self) -> HashMap<String, Mod> {
        self.config.get("mod").unwrap_or_default()
    }

    pub fn mod_path<S: AsRef<str>>(&self, name: S) -> Option<PathBuf> {
        self.mod_paths.get(name.as_ref()).cloned()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn mod_config_is_additive() {
        let settings = SettingsBuilder::default()
            .add_string(
                r#"
            [mod.a]
            file_root = "abc"
            "#,
            )
            .add_string(
                r#"
            [mod.b]
            file_root = "def"
            "#,
            )
            .build()
            .expect("failed to build test config for mod_config_is_additive");

        let mods = settings.mods();

        assert_eq!(mods.len(), 2);
        assert_eq!(mods["a"].file_root, Some("abc".to_owned()));
        assert_eq!(mods["b"].file_root, Some("def".to_owned()));
    }

    #[test]
    fn mod_config_is_enabled_by_default() {
        let settings = SettingsBuilder::default()
            .add_string(
                r#"
        [mod.a]
        file_root = "abc"
        "#,
            )
            .build()
            .expect("failed to build test config for mod_config_is_enabled_by_default");

        let mods = settings.mods();
        assert!(mods["a"].enabled);
    }

    #[test]
    fn mod_path_is_remembered() -> Result<(), Box<dyn Error>> {
        let root = format!("{}/test-data/", env!("CARGO_MANIFEST_DIR"));
        let settings = SettingsBuilder::default()
            .add_file(format!("{}/mod-path-is-remembered.toml", root), true)
            .expect("failed to load test config file for mod_path_is_remembered")
            .build()
            .expect("failed to build test config for mod_path_is_remembered");

        assert_eq!(PathBuf::from_str(&root).ok(), settings.mod_path("a"));

        Ok(())
    }
}
