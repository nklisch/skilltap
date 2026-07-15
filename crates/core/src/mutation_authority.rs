//! Pure mutation authority for exact compiled harness profiles.
//!
//! This module is deliberately independent of adapters and execution. It is
//! the single ceiling for native and managed mutation: callers provide the
//! exact profile, concrete scope, required capabilities, concrete managed
//! surfaces, and (only for managed unverified work) an adapter contract.

use std::{collections::BTreeSet, fmt};

use crate::domain::{
    CapabilityId, CapabilityProfileSelection, CapabilitySupport, ComponentId, ComponentKind,
    ProfileAuthority, Scope,
};

/// The mutation channel a capability request will use.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MutationChannel {
    /// A documented native CLI or lifecycle command.
    NativeCommand,
    /// A documented skilltap-owned file or complete-tree declaration.
    ManagedProjection,
}

/// The bounded declaration surfaces that may be opted into by an adapter.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManagedSurfaceKind {
    /// An adapter-encoded managed document (for example an MCP/settings file).
    ManagedDocument,
    /// A complete directory tree containing a top-level skill entry point.
    CompleteSkillTree,
}

/// Adapter-authored permission for the exact managed declaration surfaces it
/// has attested as documented, lossless, ownership-safe, and rollback-safe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedDeclarationContract {
    surfaces: BTreeSet<ManagedSurfaceKind>,
}

impl ManagedDeclarationContract {
    pub fn new(
        surfaces: impl IntoIterator<Item = ManagedSurfaceKind>,
    ) -> Result<Self, MutationAuthorityError> {
        let surfaces = surfaces.into_iter().collect::<BTreeSet<_>>();
        if surfaces.is_empty() {
            return Err(MutationAuthorityError::EmptyDeclarationContract);
        }
        Ok(Self { surfaces })
    }

    pub fn covers(&self, surfaces: &BTreeSet<ManagedSurfaceKind>) -> bool {
        !surfaces.is_empty() && surfaces.is_subset(&self.surfaces)
    }

    pub fn surfaces(&self) -> &BTreeSet<ManagedSurfaceKind> {
        &self.surfaces
    }
}

/// One capability required by a concrete operation. Component ids make an
/// unverified result explainable without widening unrelated siblings.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapabilityRequirement {
    pub capability: CapabilityId,
    pub affected_components: BTreeSet<ComponentId>,
}

impl CapabilityRequirement {
    pub fn new(
        capability: CapabilityId,
        affected_components: impl IntoIterator<Item = ComponentId>,
    ) -> Self {
        Self {
            capability,
            affected_components: affected_components.into_iter().collect(),
        }
    }
}

/// The authority granted after every exact requirement has been checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationAuthorization {
    Supported,
    DeclarationManaged {
        unverified: BTreeSet<CapabilityRequirement>,
    },
}

impl MutationAuthorization {
    pub const fn is_declaration_managed(&self) -> bool {
        matches!(self, Self::DeclarationManaged { .. })
    }

    pub fn unverified(&self) -> Option<&BTreeSet<CapabilityRequirement>> {
        match self {
            Self::Supported => None,
            Self::DeclarationManaged { unverified } => Some(unverified),
        }
    }
}

/// All facts required to decide mutation authority. The request is pure and
/// contains no caller acknowledgment; acknowledgment belongs to operations.
pub struct MutationAuthorityRequest<'a> {
    pub profile: &'a CapabilityProfileSelection,
    pub scope: &'a Scope,
    pub channel: MutationChannel,
    pub required: &'a [CapabilityRequirement],
    pub surfaces: &'a BTreeSet<ManagedSurfaceKind>,
    pub declaration: Option<&'a ManagedDeclarationContract>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationAuthorityError {
    ProfileNotMutationAuthorized {
        authority: ProfileAuthority,
    },
    MissingCapability {
        capability: CapabilityId,
    },
    UnsupportedCapability {
        capability: CapabilityId,
    },
    NativeCapabilityUnverified {
        capability: CapabilityId,
    },
    EmptyDeclarationSurface,
    EmptyDeclarationContract,
    DeclarationContractMissing,
    DeclarationSurfaceNotCovered {
        required: BTreeSet<ManagedSurfaceKind>,
    },
}

