use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{ValidationError, validate_identifier, validate_text};

macro_rules! validated_text_type {
    ($name:ident, $kind:literal, $max:expr, $validator:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                $validator(&value, $kind, $max)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_text_type!(HarnessId, "harness id", 64, validate_identifier);
validated_text_type!(ResourceId, "resource id", 256, validate_identifier);
validated_text_type!(OperationId, "operation id", 128, validate_identifier);
validated_text_type!(NativeId, "native id", 512, validate_text);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_ids_enforce_the_same_boundary_during_deserialization() {
        let expected = HarnessId::new(" Codex").unwrap_err();
        let persisted = serde_json::from_str::<HarnessId>(r#"" Codex""#).unwrap_err();

        assert_eq!(
            expected,
            ValidationError::SurroundingWhitespace { kind: "harness id" }
        );
        assert!(persisted.to_string().contains(&expected.to_string()));
        assert!(matches!(
            ResourceId::new("Plugin/Name"),
            Err(ValidationError::InvalidFormat { .. })
        ));
        assert!(matches!(
            OperationId::new("sync\nresource"),
            Err(ValidationError::ControlCharacter { .. })
        ));
    }

    #[test]
    fn native_ids_remain_opaque_but_bounded() {
        let native = NativeId::new("Plugin Name@marketplace/v1").unwrap();
        assert_eq!(native.as_str(), "Plugin Name@marketplace/v1");
        assert!(matches!(
            NativeId::new("x".repeat(513)),
            Err(ValidationError::TooLong { .. })
        ));
    }

    #[test]
    fn identity_values_round_trip_as_strings() {
        let id = ResourceId::new("plugin:claude.tools-v1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""plugin:claude.tools-v1""#);
        assert_eq!(serde_json::from_str::<ResourceId>(&json).unwrap(), id);
    }
}
