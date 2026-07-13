use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

use skilltap_core::{
    adoption::{
        AdoptionApplyError, AdoptionDecision, AdoptionObservationError, AdoptionSelection,
        apply_adoption, plan_adoption,
    },
    domain::{
        AbsolutePath, ArtifactFile, CapabilityId, CapabilitySupport, CommandArgument,
        ComponentGraph, ConfiguredBinary, DesiredOrigin, DesiredResource, Fingerprint,
        FingerprintAlgorithm, GitCommit, HarnessId, HarnessObservation, HarnessObservationOutcome,
        HarnessReachability, HarnessSet, NativeId, ObservationAdapterError, ObservationBatch,
        ObservationEvidence, ObservationFields, ObservationFinding, ObservationFindingCode,
        ObservationKey, ObservationLayer, ObservationRequest, ObservationSeverity,
        ObservationSubject, ObservationSummary, ObservationTarget, ObservedResource,
        OperationAction, OperationId, OperationOutcome, OperationResult, OperationSelector,
        Ownership, Plan, ProfileAuthority, Provenance, ResourceHealth, ResourceId, ResourceKey,
        ResourceKind, Scope, Source, SourceKind, SourceLocator, UpdateIntent,
    },
    executor::{ExecutionError, ExecutionJournal, ExecutionPort, execute_plan},
    foreground_update::{
        ForegroundUpdateRequest, plan_foreground_updates,
        select_foreground_updates_with_acknowledgment,
    },
    instructions::{
        InstructionBridgeMode as CoreInstructionBridgeMode, InstructionBridgeRepresentation,
        InstructionBridgeSpec, InstructionHealth, ObservedInstructionBridge, classify_bridge,
        fingerprint_contents, relative_symlink_target, resolve_symlink_target,
    },
    lifecycle_operation::native_operation,
    runtime::{
        DirectoryTreeFileSystem, ExecutableResolutionRequest, ExecutableResolver,
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, FileKind, FileSystem,
        JsonLimits, NativeProcessRequest, NativeProcessRunner, PlatformPaths, ProcessEnvironment,
        ProcessLimits, RelativeSymlinkTarget, ScopeRequest, ScopeResolver, SystemConfigurationLock,
        SystemExecutableResolver, SystemExternalTreeObserver, SystemFileSystem,
        SystemNativeProcessRunner, WorkingDirectory, resolve_targets,
    },
    skill::ValidatedSkillTree,
    skill_compatibility::{SkillCompatibility, SkillCompatibilityClass},
    storage::{
        ArtifactTree, ClaudeInstructionMode, ConfigDocument, ConfigRepository, DocumentState,
        InventoryDocument, InventoryRepository, ManagedArtifactRepository, ResourceState,
        StateDocument, StateRepository, StorageError, StorageFailure, TargetResourceState,
        Timestamp,
    },
    updates::{
        ResolutionError, SourceRevisionResolver, UpdateCandidate, UpdateDecision,
        UpdateDecisionReason, UpdateResolutionRequest, UpdateSafety, candidate_for,
        classify_update_with_mode, resolve_candidate,
    },
};
use skilltap_harnesses::{
    CanonicalObservation, CodexPluginGraphReader, DetectionError, GitSourceRevisionResolver,
    HarnessKind, ManagedCodexCatalog, NativeLifecycleAction, NativeLifecyclePort,
    NativeLifecycleRequest, NativeResourcePresence, ObservedNativeRevisionResolver,
    detect_configured_installation, native_arguments, normalize_observations,
    observe_claude_canonical_resources, observe_codex_canonical_resources, observe_native_resource,
    select_profile,
};

pub(super) struct DetectionDiagnostic {
    pub(super) warning: Warning,
    pub(super) next_action: NextAction,
}

pub(super) fn detection_diagnostic(error: &DetectionError, harness: &str) -> DetectionDiagnostic {
    use skilltap_core::runtime::ObservationRuntimeError;

    let (code, summary, action_code, action_summary, command) = match error {
        DetectionError::InvalidVersion => (
            "native_version_invalid",
            "The harness returned an invalid version response.",
            "inspect_harness_version",
            "Inspect the harness version response and configure a supported binary.",
            format!("{harness} --version"),
        ),
        DetectionError::NonZeroExit => (
            "native_version_command_failed",
            "The harness version command returned a nonzero status.",
            "inspect_harness_version",
            "Run the harness version command directly and resolve its failure.",
            format!("{harness} --version"),
        ),
        DetectionError::Runtime(ObservationRuntimeError::ExecutableNotFound) => (
            "native_executable_not_found",
            "The configured harness executable was not found.",
            "configure_harness_binary",
            "Configure an installed harness executable and retry.",
            format!("skilltap harness enable {harness} --binary <path>"),
        ),
        DetectionError::Runtime(
            ObservationRuntimeError::ProcessDeadlineExceeded
            | ObservationRuntimeError::ProcessOutputLimitExceeded { .. },
        ) => (
            "native_detection_bounded",
            "Harness detection exceeded a safety limit.",
            "inspect_harness_version",
            "Run the harness version command directly and resolve the bounded failure.",
            format!("{harness} --version"),
        ),
        DetectionError::Runtime(_) => (
            "native_detection_runtime_failed",
            "The configured harness could not be started or inspected safely.",
            "inspect_harness_binary",
            "Inspect the configured harness executable and retry.",
            format!("skilltap harness enable {harness} --binary <path>"),
        ),
    };
    DetectionDiagnostic {
        warning: Warning::new(code, summary).with_context("harness", harness),
        next_action: NextAction::new(action_code, action_summary).with_command(command),
    }
}

use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, OutputScope, OutputValue, ResultClass, Warning,
    command::{
        AdoptArgs, OutputArgs, PlanArgs, ScopeArgs, ScopeArgument, ScopedOutputArgs,
        ScopedTargetArgs, StatusArgs, SyncArgs, TargetArgs,
    },
};

mod execution;
mod instructions;
mod lifecycle;
mod reconciliation;
mod status;

pub(super) use status::first_use_harness_report;
use status::{NativeObservation, StatusDocuments, StatusScope, StatusTargetError, StatusTargets};

use execution::{
    HybridLifecyclePort, InstructionEntry, InstructionPort, InstructionWrite,
    ManagedProjectLifecycleEntry, ManagedProjectLifecyclePort, ManagedProjectPluginWrite,
    ManagedSkillAction, ManagedSkillEntry, ManagedSkillPort, StateExecutionJournal,
};

