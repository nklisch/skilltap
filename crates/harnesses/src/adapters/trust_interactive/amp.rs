use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        HarnessId, NativeId, NativeVersion, Scope, ScopedCapabilitySets,
    },
    mutation_authority::{ManagedDeclarationContract, ManagedSurfaceKind},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        ObservationRuntimeError, PlatformPaths, StrictJson, StrictJsonDecoder,
        SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::{amp_projection::AmpManagedProjection, contracts::AMP};

static ADAPTER: AmpAdapter = AmpAdapter;
static SKILLS: AmpSkillProjection = AmpSkillProjection;
static DECLARATION_CONTRACT: std::sync::LazyLock<ManagedDeclarationContract> =
    std::sync::LazyLock::new(|| {
        ManagedDeclarationContract::new([
            ManagedSurfaceKind::ManagedDocument,
            ManagedSurfaceKind::CompleteSkillTree,
        ])
        .expect("Amp declaration contract is non-empty")
    });

pub struct AmpAdapter;
pub struct AmpSkillProjection;

impl AmpAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn capabilities() -> ScopedCapabilitySets {
    let capability = |id: &str| {
        (
            CapabilityId::new(id).expect("Amp capability is valid"),
            CapabilitySupport::Unverified,
        )
    };
    let make = || {
        CapabilitySet::new([
            (
                CapabilityId::new("harness.observe").expect("Amp capability is valid"),
                CapabilitySupport::Supported,
            ),
            capability("managed.projection"),
            capability("component.skill"),
            capability("component.mcp"),
            capability("skill.install"),
            capability("skill.update"),
            capability("skill.remove"),
        ])
    };
    ScopedCapabilitySets::new(make(), make())
}

impl HarnessAdapter for AmpAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("amp").expect("static harness id is valid"),
            display_name: "Amp",
            default_binary: Some(AMP.default_binary),
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: crate::TargetIdentityBoundary::Executable,
        }
    }

    fn version_arguments(&self) -> Option<Vec<OsString>> {
        Some(AMP.version_arguments.iter().map(OsString::from).collect())
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| crate::DetectionError::InvalidVersion)?
            .strip_suffix('\n')
            .ok_or(crate::DetectionError::InvalidVersion)?;
        if text.contains('\r') || text.chars().any(char::is_control) {
            return Err(crate::DetectionError::InvalidVersion);
        }
        let (version, rest) = text
            .split_once(" (released ")
            .ok_or(crate::DetectionError::InvalidVersion)?;
        let (release, age) = rest
            .strip_suffix(")")
            .and_then(|value| value.split_once(", "))
            .ok_or(crate::DetectionError::InvalidVersion)?;
        if !is_version_token(version) || !is_release_timestamp(release) || !is_age_token(age) {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(format!("{version}+{release}"))
            .map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            AMP.verified_version,
            AMP.profile_id,
            capabilities(),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let roots = match scope {
            Scope::Global => vec![
                (
                    "amp.config.agents.skills",
                    child(paths.config_home(), "agents/skills"),
                ),
                ("amp.agents.skills", child(paths.home(), ".agents/skills")),
                (
                    "amp.config.skills",
                    child(paths.config_home(), "amp/skills"),
                ),
                ("amp.claude.skills", child(paths.home(), ".claude/skills")),
            ],
            Scope::Project(project) => vec![
                (
                    "project.amp.agents.skills",
                    child(project, ".agents/skills"),
                ),
                (
                    "project.amp.claude.skills",
                    child(project, ".claude/skills"),
                ),
            ],
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = 0usize;
        for (label, root) in roots
            .into_iter()
            .filter_map(|(label, root)| root.map(|root| (label, root)))
        {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    project_entry_count =
                        project_entry_count.saturating_add(snapshot.entries().len());
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        let mut surface_labels = Vec::new();
        for (label, path) in settings_surfaces(paths, scope) {
            if adapter_helpers::path_exists(&path) {
                surface_labels.push(label);
            }
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(project_entry_count),
            surface_labels,
        })
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }

    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(AmpManagedProjection::static_ref())
    }

    fn managed_declaration_contract(
        &self,
        _scope: skilltap_core::domain::CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(paths.config_home().clone())
    }
}

impl SkillProjectionPort for AmpSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => child(paths.home(), ".agents/skills"),
            Scope::Project(project) => child(project, ".agents/skills"),
        }
    }
}

/// The only native JSON command that may be considered by this adapter is a
/// finite declaration listing. It is intentionally not exposed as an
/// `EffectiveStateProbePort`: its result never proves connection, trust, or
/// authentication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmpDeclaredServer {
    pub name: NativeId,
    pub source: AmpDeclaredSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmpDeclaredSource {
    Global,
    Workspace,
    CommandLine,
}

