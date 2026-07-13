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
        let current = if current.resources().contains_key(resource) {
            if let Some(seed) = self.seeds.get(resource) {
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
            } else {
                current
            }
        } else if let Some(seed) = self.seeds.get(resource) {
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
        } else {
            return Err(ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.resource_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The operation resource is not present in state.",
                )
                .expect("static evidence detail is valid"),
            ));
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

pub(super) struct ManagedSkillPort<'a> {
    pub(super) filesystem: &'a dyn DirectoryTreeFileSystem,
    pub(super) entries: BTreeMap<OperationId, ManagedSkillEntry>,
}

pub(super) trait ManagedProjectFileSystem: FileSystem + DirectoryTreeFileSystem {}

impl<T: FileSystem + DirectoryTreeFileSystem> ManagedProjectFileSystem for T {}

pub(super) struct ManagedProjectLifecyclePort<'a> {
    pub(super) filesystem: &'a dyn ManagedProjectFileSystem,
    pub(super) entries: BTreeMap<OperationId, ManagedProjectLifecycleEntry>,
}

pub(super) struct ManagedProjectLifecycleEntry {
    pub(super) files: Vec<ManagedProjectFileWrite>,
    pub(super) trees: Vec<ManagedProjectPluginWrite>,
}

pub(super) struct ManagedProjectFileWrite {
    pub(super) path: AbsolutePath,
    pub(super) expected: Option<Vec<u8>>,
    pub(super) desired: Option<Vec<u8>>,
}

pub(super) struct ManagedProjectPluginWrite {
    pub(super) root: AbsolutePath,
    pub(super) destination: skilltap_core::domain::RelativeArtifactPath,
    pub(super) desired_tree: Option<ArtifactTree>,
    pub(super) expected_tree: Option<ArtifactTree>,
    pub(super) expected_identity: Option<skilltap_core::runtime::DirectoryIdentity>,
}