pub(crate) struct StatusApplication<'a> {
    pub(crate) config: &'a dyn ConfigRepository,
    pub(crate) inventory: &'a dyn InventoryRepository,
    pub(crate) state: &'a dyn StateRepository,
    pub(crate) scopes: &'a ScopeResolver<'a>,
    pub(crate) working_directory: &'a dyn WorkingDirectory,
    pub(crate) native_observation: NativeObservationMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeObservationMode {
    Disabled,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeLifecycleKind {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

pub(crate) struct SkillInstallRequest<'a> {
    pub(crate) source: &'a str,
    pub(crate) name: Option<&'a str>,
    pub(crate) preserve_name: bool,
    pub(crate) requested_revision: Option<&'a str>,
    pub(crate) subdirectory: Option<&'a str>,
}

/// State-backed journal for mutating lifecycle composition. It resolves each
/// result through the validated plan, seeds only explicitly planned resources,
/// updates exact resource records, and publishes atomically through the
/// repository port.
impl StatusApplication<'_> {
    fn persist_daemon_run(
        &self,
        outcome: &mut Outcome,
        safe_operations: u64,
        pending_operations: u64,
    ) {
        let at = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(at) => at,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result timestamp could not be recorded safely.",
                ));
                return;
            }
        };
        let result = if pending_operations > 0 {
            skilltap_core::storage::DaemonRunResult::Pending
        } else if outcome.result == ResultClass::Completed {
            skilltap_core::storage::DaemonRunResult::Completed
        } else {
            skilltap_core::storage::DaemonRunResult::Failed
        };
        let failure_code = (!matches!(result, skilltap_core::storage::DaemonRunResult::Completed))
            .then(|| skilltap_core::domain::EvidenceCode::new("daemon.update_failed").unwrap());
        let record = match skilltap_core::storage::DaemonRunRecord::new(
            at,
            result,
            safe_operations,
            pending_operations,
            failure_code,
        ) {
            Ok(record) => record,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result could not be represented safely.",
                ));
                return;
            }
        };
        let current = match self.state.load() {
            Ok(DocumentState::Present(state)) => state,
            Ok(DocumentState::Missing) => match StateDocument::new(
                skilltap_core::storage::STATE_SCHEMA_VERSION,
                [],
                [],
                None,
                None,
                None,
            ) {
                Ok(state) => state,
                Err(_) => {
                    outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                    *outcome = outcome.clone().with_warning(Warning::new(
                        "daemon_record_failed",
                        "The daemon state document could not be initialized safely.",
                    ));
                    return;
                }
            },
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon state document could not be loaded safely.",
                ));
                return;
            }
        };
        let next = match current.with_daemon_run(record) {
            Ok(next) => next,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result could not be attached to state safely.",
                ));
                return;
            }
        };
        if self.state.replace(&next).is_err() {
            outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
            *outcome = outcome.clone().with_warning(Warning::new(
                "daemon_record_failed",
                "The daemon result could not be published atomically.",
            ));
        }
    }

    /// List desired standalone skills only. This is deliberately inventory
    /// backed and never scans source directories or marketplace contents.
    pub(crate) fn execute_skill_list(&self, args: &ScopedTargetArgs) -> Outcome {
        self.execute_resource_list("skill list", args, ResourceKind::StandaloneSkill)
    }

    pub(crate) fn execute_resource_list(
        &self,
        command: &'static str,
        args: &ScopedTargetArgs,
        kind: ResourceKind,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: args.target.clone(),
            scope: args.scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "no_enabled_harnesses",
                        "No harness is enabled in skilltap configuration.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable Codex or Claude management.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let mut count = 0_u64;
        if let Some(inventory) = &documents.inventory {
            for resource in inventory.resources().values() {
                if resource.kind() != kind
                    || !scope.resolved.iter().any(|value| value == resource.scope())
                    || !resource
                        .targets()
                        .iter()
                        .any(|target| targets.resolved.contains(target))
                {
                    continue;
                }
                count += 1;
                outcome = outcome.with_resource(
                    OutputEntry::new(resource.key().to_string(), "desired")
                        .with_field("kind", format!("{:?}", kind).to_lowercase())
                        .with_field("scope", scope_label(resource.scope()))
                        .with_field("targets", resource.targets().iter().count() as u64),
                );
            }
        }
        outcome.result = ResultClass::Completed;
        outcome
            .with_summary("resources", count)
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
    }
}

fn skill_relative_destination(
    name: &NativeId,
) -> Option<skilltap_core::domain::RelativeArtifactPath> {
    skilltap_core::domain::RelativeArtifactPath::new(format!("skills/{}", name.as_str())).ok()
}

fn skill_install_can_complete(outcome: &Outcome, acknowledged: bool) -> bool {
    outcome.errors.is_empty()
        && (outcome.warnings.is_empty()
            || (acknowledged
                && outcome
                    .warnings
                    .iter()
                    .all(|warning| warning.code == "skill_frontmatter_warning")))
}

fn scope_args_for_scope(scope: &Scope) -> ScopeArgs {
    match scope {
        Scope::Global => ScopeArgs::default(),
        Scope::Project(path) => ScopeArgs {
            project: Some(Some(PathBuf::from(path.as_str()))),
            all_scopes: false,
        },
    }
}

fn merge_result(current: ResultClass, next: ResultClass) -> ResultClass {
    fn rank(result: ResultClass) -> u8 {
        match result {
            ResultClass::Completed => 0,
            ResultClass::Invalid => 1,
            ResultClass::AttentionRequired => 2,
            ResultClass::PartialApply => 3,
        }
    }

    if rank(next) > rank(current) {
        next
    } else {
        current
    }
}

/// A document-load phase starts outcomes in `attention_required` so malformed
/// owned documents cannot be mistaken for successful commands. A daemon cycle
/// that completes safe updates may inherit that provisional class even when
/// every child update completed cleanly. The absence of warnings/errors and
/// pending work is the evidence needed to normalize that aggregate result.
fn normalize_daemon_noop_result(
    outcome: &mut Outcome,
    safe_operations: u64,
    pending_operations: u64,
) {
    if safe_operations > 0
        && pending_operations == 0
        && outcome.warnings.is_empty()
        && outcome.errors.is_empty()
    {
        outcome.result = ResultClass::Completed;
    }
}

/// Merge one resource-scoped lifecycle result into the aggregate reconciliation
/// result.  Child adapters own their operation and journal details; the
/// reconciliation command only combines those already-rendered records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstructionBridgeHealth {
    Missing,
    Managed,
    Conflict,
}

fn instruction_locations(
    paths: &PlatformPaths,
    scope: &Scope,
    enabled: &[HarnessId],
) -> (AbsolutePath, Vec<(HarnessId, AbsolutePath)>) {
    match scope {
        Scope::Global => {
            let canonical = paths.global_agents().clone();
            let mut bridges = Vec::new();
            if enabled.iter().any(|target| target.as_str() == "codex") {
                bridges.push((
                    HarnessId::new("codex").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/AGENTS.md", paths.codex_home().as_str()))
                        .expect("codex bridge path is valid"),
                ));
            }
            if enabled.iter().any(|target| target.as_str() == "claude") {
                bridges.push((
                    HarnessId::new("claude").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/CLAUDE.md", paths.claude_home().as_str()))
                        .expect("claude bridge path is valid"),
                ));
            }
            (canonical, bridges)
        }
        Scope::Project(project) => {
            let canonical = AbsolutePath::new(format!("{}/AGENTS.md", project.as_str()))
                .expect("project canonical path is valid");
            let bridges = if enabled.iter().any(|target| target.as_str() == "claude") {
                vec![(
                    HarnessId::new("claude").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                        .expect("project Claude bridge path is valid"),
                )]
            } else {
                Vec::new()
            };
            (canonical, bridges)
        }
    }
}

/// Resolve the materialized Claude bridge for a project using setup's
/// nested-only preservation policy. The inventory identity remains the
/// ordinary project bridge resource even when setup keeps `.claude/CLAUDE.md`.
fn preferred_instruction_bridge_path(
    filesystem: &dyn FileSystem,
    scope: &Scope,
    target: &HarnessId,
    root_bridge: AbsolutePath,
) -> AbsolutePath {
    if target.as_str() != "claude" {
        return root_bridge;
    }
    let Scope::Project(project) = scope else {
        return root_bridge;
    };
    let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
        .expect("nested project Claude bridge path is valid");
    let root_missing = filesystem
        .inspect(&root_bridge)
        .map(|metadata| metadata.kind() == FileKind::Missing)
        .unwrap_or(false);
    let nested_present = filesystem
        .inspect(&nested)
        .map(|metadata| metadata.kind() != FileKind::Missing)
        .unwrap_or(false);
    if root_missing && nested_present {
        nested
    } else {
        root_bridge
    }
}

fn instruction_resource_key(scope: &Scope, role: &str, target: &str) -> Option<ResourceKey> {
    let scope_label = match scope {
        Scope::Global => "global".to_owned(),
        Scope::Project(path) => format!("project-{:016x}", stable_hash(path.as_str())),
    };
    ResourceId::new(format!("instructions:{scope_label}:{role}:{target}"))
        .ok()
        .map(|id| ResourceKey::new(id, scope.clone()))
}

fn instruction_operation_id(scope: &Scope, role: &str, target: &str) -> OperationId {
    let resource = instruction_resource_key(scope, role, target)
        .expect("instruction resource identity is valid");
    let hash = stable_hash(resource.id().as_str());
    OperationId::new(format!("instructions:{hash:016x}"))
        .expect("instruction operation id is valid")
}

fn instruction_backup_path(paths: &PlatformPaths, bridge: &AbsolutePath) -> AbsolutePath {
    let hash = stable_hash(bridge.as_str());
    AbsolutePath::new(format!(
        "{}/managed/backups/instructions/{hash:016x}.bak",
        paths.skilltap_config().as_str()
    ))
    .expect("instruction backup path is valid")
}

fn instruction_desired_resource(resource: ResourceKey, target: HarnessId) -> DesiredResource {
    DesiredResource::new(
        resource,
        ResourceKind::InstructionLocation,
        HarnessSet::new([target]).expect("instruction target set is non-empty"),
        DesiredOrigin::Direct,
        None,
        UpdateIntent::Pinned,
        ComponentGraph::new([]).expect("empty component graph is valid"),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeSet::new(),
    )
    .expect("instruction desired resource is valid")
}