impl fmt::Display for MutationAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileNotMutationAuthorized { authority } => {
                write!(formatter, "profile authority {authority:?} is observe-only")
            }
            Self::MissingCapability { capability } => {
                write!(
                    formatter,
                    "compiled profile omits required capability `{capability}`"
                )
            }
            Self::UnsupportedCapability { capability } => {
                write!(
                    formatter,
                    "required capability `{capability}` is unsupported"
                )
            }
            Self::NativeCapabilityUnverified { capability } => write!(
                formatter,
                "native capability `{capability}` is unverified and cannot authorize a command"
            ),
            Self::EmptyDeclarationSurface => formatter
                .write_str("managed declaration authority requires a non-empty surface set"),
            Self::EmptyDeclarationContract => {
                formatter.write_str("managed declaration contract must cover at least one surface")
            }
            Self::DeclarationContractMissing => formatter.write_str(
                "unverified managed mutation requires an explicit adapter declaration contract",
            ),
            Self::DeclarationSurfaceNotCovered { required } => write!(
                formatter,
                "adapter declaration contract does not cover required surfaces: {required:?}"
            ),
        }
    }
}

impl std::error::Error for MutationAuthorityError {}

/// Derive the normalized component capability id from the one component-kind
/// registry used by compatibility analysis.
pub fn capability_for_component_kind(kind: &ComponentKind) -> Option<CapabilityId> {
    crate::compatibility::capability_for(kind)
}

