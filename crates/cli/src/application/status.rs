use super::*;

impl StatusApplication<'_> {
    pub(crate) fn execute(&self, args: &StatusArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("status") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };

        let scope = match StatusScope::resolve(self, args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());

        let targets = match StatusTargets::resolve(args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return first_use_harness_report(
                    &documents.config,
                    outcome,
                    self.native_observation,
                    args.target.target.as_ref(),
                )
                .with_summary("targets", 0_u64)
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
                return outcome
                    .with_error(ErrorDetail::new(
                        "target_not_enabled",
                        "The requested harness target is not enabled.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable the requested harness.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
        };

        StatusProjection {
            documents: &documents,
            scope: &scope,
            targets: &targets,
            native_observation: self.native_observation,
        }
        .apply(outcome)
    }

    pub(crate) fn execute_adopt(&self, args: &AdoptArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("adopt") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };

        let status_args = StatusArgs {
            target: TargetArgs {
                target: args.from.clone(),
            },
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
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                outcome.result = ResultClass::AttentionRequired;
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
        outcome.scope = Some(scope.output.clone());

        let observation = NativeObservation::run(&documents, &scope, &targets);
        let Some(environment) = observation.environment.clone() else {
            outcome.result = ResultClass::AttentionRequired;
            return outcome
                .with_error(ErrorDetail::new(
                    "native_observation_unavailable",
                    "Native resources could not be observed safely; adoption did not write inventory.",
                ))
                .with_next_action(NextAction::new(
                    "repair_native_observation",
                    "Resolve the reported harness observation problem and retry adoption.",
                ));
        };
        for warning in observation.warnings {
            outcome = outcome.with_warning(warning);
        }
        outcome = outcome
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("native_entries", observation.native_entries as u64);
        let selection = AdoptionSelection::new(targets.iter().flat_map(|harness| {
            scope
                .resolved
                .iter()
                .map(move |scope| ObservationTarget::new(harness.clone(), scope.clone()))
        }));
        let initial_plan =
            match plan_adoption(documents.inventory.as_ref(), &environment, &selection) {
                Ok(plan) => plan,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "adoption_plan_invalid",
                        "Native observations could not be converted into a safe adoption plan.",
                    ));
                }
            };

        if initial_plan.additions.is_empty() {
            return project_adoption(outcome, &initial_plan, false, observation.failed_targets);
        }

        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "platform_paths_unavailable",
                        "The skilltap configuration paths could not be resolved.",
                    ))
                    .with_next_action(NextAction::new(
                        "repair_environment",
                        "Repair the configured home and XDG paths before retrying adoption.",
                    ));
            }
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                return outcome.with_error(ErrorDetail::new(
                    "lock_path_invalid",
                    "The skilltap configuration lock path is invalid.",
                ));
            }
        };
        let applied = apply_adoption(
            &SystemConfigurationLock,
            &lock_path,
            self.inventory,
            &initial_plan,
            |evidence| {
                let refreshed = NativeObservation::run(&documents, &scope, &targets);
                if refreshed.environment.is_none() {
                    return Err(AdoptionObservationError::Unavailable);
                }
                if refreshed.failed_targets > 0 {
                    // The normalized environment still carries healthy sibling
                    // observations; the core revalidation decides whether the
                    // selected evidence itself is stale.
                }
                let _ = evidence;
                Ok(refreshed.environment.expect("checked above"))
            },
        );
        match applied {
            Ok(result) => project_adoption(
                outcome,
                &result.plan,
                result.changed,
                observation.failed_targets,
            ),
            Err(error) => outcome
                .with_error(adoption_apply_error(&error))
                .with_next_action(adoption_next_action(&error)),
        }
    }

    pub(super) fn load_documents(
        &self,
        command: &'static str,
    ) -> Result<(StatusDocuments, Outcome), Box<Outcome>> {
        let loaded = DocumentLoadPhase::execute(self);
        let mut outcome = loaded.project(Outcome::new(command, ResultClass::AttentionRequired));
        match loaded.finish() {
            Ok(documents) => Ok((documents, outcome)),
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                Err(Box::new(outcome.with_next_action(NextAction::new(
                    "repair_owned_documents",
                    "Repair the reported skilltap-owned documents before retrying.",
                ))))
            }
        }
    }

    pub(super) fn scope_request(
        &self,
        args: &StatusArgs,
        inventory: Option<&InventoryDocument>,
    ) -> Result<ScopeRequest, ErrorDetail> {
        match args.scope.argument() {
            ScopeArgument::Global => Ok(ScopeRequest::Global),
            ScopeArgument::AllScopes => Ok(ScopeRequest::AllScopes {
                recorded_projects: inventory
                    .map(|value| value.projects().iter().cloned().collect())
                    .unwrap_or_default(),
            }),
            ScopeArgument::Project(None) => Ok(ScopeRequest::Project { path: None }),
            ScopeArgument::Project(Some(path)) => {
                let path =
                    absolute_project_argument(&path, self.working_directory).map_err(|_| {
                        ErrorDetail::new(
                            "invalid_project_path",
                            "The project path could not be converted to a canonical absolute path.",
                        )
                    })?;
                Ok(ScopeRequest::Project { path: Some(path) })
            }
        }
    }
}

