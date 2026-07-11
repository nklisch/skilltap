//! Compatibility and transfer-fidelity contracts.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    HarnessId, ValidationError, resource::ComponentId, validate_identifier, validate_text,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityClass {
    Compatible,
    TargetSpecific,
    Unknown,
    Incompatible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferFidelity {
    Faithful,
    Materializable,
    Partial,
    Blocked,
}

impl TransferFidelity {
    pub fn is_faithful(self) -> bool {
        self == Self::Faithful
    }
}

macro_rules! validated_compatibility_text {
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

validated_compatibility_text!(
    EvidenceCode,
    "compatibility evidence code",
    128,
    validate_identifier
);
validated_compatibility_text!(
    EvidenceDetail,
    "compatibility evidence detail",
    1024,
    validate_text
);
validated_compatibility_text!(
    ConsequenceCode,
    "compatibility consequence code",
    128,
    validate_identifier
);
validated_compatibility_text!(
    ConsequenceSummary,
    "compatibility consequence summary",
    1024,
    validate_text
);

/// Target-specific evidence supporting a compatibility classification.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityEvidence {
    pub code: EvidenceCode,
    pub target: HarnessId,
    pub affected_components: BTreeSet<ComponentId>,
    pub detail: EvidenceDetail,
}

impl CompatibilityEvidence {
    pub fn new(
        code: EvidenceCode,
        target: HarnessId,
        affected_components: impl IntoIterator<Item = ComponentId>,
        detail: EvidenceDetail,
    ) -> Self {
        Self {
            code,
            target,
            affected_components: affected_components.into_iter().collect(),
            detail,
        }
    }
}

/// A concrete user-visible effect of accepting a non-faithful transfer.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialConsequence {
    pub code: ConsequenceCode,
    pub affected_components: BTreeSet<ComponentId>,
    pub summary: ConsequenceSummary,
}

impl MaterialConsequence {
    pub fn new(
        code: ConsequenceCode,
        affected_components: impl IntoIterator<Item = ComponentId>,
        summary: ConsequenceSummary,
    ) -> Self {
        Self {
            code,
            affected_components: affected_components.into_iter().collect(),
            summary,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompatibilityError {
    MissingEvidence {
        fidelity: TransferFidelity,
    },
    MissingConsequence {
        fidelity: TransferFidelity,
    },
    FaithfulHasMaterialConsequences,
    EvidenceTargetMismatch {
        target: HarnessId,
        evidence_target: HarnessId,
        evidence_code: EvidenceCode,
    },
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEvidence { fidelity } => {
                write!(
                    formatter,
                    "{fidelity:?} transfer requires compatibility evidence"
                )
            }
            Self::MissingConsequence { fidelity } => {
                write!(
                    formatter,
                    "{fidelity:?} transfer requires a material consequence"
                )
            }
            Self::FaithfulHasMaterialConsequences => {
                write!(
                    formatter,
                    "faithful transfer cannot have material consequences"
                )
            }
            Self::EvidenceTargetMismatch {
                target,
                evidence_target,
                evidence_code,
            } => write!(
                formatter,
                "compatibility evidence `{evidence_code}` targets `{evidence_target}`, expected `{target}`"
            ),
        }
    }
}

impl std::error::Error for CompatibilityError {}

/// Independent behavioral-compatibility and transfer-fidelity conclusions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityResult {
    target: HarnessId,
    compatibility: CompatibilityClass,
    fidelity: TransferFidelity,
    evidence: BTreeSet<CompatibilityEvidence>,
    consequences: BTreeSet<MaterialConsequence>,
}

