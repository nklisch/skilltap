use super::*;

pub(super) struct StateExecutionJournal<'a> {
    pub(crate) plan: &'a Plan,
    pub(crate) state: &'a dyn StateRepository,
    pub(crate) seeds: BTreeMap<ResourceKey, ResourceState>,
}

impl ExecutionJournal for StateExecutionJournal<'_> {
    fn record(&self, result: &OperationResult) -> Result<(), ExecutionError> {
        let operation = self.plan.get(result.operation_id()).ok_or_else(|| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.operation_unknown")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state journal received an operation outside the validated plan.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let resource = operation.selector().resource();
        let current = self.state.load().map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.load_failed")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state document could not be loaded for journaling.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let current = match current {
            DocumentState::Present(current) => current,
            DocumentState::Missing => skilltap_core::storage::StateDocument::new(
                skilltap_core::storage::STATE_SCHEMA_VERSION,
                [],
                [],
                None,
                None,
                None,
            )
            .map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.seed_invalid")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The seed state for the operation was invalid.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?,
        };
        let native_attempt_seed = self.seeds.get(resource).is_some_and(|seed| {
            !seed.targets().is_empty()
                && seed.targets().values().all(|target| {
                    target.provenance() == Provenance::Native
                        && target.ownership() == Ownership::Harness
                        && target.managed_projections().is_empty()
                })
        });
        let publish_seed = native_attempt_seed
            || matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            );
        let pending_managed_attempt =
            !native_attempt_seed && matches!(result.outcome(), OperationOutcome::Pending);
        let current = if current.resources().contains_key(resource) {
            if publish_seed && let Some(seed) = self.seeds.get(resource) {
                current.refresh_resource_state(seed.clone()).map_err(|_| {
                    ExecutionError::journal_failure(
                        skilltap_core::domain::EvidenceCode::new("state.seed_refresh_failed")
                            .expect("static evidence code is valid"),
                        skilltap_core::domain::EvidenceDetail::new(
                            "The existing resource metadata could not be refreshed safely.",
                        )
                        .expect("static evidence detail is valid"),
                    )
                })?
            } else if pending_managed_attempt && let Some(seed) = self.seeds.get(resource) {
                let existing = current
                    .resources()
                    .get(resource)
                    .expect("resource was checked");
                let attempt = managed_pending_resource(Some(existing), seed, result.operation_id())
                    .map_err(|_| managed_attempt_journal_failure())?;
                current
                    .refresh_resource_state(attempt)
                    .map_err(|_| managed_attempt_journal_failure())?
            } else {
                current
            }
        } else if publish_seed && let Some(seed) = self.seeds.get(resource) {
            current.with_resource_state(seed.clone()).map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.seed_conflict")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The operation resource could not be seeded in state.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?
        } else if pending_managed_attempt && let Some(seed) = self.seeds.get(resource) {
            let attempt_targets = managed_pending_resource(None, seed, result.operation_id())
                .map_err(|_| managed_attempt_journal_failure())?;
            current
                .with_resource_state(attempt_targets)
                .map_err(|_| managed_attempt_journal_failure())?
        } else {
            return Ok(());
        };
        let at = Timestamp::from_system_time(std::time::SystemTime::now()).map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.clock_invalid")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The operation timestamp could not be recorded.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let next = current
            .with_operation_result(resource, operation.target(), at, result.clone())
            .map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.resource_unavailable")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The operation resource could not be journaled in state.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?;
        self.state.replace(&next).map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.publish_failed")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state journal could not be published atomically.",
                )
                .expect("static evidence detail is valid"),
            )
        })
    }
}

fn managed_pending_resource(
    existing: Option<&ResourceState>,
    desired: &ResourceState,
    operation_id: &OperationId,
) -> Result<ResourceState, ()> {
    let mut next = existing.cloned();
    for target in desired.targets().values() {
        let fingerprint = target.fingerprint().cloned().ok_or(())?;
        let attempt = PendingManagedAttempt::new(
            operation_id.clone(),
            fingerprint,
            target.managed_projections().iter().cloned(),
            target.installed_revision().cloned(),
        )
        .map_err(|_| ())?;
        let binding =
            if let Some(current) = existing.and_then(|state| state.target(target.harness())) {
                current.clone().with_pending_managed_attempt(attempt)
            } else {
                TargetResourceState::new(
                    target.harness().clone(),
                    target.native_id().cloned(),
                    target.provenance(),
                    target.ownership(),
                    target.source().cloned(),
                    None,
                    None,
                    None,
                    None,
                    target.observed_at(),
                    None,
                )
                .map_err(|_| ())?
                .with_pending_managed_attempt(attempt)
            };
        next = Some(match next {
            Some(state) => state.with_target(binding).map_err(|_| ())?,
            None => ResourceState::new(desired.key().clone(), [binding]).map_err(|_| ())?,
        });
    }
    next.ok_or(())
}

