use serde::{Deserialize, Serialize};

use super::{Scope, ValidationError, validate_identifier, validate_text};
use crate::domain::validated_newtype::validated_string_newtype;

validated_string_newtype!(HarnessId, "harness id", 64, validate_identifier, try_from);
validated_string_newtype!(
    ResourceId,
    "resource id",
    256,
    validate_resource_identifier,
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

fn validate_resource_identifier(
    value: &str,
    kind: &'static str,
    max: usize,
) -> Result<(), ValidationError> {
    validate_text(value, kind, max)?;

    if let Some((local, qualifier)) = value.split_once('@') {
        if qualifier.contains('@') {
            return Err(qualified_resource_id_error(kind));
        }
        validate_identifier(local, kind, max).map_err(|_| qualified_resource_id_error(kind))?;
        validate_identifier(qualifier, kind, max).map_err(|_| qualified_resource_id_error(kind))?;
    } else {
        validate_identifier(value, kind, max)?;
    }
    Ok(())
}

const fn qualified_resource_id_error(kind: &'static str) -> ValidationError {
    ValidationError::InvalidFormat {
        kind,
        expected: "an identifier or two identifiers separated by one `@`",
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceKey {
    id: ResourceId,
    scope: Scope,
}

impl ResourceKey {
    pub const fn new(id: ResourceId, scope: Scope) -> Self {
        Self { id, scope }
    }

    pub const fn id(&self) -> &ResourceId {
        &self.id
    }

    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        const DOMAIN: &[u8] = b"skilltap.resource-key\0\x01";

        let id = self.id.as_str().as_bytes();
        let path_length = match &self.scope {
            Scope::Global => 0,
            Scope::Project(path) => size_of::<u32>() + path.as_str().len(),
        };
        let mut encoded =
            Vec::with_capacity(DOMAIN.len() + size_of::<u32>() + id.len() + 1 + path_length);
        encoded.extend_from_slice(DOMAIN);
        append_length_prefixed(&mut encoded, id);
        match &self.scope {
            Scope::Global => encoded.push(0),
            Scope::Project(path) => {
                encoded.push(1);
                append_length_prefixed(&mut encoded, path.as_str().as_bytes());
            }
        }
        encoded
    }
}

fn append_length_prefixed(encoded: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("validated resource-key fields fit in u32");
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(value);
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::*;
    use crate::domain::{AbsolutePath, ValidationError};

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

    #[test]
    fn resource_ids_accept_one_well_formed_qualification_only() {
        for valid in [
            "plugin:formatter",
            "plugin:formatter@anthropic-official",
            "formatter@marketplace:local",
        ] {
            assert_eq!(ResourceId::new(valid).unwrap().as_str(), valid);
        }

        for invalid in [
            "@marketplace",
            "plugin@",
            "plugin@@marketplace",
            "plugin@Marketplace",
            "plugin@market/place",
        ] {
            assert!(
                matches!(
                    ResourceId::new(invalid),
                    Err(ValidationError::InvalidFormat { .. })
                ),
                "{invalid}"
            );
            assert!(
                serde_json::from_str::<ResourceId>(&format!(r#""{invalid}""#)).is_err(),
                "persisted {invalid}"
            );
        }

        assert!(HarnessId::new("claude@managed").is_err());
        assert!(OperationId::new("sync@managed").is_err());
    }

    #[test]
    fn resource_keys_are_strict_nested_values() {
        let key = ResourceKey::new(
            ResourceId::new("plugin:formatter@official").unwrap(),
            Scope::Project(AbsolutePath::new("/work/project").unwrap()),
        );
        let json = serde_json::to_string(&key).unwrap();

        assert_eq!(
            json,
            r#"{"id":"plugin:formatter@official","scope":{"kind":"project","path":"/work/project"}}"#
        );
        assert_eq!(serde_json::from_str::<ResourceKey>(&json).unwrap(), key);
        assert!(
            serde_json::from_str::<ResourceKey>(
                r#"{"id":"plugin:formatter","scope":{"kind":"global"},"extra":true}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<ResourceKey>(
                r#"{"id":"plugin:formatter","scope":{"kind":"global","path":"/work"}}"#
            )
            .is_err()
        );
    }

    #[test]
    fn equal_ids_in_distinct_scopes_remain_distinct_everywhere() {
        let id = ResourceId::new("skill:shared").unwrap();
        let global = ResourceKey::new(id.clone(), Scope::Global);
        let project_a = ResourceKey::new(
            id.clone(),
            Scope::Project(AbsolutePath::new("/work/a").unwrap()),
        );
        let project_b = ResourceKey::new(id, Scope::Project(AbsolutePath::new("/work/b").unwrap()));

        assert_eq!(
            BTreeSet::from([global.clone(), project_a.clone(), project_b.clone()]).len(),
            3
        );
        assert_eq!(
            HashSet::from([global.clone(), project_a.clone(), project_b.clone()]).len(),
            3
        );
        assert_ne!(global.canonical_bytes(), project_a.canonical_bytes());
        assert_ne!(project_a.canonical_bytes(), project_b.canonical_bytes());
        assert_eq!(
            serde_json::from_str::<ResourceKey>(&serde_json::to_string(&project_a).unwrap())
                .unwrap(),
            project_a
        );
    }

    #[test]
    fn canonical_resource_key_encoding_is_versioned_and_unambiguous() {
        let global = ResourceKey::new(ResourceId::new("a:b").unwrap(), Scope::Global);
        let project = ResourceKey::new(
            ResourceId::new("a").unwrap(),
            Scope::Project(AbsolutePath::new("/b").unwrap()),
        );

        assert_eq!(
            global.canonical_bytes(),
            [
                b"skilltap.resource-key\0\x01".as_slice(),
                &[0, 0, 0, 3],
                b"a:b",
                &[0],
            ]
            .concat()
        );
        assert_eq!(
            project.canonical_bytes(),
            [
                b"skilltap.resource-key\0\x01".as_slice(),
                &[0, 0, 0, 1],
                b"a",
                &[1],
                &[0, 0, 0, 2],
                b"/b",
            ]
            .concat()
        );
        assert_ne!(global.canonical_bytes(), project.canonical_bytes());
    }
}