impl CompatibilityResult {
    pub fn new(
        target: HarnessId,
        compatibility: CompatibilityClass,
        fidelity: TransferFidelity,
        evidence: impl IntoIterator<Item = CompatibilityEvidence>,
        consequences: impl IntoIterator<Item = MaterialConsequence>,
    ) -> Result<Self, CompatibilityError> {
        let evidence = evidence.into_iter().collect::<BTreeSet<_>>();
        let consequences = consequences.into_iter().collect::<BTreeSet<_>>();

        if let Some(mismatched) = evidence.iter().find(|item| item.target != target) {
            return Err(CompatibilityError::EvidenceTargetMismatch {
                target,
                evidence_target: mismatched.target.clone(),
                evidence_code: mismatched.code.clone(),
            });
        }

        if fidelity.is_faithful() {
            if !consequences.is_empty() {
                return Err(CompatibilityError::FaithfulHasMaterialConsequences);
            }
        } else {
            if evidence.is_empty() {
                return Err(CompatibilityError::MissingEvidence { fidelity });
            }
            if consequences.is_empty() {
                return Err(CompatibilityError::MissingConsequence { fidelity });
            }
        }

        Ok(Self {
            target,
            compatibility,
            fidelity,
            evidence,
            consequences,
        })
    }

    pub fn target(&self) -> &HarnessId {
        &self.target
    }

    pub fn compatibility(&self) -> CompatibilityClass {
        self.compatibility
    }

    pub fn fidelity(&self) -> TransferFidelity {
        self.fidelity
    }

    pub fn evidence(&self) -> &BTreeSet<CompatibilityEvidence> {
        &self.evidence
    }

    pub fn consequences(&self) -> &BTreeSet<MaterialConsequence> {
        &self.consequences
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityResultWire {
    target: HarnessId,
    compatibility: CompatibilityClass,
    fidelity: TransferFidelity,
    evidence: BTreeSet<CompatibilityEvidence>,
    consequences: BTreeSet<MaterialConsequence>,
}

impl From<CompatibilityResult> for CompatibilityResultWire {
    fn from(result: CompatibilityResult) -> Self {
        Self {
            target: result.target,
            compatibility: result.compatibility,
            fidelity: result.fidelity,
            evidence: result.evidence,
            consequences: result.consequences,
        }
    }
}

impl TryFrom<CompatibilityResultWire> for CompatibilityResult {
    type Error = CompatibilityError;

    fn try_from(wire: CompatibilityResultWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.target,
            wire.compatibility,
            wire.fidelity,
            wire.evidence,
            wire.consequences,
        )
    }
}

impl Serialize for CompatibilityResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CompatibilityResultWire {
            target: self.target.clone(),
            compatibility: self.compatibility,
            fidelity: self.fidelity,
            evidence: self.evidence.clone(),
            consequences: self.consequences.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CompatibilityResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CompatibilityResultWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    fn evidence() -> CompatibilityEvidence {
        CompatibilityEvidence::new(
            EvidenceCode::new("target.component.unsupported").unwrap(),
            HarnessId::new("codex").unwrap(),
            [component("lsp:typescript")],
            EvidenceDetail::new("Claude LSP components have no Codex equivalent").unwrap(),
        )
    }

    fn consequence() -> MaterialConsequence {
        MaterialConsequence::new(
            ConsequenceCode::new("component.omitted").unwrap(),
            [component("lsp:typescript")],
            ConsequenceSummary::new("The TypeScript language server will not be installed")
                .unwrap(),
        )
    }

    #[test]
    fn compatibility_and_fidelity_are_independent_axes() {
        let native_target_specific = CompatibilityResult::new(
            HarnessId::new("codex").unwrap(),
            CompatibilityClass::TargetSpecific,
            TransferFidelity::Faithful,
            [evidence()],
            [],
        )
        .unwrap();

        assert_eq!(
            native_target_specific.target(),
            &HarnessId::new("codex").unwrap()
        );
        assert_eq!(
            native_target_specific.compatibility(),
            CompatibilityClass::TargetSpecific
        );
        assert_eq!(
            native_target_specific.fidelity(),
            TransferFidelity::Faithful
        );
    }