fn instruction_bridge_status(
    filesystem: &dyn FileSystem,
    canonical: &AbsolutePath,
    bridge: &AbsolutePath,
    scope: &Scope,
    mode: ClaudeInstructionMode,
) -> &'static str {
    let import_contents = match scope {
        Scope::Global => b"@~/AGENTS.md\n".as_slice(),
        Scope::Project(_) => b"@AGENTS.md\n".as_slice(),
    };
    instruction_bridge_status_with_target(filesystem, canonical, bridge, mode, import_contents)
}

fn instruction_bridge_status_with_target(
    filesystem: &dyn FileSystem,
    canonical: &AbsolutePath,
    bridge: &AbsolutePath,
    mode: ClaudeInstructionMode,
    import_contents: &[u8],
) -> &'static str {
    let representation = match mode {
        ClaudeInstructionMode::Symlink => match relative_symlink_target(bridge, canonical) {
            Ok(target) => InstructionBridgeRepresentation::Symlink(target),
            Err(_) => return "broken",
        },
        ClaudeInstructionMode::Import => {
            InstructionBridgeRepresentation::Import(import_contents.to_vec())
        }
    };
    let spec = InstructionBridgeSpec {
        canonical: canonical.clone(),
        bridge: bridge.clone(),
        mode: match mode {
            ClaudeInstructionMode::Symlink => CoreInstructionBridgeMode::Symlink,
            ClaudeInstructionMode::Import => CoreInstructionBridgeMode::Import,
        },
        representation,
    };
    let metadata = match filesystem.inspect(bridge) {
        Ok(metadata) => metadata,
        Err(_) => return "unreadable",
    };
    let observed = match metadata.kind() {
        FileKind::Missing => ObservedInstructionBridge::Missing,
        FileKind::Symlink => {
            let effective_target = metadata
                .link_target()
                .and_then(|target| resolve_symlink_target(bridge, target).ok());
            let target_kind = effective_target
                .as_ref()
                .and_then(|target| filesystem.inspect(target).ok())
                .map(|target| target.kind());
            ObservedInstructionBridge::Symlink {
                effective_target,
                target_exists: target_kind.is_some_and(|kind| kind != FileKind::Missing),
                target_is_regular: target_kind == Some(FileKind::RegularFile),
            }
        }
        FileKind::RegularFile => {
            let Some(contents) = filesystem.read_regular_no_follow(bridge).ok().flatten() else {
                return "unreadable";
            };
            ObservedInstructionBridge::RegularFile {
                fingerprint: fingerprint_contents(&contents),
            }
        }
        FileKind::Directory | FileKind::Other => ObservedInstructionBridge::Other,
    };
    match classify_bridge(&spec, &observed) {
        InstructionHealth::Missing => "missing",
        InstructionHealth::Managed => "managed",
        InstructionHealth::Divergent => "divergent",
        InstructionHealth::Broken => "broken",
        InstructionHealth::Duplicate => "duplicate",
        InstructionHealth::Unmanaged => "unmanaged",
    }
}

fn skill_destination(
    paths: &PlatformPaths,
    scope: &Scope,
    target: &HarnessId,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<(AbsolutePath, AbsolutePath)> {
    let root = match (scope, target.as_str()) {
        (Scope::Global, "codex") => {
            AbsolutePath::new(format!("{}/.agents", paths.home().as_str())).ok()?
        }
        (Scope::Global, "claude") => paths.claude_home().clone(),
        (Scope::Project(project), "codex") => {
            AbsolutePath::new(format!("{}/.agents", project.as_str())).ok()?
        }
        (Scope::Project(project), "claude") => {
            AbsolutePath::new(format!("{}/.claude", project.as_str())).ok()?
        }
        _ => return None,
    };
    let full = AbsolutePath::new(format!("{}/{}", root.as_str(), destination.as_str())).ok()?;
    Some((root, full))
}

fn canonical_skill_destination(
    paths: &PlatformPaths,
    scope: &Scope,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<(AbsolutePath, AbsolutePath)> {
    let root = match scope {
        Scope::Global => AbsolutePath::new(format!("{}/.agents", paths.home().as_str())).ok()?,
        Scope::Project(project) => {
            AbsolutePath::new(format!("{}/.agents", project.as_str())).ok()?
        }
    };
    let full = AbsolutePath::new(format!("{}/{}", root.as_str(), destination.as_str())).ok()?;
    Some((root, full))
}

fn skill_destinations(
    paths: &PlatformPaths,
    scope: &Scope,
    targets: &HarnessSet,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<Vec<SkillDestination>> {
    let mut destinations = Vec::new();
    let codex_selected = targets.iter().any(|target| target.as_str() == "codex");
    if !codex_selected {
        let target = targets.iter().next()?.clone();
        let (root, full_path) = canonical_skill_destination(paths, scope, destination)?;
        destinations.push(SkillDestination {
            target,
            canonical: true,
            root,
            full_path,
        });
    }
    for target in targets.iter() {
        let (root, full_path) = skill_destination(paths, scope, target, destination)?;
        destinations.push(SkillDestination {
            target: target.clone(),
            canonical: false,
            root,
            full_path,
        });
    }
    Some(destinations)
}

struct ResolvedGitSkill {
    root: AbsolutePath,
    commit: GitCommit,
}

struct SkillDestination {
    target: HarnessId,
    canonical: bool,
    root: AbsolutePath,
    full_path: AbsolutePath,
}

fn is_option_like_git_value(value: &str) -> bool {
    value.starts_with('-')
}

/// Resolve a Git source into skilltap's private managed cache using bounded,
/// direct Git invocations. The cache identity is derived from the locator, so
/// repeated installs fetch into the same checkout and can observe a changed
/// commit without scanning unrelated repositories.
fn resolve_git_skill_source(
    paths: &PlatformPaths,
    locator: &SourceLocator,
    requested_revision: Option<&skilltap_core::domain::RequestedRevision>,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Result<ResolvedGitSkill, ()> {
    // Keep this private boundary defensive even though callers validate the
    // typed values first. Git treats leading-dash positional values as
    // options unless an argument delimiter is present.
    if is_option_like_git_value(locator.as_str())
        || requested_revision
            .as_ref()
            .is_some_and(|revision| is_option_like_git_value(revision.as_str()))
    {
        return Err(());
    }
    let source_root = AbsolutePath::new(format!(
        "{}/managed/sources",
        paths.skilltap_config().as_str()
    ))
    .map_err(|_| ())?;
    SystemFileSystem
        .create_directory_all(&source_root)
        .map_err(|_| ())?;
    let hash = stable_hash(locator.as_str());
    let checkout =
        AbsolutePath::new(format!("{}/git-{hash:016x}", source_root.as_str())).map_err(|_| ())?;
    let git = NativeId::new("git").map_err(|_| ())?;
    let configured = ConfiguredBinary::path_lookup(git).map_err(|_| ())?;
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured,
            std::env::var_os("PATH"),
        ))
        .map_err(|_| ())?;
    let limits = ProcessLimits::new(120_000, 256 * 1024, 256 * 1024, 512 * 1024).map_err(|_| ())?;
    let filesystem = SystemFileSystem;
    let existing = filesystem.inspect(&checkout).map_err(|_| ())?;
    if existing.kind() == FileKind::Missing {
        let clone = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("clone"),
                OsString::from("--no-checkout"),
                OsString::from("--depth"),
                OsString::from("1"),
                OsString::from("--"),
                OsString::from(locator.as_str()),
                OsString::from(checkout.as_str()),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner.run(&clone).map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
    } else if existing.kind() != FileKind::Directory {
        return Err(());
    } else {
        let fetch = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("-C"),
                OsString::from(checkout.as_str()),
                OsString::from("fetch"),
                OsString::from("--prune"),
                OsString::from("origin"),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner.run(&fetch).map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
        if requested_revision.is_none() {
            let set_head = NativeProcessRequest::new(
                executable.clone(),
                [
                    OsString::from("-C"),
                    OsString::from(checkout.as_str()),
                    OsString::from("remote"),
                    OsString::from("set-head"),
                    OsString::from("origin"),
                    OsString::from("--auto"),
                ],
                BTreeMap::new(),
                None,
                limits,
            );
            let _ = SystemNativeProcessRunner.run(&set_head);
        }
    }
    if let Some(revision) = requested_revision {
        let fetch_revision = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("-C"),
                OsString::from(checkout.as_str()),
                OsString::from("fetch"),
                OsString::from("--depth"),
                OsString::from("1"),
                OsString::from("origin"),
                OsString::from("--"),
                OsString::from(revision.as_str()),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner
            .run(&fetch_revision)
            .map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
    }
    let revision = requested_revision
        .map(|_| "FETCH_HEAD")
        .unwrap_or("origin/HEAD");
    let verify = NativeProcessRequest::new(
        executable.clone(),
        [
            OsString::from("-C"),
            OsString::from(checkout.as_str()),
            OsString::from("rev-parse"),
            OsString::from("--verify"),
            OsString::from(format!("{revision}^{{commit}}")),
        ],
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&verify).map_err(|_| ())?;
    if !output.status().success() {
        return Err(());
    }
    let commit_text = std::str::from_utf8(output.stdout())
        .map_err(|_| ())?
        .trim()
        .to_owned();
    let commit = GitCommit::new(commit_text).map_err(|_| ())?;
    let checkout_root = checkout.clone();
    let checkout = NativeProcessRequest::new(
        executable,
        [
            OsString::from("-C"),
            OsString::from(checkout.as_str()),
            OsString::from("checkout"),
            OsString::from("--detach"),
            OsString::from("--force"),
            OsString::from(commit.as_str()),
        ],
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&checkout).map_err(|_| ())?;
    if !output.status().success() {
        return Err(());
    }
    let root = append_skill_subdirectory(checkout_root, subdirectory).ok_or(())?;
    Ok(ResolvedGitSkill { root, commit })
}

fn append_skill_subdirectory(
    root: AbsolutePath,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Option<AbsolutePath> {
    subdirectory
        .map(|path| AbsolutePath::new(format!("{}/{}", root.as_str(), path.as_str())).ok())
        .unwrap_or(Some(root))
}

fn skill_operation_id(target: &HarnessId, resource: &ResourceKey) -> OperationId {
    let label = format!("skill:{target}:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill:{target}:{hash:016x}")).expect("skill operation id is valid")
}

fn skill_canonical_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-canonical:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill-canonical:{hash:016x}"))
        .expect("canonical skill operation id is valid")
}

fn skill_remove_operation_id(target: &HarnessId, resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove:{target}:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill-remove:{target}:{hash:016x}"))
        .expect("skill removal operation id is valid")
}

fn skill_canonical_remove_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove-canonical:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill-remove-canonical:{hash:016x}"))
        .expect("canonical skill removal operation id is valid")
}

