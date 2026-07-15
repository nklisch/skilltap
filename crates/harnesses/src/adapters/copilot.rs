use std::{collections::BTreeMap, ffi::OsString, sync::LazyLock};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileId, CapabilityScope, CapabilitySet,
        CapabilitySupport, Fingerprint, HarnessId, NativeId, NativeVersion, ObservationFields,
        ObservationFinding, ObservationFindingCode, ObservationSeverity, ObservationSubject,
        ObservationSummary, Scope, ScopedCapabilitySets,
    },
    instructions::fingerprint_contents,
    mutation_authority::{ManagedDeclarationContract, ManagedSurfaceKind},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        ObservationRuntimeError, PlatformPaths, StrictJsonDecoder, SystemExternalTreeObserver,
    },
};

use crate::{
    effective_state::{
        EffectiveMcpStatus, EffectiveProbeError, EffectiveProbeSpec, EffectiveServerHealth,
        EffectiveStateProbePort, ProjectTrustHealth, ReloadSemantics,
    },
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::copilot_managed::CopilotManagedProjection;

const VERIFIED_VERSION: &str = "1.0.70";
const PROFILE_ID: &str = "copilot-1-0-70";
const COPILOT_HOME: &str = ".copilot";

pub struct CopilotAdapter;
pub struct CopilotSkillProjection;
pub struct CopilotEffectiveStateProbe;

static ADAPTER: CopilotAdapter = CopilotAdapter;
static SKILLS: CopilotSkillProjection = CopilotSkillProjection;
static PROBE: CopilotEffectiveStateProbe = CopilotEffectiveStateProbe;
static DECLARATION_CONTRACT: LazyLock<ManagedDeclarationContract> = LazyLock::new(|| {
    ManagedDeclarationContract::new([
        ManagedSurfaceKind::ManagedDocument,
        ManagedSurfaceKind::CompleteSkillTree,
    ])
    .expect("Copilot declaration contract is non-empty")
});

impl CopilotAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for CopilotAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("copilot").expect("static harness id is valid"),
            display_name: "GitHub Copilot CLI",
            default_binary: "copilot",
            distribution_surface: DistributionSurface::Managed,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text =
            std::str::from_utf8(stdout).map_err(|_| crate::DetectionError::InvalidVersion)?;
        let mut lines = text.lines();
        let first = lines.next().ok_or(crate::DetectionError::InvalidVersion)?;
        let version = first
            .strip_prefix("GitHub Copilot CLI ")
            .and_then(|value| value.strip_suffix('.'))
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_whitespace))
            .ok_or(crate::DetectionError::InvalidVersion)?;
        if lines.next() != Some("Run 'copilot update' to check for updates.")
            || lines.next().is_some()
        {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(version).map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn select_profile(
        &self,
        version: &NativeVersion,
    ) -> skilltap_core::domain::CapabilityProfileSelection {
        let capabilities = copilot_capabilities();
        if version.as_str() == VERIFIED_VERSION {
            skilltap_core::domain::CapabilityProfileSelection::verified(
                CapabilityProfileId::new(PROFILE_ID).expect("compiled profile id is valid"),
                capabilities,
            )
        } else {
            let unknown = ScopedCapabilitySets::new(
                unknown_set(capabilities.for_scope_kind(CapabilityScope::Global)),
                unknown_set(capabilities.for_scope_kind(CapabilityScope::Project)),
            );
            skilltap_core::domain::CapabilityProfileSelection::unknown_version(unknown)
        }
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let (roots, surfaces) = match scope {
            Scope::Global => (
                vec![
                    (
                        "copilot.agents.skills",
                        child(paths.home(), ".agents/skills"),
                    ),
                    ("copilot.skills", child(paths.home(), ".copilot/skills")),
                ],
                copilot_global_surfaces(paths.home()),
            ),
            Scope::Project(project) => (
                vec![
                    (
                        "project.copilot.agents.skills",
                        child(project, ".agents/skills"),
                    ),
                    (
                        "project.copilot.github.skills",
                        child(project, ".github/skills"),
                    ),
                    (
                        "project.copilot.claude.skills",
                        child(project, ".claude/skills"),
                    ),
                ],
                copilot_project_surfaces(project),
            ),
        };
        let mut canonical = Vec::new();
        let mut entries = 0usize;
        for (label, root) in roots
            .into_iter()
            .filter_map(|(label, root)| root.map(|root| (label, root)))
        {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    entries = entries.saturating_add(snapshot.entries().len());
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(entries),
            surface_labels: surfaces,
        })
    }

    // Copilot's exact stable 1.0.70 binary has no verified project-scoped
    // native lifecycle. Its documented plugin commands are therefore not
    // exposed as native authority, and no native cache is ever written.
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }

    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(CopilotManagedProjection::static_ref())
    }

    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        Some(&PROBE)
    }

    fn managed_declaration_contract(
        &self,
        _scope: CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        child(paths.home(), COPILOT_HOME)
    }
}