impl ExecutionPort for ManagedProjectLifecyclePort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (id, entry) in &self.entries {
            let operation = plan.get(id).ok_or_else(|| {
                managed_project_apply_failure(
                    "A managed project lifecycle request no longer belongs to the plan.",
                )
            })?;
            for file in &entry.files {
                if !operation
                    .affected_surfaces()
                    .iter()
                    .any(|surface| surface.path() == Some(&file.path))
                {
                    return Err(managed_project_apply_failure(
                        "A managed project file no longer matches the planned surface.",
                    ));
                }
                let current = self
                    .filesystem
                    .read_regular_no_follow(&file.path)
                    .map_err(|_| {
                        managed_project_apply_failure(
                            "A managed project file could not be re-read safely.",
                        )
                    })?;
                if current != file.expected {
                    return Err(managed_project_apply_failure(
                        "A managed project file changed after planning.",
                    ));
                }
            }
            for plugin in &entry.trees {
                let current = self
                    .filesystem
                    .load_tree_no_follow(&plugin.root, &plugin.destination)
                    .ok();
                match (&plugin.expected_tree, current) {
                    (None, None) => {}
                    (Some(expected), Some((identity, files))) => {
                        if plugin.expected_identity != Some(identity)
                            || artifact_tree_from_loaded(files).as_ref() != Some(expected)
                        {
                            return Err(managed_project_apply_failure(
                                "The managed project plugin changed after planning.",
                            ));
                        }
                    }
                    _ => {
                        return Err(managed_project_apply_failure(
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
            managed_project_apply_failure(
                "The managed project lifecycle adapter did not receive the planned request.",
            )
        })?;
        if matches!(
            operation.action(),
            OperationAction::PluginInstall | OperationAction::PluginUpdate
        ) && entry.trees.is_empty()
            && entry.files.is_empty()
        {
            return Err(managed_project_apply_failure(
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

        let mut applied_trees: Vec<&ManagedProjectPluginWrite> = Vec::new();
        for plugin in &entry.trees {
            let result = (|| -> Result<(), ExecutionError> {
                match (&plugin.expected_tree, &plugin.desired_tree) {
                    (None, Some(tree)) => {
                        self.filesystem
                            .publish_tree_no_follow(&plugin.root, &plugin.destination, tree.files())
                            .map_err(|_| {
                                managed_project_apply_failure(
                                    "The managed project plugin could not be published.",
                                )
                            })?;
                    }
                    (Some(_), None) => {
                        let identity = plugin.expected_identity.ok_or_else(|| {
                            managed_project_apply_failure(
                                "The managed project plugin has no owned identity.",
                            )
                        })?;
                        self.filesystem
                            .remove_tree_no_follow(&plugin.root, &plugin.destination, identity)
                            .map_err(|_| {
                                managed_project_apply_failure(
                                    "The managed project plugin could not be removed safely.",
                                )
                            })?;
                    }
                    (Some(previous), Some(next)) if previous != next => {
                        let identity = plugin.expected_identity.ok_or_else(|| {
                            managed_project_apply_failure(
                                "The managed project plugin has no owned identity.",
                            )
                        })?;
                        self.filesystem
                            .remove_tree_no_follow(&plugin.root, &plugin.destination, identity)
                            .map_err(|_| {
                                managed_project_apply_failure(
                                    "The managed project plugin could not be replaced safely.",
                                )
                            })?;
                        if self
                            .filesystem
                            .publish_tree_no_follow(&plugin.root, &plugin.destination, next.files())
                            .is_err()
                        {
                            let _ = self.filesystem.publish_tree_no_follow(
                                &plugin.root,
                                &plugin.destination,
                                previous.files(),
                            );
                            return Err(managed_project_apply_failure(
                                "The replacement project plugin could not be published.",
                            ));
                        }
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(error) = result {
                rollback_managed_project(self.filesystem, &[], &applied_trees);
                return Err(error);
            }
            applied_trees.push(plugin);
        }

        let mut applied_files: Vec<&ManagedProjectFileWrite> = Vec::new();
        for file in &entry.files {
            let parent = file
                .path
                .as_str()
                .rsplit_once('/')
                .and_then(|(parent, _)| AbsolutePath::new(parent).ok())
                .ok_or_else(|| {
                    managed_project_apply_failure("A managed project file parent is invalid.")
                })?;
            if self.filesystem.create_directory_all(&parent).is_err()
                || match &file.desired {
                    Some(bytes) => self.filesystem.atomic_write(&file.path, bytes).is_err(),
                    None => self.filesystem.remove(&file.path).is_err(),
                }
            {
                rollback_managed_project(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_project_apply_failure(
                    "A managed project file could not be published; earlier changes were rolled back.",
                ));
            }
            applied_files.push(file);
        }
        for file in &entry.files {
            if self.filesystem.read_regular_no_follow(&file.path).ok() != Some(file.desired.clone())
            {
                rollback_managed_project(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_project_apply_failure(
                    "Managed project file verification failed.",
                ));
            }
        }
        for tree in &entry.trees {
            let observed = self
                .filesystem
                .load_tree_no_follow(&tree.root, &tree.destination)
                .ok()
                .and_then(|(_, files)| artifact_tree_from_loaded(files));
            if observed != tree.desired_tree {
                rollback_managed_project(self.filesystem, &applied_files, &applied_trees);
                return Err(managed_project_apply_failure(
                    "Managed project skill verification failed.",
                ));
            }
        }
        Ok(OperationOutcome::Applied)
    }
}

fn rollback_managed_project(
    filesystem: &dyn ManagedProjectFileSystem,
    files: &[&ManagedProjectFileWrite],
    trees: &[&ManagedProjectPluginWrite],
) {
    for file in files.iter().rev() {
        match &file.expected {
            Some(bytes) => {
                let _ = filesystem.atomic_write(&file.path, bytes);
            }
            None => {
                let _ = filesystem.remove(&file.path);
            }
        }
    }
    for tree in trees.iter().rev() {
        if let Ok((identity, _)) = filesystem.load_tree_no_follow(&tree.root, &tree.destination) {
            let _ = filesystem.remove_tree_no_follow(&tree.root, &tree.destination, identity);
        }
        if let Some(previous) = &tree.expected_tree {
            let _ =
                filesystem.publish_tree_no_follow(&tree.root, &tree.destination, previous.files());
        }
    }
}

pub(super) struct HybridLifecyclePort<'a> {
    pub(super) native: NativeLifecyclePort,
    pub(super) managed: ManagedProjectLifecyclePort<'a>,
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

fn managed_project_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.project_lifecycle_failed")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(detail)
            .expect("static evidence detail is valid"),
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
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_request_missing")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
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
            let backup = repository.backup(owner, backup_tree).map_err(|_| {
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
                    let _ = self.filesystem.publish_tree_no_follow(
                        &entry.root,
                        &entry.destination,
                        backup_tree.files(),
                    );
                    let _ = backup;
                    Err(managed_skill_apply_failure(
                        "The replacement skill tree could not be published after backup.",
                    ))
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

fn managed_skill_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.skill_publish_failed")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(detail)
            .expect("static evidence detail is valid"),
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