struct DocumentLoadPhase {
    config: Result<DocumentState<ConfigDocument>, StorageError>,
    inventory: Result<DocumentState<InventoryDocument>, StorageError>,
    state: Result<DocumentState<StateDocument>, StorageError>,
}

impl DocumentLoadPhase {
    fn execute(application: &StatusApplication<'_>) -> Self {
        Self {
            config: application.config.load(),
            inventory: application.inventory.load(),
            state: application.state.load(),
        }
    }

    fn project(&self, outcome: Outcome) -> Outcome {
        let outcome = document_result(outcome, "config", &self.config);
        let outcome = document_result(outcome, "inventory", &self.inventory);
        document_result(outcome, "state", &self.state)
    }

    fn finish(self) -> Result<StatusDocuments, Vec<ErrorDetail>> {
        let mut errors = Vec::new();
        if let Err(error) = &self.config {
            errors.push(storage_error(error));
        }
        if let Err(error) = &self.inventory {
            errors.push(storage_error(error));
        }
        if let Err(error) = &self.state {
            errors.push(storage_error(error));
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(StatusDocuments {
            config: match self.config.expect("checked above") {
                DocumentState::Missing => ConfigDocument::defaults(),
                DocumentState::Present(config) => config,
            },
            inventory: match self.inventory.expect("checked above") {
                DocumentState::Missing => None,
                DocumentState::Present(inventory) => Some(inventory),
            },
            state: match self.state.expect("checked above") {
                DocumentState::Missing => None,
                DocumentState::Present(state) => Some(state),
            },
        })
    }
}

pub(super) struct StatusDocuments {
    pub(super) config: ConfigDocument,
    pub(super) inventory: Option<InventoryDocument>,
    pub(super) state: Option<StateDocument>,
}

pub(super) struct StatusScope {
    pub(super) output: OutputScope,
    pub(super) count: u64,
    pub(super) resolved: Vec<Scope>,
}

impl StatusScope {
    pub(super) fn resolve(
        application: &StatusApplication<'_>,
        args: &StatusArgs,
        documents: &StatusDocuments,
    ) -> Result<Self, ErrorDetail> {
        let request = application.scope_request(args, documents.inventory.as_ref())?;
        let scopes = application.scopes.resolve(&request).map_err(|_| {
            ErrorDetail::new(
                "project_scope_unavailable",
                "The requested project scope could not be resolved.",
            )
        })?;
        let resolved = scopes.into_scopes();
        Ok(Self {
            output: output_scope(&args.scope.argument(), &resolved),
            count: resolved.len() as u64,
            resolved,
        })
    }
}

pub(super) struct StatusTargets {
    pub(super) resolved: HarnessSet,
}

pub(super) enum StatusTargetError {
    NoneEnabled,
    NotEnabled,
}

impl StatusTargets {
    pub(super) fn resolve(
        args: &StatusArgs,
        documents: &StatusDocuments,
    ) -> Result<Self, StatusTargetError> {
        let enabled = enabled_harnesses(&documents.config);
        if enabled.is_empty() {
            return Err(StatusTargetError::NoneEnabled);
        }
        resolve_targets(args.target.target.as_ref(), enabled)
            .map(|resolved| Self { resolved })
            .map_err(|_| StatusTargetError::NotEnabled)
    }

    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = &HarnessId> {
        self.resolved.iter()
    }
}

pub(super) struct StatusProjection<'a> {
    documents: &'a StatusDocuments,
    scope: &'a StatusScope,
    targets: &'a StatusTargets,
    native_observation: NativeObservationMode,
}

