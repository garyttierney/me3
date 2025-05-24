use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dependency::{Dependency, Dependent};

fn off() -> bool {
    false
}

fn on() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub enum NativeInitializerCondition {
    #[serde(rename = "delay")]
    Delay { ms: usize },
    #[serde(rename = "function")]
    Function(String),
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Native {
    /// Path to the DLL. Can be relative to the mod profile.
    path: PathBuf,

    /// If this native fails to load and this vakye is false, treat it as a critical error.
    #[serde(default = "off")]
    optional: bool,

    /// Should this native be loaded?
    #[serde(default = "on")]
    enabled: bool,

    #[serde(default)]
    load_before: Vec<Dependent<String>>,

    #[serde(default)]
    load_after: Vec<Dependent<String>>,

    /// An optional symbol to be called after this native succesfully loads.
    initializer: Option<NativeInitializerCondition>,

    /// An optional symbol to be called when this native successfully is queued for unload.
    finalizer: Option<String>,
}

impl Dependency for Native {
    type UniqueId = String;

    fn id(&self) -> Self::UniqueId {
        self.path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .expect("native had no file name")
    }

    fn loads_after(&self) -> &[Dependent<Self::UniqueId>] {
        &self.load_after
    }

    fn loads_before(&self) -> &[Dependent<Self::UniqueId>] {
        &self.load_before
    }
}
