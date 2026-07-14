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
                    self.registry,
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
                    NextAction::new("enable_harness", "Enable a registered harness.")
                        .with_command("skilltap harness enable <registered-harness>"),
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
                            .with_command("skilltap harness enable <registered-harness>"),
                    );
            }
        };

        StatusProjection {
            documents: &documents,
            scope: &scope,
            targets: &targets,
            native_observation: self.native_observation,
            registry: self.registry,
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
        outcome.scope = Some(scope.output.clone());

        let observation = NativeObservation::run(self.registry, &documents, &scope, &targets);
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
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        for action in observation.next_actions.iter().cloned() {
            outcome = outcome.with_next_action(action);
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
                let refreshed = NativeObservation::run(self.registry, &documents, &scope, &targets);
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
            Ok(documents) => {
                if let Some(target) = documents
                    .config
                    .harnesses()
                    .iter()
                    .map(|(target, _)| target)
                    .find(|target| !self.registry.contains(target))
                {
                    outcome.result = ResultClass::Invalid;
                    return Err(Box::new(outcome.with_error(
                        ErrorDetail::new(
                            "target_not_registered",
                            "The configuration contains a harness that is not registered in this build.",
                        )
                        .with_context("harness", target.as_str()),
                    )));
                }
                Ok((documents, outcome))
            }
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
    registry: &'a skilltap_harnesses::TargetRegistry,
}

impl StatusProjection<'_> {
    pub(super) fn apply(self, mut outcome: Outcome) -> Outcome {
        for target in self.targets.iter() {
            outcome = outcome.with_resource(OutputEntry::new(target.as_str(), "selected"));
        }
        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => {
                NativeObservation::run(self.registry, self.documents, self.scope, self.targets)
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
        for action in observation.next_actions.iter().cloned() {
            outcome = outcome.with_next_action(action);
        }
        for entry in update_entries {
            outcome = outcome.with_resource(entry);
        }
        for warning in update_warnings {
            outcome = outcome.with_warning(warning);
        }
        if let Some(state) = self.documents.state.as_ref() {
            for entry in daemon_status_projection(state, state.daemon_run()) {
                outcome = outcome.with_resource(entry);
            }
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
    pub(super) next_actions: Vec<NextAction>,
}

impl NativeObservation {
    pub(super) fn run(
        registry: &skilltap_harnesses::TargetRegistry,
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
                    next_actions: Vec::new(),
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
        let native_environment = match paths.native_process_environment(search_path.clone()) {
            Ok(environment) => environment,
            Err(_) => {
                return Self {
                    resources: Vec::new(),
                    warnings: vec![Warning::new(
                        "native_environment_unavailable",
                        "The bounded native process environment could not be resolved for read-only status.",
                    )],
                    observed_targets: 0,
                    failed_targets: targets.iter().len() * scope.resolved.len(),
                    native_entries: 0,
                    environment: None,
                    next_actions: Vec::new(),
                };
            }
        };
        let mut result = Self::default();
        let mut requests = Vec::new();
        let mut outcomes = Vec::new();
        let mut metadata = Vec::new();

        for target in targets.iter() {
            let Some(adapter) = registry.adapter(target) else {
                result.failed_targets += scope.resolved.len();
                result.warnings.push(
                    Warning::new(
                        "unsupported_harness",
                        "The selected harness is not supported.",
                    )
                    .with_context("harness", target.as_str()),
                );
                continue;
            };
            let Some(policy) = documents.config.harnesses().get(target) else {
                result.failed_targets += scope.resolved.len();
                continue;
            };
            let binary = policy.binary.as_str();
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
                adapter,
                configured,
                search_path.clone(),
                &native_environment,
                process_limits,
                json_limits,
            ) {
                Ok(installation) => installation,
                Err(error) => {
                    result.failed_targets += scope.resolved.len();
                    result.resources.extend(scope.resolved.iter().map(|scope| {
                        OutputEntry::new(observation_id(target, scope), "unreachable")
                            .with_field("harness", target.as_str())
                            .with_field("scope", scope_label(scope))
                    }));
                    let diagnostic = detection_diagnostic(&error, target.as_str(), binary);
                    result.warnings.push(diagnostic.warning);
                    result.next_actions.push(diagnostic.next_action);
                    continue;
                }
            };

            let HarnessReachability::Reachable { native_version, .. } = installation.reachability()
            else {
                result.failed_targets += scope.resolved.len();
                continue;
            };
            let profile = adapter.select_profile(native_version);
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
                match adapter.observe(&paths, current_scope, tree_limits) {
                    Ok(observed_paths) => {
                        let mut resources = observed_paths
                            .canonical
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
                        resources.extend(observed_paths.surface_labels.into_iter().map(|root| {
                            native_surface_resource(
                                target,
                                current_scope,
                                root,
                                profile.authority(),
                                0,
                            )
                        }));
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
pub(crate) fn first_use_harness_report(
    registry: &skilltap_harnesses::TargetRegistry,
    config: &ConfigDocument,
    mut outcome: Outcome,
    mode: NativeObservationMode,
    requested: Option<&skilltap_core::domain::TargetSelection>,
) -> Outcome {
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded status process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded status JSON limits are valid");
    let search_path = std::env::var_os("PATH");
    let native_environment = PlatformPaths::resolve(&ProcessEnvironment)
        .and_then(|paths| paths.native_process_environment(search_path.clone()));
    let all_harnesses = registry.ids().cloned().collect::<Vec<_>>();
    let selected = skilltap_core::runtime::resolve_targets(requested, all_harnesses.clone())
        .unwrap_or_else(|_| {
            skilltap_core::domain::HarnessSet::new(all_harnesses).expect("registry is non-empty")
        });
    for adapter in registry
        .iter()
        .filter(|adapter| selected.contains(&adapter.identity().id))
    {
        let harness = adapter.identity().id;
        let Some(policy) = config.harnesses().get(&harness) else {
            continue;
        };
        let binary = policy.binary.as_str();
        let mut entry =
            OutputEntry::new(harness.as_str(), "not_enabled").with_field("enabled", false);
        if mode == NativeObservationMode::Disabled {
            outcome = outcome.with_resource(entry);
            continue;
        }
        let configured = match configured_binary(binary) {
            Ok(value) => value,
            Err(_) => {
                outcome = outcome.with_warning(
                    Warning::new(
                        "invalid_harness_binary",
                        "The configured harness binary could not be resolved.",
                    )
                    .with_context("harness", harness.as_str()),
                );
                outcome = outcome.with_resource(entry.with_field("reachable", false));
                continue;
            }
        };
        let Ok(native_environment) = native_environment.as_ref() else {
            outcome = outcome.with_warning(
                Warning::new(
                    "native_environment_unavailable",
                    "The bounded native process environment could not be resolved.",
                )
                .with_context("harness", harness.as_str()),
            );
            outcome = outcome.with_resource(entry.with_field("reachable", false));
            continue;
        };
        match detect_configured_installation(
            adapter,
            configured,
            search_path.clone(),
            native_environment,
            process_limits,
            json_limits,
        ) {
            Ok(installation) => {
                if let HarnessReachability::Reachable { native_version, .. } =
                    installation.reachability()
                {
                    entry.status = "installed".to_owned();
                    entry = entry
                        .with_field("reachable", true)
                        .with_field("version", native_version.as_str());
                }
            }
            Err(error) => {
                entry.status = "unreachable".to_owned();
                entry = entry.with_field("reachable", false);
                let diagnostic = detection_diagnostic(&error, harness.as_str(), binary);
                outcome = outcome
                    .with_warning(diagnostic.warning)
                    .with_next_action(diagnostic.next_action);
            }
        }
        outcome = outcome.with_resource(entry);
    }
    outcome
}

fn daemon_status_projection(
    state: &skilltap_core::storage::StateDocument,
    record: Option<&skilltap_core::storage::DaemonRunRecord>,
) -> Vec<OutputEntry> {
    let Some(record) = record else {
        return vec![OutputEntry::new("daemon", "never_run")];
    };
    let status = match record.result() {
        skilltap_core::storage::DaemonRunResult::Completed => "completed",
        skilltap_core::storage::DaemonRunResult::Pending => "pending",
        skilltap_core::storage::DaemonRunResult::Contended => "contended",
        skilltap_core::storage::DaemonRunResult::Failed => "failed",
    };
    let mut entries = Vec::with_capacity(record.operations().len() + 1);
    let mut daemon = OutputEntry::new("daemon", status)
        .with_field("last_run_seconds", record.at().seconds())
        .with_field("safe_operations", record.safe_operations())
        .with_field("pending_operations", record.pending_operations());
    if let Some(code) = record.failure_code() {
        daemon = daemon.with_field("failure", code.as_str());
    }
    entries.push(daemon);
    for reference in record.operations() {
        let result = state
            .resources()
            .get(reference.resource())
            .and_then(|resource| resource.target(reference.target()))
            .and_then(|target| target.last_apply())
            .and_then(|apply| apply.operations().get(reference.operation()));
        let (status, result_label, dependencies) = match result {
            Some(result) => (
                operation_result_status(result.outcome()),
                operation_result_status(result.outcome()),
                match result.outcome() {
                    OperationOutcome::SkippedDependency { dependencies } => dependencies
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                    _ => String::new(),
                },
            ),
            None => ("pending", "indeterminate", String::new()),
        };
        let mut entry = OutputEntry::new(
            format!("daemon-operation:{}", reference.operation()),
            status,
        )
        .with_field("phase", daemon_operation_phase(reference.action()))
        .with_field("operation", reference.operation().to_string())
        .with_field("resource", reference.resource().to_string())
        .with_field("target", reference.target().as_str())
        .with_field("result", result_label);
        if !dependencies.is_empty() {
            entry = entry.with_field("dependencies", dependencies);
        }
        entries.push(entry);
    }
    entries
}

fn daemon_operation_phase(action: skilltap_core::domain::OperationAction) -> &'static str {
    match action {
        skilltap_core::domain::OperationAction::MarketplaceUpdate => "marketplace_refresh",
        skilltap_core::domain::OperationAction::PluginUpdate => "plugin_update",
        skilltap_core::domain::OperationAction::SkillUpdate => "skill_update",
        _ => "lifecycle",
    }
}

struct UnavailableSourceRevisionResolver;

impl SourceRevisionResolver for UnavailableSourceRevisionResolver {
    fn resolve(
        &self,
        source: &skilltap_core::domain::Source,
    ) -> Result<skilltap_core::domain::ResolvedRevision, ResolutionError> {
        Err(ResolutionError::UnsupportedSourceKind(source.kind()))
    }
}

fn status_update_projection(
    documents: &StatusDocuments,
    scope: &StatusScope,
    targets: &StatusTargets,
    observation: &NativeObservation,
) -> (Vec<OutputEntry>, Vec<Warning>, usize) {
    let Some(inventory) = documents.inventory.as_ref() else {
        return (Vec::new(), Vec::new(), 0);
    };
    let Some(environment) = observation.environment.as_ref() else {
        return (Vec::new(), Vec::new(), 0);
    };
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded update resolution process limits are valid");
    let git_resolver = GitSourceRevisionResolver::system(process_limits).ok();
    let native_resolver = ObservedNativeRevisionResolver::new(environment);
    let fallback_resolver = UnavailableSourceRevisionResolver;
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut available_updates = 0;
    let update_mode = documents.config.updates().mode;
    for resource in inventory.resources().values().filter(|resource| {
        scope.resolved.contains(resource.scope())
            && resource
                .targets()
                .iter()
                .any(|target| targets.resolved.contains(target))
    }) {
        let installed = documents
            .state
            .as_ref()
            .and_then(|state| state.resources().get(resource.key()))
            .and_then(|state| {
                resource
                    .targets()
                    .iter()
                    .filter(|target| targets.resolved.contains(target))
                    .find_map(|target| state.target(target))
            })
            .and_then(|target| target.installed_revision());
        if update_mode == skilltap_core::storage::UpdateMode::Off
            || resource.update() == UpdateIntent::Disabled
        {
            let candidate = UpdateCandidate {
                resource: resource.key().clone(),
                current_revision: installed.cloned(),
                available_revision: None,
                resolution_error: None,
                pinned: resource.update() == UpdateIntent::Pinned,
                drifted: false,
                compatibility_changed: false,
                requires_acknowledgment: false,
                intent: resource.update(),
                acknowledgment_selectors: BTreeSet::new(),
            };
            let decision = classify_update_with_mode(&candidate, update_mode);
            entries.push(update_projection_entry(resource, &candidate, decision));
            continue;
        }
        let request = UpdateResolutionRequest {
            resource,
            installed,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
        };
        let resolved = if resource.source().is_some() {
            match git_resolver.as_ref() {
                Some(resolver) => resolve_candidate(resolver, &native_resolver, request),
                None => resolve_candidate(&fallback_resolver, &native_resolver, request),
            }
        } else {
            resolve_candidate(&fallback_resolver, &native_resolver, request)
        };
        if let Some(error) = resolved.error.as_ref() {
            warnings.push(
                Warning::new(
                    "update_resolution_unavailable",
                    "An available revision could not be resolved without mutation.",
                )
                .with_context("resource", resource.key().to_string())
                .with_context("reason", resolution_error_label(error)),
            );
        }
        let candidate = candidate_for(resource, &request, &resolved);
        let decision = classify_update_with_mode(&candidate, update_mode);
        if decision.is_actionable_available() {
            available_updates += 1;
        }
        entries.push(update_projection_entry(resource, &candidate, decision));
    }
    (entries, warnings, available_updates)
}

fn update_projection_entry(
    resource: &DesiredResource,
    candidate: &UpdateCandidate,
    decision: UpdateDecision,
) -> OutputEntry {
    let status = match (decision.safety, decision.reason) {
        (UpdateSafety::NoUpdate, Some(UpdateDecisionReason::DisabledResource)) => "disabled",
        (UpdateSafety::Blocked, Some(UpdateDecisionReason::GlobalModeOff)) => "policy_off",
        (UpdateSafety::NeedsDecision, Some(UpdateDecisionReason::CheckOnly)) => "check_only",
        (UpdateSafety::NeedsDecision, Some(UpdateDecisionReason::PinnedResource)) => "pinned",
        (UpdateSafety::NoUpdate, _) => "up_to_date",
        (UpdateSafety::Safe, _) => "safe",
        (UpdateSafety::NeedsDecision, _) => "needs_decision",
        (UpdateSafety::Blocked, _) => "blocked",
    };
    let mut entry = OutputEntry::new(format!("update:{}", resource.key()), status)
        .with_field("resource", resource.key().to_string());
    if let Some(reason) = decision.reason {
        entry = entry.with_field("reason", update_decision_reason_label(reason));
    }
    if let Some(current) = candidate.current_revision.as_ref() {
        entry = entry.with_field("current", revision_label(current));
    }
    if let Some(available) = candidate.available_revision.as_ref() {
        entry = entry.with_field("available", revision_label(available));
    }
    entry
}

fn update_decision_reason_label(reason: UpdateDecisionReason) -> &'static str {
    match reason {
        UpdateDecisionReason::DisabledResource => "disabled_resource",
        UpdateDecisionReason::GlobalModeOff => "global_mode_off",
        UpdateDecisionReason::CheckOnly => "check_only",
        UpdateDecisionReason::PinnedResource => "pinned_resource",
        UpdateDecisionReason::Drifted => "drifted",
        UpdateDecisionReason::CompatibilityChanged => "compatibility_changed",
        UpdateDecisionReason::AcknowledgmentRequired => "acknowledgment_required",
        UpdateDecisionReason::ResolutionFailed => "resolution_failed",
    }
}

fn resolution_error_label(error: &ResolutionError) -> &'static str {
    match error {
        ResolutionError::UnreachableSource => "unreachable_source",
        ResolutionError::InvalidRequestedRevision => "invalid_requested_revision",
        ResolutionError::UnsupportedSourceKind(_) => "unsupported_source_kind",
        ResolutionError::NativeObservationUnavailable => "native_observation_unavailable",
        ResolutionError::TargetDisagreement => "target_disagreement",
    }
}
fn native_surface_resource(
    harness: &HarnessId,
    scope: &Scope,
    root: &str,
    authority: ProfileAuthority,
    entries: usize,
) -> ObservedResource {
    let id = ResourceId::new(stable_resource_id(harness, root))
        .expect("stable native surface identifier is valid");
    let key = ResourceKey::new(id, scope.clone());
    let observation_key = ObservationKey::new(key, harness.clone(), ObservationLayer::Effective);
    let kind = native_surface_kind(root);
    let native_identity = NativeId::new(format!("{harness}:{root}:entries-{entries}"))
        .expect("native surface identity is valid");
    ObservedResource::new(
        observation_key,
        kind,
        Provenance::Native,
        Ownership::Unmanaged,
        if authority == ProfileAuthority::ObserveOnly {
            ResourceHealth::Unknown
        } else {
            ResourceHealth::Healthy
        },
        None,
        ComponentGraph::new([]).expect("empty component graph is valid"),
        BTreeSet::new(),
        native_identity,
        None,
        None,
    )
}

fn stable_resource_id(harness: &HarnessId, root: &str) -> String {
    let hash = stable_hash(&format!("{harness}:{root}"));
    format!("native-{hash:016x}")
}

fn native_surface_kind(root: &str) -> ResourceKind {
    if root.ends_with("skills") {
        ResourceKind::StandaloneSkill
    } else if root.contains("marketplace") {
        ResourceKind::Marketplace
    } else if root.ends_with("plugins") || root.ends_with("claude") {
        ResourceKind::Plugin
    } else {
        ResourceKind::InstructionLocation
    }
}

fn observation_error(error: skilltap_harnesses::ObservationPathError) -> ObservationAdapterError {
    use skilltap_core::runtime::ObservationRuntimeError as RuntimeError;
    let skilltap_harnesses::ObservationPathError::Runtime(error) = error else {
        return ObservationAdapterError::NativeShapeUnsupported {};
    };
    match error {
        RuntimeError::TreeDepthLimitExceeded
        | RuntimeError::TreeEntryLimitExceeded
        | RuntimeError::TreeFileLimitExceeded
        | RuntimeError::TreeTotalLimitExceeded
        | RuntimeError::TreeSymlinkTargetLimitExceeded => {
            ObservationAdapterError::ResourceLimitExceeded {}
        }
        RuntimeError::ProcessDeadlineExceeded => ObservationAdapterError::DeadlineExceeded {},
        RuntimeError::TreeEntryUnsupported
        | RuntimeError::TreeEntryNonUtf8
        | RuntimeError::DuplicateTreeEntry => ObservationAdapterError::NativeShapeUnsupported {},
        _ => ObservationAdapterError::NativeStateUnreadable {},
    }
}

fn resource_identity(resource: &ObservedResource) -> String {
    format!(
        "{}:{}:{}",
        resource.key().harness(),
        resource.key().resource().id(),
        match resource.key().layer() {
            ObservationLayer::Declared => "declared",
            ObservationLayer::Effective => "effective",
        }
    )
}

fn resource_health(health: ResourceHealth) -> &'static str {
    match health {
        ResourceHealth::Healthy => "healthy",
        ResourceHealth::Drifted => "drifted",
        ResourceHealth::Degraded => "degraded",
        ResourceHealth::Unknown => "unknown",
    }
}

fn resource_kind(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Harness => "harness",
        ResourceKind::Marketplace => "marketplace",
        ResourceKind::Plugin => "plugin",
        ResourceKind::StandaloneSkill => "standalone_skill",
        ResourceKind::InstructionLocation => "instruction_location",
    }
}

fn profile_authority(authority: ProfileAuthority) -> &'static str {
    match authority {
        ProfileAuthority::VerifiedCompiled => "verified_compiled",
        ProfileAuthority::ObserveOnly => "observe_only",
    }
}

fn capability_count(
    profile: &skilltap_core::domain::CapabilityProfileSelection,
    scope: &Scope,
    support: CapabilitySupport,
) -> usize {
    profile
        .observation_capabilities()
        .for_scope(scope)
        .iter()
        .filter(|(_, value)| *value == support)
        .count()
}

fn finding_warning(finding: &ObservationFinding) -> Warning {
    Warning::new(finding.code().as_str(), finding.summary().as_str()).with_context(
        "severity",
        format!("{:?}", finding.severity()).to_lowercase(),
    )
}

fn observation_id(harness: &HarnessId, scope: &Scope) -> String {
    format!("{}:{}", harness, scope_label(scope))
}
