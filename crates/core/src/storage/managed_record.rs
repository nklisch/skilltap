use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use super::SchemaError;
use crate::domain::{Fingerprint, RelativeArtifactPath, ResourceId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    MaterializedPlugin,
    DirectSkill,
    Backup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ManagedArtifactRecordWire")]
pub struct ManagedArtifactRecord {
    owner: ResourceId,
    role: ArtifactRole,
    path: RelativeArtifactPath,
    fingerprint: Option<Fingerprint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManagedArtifactRecordWire {
    owner: ResourceId,
    role: ArtifactRole,
    path: RelativeArtifactPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<Fingerprint>,
}

impl ManagedArtifactRecord {
    pub fn new(
        owner: ResourceId,
        role: ArtifactRole,
        path: RelativeArtifactPath,
        fingerprint: Option<Fingerprint>,
    ) -> Result<Self, SchemaError> {
        let record = Self {
            owner,
            role,
            path,
            fingerprint,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn for_artifact(
        owner: ResourceId,
        role: ArtifactRole,
        fingerprint: Fingerprint,
    ) -> Result<Self, SchemaError> {
        if role == ArtifactRole::Backup {
            return Err(invalid_record(&owner));
        }
        let path = artifact_path(&owner, role, &fingerprint)?;
        Self::new(owner, role, path, Some(fingerprint))
    }

    pub fn for_backup(owner: ResourceId, process: u32, sequence: u64) -> Result<Self, SchemaError> {
        if process == 0 {
            return Err(invalid_record(&owner));
        }
        let path =
            RelativeArtifactPath::new(format!("backup-{}-{process}-{sequence}", owner_key(&owner)))
                .map_err(|_| invalid_record(&owner))?;
        Self::new(owner, ArtifactRole::Backup, path, None)
    }

    pub(crate) fn validate_for_owner(&self, owner: &ResourceId) -> Result<(), SchemaError> {
        if &self.owner != owner {
            return Err(SchemaError::ManagedOwnerMismatch {
                resource: owner.clone(),
                owner: self.owner.clone(),
            });
        }
        self.validate()
    }

    fn validate(&self) -> Result<(), SchemaError> {
        let valid = match self.role {
            ArtifactRole::MaterializedPlugin | ArtifactRole::DirectSkill => {
                self.fingerprint.as_ref().is_some_and(|fingerprint| {
                    artifact_path(&self.owner, self.role, fingerprint)
                        .is_ok_and(|path| path == self.path)
                })
            }
            ArtifactRole::Backup => {
                self.fingerprint.is_none() && valid_backup_path(&self.owner, &self.path)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(invalid_record(&self.owner))
        }
    }

    pub fn owner(&self) -> &ResourceId {
        &self.owner
    }

    pub const fn role(&self) -> ArtifactRole {
        self.role
    }

    pub fn path(&self) -> &RelativeArtifactPath {
        &self.path
    }

    pub const fn fingerprint(&self) -> Option<&Fingerprint> {
        self.fingerprint.as_ref()
    }
}

impl From<ManagedArtifactRecord> for ManagedArtifactRecordWire {
    fn from(value: ManagedArtifactRecord) -> Self {
        Self {
            owner: value.owner,
            role: value.role,
            path: value.path,
            fingerprint: value.fingerprint,
        }
    }
}

impl TryFrom<ManagedArtifactRecordWire> for ManagedArtifactRecord {
    type Error = SchemaError;

    fn try_from(value: ManagedArtifactRecordWire) -> Result<Self, Self::Error> {
        Self::new(value.owner, value.role, value.path, value.fingerprint)
    }
}

impl<'de> Deserialize<'de> for ManagedArtifactRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ManagedArtifactRecordWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

fn artifact_path(
    owner: &ResourceId,
    role: ArtifactRole,
    fingerprint: &Fingerprint,
) -> Result<RelativeArtifactPath, SchemaError> {
    RelativeArtifactPath::new(format!(
        "artifact-{}-{}-{}-{}",
        role_component(role),
        owner_key(owner),
        fingerprint.algorithm(),
        fingerprint.digest()
    ))
    .map_err(|_| invalid_record(owner))
}

fn valid_backup_path(owner: &ResourceId, path: &RelativeArtifactPath) -> bool {
    let prefix = format!("backup-{}-", owner_key(owner));
    let Some((process, sequence)) = path.as_str().strip_prefix(&prefix).and_then(|suffix| {
        let (process, sequence) = suffix.split_once('-')?;
        (!sequence.contains('-')).then_some((process, sequence))
    }) else {
        return false;
    };
    let Ok(process_value) = process.parse::<u32>() else {
        return false;
    };
    let Ok(sequence_value) = sequence.parse::<u64>() else {
        return false;
    };
    process_value != 0
        && process_value.to_string() == process
        && sequence_value.to_string() == sequence
}

const fn role_component(role: ArtifactRole) -> &'static str {
    match role {
        ArtifactRole::MaterializedPlugin => "materialized-plugin",
        ArtifactRole::DirectSkill => "direct-skill",
        ArtifactRole::Backup => "backup",
    }
}

fn owner_key(owner: &ResourceId) -> String {
    format!("{:x}", Sha256::digest(owner.as_str().as_bytes()))
}

fn invalid_record(owner: &ResourceId) -> SchemaError {
    SchemaError::InvalidManagedArtifactRecord {
        owner: owner.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FingerprintAlgorithm;

    fn fingerprint() -> Fingerprint {
        Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64)).unwrap()
    }

    #[test]
    fn canonical_records_derive_bounded_owner_bound_paths_and_round_trip() {
        let owner = ResourceId::new("a".repeat(256)).unwrap();
        let artifact = ManagedArtifactRecord::for_artifact(
            owner.clone(),
            ArtifactRole::DirectSkill,
            fingerprint(),
        )
        .unwrap();
        assert!(artifact.path().as_str().len() <= 255);
        assert_eq!(artifact.owner(), &owner);
        assert_eq!(
            serde_json::from_value::<ManagedArtifactRecord>(
                serde_json::to_value(&artifact).unwrap()
            )
            .unwrap(),
            artifact
        );

        let backup = ManagedArtifactRecord::for_backup(owner, 42, 7).unwrap();
        assert_eq!(backup.role(), ArtifactRole::Backup);
        assert_eq!(backup.fingerprint(), None);
        assert_eq!(
            serde_json::from_value::<ManagedArtifactRecord>(serde_json::to_value(&backup).unwrap())
                .unwrap(),
            backup
        );
    }

    #[test]
    fn arbitrary_construction_and_deserialization_reject_noncanonical_records() {
        let owner = ResourceId::new("skill:invalid").unwrap();
        for (role, path, fingerprint) in [
            (ArtifactRole::DirectSkill, "arbitrary", Some(fingerprint())),
            (ArtifactRole::DirectSkill, "arbitrary", None),
            (ArtifactRole::Backup, "backup-arbitrary-1-2", None),
            (
                ArtifactRole::Backup,
                "backup-arbitrary-1-2",
                Some(fingerprint()),
            ),
        ] {
            assert!(matches!(
                ManagedArtifactRecord::new(
                    owner.clone(),
                    role,
                    RelativeArtifactPath::new(path).unwrap(),
                    fingerprint
                ),
                Err(SchemaError::InvalidManagedArtifactRecord { .. })
            ));
        }
        assert!(
            ManagedArtifactRecord::for_artifact(owner.clone(), ArtifactRole::Backup, fingerprint())
                .is_err()
        );
        assert!(ManagedArtifactRecord::for_backup(owner.clone(), 0, 1).is_err());

        let invalid = serde_json::json!({
            "owner": owner,
            "role": "direct_skill",
            "path": "arbitrary",
            "fingerprint": fingerprint(),
        });
        assert!(serde_json::from_value::<ManagedArtifactRecord>(invalid).is_err());

        let backup = ManagedArtifactRecord::for_backup(owner, 42, 7).unwrap();
        let mut invalid_backup = serde_json::to_value(backup).unwrap();
        invalid_backup["fingerprint"] = serde_json::to_value(fingerprint()).unwrap();
        assert!(serde_json::from_value::<ManagedArtifactRecord>(invalid_backup).is_err());
    }
}
