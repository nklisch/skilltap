use super::*;

impl StatusApplication<'_> {
    pub(crate) fn execute_plan(&self, args: &PlanArgs) -> Outcome {
        self.execute_reconciliation("plan", &args.target, &args.scope, &[], &[], false)
    }

    pub(crate) fn execute_sync(&self, args: &SyncArgs) -> Outcome {
        self.execute_reconciliation(
            "sync",
            &args.target,
            &args.scope,
            &args.selection.include,
            &args.selection.exclude,
            args.acknowledgment.yes,
        )
    }

    fn execute_reconciliation(
        &self,
        command: &'static str,
        target: &TargetArgs,
        requested_scope: &ScopeArgs,
        includes: &[NativeId],
        excludes: &[NativeId],
        acknowledged: bool,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        if acknowledged {
            // Carry the generic foreground acknowledgment through the
            // reconciliation boundary. Resource adapters decide which
            // consequences are eligible; hard blocks and drift are never
            // bypassed by this flag.
            outcome = outcome.with_summary("acknowledged", true);
        }
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
                return first_use_harness_report(
                    &documents.config,
                    outcome,
                    self.native_observation,
                    target.target.as_ref(),
                )
                .with_summary("scopes", scope.count)
                .with_summary("targets", 0_u64)
                .with_next_action(
                    NextAction::new("enable_harness", "Enable Codex or Claude management.")
                        .with_command("skilltap harness enable <codex|claude>"),
                );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(
                    ErrorDetail::new(
                        "target_not_enabled",
                        "The requested harness target is not enabled.",
                    )
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable the requested harness.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    ),
                );
            }
        };

        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => NativeObservation::run(&documents, &scope, &targets),
        };
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        for action in observation.next_actions.iter().cloned() {
            outcome = outcome.with_next_action(action);
        }

        let desired = documents
            .inventory
            .as_ref()
            .map(|inventory| inventory.resources().values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let selected = reconciliation_selections(&desired, &scope, &targets, includes, excludes);
        let desired_count = selected
            .iter()
            .map(|(resource, _)| resource.key().to_string())
            .collect::<BTreeSet<_>>()
            .len() as u64;

        // `plan` is deliberately side-effect free.  Its candidates are
        // assembled by the same resource-to-lifecycle projection used by
        // `sync`, but rendered as planned operations instead of executing a
        // native command or publishing state.
        if command == "plan" {
            for (resource, target_id) in selected.iter().copied() {
                let child_scope = scope_args_for_scope(resource.scope());
                let (source, name) = reconciliation_source_and_name(resource);
                let child = match resource.kind() {
                    ResourceKind::Marketplace
                    | ResourceKind::Plugin
                    | ResourceKind::StandaloneSkill => self.execute_lifecycle_preview(
                        "plan",
                        &child_scope,
                        &TargetArgs {
                            target: Some(skilltap_core::domain::TargetSelection::Only(
                                target_id.clone(),
                            )),
                        },
                        resource.kind(),
                        source,
                        name,
                    ),
                    ResourceKind::InstructionLocation => self
                        .execute_instruction_reconciliation_preview(
                            &child_scope,
                            target_id,
                            resource,
                        ),
                    _ => Outcome::new("plan", ResultClass::Completed).with_operation(
                        crate::OperationOutcome::new(
                            format!("reconcile:{}:{}", target_id, resource.key()),
                            "planned",
                        )
                        .with_field("target", target_id.as_str())
                        .with_field("scope", scope_label(resource.scope())),
                    ),
                };
                merge_reconciliation_outcome(&mut outcome, child);
            }
            let operation_count = outcome.operations.len() as u64;
            if observation.failed_targets > 0 {
                outcome.result = ResultClass::AttentionRequired;
            } else if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                // A plan is an attention result whenever it contains work for
                // the caller to inspect, even when every operation is safe or
                // already satisfied.  Only an empty plan is a completed
                // no-change result; sync keeps its normal applied/no-change
                // classification below.
                outcome.result = if operation_count > 0 {
                    ResultClass::AttentionRequired
                } else {
                    ResultClass::Completed
                };
            }
            return outcome
                .with_summary("desired_resources", desired_count)
                .with_summary("operations", operation_count)
                .with_summary("scopes", scope.count)
                .with_summary("targets", targets.iter().len() as u64)
                .with_summary("observed_targets", observation.observed_targets as u64)
                .with_summary("failed_targets", observation.failed_targets as u64)
                .with_summary("changed", false);
        }

        // `sync` delegates each candidate to its existing lifecycle adapter.
        // This keeps locking, revalidation, native process bounds, journaling,
        // and post-mutation observation identical to explicit commands.
        for (resource, target_id) in selected.iter().copied() {
            let child_scope = scope_args_for_scope(resource.scope());
            let (source, name) = reconciliation_source_and_name(resource);
            let child_target = TargetArgs {
                target: Some(skilltap_core::domain::TargetSelection::Only(
                    target_id.clone(),
                )),
            };
            let child = match resource.kind() {
                ResourceKind::Marketplace => self.execute_native_lifecycle(
                    "sync",
                    NativeLifecycleKind::MarketplaceAdd,
                    &child_scope,
                    &child_target,
                    NativeLifecycleValues { source, name },
                    acknowledged,
                ),
                ResourceKind::Plugin => self.execute_native_lifecycle(
                    "sync",
                    NativeLifecycleKind::PluginInstall,
                    &child_scope,
                    &child_target,
                    NativeLifecycleValues {
                        source: name,
                        name: None,
                    },
                    acknowledged,
                ),
                ResourceKind::StandaloneSkill => match source {
                    Some(source) => self.execute_skill_install(
                        "sync",
                        &child_scope,
                        &child_target,
                        acknowledged,
                        SkillInstallRequest {
                            source,
                            name,
                            preserve_name: true,
                            requested_revision: resource
                                .source()
                                .and_then(|value| value.requested_revision())
                                .map(|value| value.as_str()),
                            subdirectory: resource
                                .source()
                                .and_then(|value| value.subdirectory())
                                .map(|value| value.as_str()),
                        },
                    ),
                    None => Outcome::new("sync", ResultClass::AttentionRequired).with_warning(
                        Warning::new(
                            "skill_source_unavailable",
                            "The desired skill has no source locator and cannot be synchronized.",
                        ),
                    ),
                },
                ResourceKind::InstructionLocation => self.execute_instruction_setup_for_target(
                    "sync",
                    &child_scope,
                    None,
                    acknowledged,
                    true,
                    Some(target_id),
                ),
                ResourceKind::Harness => Outcome::new("sync", ResultClass::Completed),
            };
            merge_reconciliation_outcome(&mut outcome, child);
        }
        if observation.failed_targets > 0 {
            outcome.result = ResultClass::AttentionRequired;
        } else if outcome.errors.is_empty()
            && outcome.warnings.iter().all(|warning| {
                matches!(
                    warning.code.as_str(),
                    "instruction_bridge_repair" | "instruction_bridge_consolidation"
                )
            })
        {
            outcome.result = ResultClass::Completed;
        }
        let changed = outcome.summary.get("changed") == Some(&OutputValue::Boolean(true));
        let operation_count = outcome.operations.len() as u64;
        outcome
            .with_summary("desired_resources", desired_count)
            .with_summary("operations", operation_count)
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("changed", changed)
    }
}