#[derive(Clone)]
struct NativeLifecycleSpec {
    operation_action: OperationAction,
    native_action: NativeLifecycleAction,
    resource_kind: ResourceKind,
    resource_prefix: &'static str,
    native_name: NativeId,
    source: Option<Source>,
}

fn native_resource_kind(kind: NativeLifecycleKind) -> ResourceKind {
    match kind {
        NativeLifecycleKind::MarketplaceAdd
        | NativeLifecycleKind::MarketplaceRemove
        | NativeLifecycleKind::MarketplaceUpdate => ResourceKind::Marketplace,
        NativeLifecycleKind::PluginInstall
        | NativeLifecycleKind::PluginRemove
        | NativeLifecycleKind::PluginUpdate => ResourceKind::Plugin,
    }
}

fn native_resource_prefix(kind: NativeLifecycleKind) -> &'static str {
    match native_resource_kind(kind) {
        ResourceKind::Marketplace => "marketplace:",
        ResourceKind::Plugin => "plugin:",
        _ => unreachable!("native lifecycle resources have a marketplace or plugin kind"),
    }
}

impl NativeLifecycleSpec {
    fn parse(
        kind: NativeLifecycleKind,
        source_value: Option<&str>,
        name_value: Option<&str>,
    ) -> Result<Self, ErrorDetail> {
        match kind {
            NativeLifecycleKind::MarketplaceAdd => {
                let source_value = source_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "source_required",
                        "Marketplace registration requires an explicit source.",
                    )
                })?;
                let locator = SourceLocator::new(source_value).map_err(|_| {
                    ErrorDetail::new("invalid_source", "The marketplace source is invalid.")
                })?;
                let native_name = match name_value {
                    Some(name) => NativeId::new(name).map_err(|_| {
                        ErrorDetail::new("invalid_name", "The marketplace name is invalid.")
                    })?,
                    None => derive_marketplace_name(locator.as_str()).ok_or_else(|| {
                        ErrorDetail::new(
                            "name_required",
                            "The marketplace name could not be derived; provide --name.",
                        )
                    })?,
                };
                let source_kind = if Path::new(locator.as_str()).is_absolute() {
                    SourceKind::Local
                } else {
                    SourceKind::Git
                };
                let source = Source::new(source_kind, locator, None).map_err(|_| {
                    ErrorDetail::new("invalid_source", "The marketplace source is invalid.")
                })?;
                Ok(Self {
                    operation_action: OperationAction::MarketplaceRegister,
                    native_action: NativeLifecycleAction::MarketplaceAdd,
                    resource_kind: ResourceKind::Marketplace,
                    resource_prefix: "marketplace",
                    native_name,
                    source: Some(source),
                })
            }
            NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::MarketplaceUpdate => {
                let native_name = name_value
                    .ok_or_else(|| {
                        ErrorDetail::new(
                            "name_required",
                            "The marketplace name is required for this lifecycle operation.",
                        )
                    })
                    .and_then(|name| {
                        NativeId::new(name).map_err(|_| {
                            ErrorDetail::new("invalid_name", "The marketplace name is invalid.")
                        })
                    })?;
                Ok(Self {
                    operation_action: if kind == NativeLifecycleKind::MarketplaceRemove {
                        OperationAction::MarketplaceRemove
                    } else {
                        OperationAction::MarketplaceUpdate
                    },
                    native_action: if kind == NativeLifecycleKind::MarketplaceRemove {
                        NativeLifecycleAction::MarketplaceRemove
                    } else {
                        NativeLifecycleAction::MarketplaceUpdate
                    },
                    resource_kind: ResourceKind::Marketplace,
                    resource_prefix: "marketplace",
                    native_name,
                    source: None,
                })
            }
            NativeLifecycleKind::PluginInstall => {
                let selector = source_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "plugin_required",
                        "Plugin installation requires an exact plugin@marketplace selector.",
                    )
                })?;
                skilltap_core::marketplace::PluginSelector::parse(selector).map_err(|_| {
                    ErrorDetail::new(
                        "invalid_plugin_selector",
                        "The plugin selector must be an exact plugin@marketplace value.",
                    )
                })?;
                let native_name = NativeId::new(selector).map_err(|_| {
                    ErrorDetail::new("invalid_plugin_selector", "The plugin selector is invalid.")
                })?;
                Ok(Self {
                    operation_action: OperationAction::PluginInstall,
                    native_action: NativeLifecycleAction::PluginInstall,
                    resource_kind: ResourceKind::Plugin,
                    resource_prefix: "plugin",
                    native_name,
                    source: None,
                })
            }
            NativeLifecycleKind::PluginRemove | NativeLifecycleKind::PluginUpdate => {
                let selector = name_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "plugin_required",
                        "The plugin selector is required for this lifecycle operation.",
                    )
                })?;
                skilltap_core::marketplace::PluginSelector::parse(selector).map_err(|_| {
                    ErrorDetail::new(
                        "invalid_plugin_selector",
                        "The plugin selector must be an exact plugin@marketplace value.",
                    )
                })?;
                let native_name = NativeId::new(selector).map_err(|_| {
                    ErrorDetail::new("invalid_plugin_selector", "The plugin selector is invalid.")
                })?;
                Ok(Self {
                    operation_action: if kind == NativeLifecycleKind::PluginRemove {
                        OperationAction::PluginRemove
                    } else {
                        OperationAction::PluginUpdate
                    },
                    native_action: if kind == NativeLifecycleKind::PluginRemove {
                        NativeLifecycleAction::PluginRemove
                    } else {
                        NativeLifecycleAction::PluginUpdate
                    },
                    resource_kind: ResourceKind::Plugin,
                    resource_prefix: "plugin",
                    native_name,
                    source: None,
                })
            }
        }
    }

    fn retains_desired(&self) -> bool {
        !matches!(
            self.operation_action,
            OperationAction::MarketplaceRemove | OperationAction::PluginRemove
        )
    }

    fn is_update(&self) -> bool {
        matches!(
            self.operation_action,
            OperationAction::MarketplaceUpdate | OperationAction::PluginUpdate
        )
    }

    fn resource_key(&self, scope: &Scope) -> Result<ResourceKey, ErrorDetail> {
        ResourceId::new(format!(
            "{}:{}",
            self.resource_prefix,
            self.native_name.as_str()
        ))
        .map(|id| ResourceKey::new(id, scope.clone()))
        .map_err(|_| {
            ErrorDetail::new(
                "resource_id_invalid",
                "The requested native resource identifier is invalid.",
            )
        })
    }

    fn desired_resource(
        &self,
        scope: &Scope,
        targets: &HarnessSet,
    ) -> Result<DesiredResource, ErrorDetail> {
        let key = ResourceKey::new(
            ResourceId::new(format!(
                "{}:{}",
                self.resource_prefix,
                self.native_name.as_str()
            ))
            .map_err(|_| {
                ErrorDetail::new(
                    "resource_id_invalid",
                    "The requested native resource identifier is invalid.",
                )
            })?,
            scope.clone(),
        );
        DesiredResource::new(
            key,
            self.resource_kind,
            targets.clone(),
            DesiredOrigin::Direct,
            self.source.clone(),
            UpdateIntent::Track,
            ComponentGraph::new([]).expect("empty component graph is valid"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .map_err(|_| {
            ErrorDetail::new(
                "resource_contract_invalid",
                "The requested native resource could not be represented safely.",
            )
        })
    }

    fn native_request(&self, harness: HarnessKind, scope: Scope) -> NativeLifecycleRequest {
        NativeLifecycleRequest {
            harness,
            action: self.native_action,
            scope,
            name: self.native_name.clone(),
            source: self.source.as_ref().map(|source| source.locator().clone()),
        }
    }

    const fn operation_action(&self) -> OperationAction {
        self.operation_action
    }
}

fn derive_marketplace_name(locator: &str) -> Option<NativeId> {
    let trimmed = locator.trim_end_matches('/');
    let segment = trimmed
        .rsplit('/')
        .next()?
        .strip_suffix(".git")
        .unwrap_or(trimmed.rsplit('/').next()?);
    NativeId::new(segment).ok()
}

fn derive_skill_name(
    locator: &SourceLocator,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Option<NativeId> {
    let segment = subdirectory
        .and_then(|path| path.as_str().rsplit('/').next())
        .or_else(|| locator.as_str().trim_end_matches('/').rsplit('/').next())?;
    let segment = segment.strip_suffix(".git").unwrap_or(segment);
    NativeId::new(segment).ok()
}

fn configured_native_profile(
    config: &ConfigDocument,
    target: &HarnessId,
    scope: &Scope,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    search_path: Option<std::ffi::OsString>,
    capability_name: &str,
) -> Result<Option<(HarnessKind, ConfiguredBinary, NativeId, CapabilitySupport)>, DetectionError> {
    let (harness, binary) = match target.as_str() {
        "codex" => (HarnessKind::Codex, config.harnesses().codex.binary.as_str()),
        "claude" => (
            HarnessKind::Claude,
            config.harnesses().claude.binary.as_str(),
        ),
        _ => return Ok(None),
    };
    let Some(configured) = configured_binary(binary).ok() else {
        return Ok(None);
    };
    let Some(executable) = NativeId::new(binary).ok() else {
        return Ok(None);
    };
    let Some(environment) = PlatformPaths::resolve(&ProcessEnvironment)
        .ok()
        .and_then(|paths| paths.native_process_environment(search_path.clone()).ok())
    else {
        return Ok(None);
    };
    let installation = detect_configured_installation(
        harness,
        configured.clone(),
        search_path,
        &environment,
        process_limits,
        json_limits,
    )?;
    let HarnessReachability::Reachable { native_version, .. } = installation.reachability() else {
        return Ok(None);
    };
    let profile = select_profile(harness, native_version);
    let Some(capability_id) = CapabilityId::new(capability_name).ok() else {
        return Ok(None);
    };
    let capability = profile
        .mutation_capabilities()
        .and_then(|capabilities| capabilities.for_scope(scope).support(&capability_id))
        .unwrap_or(CapabilitySupport::Unverified);
    Ok(Some((harness, configured, executable, capability)))
}

fn command_arguments(arguments: Vec<std::ffi::OsString>) -> Result<Vec<CommandArgument>, ()> {
    arguments
        .into_iter()
        .map(|argument| {
            let value = argument.into_string().map_err(|_| ())?;
            Ok(CommandArgument::literal(
                NativeId::new(value).map_err(|_| ())?,
            ))
        })
        .collect()
}

fn lifecycle_operation_id(
    kind: NativeLifecycleKind,
    target: &HarnessId,
    scope: &Scope,
    resource: &ResourceKey,
) -> OperationId {
    let label = format!(
        "{kind:?}:{target}:{}:{}",
        scope_label(scope),
        resource.id().as_str()
    );
    let hash = stable_hash(&label);
    OperationId::new(format!("lifecycle:{}:{hash:016x}", target.as_str()))
        .expect("lifecycle operation id is valid")
}

struct PlannedManagedProjectLifecycle {
    operation: skilltap_core::domain::Operation,
    entry: ManagedProjectLifecycleEntry,
    seed: Option<ResourceState>,
}

fn plan_managed_codex_project_lifecycle(
    kind: NativeLifecycleKind,
    request: &NativeLifecycleSpec,
    resource: &DesiredResource,
    project: &AbsolutePath,
    documents: &StatusDocuments,
    timestamp: Timestamp,
    json_limits: JsonLimits,
) -> Result<PlannedManagedProjectLifecycle, ErrorDetail> {
    let codex = HarnessId::new("codex").expect("static harness id is valid");
    let catalog_path = AbsolutePath::new(format!(
        "{}/.agents/plugins/marketplace.json",
        project.as_str()
    ))
    .map_err(|_| {
        managed_project_error(
            "managed_project_path_invalid",
            "The managed Codex project catalog path is invalid.",
        )
    })?;
    let filesystem = SystemFileSystem;
    let current_catalog = filesystem
        .read_regular_no_follow(&catalog_path)
        .map_err(|_| {
            managed_project_error(
                "managed_project_catalog_unreadable",
                "The Codex project catalog could not be read safely.",
            )
        })?;
    let existing_state = documents
        .state
        .as_ref()
        .and_then(|state| state.resources().get(resource.key()));

    let (desired_catalog, plugin, fingerprint, source) = match resource.kind() {
        ResourceKind::Marketplace => {
            let source = resource
                .source()
                .or_else(|| {
                    existing_state
                        .and_then(|state| state.target(&codex))
                        .and_then(TargetResourceState::source)
                })
                .cloned()
                .ok_or_else(|| {
                    managed_project_error(
                        "managed_project_source_missing",
                        "The managed project marketplace has no explicit source.",
                    )
                })?;
            let source_catalog = read_local_codex_catalog(&filesystem, &source, json_limits)?;
            let desired = if kind == NativeLifecycleKind::MarketplaceRemove {
                None
            } else {
                Some(source_catalog.into_bytes().map_err(|_| {
                    managed_project_error(
                        "managed_project_catalog_invalid",
                        "The selected Codex marketplace is invalid.",
                    )
                })?)
            };
            let fingerprint = desired.as_deref().map(fingerprint_contents);
            (desired, None, fingerprint, Some(source))
        }
        ResourceKind::Plugin => {
            let selector =
                skilltap_core::marketplace::PluginSelector::parse(request.native_name.as_str())
                    .map_err(|_| {
                        managed_project_error(
                            "invalid_plugin_selector",
                            "The managed project plugin selector is invalid.",
                        )
                    })?;
            let marketplace_key = ResourceKey::new(
                ResourceId::new(format!("marketplace:{}", selector.marketplace().as_str()))
                    .map_err(|_| {
                        managed_project_error(
                            "managed_project_marketplace_invalid",
                            "The selected marketplace identifier is invalid.",
                        )
                    })?,
                Scope::Project(project.clone()),
            );
            let marketplace_source = documents
                .inventory
                .as_ref()
                .and_then(|inventory| inventory.resources().get(&marketplace_key))
                .and_then(DesiredResource::source)
                .or_else(|| {
                    documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(&marketplace_key))
                        .and_then(|state| state.target(&codex))
                        .and_then(TargetResourceState::source)
                })
                .cloned()
                .ok_or_else(|| managed_project_error("managed_project_marketplace_missing", "Register the selected marketplace in this project before installing its plugin."))?;
            let (catalog_root, source_catalog) =
                read_local_codex_catalog_with_root(&filesystem, &marketplace_source, json_limits)?;
            let plugin_root = source_catalog
                .plugin_source(selector.plugin(), &catalog_root)
                .map_err(|_| {
                    managed_project_error(
                        "managed_project_plugin_source_invalid",
                        "The selected plugin source is not a contained local marketplace entry.",
                    )
                })?;
            let destination =
                skilltap_core::domain::RelativeArtifactPath::new(selector.plugin().as_str())
                    .map_err(|_| {
                        managed_project_error(
                            "managed_project_plugin_path_invalid",
                            "The selected plugin name cannot be used as a managed directory.",
                        )
                    })?;
            let plugin_parent = AbsolutePath::new(format!("{}/.agents/plugins", project.as_str()))
                .map_err(|_| {
                    managed_project_error(
                        "managed_project_plugin_path_invalid",
                        "The managed plugin parent path is invalid.",
                    )
                })?;
            let current_tree = filesystem
                .load_tree_no_follow(&plugin_parent, &destination)
                .ok()
                .and_then(|(identity, files)| {
                    ArtifactTree::new(
                        files
                            .into_iter()
                            .map(|(path, file)| (path.as_str().to_owned(), file)),
                    )
                    .ok()
                    .map(|tree| (identity, tree))
                });
            let desired_tree = if kind == NativeLifecycleKind::PluginRemove {
                None
            } else {
                Some(read_complete_codex_plugin(
                    &plugin_root,
                    &marketplace_source,
                    json_limits,
                )?)
            };
            let base_catalog = current_catalog
                .as_deref()
                .map(|bytes| ManagedCodexCatalog::parse(bytes, json_limits))
                .transpose()
                .map_err(|_| {
                    managed_project_error(
                        "managed_project_catalog_invalid",
                        "The existing Codex project catalog is invalid.",
                    )
                })?
                .unwrap_or(source_catalog);
            let desired_catalog = if kind == NativeLifecycleKind::PluginRemove {
                Some(
                    base_catalog
                        .without_plugin(selector.plugin())
                        .map_err(|_| {
                            managed_project_error(
                                "managed_project_plugin_missing",
                                "The managed plugin entry is missing from the project catalog.",
                            )
                        })?,
                )
            } else {
                Some(
                    base_catalog
                        .with_local_plugin(selector.plugin(), &destination)
                        .map_err(|_| {
                            managed_project_error(
                                "managed_project_plugin_missing",
                                "The selected plugin is missing from the marketplace catalog.",
                            )
                        })?,
                )
            };
            let fingerprint = desired_tree
                .as_ref()
                .zip(desired_catalog.as_deref())
                .map(|(tree, catalog)| managed_project_fingerprint(catalog, tree));
            let source = Source::new(
                SourceKind::Local,
                SourceLocator::new(plugin_root.as_str()).map_err(|_| {
                    managed_project_error(
                        "managed_project_plugin_source_invalid",
                        "The managed plugin source is invalid.",
                    )
                })?,
                None,
            )
            .map_err(|_| {
                managed_project_error(
                    "managed_project_plugin_source_invalid",
                    "The managed plugin source is invalid.",
                )
            })?;
            (
                desired_catalog,
                Some(ManagedProjectPluginWrite {
                    root: plugin_parent,
                    destination,
                    desired_tree,
                    expected_tree: current_tree.as_ref().map(|(_, tree)| tree.clone()),
                    expected_identity: current_tree.map(|(identity, _)| identity),
                }),
                fingerprint,
                Some(source),
            )
        }
        _ => {
            return Err(managed_project_error(
                "managed_project_resource_invalid",
                "Only Codex project marketplace and plugin resources can use this managed lifecycle.",
            ));
        }
    };

    validate_managed_project_ownership(
        kind,
        existing_state,
        current_catalog.as_deref(),
        plugin.as_ref(),
        fingerprint.as_ref(),
    )?;
    let operation_id = lifecycle_operation_id(
        kind,
        &codex,
        &Scope::Project(project.clone()),
        resource.key(),
    );
    let mut surfaces = vec![catalog_path.clone()];
    if let Some(plugin) = &plugin {
        surfaces.push(
            AbsolutePath::new(format!(
                "{}/{}",
                plugin.root.as_str(),
                plugin.destination.as_str()
            ))
            .map_err(|_| {
                managed_project_error(
                    "managed_project_plugin_path_invalid",
                    "The managed plugin destination is invalid.",
                )
            })?,
        );
    }
    let operation = skilltap_core::lifecycle_operation::managed_materialization_operation(
        operation_id,
        codex.clone(),
        resource.key().clone(),
        request.operation_action(),
        surfaces,
    )
    .map_err(|_| {
        managed_project_error(
            "operation_contract_invalid",
            "The managed Codex project operation could not be represented safely.",
        )
    })?;
    let seed = if matches!(
        kind,
        NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::PluginRemove
    ) {
        None
    } else {
        Some(
            TargetResourceState::new(
                codex,
                Some(request.native_name.clone()),
                Provenance::Materialized,
                Ownership::Skilltap,
                source,
                None,
                fingerprint,
                None,
                None,
                timestamp,
                None,
            )
            .and_then(|target| ResourceState::new(resource.key().clone(), [target]))
            .map_err(|_| {
                managed_project_error(
                    "state_seed_invalid",
                    "The managed Codex project state evidence is invalid.",
                )
            })?,
        )
    };
    Ok(PlannedManagedProjectLifecycle {
        operation,
        entry: ManagedProjectLifecycleEntry {
            catalog_path,
            expected_catalog: current_catalog,
            desired_catalog,
            plugin,
        },
        seed,
    })
}

fn read_local_codex_catalog(
    filesystem: &dyn FileSystem,
    source: &Source,
    limits: JsonLimits,
) -> Result<ManagedCodexCatalog, ErrorDetail> {
    read_local_codex_catalog_with_root(filesystem, source, limits).map(|(_, catalog)| catalog)
}

fn read_local_codex_catalog_with_root(
    filesystem: &dyn FileSystem,
    source: &Source,
    limits: JsonLimits,
) -> Result<(AbsolutePath, ManagedCodexCatalog), ErrorDetail> {
    if source.kind() != SourceKind::Local {
        return Err(managed_project_error(
            "managed_project_source_requires_checkout",
            "Managed Codex project lifecycle requires a bounded local source checkout.",
        ));
    }
    let root = AbsolutePath::new(source.locator().as_str()).map_err(|_| {
        managed_project_error(
            "managed_project_source_invalid",
            "The managed project source path is invalid.",
        )
    })?;
    for relative in [
        ".agents/plugins/marketplace.json",
        ".claude-plugin/marketplace.json",
    ] {
        let path = AbsolutePath::new(format!("{}/{}", root.as_str(), relative)).map_err(|_| {
            managed_project_error(
                "managed_project_source_invalid",
                "The marketplace document path is invalid.",
            )
        })?;
        if let Some(bytes) = filesystem.read_regular_no_follow(&path).map_err(|_| {
            managed_project_error(
                "managed_project_source_unreadable",
                "The selected marketplace document could not be read safely.",
            )
        })? {
            let catalog = ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                managed_project_error(
                    "managed_project_catalog_invalid",
                    "The selected marketplace document is invalid.",
                )
            })?;
            return Ok((root, catalog));
        }
    }
    Err(managed_project_error(
        "managed_project_catalog_missing",
        "The selected source has no Codex-compatible marketplace document.",
    ))
}

