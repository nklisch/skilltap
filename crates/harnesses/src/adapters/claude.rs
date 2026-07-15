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

pub struct ClaudeAdapter;
pub struct ClaudeLifecycle;
pub struct ClaudeInstructionBridge;
pub struct ClaudeSkillProjection;

static ADAPTER: ClaudeAdapter = ClaudeAdapter;
static LIFECYCLE: ClaudeLifecycle = ClaudeLifecycle;
static INSTRUCTIONS: ClaudeInstructionBridge = ClaudeInstructionBridge;
static SKILLS: ClaudeSkillProjection = ClaudeSkillProjection;

impl ClaudeAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for ClaudeAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("claude").expect("static harness id is valid"),
            display_name: "Claude Code",
            default_binary: "claude",
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
        adapter_helpers::decode_claude_version(stdout, limits)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            "2.1.201",
            "claude-2-1-201",
            adapter_helpers::compiled_capabilities(true, true, false),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        adapter_helpers::observe_claude(paths, scope, limits)
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

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(paths.claude_home().clone())
    }

    fn bootstrap_capability_next_action(&self) -> &'static str {
        "Run `claude plugin install skilltap --scope user` through Claude's native consent flow."
    }
}

impl NativeLifecycleVector for ClaudeLifecycle {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        let scope = if matches!(request.scope, Scope::Project(_)) {
            "local"
        } else {
            "user"
        };
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
                ["marketplace", "update", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginInstall => args.extend(
                ["install", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginRemove => args.extend(
                ["uninstall", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginUpdate => args.extend(
                ["update", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
        }
        if request.action != NativeLifecycleAction::MarketplaceUpdate {
            args.extend(["--scope", scope].into_iter().map(OsString::from));
        }
        Ok(args)
    }

    fn observation_scope(&self, scope: &Scope) -> Option<CapabilityScope> {
        Some(CapabilityScope::from(scope))
    }
}

impl InstructionBridgePort for ClaudeInstructionBridge {
    fn global_bridge(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.claude_home(), "CLAUDE.md")
    }

    fn project_bridge(&self, project: &AbsolutePath) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(project, "CLAUDE.md")
    }

    fn alternate_project_bridges(&self, project: &AbsolutePath) -> Vec<AbsolutePath> {
        adapter_helpers::absolute_child(project, ".claude/CLAUDE.md")
            .into_iter()
            .collect()
    }
}

impl SkillProjectionPort for ClaudeSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.claude_home(), "skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".claude/skills"),
        }
    }
}

fn default_json_limits() -> JsonLimits {
    JsonLimits::new(64 * 1024, 32).expect("static adapter JSON limits are valid")
}
