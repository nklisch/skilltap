//! Strict, bounded JSON decoding for native harness output.

use std::{collections::BTreeSet, fmt};

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use super::{DecodedJson, JsonLimits, ObservationRuntimeError, StrictJsonDecoder};

const DUPLICATE_KEY_MARKER: &str = "skilltap duplicate JSON key";
const DEPTH_MARKER: &str = "skilltap JSON depth exceeded";

#[derive(Clone, Copy, Debug, Default)]
pub struct StrictJson;

impl StrictJsonDecoder for StrictJson {
    fn decode(
        &self,
        input: &[u8],
        limits: JsonLimits,
    ) -> Result<DecodedJson, ObservationRuntimeError> {
        let input_bytes = u64::try_from(input.len())
            .map_err(|_| ObservationRuntimeError::JsonByteLimitExceeded)?;
        if input_bytes > limits.bytes() {
            return Err(ObservationRuntimeError::JsonByteLimitExceeded);
        }
        let source =
            std::str::from_utf8(input).map_err(|_| ObservationRuntimeError::JsonInvalidUtf8)?;
        let mut deserializer = serde_json::Deserializer::from_str(source);
        let value = StrictValueSeed {
            depth: 0,
            maximum_depth: limits.depth(),
        }
        .deserialize(&mut deserializer)
        .map_err(classify_decode_error)?;
        deserializer
            .end()
            .map_err(|_| ObservationRuntimeError::JsonTrailingContent)?;
        Ok(DecodedJson::new(value))
    }
}

fn classify_decode_error(error: serde_json::Error) -> ObservationRuntimeError {
    let rendered = error.to_string();
    if rendered.starts_with(DUPLICATE_KEY_MARKER) {
        ObservationRuntimeError::JsonDuplicateKey
    } else if rendered.starts_with(DEPTH_MARKER) || rendered.starts_with("recursion limit exceeded")
    {
        ObservationRuntimeError::JsonDepthLimitExceeded
    } else {
        ObservationRuntimeError::JsonInvalidSyntax
    }
}

#[derive(Clone, Copy)]
struct StrictValueSeed {
    depth: u32,
    maximum_depth: u32,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = serde_json::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor(self))
    }
}

struct StrictValueVisitor(StrictValueSeed);

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one bounded JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.0.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child = self.0.child::<A::Error>()?;
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element_seed(child)? {
            values.push(value);
        }
        Ok(serde_json::Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child = self.0.child::<A::Error>()?;
        let mut keys = BTreeSet::new();
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(A::Error::custom(DUPLICATE_KEY_MARKER));
            }
            values.insert(key, map.next_value_seed(child)?);
        }
        Ok(serde_json::Value::Object(values))
    }
}

impl StrictValueSeed {
    fn child<E>(self) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        if self.depth >= self.maximum_depth {
            return Err(E::custom(DEPTH_MARKER));
        }
        Ok(Self {
            depth: self.depth + 1,
            maximum_depth: self.maximum_depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MAX_JSON_BYTES, MAX_JSON_DEPTH};

    fn limits(bytes: u64, depth: u32) -> JsonLimits {
        JsonLimits::new(bytes, depth).unwrap()
    }

    fn nested_arrays(depth: u32) -> Vec<u8> {
        let mut source = "[".repeat(depth as usize);
        source.push('0');
        source.push_str(&"]".repeat(depth as usize));
        source.into_bytes()
    }

    #[test]
    fn byte_cap_precedes_utf8_and_accepts_boundary_minus_and_at_only() {
        for input in [b"0      ".as_slice(), b"0       ".as_slice()] {
            assert!(StrictJson.decode(input, limits(8, 1)).is_ok());
        }
        assert_eq!(
            StrictJson.decode(b"0        ", limits(8, 1)),
            Err(ObservationRuntimeError::JsonByteLimitExceeded)
        );
        assert_eq!(
            StrictJson.decode(&[0xff; 9], limits(8, 1)),
            Err(ObservationRuntimeError::JsonByteLimitExceeded)
        );
        assert_eq!(
            StrictJson.decode(&[0xff], limits(8, 1)),
            Err(ObservationRuntimeError::JsonInvalidUtf8)
        );
        assert!(JsonLimits::new(0, 1).is_err());
        assert!(JsonLimits::new(MAX_JSON_BYTES + 1, 1).is_err());
    }

    #[test]
    fn nesting_depth_is_container_count_at_minus_at_and_plus_one() {
        for depth in [2, 3] {
            assert!(
                StrictJson
                    .decode(&nested_arrays(depth), limits(1024, 3))
                    .is_ok()
            );
        }
        assert_eq!(
            StrictJson.decode(&nested_arrays(4), limits(1024, 3)),
            Err(ObservationRuntimeError::JsonDepthLimitExceeded)
        );

        for depth in [MAX_JSON_DEPTH - 1, MAX_JSON_DEPTH] {
            assert!(
                StrictJson
                    .decode(&nested_arrays(depth), limits(1024, MAX_JSON_DEPTH))
                    .is_ok(),
                "depth {depth} must fit the contract and parser limits"
            );
        }
        assert_eq!(
            StrictJson.decode(
                &nested_arrays(MAX_JSON_DEPTH + 1),
                limits(1024, MAX_JSON_DEPTH),
            ),
            Err(ObservationRuntimeError::JsonDepthLimitExceeded)
        );
        assert!(JsonLimits::new(1, 0).is_err());
        assert!(JsonLimits::new(1, MAX_JSON_DEPTH + 1).is_err());
    }

    #[test]
    fn duplicate_keys_are_rejected_at_every_nested_shape() {
        for source in [
            br#"{"a":1,"a":2}"#.as_slice(),
            br#"{"outer":{"a":1,"a":2}}"#.as_slice(),
            br#"[{"a":1,"a":2}]"#.as_slice(),
        ] {
            assert_eq!(
                StrictJson.decode(source, limits(1024, 8)),
                Err(ObservationRuntimeError::JsonDuplicateKey)
            );
        }
    }

    #[test]
    fn exactly_one_document_allows_only_trailing_whitespace() {
        let decoded = StrictJson
            .decode(b"{\"ok\":true} \n\t", limits(1024, 8))
            .unwrap();
        assert_eq!(decoded.value()["ok"], true);
        for source in [
            br#"{"ok":true} {}"#.as_slice(),
            br#"{"ok":true} trailing"#.as_slice(),
        ] {
            assert_eq!(
                StrictJson.decode(source, limits(1024, 8)),
                Err(ObservationRuntimeError::JsonTrailingContent)
            );
        }
        assert_eq!(
            StrictJson.decode(br#"{"#, limits(1024, 8)),
            Err(ObservationRuntimeError::JsonInvalidSyntax)
        );
    }

    #[test]
    fn all_failures_are_fixed_and_secret_safe() {
        const SECRET: &str = "secret-parser-canary";
        let failures = [
            StrictJson.decode(
                format!(r#"{{"{SECRET}":1,"{SECRET}":2}}"#).as_bytes(),
                limits(1024, 8),
            ),
            StrictJson.decode(
                format!(r#"{{"x":"{SECRET}"}} trailing"#).as_bytes(),
                limits(1024, 8),
            ),
            StrictJson.decode(format!(r#"{{"x":"{SECRET}""#).as_bytes(), limits(1024, 8)),
        ];
        for failure in failures {
            let error = failure.unwrap_err();
            assert!(!error.to_string().contains(SECRET));
            assert!(!format!("{error:?}").contains(SECRET));
            assert!(!serde_json::to_string(&error).unwrap().contains(SECRET));
        }
    }
}
