//! Ephemeral contracts for compound harness capability providers.
//!
//! A profile component is capability evidence, not a managed resource. These
//! values deliberately have no serde implementation: companion packages and
//! their health belong to the current observation only, never to inventory,
//! state, or adoption.

use std::{collections::BTreeMap, fmt};

use super::{
    CapabilityId, CapabilityProfileSelection, CapabilityScope, CapabilitySupport, NativeId,
    NativeVersion, ObservationFinding, ObservationTarget, Ownership, ProfileContractError, Scope,
    ScopedCapabilitySets,
};

/// The independent capability supplied by one conditional-profile component.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProfileComponentRole {
    McpCompanion,
    HookCompanion,
}

/// Whether the declared companion package is present in the observed scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProfileComponentPresence {
    Missing,
    Present,
}

/// Runtime activation evidence for one companion.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProfileComponentActivation {
    Inert,
    ConfiguredUnverified,
    Effective,
    TrustRequired,
    Unverified,
}

/// Static and runtime compatibility evidence for one companion.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProfileComponentCompatibility {
    Compatible,
    Partial,
    Incompatible,
    Unverified,
}

/// Normalized evidence for a single user-owned or harness-owned companion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponentObservation {
    pub id: NativeId,
    pub package: NativeId,
    pub role: ProfileComponentRole,
    pub required: bool,
    pub declared_scope: Option<CapabilityScope>,
    pub presence: ProfileComponentPresence,
    pub version: Option<NativeVersion>,
    pub activation: ProfileComponentActivation,
    pub compatibility: ProfileComponentCompatibility,
    pub ownership: Ownership,
}

impl ProfileComponentObservation {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        id: NativeId,
        package: NativeId,
        role: ProfileComponentRole,
        required: bool,
        declared_scope: Option<CapabilityScope>,
        presence: ProfileComponentPresence,
        version: Option<NativeVersion>,
        activation: ProfileComponentActivation,
        compatibility: ProfileComponentCompatibility,
        ownership: Ownership,
    ) -> Self {
        Self {
            id,
            package,
            role,
            required,
            declared_scope,
            presence,
            version,
            activation,
            compatibility,
            ownership,
        }
    }
}

/// Independently addressable companion observations, keyed by normalized id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponentSet(BTreeMap<NativeId, ProfileComponentObservation>);

impl ProfileComponentSet {
    pub fn new(
        components: impl IntoIterator<Item = ProfileComponentObservation>,
    ) -> Result<Self, ConditionalProfileError> {
        let mut collected = BTreeMap::new();
        for component in components {
            let id = component.id.clone();
            if collected.insert(id.clone(), component).is_some() {
                return Err(ConditionalProfileError::DuplicateComponent { id });
            }
        }
        Ok(Self(collected))
    }

    pub fn get(&self, id: &NativeId) -> Option<&ProfileComponentObservation> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&NativeId, &ProfileComponentObservation)> {
        self.0.iter()
    }

    pub fn for_role(
        &self,
        role: ProfileComponentRole,
    ) -> impl Iterator<Item = &ProfileComponentObservation> {
        self.0
            .values()
            .filter(move |component| component.role == role)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// The result of inspecting required companions for one concrete scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalComponentReport {
    components: ProfileComponentSet,
    narrowing: ScopedCapabilitySets,
    findings: Vec<ObservationFinding>,
}