/// Decide whether the exact compiled profile authorizes the requested channel.
///
/// Native commands accept only `Supported`. Managed projections may accept a
/// fully covered set of `Unverified` requirements, but only with an explicit
/// declaration contract. A missing capability is not inferred as unverified.
pub fn authorize_mutation(
    request: MutationAuthorityRequest<'_>,
) -> Result<MutationAuthorization, MutationAuthorityError> {
    if request.profile.authority() != ProfileAuthority::VerifiedCompiled {
        return Err(MutationAuthorityError::ProfileNotMutationAuthorized {
            authority: request.profile.authority(),
        });
    }

    if request.channel == MutationChannel::ManagedProjection && request.surfaces.is_empty() {
        return Err(MutationAuthorityError::EmptyDeclarationSurface);
    }

    let mut unverified = BTreeSet::new();
    for requirement in request.required {
        match request
            .profile
            .mutation_support(request.scope, &requirement.capability)
        {
            None => {
                return Err(MutationAuthorityError::MissingCapability {
                    capability: requirement.capability.clone(),
                });
            }
            Some(CapabilitySupport::Unsupported) => {
                return Err(MutationAuthorityError::UnsupportedCapability {
                    capability: requirement.capability.clone(),
                });
            }
            Some(CapabilitySupport::Unverified) => {
                if request.channel == MutationChannel::NativeCommand {
                    return Err(MutationAuthorityError::NativeCapabilityUnverified {
                        capability: requirement.capability.clone(),
                    });
                }
                unverified.insert(requirement.clone());
            }
            Some(CapabilitySupport::Supported) => {}
        }
    }

    if unverified.is_empty() {
        return Ok(MutationAuthorization::Supported);
    }

    let Some(declaration) = request.declaration else {
        return Err(MutationAuthorityError::DeclarationContractMissing);
    };
    if !declaration.covers(request.surfaces) {
        return Err(MutationAuthorityError::DeclarationSurfaceNotCovered {
            required: request.surfaces.clone(),
        });
    }
    Ok(MutationAuthorization::DeclarationManaged { unverified })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AbsolutePath, CapabilityProfileId, CapabilitySet, NativeId, ScopedCapabilitySets,
    };

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::new(value).unwrap()
    }

    fn component(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    fn profile(global: CapabilitySet, project: CapabilitySet) -> CapabilityProfileSelection {
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new("exact-test").unwrap(),
            ScopedCapabilitySets::new(global, project),
        )
    }

    fn request<'a>(
        profile: &'a CapabilityProfileSelection,
        scope: &'a Scope,
        channel: MutationChannel,
        required: &'a [CapabilityRequirement],
        surfaces: &'a BTreeSet<ManagedSurfaceKind>,
        declaration: Option<&'a ManagedDeclarationContract>,
    ) -> MutationAuthorityRequest<'a> {
        MutationAuthorityRequest {
            profile,
            scope,
            channel,
            required,
            surfaces,
            declaration,
        }
    }

    #[test]
    fn supported_exact_profile_authorizes_both_channels() {
        let capability = capability("component.skill");
        let profile = profile(
            CapabilitySet::new([(capability.clone(), CapabilitySupport::Supported)]),
            CapabilitySet::new([(capability.clone(), CapabilitySupport::Supported)]),
        );
        let scope = Scope::Global;
        let required = [CapabilityRequirement::new(
            capability,
            [component("skill:demo")],
        )];
        let surfaces = BTreeSet::from([ManagedSurfaceKind::CompleteSkillTree]);
        let contract =
            ManagedDeclarationContract::new([ManagedSurfaceKind::CompleteSkillTree]).unwrap();

        assert_eq!(
            authorize_mutation(request(
                &profile,
                &scope,
                MutationChannel::NativeCommand,
                &required,
                &surfaces,
                None,
            )),
            Ok(MutationAuthorization::Supported)
        );
        assert_eq!(
            authorize_mutation(request(
                &profile,
                &scope,
                MutationChannel::ManagedProjection,
                &required,
                &surfaces,
                Some(&contract),
            )),
            Ok(MutationAuthorization::Supported)
        );
    }

    #[test]
    fn managed_unverified_requires_contract_and_exact_surface_coverage() {
        let capability = capability("component.mcp");
        let profile = profile(
            CapabilitySet::new([(capability.clone(), CapabilitySupport::Unverified)]),
            CapabilitySet::default(),
        );
        let scope = Scope::Global;
        let required = [CapabilityRequirement::new(
            capability,
            [component("mcp:demo")],
        )];
        let document = BTreeSet::from([ManagedSurfaceKind::ManagedDocument]);
        let tree = BTreeSet::from([ManagedSurfaceKind::CompleteSkillTree]);
        let contract =
            ManagedDeclarationContract::new([ManagedSurfaceKind::ManagedDocument]).unwrap();

        assert_eq!(
            authorize_mutation(request(
                &profile,
                &scope,
                MutationChannel::ManagedProjection,
                &required,
                &document,
                None,
            )),
            Err(MutationAuthorityError::DeclarationContractMissing)
        );
        assert_eq!(
            authorize_mutation(request(
                &profile,
                &scope,
                MutationChannel::ManagedProjection,
                &required,
                &tree,
                Some(&contract),
            )),
            Err(MutationAuthorityError::DeclarationSurfaceNotCovered { required: tree })
        );
        assert!(matches!(
            authorize_mutation(request(
                &profile,
                &scope,
                MutationChannel::ManagedProjection,
                &required,
                &document,
                Some(&contract),
            )),
            Ok(MutationAuthorization::DeclarationManaged { .. })
        ));
    }

    #[test]
    fn native_unverified_and_all_non_exact_profiles_fail_closed() {
        let capability = capability("plugin.install");
        let required = [CapabilityRequirement::new(capability.clone(), [])];
        let surfaces = BTreeSet::from([ManagedSurfaceKind::ManagedDocument]);
        let scope = Scope::Project(AbsolutePath::new("/tmp/project").unwrap());
        let contract =
            ManagedDeclarationContract::new([ManagedSurfaceKind::ManagedDocument]).unwrap();
        for profile in [
            CapabilityProfileSelection::unknown_version(ScopedCapabilitySets::new(
                CapabilitySet::new([(capability.clone(), CapabilitySupport::Supported)]),
                CapabilitySet::default(),
            )),
            CapabilityProfileSelection::verified_observe_only(
                CapabilityProfileId::new("observe-only").unwrap(),
                ScopedCapabilitySets::new(
                    CapabilitySet::new([(capability.clone(), CapabilitySupport::Supported)]),
                    CapabilitySet::default(),
                ),
            ),
        ] {
            assert!(matches!(
                authorize_mutation(request(
                    &profile,
                    &scope,
                    MutationChannel::ManagedProjection,
                    &required,
                    &surfaces,
                    Some(&contract),
                )),
                Err(MutationAuthorityError::ProfileNotMutationAuthorized { .. })
            ));
        }

        let unverified = profile(
            CapabilitySet::new([(capability.clone(), CapabilitySupport::Unverified)]),
            CapabilitySet::default(),
        );
        assert_eq!(
            authorize_mutation(request(
                &unverified,
                &Scope::Global,
                MutationChannel::NativeCommand,
                &required,
                &surfaces,
                None,
            )),
            Err(MutationAuthorityError::NativeCapabilityUnverified { capability })
        );
    }

    #[test]
    fn component_capability_mapping_is_shared_with_compatibility() {
        assert_eq!(
            capability_for_component_kind(&ComponentKind::Skill)
                .unwrap()
                .as_str(),
            "component.skill"
        );
        assert_eq!(
            capability_for_component_kind(&ComponentKind::McpServer)
                .unwrap()
                .as_str(),
            "component.mcp"
        );
        assert!(
            capability_for_component_kind(&ComponentKind::HarnessSpecific(
                NativeId::new("native").unwrap()
            ))
            .is_none()
        );
    }
}
