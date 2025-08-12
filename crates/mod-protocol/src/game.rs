use std::{fmt::Display, path::Path};

use schemars::{json_schema, JsonSchema};
use serde::{de::Error, Deserialize, Serialize};
use serde_json::json;
use strum::VariantArray;
use strum_macros::VariantArray;

/// Chronologically sorted list of games supported by me3.
///
/// Feature gates can use [`Ord`] comparisons between game type constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, VariantArray)]
pub enum Game {
    DarkSouls3,
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
            DarkSouls3 => "darksouls3",
            Sekiro => "sekiro",
            EldenRing => "eldenring",
            ArmoredCore6 => "armoredcore6",
            Nightreign => "nightreign",
        }
    }

    /// The full, official name of a game.
    pub const fn title(self) -> &'static str {
        use Game::*;
        match self {
            DarkSouls3 => "Dark Souls III",
            Sekiro => "Sekiro: Shadows Die Twice",
            EldenRing => "Elden Ring",
            ArmoredCore6 => "Armored Core VI: Fires of Rubicon",
            Nightreign => "Elden Ring Nightreign",
        }
    }

    /// All names and aliases of a game as lowercase strings, including the primary name.
    pub fn possible_names(self) -> &'static [&'static str] {
        use Game::*;
        match self {
            DarkSouls3 => &[const { DarkSouls3.name() }, "ds3"],
            Sekiro => &[const { Sekiro.name() }, "sdt"],
            EldenRing => &[const { EldenRing.name() }, "er", "elden-ring"],
            ArmoredCore6 => &[const { ArmoredCore6.name() }, "ac6"],
            Nightreign => &[const { Nightreign.name() }, "nr", "nightrein"],
        }
    }

    /// All aliases of a game as lowercase strings, excluding the primary name.
    pub fn aliases(self) -> &'static [&'static str] {
        &self.possible_names()[1..]
    }

    fn to_json(self) -> serde_json::Value {
        json!({
            "description": format!("{} (Steam App ID: {})", self.title(), self.app_id()),
            "enum": self.possible_names(),
            "title": self.title()
        })
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Game {
    type Error = InvalidGame;

    fn try_from(mut name: String) -> Result<Self, Self::Error> {
        name.make_ascii_lowercase();

        Self::VARIANTS
            .iter()
            .copied()
            .find(|game| game.possible_names().contains(&&*name))
            .ok_or(InvalidGame(name))
    }
}

impl Game {
    /// Returns the Steam App ID of a game.
    pub fn app_id(self) -> u32 {
        use Game::*;
        match self {
            DarkSouls3 => 374320,
            Sekiro => 814380,
            EldenRing => 1245620,
            ArmoredCore6 => 1888160,
            Nightreign => 2622380,
        }
    }

    /// Returns a game from its Steam App ID.
    pub fn from_app_id(id: u32) -> Option<Self> {
        Self::VARIANTS
            .iter()
            .copied()
            .find(|game| game.app_id() == id)
    }

    /// Returns the path to a game's executable in its Steam installation folder.
    pub fn executable(self) -> &'static Path {
        use Game::*;
        Path::new(match self {
            DarkSouls3 => "Game/DarkSoulsIII.exe",
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
        Game::try_from(name).map_err(D::Error::custom)
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
        json_schema!({
            "description": "List of games supported by me3",
            "type": "string",
            "oneOf": Self::VARIANTS.iter().copied().map(Self::to_json).collect::<Vec<_>>()
        })
    }
}