fn read_complete_codex_plugin(
    root: &AbsolutePath,
    source: &Source,
    json_limits: JsonLimits,
) -> Result<ArtifactTree, ErrorDetail> {
    let tree_limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded plugin tree limits are valid");
    use skilltap_core::plugin_graph::PluginGraphReader;
    CodexPluginGraphReader::new(root.clone(), tree_limits, json_limits)
        .read(source)
        .map_err(|_| managed_project_error("managed_project_plugin_invalid", "The selected plugin does not contain a valid Codex manifest and complete component graph."))?;
    let snapshot = SystemExternalTreeObserver
        .observe(&ExternalTreeRequest::new(root.clone(), tree_limits))
        .map_err(|_| {
            managed_project_error(
                "managed_project_plugin_unreadable",
                "The selected plugin tree could not be read safely.",
            )
        })?;
    let mut files = Vec::new();
    for entry in snapshot.entries() {
        match entry.kind() {
            skilltap_core::runtime::ExternalTreeEntryKind::Directory => {}
            skilltap_core::runtime::ExternalTreeEntryKind::File => files.push((
                entry.path().as_str().to_owned(),
                ArtifactFile::new(
                    entry.file_bytes().unwrap_or_default().to_vec(),
                    entry.file_executable().unwrap_or(false),
                ),
            )),
            skilltap_core::runtime::ExternalTreeEntryKind::Symlink => {
                return Err(managed_project_error(
                    "managed_project_plugin_symlink",
                    "Managed project plugins cannot contain symlinks.",
                ));
            }
        }
    }
    ArtifactTree::new(files).map_err(|_| {
        managed_project_error(
            "managed_project_plugin_invalid",
            "The selected plugin tree is invalid.",
        )
    })
}

