use super::{validate_identifier, validate_text};
use crate::domain::validated_newtype::validated_string_newtype;

validated_string_newtype!(HarnessId, "harness id", 64, validate_identifier, try_from);
validated_string_newtype!(
    ResourceId,
    "resource id",
    256,
    validate_identifier,
    try_from
);
validated_string_newtype!(
    OperationId,
    "operation id",
    128,
    validate_identifier,
    try_from
);
validated_string_newtype!(NativeId, "native id", 512, validate_text, try_from);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ValidationError;

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