fn merge_reconciliation_outcome(aggregate: &mut Outcome, child: Outcome) {
    let child_changed = child.summary.get("changed") == Some(&OutputValue::Boolean(true));
    aggregate.result = merge_result(aggregate.result, child.result);
    aggregate.resources.extend(child.resources);
    aggregate.operations.extend(child.operations);
    aggregate.warnings.extend(child.warnings);
    aggregate.errors.extend(child.errors);
    aggregate.next_actions.extend(child.next_actions);
    if child_changed {
        aggregate
            .summary
            .insert("changed".to_owned(), OutputValue::Boolean(true));
    }
}

fn reconciliation_selector_matches(
    resource: &DesiredResource,
    includes: &[NativeId],
    excludes: &[NativeId],
) -> bool {
    let id = resource.id().as_str();
    let included = includes.is_empty()
        || includes.iter().any(|selector| {
            selector.as_str() == id || selector.as_str() == resource.key().to_string()
        });
    let excluded = excludes
        .iter()
        .any(|selector| selector.as_str() == id || selector.as_str() == resource.key().to_string());
    included && !excluded
}

fn reconciliation_selections<'a>(
    desired: &'a [DesiredResource],
    scope: &StatusScope,
    targets: &StatusTargets,
    includes: &[NativeId],
    excludes: &[NativeId],
) -> Vec<(&'a DesiredResource, &'a HarnessId)> {
    desired
        .iter()
        .filter(|resource| {
            scope
                .resolved
                .iter()
                .any(|selected| selected == resource.scope())
                && reconciliation_selector_matches(resource, includes, excludes)
        })
        .flat_map(|resource| {
            resource
                .targets()
                .iter()
                .filter(|target| targets.resolved.contains(target))
                .map(move |target| (resource, target))
        })
        .collect()
}

fn reconciliation_source_and_name(resource: &DesiredResource) -> (Option<&str>, Option<&str>) {
    let source = resource.source().map(|source| source.locator().as_str());
    let name = match resource.kind() {
        ResourceKind::Marketplace => resource.id().as_str().strip_prefix("marketplace:"),
        ResourceKind::Plugin => resource.id().as_str().strip_prefix("plugin:"),
        ResourceKind::StandaloneSkill => resource.id().as_str().strip_prefix("skill:"),
        _ => None,
    };
    (source, name)
}