fn managed_project_fingerprint(catalog: &[u8], tree: &ArtifactTree) -> Fingerprint {
    let mut digest = Sha256::new();
    digest.update((catalog.len() as u64).to_be_bytes());
    digest.update(catalog);
    for (path, file) in tree.files() {
        digest.update((path.as_str().len() as u64).to_be_bytes());
        digest.update(path.as_str().as_bytes());
        digest.update([u8::from(file.is_executable())]);
        digest.update((file.contents().len() as u64).to_be_bytes());
        digest.update(file.contents());
    }
    let hex = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Fingerprint::new(FingerprintAlgorithm::Sha256, hex).expect("SHA-256 fingerprint is valid")
}

fn validate_managed_project_ownership(
    kind: NativeLifecycleKind,
    state: Option<&ResourceState>,
    current_catalog: Option<&[u8]>,
    plugin: Option<&ManagedProjectPluginWrite>,
    desired_fingerprint: Option<&Fingerprint>,
) -> Result<(), ErrorDetail> {
    let current_fingerprint = match plugin {
        Some(plugin) => plugin
            .expected_tree
            .as_ref()
            .zip(current_catalog)
            .map(|(tree, catalog)| managed_project_fingerprint(catalog, tree)),
        None => current_catalog.map(fingerprint_contents),
    };
    if let Some(current_fingerprint) = current_fingerprint.as_ref() {
        let state = state.ok_or_else(|| {
            managed_project_error(
                "managed_project_unowned",
                "The existing managed destination has no skilltap ownership record.",
            )
        })?;
        let codex = HarnessId::new("codex").expect("static harness id is valid");
        let state = state.target(&codex).ok_or_else(|| {
            managed_project_error(
                "managed_project_unowned",
                "The existing managed destination has no Codex ownership binding.",
            )
        })?;
        if state.ownership() != Ownership::Skilltap
            || state.provenance() != Provenance::Materialized
        {
            return Err(managed_project_error(
                "managed_project_unowned",
                "The existing managed destination is not owned by skilltap.",
            ));
        }
        if state.fingerprint() != Some(current_fingerprint) {
            return Err(managed_project_error(
                "managed_project_drifted",
                "The managed project destination drifted; no files were changed.",
            ));
        }
        if matches!(
            kind,
            NativeLifecycleKind::MarketplaceAdd | NativeLifecycleKind::PluginInstall
        ) && desired_fingerprint != Some(current_fingerprint)
        {
            return Err(managed_project_error(
                "managed_project_update_required",
                "The managed project source changed; use the explicit update command.",
            ));
        }
    }
    Ok(())
}

