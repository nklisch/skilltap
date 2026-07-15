use super::*;

#[derive(Default)]
struct NativeLifecyclePlanBuilder {
    operations: Vec<skilltap_core::domain::Operation>,
    native_bindings: Vec<NativeLifecycleBinding>,
    managed_entries: BTreeMap<OperationId, ManagedLifecycleEntry>,
    seeds: BTreeMap<ResourceKey, ResourceState>,
    foreign_operations: BTreeSet<OperationId>,
    pending_updates: Vec<skilltap_core::daemon::DaemonPendingUpdate>,
}

struct DaemonNativeExecution {
    outcome: Outcome,
    changed: bool,
    safe_operations: u64,
    pending_operations: u64,
    operations: Vec<DaemonOperationRef>,
}

fn standalone_skill_operation(
    operation_id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    path: AbsolutePath,
    partial: bool,
) -> Result<skilltap_core::domain::Operation, skilltap_core::domain::OperationContractError> {
    if !partial {
        return skilltap_core::lifecycle_operation::faithful_file_operation(
            operation_id,
            target,
            resource,
            OperationAction::SkillInstall,
            path,
        );
    }
    let component = ComponentId::new(resource.id().as_str().to_owned())
        .expect("standalone skill component id is valid");
    partial_file_operation(
        operation_id,
        target.clone(),
        resource,
        OperationAction::SkillInstall,
        path,
        [CompatibilityEvidence::new(
            EvidenceCode::new("skill.frontmatter_unverified").expect("static evidence code is valid"),
            target,
            [component.clone()],
            EvidenceDetail::new(
                "The skill is loadable, but its frontmatter is not fully strict for this target.",
            )
            .expect("static evidence detail is valid"),
        )],
        [MaterialConsequence::new(
            ConsequenceCode::new("skill.frontmatter_unverified")
                .expect("static consequence code is valid"),
            [component],
            skilltap_core::domain::ConsequenceSummary::new(
                "The skill will be installed while strict frontmatter compatibility remains unverified.",
            )
            .expect("static consequence summary is valid"),
        )],
    )
}

fn daemon_capability_name(kind: NativeLifecycleKind) -> &'static str {
    match kind {
        NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
        NativeLifecycleKind::PluginUpdate => "plugin.update",
        NativeLifecycleKind::MarketplaceAdd
        | NativeLifecycleKind::MarketplaceRemove
        | NativeLifecycleKind::PluginInstall
        | NativeLifecycleKind::PluginRemove => "daemon.unsupported",
    }
}

fn daemon_action_label(action: OperationAction) -> &'static str {
    match action {
        OperationAction::MarketplaceUpdate => "marketplace_refresh",
        OperationAction::PluginUpdate => "plugin_update",
        OperationAction::SkillUpdate => "skill_update",
        _ => "lifecycle",
    }
}

fn daemon_block_reason(
    reason: skilltap_core::daemon::DaemonPluginBlockReason,
) -> (&'static str, &'static str) {
    match reason {
        skilltap_core::daemon::DaemonPluginBlockReason::InvalidSelector => (
            "daemon.invalid_plugin_selector",
            "The desired plugin selector is not a valid plugin@marketplace identity.",
        ),
        skilltap_core::daemon::DaemonPluginBlockReason::MarketplaceMissing => (
            "daemon.marketplace_missing",
            "The plugin's exact marketplace is not registered in the same scope.",
        ),
        skilltap_core::daemon::DaemonPluginBlockReason::MarketplaceTargetMissing => (
            "daemon.marketplace_target_missing",
            "The plugin target is not registered for its exact marketplace.",
        ),
        skilltap_core::daemon::DaemonPluginBlockReason::MarketplaceUpdateDisabled => (
            "daemon.marketplace_update_disabled",
            "The plugin's marketplace has automatic updates disabled.",
        ),
        skilltap_core::daemon::DaemonPluginBlockReason::MarketplacePinned => (
            "daemon.marketplace_pinned",
            "The plugin's marketplace is pinned and requires foreground review.",
        ),
    }
}

fn daemon_blocked_operation_id(resource: &ResourceKey, target: &HarnessId) -> OperationId {
    let label = format!("daemon-blocked:{target}:{resource}");
    let hash = stable_hash(&label);
    OperationId::new(format!("daemon-blocked:{}:{hash:016x}", target.as_str()))
        .expect("daemon blocked operation id is valid")
}

fn mark_daemon_declaration_pending(
    builder: &mut NativeLifecyclePlanBuilder,
    resource: &DesiredResource,
    target: &HarnessId,
    outcome: &mut Outcome,
) {
    builder
        .pending_updates
        .push(skilltap_core::daemon::DaemonPendingUpdate::new(
            resource.key().clone(),
            target.clone(),
            skilltap_core::daemon::DaemonPendingReason::DeclarationManaged,
        ));
    *outcome = outcome
        .clone()
        .with_warning(
            Warning::new(
                "daemon.declaration_managed_pending",
                "A declaration-managed update remains pending for foreground acknowledgment; the daemon did not construct or execute it.",
            )
            .with_context("target", target.as_str())
            .with_context("scope", scope_label(resource.scope()))
            .with_context("resource", resource.key().to_string()),
        );
    outcome.result = ResultClass::AttentionRequired;
}

fn native_state_seed(
    resource: &DesiredResource,
    target: &HarnessId,
    native_id: &NativeId,
    observed_at: Timestamp,
    before: &NativeResourceObservation,
) -> ResourceState {
    let installed_revision = match before {
        NativeResourceObservation::Present { revision, .. } => revision.clone(),
        NativeResourceObservation::Missing | NativeResourceObservation::Indeterminate(_) => None,
    };
    let binding = TargetResourceState::new(
        target.clone(),
        Some(native_id.clone()),
        Provenance::Native,
        Ownership::Harness,
        resource.source().cloned(),
        None,
        None,
        installed_revision,
        None,
        observed_at,
        None,
    )
    .expect("daemon native state seed is valid");
    ResourceState::new(resource.key().clone(), [binding]).expect("daemon native state is valid")
}

fn previously_attempted(
    state: Option<&StateDocument>,
    resource: &ResourceKey,
    target: &HarnessId,
    operation: &OperationId,
) -> bool {
    if previously_applied(state, resource, target, operation) {
        return true;
    }
    state
        .and_then(|state| state.resources().get(resource))
        .and_then(|state| state.target(target))
        .and_then(|state| state.last_apply())
        .and_then(|apply| apply.operations().get(operation))
        .is_some()
}

