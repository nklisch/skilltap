//! Validating adapter for Codex's repository marketplace document.

use std::path::{Component, Path, PathBuf};

use skilltap_core::{
    domain::{AbsolutePath, NativeId},
    runtime::{JsonLimits, StrictJson, StrictJsonDecoder},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedCodexCatalogError {
    InvalidJson,
    InvalidShape,
    InvalidPluginName,
    PluginMissing,
    DuplicatePlugin,
    UnsupportedSource,
    SourceEscapesCatalog,
}

impl std::fmt::Display for ManagedCodexCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidJson => "the selected Codex marketplace is not valid bounded JSON",
            Self::InvalidShape => "the selected Codex marketplace has an invalid document shape",
            Self::InvalidPluginName => "the selected Codex marketplace has an invalid plugin name",
            Self::PluginMissing => "the selected plugin is not present in the marketplace",
            Self::DuplicatePlugin => "the selected plugin occurs more than once in the marketplace",
            Self::UnsupportedSource => "the selected plugin does not use a managed local source",
            Self::SourceEscapesCatalog => "the selected plugin source escapes the marketplace root",
        })
    }
}

impl std::error::Error for ManagedCodexCatalogError {}

/// A bounded Codex marketplace document that validates selected sources while
/// preserving unknown fields in its original JSON value.
#[derive(Clone, Debug)]
pub struct ManagedCodexCatalog {
    value: serde_json::Value,
}

impl ManagedCodexCatalog {
    pub fn parse(bytes: &[u8], limits: JsonLimits) -> Result<Self, ManagedCodexCatalogError> {
        let decoded = StrictJson
            .decode(bytes, limits)
            .map_err(|_| ManagedCodexCatalogError::InvalidJson)?;
        let value = decoded.value().clone();
        let object = value
            .as_object()
            .ok_or(ManagedCodexCatalogError::InvalidShape)?;
        object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .and_then(|name| NativeId::new(name).ok())
            .ok_or(ManagedCodexCatalogError::InvalidShape)?;
        let plugins = object
            .get("plugins")
            .and_then(serde_json::Value::as_array)
            .ok_or(ManagedCodexCatalogError::InvalidShape)?;
        for plugin in plugins {
            let name = plugin
                .as_object()
                .and_then(|plugin| plugin.get("name"))
                .and_then(serde_json::Value::as_str)
                .ok_or(ManagedCodexCatalogError::InvalidShape)?;
            NativeId::new(name).map_err(|_| ManagedCodexCatalogError::InvalidPluginName)?;
        }
        Ok(Self { value })
    }

    pub fn plugin_source(
        &self,
        plugin: &NativeId,
        catalog_root: &AbsolutePath,
    ) -> Result<AbsolutePath, ManagedCodexCatalogError> {
        let entry = self.unique_plugin(plugin)?;
        let source = entry
            .get("source")
            .ok_or(ManagedCodexCatalogError::UnsupportedSource)?;
        let raw_path = match source {
            serde_json::Value::String(path) => path.as_str(),
            serde_json::Value::Object(source)
                if source.get("source").and_then(serde_json::Value::as_str) == Some("local") =>
            {
                source
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .ok_or(ManagedCodexCatalogError::UnsupportedSource)?
            }
            _ => return Err(ManagedCodexCatalogError::UnsupportedSource),
        };
        let relative = contained_relative(raw_path)?;
        AbsolutePath::new(
            Path::new(catalog_root.as_str())
                .join(relative.as_path())
                .to_string_lossy()
                .into_owned(),
        )
        .map_err(|_| ManagedCodexCatalogError::SourceEscapesCatalog)
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, ManagedCodexCatalogError> {
        let mut bytes = serde_json::to_vec_pretty(&self.value)
            .map_err(|_| ManagedCodexCatalogError::InvalidShape)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    fn unique_plugin(
        &self,
        plugin: &NativeId,
    ) -> Result<&serde_json::Map<String, serde_json::Value>, ManagedCodexCatalogError> {
        let mut matches = self
            .value
            .get("plugins")
            .and_then(serde_json::Value::as_array)
            .ok_or(ManagedCodexCatalogError::InvalidShape)?
            .iter()
            .filter_map(serde_json::Value::as_object)
            .filter(|entry| {
                entry.get("name").and_then(serde_json::Value::as_str) == Some(plugin.as_str())
            });
        let entry = matches
            .next()
            .ok_or(ManagedCodexCatalogError::PluginMissing)?;
        if matches.next().is_some() {
            return Err(ManagedCodexCatalogError::DuplicatePlugin);
        }
        Ok(entry)
    }

}

fn contained_relative(value: &str) -> Result<PathBuf, ManagedCodexCatalogError> {
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ManagedCodexCatalogError::SourceEscapesCatalog);
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => result.push(value),
            _ => return Err(ManagedCodexCatalogError::SourceEscapesCatalog),
        }
    }
    if result.as_os_str().is_empty() {
        return Err(ManagedCodexCatalogError::SourceEscapesCatalog);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16_384, 16).unwrap()
    }

    #[test]
    fn selected_local_source_is_contained_and_unknown_fields_survive() {
        let catalog = ManagedCodexCatalog::parse(
            br#"{"name":"team","future":{"enabled":true},"plugins":[{"name":"demo","source":{"source":"local","path":"./plugins/demo"},"future":"keep"}]}"#,
            limits(),
        )
        .unwrap();
        assert_eq!(
            catalog
                .plugin_source(
                    &NativeId::new("demo").unwrap(),
                    &AbsolutePath::new("/tmp/catalog").unwrap(),
                )
                .unwrap()
                .as_str(),
            "/tmp/catalog/plugins/demo"
        );
        let bytes = catalog.into_bytes().unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["future"]["enabled"], true);
        assert_eq!(value["plugins"][0]["future"], "keep");
        assert_eq!(value["plugins"][0]["source"]["path"], "./plugins/demo");
    }

    #[test]
    fn path_escape_duplicates_and_missing_plugins_fail_closed() {
        let escaping = ManagedCodexCatalog::parse(
            br#"{"name":"team","plugins":[{"name":"demo","source":"../demo"}]}"#,
            limits(),
        )
        .unwrap();
        assert_eq!(
            escaping.plugin_source(
                &NativeId::new("demo").unwrap(),
                &AbsolutePath::new("/tmp/catalog").unwrap(),
            ),
            Err(ManagedCodexCatalogError::SourceEscapesCatalog)
        );
        let duplicate = ManagedCodexCatalog::parse(
            br#"{"name":"team","plugins":[{"name":"demo","source":"./a"},{"name":"demo","source":"./b"}]}"#,
            limits(),
        )
        .unwrap();
        assert_eq!(
            duplicate.plugin_source(
                &NativeId::new("demo").unwrap(),
                &AbsolutePath::new("/tmp/catalog").unwrap(),
            ),
            Err(ManagedCodexCatalogError::DuplicatePlugin)
        );
    }
}