fn managed_project_error(code: &'static str, summary: &'static str) -> ErrorDetail {
    ErrorDetail::new(code, summary)
}

fn previously_applied(
    state: Option<&StateDocument>,
    resource: &ResourceKey,
    target: &HarnessId,
    operation: &OperationId,
) -> bool {
    state
        .and_then(|state| state.resources().get(resource))
        .and_then(|state| state.target(target))
        .and_then(|state| state.last_apply())
        .and_then(|apply| apply.operations().get(operation))
        .is_some_and(|result| {
            matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            )
        })
}

fn lifecycle_preview_presence(
    documents: &StatusDocuments,
    kind: ResourceKind,
    harness: &HarnessId,
    scope: &Scope,
    name: &str,
) -> NativeResourcePresence {
    let action = match kind {
        ResourceKind::Marketplace => NativeLifecycleAction::MarketplaceAdd,
        ResourceKind::Plugin => NativeLifecycleAction::PluginInstall,
        ResourceKind::StandaloneSkill
        | ResourceKind::InstructionLocation
        | ResourceKind::Harness => return NativeResourcePresence::Unknown,
    };
    let harness_kind = match harness.as_str() {
        "codex" => HarnessKind::Codex,
        "claude" => HarnessKind::Claude,
        _ => return NativeResourcePresence::Unknown,
    };
    let configured = match harness_kind {
        HarnessKind::Codex => configured_binary(documents.config.harnesses().codex.binary.as_str()),
        HarnessKind::Claude => {
            configured_binary(documents.config.harnesses().claude.binary.as_str())
        }
    };
    let Ok(configured) = configured else {
        return NativeResourcePresence::Unknown;
    };
    let Ok(name) = NativeId::new(name) else {
        return NativeResourcePresence::Unknown;
    };
    let request = NativeLifecycleRequest {
        harness: harness_kind,
        action,
        scope: scope.clone(),
        name,
        source: None,
    };
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded lifecycle process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded lifecycle JSON limits are valid");
    let Ok(paths) = PlatformPaths::resolve(&ProcessEnvironment) else {
        return NativeResourcePresence::Unknown;
    };
    let search_path = std::env::var_os("PATH");
    let Ok(environment) = paths.native_process_environment(search_path.clone()) else {
        return NativeResourcePresence::Unknown;
    };
    observe_native_resource(
        configured,
        search_path,
        &environment,
        &request,
        process_limits,
        json_limits,
    )
    .unwrap_or(NativeResourcePresence::Unknown)
}

fn lifecycle_presence_label(presence: NativeResourcePresence) -> &'static str {
    match presence {
        NativeResourcePresence::Present => "present",
        NativeResourcePresence::Missing => "missing",
        NativeResourcePresence::Unknown => "unknown",
    }
}

fn lifecycle_recorded_state(
    documents: &StatusDocuments,
    kind: ResourceKind,
    scope: &Scope,
    name: &str,
) -> bool {
    let prefix = match kind {
        ResourceKind::Marketplace => "marketplace:",
        ResourceKind::Plugin => "plugin:",
        ResourceKind::StandaloneSkill
        | ResourceKind::InstructionLocation
        | ResourceKind::Harness => return false,
    };
    let Ok(id) = ResourceId::new(format!("{prefix}{name}")) else {
        return false;
    };
    let key = ResourceKey::new(id, scope.clone());
    documents
        .state
        .as_ref()
        .is_some_and(|state| state.resources().contains_key(&key))
}

fn operation_result_status(outcome: &OperationOutcome) -> &'static str {
    match outcome {
        OperationOutcome::Applied => "applied",
        OperationOutcome::NoChange => "no_change",
        OperationOutcome::Failed { .. } => "failed",
        OperationOutcome::Blocked { .. } => "blocked",
        OperationOutcome::SkippedDependency { .. } => "skipped_dependency",
        OperationOutcome::Pending => "pending",
    }
}

fn native_execution_error(error: &ExecutionError) -> ErrorDetail {
    let (code, summary) = match error {
        ExecutionError::Lock(_) => (
            "configuration_locked",
            "Another skilltap mutation holds the configuration lock.",
        ),
        ExecutionError::Release(_) => (
            "configuration_lock_release_failed",
            "The configuration lock could not be released safely.",
        ),
        ExecutionError::Revalidation { .. } => (
            "stale_native_evidence",
            "Native lifecycle evidence changed before mutation.",
        ),
        ExecutionError::Apply { .. } => (
            "native_command_failed",
            "The native lifecycle command failed.",
        ),
        ExecutionError::Journal { after_apply, .. } if *after_apply => (
            "state_journal_failed_after_apply",
            "Native work may have completed but state journaling failed; re-observe before retrying.",
        ),
        ExecutionError::Journal { .. } | ExecutionError::JournalBoundary { .. } => (
            "state_journal_failed",
            "The native operation result could not be recorded safely.",
        ),
        ExecutionError::InvalidOutcome { .. } => (
            "native_outcome_invalid",
            "The native lifecycle adapter returned an invalid operation outcome.",
        ),
        ExecutionError::Graph(_) | ExecutionError::Contract(_) => (
            "operation_plan_invalid",
            "The native lifecycle operation plan was invalid.",
        ),
    };
    ErrorDetail::new(code, summary)
}

