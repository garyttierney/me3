use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use indexmap::{map::Entry, IndexMap};
use schemars::JsonSchema;
use serde::{de, Deserialize, Serialize};

/// The value of a game property.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
#[schemars(untagged)]
#[schemars(extend("anyOf" = [
    {"type": "string"},
    {"type": "boolean"},
    {"type": "number"},
    {
        "type": "object",
        "additionalProperties": {
            "$ref": "#/$defs/Property"
        }
    }
]))]
pub enum Property {
    String(String),
    Bool(bool),
    Uint(u64),
    Int(i64),
    Float(f64),
}

impl Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(x) => Display::fmt(&x, f),
            Self::Bool(x) => Display::fmt(&x, f),
            Self::Int(x) => Display::fmt(&x, f),
            Self::Uint(x) => Display::fmt(&x, f),
            Self::Float(x) => Display::fmt(&x, f),
        }
    }
}

/// Arbitrary debug game property overrides.
///
/// Do not use this unless you know what that means and what the properties you are setting do!
#[derive(Default, Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct DebugProperties {
    #[schemars(with = "HashMap<String, Property>")]
    pub(crate) props: IndexMap<String, Property>,
}

impl<'de> Deserialize<'de> for DebugProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(DebugPropertiesVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum PropertyRepr {
    Nested(IndexMap<String, PropertyRepr>),
    String(String),
    Bool(bool),
    // Uint case is only for lossless integer roundtripping when larger than i64::MAX. I can't
    // think of a property that actually needs this, but just in case.
    Uint(u64),
    Int(i64),
    Float(f64),
}

struct DebugPropertiesVisitor;
impl<'de> de::Visitor<'de> for DebugPropertiesVisitor {
    type Value = DebugProperties;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("key-value map of strings, numbers or booleans")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        fn flatten_prop_value<E: de::Error>(
            map: &mut IndexMap<String, Property>,
            key: String,
            val: PropertyRepr,
        ) -> Result<(), E> {
            let prop = match val {
                PropertyRepr::Nested(table) => {
                    for (nested_key, nested_val) in table {
                        flatten_prop_value(map, format!("{key}.{nested_key}"), nested_val)?
                    }
                    return Ok(());
                }
                PropertyRepr::String(x) => Property::String(x),
                PropertyRepr::Bool(x) => Property::Bool(x),
                PropertyRepr::Uint(x) => Property::Uint(x),
                PropertyRepr::Int(x) => Property::Int(x),
                PropertyRepr::Float(x) => Property::Float(x),
            };
            match map.entry(key) {
                Entry::Occupied(entry) => Err(E::custom(format_args!(
                    "duplicate property: `{}`",
                    entry.key()
                ))),
                Entry::Vacant(entry) => {
                    entry.insert(prop);
                    Ok(())
                }
            }
        }

        let mut props = IndexMap::new();
        while let Some((key, val)) = map.next_entry::<String, PropertyRepr>()? {
            flatten_prop_value(&mut props, key, val)?;
        }
        Ok(DebugProperties { props })
    }
}

#[cfg(test)]
mod tests {
    use super::DebugProperties;

    #[test]
    fn debug_property_reprs_match() {
        let repr1 = r#"
        Debug.Game.Foo = true
        Debug.Game.Bar = 0
        Debug.System.Baz = 3.52
        Game.Foo = "string"
        "#;
        let repr2 = r#"
        "Debug.Game.Foo" = true
        Debug."Game.Bar" = 0
        "Debug.System".Baz = 3.52
        "Game.Foo" = "string"
        "#;

        let props1: DebugProperties = toml::from_str(repr1).unwrap();
        let props2: DebugProperties = toml::from_str(repr2).unwrap();

        assert_eq!(format!("{props1:#?}"), format!("{props2:#?}"));
    }

    #[test]
    fn debug_property_duplicates_error() {
        let has_duplicate = r#"
        Debug.Game.Foo = true
        Debug.Game.Bar = 0
        "Debug.Game".Bar = 1
        "#;

        assert!(toml::from_str::<DebugProperties>(has_duplicate).is_err());
    }
}