impl ConditionalComponentReport {
    pub fn new(
        components: ProfileComponentSet,
        narrowing: ScopedCapabilitySets,
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> Result<Self, ConditionalProfileError> {
        let mut findings = findings.into_iter().collect::<Vec<_>>();
        validate_finding_context(&findings)?;
        findings.sort();
        Ok(Self {
            components,
            narrowing,
            findings,
        })
    }

    /// Build a report from component observations while retaining duplicate-id
    /// validation at the normalization boundary.
    pub fn from_components(
        components: impl IntoIterator<Item = ProfileComponentObservation>,
        narrowing: ScopedCapabilitySets,
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> Result<Self, ConditionalProfileError> {
        Self::new(ProfileComponentSet::new(components)?, narrowing, findings)
    }

    /// Construct a report while enforcing the exact harness and scope that
    /// requested the observation. This is the adapter-facing boundary for
    /// findings; [`Self::new`] remains useful for already-contextual reports.
    pub fn new_for_target(
        target: &ObservationTarget,
        components: ProfileComponentSet,
        narrowing: ScopedCapabilitySets,
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> Result<Self, ConditionalProfileError> {
        let report = Self::new(components, narrowing, findings)?;
        report.validate_target(target)?;
        Ok(report)
    }

    pub fn validate_target(
        &self,
        target: &ObservationTarget,
    ) -> Result<(), ConditionalProfileError> {
        for finding in &self.findings {
            let actual = finding_target(finding);
            if actual != *target {
                return Err(ConditionalProfileError::FindingContextMismatch {
                    expected: Box::new(target.clone()),
                    actual: Box::new(actual),
                });
            }
        }
        Ok(())
    }

    pub const fn components(&self) -> &ProfileComponentSet {
        &self.components
    }

    pub const fn narrowing(&self) -> &ScopedCapabilitySets {
        &self.narrowing
    }

    pub fn findings(&self) -> &[ObservationFinding] {
        &self.findings
    }
}

/// A fully normalized, ephemeral conditional profile observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalProfileObservation {
    profile: CapabilityProfileSelection,
    components: ProfileComponentSet,
    findings: Vec<ObservationFinding>,
}

impl ConditionalProfileObservation {
    /// Compose runtime evidence only by narrowing the selected compiled
    /// profile. This is the sole path from a report to effective authority.
    pub fn compose(
        compiled: CapabilityProfileSelection,
        report: ConditionalComponentReport,
    ) -> Result<Self, ConditionalProfileError> {
        let profile = compiled
            .narrow(report.narrowing())
            .map_err(ConditionalProfileError::ProfileContract)?;
        Ok(Self {
            profile,
            components: report.components,
            findings: report.findings,
        })
    }

    pub const fn profile(&self) -> &CapabilityProfileSelection {
        &self.profile
    }

    pub const fn components(&self) -> &ProfileComponentSet {
        &self.components
    }

    pub fn findings(&self) -> &[ObservationFinding] {
        &self.findings
    }

    /// Returns mutation support for the concrete scope, failing closed when
    /// the selected profile is unknown or observe-only.
    pub fn mutation_support(&self, scope: &Scope, capability: &CapabilityId) -> CapabilitySupport {
        self.profile
            .mutation_capabilities()
            .and_then(|capabilities| capabilities.for_scope(scope).support(capability))
            .unwrap_or(CapabilitySupport::Unsupported)
    }
}

fn validate_finding_context(
    findings: &[ObservationFinding],
) -> Result<(), ConditionalProfileError> {
    let Some(first) = findings.first() else {
        return Ok(());
    };
    let expected = finding_target(first);
    for finding in &findings[1..] {
        let actual = finding_target(finding);
        if actual != expected {
            return Err(ConditionalProfileError::FindingContextMismatch {
                expected: Box::new(expected.clone()),
                actual: Box::new(actual),
            });
        }
    }
    Ok(())
}

fn finding_target(finding: &ObservationFinding) -> ObservationTarget {
    ObservationTarget::new(
        finding.subject().harness().clone(),
        finding.subject().scope().clone(),
    )
}

/// Errors raised while normalizing or composing conditional profile evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionalProfileError {
    DuplicateComponent {
        id: NativeId,
    },
    FindingContextMismatch {
        expected: Box<ObservationTarget>,
        actual: Box<ObservationTarget>,
    },
    ProfileContract(ProfileContractError),
}

impl fmt::Display for ConditionalProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateComponent { id } => {
                write!(formatter, "duplicate profile component `{id}`")
            }
            Self::FindingContextMismatch { expected, actual } => write!(
                formatter,
                "conditional profile finding target {actual:?} does not match {expected:?}"
            ),
            Self::ProfileContract(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ConditionalProfileError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AbsolutePath, CapabilityProfileId, CapabilitySet, ObservationField, ObservationFields,
        ObservationFindingCode, ObservationSeverity, ObservationSummary,
    };