fn managed_attempt_journal_failure() -> ExecutionError {
    ExecutionError::journal_failure(
        skilltap_core::domain::EvidenceCode::new("state.attempt_seed_invalid")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(
            "The managed operation attempt could not be recorded safely.",
        )
        .expect("static evidence detail is valid"),
    )
}

pub(super) struct ManagedSkillPort<'a> {
    pub(super) filesystem: &'a dyn DirectoryTreeFileSystem,
    pub(super) entries: BTreeMap<OperationId, ManagedSkillEntry>,
}

pub(super) struct ProjectSkillLinkEntry {
    pub(super) root: AbsolutePath,
    pub(super) destination: skilltap_core::domain::RelativeArtifactPath,
    pub(super) target: RelativeSymlinkTarget,
    pub(super) action: ProjectSkillLinkAction,
}

pub(super) enum ProjectSkillLinkAction {
    Create,
    Replace {
        expected_identity: LinkIdentity,
        previous_target: RelativeSymlinkTarget,
    },
    Remove {
        expected_identity: LinkIdentity,
    },
}

pub(super) struct ProjectSkillLinkPort<'a> {
    pub(super) filesystem: &'a dyn ManagedLifecycleFileSystem,
    pub(super) entries: BTreeMap<OperationId, ProjectSkillLinkEntry>,
    pub(super) foreign_operations: BTreeSet<OperationId>,
}

pub(super) struct ProjectSkillLifecyclePort<'a> {
    pub(super) canonical: ManagedSkillPort<'a>,
    pub(super) links: ProjectSkillLinkPort<'a>,
}

pub(crate) trait ManagedLifecycleFileSystem:
    FileSystem + DirectoryTreeFileSystem + skilltap_core::runtime::ConfinedFileSystem
{
}

impl<T: FileSystem + DirectoryTreeFileSystem + skilltap_core::runtime::ConfinedFileSystem>
    ManagedLifecycleFileSystem for T
{
}

pub(super) struct ManagedLifecyclePort<'a> {
    pub(super) filesystem: &'a dyn ManagedLifecycleFileSystem,
    pub(super) entries: BTreeMap<OperationId, ManagedLifecycleEntry>,
    pub(super) registry: &'a skilltap_harnesses::TargetRegistry,
    pub(super) config: &'a ConfigDocument,
    pub(super) environment: &'a BTreeMap<OsString, OsString>,
    pub(super) search_path: Option<OsString>,
    pub(super) process_limits: ProcessLimits,
    pub(super) json_limits: JsonLimits,
}

pub(super) struct ManagedLifecycleEntry {
    pub(super) files: Vec<ManagedLifecycleFileWrite>,
    pub(super) trees: Vec<ManagedLifecyclePluginWrite>,
    pub(super) profile: ConfiguredAdapterProfile,
}

pub(super) struct ManagedLifecycleFileWrite {
    pub(super) path: AbsolutePath,
    pub(super) root: AbsolutePath,
    pub(super) destination: skilltap_core::domain::RelativeArtifactPath,
    pub(super) expected: Option<Vec<u8>>,
    pub(super) desired: Option<Vec<u8>>,
}

pub(super) struct ManagedLifecyclePluginWrite {
    pub(super) root: AbsolutePath,
    pub(super) destination: skilltap_core::domain::RelativeArtifactPath,
    pub(super) desired_tree: Option<ArtifactTree>,
    pub(super) expected_tree: Option<ArtifactTree>,
    pub(super) expected_identity: Option<skilltap_core::runtime::DirectoryIdentity>,
}

