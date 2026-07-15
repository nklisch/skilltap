use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use skilltap_core::{
    adoption::{
        AdoptionApplyError, AdoptionDecision, AdoptionObservationError, AdoptionSelection,
        apply_adoption, plan_adoption,
    },
    domain::{
        AbsolutePath, ArtifactFile, CapabilityId, CapabilityProfileSelection, CapabilityScope,
        CapabilitySupport, CommandArgument, CompatibilityClass, CompatibilityEvidence,
        ComponentGraph, ComponentId, ConfiguredBinary, ConsequenceCode, DesiredOrigin,
        DesiredResource, EvidenceCode, EvidenceDetail, ExecutableIdentity, Fingerprint, GitCommit,
        HarnessId, HarnessObservation, HarnessObservationOutcome, HarnessReachability, HarnessSet,
        MaterialConsequence, NativeId, NativeVersion, ObservationAdapterError, ObservationBatch,
        ObservationEvidence, ObservationFields, ObservationFinding, ObservationFindingCode,
        ObservationKey, ObservationLayer, ObservationRequest, ObservationSeverity,
        ObservationSubject, ObservationSummary, ObservationTarget, ObservedResource,
        OperationAction, OperationDependency, OperationId, OperationOutcome, OperationResult,
        OperationSelector, Ownership, Plan, ProfileAuthority, Provenance, ResourceHealth,
        ResourceId, ResourceKey, ResourceKind, Scope, Source, SourceKind, SourceLocator,
        UpdateIntent,
    },
    executor::{
        ExecutionAcknowledgments, ExecutionError, ExecutionJournal, ExecutionPort, execute_plan,
        execute_plan_with_acknowledgments,
    },
    foreground_update::{
        ForegroundUpdateRequest, plan_foreground_updates,
        select_foreground_updates_with_acknowledgment,
    },
    instructions::{
        InstructionBridgeMode as CoreInstructionBridgeMode, InstructionBridgeRepresentation,
        InstructionBridgeSpec, InstructionHealth, ObservedInstructionBridge, classify_bridge,
        fingerprint_contents, relative_symlink_target, resolve_symlink_target,
    },
    lifecycle_operation::{
        managed_materialization_operation, managed_partial_materialization_operation,
        native_operation,
    },
    lifecycle_representation::{
        LifecycleRepresentation, LifecycleRepresentationError, RepresentationCandidate,
        RepresentationEvidence, applied_lifecycle_representation, select_lifecycle_representation,
    },
    managed_projection::{ManagedFileWrite, ManagedPluginWrite, ResolvedSourceCheckout},
    materialization::MaterializationPlan,
    mutation_authority::{
        CapabilityRequirement, ManagedSurfaceKind, MutationAuthorityRequest, MutationAuthorization,
        MutationChannel, authorize_mutation,
    },
    runtime::{
        ConfinedEntryObservation, DirectoryTreeFileSystem, ExecutableResolutionRequest,
        ExecutableResolver, ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest,
        FileKind, FileSystem, JsonLimits, LinkIdentity, NativeProcessRequest, NativeProcessRunner,
        PlatformPaths, ProcessEnvironment, ProcessLimits, RelativeSymlinkTarget, ScopeRequest,
        ScopeResolver, SystemConfigurationLock, SystemExecutableResolver,
        SystemExternalTreeObserver, SystemFileSystem, SystemNativeProcessRunner, WorkingDirectory,
        resolve_targets,
    },
    skill::ValidatedSkillTree,
    skill_compatibility::{SkillCompatibility, SkillLoadability},
    storage::{
        ArtifactTree, ClaudeInstructionMode, ConfigDocument, ConfigRepository, DaemonOperationRef,
        DocumentState, InventoryDocument, InventoryRepository, ManagedArtifactRepository,
        ManagedProjection, PendingManagedAttempt, ResourceState, StateDocument, StateRepository,
        StorageError, StorageFailure, TargetResourceState, Timestamp,
    },
    updates::{
        ResolutionError, SourceRevisionResolver, UpdateCandidate, UpdateDecision,
        UpdateDecisionReason, UpdateResolutionRequest, UpdateSafety, candidate_for,
        classify_update_with_mode, resolve_candidate,
    },
};
use skilltap_harnesses::{
    DetectionError, GitSourceRevisionResolver, ManagedLifecycleKind, ManagedProjectionContext,
    ManagedProjectionInput, NativeDistributionContext, NativeLifecycleAction,
    NativeLifecycleBinding, NativeLifecycleDispatch, NativeLifecyclePort, NativeLifecycleRequest,
    NativeObservationFailure, NativeResourceObservation, ObservedNativeRevisionResolver,
    detect_configured_installation, native_arguments, normalize_observations,
    observe_native_resource,
};

pub(super) struct DetectionDiagnostic {
    pub(super) warning: Warning,
    pub(super) next_action: NextAction,
}

