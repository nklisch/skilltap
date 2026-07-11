use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor},
};

pub(super) enum CodecFailure {
    Malformed,
    Invalid,
    UnsupportedSchema { version: u32 },
    Encode,
}

pub(super) trait DocumentCodec<T> {
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure>;
    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure>;
}

#[derive(Clone, Copy)]
pub(super) struct TomlCodec {
    expected_schema: u32,
}

impl TomlCodec {
    pub(super) const fn new(expected_schema: u32) -> Self {
        Self { expected_schema }
    }
}

impl<T> DocumentCodec<T> for TomlCodec
where
    T: DeserializeOwned + Serialize,
{
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure> {
        let contents = std::str::from_utf8(contents).map_err(|_| CodecFailure::Malformed)?;
        let table = toml::from_str::<toml::Table>(contents).map_err(|_| CodecFailure::Malformed)?;
        validate_toml_schema(&table, self.expected_schema)?;
        toml::from_str(contents).map_err(|_| CodecFailure::Invalid)
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure> {
        toml::to_string_pretty(value)
            .map(String::into_bytes)
            .map_err(|_| CodecFailure::Encode)
    }
}

#[derive(Clone, Copy)]
pub(super) struct JsonCodec {
    expected_schema: u32,
}

impl JsonCodec {
    pub(super) const fn new(expected_schema: u32) -> Self {
        Self { expected_schema }
    }
}

impl<T> DocumentCodec<T> for JsonCodec
where
    T: DeserializeOwned + Serialize,
{
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure> {
        serde_json::from_slice::<serde_json::Value>(contents)
            .map_err(|_| CodecFailure::Malformed)?;
        let probe = serde_json::from_slice::<JsonSchemaProbe>(contents)
            .map_err(|_| CodecFailure::Invalid)?;
        if let Some(version) = probe.schema
            && version != self.expected_schema
        {
            return Err(CodecFailure::UnsupportedSchema { version });
        }
        serde_json::from_slice(contents).map_err(|_| CodecFailure::Invalid)
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure> {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| CodecFailure::Encode)?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

fn validate_toml_schema(table: &toml::Table, expected_schema: u32) -> Result<(), CodecFailure> {
    if let Some(version) = table.get("schema").and_then(toml::Value::as_integer)
        && version >= 0
        && version as u64 <= u32::MAX as u64
        && version as u32 != expected_schema
    {
        return Err(CodecFailure::UnsupportedSchema {
            version: version as u32,
        });
    }
    Ok(())
}

struct JsonSchemaProbe {
    schema: Option<u32>,
}

impl<'de> Deserialize<'de> for JsonSchemaProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProbeVisitor;

        impl<'de> Visitor<'de> for ProbeVisitor {
            type Value = JsonSchemaProbe;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object with unique top-level fields")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut seen = BTreeSet::new();
                let mut schema = None;
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom("duplicate top-level field"));
                    }
                    if key == "schema" {
                        schema = Some(map.next_value::<u32>()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(JsonSchemaProbe { schema })
            }
        }

        deserializer.deserialize_map(ProbeVisitor)
    }
}