impl StatusApplication<'_> {
    pub(super) fn lifecycle_platform_paths(
        &self,
    ) -> Result<PlatformPaths, skilltap_core::runtime::RuntimeError> {
        #[cfg(test)]
        if let Some(paths) = &self.test_platform_paths {
            return Ok(paths.clone());
        }
        PlatformPaths::resolve(&ProcessEnvironment)
    }

    fn managed_filesystem(&self) -> &dyn ManagedLifecycleFileSystem {
        #[cfg(test)]
        if let Some(filesystem) = self.test_managed_filesystem {
            return filesystem;
        }
        &SystemFileSystem
    }

    #[allow(dead_code)]
    pub(crate) fn execute_daemon_cycle(&self) -> Outcome {
        self.execute_daemon_cycle_with_binary(None)
    }

    /// Execute one unattended resource cycle, optionally incorporating the
    /// verified self-update result produced by the CLI composition boundary.
    /// The binary policy is deliberately passed in as an already-rendered
    /// result so this application service never creates a second resolver or
    /// installer path.
    pub(crate) fn execute_daemon_cycle_with_binary(&self, binary: Option<Outcome>) -> Outcome {
        let command = "daemon run";
        let (documents, mut aggregate) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let mut safe_operations = 0_u64;
        let mut pending_operations = 0_u64;
        if let Some(binary) = binary {
            if binary.summary.get("binary_changed") == Some(&OutputValue::Boolean(true)) {
                safe_operations += 1;
            }
            if binary.summary.get("binary_pending") == Some(&OutputValue::Boolean(true)) {
                pending_operations += 1;
            }
            aggregate.result = merge_result(aggregate.result, binary.result);
            aggregate.resources.extend(binary.resources);
            aggregate.operations.extend(binary.operations);
            aggregate.warnings.extend(binary.warnings);
            aggregate.errors.extend(binary.errors);
            aggregate.next_actions.extend(binary.next_actions);
        }
        if documents.config.updates().mode != skilltap_core::storage::UpdateMode::ApplySafe {
            aggregate = aggregate
                .with_warning(Warning::new(
                    "daemon_policy_not_apply_safe",
                    "The configured update policy does not permit automatic application.",
                ))
                .with_summary("changed", safe_operations > 0)
                .with_summary("safe_operations", safe_operations)
                .with_summary("pending_operations", pending_operations);
            normalize_daemon_noop_result(&mut aggregate, safe_operations, pending_operations);
            self.persist_daemon_run(&mut aggregate, safe_operations, pending_operations, []);
            return aggregate;
        }

        let native = self.execute_daemon_native_plan(&documents, aggregate);
        let daemon_operations = native.operations.clone();
        let mut aggregate = native.outcome;
        let mut changed = native.changed || safe_operations > 0;
        safe_operations += native.safe_operations;
        pending_operations += native.pending_operations;

        // Git-backed standalone skills deliberately remain a separate child
        // phase. They have no native marketplace prerequisite and reuse their
        // existing managed replacement port after the native batch completes.
        if let Some(inventory) = documents.inventory.as_ref() {
            let skills = inventory
                .resources()
                .values()
                .filter(|resource| {
                    resource.kind() == ResourceKind::StandaloneSkill
                        && resource.update() == UpdateIntent::Track
                        && resource
                            .source()
                            .is_some_and(|source| source.kind() == SourceKind::Git)
                })
                .filter_map(|resource| {
                    let name = resource.id().as_str().strip_prefix("skill:")?;
                    Some((name.to_owned(), resource.scope().clone()))
                })
                .collect::<Vec<_>>();
            for (name, scope) in skills {
                let child_scope = scope_args_for_scope(&scope);
                let child = self.execute_skill_update(
                    "skill update",
                    &child_scope,
                    &TargetArgs::default(),
                    Some(&name),
                    false,
                );
                changed |= child.summary.get("changed") == Some(&OutputValue::Boolean(true));
                if child.result == ResultClass::Completed {
                    safe_operations += child.operations.len() as u64;
                } else {
                    pending_operations += 1;
                }
                aggregate.result = merge_result(aggregate.result, child.result);
                aggregate.resources.extend(child.resources);
                aggregate.operations.extend(child.operations);
                aggregate.warnings.extend(child.warnings);
                aggregate.errors.extend(child.errors);
                aggregate.next_actions.extend(child.next_actions);
            }
        }
        aggregate = aggregate
            .with_summary("changed", changed)
            .with_summary("safe_operations", safe_operations)
            .with_summary("pending_operations", pending_operations);
        normalize_daemon_noop_result(&mut aggregate, safe_operations, pending_operations);
        self.persist_daemon_run(
            &mut aggregate,
            safe_operations,
            pending_operations,
            daemon_operations,
        );
        aggregate
    }

    fn execute_daemon_native_plan(
        &self,
        documents: &StatusDocuments,
        mut outcome: Outcome,
    ) -> DaemonNativeExecution {
        let Some(inventory) = documents.inventory.as_ref() else {
            return DaemonNativeExecution {
                outcome,
                changed: false,
                safe_operations: 0,
                pending_operations: 0,
                operations: Vec::new(),
            };
        };
        let paths = match self.lifecycle_platform_paths() {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return DaemonNativeExecution {
                    outcome: outcome.with_error(ErrorDetail::new(
                        "platform_paths_unavailable",
                        "The skilltap configuration paths could not be resolved for the daemon update cycle.",
                    )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: 1,
                    operations: Vec::new(),
                };
            }
        };
        let search_path = std::env::var_os("PATH");
        let native_environment = match paths.native_process_environment(search_path.clone()) {
            Ok(environment) => environment,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return DaemonNativeExecution {
                    outcome: outcome.with_error(ErrorDetail::new(
                        "native_environment_unavailable",
                        "The bounded native process environment could not be resolved for the daemon update cycle.",
                    )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: 1,
                    operations: Vec::new(),
                };
            }
        };
        let plan =
            skilltap_core::daemon::plan_daemon_native_updates(inventory.resources().values());
        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded daemon lifecycle process limits are valid");
        let json_limits = JsonLimits::new(256 * 1024, 64)
            .expect("bounded daemon lifecycle JSON limits are valid");
        let timestamp = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(timestamp) => timestamp,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return DaemonNativeExecution {
                    outcome: outcome.with_error(ErrorDetail::new(
                        "clock_unavailable",
                        "The daemon lifecycle operation timestamp could not be recorded safely.",
                    )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: 1,
                    operations: Vec::new(),
                };
            }
        };
        let mut builder = NativeLifecyclePlanBuilder::default();
        let mut refresh_ids = BTreeMap::new();
        let mut pending_refreshes = BTreeSet::new();
        for refresh in plan.refreshes() {
            let Some(resource) = inventory.resources().get(refresh.key().resource()) else {
                continue;
            };
            let pending_before = builder.pending_updates.len();
            let operation = self.plan_daemon_lifecycle_target(
                &mut builder,
                documents,
                &paths,
                &native_environment,
                process_limits,
                json_limits,
                timestamp,
                NativeLifecycleKind::MarketplaceUpdate,
                resource,
                refresh.key().target(),
                refresh.name(),
                BTreeSet::new(),
                &mut outcome,
            );
            if let Some(operation) = operation {
                refresh_ids.insert(refresh.key().clone(), operation);
            } else if builder.pending_updates.len() > pending_before {
                pending_refreshes.insert(refresh.key().clone());
            }
        }
        for plugin in plan.plugins() {
            let Some(resource) = inventory.resources().get(plugin.resource()) else {
                continue;
            };
            if pending_refreshes.contains(plugin.refresh()) {
                mark_daemon_declaration_pending(
                    &mut builder,
                    resource,
                    plugin.target(),
                    &mut outcome,
                );
                continue;
            }
            let dependencies = refresh_ids
                .get(plugin.refresh())
                .cloned()
                .map(|id| BTreeSet::from([OperationDependency::new(id)]))
                .unwrap_or_default();
            self.plan_daemon_lifecycle_target(
                &mut builder,
                documents,
                &paths,
                &native_environment,
                process_limits,
                json_limits,
                timestamp,
                NativeLifecycleKind::PluginUpdate,
                resource,
                plugin.target(),
                &NativeId::new(plugin.selector().to_string()).expect("validated plugin selector"),
                dependencies,
                &mut outcome,
            );
        }
        for blocked in plan.blocked_plugins() {
            let operation_id = daemon_blocked_operation_id(blocked.resource(), blocked.target());
            let action = OperationAction::PluginUpdate;
            let (code, detail) = daemon_block_reason(blocked.reason());
            let operation = skilltap_core::lifecycle_operation::blocked_native_operation(
                operation_id.clone(),
                blocked.target().clone(),
                blocked.resource().clone(),
                action,
                EvidenceCode::new(code).expect("static daemon evidence code is valid"),
                EvidenceDetail::new(detail).expect("static daemon evidence detail is valid"),
            );
            match operation {
                Ok(operation) => {
                    builder.foreign_operations.insert(operation_id);
                    builder.operations.push(operation);
                }
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(Warning::new(
                        "daemon_plan_invalid",
                        "A blocked daemon plugin relationship could not be represented safely.",
                    ));
                }
            }
        }
        if builder.operations.is_empty() {
            return DaemonNativeExecution {
                outcome,
                changed: false,
                safe_operations: 0,
                pending_operations: builder.pending_updates.len() as u64,
                operations: Vec::new(),
            };
        }
        let plan = match Plan::new(builder.operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return DaemonNativeExecution {
                    outcome: outcome.with_error(ErrorDetail::new(
                        "operation_plan_invalid",
                        "The daemon native update operations did not form a valid dependency plan.",
                    )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: 1,
                    operations: Vec::new(),
                };
            }
        };
        let native_port = NativeLifecyclePort::new_bound_with_environment(
            builder.native_bindings,
            native_environment.clone(),
        )
        .with_foreign_operations(builder.foreign_operations.iter().cloned());
        let port = HybridLifecyclePort {
            native: native_port,
            managed: ManagedLifecyclePort {
                filesystem: self.managed_filesystem(),
                entries: builder.managed_entries,
                registry: self.registry,
                config: &documents.config,
                environment: &native_environment,
                search_path: std::env::var_os("PATH"),
                process_limits,
                json_limits,
            },
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
            seeds: builder.seeds,
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return DaemonNativeExecution {
                    outcome: outcome.with_error(ErrorDetail::new(
                        "lock_path_invalid",
                        "The daemon configuration lock path is invalid.",
                    )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: 1,
                    operations: Vec::new(),
                };
            }
        };
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return DaemonNativeExecution {
                    outcome: outcome
                        .with_error(native_execution_error(&error))
                        .with_next_action(NextAction::new(
                            "reobserve_before_retry",
                            "Re-observe the selected harnesses before retrying the daemon cycle.",
                        )),
                    changed: false,
                    safe_operations: 0,
                    pending_operations: plan.iter().count() as u64,
                    operations: Vec::new(),
                };
                }
            };
        let mut changed = false;
        let mut safe_operations = 0_u64;
        let mut pending_operations = builder.pending_updates.len() as u64;
        let mut operations = Vec::new();
        for result in report.result.operations().values() {
            let Some(operation) = report.result.plan().get(result.operation_id()) else {
                continue;
            };
            let status = operation_result_status(result.outcome());
            if let Ok(reference) = DaemonOperationRef::new(
                result.operation_id().clone(),
                operation.selector().resource().clone(),
                operation.target().clone(),
                operation.action(),
            ) {
                operations.push(reference);
            }
            outcome = outcome.with_operation(
                crate::OperationOutcome::new(result.operation_id().to_string(), status)
                    .with_field("target", operation.target().as_str())
                    .with_field("resource", operation.selector().resource().to_string())
                    .with_field("action", daemon_action_label(operation.action())),
            );
            if matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            ) {
                safe_operations += 1;
                if operation.action() == OperationAction::PluginUpdate
                    && result.outcome() == &OperationOutcome::Applied
                {
                    changed = true;
                }
            } else {
                pending_operations += 1;
                outcome.result = ResultClass::AttentionRequired;
            }
            if let OperationOutcome::Failed { reason } = result.outcome()
                && let (Some(code), Some(detail)) = (reason.code(), reason.detail())
            {
                outcome =
                    outcome.with_error(ErrorDetail::new(code.to_string(), detail.to_string()));
            }
        }
        if report.result.outcome() == skilltap_core::domain::ApplyOutcome::Succeeded
            && outcome.errors.is_empty()
            && outcome.warnings.is_empty()
        {
            outcome.result = ResultClass::Completed;
        }
        DaemonNativeExecution {
            outcome: outcome
                .with_summary("native_operations", report.result.operations().len() as u64)
                .with_summary("native_changed", changed),
            changed,
            safe_operations,
            pending_operations,
            operations,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn plan_daemon_lifecycle_target(
        &self,
        builder: &mut NativeLifecyclePlanBuilder,
        documents: &StatusDocuments,
        paths: &PlatformPaths,
        native_environment: &BTreeMap<OsString, OsString>,
        process_limits: ProcessLimits,
        json_limits: JsonLimits,
        timestamp: Timestamp,
        kind: NativeLifecycleKind,
        resource: &DesiredResource,
        target: &HarnessId,
        name: &NativeId,
        dependencies: BTreeSet<OperationDependency>,
        outcome: &mut Outcome,
    ) -> Option<OperationId> {
        if self
            .registry
            .adapter(target)
            .is_some_and(|adapter| adapter.conditional_profile().is_some())
        {
            let capability_name = match kind {
                NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
                NativeLifecycleKind::PluginUpdate => "plugin.update",
                NativeLifecycleKind::MarketplaceAdd
                | NativeLifecycleKind::MarketplaceRemove
                | NativeLifecycleKind::PluginInstall
                | NativeLifecycleKind::PluginRemove => "daemon.unsupported",
            };
            let capability = CapabilityId::new(capability_name)
                .expect("static daemon capability identifier is valid");
            match super::conditional_profile::resolve_conditional_profile(
                self.registry,
                &documents.config,
                target,
                resource.scope(),
                paths,
                process_limits,
                json_limits,
                &SystemFileSystem,
            ) {
                Ok(resolved) => {
                    if let Err(error) =
                        super::conditional_profile::require_target_mutation_capability(
                            resolved.as_ref(),
                            &capability,
                            resource.scope(),
                        )
                    {
                        *outcome = outcome
                            .clone()
                            .with_error(error.with_context("target", target.as_str()))
                            .with_next_action(
                                super::conditional_profile::conditional_profile_next_action(),
                            );
                        outcome.result =
                            merge_result(outcome.result, ResultClass::AttentionRequired);
                        return None;
                    }
                }
                Err(error) => {
                    *outcome = outcome
                        .clone()
                        .with_warning(super::conditional_profile::conditional_profile_warning(
                            target,
                            resource.scope(),
                            &error,
                        ))
                        .with_next_action(
                            super::conditional_profile::conditional_profile_next_action(),
                        );
                    outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                    return None;
                }
            }
        }
        let operation_id = lifecycle_operation_id(kind, target, resource.scope(), resource.key());
        let request = match NativeLifecycleSpec::parse(kind, None, Some(name.as_str())) {
            Ok(request) => request,
            Err(_) => {
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    dependencies,
                    outcome,
                );
            }
        };
        // Resolve the managed projection profile before route selection. An
        // exact Unverified profile is a foreground-only declaration path; do
        // not resolve a checkout, build a managed entry/seed, or probe an
        // effective state merely to discover that the daemon cannot apply it.
        if self
            .registry
            .adapter(target)
            .is_some_and(|adapter| adapter.managed_projection().is_some())
            && configured_adapter_profile(
                self.registry,
                &documents.config,
                target,
                NativeProfileRequest {
                    scope: resource.scope(),
                    environment: native_environment,
                    process_limits,
                    json_limits,
                    search_path: std::env::var_os("PATH"),
                    capability_name: "managed.projection",
                },
            )
            .ok()
            .flatten()
            .is_some_and(|profile| profile.capability == CapabilitySupport::Unverified)
        {
            mark_daemon_declaration_pending(builder, resource, target, outcome);
            return None;
        }
        let route = match select_lifecycle_route(LifecycleRouteContext {
            registry: self.registry,
            documents,
            paths,
            target,
            kind,
            request: &request,
            resource,
            scope: resource.scope(),
            environment: native_environment,
            search_path: std::env::var_os("PATH"),
            process_limits,
            json_limits,
            timestamp,
            filesystem: self.managed_filesystem(),
        }) {
            Ok(route) => route,
            Err(error) => {
                *outcome = outcome
                    .clone()
                    .with_warning(Warning::new(error.code.clone(), error.summary.clone()));
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    dependencies,
                    outcome,
                );
            }
        };
        if let LifecycleRoute::Managed(preplanned) = route {
            let managed_profile = configured_adapter_profile(
                self.registry,
                &documents.config,
                target,
                NativeProfileRequest {
                    scope: resource.scope(),
                    environment: native_environment,
                    process_limits,
                    json_limits,
                    search_path: std::env::var_os("PATH"),
                    capability_name: "managed.projection",
                },
            );
            let managed_profile = match managed_profile {
                Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => profile,
                Ok(Some(_)) | Ok(None) | Err(_) => {
                    *outcome = outcome.clone().with_warning(
                        Warning::new(
                            "daemon_managed_projection_unavailable",
                            "The selected managed projection is not mutation-authorized for this target and scope.",
                        )
                        .with_context("target", target.as_str())
                        .with_context("scope", scope_label(resource.scope())),
                    );
                    return self.add_daemon_blocked_operation(
                        builder,
                        resource,
                        target,
                        operation_id,
                        dependencies,
                        outcome,
                    );
                }
            };
            let observed_at = timestamp;
            let planned = if let Some(planned) = preplanned {
                *planned
            } else {
                match plan_managed_lifecycle(
                    self.registry,
                    target,
                    kind,
                    &request,
                    resource,
                    managed_profile,
                    ManagedPlanContext {
                        scope: resource.scope(),
                        documents,
                        paths,
                        timestamp: observed_at,
                        json_limits,
                        filesystem: self.managed_filesystem(),
                        checkout: None,
                    },
                ) {
                    Ok(planned) => planned,
                    Err(error) => {
                        *outcome = outcome
                            .clone()
                            .with_warning(Warning::new(error.code, error.summary));
                        return self.add_daemon_blocked_operation(
                            builder,
                            resource,
                            target,
                            operation_id,
                            dependencies,
                            outcome,
                        );
                    }
                }
            };
            if planned.operation.class() == skilltap_core::domain::OperationClass::Partial {
                mark_daemon_declaration_pending(builder, resource, target, outcome);
                return None;
            }
            let operation = match planned.operation.with_added_dependencies(dependencies) {
                Ok(operation) => operation,
                Err(_) => {
                    return self.add_daemon_blocked_operation(
                        builder,
                        resource,
                        target,
                        operation_id,
                        BTreeSet::new(),
                        outcome,
                    );
                }
            };
            builder.foreign_operations.insert(operation_id.clone());
            builder
                .managed_entries
                .insert(operation_id.clone(), planned.entry);
            if let Some(seed) = planned.seed {
                builder.seeds.insert(resource.key().clone(), seed);
            }
            builder.operations.push(operation);
            return Some(operation_id);
        }
        let profile = configured_native_profile(
            self.registry,
            &documents.config,
            target,
            NativeProfileRequest {
                scope: resource.scope(),
                environment: native_environment,
                process_limits,
                json_limits,
                search_path: std::env::var_os("PATH"),
                capability_name: daemon_capability_name(kind),
            },
        );
        let profile = match profile {
            Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => profile,
            Ok(Some(_)) | Ok(None) | Err(_) => {
                *outcome = outcome.clone().with_warning(
                    Warning::new(
                        "daemon_native_update_unavailable",
                        "The selected native lifecycle update is not mutation-authorized for this target and scope.",
                    )
                    .with_context("target", target.as_str())
                    .with_context("scope", scope_label(resource.scope())),
                );
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    dependencies,
                    outcome,
                );
            }
        };
        let dispatch = NativeLifecycleDispatch::new(
            target.clone(),
            profile.lifecycle,
            request.native_request(resource.scope().clone()),
        );
        let native_arguments = match native_arguments(&dispatch) {
            Ok(arguments) => arguments,
            Err(_) => {
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    dependencies,
                    outcome,
                );
            }
        };
        let arguments = match command_arguments(native_arguments) {
            Ok(arguments) => arguments,
            Err(_) => {
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    dependencies,
                    outcome,
                );
            }
        };
        let before = observe_native_resource_bound(
            &profile.executable_identity,
            native_environment,
            &dispatch,
            process_limits,
            json_limits,
        )
        .unwrap_or(NativeResourceObservation::Indeterminate(
            NativeObservationFailure::CommandFailed,
        ));
        if matches!(&before, NativeResourceObservation::Indeterminate(_)) {
            *outcome = outcome.clone().with_warning(
                Warning::new(
                    "daemon_native_observation_unavailable",
                    "The daemon could not establish fresh native evidence before updating.",
                )
                .with_context("target", target.as_str())
                .with_context("scope", scope_label(resource.scope())),
            );
            return self.add_daemon_blocked_operation(
                builder,
                resource,
                target,
                operation_id,
                dependencies,
                outcome,
            );
        }
        let operation = match native_operation(
            operation_id.clone(),
            target.clone(),
            resource.key().clone(),
            request.operation_action(),
            profile.executable.clone(),
            arguments,
        )
        .and_then(|operation| operation.with_added_dependencies(dependencies))
        {
            Ok(operation) => operation,
            Err(_) => {
                return self.add_daemon_blocked_operation(
                    builder,
                    resource,
                    target,
                    operation_id,
                    BTreeSet::new(),
                    outcome,
                );
            }
        };
        let native_id = request.native_name.clone();
        let seed = native_state_seed(resource, target, &native_id, timestamp, &before);
        builder.seeds.insert(resource.key().clone(), seed);
        builder.native_bindings.push(NativeLifecycleBinding {
            operation_id: operation_id.clone(),
            executable: profile.executable_identity,
            limits: process_limits,
            dispatch,
            before: Some(before),
        });
        builder.operations.push(operation);
        Some(operation_id)
    }

    fn add_daemon_blocked_operation(
        &self,
        builder: &mut NativeLifecyclePlanBuilder,
        resource: &DesiredResource,
        target: &HarnessId,
        operation_id: OperationId,
        dependencies: BTreeSet<OperationDependency>,
        outcome: &mut Outcome,
    ) -> Option<OperationId> {
        let action = match resource.kind() {
            ResourceKind::Marketplace => OperationAction::MarketplaceUpdate,
            ResourceKind::Plugin => OperationAction::PluginUpdate,
            _ => return None,
        };
        let operation = skilltap_core::lifecycle_operation::blocked_native_operation(
            operation_id.clone(),
            target.clone(),
            resource.key().clone(),
            action,
            EvidenceCode::new("daemon.native_update_blocked")
                .expect("static daemon evidence code is valid"),
            EvidenceDetail::new(
                "The daemon could not establish a safe native lifecycle update for this target.",
            )
            .expect("static daemon evidence detail is valid"),
        )
        .and_then(|operation| operation.with_added_dependencies(dependencies));
        match operation {
            Ok(operation) => {
                builder.foreign_operations.insert(operation_id.clone());
                builder.operations.push(operation);
                Some(operation_id)
            }
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_plan_invalid",
                    "The daemon blocked lifecycle operation could not be represented safely.",
                ));
                None
            }
        }
    }

    pub(crate) fn execute_lifecycle_preview(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        kind: ResourceKind,
        source: Option<&str>,
        name: Option<&str>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
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
        let source = source.unwrap_or("not supplied");
        let name = name.unwrap_or("derived by lifecycle adapter");
        let paths = match self.lifecycle_platform_paths() {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The configuration paths could not be resolved for lifecycle planning.",
                ));
            }
        };
        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded lifecycle preview process limits are valid");
        let json_limits = JsonLimits::new(256 * 1024, 64)
            .expect("bounded lifecycle preview JSON limits are valid");
        let capability = CapabilityId::new(match kind {
            ResourceKind::Marketplace => "marketplace.register",
            ResourceKind::Plugin => "plugin.install",
            _ => "skill.install",
        })
        .expect("static lifecycle preview capability is valid");
        let mut operation_count = 0_u64;
        for concrete_scope in &scope.resolved {
            let (authorized_targets, next_outcome) =
                super::conditional_profile::filter_targets_for_capability(
                    self.registry,
                    &documents.config,
                    &targets.resolved,
                    concrete_scope,
                    &paths,
                    process_limits,
                    json_limits,
                    &SystemFileSystem,
                    &capability,
                    outcome,
                );
            outcome = next_outcome;
            let Some(authorized_targets) = authorized_targets else {
                continue;
            };
            for harness in authorized_targets.iter() {
                operation_count += 1;
                let presence = lifecycle_preview_presence(
                    self.registry,
                    &documents,
                    kind,
                    harness,
                    concrete_scope,
                    name,
                );
                let recorded = lifecycle_recorded_state(&documents, kind, concrete_scope, name);
                let status = match (presence.clone(), recorded) {
                    (NativeResourceObservation::Present { .. }, true) => "no_change",
                    (NativeResourceObservation::Present { .. }, false)
                    | (NativeResourceObservation::Missing, true)
                    | (NativeResourceObservation::Missing, false) => "repair",
                    (NativeResourceObservation::Indeterminate(_), _) => "planned",
                };
                outcome = outcome.with_operation(
                    crate::OperationOutcome::new(
                        format!("{command}:{harness}:{}", scope_label(concrete_scope)),
                        status,
                    )
                    .with_field("target", harness.as_str())
                    .with_field("scope", scope_label(concrete_scope))
                    .with_field("source", source)
                    .with_field("name", name)
                    .with_field(
                        "recorded_state",
                        if recorded { "present" } else { "missing" },
                    )
                    .with_field("fresh_state", lifecycle_presence_label(presence)),
                );
            }
        }
        outcome
            .with_summary("operations", operation_count)
            .with_summary("changed", false)
    }

    pub(crate) fn execute_native_lifecycle(
        &self,
        command: &'static str,
        kind: NativeLifecycleKind,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        values: NativeLifecycleValues<'_>,
        acknowledged: bool,
    ) -> Outcome {
        let NativeLifecycleValues {
            source: source_value,
            name: name_value,
        } = values;
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
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

        let update_all = name_value.is_none()
            && matches!(
                kind,
                NativeLifecycleKind::MarketplaceUpdate | NativeLifecycleKind::PluginUpdate
            );
        let request = if update_all {
            None
        } else {
            match NativeLifecycleSpec::parse(kind, source_value, name_value) {
                Ok(request) => Some(request),
                Err(error) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(error);
                }
            }
        };
        let paths = match self.lifecycle_platform_paths() {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let original_inventory = inventory.clone();
        let removal = matches!(
            kind,
            NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::PluginRemove
        );
        let mut operations = Vec::new();
        let mut native_bindings = Vec::new();
        let mut managed_entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let mut target_projection_keys = BTreeSet::new();
        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded lifecycle process limits are valid");
        let json_limits =
            JsonLimits::new(256 * 1024, 64).expect("bounded lifecycle JSON limits are valid");
        let search_path = std::env::var_os("PATH");
        let native_environment = match paths.native_process_environment(search_path.clone()) {
            Ok(environment) => environment,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "native_environment_unavailable",
                    "The bounded native process environment could not be resolved.",
                ));
            }
        };
        let timestamp = Timestamp::from_system_time(std::time::SystemTime::now()).map_err(|_| ());
        let capability_name = match kind {
            NativeLifecycleKind::MarketplaceAdd => "marketplace.register",
            NativeLifecycleKind::MarketplaceRemove => "marketplace.remove",
            NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
            NativeLifecycleKind::PluginInstall => "plugin.install",
            NativeLifecycleKind::PluginRemove => "plugin.remove",
            NativeLifecycleKind::PluginUpdate => "plugin.update",
        };
        let capability = CapabilityId::new(capability_name)
            .expect("static lifecycle capability identifier is valid");
        let mut authorized_targets = Vec::new();

        for concrete_scope in &scope.resolved {
            let (mutating_targets, next_outcome) =
                super::conditional_profile::filter_targets_for_capability(
                    self.registry,
                    &documents.config,
                    &targets.resolved,
                    concrete_scope,
                    &paths,
                    process_limits,
                    json_limits,
                    &SystemFileSystem,
                    &capability,
                    outcome,
                );
            outcome = next_outcome;
            let Some(mutating_targets) = mutating_targets else {
                continue;
            };
            let scope_requests = match request.as_ref() {
                Some(request) => vec![request.clone()],
                None => inventory
                    .resources()
                    .values()
                    .filter(|resource| {
                        resource.scope() == concrete_scope
                            && resource.kind() == native_resource_kind(kind)
                    })
                    .filter_map(|resource| {
                        resource
                            .key()
                            .id()
                            .as_str()
                            .strip_prefix(native_resource_prefix(kind))
                            .and_then(|name| {
                                NativeLifecycleSpec::parse(kind, None, Some(name)).ok()
                            })
                    })
                    .collect::<Vec<_>>(),
            };
            for request in scope_requests {
                // Establish at least one real mutation route before changing
                // desired inventory. Observe-only/file-only candidates have
                // neither an exact native profile nor a managed projection;
                // publishing their desired resource would violate the
                // no-write contract even though route selection later blocks.
                let routable_targets = mutating_targets
                    .iter()
                    .filter(|target_id| {
                        // This is intentionally a static preflight. Native
                        // profile detection is performed once by route
                        // selection; probing here would consume stateful
                        // version fixtures and could change existing
                        // revalidation behavior. A configured binary or a
                        // declared managed projection is enough to defer to
                        // that authoritative route selection.
                        let has_binary = documents
                            .config
                            .harnesses()
                            .get(target_id)
                            .and_then(|policy| policy.binary.as_ref())
                            .is_some();
                        let has_managed_projection = self
                            .registry
                            .adapter(target_id)
                            .is_some_and(|adapter| adapter.managed_projection().is_some());
                        has_binary || has_managed_projection
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let Ok(mutating_targets) = HarnessSet::new(routable_targets) else {
                    outcome.result = ResultClass::AttentionRequired;
                    continue;
                };
                if mutating_targets.iter().next().is_none() {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "native_profile_unavailable",
                            "The selected harness is observe-only for this lifecycle action; no desired or native state was changed.",
                        )
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                    continue;
                }
                authorized_targets.extend(mutating_targets.iter().cloned());
                let resource = if request.is_update() || removal {
                    let key = request.resource_key(concrete_scope).map_err(|_| {
                        ErrorDetail::new(
                            "resource_id_invalid",
                            "The requested native resource identifier is invalid.",
                        )
                    });
                    let key = match key {
                        Ok(key) => key,
                        Err(error) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(error);
                        }
                    };
                    match inventory.resources().get(&key) {
                        Some(existing) => existing.clone(),
                        None => match request.desired_resource(concrete_scope, &mutating_targets) {
                            Ok(resource) => resource,
                            Err(error) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(error);
                            }
                        },
                    }
                } else {
                    let proposed = match request.desired_resource(concrete_scope, &mutating_targets)
                    {
                        Ok(resource) => resource,
                        Err(error) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(error);
                        }
                    };
                    match inventory.resources().get(proposed.key()) {
                        Some(existing) if existing.source() == proposed.source() => {
                            let widened = existing
                                .targets()
                                .iter()
                                .chain(proposed.targets().iter())
                                .cloned()
                                .collect::<Vec<_>>();
                            let widened = HarnessSet::new(widened)
                                .map_err(|_| ())
                                .and_then(|targets| existing.with_targets(targets).map_err(|_| ()));
                            match widened {
                                Ok(resource) => resource,
                                Err(_) => {
                                    outcome.result = ResultClass::Invalid;
                                    return outcome.with_error(ErrorDetail::new(
                                        "inventory_target_union_invalid",
                                        "The existing and requested harness targets could not be combined safely.",
                                    ));
                                }
                            }
                        }
                        _ => proposed,
                    }
                };
                if request.retains_desired() && !request.is_update() {
                    let next_inventory = match inventory.resources().get(resource.key()) {
                        Some(existing)
                            if existing
                                .with_targets(resource.targets().clone())
                                .is_ok_and(|projected| projected == resource) =>
                        {
                            inventory.replace_resource(resource.clone())
                        }
                        _ => inventory.with_resource(resource.clone()),
                    };
                    match next_inventory {
                        Ok(next) => inventory = next,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            return outcome
                            .with_error(ErrorDetail::new(
                                "inventory_resource_conflict",
                                "The requested resource conflicts with an existing desired definition.",
                            ))
                            .with_next_action(NextAction::new(
                                "inspect_inventory",
                                "Inspect the existing resource definition before retrying.",
                            ));
                        }
                    }
                }

                let warning_count = outcome.warnings.len();
                let mut native_ids: BTreeMap<HarnessId, NativeId> = documents
                    .state
                    .as_ref()
                    .and_then(|state| state.resources().get(resource.key()))
                    .map(|state| {
                        state
                            .targets()
                            .iter()
                            .filter_map(|(harness, target)| {
                                target
                                    .native_id()
                                    .cloned()
                                    .map(|native| (harness.clone(), native))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut native_route_selected = false;
                for target_id in mutating_targets.iter() {
                    let route = match select_lifecycle_route(LifecycleRouteContext {
                        registry: self.registry,
                        documents: &documents,
                        paths: &paths,
                        target: target_id,
                        kind,
                        request: &request,
                        resource: &resource,
                        scope: concrete_scope,
                        environment: &native_environment,
                        search_path: search_path.clone(),
                        process_limits,
                        json_limits,
                        timestamp: match timestamp {
                            Ok(timestamp) => timestamp,
                            Err(()) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(ErrorDetail::new(
                                    "clock_unavailable",
                                    "The operation timestamp could not be recorded safely.",
                                ));
                            }
                        },
                        filesystem: self.managed_filesystem(),
                    }) {
                        Ok(route) => route,
                        Err(error) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_error(error);
                            continue;
                        }
                    };
                    if let LifecycleRoute::Managed(preplanned) = route {
                        let desired_here = documents.inventory.as_ref().is_some_and(|inventory| {
                            inventory
                                .resources()
                                .get(resource.key())
                                .is_some_and(|desired| desired.targets().contains(target_id))
                        });
                        let owned_here = documents.state.as_ref().is_some_and(|state| {
                            state
                                .resources()
                                .get(resource.key())
                                .and_then(|state| state.target(target_id))
                                .is_some_and(|target| target.ownership() == Ownership::Skilltap)
                        });
                        if removal && !desired_here && !owned_here {
                            continue;
                        }
                        let planned = if let Some(planned) = preplanned {
                            *planned
                        } else {
                            let managed_profile = configured_adapter_profile(
                                self.registry,
                                &documents.config,
                                target_id,
                                NativeProfileRequest {
                                    scope: concrete_scope,
                                    environment: &native_environment,
                                    process_limits,
                                    json_limits,
                                    search_path: search_path.clone(),
                                    capability_name: "managed.projection",
                                },
                            );
                            let managed_profile = match managed_profile {
                                Ok(Some(profile))
                                    if matches!(
                                        profile.capability,
                                        CapabilitySupport::Supported
                                            | CapabilitySupport::Unverified
                                    ) =>
                                {
                                    profile
                                }
                                Ok(Some(_)) | Ok(None) => {
                                    outcome.result = ResultClass::AttentionRequired;
                                    outcome = outcome.with_warning(
                                        Warning::new(
                                            "native_capability_unverified",
                                            "The selected managed projection is not verified for mutation.",
                                        )
                                        .with_context("harness", target_id.as_str())
                                        .with_context("scope", scope_label(concrete_scope)),
                                    );
                                    continue;
                                }
                                Err(error) => {
                                    outcome.result = ResultClass::AttentionRequired;
                                    let binary = documents
                                        .config
                                        .harnesses()
                                        .get(target_id)
                                        .and_then(|policy| policy.binary.as_ref())
                                        .map(|binary| binary.as_str())
                                        .unwrap_or_else(|| target_id.as_str());
                                    let diagnostic =
                                        detection_diagnostic(&error, target_id.as_str(), binary);
                                    outcome = outcome
                                        .with_warning(diagnostic.warning)
                                        .with_next_action(diagnostic.next_action);
                                    continue;
                                }
                            };
                            match plan_managed_lifecycle(
                                self.registry,
                                target_id,
                                kind,
                                &request,
                                &resource,
                                managed_profile,
                                ManagedPlanContext {
                                    scope: concrete_scope,
                                    documents: &documents,
                                    paths: &paths,
                                    timestamp: timestamp.expect("timestamp validated above"),
                                    json_limits,
                                    filesystem: self.managed_filesystem(),
                                    checkout: None,
                                },
                            ) {
                                Ok(planned) => planned,
                                Err(error) => {
                                    outcome.result = ResultClass::AttentionRequired;
                                    outcome = outcome.with_error(error);
                                    continue;
                                }
                            }
                        };
                        let operation_id = planned.operation.id().clone();
                        operations.push(planned.operation);
                        managed_entries.insert(operation_id, planned.entry);
                        if let Some(seed) = planned.seed {
                            seeds.insert(resource.key().clone(), seed);
                        }
                        if removal {
                            target_projection_keys.insert(resource.key().clone());
                        }
                        continue;
                    }
                    let profile = configured_native_profile(
                        self.registry,
                        &documents.config,
                        target_id,
                        NativeProfileRequest {
                            scope: concrete_scope,
                            environment: &native_environment,
                            process_limits,
                            json_limits,
                            search_path: search_path.clone(),
                            capability_name: match kind {
                                NativeLifecycleKind::MarketplaceAdd => "marketplace.register",
                                NativeLifecycleKind::MarketplaceRemove => "marketplace.remove",
                                NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
                                NativeLifecycleKind::PluginInstall => "plugin.install",
                                NativeLifecycleKind::PluginRemove => "plugin.remove",
                                NativeLifecycleKind::PluginUpdate => "plugin.update",
                            },
                        },
                    );
                    let profile = match profile {
                        Ok(Some(profile)) => profile,
                        Err(error) => {
                            outcome.result = ResultClass::AttentionRequired;
                            let binary = documents
                                .config
                                .harnesses()
                                .get(target_id)
                                .and_then(|policy| policy.binary.as_ref())
                                .map(|binary| binary.as_str())
                                .unwrap_or_else(|| target_id.as_str());
                            let diagnostic =
                                detection_diagnostic(&error, target_id.as_str(), binary);
                            outcome = outcome
                                .with_resource(
                                    OutputEntry::new(
                                        format!("{}:{}", target_id, resource.key()),
                                        "detection_failed",
                                    )
                                    .with_field("target", target_id.as_str())
                                    .with_field("scope", scope_label(concrete_scope)),
                                )
                                .with_warning(diagnostic.warning)
                                .with_next_action(diagnostic.next_action);
                            continue;
                        }
                        Ok(None) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome
                                .with_resource(
                                    OutputEntry::new(
                                        format!("{}:{}", target_id, resource.key()),
                                        "mutation_unavailable",
                                    )
                                    .with_field("target", target_id.as_str())
                                    .with_field("scope", scope_label(concrete_scope)),
                                )
                                .with_warning(
                                    Warning::new(
                                        "native_profile_unavailable",
                                        "The selected harness is not mutation-authorized for this lifecycle action.",
                                    )
                                    .with_context("harness", target_id.as_str()),
                                );
                            continue;
                        }
                    };
                    if profile.capability != CapabilitySupport::Supported {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "native_capability_unverified",
                                "The selected harness capability is not verified for mutation.",
                            )
                            .with_context("harness", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    native_route_selected = true;
                    let native_request = request.native_request(concrete_scope.clone());
                    let native_dispatch = NativeLifecycleDispatch::new(
                        profile.target.clone(),
                        profile.lifecycle,
                        native_request,
                    );
                    let arguments = match native_arguments(&native_dispatch) {
                        Ok(arguments) => arguments,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_warning(
                            Warning::new(
                                "native_scope_unsupported",
                                "The selected harness has no verified lifecycle command for this scope.",
                            )
                            .with_context("harness", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                            continue;
                        }
                    };
                    let operation_id =
                        lifecycle_operation_id(kind, target_id, concrete_scope, resource.key());
                    native_ids.insert(target_id.clone(), request.native_name.clone());
                    let journal_has_attempt = previously_attempted(
                        documents.state.as_ref(),
                        resource.key(),
                        target_id,
                        &operation_id,
                    );
                    let requires_fresh_precondition = journal_has_attempt || removal;
                    let fresh_presence = if requires_fresh_precondition {
                        observe_native_resource_bound(
                            &profile.executable_identity,
                            &native_environment,
                            &native_dispatch,
                            process_limits,
                            json_limits,
                        )
                        .unwrap_or(
                            NativeResourceObservation::Indeterminate(
                                NativeObservationFailure::CommandFailed,
                            ),
                        )
                    } else {
                        NativeResourceObservation::Indeterminate(
                            NativeObservationFailure::CommandFailed,
                        )
                    };
                    if requires_fresh_precondition {
                        match (fresh_presence, removal, journal_has_attempt) {
                            (NativeResourceObservation::Present { .. }, false, true)
                            | (NativeResourceObservation::Missing, true, _) => {
                                if journal_has_attempt {
                                    let command_arguments = match command_arguments(arguments) {
                                        Ok(arguments) => arguments,
                                        Err(_) => {
                                            outcome.result = ResultClass::Invalid;
                                            return outcome.with_error(ErrorDetail::new(
                                                "native_argument_encoding",
                                                "The native lifecycle arguments could not be represented safely.",
                                            ));
                                        }
                                    };
                                    let operation = match skilltap_core::lifecycle_operation::native_noop_operation(
                                        operation_id.clone(),
                                        target_id.clone(),
                                        resource.key().clone(),
                                        request.operation_action(),
                                        profile.executable,
                                        command_arguments,
                                    ) {
                                        Ok(operation) => operation,
                                        Err(_) => {
                                            outcome.result = ResultClass::Invalid;
                                            return outcome.with_error(ErrorDetail::new(
                                                "operation_contract_invalid",
                                                "The verified native no-change operation could not be constructed safely.",
                                            ));
                                        }
                                    };
                                    operations.push(operation);
                                    native_bindings.push(NativeLifecycleBinding {
                                        operation_id,
                                        executable: profile.executable_identity,
                                        limits: process_limits,
                                        dispatch: native_dispatch,
                                        before: None,
                                    });
                                } else {
                                    outcome = outcome.with_operation(
                                        crate::OperationOutcome::new(
                                            operation_id.to_string(),
                                            "no_change",
                                        )
                                        .with_field("target", target_id.as_str())
                                        .with_field("scope", scope_label(concrete_scope)),
                                    );
                                }
                                continue;
                            }
                            (NativeResourceObservation::Missing, false, true)
                            | (NativeResourceObservation::Present { .. }, true, _) => {}
                            (NativeResourceObservation::Indeterminate(failure), _, _) => {
                                outcome.result = ResultClass::AttentionRequired;
                                outcome = outcome
                                    .with_operation(
                                        crate::OperationOutcome::new(
                                            operation_id.to_string(),
                                            "observation_required",
                                        )
                                        .with_field("target", target_id.as_str())
                                        .with_field("scope", scope_label(concrete_scope)),
                                    )
                                    .with_warning(
                                        Warning::new(failure.diagnostic_code(), failure.summary())
                                            .with_context("harness", target_id.as_str())
                                            .with_context("scope", scope_label(concrete_scope)),
                                    )
                                    .with_next_action(
                                        NextAction::new(
                                            "reobserve_before_retry",
                                            "Restore native list observation before retrying; skilltap did not repeat the mutation.",
                                        )
                                        .with_command(format!(
                                            "skilltap status --target {} --json",
                                            target_id.as_str()
                                        )),
                                    );
                                continue;
                            }
                            (_, false, false) => {}
                        }
                    }
                    let command_arguments = match command_arguments(arguments) {
                        Ok(arguments) => arguments,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "native_argument_encoding",
                                "The native lifecycle arguments could not be represented safely.",
                            ));
                        }
                    };
                    let operation = match native_operation(
                        operation_id.clone(),
                        target_id.clone(),
                        resource.key().clone(),
                        request.operation_action(),
                        profile.executable,
                        command_arguments,
                    ) {
                        Ok(operation) => operation,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The native lifecycle operation could not be constructed safely.",
                            ));
                        }
                    };
                    operations.push(operation);
                    native_bindings.push(NativeLifecycleBinding {
                        operation_id,
                        executable: profile.executable_identity,
                        limits: process_limits,
                        dispatch: native_dispatch,
                        before: None,
                    });
                }
                if native_route_selected && !native_ids.is_empty() {
                    let observed_at = match timestamp {
                        Ok(timestamp) => timestamp,
                        Err(()) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "clock_unavailable",
                                "The operation timestamp could not be recorded safely.",
                            ));
                        }
                    };
                    let existing = documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(resource.key()));
                    let mut bindings = existing
                        .map(|state| state.targets().clone())
                        .unwrap_or_default();
                    for (harness, native_id) in native_ids {
                        if !targets.resolved.contains(&harness) {
                            continue;
                        }
                        let source = resource
                            .source()
                            .cloned()
                            .or_else(|| request.source.clone())
                            .or_else(|| {
                                existing
                                    .and_then(|state| state.target(&harness))
                                    .and_then(|target| target.source().cloned())
                            });
                        let binding = match TargetResourceState::new(
                            harness.clone(),
                            Some(native_id),
                            Provenance::Native,
                            Ownership::Harness,
                            source,
                            None,
                            None,
                            None,
                            None,
                            observed_at,
                            None,
                        ) {
                            Ok(binding) => binding,
                            Err(_) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(ErrorDetail::new(
                                    "state_seed_invalid",
                                    "The native lifecycle target evidence was invalid.",
                                ));
                            }
                        };
                        bindings.insert(harness, binding);
                    }
                    let native_state =
                        match ResourceState::new(resource.key().clone(), bindings.into_values()) {
                            Ok(state) => state,
                            Err(_) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(ErrorDetail::new(
                                    "state_seed_invalid",
                                    "The native lifecycle state seed was invalid.",
                                ));
                            }
                        };
                    seeds.insert(resource.key().clone(), native_state);
                }
                if removal
                    && inventory.resources().contains_key(resource.key())
                    && outcome.warnings.len() == warning_count
                {
                    target_projection_keys.insert(resource.key().clone());
                }
            }
        }

        let authorized_targets = HarnessSet::new(authorized_targets).ok();
        if authorized_targets.is_none() {
            return outcome
                .with_summary("operations", 0_u64)
                .with_summary("changed", false);
        }
        let authorized_targets = authorized_targets.expect("checked above");
        let inventory_changed = inventory != original_inventory;
        if operations.is_empty() && !outcome.errors.is_empty() {
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false);
        }
        if inventory_changed && !removal && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The desired inventory could not be published before the native operation.",
            ));
        }
        if operations.is_empty() {
            if removal {
                inventory = match project_inventory_targets(
                    &inventory,
                    &target_projection_keys,
                    &authorized_targets,
                ) {
                    Ok(inventory) => inventory,
                    Err(()) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "inventory_publish_failed",
                            "The desired inventory could not be updated safely.",
                        ));
                    }
                };
            }
            if removal
                && inventory != original_inventory
                && self.inventory.replace(&inventory).is_err()
            {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The desired inventory could not be published after the native removal.",
                ));
            }
            if let Err(()) = seed_state_if_missing(self.state, &seeds) {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The native lifecycle state could not be recorded safely.",
                ));
            }
            if removal
                && project_state_targets_after_remove(
                    self.state,
                    &target_projection_keys,
                    &authorized_targets,
                )
                .is_err()
            {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_publish_failed",
                    "The native lifecycle state could not be updated safely.",
                ));
            }
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false)
                .with_next_action(NextAction::new(
                    "inspect_status",
                    "Inspect status if the native resource may have drifted externally.",
                ));
        }

        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The native lifecycle operation plan was invalid.",
                ));
            }
        };
        let foreign_operations = managed_entries.keys().cloned().collect::<Vec<_>>();
        let native_port =
            NativeLifecyclePort::new_with_environment(native_bindings, native_environment.clone())
                .with_foreign_operations(foreign_operations);
        let port = HybridLifecyclePort {
            native: native_port,
            managed: ManagedLifecyclePort {
                filesystem: self.managed_filesystem(),
                entries: managed_entries,
                registry: self.registry,
                config: &documents.config,
                environment: &native_environment,
                search_path: search_path.clone(),
                process_limits,
                json_limits,
            },
        };
        let acknowledged_omissions = seeds
            .values()
            .flat_map(|resource| resource.targets().values())
            .flat_map(TargetResourceState::managed_projections)
            .filter_map(|projection| match projection {
                ManagedProjection::Omitted { id, consequence } => {
                    Some((id.as_str().to_owned(), consequence.as_str().to_owned()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
            seeds,
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "lock_path_invalid",
                    "The skilltap configuration lock path is invalid.",
                ));
            }
        };
        let acknowledgments = if acknowledged {
            ExecutionAcknowledgments::foreground_all(&plan)
        } else {
            ExecutionAcknowledgments::default()
        };
        let report = match execute_plan_with_acknowledgments(
            &SystemConfigurationLock,
            &lock_path,
            &port,
            &journal,
            &plan,
            &acknowledgments,
        ) {
            Ok(report) => report,
            Err(error) => {
                outcome.result = ResultClass::AttentionRequired;
                let action = NextAction::new(
                    "reobserve_before_retry",
                    "Re-observe the selected harness before retrying the lifecycle operation.",
                );
                return outcome
                    .with_error(native_execution_error(&error).with_next_action(action.clone()))
                    .with_next_action(action);
            }
        };
        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => {
                NativeObservation::run(self.registry, &documents, &scope, &targets)
            }
        };
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        for (id, consequence) in acknowledged_omissions {
            outcome = outcome.with_resource(
                OutputEntry::new(format!("omitted:{id}"), "omitted")
                    .with_field("component", id)
                    .with_field("consequence", consequence),
            );
        }
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        for action in observation.next_actions.iter().cloned() {
            outcome = outcome.with_next_action(action);
        }
        if observation.failed_targets > 0 {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "post_mutation_observation_incomplete",
                "The native operation completed, but fresh post-mutation observation was incomplete.",
            ));
        }
        for result in report.result.operations().values() {
            let status = operation_result_status(result.outcome());
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                status,
            ));
            if matches!(
                result.outcome(),
                OperationOutcome::Failed { .. }
                    | OperationOutcome::Blocked { .. }
                    | OperationOutcome::SkippedDependency { .. }
                    | OperationOutcome::Pending
            ) {
                outcome.result = ResultClass::AttentionRequired;
            }
            if !acknowledged
                && report
                    .result
                    .plan()
                    .get(result.operation_id())
                    .is_some_and(|operation| {
                        operation.class() == skilltap_core::domain::OperationClass::Partial
                    })
                && matches!(result.outcome(), OperationOutcome::Blocked { .. })
            {
                outcome = outcome
                    .with_error(ErrorDetail::new(
                        "partial_operation_requires_acknowledgment",
                        "The managed plan is partial; rerun with `--yes` to accept its exact reported consequences.",
                    ))
                    .with_next_action(
                        NextAction::new(
                            "acknowledge_partial_operation",
                            "Review the plan and rerun the same command with `--yes`.",
                        )
                        .with_command("skilltap sync --yes"),
                    );
            }
            if let OperationOutcome::Failed { reason } = result.outcome()
                && let (Some(code), Some(detail)) = (reason.code(), reason.detail())
            {
                let action = NextAction::new(
                    "reobserve_before_retry",
                    "Re-observe the selected harness before retrying the lifecycle operation.",
                )
                .with_command("skilltap status --json");
                outcome = outcome
                    .with_error(
                        ErrorDetail::new(code.to_string(), detail.to_string())
                            .with_next_action(action.clone()),
                    )
                    .with_next_action(action);
            }
        }
        if removal
            && report.result.operations().values().all(|result| {
                matches!(
                    result.outcome(),
                    OperationOutcome::Applied | OperationOutcome::NoChange
                )
            })
        {
            inventory = match project_inventory_targets(
                &inventory,
                &target_projection_keys,
                &targets.resolved,
            ) {
                Ok(inventory) => inventory,
                Err(()) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_publish_failed",
                        "The desired inventory could not be updated safely.",
                    ));
                }
            };
            if inventory != original_inventory && self.inventory.replace(&inventory).is_err() {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The desired inventory could not be updated safely.",
                ));
            }
            if project_state_targets_after_remove(
                self.state,
                &target_projection_keys,
                &targets.resolved,
            )
            .is_err()
            {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_publish_failed",
                    "The native lifecycle state could not be updated safely.",
                ));
            }
        }
        let successful = report.result.operations().values().all(|result| {
            matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            )
        });
        if removal && inventory_changed && successful && self.inventory.replace(&inventory).is_err()
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The desired inventory could not be published after the native removal.",
            ));
        }
        if observation.failed_targets == 0
            && successful
            && outcome.errors.is_empty()
            && outcome.warnings.is_empty()
        {
            outcome.result = ResultClass::Completed;
        }
        outcome = outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed);
        if successful {
            outcome = outcome.with_next_action(NextAction::new(
                "verify_status",
                "Run status to verify the fresh native observation and recorded state.",
            ));
        }
        outcome
    }

    pub(crate) fn execute_skill_install(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        acknowledged: bool,
        request: SkillInstallRequest<'_>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
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
                return outcome.with_error(ErrorDetail::new(
                    "no_enabled_harnesses",
                    "No harness is enabled in skilltap configuration.",
                ));
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        // Resolve standalone skill mutation authority before resolving the
        // source. A missing local source may otherwise trigger a Git checkout
        // into the managed source store even though every selected candidate
        // is observe-only and the command must be zero-write.
        let project_only = scope
            .resolved
            .iter()
            .all(|scope| matches!(scope, Scope::Project(_)));
        let file_only_candidates = !project_only
            && targets.resolved.iter().all(|target_id| {
                let has_binary = documents
                    .config
                    .harnesses()
                    .get(target_id)
                    .and_then(|policy| policy.binary.as_ref())
                    .is_some();
                let has_managed_projection = self
                    .registry
                    .adapter(target_id)
                    .is_some_and(|adapter| adapter.managed_projection().is_some());
                !has_binary && !has_managed_projection
            });
        if file_only_candidates {
            return outcome.with_warning(Warning::new(
                "skill_mutation_unavailable",
                "The selected harness is observe-only and has no documented standalone skill mutation route; no files were written.",
            ));
        }
        let mut profile_targets = None;
        let mut conditional_process_limits = None;
        let mut conditional_json_limits = None;
        if !project_only {
            let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
                Ok(paths) => paths,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "platform_paths_unavailable",
                        "The skilltap configuration paths could not be resolved.",
                    ));
                }
            };
            let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
                .expect("bounded conditional profile process limits are valid");
            let json_limits = JsonLimits::new(256 * 1024, 64)
                .expect("bounded conditional profile JSON limits are valid");
            let search_path = std::env::var_os("PATH");
            let environment = match paths.native_process_environment(search_path.clone()) {
                Ok(environment) => environment,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "native_environment_unavailable",
                        "The bounded native process environment could not be resolved.",
                    ));
                }
            };
            let mut profile_target_ids = Vec::new();
            for target_id in targets.resolved.iter() {
                match configured_adapter_profile(
                    self.registry,
                    &documents.config,
                    target_id,
                    NativeProfileRequest {
                        scope: &Scope::Global,
                        environment: &environment,
                        process_limits,
                        json_limits,
                        search_path: search_path.clone(),
                        capability_name: if command == "skill update" || command == "sync" {
                            "skill.update"
                        } else {
                            "skill.install"
                        },
                    },
                ) {
                    Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => {
                        let skill_capability = CapabilityId::new("component.skill")
                            .expect("static component skill capability is valid");
                        match profile.profile.mutation_support(
                            &Scope::Global,
                            &skill_capability,
                        ) {
                            Some(CapabilitySupport::Supported) => {
                                profile_target_ids.push(target_id.clone())
                            }
                            Some(CapabilitySupport::Unverified)
                                if profile
                                    .declaration_contract
                                    .as_ref()
                                    .is_some_and(|contract| {
                                        contract.covers(&BTreeSet::from([
                                            skilltap_core::mutation_authority::ManagedSurfaceKind::CompleteSkillTree,
                                        ]))
                                    }) && acknowledged =>
                            {
                                profile_target_ids.push(target_id.clone());
                                outcome = outcome.with_warning(
                                    Warning::new(
                                        "skill_effective_unverified",
                                        "The complete skill tree will be written, but Copilot skill loading remains unverified.",
                                    )
                                    .with_context("harness", target_id.as_str()),
                                );
                            }
                            _ => {
                                outcome = outcome.with_warning(
                                    Warning::new(
                                        "skill_mutation_unavailable",
                                        "The selected harness profile requires explicit declaration acknowledgment for standalone skill mutation; no files were written for it.",
                                    )
                                    .with_context("harness", target_id.as_str()),
                                );
                            }
                        }
                    }
                    Ok(Some(profile)) => {
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_mutation_unavailable",
                                "The selected harness profile is not verified for standalone skill mutation; no files were written for it.",
                            )
                            .with_context("harness", target_id.as_str())
                            .with_context("support", format!("{:?}", profile.capability)),
                        );
                    }
                    Ok(None) | Err(_) => {
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_mutation_unavailable",
                                "The selected harness profile could not be verified for standalone skill mutation; no files were written for it.",
                            )
                            .with_context("harness", target_id.as_str()),
                        );
                    }
                }
            }
            profile_targets = HarnessSet::new(profile_target_ids).ok();
            conditional_process_limits = Some(process_limits);
            conditional_json_limits = Some(json_limits);
        }
        let project_profile_targets = if project_only {
            let Some(project) = scope.resolved.iter().find_map(|scope| match scope {
                Scope::Project(project) => Some(project.clone()),
                Scope::Global => None,
            }) else {
                return outcome;
            };
            let project_paths = match self.lifecycle_platform_paths() {
                Ok(paths) => paths,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "platform_paths_unavailable",
                        "The skilltap configuration paths could not be resolved for project skill preflight.",
                    ));
                }
            };
            let project_process_limits =
                ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
                    .expect("bounded project skill preflight process limits are valid");
            let project_json_limits = JsonLimits::new(256 * 1024, 64)
                .expect("bounded project skill preflight JSON limits are valid");
            let search_path = std::env::var_os("PATH");
            let project_environment = match project_paths.native_process_environment(search_path) {
                Ok(environment) => environment,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "native_environment_unavailable",
                        "The bounded native process environment could not be resolved for project skill preflight.",
                    ));
                }
            };
            let capability_name = if command == "skill update" || command == "sync" {
                "skill.update"
            } else {
                "skill.install"
            };
            let (profile_targets, next_outcome) =
                super::project_skills::preflight_project_skill_route(
                    self,
                    &documents.config,
                    &targets,
                    &project,
                    acknowledged,
                    &project_paths,
                    &project_environment,
                    project_process_limits,
                    project_json_limits,
                    capability_name,
                    outcome,
                );
            outcome = next_outcome;
            let Some(profile_targets) = profile_targets else {
                return outcome
                    .with_summary("operations", 0_u64)
                    .with_summary("changed", false);
            };
            Some(profile_targets)
        } else {
            None
        };
        let locator = match SourceLocator::new(request.source) {
            Ok(locator) => locator,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "invalid_skill_source",
                    "The explicit skill source is invalid.",
                ));
            }
        };
        if is_option_like_git_value(locator.as_str()) {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "invalid_skill_source",
                "The explicit skill source must not begin with `-`.",
            ));
        }
        let requested_revision = match request.requested_revision {
            Some(value) => match skilltap_core::domain::RequestedRevision::new(value) {
                Ok(value) => Some(value),
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_requested_revision",
                        "The requested Git revision is invalid.",
                    ));
                }
            },
            None => None,
        };
        if requested_revision
            .as_ref()
            .is_some_and(|revision| is_option_like_git_value(revision.as_str()))
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "invalid_requested_revision",
                "The requested Git revision must not begin with `-`.",
            ));
        }
        let subdirectory = match request.subdirectory {
            Some(value) => match skilltap_core::domain::RelativeArtifactPath::new(value) {
                Ok(value) => Some(value),
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_skill_subdirectory",
                        "The skill source subdirectory is invalid.",
                    ));
                }
            },
            None => None,
        };
        let (source_root, source_kind, git_commit) = match AbsolutePath::new(locator.as_str()) {
            Ok(path) => match append_skill_subdirectory(path, subdirectory.as_ref()) {
                Some(path) => (path, SourceKind::Local, None),
                None => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_skill_subdirectory",
                        "The skill source subdirectory could not be joined safely.",
                    ));
                }
            },
            Err(_) => {
                let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
                    Ok(paths) => paths,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "platform_paths_unavailable",
                            "The skilltap configuration paths could not be resolved.",
                        ));
                    }
                };
                match resolve_git_skill_source(
                    &paths,
                    &locator,
                    requested_revision.as_ref(),
                    subdirectory.as_ref(),
                ) {
                    Ok(resolved) => (resolved.root, SourceKind::Git, Some(resolved.commit)),
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        return outcome
                            .with_warning(Warning::new(
                                "git_skill_source_unavailable",
                                "The Git skill source could not be cloned and checked out safely.",
                            ))
                            .with_next_action(NextAction::new(
                                "verify_git_source",
                                "Verify the Git source, revision, and credentials before retrying.",
                            ));
                    }
                }
            }
        };
        let limits =
            ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
                .expect("bounded skill tree limits are valid");
        let filesystem = SystemFileSystem;
        let source_snapshot = match SystemExternalTreeObserver
            .observe(&ExternalTreeRequest::new(source_root.clone(), limits))
            .and_then(|snapshot| snapshot.without_top_level_directory(".git", limits))
        {
            Ok(snapshot) => snapshot,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_source_unavailable",
                    "The explicit local skill directory could not be observed safely.",
                ));
            }
        };
        let skill = match ValidatedSkillTree::validate(&source_snapshot) {
            Ok(skill) => skill,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_tree_invalid",
                    "The skill source must be a complete directory with a top-level SKILL.md.",
                ));
            }
        };
        let mut compatibility_label = "compatible";
        let mut partial_targets = BTreeSet::new();
        for compatibility in SkillCompatibility::evaluate(&skill, &targets.resolved) {
            match (compatibility.class(), compatibility.loadability()) {
                (CompatibilityClass::Compatible, SkillLoadability::Loadable) => {}
                (CompatibilityClass::Incompatible, SkillLoadability::Blocked) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_incompatible",
                            "The skill frontmatter is not loadable by the selected harness.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                (CompatibilityClass::Unknown, SkillLoadability::Unknown) => {
                    compatibility_label = "warning";
                    partial_targets.insert(compatibility.target().clone());
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_frontmatter_warning",
                            "The skill is loadable but its frontmatter is not fully strict.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                _ => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_incompatible",
                            "The skill frontmatter is not loadable by the selected harness.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
            }
        }
        let name = match request.name {
            Some(name) => NativeId::new(name).map_err(|_| ()).ok(),
            None => derive_skill_name(&locator, subdirectory.as_ref()),
        };
        let Some(name) = name else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_required",
                "The skill name could not be derived; provide --name.",
            ));
        };
        if request.name.is_some()
            && !request.preserve_name
            && skill.declared_name().as_ref() != Some(&name)
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_mismatch",
                "The supplied --name must match the SKILL.md frontmatter name.",
            ));
        }
        let destination = match skill_relative_destination(&name) {
            Some(destination) => destination,
            None => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name cannot be used as a safe directory component.",
                ));
            }
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let source = Source::new_with_subdirectory(
            source_kind,
            locator,
            requested_revision,
            subdirectory.clone(),
        )
        .map_err(|_| ())
        .ok();
        let Some(source) = source else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "invalid_skill_source",
                "The local skill source could not be represented safely.",
            ));
        };
        let update_intent = if source
            .requested_revision()
            .and_then(|revision| GitCommit::new(revision.as_str()).ok())
            .is_some()
        {
            UpdateIntent::Pinned
        } else {
            UpdateIntent::Track
        };
        if scope
            .resolved
            .iter()
            .all(|scope| matches!(scope, Scope::Project(_)))
        {
            return super::project_skills::execute_project_skill_install(
                self,
                command,
                &scope,
                &targets,
                acknowledged,
                name,
                skill,
                source,
                update_intent,
                git_commit,
                project_profile_targets.expect("project skill preflight established targets"),
                paths,
                outcome,
            );
        }
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let mut old_revision = None;
        let conditional_process_limits = conditional_process_limits
            .expect("global skill mutation preflight established process limits");
        let conditional_json_limits = conditional_json_limits
            .expect("global skill mutation preflight established JSON limits");
        let mutation_capability_name = if command == "skill update" || command == "sync" {
            "skill.update"
        } else {
            "skill.install"
        };
        let timestamp = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(timestamp) => timestamp,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "clock_unavailable",
                    "The skill operation timestamp could not be recorded safely.",
                ));
            }
        };
        for concrete_scope in &scope.resolved {
            let (mutating_targets, next_outcome) =
                super::conditional_profile::filter_targets_for_capability(
                    self.registry,
                    &documents.config,
                    &targets.resolved,
                    concrete_scope,
                    &paths,
                    conditional_process_limits,
                    conditional_json_limits,
                    &filesystem,
                    &CapabilityId::new(mutation_capability_name)
                        .expect("static skill capability is valid"),
                    outcome,
                );
            outcome = next_outcome;
            let Some(mutating_targets) = mutating_targets else {
                continue;
            };
            let Some(profile_targets) = profile_targets.as_ref() else {
                outcome = outcome.with_warning(Warning::new(
                    "skill_mutation_unavailable",
                    "No verified standalone skill mutation profile is available; no files were written.",
                ));
                continue;
            };
            let authorized_target_ids = mutating_targets
                .iter()
                .filter(|target| profile_targets.contains(target))
                .cloned()
                .collect::<Vec<_>>();
            let Some(mutating_targets) = HarnessSet::new(authorized_target_ids).ok() else {
                outcome = outcome.with_warning(Warning::new(
                    "skill_mutation_unavailable",
                    "No selected harness has a verified standalone skill mutation profile; no files were written.",
                ));
                continue;
            };
            let key = ResourceKey::new(
                match ResourceId::new(format!("skill:{}", name.as_str())) {
                    Ok(id) => id,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "skill_name_invalid",
                            "The skill name cannot be represented as a resource identifier.",
                        ));
                    }
                },
                concrete_scope.clone(),
            );
            if old_revision.is_none() {
                old_revision = documents
                    .state
                    .as_ref()
                    .and_then(|state| state.resources().get(&key))
                    .and_then(|state| {
                        profile_targets
                            .iter()
                            .find_map(|target| state.target(target))
                    })
                    .and_then(|target| target.installed_revision())
                    .cloned();
            }
            let desired_targets = inventory
                .resources()
                .get(&key)
                .map(|resource| resource.targets().clone())
                .unwrap_or_else(|| mutating_targets.clone());
            let desired = match DesiredResource::new(
                key.clone(),
                ResourceKind::StandaloneSkill,
                desired_targets,
                DesiredOrigin::Direct,
                Some(source.clone()),
                update_intent,
                ComponentGraph::new([]).expect("empty component graph is valid"),
                BTreeMap::new(),
                BTreeMap::new(),
                BTreeSet::new(),
            ) {
                Ok(desired) => desired,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "skill_resource_invalid",
                        "The skill resource could not be represented safely.",
                    ));
                }
            };
            inventory = match inventory.with_resource(desired) {
                Ok(inventory) => inventory,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_resource_conflict",
                        "The requested skill conflicts with an existing desired definition.",
                    ));
                }
            };
            let mut native_ids: BTreeMap<HarnessId, NativeId> = BTreeMap::new();
            let destinations = match skill_destinations(
                self.registry,
                &paths,
                concrete_scope,
                &mutating_targets,
                &destination,
            ) {
                Some(destinations) => destinations,
                None => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "skill_destination_invalid",
                        "The selected harness skill destination could not be resolved.",
                    ));
                }
            };
            for destination_entry in destinations {
                let SkillDestination {
                    target,
                    canonical,
                    root,
                    full_path,
                } = destination_entry;
                let target_id = &target;
                let current = match SystemExternalTreeObserver
                    .observe(&ExternalTreeRequest::new(full_path.clone(), limits))
                {
                    Ok(snapshot) => match ValidatedSkillTree::validate(&snapshot) {
                        Ok(current) => Some(current),
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_warning(
                                Warning::new(
                                    "skill_destination_invalid",
                                    "An existing skill destination is not a valid complete skill tree.",
                                )
                                .with_context("target", target_id.as_str())
                                .with_context("scope", scope_label(concrete_scope)),
                            );
                            None
                        }
                    },
                    Err(_) => None,
                };
                if let Some(current) = current {
                    if current.fingerprint() == skill.fingerprint() {
                        outcome = outcome.with_operation(crate::OperationOutcome::new(
                            format!("skill:{}:{}", target_id, name),
                            "no_change",
                        ));
                        native_ids.insert(target_id.clone(), name.clone());
                        continue;
                    }
                    let managed_fingerprint = documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(&key))
                        .and_then(|state| state.target(target_id))
                        .and_then(|target| target.fingerprint());
                    if managed_fingerprint != Some(current.fingerprint()) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_destination_drifted",
                                "The installed skill has local drift; no replacement was made. `--yes` does not override unidentified edits.",
                            )
                            .with_context("target", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    if command != "skill update" {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_update_required",
                                "The source changed while the installed tree is intact; use `skill update <name>` to replace it explicitly.",
                            )
                            .with_context("target", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    let (identity, _) = match filesystem.load_tree_no_follow(&root, &destination) {
                        Ok(value) => value,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            return outcome.with_warning(Warning::new(
                                "skill_destination_changed",
                                "The skill destination changed before a safe replacement could be planned.",
                            ));
                        }
                    };
                    let operation_id = if canonical {
                        skill_canonical_operation_id(&key)
                    } else {
                        skill_operation_id(target_id, &key)
                    };
                    let operation = match standalone_skill_operation(
                        operation_id.clone(),
                        target_id.clone(),
                        key.clone(),
                        full_path,
                        partial_targets.contains(target_id),
                    ) {
                        Ok(operation) => operation,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The managed skill replacement operation could not be constructed safely.",
                            ));
                        }
                    };
                    operations.push(operation);
                    entries.insert(
                        operation_id,
                        ManagedSkillEntry {
                            root,
                            destination: destination.clone(),
                            tree: skill.tree().clone(),
                            backup_tree: Some(current.tree().clone()),
                            action: ManagedSkillAction::Replace,
                            expected_identity: Some(identity),
                            owner: Some(key.clone()),
                            config_root: Some(paths.skilltap_config().clone()),
                        },
                    );
                    native_ids.insert(target_id.clone(), name.clone());
                    continue;
                }
                let operation_id = if canonical {
                    skill_canonical_operation_id(&key)
                } else {
                    skill_operation_id(target_id, &key)
                };
                let operation = match standalone_skill_operation(
                    operation_id.clone(),
                    target_id.clone(),
                    key.clone(),
                    full_path,
                    partial_targets.contains(target_id),
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The managed skill operation could not be constructed safely.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    ManagedSkillEntry {
                        root,
                        destination: destination.clone(),
                        tree: skill.tree().clone(),
                        backup_tree: None,
                        action: ManagedSkillAction::Install,
                        expected_identity: None,
                        owner: None,
                        config_root: None,
                    },
                );
                native_ids.insert(target_id.clone(), name.clone());
            }
            if !native_ids.is_empty() {
                let mut bindings = Vec::new();
                for (harness, native_id) in native_ids {
                    let binding = match TargetResourceState::new(
                        harness,
                        Some(native_id),
                        Provenance::Direct,
                        Ownership::Skilltap,
                        Some(source.clone()),
                        None,
                        Some(skill.fingerprint().clone()),
                        git_commit
                            .clone()
                            .map(skilltap_core::domain::ResolvedRevision::GitCommit),
                        None,
                        timestamp,
                        None,
                    ) {
                        Ok(binding) => binding,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "state_seed_invalid",
                                "The standalone skill target evidence was invalid.",
                            ));
                        }
                    };
                    bindings.push(binding);
                }
                let state = match ResourceState::new(key.clone(), bindings) {
                    Ok(state) => state,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "state_seed_invalid",
                            "The standalone skill state seed was invalid.",
                        ));
                    }
                };
                seeds.insert(key, state);
            }
        }
        if !acknowledged
            && operations.iter().any(|operation| {
                operation.class() == skilltap_core::domain::OperationClass::Partial
            })
        {
            outcome.result = ResultClass::AttentionRequired;
            return outcome
                .with_warning(Warning::new(
                    "partial_operation_requires_acknowledgment",
                    "The skill plan contains an exact compatibility consequence; rerun the same command with `--yes` to accept it.",
                ))
                .with_next_action(NextAction::new(
                    "acknowledge_partial_operation",
                    "Review the skill plan and rerun the same command with `--yes`.",
                ))
                .with_summary("operations", operations.len() as u64)
                .with_summary("changed", false);
        }
        let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The skill inventory could not be published before installation.",
            ));
        }
        if operations.is_empty() {
            let seed_result = if command == "skill update" {
                refresh_state_seeds(self.state, &seeds)
            } else {
                seed_state_if_missing(self.state, &seeds)
            };
            if let Err(()) = seed_result {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The standalone skill state could not be recorded safely.",
                ));
            }
            if skill_install_can_complete(&outcome, acknowledged) {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            let mut outcome = outcome
                .with_summary("operations", operation_count)
                .with_summary(
                    "changed",
                    command == "skill update"
                        && git_revision_changed(old_revision.as_ref(), git_commit.as_ref()),
                );
            if command == "skill update" {
                outcome = outcome
                    .with_summary("compatibility", compatibility_label)
                    .with_summary("affected_targets", harnesses_label(&targets.resolved));
                if let Some(new_revision) = git_commit.as_ref() {
                    if let Some(old_revision) = old_revision.as_ref() {
                        outcome =
                            outcome.with_summary("old_revision", revision_label(old_revision));
                    }
                    outcome = outcome.with_summary("new_revision", format!("git:{new_revision}"));
                }
            }
            return outcome;
        }
        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The standalone skill operation plan was invalid.",
                ));
            }
        };
        let port = ManagedSkillPort {
            filesystem: &filesystem,
            entries,
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
            seeds,
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "lock_path_invalid",
                    "The skilltap configuration lock path is invalid.",
                ));
            }
        };
        let acknowledgments = if acknowledged {
            ExecutionAcknowledgments::foreground_all(&plan)
        } else {
            ExecutionAcknowledgments::default()
        };
        let report = match execute_plan_with_acknowledgments(
            &SystemConfigurationLock,
            &lock_path,
            &port,
            &journal,
            &plan,
            &acknowledgments,
        ) {
            Ok(report) => report,
            Err(error) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_error(native_execution_error(&error));
            }
        };
        for result in report.result.operations().values() {
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                operation_result_status(result.outcome()),
            ));
            if !matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            ) {
                outcome.result = ResultClass::AttentionRequired;
            }
        }
        if report.changed && skill_install_can_complete(&outcome, acknowledged) {
            outcome.result = ResultClass::Completed;
        }
        let mut outcome = outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed);
        if command == "skill update" {
            outcome = outcome
                .with_summary("compatibility", compatibility_label)
                .with_summary("affected_targets", harnesses_label(&targets.resolved));
            if let Some(new_revision) = git_commit.as_ref() {
                if let Some(old_revision) = old_revision.as_ref() {
                    outcome = outcome.with_summary("old_revision", revision_label(old_revision));
                }
                outcome = outcome.with_summary("new_revision", format!("git:{new_revision}"));
            }
        }
        outcome
    }

    pub(crate) fn execute_skill_update(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        skill_name: Option<&str>,
        acknowledged: bool,
    ) -> Outcome {
        let (documents, _) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(error);
            }
        };

        let Some(skill_name) = skill_name else {
            let candidates = documents
                .inventory
                .as_ref()
                .into_iter()
                .flat_map(|inventory| inventory.resources().values())
                .filter(|resource| {
                    resource.kind() == ResourceKind::StandaloneSkill
                        && scope
                            .resolved
                            .iter()
                            .any(|selected| selected == resource.scope())
                        && resource
                            .targets()
                            .iter()
                            .any(|selected| match target.target.as_ref() {
                                None | Some(skilltap_core::domain::TargetSelection::All) => true,
                                Some(skilltap_core::domain::TargetSelection::Only(requested)) => {
                                    requested == selected
                                }
                            })
                })
                .filter_map(|resource| {
                    let name = resource
                        .id()
                        .as_str()
                        .strip_prefix("skill:")
                        .and_then(|value| NativeId::new(value).ok())?;
                    let source = documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(resource.key()))
                        .and_then(|state| {
                            resource
                                .targets()
                                .iter()
                                .find_map(|target| state.target(target))
                        })
                        .and_then(|target| target.source())
                        .cloned();
                    Some((name, resource.scope().clone(), source))
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                return Outcome::new(command, ResultClass::Completed)
                    .with_scope(scope.output)
                    .with_summary("operations", 0_u64)
                    .with_summary("changed", false);
            }
            let mut aggregate = Outcome::new(command, ResultClass::Completed)
                .with_scope(scope.output)
                .with_summary("operations", 0_u64)
                .with_summary("changed", false);
            for (name, concrete_scope, source) in candidates {
                if source.is_none() {
                    aggregate.result =
                        merge_result(aggregate.result, ResultClass::AttentionRequired);
                    aggregate = aggregate.with_warning(
                        Warning::new(
                            "skill_source_unavailable",
                            "A selected skill has no recorded source; adopt or install it before updating.",
                        )
                        .with_context("skill", name.as_str())
                        .with_context("scope", scope_label(&concrete_scope)),
                    );
                    continue;
                }
                let child_scope = scope_args_for_scope(&concrete_scope);
                let child = self.execute_skill_update(
                    command,
                    &child_scope,
                    target,
                    Some(name.as_str()),
                    acknowledged,
                );
                let child_changed =
                    child.summary.get("changed") == Some(&OutputValue::Boolean(true));
                let child_operations = child.operations.len() as u64;
                aggregate.result = merge_result(aggregate.result, child.result);
                aggregate.summary.insert(
                    "operations".to_owned(),
                    OutputValue::Unsigned(
                        aggregate
                            .summary
                            .get("operations")
                            .and_then(|value| match value {
                                OutputValue::Unsigned(value) => Some(*value),
                                _ => None,
                            })
                            .unwrap_or_default()
                            + child_operations,
                    ),
                );
                if child_changed {
                    aggregate
                        .summary
                        .insert("changed".to_owned(), OutputValue::Boolean(true));
                }
                aggregate.resources.extend(child.resources);
                aggregate.operations.extend(child.operations);
                aggregate.warnings.extend(child.warnings);
                aggregate.errors.extend(child.errors);
                aggregate.next_actions.extend(child.next_actions);
            }
            return aggregate;
        };
        let name = match NativeId::new(skill_name) {
            Ok(name) => name,
            Err(_) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name is not a valid managed resource identifier.",
                ));
            }
        };
        let mut source = None;
        for concrete_scope in &scope.resolved {
            let Some(key) = ResourceId::new(format!("skill:{}", name.as_str()))
                .ok()
                .map(|id| ResourceKey::new(id, concrete_scope.clone()))
            else {
                continue;
            };
            if let Some(value) = documents
                .state
                .as_ref()
                .and_then(|state| state.resources().get(&key))
                .and_then(|state| {
                    state
                        .targets()
                        .values()
                        .find(|binding| match target.target.as_ref() {
                            None | Some(skilltap_core::domain::TargetSelection::All) => true,
                            Some(skilltap_core::domain::TargetSelection::Only(requested)) => {
                                binding.harness() == requested
                            }
                        })
                })
                .and_then(|target| target.source())
            {
                source = Some(value.clone());
                break;
            }
        }
        let Some(source) = source else {
            return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                Warning::new(
                    "skill_source_unavailable",
                    "The selected skill has no recorded source; adopt or install it before updating.",
                ),
            );
        };
        self.execute_skill_install(
            command,
            requested_scope,
            target,
            acknowledged,
            SkillInstallRequest {
                source: source.locator().as_str(),
                name: Some(name.as_str()),
                preserve_name: true,
                requested_revision: source.requested_revision().map(|value| value.as_str()),
                subdirectory: source.subdirectory().map(|value| value.as_str()),
            },
        )
    }

    pub(crate) fn execute_skill_remove(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        skill_name: &str,
        acknowledged: bool,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let name = match NativeId::new(skill_name) {
            Ok(name) => name,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name is not a valid managed resource identifier.",
                ));
            }
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
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
                return outcome.with_error(ErrorDetail::new(
                    "no_enabled_harnesses",
                    "No harness is enabled in skilltap configuration.",
                ));
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        if scope
            .resolved
            .iter()
            .all(|scope| matches!(scope, Scope::Project(_)))
        {
            return super::project_skills::execute_project_skill_remove(
                self,
                &scope,
                &targets,
                name,
                acknowledged,
                paths,
                outcome,
            );
        }
        let destination = match skill_relative_destination(&name) {
            Some(destination) => destination,
            None => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name cannot be used as a safe directory component.",
                ));
            }
        };
        let filesystem = SystemFileSystem;
        let limits =
            ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
                .expect("bounded skill tree limits are valid");
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let seeds = BTreeMap::new();
        let mut target_projection_keys = BTreeSet::new();
        let mut authorized_targets = Vec::new();
        let conditional_process_limits =
            ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
                .expect("bounded conditional profile process limits are valid");
        let conditional_json_limits = JsonLimits::new(256 * 1024, 64)
            .expect("bounded conditional profile JSON limits are valid");
        let search_path = std::env::var_os("PATH");
        let native_environment = match paths.native_process_environment(search_path.clone()) {
            Ok(environment) => environment,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_error(ErrorDetail::new(
                    "native_environment_unavailable",
                    "The bounded native process environment could not be resolved.",
                ));
            }
        };
        let mut profile_target_ids = Vec::new();
        for target_id in targets.resolved.iter() {
            match configured_adapter_profile(
                self.registry,
                &documents.config,
                target_id,
                NativeProfileRequest {
                    scope: &Scope::Global,
                    environment: &native_environment,
                    process_limits: conditional_process_limits,
                    json_limits: conditional_json_limits,
                    search_path: search_path.clone(),
                    capability_name: "skill.remove",
                },
            ) {
                Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => {
                    profile_target_ids.push(target_id.clone());
                }
                Ok(Some(_)) | Ok(None) | Err(_) => {
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_mutation_unavailable",
                            "The selected harness profile is not verified for standalone skill removal; no files were removed for it.",
                        )
                        .with_context("harness", target_id.as_str()),
                    );
                }
            }
        }
        if profile_target_ids.is_empty() {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(ErrorDetail::new(
                "skill_mutation_unavailable",
                "No selected harness has a verified standalone skill removal profile.",
            ));
        }
        let profile_targets =
            HarnessSet::new(profile_target_ids).expect("verified profile target set is non-empty");
        for concrete_scope in &scope.resolved {
            let (mutating_targets, next_outcome) =
                super::conditional_profile::filter_targets_for_capability(
                    self.registry,
                    &documents.config,
                    &profile_targets,
                    concrete_scope,
                    &paths,
                    conditional_process_limits,
                    conditional_json_limits,
                    &filesystem,
                    &CapabilityId::new("skill.remove").expect("static skill capability is valid"),
                    outcome,
                );
            outcome = next_outcome;
            let Some(mutating_targets) = mutating_targets else {
                continue;
            };
            authorized_targets.extend(mutating_targets.iter().cloned());
            let Some(key) = ResourceId::new(format!("skill:{}", name.as_str()))
                .ok()
                .map(|id| ResourceKey::new(id, concrete_scope.clone()))
            else {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_resource_invalid",
                    "The standalone skill resource could not be represented safely.",
                ));
            };
            let Some(state) = documents
                .state
                .as_ref()
                .and_then(|document| document.resources().get(&key))
            else {
                outcome.result = ResultClass::AttentionRequired;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_managed",
                        "The requested skill has no skilltap ownership record; no files were removed.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
                continue;
            };
            if !mutating_targets.iter().any(|target| {
                state
                    .target(target)
                    .is_some_and(|binding| binding.ownership() == Ownership::Skilltap)
            }) {
                outcome.result = ResultClass::AttentionRequired;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_owned",
                        "The requested skill is not owned by skilltap; no files were removed.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
                continue;
            }
            let warning_count = outcome.warnings.len();
            let destinations = match skill_destinations(
                self.registry,
                &paths,
                concrete_scope,
                &mutating_targets,
                &destination,
            ) {
                Some(destinations) => destinations,
                None => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "skill_destination_invalid",
                        "The selected harness skill destination could not be resolved.",
                    ));
                }
            };
            for destination_entry in destinations {
                let SkillDestination {
                    target,
                    canonical,
                    root,
                    full_path,
                } = destination_entry;
                let target_id = &target;
                let Some(target_state) = state.target(target_id) else {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_not_managed_for_target",
                            "The requested skill has no ownership record for the selected target; no files were removed.",
                        )
                        .with_context("target", target_id.as_str())
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                    continue;
                };
                let snapshot = match SystemExternalTreeObserver
                    .observe(&ExternalTreeRequest::new(full_path.clone(), limits))
                {
                    Ok(snapshot) => snapshot,
                    Err(_) => {
                        outcome = outcome.with_operation(crate::OperationOutcome::new(
                            format!("skill:{target_id}:{}", name.as_str()),
                            "no_change",
                        ));
                        continue;
                    }
                };
                let current = match ValidatedSkillTree::validate(&snapshot) {
                    Ok(current) => current,
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_destination_invalid",
                                "The existing skill destination is not a complete skill tree; no files were removed.",
                            )
                            .with_context("target", target_id.as_str()),
                        );
                        continue;
                    }
                };
                if target_state.fingerprint() != Some(current.fingerprint()) {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            if acknowledged {
                                "skill_destination_drifted"
                            } else {
                                "skill_destination_drifted_requires_acknowledgment"
                            },
                            "The current skill tree differs from the skilltap-owned fingerprint; no files were removed.",
                        )
                        .with_context("target", target_id.as_str())
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                    continue;
                }
                let (identity, files) = match filesystem.load_tree_no_follow(&root, &destination) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let tree = match ArtifactTree::new(
                    files
                        .into_iter()
                        .map(|(path, bytes)| (path.as_str().to_owned(), bytes)),
                ) {
                    Ok(tree) => tree,
                    Err(_) => continue,
                };
                let operation_id = if canonical {
                    skill_canonical_remove_operation_id(&key)
                } else {
                    skill_remove_operation_id(target_id, &key)
                };
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    operation_id.clone(),
                    target_id.clone(),
                    key.clone(),
                    OperationAction::SkillRemove,
                    full_path,
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The managed skill removal operation could not be constructed safely.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    ManagedSkillEntry {
                        root,
                        destination: destination.clone(),
                        tree,
                        backup_tree: None,
                        action: ManagedSkillAction::Remove,
                        expected_identity: Some(identity),
                        owner: None,
                        config_root: None,
                    },
                );
            }
            if outcome.warnings.len() == warning_count {
                target_projection_keys.insert(key);
            }
        }
        let authorized_targets = HarnessSet::new(authorized_targets)
            .expect("authorized standalone skill targets remain unique");
        if !acknowledged && !outcome.warnings.is_empty() {
            return outcome
                .with_summary("operations", 0_u64)
                .with_summary("changed", false);
        }
        if operations.is_empty() {
            inventory = match project_inventory_targets(
                &inventory,
                &target_projection_keys,
                &authorized_targets,
            ) {
                Ok(inventory) => inventory,
                Err(()) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_publish_failed",
                        "The skill inventory could not be updated safely.",
                    ));
                }
            };
            let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
                InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                    .expect("empty inventory is valid")
            });
            if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The skill inventory could not be updated safely.",
                ));
            }
            if project_state_targets_after_remove(
                self.state,
                &target_projection_keys,
                &targets.resolved,
            )
            .is_err()
            {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_publish_failed",
                    "The skill state could not be updated safely.",
                ));
            }
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false);
        }
        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The managed skill removal plan was invalid.",
                ));
            }
        };
        let port = ManagedSkillPort {
            filesystem: &filesystem,
            entries,
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
            seeds,
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "lock_path_invalid",
                    "The skilltap configuration lock path is invalid.",
                ));
            }
        };
        let acknowledgments = if acknowledged {
            ExecutionAcknowledgments::foreground_all(&plan)
        } else {
            ExecutionAcknowledgments::default()
        };
        let report = match execute_plan_with_acknowledgments(
            &SystemConfigurationLock,
            &lock_path,
            &port,
            &journal,
            &plan,
            &acknowledgments,
        ) {
            Ok(report) => report,
            Err(error) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_error(native_execution_error(&error));
            }
        };
        let report_successful = report.result.operations().values().all(|result| {
            matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            )
        });
        for result in report.result.operations().values() {
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                operation_result_status(result.outcome()),
            ));
            if !matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            ) {
                outcome.result = ResultClass::AttentionRequired;
            }
        }
        if report.changed && outcome.errors.is_empty() && outcome.warnings.is_empty() {
            outcome.result = ResultClass::Completed;
        }
        if report_successful {
            inventory = match project_inventory_targets(
                &inventory,
                &target_projection_keys,
                &authorized_targets,
            ) {
                Ok(inventory) => inventory,
                Err(()) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_publish_failed",
                        "The skill inventory could not be updated safely.",
                    ));
                }
            };
            let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
                InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                    .expect("empty inventory is valid")
            });
            if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The skill inventory could not be updated safely.",
                ));
            }
            if project_state_targets_after_remove(
                self.state,
                &target_projection_keys,
                &targets.resolved,
            )
            .is_err()
            {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_publish_failed",
                    "The skill state could not be updated safely.",
                ));
            }
        }
        outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed)
    }
}