impl SkillProjectionPort for CopilotSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => child(paths.home(), ".agents/skills"),
            Scope::Project(project) => child(project, ".agents/skills"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopilotEffectiveMcpObservation {
    pub declared: BTreeMap<NativeId, Fingerprint>,
    pub effective: BTreeMap<NativeId, Fingerprint>,
    pub policy: CopilotPolicyHealth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopilotPolicyHealth {
    Allowed,
    TrustRequired,
    EnterpriseBlocked,
    Unknown,
}

impl CopilotEffectiveStateProbe {
    pub fn list_arguments(&self) -> Vec<OsString> {
        ["mcp", "list", "--json"]
            .into_iter()
            .map(OsString::from)
            .collect()
    }

    pub fn get_arguments(&self, name: &NativeId) -> Vec<OsString> {
        ["mcp", "get", name.as_str(), "--json"]
            .into_iter()
            .map(OsString::from)
            .collect()
    }

    pub fn decode_effective(
        &self,
        output: &[u8],
        limits: JsonLimits,
    ) -> Result<CopilotEffectiveMcpObservation, EffectiveProbeError> {
        decode_copilot_mcp_json(output, limits)
    }
}

impl EffectiveStateProbePort for CopilotEffectiveStateProbe {
    fn mcp_status_spec(&self, scope: &Scope) -> EffectiveProbeSpec {
        EffectiveProbeSpec {
            arguments: self.list_arguments(),
            working_directory: match scope {
                Scope::Global => None,
                Scope::Project(project) => Some(project.clone()),
            },
        }
    }

    fn decode_mcp_status(
        &self,
        output: &[u8],
        limits: JsonLimits,
    ) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
        let decoded = decode_copilot_mcp_json(output, limits)?;
        let servers = decoded
            .effective
            .keys()
            .cloned()
            .map(|name| (name, EffectiveServerHealth::Healthy))
            .collect();
        Ok(EffectiveMcpStatus {
            servers,
            project_trust: match decoded.policy {
                CopilotPolicyHealth::TrustRequired => Some(ProjectTrustHealth::Untrusted),
                CopilotPolicyHealth::Allowed => Some(ProjectTrustHealth::Trusted),
                CopilotPolicyHealth::EnterpriseBlocked | CopilotPolicyHealth::Unknown => None,
            },
        })
    }

    fn reload_semantics(&self) -> ReloadSemantics {
        ReloadSemantics::HotReload
    }
}

/// Convert policy/trust evidence into the existing bounded finding vocabulary.
/// The native payload is never carried into a finding.
pub fn copilot_policy_finding(
    scope: Scope,
    policy: CopilotPolicyHealth,
) -> Option<ObservationFinding> {
    let (code, summary, severity) = match policy {
        CopilotPolicyHealth::TrustRequired => (
            ObservationFindingCode::TrustRequired,
            ObservationSummary::TrustRequired,
            ObservationSeverity::Warning,
        ),
        CopilotPolicyHealth::EnterpriseBlocked => (
            ObservationFindingCode::HigherPrecedenceConfiguration,
            ObservationSummary::HigherPrecedenceConfiguration,
            ObservationSeverity::Blocking,
        ),
        CopilotPolicyHealth::Allowed | CopilotPolicyHealth::Unknown => return None,
    };
    Some(ObservationFinding::new(
        code,
        summary,
        severity,
        ObservationSubject::Harness {
            harness: HarnessId::new("copilot").expect("static harness id is valid"),
            scope,
        },
        ObservationFields::default(),
    ))
}

fn decode_copilot_mcp_json(
    output: &[u8],
    limits: JsonLimits,
) -> Result<CopilotEffectiveMcpObservation, EffectiveProbeError> {
    let decoded = skilltap_core::runtime::StrictJson
        .decode(output, limits)
        .map_err(EffectiveProbeError::Runtime)?;
    let object = decoded
        .value()
        .as_object()
        .ok_or(EffectiveProbeError::InvalidPayload)?;
    let servers = object
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .ok_or(EffectiveProbeError::InvalidPayload)?;
    let mut fingerprints = BTreeMap::new();
    let mut policy = policy_from_document(object);
    for (name, value) in servers {
        let id = NativeId::new(name).map_err(|_| EffectiveProbeError::InvalidPayload)?;
        let fingerprint = fingerprint_contents(
            &serde_json::to_vec(value).map_err(|_| EffectiveProbeError::InvalidPayload)?,
        );
        if fingerprints.insert(id, fingerprint).is_some() {
            return Err(EffectiveProbeError::InvalidPayload);
        }
        if server_policy(value) == CopilotPolicyHealth::EnterpriseBlocked {
            policy = CopilotPolicyHealth::EnterpriseBlocked;
        } else if server_policy(value) == CopilotPolicyHealth::TrustRequired
            && policy == CopilotPolicyHealth::Allowed
        {
            policy = CopilotPolicyHealth::TrustRequired;
        }
    }
    Ok(CopilotEffectiveMcpObservation {
        declared: fingerprints.clone(),
        effective: fingerprints,
        policy,
    })
}

fn policy_from_document(
    object: &serde_json::Map<String, serde_json::Value>,
) -> CopilotPolicyHealth {
    if object
        .get("enterprisePolicy")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| matches!(value, "blocked" | "denied"))
        || object
            .get("policy")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| matches!(value, "blocked" | "denied"))
    {
        return CopilotPolicyHealth::EnterpriseBlocked;
    }
    if object.get("trusted").and_then(serde_json::Value::as_bool) == Some(false)
        || object
            .get("workspaceTrusted")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
        || object
            .get("trust")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| matches!(value, "required" | "untrusted"))
    {
        return CopilotPolicyHealth::TrustRequired;
    }
    CopilotPolicyHealth::Allowed
}