    fn native(value: &str) -> NativeId {
        NativeId::new(value).unwrap()
    }

    fn version(value: &str) -> NativeVersion {
        NativeVersion::new(value).unwrap()
    }

    fn component(id: &str, role: ProfileComponentRole) -> ProfileComponentObservation {
        ProfileComponentObservation::new(
            native(id),
            native(id),
            role,
            true,
            Some(CapabilityScope::Global),
            ProfileComponentPresence::Present,
            Some(version("1.0.0")),
            ProfileComponentActivation::Effective,
            ProfileComponentCompatibility::Compatible,
            Ownership::Unmanaged,
        )
    }

    fn capabilities(support: CapabilitySupport) -> ScopedCapabilitySets {
        let capability = CapabilityId::new("skill.observe").unwrap();
        ScopedCapabilitySets::new(
            CapabilitySet::new([(capability.clone(), support)]),
            CapabilitySet::new([(capability, support)]),
        )
    }

    fn finding(scope: Scope) -> ObservationFinding {
        ObservationFinding::new(
            ObservationFindingCode::NativeStateIncomplete,
            ObservationSummary::NativeStateIncomplete,
            ObservationSeverity::Warning,
            crate::domain::ObservationSubject::Harness {
                harness: crate::domain::HarnessId::new("pi").unwrap(),
                scope,
            },
            ObservationFields::new([ObservationField::ProfileComponent(native("mcp"))]).unwrap(),
        )
    }