pub fn declared_list_arguments() -> Vec<OsString> {
    AMP.declared_list_arguments
        .expect("Amp declared list contract is present")
        .iter()
        .map(OsString::from)
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmpDeclaredListError {
    InvalidJson,
    InvalidPayload,
}

pub fn decode_declared_mcp_list(
    output: &[u8],
    limits: JsonLimits,
) -> Result<Vec<AmpDeclaredServer>, AmpDeclaredListError> {
    const MAX_DECLARED_SERVERS: usize = 1_024;

    let decoded = StrictJson
        .decode(output, limits)
        .map_err(|_| AmpDeclaredListError::InvalidJson)?;
    let entries = decoded
        .value()
        .as_array()
        .ok_or(AmpDeclaredListError::InvalidPayload)?;
    if entries.len() > MAX_DECLARED_SERVERS {
        return Err(AmpDeclaredListError::InvalidPayload);
    }
    let mut servers = Vec::with_capacity(entries.len());
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or(AmpDeclaredListError::InvalidPayload)?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or(AmpDeclaredListError::InvalidPayload)
            .and_then(|name| {
                NativeId::new(name).map_err(|_| AmpDeclaredListError::InvalidPayload)
            })?;
        let source = match object.get("source").and_then(serde_json::Value::as_str) {
            Some("global") => AmpDeclaredSource::Global,
            Some("workspace") => AmpDeclaredSource::Workspace,
            Some("--mcp-config flag") | Some("command_line") => AmpDeclaredSource::CommandLine,
            _ => return Err(AmpDeclaredListError::InvalidPayload),
        };
        if object
            .get("spec")
            .and_then(serde_json::Value::as_object)
            .is_none()
        {
            return Err(AmpDeclaredListError::InvalidPayload);
        }
        servers.push(AmpDeclaredServer { name, source });
    }
    Ok(servers)
}

fn settings_surfaces(paths: &PlatformPaths, scope: &Scope) -> Vec<(&'static str, AbsolutePath)> {
    match scope {
        Scope::Global => vec![
            (
                "amp.settings",
                child(paths.config_home(), "amp/settings.json").unwrap(),
            ),
            (
                "amp.settings.jsonc",
                child(paths.config_home(), "amp/settings.jsonc").unwrap(),
            ),
        ],
        Scope::Project(project) => vec![
            (
                "project.amp.settings",
                child(project, ".amp/settings.json").unwrap(),
            ),
            (
                "project.amp.settings.jsonc",
                child(project, ".amp/settings.jsonc").unwrap(),
            ),
        ],
    }
}

fn child(root: &AbsolutePath, relative: &str) -> Option<AbsolutePath> {
    adapter_helpers::absolute_child(root, relative)
}

fn is_version_token(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

fn is_release_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 24
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'.'
        && bytes[23] == b'Z'
        && value.chars().enumerate().all(|(index, character)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19 | 23) || character.is_ascii_digit()
        })
}

fn is_age_token(value: &str) -> bool {
    let Some(age) = value.strip_suffix(" ago") else {
        return false;
    };
    let Some(unit) = age.chars().last() else {
        return false;
    };
    matches!(unit, 's' | 'm' | 'h' | 'd')
        && age[..age.len() - unit.len_utf8()]
            .chars()
            .all(|character| character.is_ascii_digit())
        && age.len() > unit.len_utf8()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).unwrap()
    }

    #[test]
    fn exact_amp_identity_uses_release_timestamp_not_relative_age() {
        let adapter = AmpAdapter;
        let version = adapter
            .decode_version(b"0.0.1784073393-g9a3a12 (released 2026-07-14T23:56:33.000Z, 8m ago)\n")
            .unwrap();
        assert_eq!(version.as_str(), AMP.verified_version);
        assert!(
            adapter
                .select_profile(&version)
                .mutation_capabilities()
                .is_some()
        );
        assert_eq!(
            adapter
                .decode_version(
                    b"0.0.1784073393-g9a3a12 (released 2026-07-14T23:56:34.000Z, 8m ago)\n"
                )
                .unwrap()
                .as_str(),
            "0.0.1784073393-g9a3a12+2026-07-14T23:56:34.000Z"
        );
        for output in [
            b"0.0.1784073393-g9a3a12 (released 2026-07-14T23:56:33.000Z, 8m ago) extra\n"
                .as_slice(),
            b"0.0.1784073393-g9a3a12\n".as_slice(),
            b"0.0.1784073393-g9a3a12 (released 2026-07-14T23:56:33.000Z, login)\n".as_slice(),
        ] {
            assert_eq!(
                adapter.decode_version(output),
                Err(crate::DetectionError::InvalidVersion)
            );
        }
        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.effective_state_probe().is_none());
    }

    #[test]
    fn declared_list_decoder_never_produces_effective_health() {
        assert_eq!(
            declared_list_arguments(),
            ["mcp", "list", "--json"].map(OsString::from)
        );
        let result = decode_declared_mcp_list(
            br#"[{"name":"global-only","source":"global","spec":{"command":"/bin/true"}},{"name":"workspace","source":"workspace","spec":{"url":"https://example.invalid"}}]"#,
            limits(),
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].source, AmpDeclaredSource::Global);
        assert!(
            decode_declared_mcp_list(br#"[{"name":"x","source":"workspace"}]"#, limits()).is_err()
        );
        assert_eq!(
            decode_declared_mcp_list(b"not-json", limits()),
            Err(AmpDeclaredListError::InvalidJson)
        );
    }
}