pub(super) fn detection_diagnostic(
    error: &DetectionError,
    harness: &str,
    configured_binary: &str,
) -> DetectionDiagnostic {
    use skilltap_core::runtime::ObservationRuntimeError;

    let version_command = format!("{} --version", shell_command_word(configured_binary));

    let (code, summary, action_code, action_summary, command) = match error {
        DetectionError::InvalidVersion => (
            "native_version_invalid",
            "The harness returned an invalid version response.",
            "inspect_harness_version",
            "Inspect the harness version response and configure a supported binary.",
            version_command.clone(),
        ),
        DetectionError::NonZeroExit => (
            "native_version_command_failed",
            "The harness version command returned a nonzero status.",
            "inspect_harness_version",
            "Run the harness version command directly and resolve its failure.",
            version_command.clone(),
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
            version_command,
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

fn shell_command_word(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, OutputScope, OutputValue, ResultClass, Warning,
    command::{
        AdoptArgs, OutputArgs, PlanArgs, ScopeArgs, ScopeArgument, ScopedOutputArgs,
        ScopedTargetArgs, StatusArgs, SyncArgs, TargetArgs,
    },
};

mod conditional_profile;
mod execution;
mod instructions;
mod lifecycle;
mod project_skills;
mod reconciliation;
mod status;

pub(super) use status::first_use_harness_report;
use status::{NativeObservation, StatusDocuments, StatusScope, StatusTargetError, StatusTargets};

use execution::{
    HybridLifecyclePort, InstructionEntry, InstructionPort, InstructionWrite,
    ManagedLifecycleEntry, ManagedLifecycleFileSystem, ManagedLifecycleFileWrite,
    ManagedLifecyclePluginWrite, ManagedLifecyclePort, ManagedSkillAction, ManagedSkillEntry,
    ManagedSkillPort, StateExecutionJournal,
};

pub(crate) struct StatusApplication<'a> {
    pub(crate) config: &'a dyn ConfigRepository,
    pub(crate) inventory: &'a dyn InventoryRepository,
    pub(crate) state: &'a dyn StateRepository,
    pub(crate) scopes: &'a ScopeResolver<'a>,
    pub(crate) working_directory: &'a dyn WorkingDirectory,
    pub(crate) native_observation: NativeObservationMode,
    pub(crate) registry: &'a skilltap_harnesses::TargetRegistry,
    #[cfg(test)]
    pub(crate) test_platform_paths: Option<PlatformPaths>,
    #[cfg(test)]
    pub(crate) test_managed_filesystem: Option<&'a dyn ManagedLifecycleFileSystem>,
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
        operations: impl IntoIterator<Item = DaemonOperationRef>,
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
        )
        .and_then(|record| record.with_operations(operations))
        {
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
                        NextAction::new("enable_harness", "Enable a registered harness.")
                            .with_command("skilltap harness enable <registered-harness>"),
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
    registry: &skilltap_harnesses::TargetRegistry,
    paths: &PlatformPaths,
    scope: &Scope,
    enabled: &[HarnessId],
) -> (AbsolutePath, Vec<(HarnessId, AbsolutePath)>) {
    let canonical = match scope {
        Scope::Global => paths.global_agents().clone(),
        Scope::Project(project) => AbsolutePath::new(format!("{}/AGENTS.md", project.as_str()))
            .expect("project canonical path is valid"),
    };
    let bridges = enabled
        .iter()
        .filter_map(|target| {
            let port = registry.adapter(target)?.instruction_bridge()?;
            let bridge = match scope {
                Scope::Global => port.global_bridge(paths),
                Scope::Project(project) => port.project_bridge(project),
            }?;
            Some((target.clone(), bridge))
        })
        .collect();
    (canonical, bridges)
}

fn alternate_instruction_bridges(
    registry: &skilltap_harnesses::TargetRegistry,
    project: &AbsolutePath,
    enabled: &[HarnessId],
) -> Vec<(HarnessId, AbsolutePath, AbsolutePath)> {
    enabled
        .iter()
        .filter_map(|target| {
            let port = registry.adapter(target)?.instruction_bridge()?;
            Some((target, port, port.project_bridge(project)?))
        })
        .flat_map(|(target, port, root)| {
            port.alternate_project_bridges(project)
                .into_iter()
                .map(move |alternate| (target.clone(), root.clone(), alternate))
        })
        .collect()
}

fn is_alternate_instruction_bridge(
    registry: &skilltap_harnesses::TargetRegistry,
    scope: &Scope,
    target: &HarnessId,
    bridge: &AbsolutePath,
) -> bool {
    let Scope::Project(project) = scope else {
        return false;
    };
    registry
        .adapter(target)
        .and_then(skilltap_harnesses::HarnessAdapter::instruction_bridge)
        .is_some_and(|port| {
            port.alternate_project_bridges(project)
                .iter()
                .any(|candidate| candidate == bridge)
        })
}

/// Resolve a materialized alternative project bridge using the adapter's
/// preservation policy. The inventory identity remains the ordinary project
/// bridge resource even when setup keeps an alternate native location.
fn preferred_instruction_bridge_path(
    registry: &skilltap_harnesses::TargetRegistry,
    filesystem: &dyn FileSystem,
    scope: &Scope,
    target: &HarnessId,
    root_bridge: AbsolutePath,
) -> AbsolutePath {
    let Scope::Project(project) = scope else {
        return root_bridge;
    };
    let root_missing = filesystem
        .inspect(&root_bridge)
        .map(|metadata| metadata.kind() == FileKind::Missing)
        .unwrap_or(false);
    if !root_missing {
        return root_bridge;
    }
    registry
        .adapter(target)
        .and_then(skilltap_harnesses::HarnessAdapter::instruction_bridge)
        .into_iter()
        .flat_map(|port| port.alternate_project_bridges(project))
        .find(|candidate| {
            filesystem
                .inspect(candidate)
                .map(|metadata| metadata.kind() != FileKind::Missing)
                .unwrap_or(false)
        })
        .unwrap_or(root_bridge)
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
    registry: &skilltap_harnesses::TargetRegistry,
    paths: &PlatformPaths,
    scope: &Scope,
    target: &HarnessId,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<(AbsolutePath, AbsolutePath)> {
    let projection = registry
        .adapter(target)?
        .skill_projection()?
        .destination(paths, scope)?;
    let root = Path::new(projection.as_str()).parent()?.to_str()?;
    let name = destination.as_str().strip_prefix("skills/")?;
    let root = AbsolutePath::new(root).ok()?;
    let full = AbsolutePath::new(format!("{}/{name}", projection.as_str())).ok()?;
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
    registry: &skilltap_harnesses::TargetRegistry,
    paths: &PlatformPaths,
    scope: &Scope,
    targets: &HarnessSet,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<Vec<SkillDestination>> {
    let native = targets
        .iter()
        .map(|target| {
            skill_destination(registry, paths, scope, target, destination).map(
                |(root, full_path)| SkillDestination {
                    target: target.clone(),
                    canonical: false,
                    root,
                    full_path,
                },
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let (canonical_root, canonical_path) = canonical_skill_destination(paths, scope, destination)?;
    let mut destinations = Vec::new();
    if !native
        .iter()
        .any(|destination| destination.full_path == canonical_path)
    {
        destinations.push(SkillDestination {
            target: targets.iter().next()?.clone(),
            canonical: true,
            root: canonical_root,
            full_path: canonical_path,
        });
    }
    destinations.extend(native);
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

pub(crate) struct NativeLifecycleValues<'a> {
    pub(crate) source: Option<&'a str>,
    pub(crate) name: Option<&'a str>,
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

    fn native_request(&self, scope: Scope) -> NativeLifecycleRequest {
        NativeLifecycleRequest {
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

struct NativeProfileRequest<'a> {
    scope: &'a Scope,
    environment: &'a BTreeMap<OsString, OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    search_path: Option<OsString>,
    capability_name: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfiguredAdapterProfile {
    target: HarnessId,
    scope: Scope,
    configured: ConfiguredBinary,
    executable: ExecutableIdentity,
    native_version: NativeVersion,
    profile: CapabilityProfileSelection,
    capability: CapabilitySupport,
    declaration_contract: Option<skilltap_core::mutation_authority::ManagedDeclarationContract>,
}

struct ConfiguredNativeProfile {
    target: HarnessId,
    lifecycle: &'static dyn skilltap_harnesses::NativeLifecycleVector,
    configured: ConfiguredBinary,
    executable: NativeId,
    capability: CapabilitySupport,
}

fn configured_adapter_profile(
    registry: &skilltap_harnesses::TargetRegistry,
    config: &ConfigDocument,
    target: &HarnessId,
    request: NativeProfileRequest<'_>,
) -> Result<Option<ConfiguredAdapterProfile>, DetectionError> {
    let Some(adapter) = registry.adapter(target) else {
        return Ok(None);
    };
    let Some(policy) = config.harnesses().get(target) else {
        return Ok(None);
    };
    let binary = policy.binary.as_str();
    let Some(configured) = configured_binary(binary).ok() else {
        return Ok(None);
    };
    let installation = detect_configured_installation(
        adapter,
        configured.clone(),
        request.search_path,
        request.environment,
        request.process_limits,
        request.json_limits,
    )?;
    let HarnessReachability::Reachable {
        executable,
        native_version,
    } = installation.reachability()
    else {
        return Ok(None);
    };
    let profile = if adapter.conditional_profile().is_some() {
        match PlatformPaths::resolve(&ProcessEnvironment)
            .ok()
            .and_then(|paths| {
                conditional_profile::resolve_conditional_profile(
                    registry,
                    config,
                    target,
                    request.scope,
                    &paths,
                    request.process_limits,
                    request.json_limits,
                    &SystemFileSystem,
                )
                .ok()
                .flatten()
            }) {
            Some(resolved) => resolved.observation.profile().clone(),
            None => adapter.select_profile(native_version),
        }
    } else {
        adapter.select_profile(native_version)
    };
    let Some(capability_id) = CapabilityId::new(request.capability_name).ok() else {
        return Ok(None);
    };
    let capability = profile
        .mutation_support(request.scope, &capability_id)
        .unwrap_or(CapabilitySupport::Unsupported);
    let declaration_contract = adapter
        .managed_declaration_contract(CapabilityScope::from(request.scope))
        .cloned();
    Ok(Some(ConfiguredAdapterProfile {
        target: target.clone(),
        scope: request.scope.clone(),
        configured,
        executable: executable.clone(),
        native_version: native_version.clone(),
        profile,
        capability,
        declaration_contract,
    }))
}

fn configured_native_profile(
    registry: &skilltap_harnesses::TargetRegistry,
    config: &ConfigDocument,
    target: &HarnessId,
    request: NativeProfileRequest<'_>,
) -> Result<Option<ConfiguredNativeProfile>, DetectionError> {
    let Some(runtime) = configured_adapter_profile(registry, config, target, request)? else {
        return Ok(None);
    };
    let Some(lifecycle) = registry
        .adapter(target)
        .and_then(|adapter| adapter.native_lifecycle())
    else {
        return Ok(None);
    };
    let Some(policy) = config.harnesses().get(target) else {
        return Ok(None);
    };
    let Some(executable) = NativeId::new(policy.binary.as_str()).ok() else {
        return Ok(None);
    };
    Ok(Some(ConfiguredNativeProfile {
        target: runtime.target,
        lifecycle,
        configured: runtime.configured,
        executable,
        capability: runtime.capability,
    }))
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

struct PlannedManagedLifecycle {
    operation: skilltap_core::domain::Operation,
    entry: ManagedLifecycleEntry,
    seed: Option<ResourceState>,
    materialization: MaterializationPlan,
}

struct ManagedPlanContext<'a> {
    scope: &'a Scope,
    documents: &'a StatusDocuments,
    paths: &'a PlatformPaths,
    timestamp: Timestamp,
    json_limits: JsonLimits,
    acknowledged: bool,
    filesystem: &'a dyn ManagedLifecycleFileSystem,
    /// A caller-resolved checkout shared with native distribution assessment.
    checkout: Option<&'a ResolvedSourceCheckout>,
}

fn plan_managed_lifecycle(
    registry: &skilltap_harnesses::TargetRegistry,
    target: &HarnessId,
    kind: NativeLifecycleKind,
    request: &NativeLifecycleSpec,
    resource: &DesiredResource,
    profile: ConfiguredAdapterProfile,
    context: ManagedPlanContext<'_>,
) -> Result<PlannedManagedLifecycle, ErrorDetail> {
    let adapter = registry.adapter(target).ok_or_else(|| {
        managed_project_error(
            "managed_project_target_unregistered",
            "The selected managed project target is not registered.",
        )
    })?;
    let port = adapter.managed_projection().ok_or_else(|| {
        managed_project_error(
            "managed_project_projection_unsupported",
            "The selected target does not provide managed project projection.",
        )
    })?;
    let ManagedPlanContext {
        scope,
        documents,
        paths,
        timestamp,
        json_limits,
        acknowledged: _,
        filesystem,
        checkout: provided_checkout,
    } = context;
    let existing_state = documents
        .state
        .as_ref()
        .and_then(|state| state.resources().get(resource.key()));
    let target_state = existing_state.and_then(|state| state.target(target));
    let operation_id = lifecycle_operation_id(kind, target, scope, resource.key());
    let prior_projections = target_state
        .map(|target| {
            target
                .pending_managed_attempt()
                .filter(|attempt| {
                    attempt.operation_id() == &operation_id
                        && target.last_apply().is_some_and(|apply| {
                            apply.operations().get(&operation_id).is_some_and(|result| {
                                matches!(result.outcome(), OperationOutcome::Pending)
                            })
                        })
                })
                .map(PendingManagedAttempt::managed_projections)
                .unwrap_or_else(|| target.managed_projections())
        })
        .unwrap_or_default();

    let removal = matches!(
        kind,
        NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::PluginRemove
    );
    let resolved_checkout: Option<ResolvedSourceCheckout>;
    let checkout = match resource.kind() {
        ResourceKind::Marketplace if removal => None,
        ResourceKind::Marketplace => {
            if let Some(checkout) = provided_checkout {
                Some(checkout)
            } else {
                let source = resource
                    .source()
                    .or_else(|| target_state.and_then(TargetResourceState::source))
                    .cloned()
                    .ok_or_else(|| {
                        managed_project_error(
                            "managed_project_source_missing",
                            "The managed project marketplace has no explicit source.",
                        )
                    })?;
                resolved_checkout = Some(resolve_managed_source_checkout(paths, source)?);
                resolved_checkout.as_ref()
            }
        }
        ResourceKind::Plugin if removal => {
            if target_state.is_none() {
                return Err(managed_project_error(
                    "managed_project_unowned",
                    "The plugin has no installed managed projection manifest.",
                ));
            }
            if prior_projections.is_empty() {
                return Err(managed_project_error(
                    "managed_project_projection_manifest_missing",
                    "This installation predates projection manifests; run plugin update while its source is available, then retry removal.",
                ));
            }
            None
        }
        ResourceKind::Plugin => {
            if let Some(checkout) = provided_checkout {
                Some(checkout)
            } else {
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
                    scope.clone(),
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
                            .and_then(|state| state.target(target))
                            .and_then(TargetResourceState::source)
                    })
                    .cloned()
                    .ok_or_else(|| {
                        managed_project_error(
                            "managed_project_marketplace_missing",
                            "Register the selected marketplace in this project before installing its plugin.",
                        )
                    })?;
                resolved_checkout =
                    Some(resolve_managed_source_checkout(paths, marketplace_source)?);
                resolved_checkout.as_ref()
            }
        }
        _ => {
            return Err(managed_project_error(
                "managed_project_resource_invalid",
                "Only marketplace and plugin resources can use managed projection.",
            ));
        }
    };
    let input = checkout.map_or(ManagedProjectionInput::Remove, |checkout| {
        ManagedProjectionInput::Apply { checkout }
    });
    let native_request = NativeLifecycleRequest {
        action: request.native_action,
        scope: scope.clone(),
        name: request.native_name.clone(),
        source: request
            .source
            .as_ref()
            .map(|source| source.locator().clone()),
    };
    let plan = port
        .plan(&ManagedProjectionContext {
            target,
            scope,
            paths,
            resource_key: resource.key(),
            resource_kind: resource.kind(),
            request: &native_request,
            kind: managed_lifecycle_kind(kind),
            input,
            prior: prior_projections,
            // The adapter reports optional loss; the planner owns whether the
            // resulting exact partial operation is accepted.
            acknowledged: true,
            filesystem,
            json_limits,
        })
        .map_err(|error| managed_project_error(error.code(), error.summary()))?;
    let current_fingerprint = plan.current_fingerprint;
    let fingerprint = plan.desired_fingerprint;
    let mut managed_projections = plan.manifest;
    managed_projections.sort();
    managed_projections.dedup();
    let materialization = materialization_plan_from_projections(target, &managed_projections)
        .ok_or_else(|| {
            managed_project_error(
                "managed_project_materialization_invalid",
                "The managed projection manifest could not be compared safely.",
            )
        })?;
    let files = plan
        .files
        .into_iter()
        .map(managed_lifecycle_file_write)
        .collect::<Result<Vec<_>, _>>()?;
    let trees = plan
        .trees
        .into_iter()
        .map(managed_lifecycle_plugin_write)
        .collect::<Vec<_>>();
    let source = checkout
        .map(|checkout| checkout.source().clone())
        .or_else(|| target_state.and_then(TargetResourceState::source).cloned());
    let installed_revision = checkout
        .and_then(|checkout| checkout.revision().cloned())
        .or_else(|| target_state.and_then(|target| target.installed_revision().cloned()));

    validate_managed_ownership(
        kind,
        existing_state,
        target,
        ManagedOwnershipEvidence {
            current_fingerprint: current_fingerprint.as_ref(),
            desired_fingerprint: fingerprint.as_ref(),
            desired_projections: &managed_projections,
            installed_revision: installed_revision.as_ref(),
            operation_id: &operation_id,
        },
    )?;
    let mut surfaces = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    surfaces.extend(trees.iter().map(|tree| {
        AbsolutePath::new(format!(
            "{}/{}",
            tree.root.as_str(),
            tree.destination.as_str()
        ))
        .expect("validated projection path")
    }));
    let mut surface_kinds = BTreeSet::new();
    if !files.is_empty() {
        surface_kinds.insert(ManagedSurfaceKind::ManagedDocument);
    }
    if !trees.is_empty() {
        surface_kinds.insert(ManagedSurfaceKind::CompleteSkillTree);
    }
    let managed_requirement = CapabilityRequirement::new(
        CapabilityId::new("managed.projection").expect("static capability id is valid"),
        [],
    );
    let authorization = if surfaces.is_empty() {
        MutationAuthorization::Supported
    } else {
        authorize_mutation(MutationAuthorityRequest {
            profile: &profile.profile,
            scope,
            channel: MutationChannel::ManagedProjection,
            required: std::slice::from_ref(&managed_requirement),
            surfaces: &surface_kinds,
            declaration: profile.declaration_contract.as_ref(),
        })
        .map_err(|error| {
            ErrorDetail::new("managed_mutation_unauthorized", error.to_string())
                .with_context("target", target.as_str())
                .with_context("scope", scope_label(scope))
        })?
    };
    let mut partial_evidence = BTreeSet::new();
    let mut partial_consequences = BTreeSet::new();
    for projection in &managed_projections {
        let ManagedProjection::Omitted { id, consequence } = projection else {
            continue;
        };
        let code = consequence.clone();
        let component = id.clone();
        partial_evidence.insert(CompatibilityEvidence::new(
            code.clone(),
            target.clone(),
            [component.clone()],
            EvidenceDetail::new(
                "An optional component cannot be represented faithfully on this target.",
            )
            .expect("static evidence detail is valid"),
        ));
        partial_consequences.insert(MaterialConsequence::new(
            ConsequenceCode::new(code.as_str()).expect("stored consequence code is valid"),
            [component],
            skilltap_core::domain::ConsequenceSummary::new(
                "The optional component will be omitted from the managed declaration.",
            )
            .expect("static consequence summary is valid"),
        ));
    }
    if let MutationAuthorization::DeclarationManaged { unverified } = &authorization {
        for requirement in unverified {
            let components = requirement.affected_components.clone();
            partial_evidence.insert(CompatibilityEvidence::new(
                EvidenceCode::new("managed.effective_unverified")
                    .expect("static evidence code is valid"),
                target.clone(),
                components.clone(),
                EvidenceDetail::new(
                    "The managed declaration can be verified on disk, but the harness load or activation result is unverified.",
                )
                .expect("static evidence detail is valid"),
            ));
            partial_consequences.insert(MaterialConsequence::new(
                ConsequenceCode::new("managed.effective_unverified")
                    .expect("static consequence code is valid"),
                components,
                skilltap_core::domain::ConsequenceSummary::new(
                    "The declaration will be written, but effective harness loading remains unverified.",
                )
                .expect("static consequence summary is valid"),
            ));
        }
    }
    let operation = if surfaces.is_empty() && resource.kind() == ResourceKind::Marketplace {
        skilltap_core::lifecycle_operation::managed_source_registration_operation(
            operation_id,
            target.clone(),
            resource.key().clone(),
            request.operation_action(),
        )
    } else if !partial_consequences.is_empty() {
        managed_partial_materialization_operation(
            operation_id,
            target.clone(),
            resource.key().clone(),
            request.operation_action(),
            surfaces,
            partial_evidence,
            partial_consequences,
        )
    } else {
        managed_materialization_operation(
            operation_id,
            target.clone(),
            resource.key().clone(),
            request.operation_action(),
            surfaces,
        )
    }
    .map_err(|_| {
        managed_project_error(
            "operation_contract_invalid",
            "The managed lifecycle operation could not be represented safely.",
        )
    })?;
    let seed = if removal {
        None
    } else {
        Some(
            TargetResourceState::new(
                target.clone(),
                Some(request.native_name.clone()),
                Provenance::Materialized,
                Ownership::Skilltap,
                source,
                None,
                fingerprint,
                installed_revision,
                None,
                timestamp,
                None,
            )
            .map(|target| target.with_managed_projections(managed_projections))
            .and_then(|target| ResourceState::new(resource.key().clone(), [target]))
            .map_err(|_| {
                managed_project_error(
                    "state_seed_invalid",
                    "The managed project state evidence is invalid.",
                )
            })?,
        )
    };
    Ok(PlannedManagedLifecycle {
        operation,
        entry: ManagedLifecycleEntry {
            files,
            trees,
            profile,
            requirements: vec![managed_requirement],
            surfaces: surface_kinds,
            authorization,
        },
        seed,
        materialization,
    })
}

fn materialization_plan_from_projections(
    target: &HarnessId,
    projections: &[ManagedProjection],
) -> Option<MaterializationPlan> {
    let mut included = BTreeSet::new();
    let mut omitted_optional = BTreeSet::new();
    for projection in projections {
        let (prefix, id, omitted) = match projection {
            ManagedProjection::Skill { id, .. } => ("skill", id.as_str(), false),
            ManagedProjection::Mcp { id, .. } => ("mcp", id.as_str(), false),
            ManagedProjection::Omitted { id, .. } => ("", id.as_str(), true),
        };
        let component = if omitted {
            ComponentId::new(id.to_owned()).ok()?
        } else {
            ComponentId::new(format!("{prefix}:{id}")).ok()?
        };
        if omitted {
            omitted_optional.insert(component);
        } else {
            included.insert(component);
        }
    }
    Some(MaterializationPlan {
        target: target.clone(),
        included,
        omitted_optional,
        blocked_required: BTreeSet::new(),
    })
}

enum LifecycleRoute {
    Native,
    Managed(Option<Box<PlannedManagedLifecycle>>),
}

struct LifecycleRouteContext<'a> {
    registry: &'a skilltap_harnesses::TargetRegistry,
    documents: &'a StatusDocuments,
    paths: &'a PlatformPaths,
    target: &'a HarnessId,
    kind: NativeLifecycleKind,
    request: &'a NativeLifecycleSpec,
    resource: &'a DesiredResource,
    scope: &'a Scope,
    environment: &'a BTreeMap<OsString, OsString>,
    search_path: Option<OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    timestamp: Timestamp,
    acknowledged: bool,
    filesystem: &'a dyn ManagedLifecycleFileSystem,
}

/// Select one target-local lifecycle representation before building an
/// execution operation. The result is target-neutral; concrete adapters keep
/// their native parsing and the existing executors keep ownership/recovery.
fn select_lifecycle_route(
    context: LifecycleRouteContext<'_>,
) -> Result<LifecycleRoute, ErrorDetail> {
    let LifecycleRouteContext {
        registry,
        documents,
        paths,
        target,
        kind,
        request,
        resource,
        scope,
        environment,
        search_path,
        process_limits,
        json_limits,
        timestamp,
        acknowledged,
        filesystem,
    } = context;
    let existing_state = documents
        .state
        .as_ref()
        .and_then(|state| state.resources().get(resource.key()))
        .and_then(|state| state.target(target));
    if let Some(state) = existing_state {
        return applied_lifecycle_representation(state)
            .map(|representation| match representation {
                LifecycleRepresentation::Native => LifecycleRoute::Native,
                LifecycleRepresentation::Managed => LifecycleRoute::Managed(None),
            })
            .map_err(lifecycle_representation_error);
    }

    if kind == NativeLifecycleKind::PluginInstall {
        let selector =
            skilltap_core::marketplace::PluginSelector::parse(request.native_name.as_str())
                .map_err(|_| {
                    managed_project_error(
                        "invalid_plugin_selector",
                        "The selected plugin selector is invalid.",
                    )
                })?;
        let marketplace_key = ResourceKey::new(
            ResourceId::new(format!("marketplace:{}", selector.marketplace().as_str())).map_err(
                |_| {
                    managed_project_error(
                        "marketplace_resource_invalid",
                        "The selected marketplace identity is invalid.",
                    )
                },
            )?,
            scope.clone(),
        );
        if let Some(marketplace_state) = documents
            .state
            .as_ref()
            .and_then(|state| state.resources().get(&marketplace_key))
            .and_then(|state| state.target(target))
        {
            return applied_lifecycle_representation(marketplace_state)
                .map(|representation| match representation {
                    LifecycleRepresentation::Native => LifecycleRoute::Native,
                    LifecycleRepresentation::Managed => LifecycleRoute::Managed(None),
                })
                .map_err(lifecycle_representation_error);
        }
        // A legacy/native installation can be requested before the marketplace
        // has a persisted target binding. Preserve the existing capability
        // route in that case; once a marketplace binding exists it is always
        // authoritative above.
    }

    let adapter = registry.adapter(target).ok_or_else(|| {
        managed_project_error(
            "lifecycle_target_unregistered",
            "The selected lifecycle target is not registered.",
        )
    })?;
    let native_capability = lifecycle_capability_name(kind);
    let native_profile = configured_native_profile(
        registry,
        &documents.config,
        target,
        NativeProfileRequest {
            scope,
            environment,
            process_limits,
            json_limits,
            search_path: search_path.clone(),
            capability_name: native_capability,
        },
    )
    .ok()
    .flatten();
    let managed_profile = configured_adapter_profile(
        registry,
        &documents.config,
        target,
        NativeProfileRequest {
            scope,
            environment,
            process_limits,
            json_limits,
            search_path,
            capability_name: "managed.projection",
        },
    )
    .ok()
    .flatten();

    let mut managed_error = None;
    let mut managed_plan = None;
    let native_candidate = if kind == NativeLifecycleKind::MarketplaceAdd
        && let Some(native_distribution) = adapter.native_distribution()
    {
        let source = resource
            .source()
            .cloned()
            .or_else(|| {
                existing_state
                    .and_then(TargetResourceState::source)
                    .cloned()
            })
            .ok_or_else(|| {
                managed_project_error(
                    "native_distribution_source_missing",
                    "The native distribution assessment has no explicit source.",
                )
            })?;
        let checkout = resolve_managed_source_checkout(paths, source)?;
        let assessment = native_distribution
            .assess(&NativeDistributionContext {
                target,
                scope,
                checkout: &checkout,
                requested_revision: resource
                    .source()
                    .and_then(|source| source.requested_revision()),
                filesystem,
                json_limits,
            })
            .map_err(|error| ErrorDetail::new(error.code(), error.summary()))?;
        let candidate = assessment.map(|assessment| RepresentationCandidate {
            representation: LifecycleRepresentation::Native,
            plan: assessment.plan,
        });
        if let Some(profile) = managed_profile.clone() {
            match plan_managed_lifecycle(
                registry,
                target,
                kind,
                request,
                resource,
                profile,
                ManagedPlanContext {
                    scope,
                    documents,
                    paths,
                    timestamp,
                    json_limits,
                    acknowledged,
                    filesystem,
                    checkout: Some(&checkout),
                },
            ) {
                Ok(planned) => {
                    let plan = planned.materialization.clone();
                    managed_plan = Some(planned);
                    Some((
                        candidate,
                        Some(RepresentationCandidate {
                            representation: LifecycleRepresentation::Managed,
                            plan,
                        }),
                    ))
                }
                Err(error) => {
                    managed_error = Some(error);
                    Some((candidate, None))
                }
            }
        } else {
            Some((candidate, None))
        }
    } else {
        None
    };

    let evidence = if let Some((native, managed)) = native_candidate {
        RepresentationEvidence::Fresh { native, managed }
    } else {
        RepresentationEvidence::Fresh {
            native: if !adapter
                .supports_managed_projection(skilltap_core::domain::CapabilityScope::from(scope))
                && adapter.native_lifecycle().is_some()
            {
                Some(RepresentationCandidate {
                    representation: LifecycleRepresentation::Native,
                    plan: empty_materialization_plan(target),
                })
            } else {
                native_profile
                    .filter(|profile| profile.capability == CapabilitySupport::Supported)
                    .map(|_| RepresentationCandidate {
                        representation: LifecycleRepresentation::Native,
                        plan: empty_materialization_plan(target),
                    })
            },
            managed: if adapter
                .supports_managed_projection(skilltap_core::domain::CapabilityScope::from(scope))
            {
                Some(RepresentationCandidate {
                    representation: LifecycleRepresentation::Managed,
                    plan: empty_materialization_plan(target),
                })
            } else {
                managed_profile.map(|_| RepresentationCandidate {
                    representation: LifecycleRepresentation::Managed,
                    plan: empty_materialization_plan(target),
                })
            },
        }
    };
    match select_lifecycle_representation(evidence) {
        Ok(LifecycleRepresentation::Native) => Ok(LifecycleRoute::Native),
        Ok(LifecycleRepresentation::Managed) => {
            if let Some(planned) = managed_plan {
                Ok(LifecycleRoute::Managed(Some(Box::new(planned))))
            } else if let Some(error) = managed_error {
                Err(error)
            } else {
                Ok(LifecycleRoute::Managed(None))
            }
        }
        Err(error) => Err(managed_error.unwrap_or_else(|| lifecycle_representation_error(error))),
    }
}

fn lifecycle_capability_name(kind: NativeLifecycleKind) -> &'static str {
    match kind {
        NativeLifecycleKind::MarketplaceAdd => "marketplace.register",
        NativeLifecycleKind::MarketplaceRemove => "marketplace.remove",
        NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
        NativeLifecycleKind::PluginInstall => "plugin.install",
        NativeLifecycleKind::PluginRemove => "plugin.remove",
        NativeLifecycleKind::PluginUpdate => "plugin.update",
    }
}

fn empty_materialization_plan(target: &HarnessId) -> MaterializationPlan {
    MaterializationPlan {
        target: target.clone(),
        included: BTreeSet::new(),
        omitted_optional: BTreeSet::new(),
        blocked_required: BTreeSet::new(),
    }
}

fn lifecycle_representation_error(error: LifecycleRepresentationError) -> ErrorDetail {
    let (code, summary) = match error {
        LifecycleRepresentationError::ContradictoryAppliedState => (
            "lifecycle_representation_contradictory",
            "Native and managed state evidence contradicts itself; no lifecycle route was selected.",
        ),
        LifecycleRepresentationError::MissingMarketplaceRepresentation => (
            "marketplace_representation_missing",
            "The plugin's target-local marketplace representation is missing; no lifecycle route was selected.",
        ),
        LifecycleRepresentationError::RequiredComponentsBlocked => (
            "required_components_blocked",
            "Every available lifecycle representation blocks a required component.",
        ),
        LifecycleRepresentationError::IncomparablePartialRepresentations => (
            "partial_representations_incomparable",
            "Native and managed representations preserve different partial component sets.",
        ),
        LifecycleRepresentationError::NoSupportedRepresentation => (
            "lifecycle_representation_unavailable",
            "No mutation-authorized native or managed lifecycle representation is available.",
        ),
    };
    ErrorDetail::new(code, summary)
}

fn managed_lifecycle_kind(kind: NativeLifecycleKind) -> ManagedLifecycleKind {
    match kind {
        NativeLifecycleKind::MarketplaceAdd => ManagedLifecycleKind::MarketplaceAdd,
        NativeLifecycleKind::MarketplaceRemove => ManagedLifecycleKind::MarketplaceRemove,
        NativeLifecycleKind::MarketplaceUpdate => ManagedLifecycleKind::MarketplaceUpdate,
        NativeLifecycleKind::PluginInstall => ManagedLifecycleKind::PluginInstall,
        NativeLifecycleKind::PluginRemove => ManagedLifecycleKind::PluginRemove,
        NativeLifecycleKind::PluginUpdate => ManagedLifecycleKind::PluginUpdate,
    }
}

fn resolve_managed_source_checkout(
    paths: &PlatformPaths,
    source: Source,
) -> Result<ResolvedSourceCheckout, ErrorDetail> {
    match source.kind() {
        SourceKind::Local => {
            let root = AbsolutePath::new(source.locator().as_str()).map_err(|_| {
                managed_project_error(
                    "managed_project_source_invalid",
                    "The managed marketplace source path is invalid.",
                )
            })?;
            Ok(ResolvedSourceCheckout::new(root, source, None))
        }
        SourceKind::Git => {
            let resolved = resolve_git_skill_source(
                paths,
                source.locator(),
                source.requested_revision(),
                None,
            )
            .map_err(|_| {
                managed_project_error(
                    "managed_project_source_unavailable",
                    "The Git marketplace source could not be cloned and checked out safely.",
                )
            })?;
            Ok(ResolvedSourceCheckout::new(
                resolved.root,
                source,
                Some(skilltap_core::domain::ResolvedRevision::GitCommit(
                    resolved.commit,
                )),
            ))
        }
        SourceKind::RemoteCatalog => Err(managed_project_error(
            "managed_project_source_unsupported",
            "Remote catalog payloads are not a verified plugin checkout.",
        )),
    }
}

fn managed_lifecycle_file_write(
    write: ManagedFileWrite,
) -> Result<ManagedLifecycleFileWrite, ErrorDetail> {
    let path = AbsolutePath::new(format!(
        "{}/{}",
        write.root.as_str(),
        write.destination.as_str()
    ))
    .map_err(|_| {
        managed_project_error(
            "managed_project_path_invalid",
            "A managed project file path is invalid.",
        )
    })?;
    Ok(ManagedLifecycleFileWrite {
        path,
        root: write.root,
        destination: write.destination,
        expected: write.expected,
        desired: write.desired,
    })
}

fn managed_lifecycle_plugin_write(write: ManagedPluginWrite) -> ManagedLifecyclePluginWrite {
    ManagedLifecyclePluginWrite {
        root: write.root,
        destination: write.destination,
        desired_tree: write.desired_tree,
        expected_tree: write.expected_tree,
        expected_identity: write.expected_identity,
    }
}

fn managed_tree_observation_limits() -> ExternalTreeLimits {
    ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
        .expect("bounded project tree limits are valid")
}

type ObservedManagedTree = (
    skilltap_core::runtime::DirectoryIdentity,
    BTreeMap<skilltap_core::domain::RelativeArtifactPath, ArtifactFile>,
);

struct ManagedOwnershipEvidence<'a> {
    current_fingerprint: Option<&'a Fingerprint>,
    desired_fingerprint: Option<&'a Fingerprint>,
    desired_projections: &'a [ManagedProjection],
    installed_revision: Option<&'a skilltap_core::domain::ResolvedRevision>,
    operation_id: &'a OperationId,
}

fn validate_managed_ownership(
    kind: NativeLifecycleKind,
    state: Option<&ResourceState>,
    target: &HarnessId,
    evidence: ManagedOwnershipEvidence<'_>,
) -> Result<(), ErrorDetail> {
    let ManagedOwnershipEvidence {
        current_fingerprint,
        desired_fingerprint,
        desired_projections,
        installed_revision,
        operation_id,
    } = evidence;
    if let Some(current_fingerprint) = current_fingerprint {
        let state = state.ok_or_else(|| {
            managed_project_error(
                "managed_project_unowned",
                "The existing managed destination has no skilltap ownership record.",
            )
        })?;
        let state = state.target(target).ok_or_else(|| {
            managed_project_error(
                "managed_project_unowned",
                "The existing managed destination has no ownership binding for the selected target.",
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
        let recoverable_pending_attempt = desired_fingerprint == Some(current_fingerprint)
            && state.pending_managed_attempt().is_some_and(|attempt| {
                attempt.operation_id() == operation_id
                    && attempt.fingerprint() == current_fingerprint
                    && attempt.managed_projections() == desired_projections
                    && attempt.installed_revision() == installed_revision
            })
            && state.last_apply().is_some_and(|apply| {
                apply
                    .operations()
                    .get(operation_id)
                    .is_some_and(|result| matches!(result.outcome(), OperationOutcome::Pending))
            });
        if state.fingerprint() != Some(current_fingerprint) && !recoverable_pending_attempt {
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
    registry: &skilltap_harnesses::TargetRegistry,
    documents: &StatusDocuments,
    kind: ResourceKind,
    harness: &HarnessId,
    scope: &Scope,
    name: &str,
) -> NativeResourceObservation {
    let action = match kind {
        ResourceKind::Marketplace => NativeLifecycleAction::MarketplaceAdd,
        ResourceKind::Plugin => NativeLifecycleAction::PluginInstall,
        ResourceKind::StandaloneSkill
        | ResourceKind::InstructionLocation
        | ResourceKind::Harness => {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        }
    };
    let Some(adapter) = registry.adapter(harness) else {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    };
    let Some(lifecycle) = adapter.native_lifecycle() else {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    };
    let configured = documents
        .config
        .harnesses()
        .get(harness)
        .ok_or(())
        .and_then(|policy| configured_binary(policy.binary.as_str()));
    let Ok(configured) = configured else {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::CommandFailed);
    };
    let Ok(name) = NativeId::new(name) else {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    };
    let request = NativeLifecycleRequest {
        action,
        scope: scope.clone(),
        name,
        source: None,
    };
    let dispatch = NativeLifecycleDispatch::new(harness.clone(), lifecycle, request);
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded lifecycle process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded lifecycle JSON limits are valid");
    let Ok(paths) = PlatformPaths::resolve(&ProcessEnvironment) else {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::CommandFailed);
    };
    let search_path = std::env::var_os("PATH");
    let Ok(environment) = paths.native_process_environment(search_path.clone()) else {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::CommandFailed);
    };
    observe_native_resource(
        configured,
        search_path,
        &environment,
        &dispatch,
        process_limits,
        json_limits,
    )
    .unwrap_or(NativeResourceObservation::Indeterminate(
        NativeObservationFailure::CommandFailed,
    ))
}

fn lifecycle_presence_label(presence: NativeResourceObservation) -> &'static str {
    match presence {
        NativeResourceObservation::Present { .. } => "present",
        NativeResourceObservation::Missing => "missing",
        NativeResourceObservation::Indeterminate(_) => "unknown",
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
                && existing_target.managed_projections() == seed_target.managed_projections()
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
    config.harnesses().enabled().cloned().collect()
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