impl StatusProjection<'_> {
    pub(super) fn apply(self, mut outcome: Outcome) -> Outcome {
        for target in self.targets.iter() {
            outcome = outcome.with_resource(OutputEntry::new(target.as_str(), "selected"));
        }
        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => {
                NativeObservation::run(self.documents, self.scope, self.targets)
            }
        };
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        let (update_entries, update_warnings, available_updates) =
            status_update_projection(self.documents, self.scope, self.targets, &observation);
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        for entry in update_entries {
            outcome = outcome.with_resource(entry);
        }
        for warning in update_warnings {
            outcome = outcome.with_warning(warning);
        }
        if let Some(state) = self.documents.state.as_ref() {
            outcome = outcome.with_resource(daemon_status_projection(state.daemon_run()));
        }
        if observation.failed_targets == 0 {
            outcome.result = ResultClass::Completed;
        }
        let mut outcome = outcome
            .with_summary(
                "desired_resources",
                self.documents
                    .inventory
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary(
                "recorded_resources",
                self.documents
                    .state
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary("scopes", self.scope.count)
            .with_summary("targets", self.targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("native_entries", observation.native_entries as u64)
            .with_summary("available_updates", available_updates as u64);
        let desired_keys = self
            .documents
            .inventory
            .as_ref()
            .map(|inventory| {
                inventory
                    .resources()
                    .keys()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        let recorded_keys = self
            .documents
            .state
            .as_ref()
            .map(|state| {
                state
                    .resources()
                    .keys()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        if desired_keys != recorded_keys {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "resource.drifted",
                "Desired inventory and recorded state differ; no mutation is implied by status.",
            ));
        }
        if outcome
            .warnings
            .iter()
            .any(|warning| warning.code == "capability.unverified")
        {
            outcome.result = ResultClass::AttentionRequired;
        }
        if observation.failed_targets > 0 {
            outcome = outcome.with_next_action(NextAction::new(
                "inspect_native_observation",
                "Inspect the reported native observation warnings before planning changes.",
            ));
        }
        let desired_resources = self
            .documents
            .inventory
            .as_ref()
            .map_or(0, |value| value.resources().len());
        let recorded_resources = self
            .documents
            .state
            .as_ref()
            .map_or(0, |value| value.resources().len());
        if observation.failed_targets == 0 && (desired_resources > 0 || recorded_resources > 0) {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "status_comparison_unavailable",
                "Resource-level desired comparison is not available for the observed native surfaces; review is required before planning changes.",
            ));
        }
        outcome
    }
}

#[derive(Default)]
pub(super) struct NativeObservation {
    pub(super) resources: Vec<OutputEntry>,
    pub(super) warnings: Vec<Warning>,
    pub(super) observed_targets: usize,
    pub(super) failed_targets: usize,
    pub(super) native_entries: usize,
    pub(super) environment: Option<skilltap_core::domain::ObservedEnvironment>,
}

impl NativeObservation {
    pub(super) fn run(
        documents: &StatusDocuments,
        scope: &StatusScope,
        targets: &StatusTargets,
    ) -> Self {
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                return Self {
                    resources: Vec::new(),
                    warnings: vec![Warning::new(
                        "native_paths_unavailable",
                        "Native harness paths could not be resolved for read-only status.",
                    )],
                    observed_targets: 0,
                    failed_targets: targets.iter().len() * scope.resolved.len(),
                    native_entries: 0,
                    environment: None,
                };
            }
        };

        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded status process limits are valid");
        let json_limits =
            JsonLimits::new(256 * 1024, 64).expect("bounded status JSON limits are valid");
        let tree_limits =
            ExternalTreeLimits::new(16, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024, 64 * 1024)
                .expect("bounded status tree limits are valid");
        let search_path = std::env::var_os("PATH");
        let mut result = Self::default();
        let mut requests = Vec::new();
        let mut outcomes = Vec::new();
        let mut metadata = Vec::new();

        for target in targets.iter() {
            let (kind, binary) = match target.as_str() {
                "codex" => (
                    HarnessKind::Codex,
                    documents.config.harnesses().codex.binary.as_str(),
                ),
                "claude" => (
                    HarnessKind::Claude,
                    documents.config.harnesses().claude.binary.as_str(),
                ),
                _ => {
                    result.failed_targets += scope.resolved.len();
                    result.warnings.push(
                        Warning::new(
                            "unsupported_harness",
                            "The selected harness is not supported.",
                        )
                        .with_context("harness", target.as_str()),
                    );
                    continue;
                }
            };
            let configured = match configured_binary(binary) {
                Ok(binary) => binary,
                Err(_) => {
                    result.failed_targets += scope.resolved.len();
                    result.warnings.push(
                        Warning::new(
                            "invalid_harness_binary",
                            "The configured harness binary could not be resolved.",
                        )
                        .with_context("harness", target.as_str()),
                    );
                    continue;
                }
            };
            let installation = match detect_configured_installation(
                kind,
                configured,
                search_path.clone(),
                process_limits,
                json_limits,
            ) {
                Ok(installation) => installation,
                Err(_error) => {
                    result.failed_targets += scope.resolved.len();
                    result.resources.extend(scope.resolved.iter().map(|scope| {
                        OutputEntry::new(observation_id(target, scope), "unreachable")
                            .with_field("harness", target.as_str())
                            .with_field("scope", scope_label(scope))
                    }));
                    result.warnings.push(
                        Warning::new(
                            "native_detection_failed",
                            "The configured harness could not be detected.",
                        )
                        .with_context("harness", target.as_str()),
                    );
                    continue;
                }
            };

            let HarnessReachability::Reachable { native_version, .. } = installation.reachability()
            else {
                result.failed_targets += scope.resolved.len();
                continue;
            };
            let profile = select_profile(kind, native_version);
            for current_scope in &scope.resolved {
                let evidence = match ObservationEvidence::new(&installation, profile.clone()) {
                    Ok(value) => value,
                    Err(_) => {
                        result.failed_targets += 1;
                        continue;
                    }
                };
                let request = ObservationRequest::new(current_scope.clone(), evidence);
                requests.push(request.clone());
                metadata.push((
                    target.clone(),
                    current_scope.clone(),
                    native_version.clone(),
                    profile.clone(),
                ));
                match observe_trees(kind, &paths, current_scope, tree_limits) {
                    Ok(snapshots) => {
                        let mut resources = snapshots
                            .iter()
                            .map(|snapshot| {
                                native_surface_resource(
                                    target,
                                    current_scope,
                                    &snapshot.root,
                                    profile.authority(),
                                    snapshot.snapshot.entries().len(),
                                )
                            })
                            .collect::<Vec<_>>();
                        resources.extend(
                            instruction_surface_labels(kind, &paths, current_scope)
                                .into_iter()
                                .map(|root| {
                                    native_surface_resource(
                                        target,
                                        current_scope,
                                        root,
                                        profile.authority(),
                                        0,
                                    )
                                }),
                        );
                        let findings = if profile.authority() == ProfileAuthority::ObserveOnly {
                            vec![ObservationFinding::new(
                                ObservationFindingCode::CapabilityUnverified,
                                ObservationSummary::CapabilityUnverified,
                                ObservationSeverity::Warning,
                                ObservationSubject::Harness {
                                    harness: target.clone(),
                                    scope: current_scope.clone(),
                                },
                                ObservationFields::default(),
                            )]
                        } else {
                            Vec::new()
                        };
                        match HarnessObservation::new(request.clone(), resources, findings) {
                            Ok(observation) => {
                                outcomes.push(HarnessObservationOutcome::observed(observation));
                            }
                            Err(_) => outcomes.push(HarnessObservationOutcome::failed(
                                request,
                                ObservationAdapterError::NativeShapeUnsupported {},
                            )),
                        }
                    }
                    Err(error) => outcomes.push(HarnessObservationOutcome::failed(
                        request,
                        observation_error(error),
                    )),
                }
            }
        }
        let batch = match ObservationBatch::new(requests) {
            Ok(batch) => batch,
            Err(_) => {
                result.warnings.push(Warning::new(
                    "native_observation_contract",
                    "Native observations could not be normalized safely.",
                ));
                result.failed_targets += metadata.len();
                return result;
            }
        };
        let environment = match normalize_observations(batch, outcomes) {
            Ok(environment) => environment,
            Err(_) => {
                result.warnings.push(Warning::new(
                    "native_observation_contract",
                    "Native observations could not be normalized safely.",
                ));
                result.failed_targets += metadata.len();
                return result;
            }
        };
        result.environment = Some(environment.clone());
        for (_, outcome) in environment.iter() {
            match outcome {
                HarnessObservationOutcome::Observed { observation } => {
                    result.observed_targets += 1;
                    let evidence = observation.request().evidence();
                    let profile = evidence.profile();
                    let target = observation.target();
                    let scope = target.scope();
                    result.resources.push(
                        OutputEntry::new(
                            observation_id(target.harness(), target.scope()),
                            "observed",
                        )
                        .with_field("harness", target.harness().as_str())
                        .with_field("scope", scope_label(target.scope()))
                        .with_field("typed", true)
                        .with_field("version", evidence.native_version().as_str())
                        .with_field("profile_authority", profile_authority(profile.authority()))
                        .with_field(
                            "capabilities_supported",
                            capability_count(profile, scope, CapabilitySupport::Supported) as u64,
                        )
                        .with_field(
                            "capabilities_unverified",
                            capability_count(profile, scope, CapabilitySupport::Unverified) as u64,
                        )
                        .with_field(
                            "capabilities_unsupported",
                            capability_count(profile, scope, CapabilitySupport::Unsupported) as u64,
                        )
                        .with_field("native_entries", observation.resources().len() as u64),
                    );
                    for resource in observation.resources().values() {
                        let mut entry = OutputEntry::new(
                            resource_identity(resource),
                            resource_health(resource.health()),
                        )
                        .with_field("harness", observation.target().harness().as_str())
                        .with_field("scope", scope_label(resource.scope()))
                        .with_field("kind", resource_kind(resource.kind()))
                        .with_field("native_identity", resource.native_identity().as_str());
                        if resource.native_identity().as_str().contains(".") {
                            entry = entry.with_field("native_entries", 0_u64);
                        }
                        result.resources.push(entry);
                        result.native_entries += 1;
                    }
                    for finding in observation.findings() {
                        result.warnings.push(finding_warning(finding));
                    }
                }
                HarnessObservationOutcome::Failed {
                    request,
                    error: _error,
                } => {
                    result.failed_targets += 1;
                    result.resources.push(
                        OutputEntry::new(
                            observation_id(request.target().harness(), request.scope()),
                            "observation_failed",
                        )
                        .with_field("harness", request.target().harness().as_str())
                        .with_field("scope", scope_label(request.scope())),
                    );
                    result.warnings.push(
                        Warning::new(
                            "native_observation_failed",
                            "Native harness state could not be observed within the safety limits.",
                        )
                        .with_context("harness", request.target().harness().as_str())
                        .with_context("scope", scope_label(request.scope())),
                    );
                }
            }
        }
        result
    }
}

