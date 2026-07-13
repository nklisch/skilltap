use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityProfileSelection, CapabilityScope, HarnessId, NativeVersion, Scope,
    },
    runtime::{ExternalTreeLimits, JsonLimits, PlatformPaths},
};

use crate::{
    adapter_helpers,
    lifecycle::{NativeLifecycleAction, NativeLifecycleError, NativeLifecycleRequest},
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, InstructionBridgePort,
        NativeLifecycleVector, ObservationPathError, SkillProjectionPort, TargetIdentity,
    },
};

pub struct CodexAdapter;
pub struct CodexLifecycle;
pub struct CodexInstructionBridge;
pub struct CodexSkillProjection;

static ADAPTER: CodexAdapter = CodexAdapter;
static LIFECYCLE: CodexLifecycle = CodexLifecycle;
static INSTRUCTIONS: CodexInstructionBridge = CodexInstructionBridge;
static SKILLS: CodexSkillProjection = CodexSkillProjection;

impl CodexAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for CodexAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("codex").expect("static harness id is valid"),
            display_name: "Codex",
            distribution_surface: DistributionSurface::FirstPartyPlugin,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        self.decode_version_with_limits(stdout, default_json_limits())
    }

    fn decode_version_with_limits(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<NativeVersion, crate::DetectionError> {
        adapter_helpers::decode_codex_version(stdout, limits)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            "0.144.1",
            "codex-0-144-1",
            adapter_helpers::compiled_capabilities(false, false),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        adapter_helpers::observe_codex(paths, scope, limits)
    }

    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        Some(&LIFECYCLE)
    }

    fn instruction_bridge(&self) -> Option<&dyn InstructionBridgePort> {
        Some(&INSTRUCTIONS)
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }
}

impl NativeLifecycleVector for CodexLifecycle {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        // Preserve the original diagnostic precedence: an unsupported action
        // is rejected before considering whether its scope is also unsupported.
        if request.action == NativeLifecycleAction::PluginUpdate {
            return Err(NativeLifecycleError::UnsupportedAction);
        }
        if matches!(request.scope, Scope::Project(_)) {
            return Err(NativeLifecycleError::UnsupportedProjectScope);
        }

        let mut args = vec![OsString::from("plugin")];
        match request.action {
            NativeLifecycleAction::MarketplaceAdd => {
                args.extend(["marketplace", "add"].into_iter().map(OsString::from));
                args.push(OsString::from(
                    request
                        .source
                        .as_ref()
                        .ok_or(NativeLifecycleError::MissingSource)?
                        .as_str(),
                ));
            }
            NativeLifecycleAction::MarketplaceRemove => args.extend(
                ["marketplace", "remove", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::MarketplaceUpdate => args.extend(
                ["marketplace", "upgrade", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginInstall => args.extend(
                ["add", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginRemove => args.extend(
                ["remove", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginUpdate => unreachable!("rejected above"),
        }
        Ok(args)
    }

    fn observation_scope(&self, _scope: &Scope) -> Option<CapabilityScope> {
        None
    }
}

impl InstructionBridgePort for CodexInstructionBridge {
    fn global_bridge(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.codex_home(), "AGENTS.md")
    }

    fn project_bridge(&self, _project: &AbsolutePath) -> Option<AbsolutePath> {
        // Codex reads the canonical project AGENTS.md directly.
        None
    }
}

impl SkillProjectionPort for CodexSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

fn default_json_limits() -> JsonLimits {
    JsonLimits::new(64 * 1024, 32).expect("static adapter JSON limits are valid")
}

#[cfg(test)]
mod tests {
    use skilltap_core::domain::{NativeId, SourceLocator};

    use super::*;
    use crate::HarnessKind;

    #[test]
    fn plugin_update_is_rejected_before_project_scope() {
        let request = NativeLifecycleRequest {
            harness: HarnessKind::Codex,
            action: NativeLifecycleAction::PluginUpdate,
            scope: Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            name: NativeId::new("formatter@team").unwrap(),
            source: Some(SourceLocator::new("https://example.invalid/team.git").unwrap()),
        };

        assert_eq!(
            LIFECYCLE.arguments(&request),
            Err(NativeLifecycleError::UnsupportedAction)
        );
    }
}