fn seed_state_if_missing(
    repository: &dyn StateRepository,
    seeds: &BTreeMap<ResourceKey, ResourceState>,
) -> Result<(), ()> {
    if seeds.is_empty() {
        return Ok(());
    }
    let mut document = match repository.load().map_err(|_| ())? {
        DocumentState::Present(document) => document,
        DocumentState::Missing => StateDocument::new(
            skilltap_core::storage::STATE_SCHEMA_VERSION,
            [],
            [],
            None,
            None,
            None,
        )
        .map_err(|_| ())?,
    };
    for (key, seed) in seeds {
        if !document.resources().contains_key(key) {
            document = document.with_resource_state(seed.clone()).map_err(|_| ())?;
        }
    }
    repository.replace(&document).map_err(|_| ())
}

fn refresh_state_seeds(
    repository: &dyn StateRepository,
    seeds: &BTreeMap<ResourceKey, ResourceState>,
) -> Result<(), ()> {
    if seeds.is_empty() {
        return Ok(());
    }
    let mut document = match repository.load().map_err(|_| ())? {
        DocumentState::Present(document) => document,
        DocumentState::Missing => StateDocument::new(
            skilltap_core::storage::STATE_SCHEMA_VERSION,
            [],
            [],
            None,
            None,
            None,
        )
        .map_err(|_| ())?,
    };
    for (key, seed) in seeds {
        if let Some(existing) = document.resources().get(key) {
            if state_seed_matches(existing, seed) {
                continue;
            }
            document = document
                .refresh_resource_state(seed.clone())
                .map_err(|_| ())?;
        } else {
            document = document.with_resource_state(seed.clone()).map_err(|_| ())?;
        }
    }
    repository.replace(&document).map_err(|_| ())
}

fn state_seed_matches(existing: &ResourceState, seed: &ResourceState) -> bool {
    seed.targets().iter().all(|(harness, seed_target)| {
        existing.target(harness).is_some_and(|existing_target| {
            existing_target.native_id() == seed_target.native_id()
                && existing_target.provenance() == seed_target.provenance()
                && existing_target.ownership() == seed_target.ownership()
                && existing_target.source() == seed_target.source()
                && existing_target.managed_artifact() == seed_target.managed_artifact()
                && existing_target.fingerprint() == seed_target.fingerprint()
                && existing_target.installed_revision() == seed_target.installed_revision()
                && existing_target.available_revision() == seed_target.available_revision()
        })
    })
}

fn project_inventory_targets(
    inventory: &InventoryDocument,
    keys: &BTreeSet<ResourceKey>,
    selected: &HarnessSet,
) -> Result<InventoryDocument, ()> {
    let mut next = inventory.clone();
    for key in keys {
        let Some(existing) = next.resources().get(key).cloned() else {
            continue;
        };
        let remaining = existing
            .targets()
            .iter()
            .filter(|target| !selected.contains(target))
            .cloned()
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            next = next.without_resource(key).ok_or(())?;
        } else {
            let targets = HarnessSet::new(remaining).map_err(|_| ())?;
            let projected = existing.with_targets(targets).map_err(|_| ())?;
            next = next.replace_resource(projected).map_err(|_| ())?;
        }
    }
    Ok(next)
}

fn project_state_targets_after_remove(
    repository: &dyn StateRepository,
    keys: &BTreeSet<ResourceKey>,
    selected: &HarnessSet,
) -> Result<(), ()> {
    if keys.is_empty() {
        return Ok(());
    }
    let mut document = match repository.load().map_err(|_| ())? {
        DocumentState::Present(document) => document,
        DocumentState::Missing => return Ok(()),
    };
    let mut changed = false;
    for key in keys {
        let Some(existing) = document.resources().get(key).cloned() else {
            continue;
        };
        let projected = existing.without_targets(selected).map_err(|_| ())?;
        if projected.as_ref() == Some(&existing) {
            continue;
        }
        document = match projected {
            Some(projected) => document
                .without_resource(key)
                .and_then(|state| state.with_resource_state(projected))
                .map_err(|_| ())?,
            None => document.without_resource(key).map_err(|_| ())?,
        };
        changed = true;
    }
    if changed {
        repository.replace(&document).map_err(|_| ())?;
    }
    Ok(())
}

fn revision_label(revision: &skilltap_core::domain::ResolvedRevision) -> String {
    match revision {
        skilltap_core::domain::ResolvedRevision::GitCommit(commit) => {
            format!("git:{}", commit.as_str())
        }
        skilltap_core::domain::ResolvedRevision::Native(native) => {
            format!("native:{}", native.as_str())
        }
    }
}

fn git_revision_changed(
    old: Option<&skilltap_core::domain::ResolvedRevision>,
    new: Option<&GitCommit>,
) -> bool {
    match (old, new) {
        (Some(skilltap_core::domain::ResolvedRevision::GitCommit(old)), Some(new)) => old != new,
        (None, Some(_)) => true,
        _ => false,
    }
}

fn harnesses_label(harnesses: &HarnessSet) -> String {
    harnesses
        .iter()
        .map(HarnessId::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

fn configured_binary(binary: &str) -> Result<ConfiguredBinary, ()> {
    if Path::new(binary).is_absolute() {
        AbsolutePath::new(binary)
            .map(ConfiguredBinary::absolute)
            .map_err(|_| ())
    } else {
        let id = skilltap_core::domain::NativeId::new(binary).map_err(|_| ())?;
        ConfiguredBinary::path_lookup(id).map_err(|_| ())
    }
}

fn stable_hash(input: &str) -> u64 {
    input.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "global".to_owned(),
        Scope::Project(path) => path.as_str().to_owned(),
    }
}

fn document_result<T>(
    outcome: Outcome,
    name: &str,
    result: &Result<DocumentState<T>, StorageError>,
) -> Outcome {
    let status = match result {
        Ok(DocumentState::Missing) if name == "config" => "missing; using defaults",
        Ok(DocumentState::Missing) => "missing",
        Ok(DocumentState::Present(_)) => "valid",
        Err(_) => "invalid",
    };
    outcome.with_resource(OutputEntry::new(name, status))
}

fn storage_error(error: &StorageError) -> ErrorDetail {
    let (code, summary) = match error.failure() {
        StorageFailure::Runtime => (
            "owned_document_unreadable",
            "A skilltap-owned document could not be read safely.",
        ),
        StorageFailure::Malformed => (
            "owned_document_malformed",
            "A skilltap-owned document is malformed.",
        ),
        StorageFailure::Invalid => (
            "owned_document_invalid",
            "A skilltap-owned document failed validation.",
        ),
        StorageFailure::UnsupportedSchema { .. } => (
            "owned_document_schema_unsupported",
            "A skilltap-owned document uses an unsupported schema version.",
        ),
    };
    ErrorDetail::new(code, summary)
        .with_context("document", error.document().to_string())
        .with_context("action", error.action().to_string())
}

fn enabled_harnesses(config: &ConfigDocument) -> Vec<HarnessId> {
    [
        ("codex", config.harnesses().codex.enabled),
        ("claude", config.harnesses().claude.enabled),
    ]
    .into_iter()
    .filter(|(_, enabled)| *enabled)
    .map(|(name, _)| HarnessId::new(name).expect("known harness identifier"))
    .collect()
}

fn output_scope(requested: &ScopeArgument, resolved: &[Scope]) -> OutputScope {
    match requested {
        ScopeArgument::Global => OutputScope::Global,
        ScopeArgument::AllScopes => OutputScope::All,
        ScopeArgument::Project(_) => OutputScope::Project {
            path: match resolved {
                [Scope::Project(path)] => path,
                _ => unreachable!("a resolved project request contains one project scope"),
            }
            .as_str()
            .to_owned(),
        },
    }
}

fn absolute_project_argument(
    argument: &Path,
    working_directory: &dyn WorkingDirectory,
) -> Result<AbsolutePath, ()> {
    let path = if argument.is_absolute() {
        argument.to_path_buf()
    } else {
        PathBuf::from(
            working_directory
                .current_directory()
                .map_err(|_| ())?
                .as_str(),
        )
        .join(argument)
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(());
                }
            }
        }
    }
    let value = normalized.to_str().ok_or(())?;
    AbsolutePath::new(value).map_err(|_| ())
}

#[cfg(test)]
mod tests;
