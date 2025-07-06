use std::{fmt::Display, path::Path, str::FromStr};

use schemars::{json_schema, JsonSchema};
use serde::{de::Error, Deserialize, Serialize};

/// Chronologically sorted list of games supported by me3.
///
/// Feature gates can use [`Ord`] comparisons between game type constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Game {
    Sekiro,
    EldenRing,
    ArmoredCore6,
    Nightreign,
}

impl Game {
    /// The primary name of a game as a lowercase string.
    pub const fn name(self) -> &'static str {
        use Game::*;
        match self {
            Sekiro => "sekiro",
            EldenRing => "eldenring",
            ArmoredCore6 => "armoredcore6",
            Nightreign => "nightreign",
        }
    }

    /// All names and aliases of a game as lowecase strings, including the primary name.
    pub fn possible_names(self) -> &'static [&'static str] {
        use Game::*;
        match self {
            Sekiro => &[const { Sekiro.name() }, "sdt"],
            EldenRing => &[const { EldenRing.name() }, "er", "elden-ring"],
            ArmoredCore6 => &[const { ArmoredCore6.name() }, "ac6"],
            Nightreign => &[const { Nightreign.name() }, "nr", "nightrein"],
        }
    }

    /// All aliases of a game as lowercase strings, excluding the primary name.
    pub fn aliases(&self) -> &'static [&'static str] {
        &self.possible_names()[1..]
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Game {
    type Err = InvalidGame;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        use Game::*;
        match name.to_ascii_lowercase() {
            name if Sekiro.possible_names().contains(&&*name) => Ok(Sekiro),
            name if EldenRing.possible_names().contains(&&*name) => Ok(EldenRing),
            name if ArmoredCore6.possible_names().contains(&&*name) => Ok(ArmoredCore6),
            name if Nightreign.possible_names().contains(&&*name) => Ok(Nightreign),
            name => Err(InvalidGame(name)),
        }
    }
}

impl Game {
    /// Returns the Steam App ID of a game.
    pub fn app_id(self) -> u32 {
        use Game::*;
        match self {
            Sekiro => 814380,
            EldenRing => 1245620,
            ArmoredCore6 => 1888160,
            Nightreign => 2622380,
        }
    }

    /// Returns a game from its Steam App ID.
    pub fn from_app_id(id: u32) -> Option<Self> {
        use Game::*;
        match id {
            814380 => Some(Sekiro),
            1245620 => Some(EldenRing),
            1888160 => Some(ArmoredCore6),
            2622380 => Some(Nightreign),
            _ => None,
        }
    }

    /// Returns the path to a game's executable in its Steam installation folder.
    pub fn executable(self) -> &'static Path {
        use Game::*;
        Path::new(match self {
            Sekiro => "sekiro.exe",
            EldenRing => "Game/eldenring.exe",
            ArmoredCore6 => "Game/armoredcore6.exe",
            Nightreign => "Game/nightreign.exe",
        })
    }
}

#[derive(Debug)]
pub struct InvalidGame(String);

impl Display for InvalidGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a supported game", self.0)
    }
}

impl std::error::Error for InvalidGame {}

impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Game {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Game::from_str(&name).map_err(D::Error::custom)
    }
}

impl JsonSchema for Game {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Game".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        "me3_mod_protocol::game::Game".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use Game::*;
        json_schema!({
            "description": "List of games supported by me3",
            "type": "string",
            "enum": [
                Sekiro.name(),
                EldenRing.name(),
                ArmoredCore6.name(),
                Nightreign.name()
            ]
        })
    }
}