fn server_policy(value: &serde_json::Value) -> CopilotPolicyHealth {
    let Some(object) = value.as_object() else {
        return CopilotPolicyHealth::Unknown;
    };
    if object
        .get("policy")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| matches!(value, "blocked" | "denied"))
    {
        CopilotPolicyHealth::EnterpriseBlocked
    } else if object
        .get("trust")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| matches!(value, "required" | "untrusted"))
    {
        CopilotPolicyHealth::TrustRequired
    } else {
        CopilotPolicyHealth::Allowed
    }
}

fn copilot_capabilities() -> ScopedCapabilitySets {
    let capability = |id: &'static str, support: CapabilitySupport| {
        (
            CapabilityId::new(id).expect("Copilot compiled capability is valid"),
            support,
        )
    };
    let make = || {
        CapabilitySet::new([
            capability("harness.observe", CapabilitySupport::Supported),
            capability("managed.projection", CapabilitySupport::Supported),
            capability("skill.install", CapabilitySupport::Supported),
            capability("skill.update", CapabilitySupport::Supported),
            capability("skill.remove", CapabilitySupport::Supported),
            capability("component.skill", CapabilitySupport::Unverified),
            capability("component.mcp", CapabilitySupport::Supported),
        ])
    };
    ScopedCapabilitySets::new(make(), make())
}

fn unknown_set(set: &CapabilitySet) -> CapabilitySet {
    CapabilitySet::new(
        set.iter()
            .map(|(id, _)| (id.clone(), CapabilitySupport::Unverified)),
    )
}

fn child(root: &AbsolutePath, relative: &str) -> Option<AbsolutePath> {
    AbsolutePath::new(format!("{}/{}", root.as_str(), relative)).ok()
}

fn existing(root: &AbsolutePath, relative: &str) -> bool {
    std::fs::symlink_metadata(format!("{}/{}", root.as_str(), relative)).is_ok()
}

fn copilot_global_surfaces(home: &AbsolutePath) -> Vec<&'static str> {
    [
        ("copilot.mcp", ".copilot/mcp-config.json"),
        ("copilot.settings", ".copilot/settings.json"),
        ("copilot.plugins", ".copilot/plugins"),
        ("copilot.skills", ".copilot/skills"),
    ]
    .into_iter()
    .filter_map(|(label, relative)| existing(home, relative).then_some(label))
    .collect()
}

