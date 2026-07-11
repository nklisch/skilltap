//! Safe, documented-table updates that preserve unknown native TOML fields.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeConfigError {
    Parse,
    NotTable,
    Encode,
}

impl fmt::Display for NativeConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Parse => "native TOML configuration could not be parsed",
            Self::NotTable => "native TOML configuration root is not a table",
            Self::Encode => "native TOML configuration could not be encoded",
        })
    }
}

impl std::error::Error for NativeConfigError {}

/// Apply only the caller's documented top-level changes. Unknown keys and
/// tables remain in the parsed document and are never discarded.
pub fn preserve_unknown_toml(
    original: &str,
    updates: &toml::Table,
) -> Result<String, NativeConfigError> {
    let mut table: toml::Table = toml::from_str(original).map_err(|_| NativeConfigError::Parse)?;
    for (key, update) in updates {
        if let Some(existing) = table.get_mut(key) {
            merge_value(existing, update);
        } else {
            table.insert(key.clone(), update.clone());
        }
    }
    toml::to_string_pretty(&table).map_err(|_| NativeConfigError::Encode)
}

fn merge_value(existing: &mut toml::Value, update: &toml::Value) {
    if let (Some(existing), Some(update)) = (existing.as_table_mut(), update.as_table()) {
        for (key, value) in update {
            if let Some(current) = existing.get_mut(key) {
                merge_value(current, value);
            } else {
                existing.insert(key.clone(), value.clone());
            }
        }
    } else {
        *existing = update.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_updates_keep_unknown_native_tables() {
        let original = "[plugins]\nold = 1\nunknown = true\n";
        let mut updates = toml::Table::new();
        updates.insert("plugins".to_owned(), toml::toml! { new = 2 }.into());
        let encoded = preserve_unknown_toml(original, &updates).unwrap();
        let parsed: toml::Table = toml::from_str(&encoded).unwrap();
        assert_eq!(parsed["plugins"]["unknown"].as_bool(), Some(true));
        assert_eq!(parsed["plugins"]["new"].as_integer(), Some(2));
    }
}