fn observe_managed_tree_for_execution(
    filesystem: &dyn ManagedLifecycleFileSystem,
    root: &AbsolutePath,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Result<Option<ObservedManagedTree>, ()> {
    match filesystem.load_tree_bounded_no_follow(
        root,
        destination,
        managed_tree_observation_limits(),
    ) {
        Ok(tree) => Ok(Some(tree)),
        Err(skilltap_core::runtime::RuntimeError::FileSystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(()),
    }
}

impl ExecutionPort for ManagedLifecyclePort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (id, entry) in &self.entries {
            let operation = plan.get(id).ok_or_else(|| {
                managed_lifecycle_apply_failure(
                    "A managed project lifecycle request no longer belongs to the plan.",
                )
            })?;
            if operation.target() != &entry.profile.target
                || operation.scope() != &entry.profile.scope
                || !managed_profile_matches(
                    self.registry,
                    self.config,
                    self.environment,
                    self.search_path.clone(),
                    self.process_limits,
                    self.json_limits,
                    &entry.profile,
                )
            {
                return Err(managed_lifecycle_apply_failure(
                    "The managed projection executable, version, or scoped compiled profile changed after planning.",
                ));
            }
            for file in &entry.files {
                if !operation
                    .affected_surfaces()
                    .iter()
                    .any(|surface| surface.path() == Some(&file.path))
                {
                    return Err(managed_lifecycle_apply_failure(
                        "A managed project file no longer matches the planned surface.",
                    ));
                }
                let current = self
                    .filesystem
                    .read_regular_bounded_no_follow(&file.root, &file.destination, 256 * 1024)
                    .map_err(|_| {
                        managed_lifecycle_apply_failure(
                            "A managed project file could not be re-read safely.",
                        )
                    })?;
                if current != file.expected {
                    return Err(managed_lifecycle_apply_failure(
                        "A managed project file changed after planning.",
                    ));
                }
            }
            for plugin in &entry.trees {
                let current = observe_managed_tree_for_execution(
                    self.filesystem,
                    &plugin.root,
                    &plugin.destination,
                )
                .map_err(|()| {
                    managed_lifecycle_apply_failure(
                        "A managed project skill tree could not be observed within its safety limits.",
                    )
                })?;
                match (&plugin.expected_tree, current) {
                    (None, None) => {}
                    (Some(expected), Some((identity, files))) => {
                        if plugin.expected_identity != Some(identity)
                            || artifact_tree_from_loaded(files).as_ref() != Some(expected)
                        {
                            return Err(managed_lifecycle_apply_failure(
                                "The managed project plugin changed after planning.",
                            ));
                        }
                    }
                    _ => {
                        return Err(managed_lifecycle_apply_failure(
                            "The managed project plugin presence changed after planning.",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let entry = self.entries.get(operation.id()).ok_or_else(|| {
            managed_lifecycle_apply_failure(
                "The managed project lifecycle adapter did not receive the planned request.",
            )
        })?;
        if matches!(
            operation.action(),
            OperationAction::PluginInstall | OperationAction::PluginUpdate
        ) && entry.trees.is_empty()
            && entry.files.is_empty()
        {
            return Err(managed_lifecycle_apply_failure(
                "The managed project plugin operation has no plugin tree request.",
            ));
        }
        if entry.files.iter().all(|file| file.expected == file.desired)
            && entry
                .trees
                .iter()
                .all(|tree| tree.expected_tree == tree.desired_tree)
        {
            return Ok(OperationOutcome::NoChange);
        }

        let mut applied_trees: Vec<&ManagedLifecyclePluginWrite> = Vec::new();
        for plugin in &entry.trees {
            if let Err(detail) = apply_managed_tree(self.filesystem, plugin) {
                let mut attempted = applied_trees.clone();
                attempted.push(plugin);
                let residuals = rollback_managed(self.filesystem, &[], &attempted);
                return Err(managed_rollback_failure(detail, residuals));
            }
            applied_trees.push(plugin);
        }

        let mut applied_files: Vec<&ManagedLifecycleFileWrite> = Vec::new();
        for file in &entry.files {
            if match &file.desired {
                Some(bytes) => self
                    .filesystem
                    .atomic_write_beneath_no_follow(&file.root, &file.destination, bytes)
                    .is_err(),
                None => self
                    .filesystem
                    .remove_file_beneath_no_follow(&file.root, &file.destination)
                    .is_err(),
            } {
                let residuals = rollback_managed(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_rollback_failure(
                    "A managed project file could not be published.",
                    residuals,
                ));
            }
            applied_files.push(file);
        }
        for file in &entry.files {
            if self
                .filesystem
                .read_regular_bounded_no_follow(&file.root, &file.destination, 256 * 1024)
                .ok()
                != Some(file.desired.clone())
            {
                let residuals = rollback_managed(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_rollback_failure(
                    "Managed project file verification failed.",
                    residuals,
                ));
            }
        }
        for tree in &entry.trees {
            let observed =
                observe_managed_tree_for_execution(self.filesystem, &tree.root, &tree.destination)
                    .ok()
                    .flatten()
                    .and_then(|(_, files)| artifact_tree_from_loaded(files));
            if observed != tree.desired_tree {
                let residuals = rollback_managed(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_rollback_failure(
                    "Managed project skill verification failed.",
                    residuals,
                ));
            }
        }
        Ok(OperationOutcome::Applied)
    }
}

fn apply_managed_tree(
    filesystem: &dyn ManagedLifecycleFileSystem,
    plugin: &ManagedLifecyclePluginWrite,
) -> Result<(), &'static str> {
    match (&plugin.expected_tree, &plugin.desired_tree) {
        (None, Some(tree)) => filesystem
            .publish_tree_no_follow(&plugin.root, &plugin.destination, tree.files())
            .map(|_| ())
            .map_err(|_| "The managed project plugin could not be published."),
        (Some(_), None) => {
            let identity = plugin
                .expected_identity
                .ok_or("The managed project plugin has no owned identity.")?;
            filesystem
                .remove_tree_no_follow(&plugin.root, &plugin.destination, identity)
                .map(|_| ())
                .map_err(|_| "The managed project plugin could not be removed safely.")
        }
        (Some(previous), Some(next)) if previous != next => {
            let identity = plugin
                .expected_identity
                .ok_or("The managed project plugin has no owned identity.")?;
            filesystem
                .remove_tree_no_follow(&plugin.root, &plugin.destination, identity)
                .map_err(|_| "The managed project plugin could not be replaced safely.")?;
            if filesystem
                .publish_tree_no_follow(&plugin.root, &plugin.destination, next.files())
                .is_err()
            {
                let _ = filesystem.publish_tree_no_follow(
                    &plugin.root,
                    &plugin.destination,
                    previous.files(),
                );
                return Err("The replacement project plugin could not be published.");
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn rollback_managed(
    filesystem: &dyn ManagedLifecycleFileSystem,
    files: &[&ManagedLifecycleFileWrite],
    trees: &[&ManagedLifecyclePluginWrite],
) -> Vec<String> {
    let mut residuals = Vec::new();
    for file in files.iter().rev() {
        let restored = match &file.expected {
            Some(bytes) => {
                filesystem.atomic_write_beneath_no_follow(&file.root, &file.destination, bytes)
            }
            None => filesystem.remove_file_beneath_no_follow(&file.root, &file.destination),
        };
        if restored.is_err()
            || filesystem
                .read_regular_bounded_no_follow(&file.root, &file.destination, 256 * 1024)
                .ok()
                != Some(file.expected.clone())
        {
            residuals.push(file.path.as_str().to_owned());
        }
    }
    for tree in trees.iter().rev() {
        let current =
            match observe_managed_tree_for_execution(filesystem, &tree.root, &tree.destination) {
                Ok(current) => current,
                Err(_) => {
                    residuals.push(managed_tree_path(tree));
                    continue;
                }
            };
        if let Some((identity, _)) = current
            && filesystem
                .remove_tree_no_follow(&tree.root, &tree.destination, identity)
                .is_err()
        {
            residuals.push(managed_tree_path(tree));
            continue;
        }
        if let Some(previous) = &tree.expected_tree {
            let _ =
                filesystem.publish_tree_no_follow(&tree.root, &tree.destination, previous.files());
        }
        let observed =
            observe_managed_tree_for_execution(filesystem, &tree.root, &tree.destination)
                .ok()
                .flatten()
                .and_then(|(_, files)| artifact_tree_from_loaded(files));
        if observed != tree.expected_tree {
            residuals.push(managed_tree_path(tree));
        }
    }
    residuals.sort();
    residuals.dedup();
    residuals
}

fn managed_tree_path(tree: &ManagedLifecyclePluginWrite) -> String {
    format!("{}/{}", tree.root.as_str(), tree.destination.as_str())
}

fn managed_rollback_failure(detail: &'static str, residuals: Vec<String>) -> ExecutionError {
    if residuals.is_empty() {
        managed_lifecycle_apply_failure(format!("{detail} Rollback restored every prior surface."))
    } else {
        let total = residuals.len();
        let mut listed = Vec::new();
        let mut used = detail.len() + 96;
        for residual in residuals {
            if used + residual.len() + 2 > 900 {
                break;
            }
            used += residual.len() + 2;
            listed.push(residual);
        }
        let omitted = total.saturating_sub(listed.len());
        let suffix = if omitted == 0 {
            String::new()
        } else {
            format!("; {omitted} additional residual surfaces require fresh observation")
        };
        managed_lifecycle_apply_failure(format!(
            "{detail} Rollback left {total} residual surfaces: {}{suffix}.",
            listed.join(", ")
        ))
    }
}

pub(super) struct HybridLifecyclePort<'a> {
    pub(super) native: NativeLifecyclePort,
    pub(super) managed: ManagedLifecyclePort<'a>,
}

impl ExecutionPort for HybridLifecyclePort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        self.native.revalidate(plan)?;
        self.managed.revalidate(plan)
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        if self.managed.entries.contains_key(operation.id()) {
            self.managed.apply(operation)
        } else {
            self.native.apply(operation)
        }
    }
}

fn artifact_tree_from_loaded(
    files: BTreeMap<
        skilltap_core::domain::RelativeArtifactPath,
        skilltap_core::domain::ArtifactFile,
    >,
) -> Option<ArtifactTree> {
    ArtifactTree::new(
        files
            .into_iter()
            .map(|(path, file)| (path.as_str().to_owned(), file)),
    )
    .ok()
}

fn managed_lifecycle_apply_failure(detail: impl Into<String>) -> ExecutionError {
    let detail = skilltap_core::domain::EvidenceDetail::new(detail.into()).unwrap_or_else(|_| {
        skilltap_core::domain::EvidenceDetail::new(
            "Managed project lifecycle failed and residual surfaces require fresh observation.",
        )
        .expect("static evidence detail is valid")
    });
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.project_lifecycle_failed")
            .expect("static evidence code is valid"),
        detail,
    ))
}

pub(super) struct ManagedSkillEntry {
    pub(super) root: AbsolutePath,
    pub(super) destination: skilltap_core::domain::RelativeArtifactPath,
    pub(super) tree: ArtifactTree,
    pub(super) backup_tree: Option<ArtifactTree>,
    pub(super) action: ManagedSkillAction,
    pub(super) expected_identity: Option<skilltap_core::runtime::DirectoryIdentity>,
    pub(super) owner: Option<ResourceKey>,
    pub(super) config_root: Option<AbsolutePath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ManagedSkillAction {
    Install,
    Replace,
    Remove,
}

#[allow(clippy::result_large_err)]
impl ProjectSkillLinkPort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for id in &self.foreign_operations {
            if plan.get(id).is_none() {
                return Err(project_skill_link_failure(
                    "A declared project skill foreign operation is absent from the plan.",
                ));
            }
        }
        for (id, entry) in &self.entries {
            let operation = plan.get(id).ok_or_else(|| {
                project_skill_link_failure(
                    "The project skill link request no longer belongs to the plan.",
                )
            })?;
            let expected_path = AbsolutePath::new(format!(
                "{}/{}",
                entry.root.as_str(),
                entry.destination.as_str()
            ))
            .map_err(|_| project_skill_link_failure("The project skill link path is invalid."))?;
            if !operation
                .affected_surfaces()
                .iter()
                .any(|surface| surface.path() == Some(&expected_path))
            {
                return Err(project_skill_link_failure(
                    "The project skill link destination no longer matches the plan.",
                ));
            }
            let observed = self
                .filesystem
                .inspect_entry_beneath_no_follow(&entry.root, &entry.destination)
                .map_err(|_| {
                    project_skill_link_failure(
                        "The project skill link destination could not be observed safely.",
                    )
                })?;
            let valid = match &entry.action {
                ProjectSkillLinkAction::Create => {
                    matches!(observed, ConfinedEntryObservation::Missing)
                }
                ProjectSkillLinkAction::Replace {
                    expected_identity,
                    previous_target,
                } => matches!(
                    observed,
                    ConfinedEntryObservation::RelativeSymlink { identity, target }
                        if identity == *expected_identity && target == *previous_target
                ),
                ProjectSkillLinkAction::Remove { expected_identity } => matches!(
                    observed,
                    ConfinedEntryObservation::RelativeSymlink { identity, target }
                        if identity == *expected_identity && target == entry.target
                ),
            };
            if !valid {
                return Err(project_skill_link_failure(
                    "The project skill link changed after planning.",
                ));
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let entry = self.entries.get(operation.id()).ok_or_else(|| {
            project_skill_link_failure(
                "The project skill link adapter did not receive the planned request.",
            )
        })?;
        match &entry.action {
            ProjectSkillLinkAction::Create => self
                .filesystem
                .create_relative_symlink_beneath_no_follow(
                    &entry.root,
                    &entry.destination,
                    &entry.target,
                )
                .map(|_| OperationOutcome::Applied)
                .map_err(|_| {
                    project_skill_link_failure("The project skill link could not be created.")
                }),
            ProjectSkillLinkAction::Remove { expected_identity } => self
                .filesystem
                .remove_relative_symlink_beneath_no_follow(
                    &entry.root,
                    &entry.destination,
                    *expected_identity,
                    &entry.target,
                )
                .map(|_| OperationOutcome::Applied)
                .map_err(|_| {
                    project_skill_link_failure(
                        "The project skill link could not be removed safely.",
                    )
                }),
            ProjectSkillLinkAction::Replace {
                expected_identity,
                previous_target,
            } => {
                self.filesystem
                    .remove_relative_symlink_beneath_no_follow(
                        &entry.root,
                        &entry.destination,
                        *expected_identity,
                        previous_target,
                    )
                    .map_err(|_| {
                        project_skill_link_failure(
                            "The prior project skill link could not be removed safely.",
                        )
                    })?;
                if self
                    .filesystem
                    .create_relative_symlink_beneath_no_follow(
                        &entry.root,
                        &entry.destination,
                        &entry.target,
                    )
                    .is_ok()
                {
                    return Ok(OperationOutcome::Applied);
                }
                let restored = self
                    .filesystem
                    .inspect_entry_beneath_no_follow(&entry.root, &entry.destination)
                    .ok()
                    .is_some_and(|observation| {
                        matches!(observation, ConfinedEntryObservation::Missing)
                    })
                    && self
                        .filesystem
                        .create_relative_symlink_beneath_no_follow(
                            &entry.root,
                            &entry.destination,
                            previous_target,
                        )
                        .is_ok();
                let detail = if restored {
                    "The project skill link replacement failed; the prior owned relative link was restored."
                } else {
                    "The project skill link replacement failed and restoration could not be proven."
                };
                Err(project_skill_link_failure(detail))
            }
        }
    }
}

impl ExecutionPort for ProjectSkillLifecyclePort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        self.canonical.revalidate(plan)?;
        self.links.revalidate(plan)
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        if self.canonical.entries.contains_key(operation.id()) {
            self.canonical.apply(operation)
        } else {
            self.links.apply(operation)
        }
    }
}

fn project_skill_link_failure(detail: impl Into<String>) -> ExecutionError {
    let detail = skilltap_core::domain::EvidenceDetail::new(detail.into()).unwrap_or_else(|_| {
        skilltap_core::domain::EvidenceDetail::new(
            "Project skill link execution failed; fresh observation is required.",
        )
        .expect("static evidence detail is valid")
    });
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.project_skill_link_failed")
            .expect("static evidence code is valid"),
        detail,
    ))
}

impl ExecutionPort for ManagedSkillPort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (_, operation) in plan.iter() {
            if !matches!(
                operation.action(),
                OperationAction::SkillInstall | OperationAction::SkillRemove
            ) {
                continue;
            }
            let Some(entry) = self.entries.get(operation.id()) else {
                // Project link operations use the same skill actions but are
                // validated by ProjectSkillLinkPort in the composite port.
                continue;
            };
            let expected = AbsolutePath::new(format!(
                "{}/{}",
                entry.root.as_str(),
                entry.destination.as_str()
            ))
            .map_err(|_| {
                ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_path_invalid")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill destination could not be represented safely.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?;
            if !operation
                .affected_surfaces()
                .iter()
                .any(|surface| surface.path() == Some(&expected))
            {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_surface_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill destination no longer matches the validated operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let Some(entry) = self.entries.get(operation.id()) else {
            return Err(ExecutionError::revalidation(
                skilltap_core::domain::EvidenceCode::new("managed.skill_request_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The managed skill adapter did not receive a request for a planned operation.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        if entry.action == ManagedSkillAction::Remove {
            let Some(expected) = entry.expected_identity else {
                return Err(managed_skill_apply_failure(
                    "The managed skill removal did not include an owned directory identity.",
                ));
            };
            self.filesystem
                .remove_tree_no_follow(&entry.root, &entry.destination, expected)
                .map(|_| OperationOutcome::Applied)
                .map_err(|_| {
                    managed_skill_apply_failure(
                        "The managed skill tree could not be removed safely.",
                    )
                })
        } else if entry.action == ManagedSkillAction::Replace {
            let Some(expected) = entry.expected_identity else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include an owned directory identity.",
                ));
            };
            let Some(owner) = &entry.owner else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include an ownership record.",
                ));
            };
            let Some(config_root) = &entry.config_root else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include a backup root.",
                ));
            };
            let repository = skilltap_core::storage::FileManagedArtifactRepository::new(
                self.filesystem,
                config_root.clone(),
            )
            .map_err(|_| {
                managed_skill_apply_failure(
                    "The managed skill backup repository could not be opened.",
                )
            })?;
            let Some(backup_tree) = &entry.backup_tree else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include the previous tree.",
                ));
            };
            repository.backup(owner, backup_tree).map_err(|_| {
                managed_skill_apply_failure(
                    "The existing skill tree could not be backed up safely.",
                )
            })?;
            self.filesystem
                .remove_tree_no_follow(&entry.root, &entry.destination, expected)
                .map_err(|_| {
                    managed_skill_apply_failure(
                        "The existing skill tree could not be removed safely.",
                    )
                })?;
            match self.filesystem.publish_tree_no_follow(
                &entry.root,
                &entry.destination,
                entry.tree.files(),
            ) {
                Ok(skilltap_core::runtime::DirectoryPublishOutcome::Published(_)) => {
                    Ok(OperationOutcome::Applied)
                }
                Ok(skilltap_core::runtime::DirectoryPublishOutcome::AlreadyExists) => {
                    Ok(OperationOutcome::NoChange)
                }
                Err(_) => {
                    let restored = restore_managed_skill_tree(
                        self.filesystem,
                        &entry.root,
                        &entry.destination,
                        backup_tree,
                    );
                    let destination =
                        format!("{}/{}", entry.root.as_str(), entry.destination.as_str());
                    let detail = if restored {
                        format!(
                            "The replacement skill tree could not be published after backup. The prior managed skill was restored at `{destination}`."
                        )
                    } else {
                        format!(
                            "The replacement skill tree could not be published after backup, and restoration could not be proven. The managed destination `{destination}` requires recovery before retrying."
                        )
                    };
                    Err(managed_skill_apply_failure(detail))
                }
            }
        } else {
            match self
                .filesystem
                .publish_tree_no_follow(&entry.root, &entry.destination, entry.tree.files())
                .map_err(|_| {
                    managed_skill_apply_failure("The managed skill tree could not be published.")
                })? {
                skilltap_core::runtime::DirectoryPublishOutcome::Published(_) => {
                    Ok(OperationOutcome::Applied)
                }
                skilltap_core::runtime::DirectoryPublishOutcome::AlreadyExists => {
                    let (_, files) = self
                        .filesystem
                        .load_tree_no_follow(&entry.root, &entry.destination)
                        .map_err(|_| {
                            managed_skill_apply_failure(
                                "The existing managed skill tree could not be re-read safely.",
                            )
                        })?;
                    let current = ArtifactTree::new(
                        files
                            .into_iter()
                            .map(|(path, bytes)| (path.as_str().to_owned(), bytes)),
                    )
                    .map_err(|_| {
                        managed_skill_apply_failure(
                            "The existing managed skill tree had an invalid shape.",
                        )
                    })?;
                    if current == entry.tree {
                        Ok(OperationOutcome::NoChange)
                    } else {
                        Err(managed_skill_apply_failure(
                            "The managed skill destination changed before publication.",
                        ))
                    }
                }
            }
        }
    }
}

