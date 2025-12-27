use std::{collections::HashMap, path::PathBuf, process::Command};

use me3_mod_protocol::Game;
use serde::{de::value::MapDeserializer, Deserialize, Serialize};
use serde_json::Value;

pub trait EnvVars {
    const PREFIX: &'static str;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryVars {
    pub enabled: bool,

    pub log_file_path: PathBuf,

    pub monitor_pipe_path: PathBuf,

    pub trace_id: Option<String>,
}

impl EnvVars for TelemetryVars {
    const PREFIX: &'static str = "ME3_TELEMETRY_";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LauncherVars {
    /// Path to the game EXE that should be launched.
    pub exe: PathBuf,

    /// Path to the me3 that should be attached to the game.
    pub host_dll: PathBuf,

    pub host_config_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameVars {
    /// The game launched via `me3 launch`.
    pub launched: Game,
}

impl EnvVars for GameVars {
    const PREFIX: &'static str = "ME3_GAME_";
}

impl EnvVars for LauncherVars {
    const PREFIX: &'static str = "ME3_LAUNCHER_";
}

pub trait CommandExt {
    fn with_env_vars(&mut self, vars: impl EnvVars + Serialize) -> &mut Self;
}

impl CommandExt for Command {
    fn with_env_vars(&mut self, vars: impl EnvVars + Serialize) -> &mut Self {
        serialize_into_command(vars, self);
        self
    }
}

pub fn deserialize_from_env<'de, T: Deserialize<'de> + EnvVars>() -> Result<T, serde_json::Error> {
    deserialize(
        std::env::vars()
            .filter(|(k, _)| k.starts_with(T::PREFIX))
            .map(|(k, v)| (k.trim_start_matches(T::PREFIX).to_ascii_lowercase(), v)),
    )
}
pub fn deserialize<'de, T: Deserialize<'de>>(
    input: impl IntoIterator<Item = (String, String)>,
) -> Result<T, serde_json::Error> {
    T::deserialize(MapDeserializer::new(
        input
            .into_iter()
            .map(|(k, v)| (k, serde_json::from_str::<Value>(v.as_str()).unwrap())),
    ))
}

pub fn serialize<T: Serialize>(input: T) -> Result<HashMap<String, String>, serde_json::Error> {
    let value = serde_json::to_value(input)?;
    let map: HashMap<String, Value> = serde_json::from_value(value)?;

    let serialized_map: HashMap<String, String> = map
        .into_iter()
        .flat_map(|(k, v)| serde_json::to_string(&v).map(|serialized| (k, serialized)))
        .collect();

    Ok(serialized_map)
}

pub fn serialize_into_command<T: Serialize + EnvVars>(data: T, command: &mut Command) {
    let map = serialize(data).expect("failed to serialize env vars");
    for (k, v) in map {
        command.env(format!("{}{}", T::PREFIX, k.to_ascii_uppercase()), v);
    }
}
