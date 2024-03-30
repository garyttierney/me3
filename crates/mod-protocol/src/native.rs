use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn off() -> bool {
    false
}

fn on() -> bool {
    true
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct Native {
    /// Path to the DLL. Can be relative to the mod profile.
    path: PathBuf,

    /// If this native fails to load and this vakye is false, treat it as a critical error.
    #[serde(default = "off")]
    optional: bool,

    /// Should this native be loaded?
    #[serde(default = "on")]
    enabled: bool,

    /// An optional symbol to be called after this native succesfully loads.
    initializer: Option<String>,

    /// An optional symbol to be called when this native successfully is queued for unload.
    finalizer: Option<String>,
}