fn project_adoption(
    mut outcome: Outcome,
    plan: &skilltap_core::adoption::AdoptionPlan,
    changed: bool,
    failed_targets: usize,
) -> Outcome {
    let mut adopted = 0_u64;
    let mut coalesced = 0_u64;
    let mut already_managed = 0_u64;
    let mut attention = failed_targets > 0;
    for decision in &plan.decisions {
        let (key, status) = match decision {
            AdoptionDecision::Adopted(candidate) => {
                adopted += 1;
                (candidate.desired.key(), "adopted")
            }
            AdoptionDecision::Coalesced(candidate) => {
                coalesced += 1;
                (candidate.desired.key(), "coalesced")
            }
            AdoptionDecision::AlreadyManaged { key } => {
                already_managed += 1;
                (key, "already_managed")
            }
            AdoptionDecision::Conflict { key, .. } => {
                attention = true;
                (key, "conflict")
            }
            AdoptionDecision::Unadoptable { key, .. } => {
                attention = true;
                (key, "unadoptable")
            }
            AdoptionDecision::Unchanged { key } => (key, "unchanged"),
        };
        outcome = outcome.with_resource(OutputEntry::new(key.to_string(), status));
    }
    outcome = outcome
        .with_summary("adopted", adopted)
        .with_summary("coalesced", coalesced)
        .with_summary("already_managed", already_managed)
        .with_summary("changed", changed);
    if attention {
        outcome.result = ResultClass::AttentionRequired;
        if failed_targets > 0 {
            outcome = outcome.with_warning(Warning::new(
                "partial_native_observation",
                "Some selected harness scopes could not be observed; only validated siblings were considered.",
            ));
        }
    } else {
        outcome.result = ResultClass::Completed;
    }
    outcome
}