fn restore_managed_skill_tree(
    filesystem: &dyn DirectoryTreeFileSystem,
    root: &AbsolutePath,
    destination: &skilltap_core::domain::RelativeArtifactPath,
    backup_tree: &ArtifactTree,
) -> bool {
    let published_identity =
        match filesystem.publish_tree_no_follow(root, destination, backup_tree.files()) {
            Ok(skilltap_core::runtime::DirectoryPublishOutcome::Published(identity)) => {
                Some(identity)
            }
            Ok(skilltap_core::runtime::DirectoryPublishOutcome::AlreadyExists) => None,
            Err(_) => return false,
        };
    let Ok((observed_identity, files)) = filesystem.load_tree_no_follow(root, destination) else {
        return false;
    };
    if published_identity.is_some_and(|identity| identity != observed_identity) {
        return false;
    }
    artifact_tree_from_loaded(files).as_ref() == Some(backup_tree)
}

fn managed_skill_apply_failure(detail: impl Into<String>) -> ExecutionError {
    let detail = skilltap_core::domain::EvidenceDetail::new(detail.into()).unwrap_or_else(|_| {
        skilltap_core::domain::EvidenceDetail::new(
            "Managed skill publication failed and the destination requires fresh observation.",
        )
        .expect("static evidence detail is valid")
    });
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.skill_publish_failed")
            .expect("static evidence code is valid"),
        detail,
    ))
}