    fn report(
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> ConditionalComponentReport {
        ConditionalComponentReport::from_components(
            [component("mcp", ProfileComponentRole::McpCompanion)],
            capabilities(CapabilitySupport::Supported),
            findings,
        )
        .unwrap()
    }

    #[test]
    fn duplicate_component_ids_are_rejected_while_roles_remain_distinct() {
        let duplicate = ProfileComponentSet::new([
            component("companion", ProfileComponentRole::McpCompanion),
            component("companion", ProfileComponentRole::HookCompanion),
        ])
        .unwrap_err();
        assert!(matches!(
            duplicate,
            ConditionalProfileError::DuplicateComponent { id } if id == native("companion")
        ));

        let set = ProfileComponentSet::new([
            component("mcp", ProfileComponentRole::McpCompanion),
            component("hooks", ProfileComponentRole::HookCompanion),
        ])
        .unwrap();
        assert_eq!(
            set.get(&native("mcp")).unwrap().role,
            ProfileComponentRole::McpCompanion
        );
        assert_eq!(
            set.get(&native("hooks")).unwrap().role,
            ProfileComponentRole::HookCompanion
        );
        assert_eq!(set.for_role(ProfileComponentRole::McpCompanion).count(), 1);
        assert_eq!(set.for_role(ProfileComponentRole::HookCompanion).count(), 1);
    }

    #[test]
    fn finding_scope_mismatches_are_rejected_and_same_context_is_sorted() {
        let global = finding(Scope::Global);
        let project = finding(Scope::Project(AbsolutePath::new("/work/project").unwrap()));
        assert!(matches!(
            ConditionalComponentReport::from_components(
                [component("mcp", ProfileComponentRole::McpCompanion)],
                capabilities(CapabilitySupport::Supported),
                [global.clone(), project.clone()],
            ),
            Err(ConditionalProfileError::FindingContextMismatch { .. })
        ));
        let target =
            ObservationTarget::new(crate::domain::HarnessId::new("pi").unwrap(), Scope::Global);
        assert!(matches!(
            ConditionalComponentReport::new_for_target(
                &target,
                ProfileComponentSet::new([component("mcp", ProfileComponentRole::McpCompanion)])
                    .unwrap(),
                capabilities(CapabilitySupport::Supported),
                [project],
            ),
            Err(ConditionalProfileError::FindingContextMismatch { .. })
        ));

        let report = report([global.clone(), global]);
        assert_eq!(report.findings().len(), 2);
    }

    #[test]
    fn mcp_and_hook_observations_are_independently_queryable() {
        let report = ConditionalComponentReport::from_components(
            [
                component("mcp", ProfileComponentRole::McpCompanion),
                component("hooks", ProfileComponentRole::HookCompanion),
            ],
            capabilities(CapabilitySupport::Supported),
            [],
        )
        .unwrap();
        assert_eq!(report.components().len(), 2);
        assert_eq!(
            report.components().get(&native("mcp")).unwrap().package,
            native("mcp")
        );
        assert_eq!(
            report.components().get(&native("hooks")).unwrap().role,
            ProfileComponentRole::HookCompanion
        );
    }

    #[test]
    fn narrowing_only_composition_rejects_every_capability_widening() {
        let profile = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("pi-tuple").unwrap(),
            capabilities(CapabilitySupport::Supported),
        );
        let capability = CapabilityId::new("skill.observe").unwrap();
        for proposed in [
            CapabilitySupport::Supported,
            CapabilitySupport::Unverified,
            CapabilitySupport::Unsupported,
        ] {
            let report = ConditionalComponentReport::from_components(
                [component("mcp", ProfileComponentRole::McpCompanion)],
                capabilities(proposed),
                [],
            )
            .unwrap();
            let result = ConditionalProfileObservation::compose(profile.clone(), report);
            if proposed == CapabilitySupport::Supported {
                assert!(result.is_ok());
            } else {
                assert!(result.is_ok());
                assert_eq!(
                    result
                        .unwrap()
                        .mutation_support(&Scope::Global, &capability),
                    proposed
                );
            }
        }

        let unverified = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("pi-unverified").unwrap(),
            capabilities(CapabilitySupport::Unverified),
        );
        let report = report([]);
        assert!(ConditionalProfileObservation::compose(unverified, report).is_err());

        let unsupported = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("pi-unsupported").unwrap(),
            capabilities(CapabilitySupport::Unsupported),
        );
        let report = ConditionalComponentReport::from_components(
            [component("mcp", ProfileComponentRole::McpCompanion)],
            capabilities(CapabilitySupport::Supported),
            [],
        )
        .unwrap();
        assert!(ConditionalProfileObservation::compose(unsupported, report).is_err());
    }

    #[test]
    fn unknown_compiled_tuple_is_observe_only_even_with_healthy_components() {
        let profile =
            CapabilityProfileSelection::unknown_version(capabilities(CapabilitySupport::Supported));
        let observation = ConditionalProfileObservation::compose(profile, report([])).unwrap();
        assert!(observation.profile().profile_id().is_none());
        assert!(observation.profile().mutation_capabilities().is_none());
        assert_eq!(
            observation
                .mutation_support(&Scope::Global, &CapabilityId::new("skill.observe").unwrap()),
            CapabilitySupport::Unsupported
        );
        assert_eq!(observation.components().len(), 1);
    }

    #[test]
    fn companion_evidence_retains_native_ownership_without_resource_identity() {
        let observation = ConditionalProfileObservation::compose(
            CapabilityProfileSelection::unknown_version(capabilities(CapabilitySupport::Supported)),
            report([]),
        )
        .unwrap();
        let component = observation.components().get(&native("mcp")).unwrap();
        assert_eq!(component.ownership, Ownership::Unmanaged);
        assert_eq!(component.package, native("mcp"));
    }
}