fn copilot_project_surfaces(project: &AbsolutePath) -> Vec<&'static str> {
    [
        ("project.copilot.mcp", ".mcp.json"),
        ("project.copilot.github.mcp", ".github/mcp.json"),
        ("project.copilot.settings", ".github/copilot/settings.json"),
        ("project.copilot.github.skills", ".github/skills"),
        ("project.copilot.claude.skills", ".claude/skills"),
    ]
    .into_iter()
    .filter_map(|(label, relative)| existing(project, relative).then_some(label))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_test_support::TempRoot;
    use std::{ffi::OsString, fs};

    struct TestEnvironment(OsString);

    impl skilltap_core::runtime::Environment for TestEnvironment {
        fn value(&self, variable: skilltap_core::runtime::EnvironmentVariable) -> Option<OsString> {
            (variable == skilltap_core::runtime::EnvironmentVariable::Home).then(|| self.0.clone())
        }
    }

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).unwrap()
    }

    #[test]
    fn exact_copilot_version_is_registered_but_unknown_versions_are_observe_only() {
        let adapter = CopilotAdapter;
        assert_eq!(
            adapter
                .decode_version(
                    b"GitHub Copilot CLI 1.0.70.\nRun 'copilot update' to check for updates.\n"
                )
                .unwrap()
                .as_str(),
            VERIFIED_VERSION
        );
        assert_eq!(
            adapter
                .select_profile(&NativeVersion::new(VERIFIED_VERSION).unwrap())
                .profile_id()
                .unwrap()
                .as_str(),
            PROFILE_ID
        );
        for version in ["1.0.69", "1.0.71", "99.0.0"] {
            assert!(
                adapter
                    .select_profile(&NativeVersion::new(version).unwrap())
                    .mutation_capabilities()
                    .is_none(),
                "{version} must remain observe-only"
            );
        }
    }

    #[test]
    fn copilot_observation_accepts_canonical_roots_and_file_surfaces() {
        let root = TempRoot::new("copilot-observation").unwrap();
        fs::create_dir_all(root.join(".agents/skills/sibling")).unwrap();
        fs::write(root.join(".agents/skills/sibling/SKILL.md"), b"skill").unwrap();
        fs::create_dir_all(root.join(".copilot")).unwrap();
        fs::write(root.join(".copilot/mcp-config.json"), br#"{"future":true}"#).unwrap();
        let environment =
            TestEnvironment(OsString::from(root.path().to_string_lossy().to_string()));
        let paths = PlatformPaths::resolve_for(
            skilltap_core::runtime::SupportedPlatform::Linux,
            &environment,
        )
        .unwrap();
        let observation = CopilotAdapter
            .observe(
                &paths,
                &Scope::Global,
                ExternalTreeLimits::new(64, 1000, 1024 * 1024, 1024 * 1024, 1024).unwrap(),
            )
            .unwrap();
        assert!(
            observation
                .canonical
                .iter()
                .any(|root| root.root == "copilot.agents.skills")
        );
        assert_eq!(observation.surface_labels, vec!["copilot.mcp"]);
    }

    #[test]
    fn copilot_has_no_native_plugin_or_marketplace_ports() {
        let adapter = CopilotAdapter;
        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.native_distribution().is_none());
        let selected = adapter.select_profile(&NativeVersion::new(VERIFIED_VERSION).unwrap());
        let profile = selected.mutation_capabilities().unwrap();
        for scope in [CapabilityScope::Global, CapabilityScope::Project] {
            let set = profile.for_scope_kind(scope);
            for id in [
                "plugin.install",
                "plugin.remove",
                "plugin.update",
                "marketplace.register",
                "marketplace.remove",
                "marketplace.update",
            ] {
                assert_eq!(set.support(&CapabilityId::new(id).unwrap()), None);
            }
            assert_eq!(
                set.support(&CapabilityId::new("component.skill").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
            assert_eq!(
                set.support(&CapabilityId::new("component.mcp").unwrap()),
                Some(CapabilitySupport::Supported)
            );
        }
    }

    #[test]
    fn structured_mcp_list_and_get_evidence_redacts_by_fingerprinting_and_preserves_policy() {
        let probe = CopilotEffectiveStateProbe;
        assert_eq!(
            probe.list_arguments(),
            ["mcp", "list", "--json"].map(OsString::from)
        );
        assert_eq!(
            probe.get_arguments(&NativeId::new("docs").unwrap()),
            ["mcp", "get", "docs", "--json"].map(OsString::from)
        );
        let observation = probe
            .decode_effective(
                br#"{"trusted":false,"mcpServers":{"docs":{"type":"stdio","command":"node","env":{"TOKEN":"secret"}}}}"#,
                limits(),
            )
            .unwrap();
        assert_eq!(observation.policy, CopilotPolicyHealth::TrustRequired);
        assert!(
            observation
                .declared
                .contains_key(&NativeId::new("docs").unwrap())
        );
        assert!(!format!("{:?}", observation).contains("secret"));
        assert_eq!(
            copilot_policy_finding(Scope::Global, observation.policy)
                .unwrap()
                .code(),
            ObservationFindingCode::TrustRequired
        );
    }

    #[test]
    fn enterprise_policy_is_distinct_from_trust() {
        let observation = decode_copilot_mcp_json(
            br#"{"enterprisePolicy":"blocked","mcpServers":{}}"#,
            limits(),
        )
        .unwrap();
        assert_eq!(observation.policy, CopilotPolicyHealth::EnterpriseBlocked);
        assert_eq!(
            copilot_policy_finding(
                Scope::Project(AbsolutePath::new("/project").unwrap()),
                observation.policy
            )
            .unwrap()
            .code(),
            ObservationFindingCode::HigherPrecedenceConfiguration
        );
    }
}