fn adoption_apply_error(error: &AdoptionApplyError) -> ErrorDetail {
    let code = match error {
        AdoptionApplyError::Lock(_) => "configuration_locked",
        AdoptionApplyError::Inventory(_) => "inventory_unavailable",
        AdoptionApplyError::Observation(_) => "native_observation_unavailable",
        AdoptionApplyError::StaleEvidence => "stale_observation",
        AdoptionApplyError::Plan(_) => "adoption_plan_invalid",
        AdoptionApplyError::Release(_) => "configuration_lock_release_failed",
    };
    let summary = match error {
        AdoptionApplyError::Lock(_) => "Another skilltap mutation holds the configuration lock.",
        AdoptionApplyError::Inventory(_) => {
            "The skilltap inventory could not be safely loaded or published."
        }
        AdoptionApplyError::Observation(_) => {
            "Native resources could not be re-observed safely before publication."
        }
        AdoptionApplyError::StaleEvidence => {
            "Native adoption evidence changed before publication; no inventory was written."
        }
        AdoptionApplyError::Plan(_) => {
            "The refreshed native observations no longer form a safe adoption plan."
        }
        AdoptionApplyError::Release(_) => {
            "The configuration lock could not be released after adoption."
        }
    };
    ErrorDetail::new(code, summary)
}

fn adoption_next_action(error: &AdoptionApplyError) -> NextAction {
    match error {
        AdoptionApplyError::Lock(_) => NextAction::new(
            "retry_after_lock",
            "Wait for the other skilltap mutation to finish, then retry adoption.",
        ),
        AdoptionApplyError::StaleEvidence => NextAction::new(
            "reobserve_and_retry",
            "Review native changes and retry adoption to use fresh evidence.",
        ),
        _ => NextAction::new(
            "repair_and_retry",
            "Resolve the reported adoption boundary error and retry.",
        ),
    }
}