    #[test]
    fn every_non_faithful_result_requires_evidence_and_a_consequence() {
        for fidelity in [
            TransferFidelity::Materializable,
            TransferFidelity::Partial,
            TransferFidelity::Blocked,
        ] {
            assert_eq!(
                CompatibilityResult::new(
                    HarnessId::new("codex").unwrap(),
                    CompatibilityClass::Unknown,
                    fidelity,
                    [],
                    [consequence()],
                )
                .unwrap_err(),
                CompatibilityError::MissingEvidence { fidelity }
            );
            assert_eq!(
                CompatibilityResult::new(
                    HarnessId::new("codex").unwrap(),
                    CompatibilityClass::Unknown,
                    fidelity,
                    [evidence()],
                    [],
                )
                .unwrap_err(),
                CompatibilityError::MissingConsequence { fidelity }
            );
        }

        assert_eq!(
            CompatibilityResult::new(
                HarnessId::new("codex").unwrap(),
                CompatibilityClass::Compatible,
                TransferFidelity::Faithful,
                [],
                [consequence()],
            )
            .unwrap_err(),
            CompatibilityError::FaithfulHasMaterialConsequences
        );
    }

    #[test]
    fn constructor_and_deserialization_enforce_the_same_invariants() {
        let constructor = CompatibilityResult::new(
            HarnessId::new("codex").unwrap(),
            CompatibilityClass::Incompatible,
            TransferFidelity::Blocked,
            [],
            [],
        )
        .unwrap_err();
        let persisted = serde_json::from_str::<CompatibilityResult>(
            r#"{"target":"codex","compatibility":"incompatible","fidelity":"blocked","evidence":[],"consequences":[]}"#,
        )
        .unwrap_err();

        assert_eq!(
            constructor,
            CompatibilityError::MissingEvidence {
                fidelity: TransferFidelity::Blocked
            }
        );
        assert!(persisted.to_string().contains(&constructor.to_string()));
    }

    #[test]
    fn evidence_must_target_the_results_declared_harness() {
        let constructor = CompatibilityResult::new(
            HarnessId::new("claude").unwrap(),
            CompatibilityClass::TargetSpecific,
            TransferFidelity::Partial,
            [evidence()],
            [consequence()],
        )
        .unwrap_err();
        assert_eq!(
            constructor,
            CompatibilityError::EvidenceTargetMismatch {
                target: HarnessId::new("claude").unwrap(),
                evidence_target: HarnessId::new("codex").unwrap(),
                evidence_code: EvidenceCode::new("target.component.unsupported").unwrap(),
            }
        );

        let persisted = serde_json::from_str::<CompatibilityResult>(
            r#"{"target":"claude","compatibility":"target_specific","fidelity":"partial","evidence":[{"code":"target.component.unsupported","target":"codex","affected_components":["lsp:typescript"],"detail":"Claude LSP components have no Codex equivalent"}],"consequences":[{"code":"component.omitted","affected_components":["lsp:typescript"],"summary":"The TypeScript language server will not be installed"}]}"#,
        )
        .unwrap_err();
        assert!(persisted.to_string().contains(&constructor.to_string()));
    }

    #[test]
    fn partial_result_serialization_is_deterministic_and_round_trips() {
        let result = CompatibilityResult::new(
            HarnessId::new("codex").unwrap(),
            CompatibilityClass::TargetSpecific,
            TransferFidelity::Partial,
            [evidence()],
            [consequence()],
        )
        .unwrap();

        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(
            serde_json::from_str::<CompatibilityResult>(&json).unwrap(),
            result
        );
        assert_eq!(
            json,
            r#"{"target":"codex","compatibility":"target_specific","fidelity":"partial","evidence":[{"code":"target.component.unsupported","target":"codex","affected_components":["lsp:typescript"],"detail":"Claude LSP components have no Codex equivalent"}],"consequences":[{"code":"component.omitted","affected_components":["lsp:typescript"],"summary":"The TypeScript language server will not be installed"}]}"#
        );
    }

    #[test]
    fn compatibility_wires_reject_unknown_fields_and_invalid_text() {
        let unknown = r#"{"target":"codex","compatibility":"compatible","fidelity":"faithful","evidence":[],"consequences":[],"assumed_safe":true}"#;
        assert!(serde_json::from_str::<CompatibilityResult>(unknown).is_err());

        let invalid_evidence =
            r#"{"code":"Bad Code","target":"codex","affected_components":[],"detail":"detail"}"#;
        assert!(serde_json::from_str::<CompatibilityEvidence>(invalid_evidence).is_err());
    }
}
