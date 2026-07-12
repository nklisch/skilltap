use super::*;

impl StatusApplication<'_> {
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
        let resource_updates_enabled =
            documents.config.updates().mode == skilltap_core::storage::UpdateMode::ApplySafe;
        if !resource_updates_enabled {
            aggregate = aggregate
                .with_warning(Warning::new(
                    "daemon_policy_not_apply_safe",
                    "The configured update policy does not permit automatic application.",
                ))
                .with_summary("changed", safe_operations > 0)
                .with_summary("safe_operations", safe_operations)
                .with_summary("pending_operations", pending_operations);
            normalize_daemon_noop_result(&mut aggregate, safe_operations, pending_operations);
            self.persist_daemon_run(&mut aggregate, safe_operations, pending_operations);
            return aggregate;
        }
        let mut tasks = Vec::new();
        if let Some(inventory) = documents.inventory.as_ref() {
            for resource in inventory.resources().values() {
                if resource.update() != UpdateIntent::Track {
                    continue;
                }
                let name = if resource.kind() == ResourceKind::Plugin {
                    resource.id().as_str().strip_prefix("plugin:")
                } else if resource.kind() == ResourceKind::StandaloneSkill
                    && resource
                        .source()
                        .is_some_and(|source| source.kind() == SourceKind::Git)
                {
                    resource.id().as_str().strip_prefix("skill:")
                } else {
                    None
                };
                let Some(name) = name else { continue };
                tasks.push((resource.kind(), name.to_owned(), resource.scope().clone()));
            }
        }
        let mut changed = safe_operations > 0;
        for (kind, name, scope) in tasks {
            let child_scope = scope_args_for_scope(&scope);
            let child = match kind {
                ResourceKind::Plugin => self.execute_native_lifecycle(
                    command,
                    NativeLifecycleKind::PluginUpdate,
                    &child_scope,
                    &TargetArgs::default(),
                    None,
                    Some(&name),
                ),
                ResourceKind::StandaloneSkill => self.execute_skill_update(
                    // A daemon cycle is still the safe, non-interactive
                    // update path.  Reuse the explicit skill-update
                    // command's replacement planning so a changed Git
                    // revision is applied when the destination is managed
                    // and unchanged; the daemon wrapper still refuses
                    // drift, pins, and other judgment-required work.
                    "skill update",
                    &child_scope,
                    &TargetArgs::default(),
                    Some(&name),
                    false,
                ),
                _ => continue,
            };
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
        aggregate = aggregate
            .with_summary("changed", changed)
            .with_summary("safe_operations", safe_operations)
            .with_summary("pending_operations", pending_operations);
        normalize_daemon_noop_result(&mut aggregate, safe_operations, pending_operations);
        self.persist_daemon_run(&mut aggregate, safe_operations, pending_operations);
        aggregate
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
        let source = source.unwrap_or("not supplied");
        let name = name.unwrap_or("derived by lifecycle adapter");
        let mut operation_count = 0_u64;
        for concrete_scope in &scope.resolved {
            for harness in targets.iter() {
                operation_count += 1;
                let presence =
                    lifecycle_preview_presence(&documents, kind, harness, concrete_scope, name);
                let recorded = lifecycle_recorded_state(&documents, kind, concrete_scope, name);
                let status = match (presence, recorded) {
                    (NativeResourcePresence::Present, true) => "no_change",
                    (NativeResourcePresence::Present, false)
                    | (NativeResourcePresence::Missing, true)
                    | (NativeResourcePresence::Missing, false) => "repair",
                    (NativeResourcePresence::Unknown, _) => "planned",
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
        source_value: Option<&str>,
        name_value: Option<&str>,
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
        let mut requests = Vec::new();
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

        for concrete_scope in &scope.resolved {
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
                        None => match request.desired_resource(concrete_scope, &targets.resolved) {
                            Ok(resource) => resource,
                            Err(error) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(error);
                            }
                        },
                    }
                } else {
                    let proposed = match request.desired_resource(concrete_scope, &targets.resolved)
                    {
                        Ok(resource) => resource,
                        Err(error) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(error);
                        }
                    };
                    match inventory.resources().get(proposed.key()) {
                        Some(existing) if existing.source() == proposed.source() => {
                            existing.clone()
                        }
                        _ => proposed,
                    }
                };
                if request.retains_desired() && !request.is_update() {
                    match inventory.with_resource(resource.clone()) {
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
                let mut native_ids = documents
                    .state
                    .as_ref()
                    .and_then(|state| state.resources().get(resource.key()))
                    .map(|state| state.native_ids().clone())
                    .unwrap_or_default();
                for target_id in targets.iter() {
                    let Some((harness, configured, executable, capability)) =
                        configured_native_profile(
                            &documents.config,
                            target_id,
                            concrete_scope,
                            process_limits,
                            json_limits,
                            search_path.clone(),
                            match kind {
                                NativeLifecycleKind::MarketplaceAdd => "marketplace.register",
                                NativeLifecycleKind::MarketplaceRemove => "marketplace.remove",
                                NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
                                NativeLifecycleKind::PluginInstall => "plugin.install",
                                NativeLifecycleKind::PluginRemove => "plugin.remove",
                                NativeLifecycleKind::PluginUpdate => "plugin.update",
                            },
                        )
                    else {
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
                    };
                    if capability != CapabilitySupport::Supported {
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
                    let native_request = request.native_request(harness, concrete_scope.clone());
                    let arguments = match native_arguments(&native_request) {
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
                    let journal_says_applied =
                        previously_applied(documents.state.as_ref(), resource.key(), &operation_id);
                    let fresh_presence = if journal_says_applied {
                        observe_native_resource(
                            configured.clone(),
                            search_path.clone(),
                            &native_environment,
                            &native_request,
                            process_limits,
                            json_limits,
                        )
                        .unwrap_or(NativeResourcePresence::Unknown)
                    } else {
                        NativeResourcePresence::Unknown
                    };
                    if journal_says_applied && fresh_presence != NativeResourcePresence::Missing {
                        outcome = outcome.with_operation(
                            crate::OperationOutcome::new(operation_id.to_string(), "no_change")
                                .with_field("target", target_id.as_str())
                                .with_field("scope", scope_label(concrete_scope)),
                        );
                        continue;
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
                        executable,
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
                    requests.push((
                        operation_id,
                        configured,
                        search_path.clone(),
                        process_limits,
                        native_request,
                    ));
                }
                if !native_ids.is_empty() {
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
                    let native_state = match ResourceState::new(
                        resource.key().clone(),
                        native_ids,
                        Provenance::Native,
                        Ownership::Harness,
                        resource
                            .source()
                            .cloned()
                            .or_else(|| request.source.clone()),
                        None,
                        None,
                        None,
                        None,
                        observed_at,
                        None,
                    ) {
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

        let inventory_changed = inventory != original_inventory;
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
        let port =
            NativeLifecyclePort::new_per_operation_with_environment(requests, native_environment);
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome
                    .with_error(native_execution_error(&error))
                    .with_next_action(NextAction::new(
                        "reobserve_before_retry",
                        "Re-observe the selected harness before retrying the lifecycle operation.",
                    ));
                }
            };
        let observation = NativeObservation::run(&documents, &scope, &targets);
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
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
        if report.changed && observation.failed_targets == 0 && successful {
            outcome.result = ResultClass::Completed;
        }
        outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed)
            .with_next_action(NextAction::new(
                "verify_status",
                "Run status to verify the fresh native observation and recorded state.",
            ))
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
        let mut partial_compatibility = false;
        for compatibility in SkillCompatibility::evaluate(&skill, &targets.resolved) {
            match compatibility.class() {
                SkillCompatibilityClass::Blocked => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_incompatible",
                            "The skill frontmatter is not loadable by the selected harness.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                SkillCompatibilityClass::Warning => {
                    compatibility_label = "warning";
                    partial_compatibility = true;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_frontmatter_warning",
                            "The skill is loadable but its frontmatter is not fully strict.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                SkillCompatibilityClass::Compatible => {}
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
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let mut old_revision = None;
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
                    .and_then(|state| state.installed_revision())
                    .cloned();
            }
            let desired_targets = inventory
                .resources()
                .get(&key)
                .map(|resource| resource.targets().clone())
                .unwrap_or_else(|| targets.resolved.clone());
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
            if partial_compatibility {
                let acknowledgment_selector = OperationSelector::Resource {
                    resource: key.clone(),
                };
                let candidate = UpdateCandidate {
                    resource: key.clone(),
                    // Compatibility acknowledgment is a foreground
                    // decision even when the source revision itself did not
                    // change (for example, the first install or a repeated
                    // materialization). Use a private revision pair solely
                    // to exercise the update-selection contract; the actual
                    // source revision remains recorded below.
                    current_revision: Some(skilltap_core::domain::ResolvedRevision::Native(
                        NativeId::new("skilltap-partial-current").expect("static native id"),
                    )),
                    available_revision: Some(skilltap_core::domain::ResolvedRevision::Native(
                        NativeId::new("skilltap-partial-available").expect("static native id"),
                    )),
                    resolution_error: None,
                    pinned: update_intent == UpdateIntent::Pinned,
                    drifted: false,
                    compatibility_changed: false,
                    requires_acknowledgment: true,
                    intent: update_intent,
                    acknowledgment_selectors: [acknowledgment_selector].into_iter().collect(),
                };
                let update_plan = match plan_foreground_updates(ForegroundUpdateRequest {
                    resources: std::slice::from_ref(&desired),
                    candidates: std::slice::from_ref(&candidate),
                    mode: documents.config.updates().mode,
                }) {
                    Ok(plan) => plan,
                    Err(_error) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "foreground_update_plan_invalid",
                            "The partial skill update could not be validated safely.",
                        ));
                    }
                };
                if let Err(_error) = select_foreground_updates_with_acknowledgment(
                    &update_plan,
                    &BTreeSet::new(),
                    acknowledged,
                ) {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome
                        .with_warning(
                            Warning::new(
                                "partial_operation_requires_acknowledgment",
                                "The skill is loadable but not fully strict for the selected harness; rerun with `--yes` to accept the reported loss.",
                            ),
                        )
                        .with_next_action(NextAction::new(
                            "accept_partial",
                            "Review the compatibility warning, then retry with `--yes` if the partial result is acceptable.",
                        ))
                        .with_summary("operations", 0_u64)
                        .with_summary("changed", false);
                }
            }
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
            let mut native_ids = documents
                .state
                .as_ref()
                .and_then(|state| state.resources().get(&key))
                .map(|state| state.native_ids().clone())
                .unwrap_or_default();
            let destinations =
                match skill_destinations(&paths, concrete_scope, &targets.resolved, &destination) {
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
                        .and_then(|state| state.fingerprint());
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
                    let operation =
                        match skilltap_core::lifecycle_operation::faithful_file_operation(
                            operation_id.clone(),
                            target_id.clone(),
                            key.clone(),
                            OperationAction::SkillInstall,
                            full_path,
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
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    operation_id.clone(),
                    target_id.clone(),
                    key.clone(),
                    OperationAction::SkillInstall,
                    full_path,
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
                let state = match ResourceState::new(
                    key.clone(),
                    native_ids,
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
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
                        .and_then(|state| state.source())
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
                .and_then(|state| state.source())
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
        for concrete_scope in &scope.resolved {
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
            if state.ownership() != Ownership::Skilltap {
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
            let destinations =
                match skill_destinations(&paths, concrete_scope, &targets.resolved, &destination) {
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
                if state.fingerprint() != Some(current.fingerprint()) {
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
        if !acknowledged && !outcome.warnings.is_empty() {
            return outcome
                .with_summary("operations", 0_u64)
                .with_summary("changed", false);
        }
        if operations.is_empty() {
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
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
                &targets.resolved,
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