pub(super) enum InstructionWrite {
    Canonical,
    Symlink { target: RelativeSymlinkTarget },
    Import { contents: Vec<u8> },
    Remove,
}

pub(super) struct InstructionPort<'a> {
    pub(super) filesystem: &'a dyn FileSystem,
    pub(super) entries: BTreeMap<OperationId, InstructionEntry>,
}

pub(super) struct InstructionEntry {
    pub(super) path: AbsolutePath,
    pub(super) write: InstructionWrite,
    pub(super) action: OperationAction,
    pub(super) backup: Option<AbsolutePath>,
}

impl ExecutionPort for InstructionPort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (_, operation) in plan.iter() {
            if !matches!(
                operation.action(),
                OperationAction::InstructionSetup | OperationAction::InstructionRepair
            ) {
                continue;
            }
            let Some(entry) = self.entries.get(operation.id()) else {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.request_missing")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            };
            if entry.action != operation.action() {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.action_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction operation action no longer matches the validated adapter entry.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
            if !operation
                .affected_surfaces()
                .iter()
                .any(|surface| surface.path() == Some(&entry.path))
            {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.surface_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction destination no longer matches the validated operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let Some(entry) = self.entries.get(operation.id()) else {
            return Err(ExecutionError::revalidation(
                skilltap_core::domain::EvidenceCode::new("instructions.request_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The instruction adapter did not receive a request for a planned operation.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        if matches!(&entry.write, InstructionWrite::Remove) {
            if let Some(backup) = &entry.backup {
                let backup_parent = backup
                    .as_str()
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .and_then(|parent| AbsolutePath::new(parent).ok())
                    .ok_or_else(|| {
                        instruction_apply_failure("The instruction backup path is invalid.")
                    })?;
                self.filesystem
                    .create_directory_all(&backup_parent)
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The existing instruction bridge could not be backed up safely.",
                        )
                    })?;
                self.filesystem
                    .copy_recoverable(&entry.path, backup)
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The existing instruction bridge could not be backed up safely.",
                        )
                    })?;
            }
            self.filesystem.remove(&entry.path).map_err(|_| {
                instruction_apply_failure(
                    "The duplicate instruction bridge could not be removed safely.",
                )
            })?;
            return Ok(OperationOutcome::Applied);
        }
        let parent = entry
            .path
            .as_str()
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .and_then(|parent| AbsolutePath::new(parent).ok())
            .ok_or_else(|| instruction_apply_failure("The instruction parent path is invalid."))?;
        self.filesystem.create_directory_all(&parent).map_err(|_| {
            instruction_apply_failure("The instruction parent directory could not be created.")
        })?;
        if let Some(backup) = &entry.backup {
            let backup_parent = backup
                .as_str()
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .and_then(|parent| AbsolutePath::new(parent).ok())
                .ok_or_else(|| {
                    instruction_apply_failure("The instruction backup path is invalid.")
                })?;
            self.filesystem
                .create_directory_all(&backup_parent)
                .map_err(|_| {
                    instruction_apply_failure(
                        "The instruction backup directory could not be created.",
                    )
                })?;
            self.filesystem
                .copy_recoverable(&entry.path, backup)
                .map_err(|_| {
                    instruction_apply_failure(
                        "The existing instruction bridge could not be backed up safely.",
                    )
                })?;
            self.filesystem.remove(&entry.path).map_err(|_| {
                instruction_apply_failure(
                    "The existing instruction bridge could not be replaced safely.",
                )
            })?;
        } else if entry.action == OperationAction::InstructionRepair
            && self
                .filesystem
                .inspect(&entry.path)
                .map(|metadata| metadata.kind() == FileKind::Symlink)
                .unwrap_or(false)
        {
            self.filesystem.remove(&entry.path).map_err(|_| {
                instruction_apply_failure(
                    "The divergent instruction symlink could not be replaced safely.",
                )
            })?;
        }
        match &entry.write {
            InstructionWrite::Canonical => {
                self.filesystem
                    .atomic_write(&entry.path, &[])
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The canonical instruction file could not be created.",
                        )
                    })?
            }
            InstructionWrite::Symlink { target } => self
                .filesystem
                .create_relative_symlink(target, &entry.path)
                .map_err(|_| {
                    instruction_apply_failure("The instruction bridge could not be created.")
                })?,
            InstructionWrite::Import { contents } => self
                .filesystem
                .atomic_write(&entry.path, contents)
                .map_err(|_| {
                    instruction_apply_failure("The instruction import bridge could not be created.")
                })?,
            InstructionWrite::Remove => unreachable!("remove entries return before publication"),
        }
        Ok(OperationOutcome::Applied)
    }
}

fn instruction_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("instructions.publish_failed")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(detail)
            .expect("static evidence detail is valid"),
    ))
}
