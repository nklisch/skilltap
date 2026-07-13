use super::*;

impl StatusApplication<'_> {
    pub(crate) fn execute_instruction_status(&self, args: &ScopedOutputArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("instructions status") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: TargetArgs::default(),
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
        let enabled = enabled_harnesses(&documents.config);
        if enabled.is_empty() {
            return outcome.with_error(ErrorDetail::new(
                "no_enabled_harnesses",
                "No harness is enabled in skilltap configuration.",
            ));
        }
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
        let filesystem = SystemFileSystem;
        let mode = documents.config.instructions().claude_mode;
        let mut path_count = 0_u64;
        let mut healthy = true;
        for concrete_scope in &scope.resolved {
            let (canonical, bridges) = instruction_locations(&paths, concrete_scope, &enabled);
            let canonical_status = match filesystem.inspect(&canonical) {
                Ok(metadata) => match metadata.kind() {
                    FileKind::Missing => "missing",
                    FileKind::RegularFile => "present",
                    _ => "conflict",
                },
                Err(_) => "unreadable",
            };
            path_count += 1;
            outcome = outcome.with_resource(
                OutputEntry::new(
                    instruction_resource_key(concrete_scope, "canonical", "root")
                        .map(|key| key.to_string())
                        .unwrap_or_else(|| "instructions:canonical".to_owned()),
                    canonical_status,
                )
                .with_field("path", canonical.as_str())
                .with_field("scope", scope_label(concrete_scope)),
            );
            if canonical_status != "present" {
                healthy = false;
                outcome = outcome.with_warning(
                    Warning::new(
                        "instruction_canonical_unhealthy",
                        "The canonical AGENTS.md file is missing or not a regular file.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
            }
            for (target, bridge) in bridges {
                path_count += 1;
                let status = instruction_bridge_status(
                    &filesystem,
                    &canonical,
                    &bridge,
                    concrete_scope,
                    mode,
                );
                outcome = outcome.with_resource(
                    OutputEntry::new(
                        instruction_resource_key(concrete_scope, "bridge", target.as_str())
                            .map(|key| key.to_string())
                            .unwrap_or_else(|| format!("instructions:bridge:{}", target)),
                        status,
                    )
                    .with_field("path", bridge.as_str())
                    .with_field("target", target.as_str())
                    .with_field("scope", scope_label(concrete_scope)),
                );
                if status != "managed" {
                    healthy = false;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "instruction_bridge_unhealthy",
                            "The harness instruction bridge is missing or divergent.",
                        )
                        .with_context("target", target.as_str())
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                }
            }
            if let Scope::Project(project) = concrete_scope
                && enabled.iter().any(|target| target.as_str() == "claude")
            {
                let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
                    .expect("nested project Claude bridge path is valid");
                let nested_exists = filesystem
                    .inspect(&nested)
                    .map(|metadata| metadata.kind() != FileKind::Missing)
                    .unwrap_or(false);
                if nested_exists {
                    let root = AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                        .expect("project Claude bridge path is valid");
                    let root_exists = filesystem
                        .inspect(&root)
                        .map(|metadata| metadata.kind() != FileKind::Missing)
                        .unwrap_or(false);
                    path_count += 1;
                    let nested_status = instruction_bridge_status_with_target(
                        &filesystem,
                        &canonical,
                        &nested,
                        mode,
                        b"@../AGENTS.md\n",
                    );
                    outcome = outcome.with_resource(
                        OutputEntry::new(
                            instruction_resource_key(concrete_scope, "bridge-nested", "claude")
                                .map(|key| key.to_string())
                                .unwrap_or_else(|| "instructions:bridge-nested:claude".to_owned()),
                            if root_exists {
                                "duplicate"
                            } else {
                                nested_status
                            },
                        )
                        .with_field("path", nested.as_str())
                        .with_field("target", "claude")
                        .with_field("scope", scope_label(concrete_scope)),
                    );
                    healthy = false;
                    outcome = outcome.with_warning(Warning::new(
                        "instruction_duplicate_claude_bridge",
                        if root_exists {
                            "Both project Claude instruction locations exist; consolidate to one managed bridge."
                        } else if nested_status == "managed" {
                            "The project uses the nested Claude instruction bridge; setup should preserve that location."
                        } else {
                            "The nested project Claude instruction bridge is missing or divergent."
                        },
                    )
                    .with_context("scope", scope_label(concrete_scope)));
                }
            }
        }
        if healthy {
            outcome.result = ResultClass::Completed;
        } else {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_next_action(NextAction::new(
                "repair_instruction_bridges",
                "Run instructions setup or repair after reviewing the reported paths.",
            ));
        }
        outcome
            .with_summary("scopes", scope.count)
            .with_summary("instruction_paths", path_count)
    }

    pub(super) fn execute_instruction_reconciliation_preview(
        &self,
        requested_scope: &ScopeArgs,
        target: &HarnessId,
        resource: &DesiredResource,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("plan") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: TargetArgs {
                target: Some(skilltap_core::domain::TargetSelection::Only(target.clone())),
            },
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
        let filesystem = SystemFileSystem;
        let Some(concrete_scope) = scope.resolved.first() else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "instruction_scope_unavailable",
                "The instruction resource scope could not be resolved.",
            ));
        };
        let (canonical, bridges) =
            instruction_locations(&paths, concrete_scope, std::slice::from_ref(target));
        let is_canonical = resource.id().as_str().contains(":canonical:");
        let operation_id = format!("reconcile:{target}:{}", resource.key());
        let status;
        let mut path = canonical.clone();
        let mut warning = None;
        if !is_canonical {
            let Some((_, bridge)) = bridges
                .into_iter()
                .find(|(candidate, _)| candidate == target)
            else {
                status = "blocked";
                warning = Some(Warning::new(
                    "instruction_bridge_unavailable",
                    "The selected harness has no supported instruction bridge at this scope.",
                ));
                outcome.result = ResultClass::AttentionRequired;
                outcome = outcome.with_operation(
                    crate::OperationOutcome::new(operation_id, status)
                        .with_field("target", target.as_str())
                        .with_field("scope", scope_label(concrete_scope)),
                );
                if let Some(warning) = warning {
                    outcome = outcome.with_warning(warning);
                }
                return outcome;
            };
            // Project setup preserves a nested-only Claude bridge when the
            // root `CLAUDE.md` does not exist.  The desired resource keeps
            // the stable bridge identity, so preview must resolve the
            // materialized path using the same policy before classifying it.
            path = preferred_instruction_bridge_path(&filesystem, concrete_scope, target, bridge);
        }

        let health = if is_canonical {
            match filesystem.inspect(&path) {
                Ok(metadata) => match metadata.kind() {
                    FileKind::Missing => "missing",
                    FileKind::RegularFile => "managed",
                    _ => "conflict",
                },
                Err(_) => "unreadable",
            }
        } else {
            let nested_project_bridge = matches!(concrete_scope, Scope::Project(_))
                && path.as_str().ends_with("/.claude/CLAUDE.md");
            let expected_bytes =
                if documents.config.instructions().claude_mode == ClaudeInstructionMode::Import {
                    if matches!(concrete_scope, Scope::Global) {
                        b"@~/AGENTS.md\n".as_slice()
                    } else if nested_project_bridge {
                        b"@../AGENTS.md\n".as_slice()
                    } else {
                        b"@AGENTS.md\n".as_slice()
                    }
                } else {
                    &[]
                };
            instruction_bridge_status_with_target(
                &filesystem,
                &canonical,
                &path,
                documents.config.instructions().claude_mode,
                expected_bytes,
            )
        };
        match health {
            "missing" => status = "repair",
            "managed" => status = "no_change",
            _ => {
                status = "blocked";
                warning = Some(
                    Warning::new(
                        if is_canonical {
                            "instruction_canonical_conflict"
                        } else {
                            "instruction_bridge_conflict"
                        },
                        if is_canonical {
                            "The canonical AGENTS.md path is not a regular file; no change was made."
                        } else {
                            "The bridge contains existing content; use sync --yes to repair it."
                        },
                    )
                    .with_context("target", target.as_str())
                    .with_context("path", path.as_str()),
                );
                outcome.result = ResultClass::AttentionRequired;
            }
        }
        outcome = outcome.with_operation(
            crate::OperationOutcome::new(operation_id, status)
                .with_field("target", target.as_str())
                .with_field("scope", scope_label(concrete_scope))
                .with_field("path", path.as_str()),
        );
        if let Some(warning) = warning {
            outcome = outcome.with_warning(warning);
        }
        outcome
    }

    pub(crate) fn execute_instruction_setup(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        mode: Option<ClaudeInstructionMode>,
        acknowledged: bool,
        repair: bool,
    ) -> Outcome {
        self.execute_instruction_setup_for_target(
            command,
            requested_scope,
            mode,
            acknowledged,
            repair,
            None,
        )
    }

    pub(super) fn execute_instruction_setup_for_target(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        mode: Option<ClaudeInstructionMode>,
        acknowledged: bool,
        repair: bool,
        target_filter: Option<&HarnessId>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: TargetArgs::default(),
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
        let mut enabled = enabled_harnesses(&documents.config);
        if let Some(target) = target_filter {
            enabled.retain(|candidate| candidate == target);
        }
        if enabled.is_empty() {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(ErrorDetail::new(
                "no_enabled_harnesses",
                "No harness is enabled in skilltap configuration.",
            ));
        }
        let mode = mode.unwrap_or(documents.config.instructions().claude_mode);
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
        let filesystem = SystemFileSystem;
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let timestamp = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(timestamp) => timestamp,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "clock_unavailable",
                    "The instruction operation timestamp could not be recorded safely.",
                ));
            }
        };
        for concrete_scope in &scope.resolved {
            let (canonical, mut bridges) = instruction_locations(&paths, concrete_scope, &enabled);
            let mut duplicate_nested = None;
            if let Scope::Project(project) = concrete_scope
                && enabled.iter().any(|target| target.as_str() == "claude")
            {
                let root = AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                    .expect("project Claude bridge path is valid");
                let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
                    .expect("nested project Claude bridge path is valid");
                let root_missing = filesystem
                    .inspect(&root)
                    .map(|metadata| metadata.kind() == FileKind::Missing)
                    .unwrap_or(false);
                let nested_present = filesystem
                    .inspect(&nested)
                    .map(|metadata| metadata.kind() != FileKind::Missing)
                    .unwrap_or(false);
                if !root_missing && nested_present {
                    let nested_kind = filesystem
                        .inspect(&nested)
                        .ok()
                        .map(|metadata| metadata.kind());
                    if !matches!(nested_kind, Some(FileKind::RegularFile | FileKind::Symlink)) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome
                            .with_warning(
                                Warning::new(
                                    "instruction_duplicate_bridge_broken",
                                    "The nested project Claude entry is not a removable regular file or symlink; consolidation is blocked.",
                                )
                                .with_context("scope", scope_label(concrete_scope)),
                            )
                            .with_next_action(NextAction::new(
                                "repair_duplicate_bridge_manually",
                                "Replace the broken nested Claude entry with a regular file or symlink, then retry repair.",
                            ));
                        continue;
                    }
                    duplicate_nested = Some(nested.clone());
                    if !(repair && acknowledged) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome
                            .with_warning(
                                Warning::new(
                                    "instruction_duplicate_claude_bridge",
                                    "Both project Claude instruction locations exist; use repair with --yes to consolidate to the root bridge.",
                                )
                                .with_context("scope", scope_label(concrete_scope)),
                            )
                            .with_next_action(NextAction::new(
                                "repair_duplicate_bridge",
                                "Run instructions repair --project --yes to keep the root Claude bridge and remove the nested duplicate.",
                            ));
                        continue;
                    }
                } else if root_missing && nested_present {
                    bridges = vec![(
                        HarnessId::new("claude").expect("known harness id is valid"),
                        nested,
                    )];
                }
            }
            let canonical_id = instruction_operation_id(concrete_scope, "canonical", "root");
            let canonical_resource =
                match instruction_resource_key(concrete_scope, "canonical", "root") {
                    Some(key) => key,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "instruction_resource_invalid",
                            "The instruction resource identifier could not be represented safely.",
                        ));
                    }
                };
            let canonical_missing = match filesystem.inspect(&canonical) {
                Ok(metadata) => match metadata.kind() {
                    FileKind::Missing => true,
                    FileKind::RegularFile => false,
                    _ => {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(Warning::new(
                            "instruction_canonical_conflict",
                            "The canonical AGENTS.md path is not a regular file; no change was made.",
                        ));
                        false
                    }
                },
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(Warning::new(
                        "instruction_canonical_unreadable",
                        "The canonical AGENTS.md path could not be inspected safely.",
                    ));
                    false
                }
            };
            let mut canonical_dependency = None;
            if canonical_missing {
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    canonical_id.clone(),
                    enabled.first().expect("enabled set is non-empty").clone(),
                    canonical_resource.clone(),
                    OperationAction::InstructionSetup,
                    canonical.clone(),
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The canonical instruction operation was invalid.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    canonical_id.clone(),
                    InstructionEntry {
                        path: canonical.clone(),
                        write: InstructionWrite::Canonical,
                        action: OperationAction::InstructionSetup,
                        backup: None,
                    },
                );
                canonical_dependency = Some(canonical_id);
            }
            let canonical_desired = instruction_desired_resource(
                canonical_resource.clone(),
                enabled.first().expect("enabled set is non-empty").clone(),
            );
            // A target-scoped reconciliation may select Claude while the
            // canonical resource was originally seeded under Codex. The
            // canonical key is shared by both harnesses; preserve the
            // existing desired record rather than treating that projection as
            // an inventory conflict.
            if !inventory.resources().contains_key(&canonical_resource) {
                inventory = match inventory.with_resource(canonical_desired) {
                    Ok(inventory) => inventory,
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        return outcome.with_error(ErrorDetail::new(
                            "inventory_resource_conflict",
                            "The canonical instruction resource conflicts with desired state.",
                        ));
                    }
                };
            }
            let canonical_target = TargetResourceState::new(
                enabled.first().expect("enabled set is non-empty").clone(),
                Some(NativeId::new(canonical.as_str()).expect("absolute path is valid native id")),
                Provenance::Direct,
                Ownership::Skilltap,
                None,
                None,
                Some(fingerprint_contents(&[])),
                None,
                None,
                timestamp,
                None,
            );
            let canonical_state = canonical_target
                .and_then(|target| ResourceState::new(canonical_resource, [target]))
                .map_err(|_| ())
                .ok();
            if let Some(state) = canonical_state {
                seeds.insert(state.key().clone(), state);
            }

            if let Some(nested) = duplicate_nested {
                let nested_resource = match instruction_resource_key(
                    concrete_scope,
                    "bridge-nested",
                    "claude",
                ) {
                    Some(key) => key,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "instruction_resource_invalid",
                            "The duplicate instruction resource identifier could not be represented safely.",
                        ));
                    }
                };
                let nested_id = instruction_operation_id(concrete_scope, "bridge-nested", "claude");
                let nested_operation =
                    match skilltap_core::lifecycle_operation::faithful_file_operation(
                        nested_id.clone(),
                        HarnessId::new("claude").expect("known harness id is valid"),
                        nested_resource.clone(),
                        OperationAction::InstructionRepair,
                        nested.clone(),
                    ) {
                        Ok(operation) => operation,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The duplicate instruction removal operation was invalid.",
                            ));
                        }
                    };
                let nested_metadata = filesystem.inspect(&nested).ok();
                let backup = nested_metadata
                    .as_ref()
                    .filter(|metadata| metadata.kind() == FileKind::RegularFile)
                    .map(|_| instruction_backup_path(&paths, &nested));
                let nested_bytes = nested_metadata
                    .as_ref()
                    .filter(|metadata| metadata.kind() == FileKind::RegularFile)
                    .and_then(|_| filesystem.read(&nested).ok())
                    .unwrap_or_default();
                let nested_target = TargetResourceState::new(
                    HarnessId::new("claude").expect("known harness id is valid"),
                    Some(NativeId::new(nested.as_str()).expect("absolute path is valid native id")),
                    Provenance::Direct,
                    Ownership::Skilltap,
                    None,
                    None,
                    Some(fingerprint_contents(&nested_bytes)),
                    None,
                    None,
                    timestamp,
                    None,
                );
                let nested_state = nested_target
                    .and_then(|target| ResourceState::new(nested_resource, [target]))
                    .map_err(|_| ())
                    .ok();
                operations.push(nested_operation);
                entries.insert(
                    nested_id,
                    InstructionEntry {
                        path: nested,
                        write: InstructionWrite::Remove,
                        action: OperationAction::InstructionRepair,
                        backup,
                    },
                );
                if let Some(state) = nested_state {
                    seeds.insert(state.key().clone(), state);
                }
                outcome = outcome.with_warning(Warning::new(
                    "instruction_bridge_consolidation",
                    "The root project Claude bridge is canonical; the nested duplicate will be backed up and removed.",
                ));
            }

            for (target, bridge) in bridges {
                let nested_project_bridge = matches!(concrete_scope, Scope::Project(_))
                    && bridge.as_str().ends_with("/.claude/CLAUDE.md");
                let expected_symlink = relative_symlink_target(&bridge, &canonical);
                let (write, expected_bytes) = match mode {
                    ClaudeInstructionMode::Symlink => (
                        InstructionWrite::Symlink {
                            target: match expected_symlink.clone() {
                                Ok(target) => target,
                                Err(_) => {
                                    outcome.result = ResultClass::Invalid;
                                    return outcome.with_error(ErrorDetail::new(
                                        "instruction_bridge_path_invalid",
                                        "The instruction bridge could not be related safely to the canonical AGENTS.md path.",
                                    ));
                                }
                            },
                        },
                        Vec::new(),
                    ),
                    ClaudeInstructionMode::Import => {
                        let bytes = if matches!(concrete_scope, Scope::Global) {
                            b"@~/AGENTS.md\n".to_vec()
                        } else if nested_project_bridge {
                            b"@../AGENTS.md\n".to_vec()
                        } else {
                            b"@AGENTS.md\n".to_vec()
                        };
                        (
                            InstructionWrite::Import {
                                contents: bytes.clone(),
                            },
                            bytes,
                        )
                    }
                };
                let bridge_health = match instruction_bridge_status_with_target(
                    &filesystem,
                    &canonical,
                    &bridge,
                    mode,
                    &expected_bytes,
                ) {
                    "missing" => InstructionBridgeHealth::Missing,
                    "managed" => InstructionBridgeHealth::Managed,
                    _ => InstructionBridgeHealth::Conflict,
                };
                let bridge_resource =
                    match instruction_resource_key(concrete_scope, "bridge", target.as_str()) {
                        Some(key) => key,
                        None => continue,
                    };
                let desired = instruction_desired_resource(bridge_resource.clone(), target.clone());
                inventory = match inventory.with_resource(desired) {
                    Ok(inventory) => inventory,
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        return outcome.with_error(ErrorDetail::new(
                            "inventory_resource_conflict",
                            "The instruction bridge conflicts with desired state.",
                        ));
                    }
                };
                let observed_bytes = match &write {
                    InstructionWrite::Import { contents } => contents.clone(),
                    InstructionWrite::Canonical | InstructionWrite::Symlink { .. } => Vec::new(),
                    InstructionWrite::Remove => Vec::new(),
                };
                let bridge_target = TargetResourceState::new(
                    target.clone(),
                    Some(NativeId::new(bridge.as_str()).expect("absolute path is valid native id")),
                    Provenance::Direct,
                    Ownership::Skilltap,
                    None,
                    None,
                    Some(fingerprint_contents(&observed_bytes)),
                    None,
                    None,
                    timestamp,
                    None,
                );
                let bridge_state = bridge_target
                    .and_then(|target| ResourceState::new(bridge_resource.clone(), [target]))
                    .map_err(|_| ())
                    .ok();
                if let Some(state) = bridge_state {
                    seeds.insert(state.key().clone(), state);
                }
                if bridge_health == InstructionBridgeHealth::Managed {
                    outcome = outcome.with_operation(crate::OperationOutcome::new(
                        format!("instruction:{}:{}", target, scope_label(concrete_scope)),
                        "no_change",
                    ));
                    continue;
                }
                let bridge_kind = filesystem
                    .inspect(&bridge)
                    .ok()
                    .map(|metadata| metadata.kind());
                if bridge_health == InstructionBridgeHealth::Conflict {
                    let repairable = repair
                        && acknowledged
                        && matches!(bridge_kind, Some(FileKind::RegularFile | FileKind::Symlink));
                    if repairable {
                        // Regular files are backed up before replacement.
                        // Symlinks are themselves the conflicting entry and
                        // are removed without following their target.
                    } else {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "instruction_bridge_conflict",
                                if repair {
                                    "The bridge requires --yes and must be a divergent regular file or symlink before repair."
                                } else {
                                    "The bridge contains existing content; use instructions repair with --yes."
                                },
                            )
                            .with_context("target", target.as_str()),
                        );
                        continue;
                    }
                }
                let repair_operation =
                    repair && acknowledged && bridge_health == InstructionBridgeHealth::Conflict;
                if repair_operation {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "instruction_bridge_repair",
                            if bridge_kind == Some(FileKind::RegularFile) {
                                "The divergent instruction bridge will be backed up before replacement."
                            } else {
                                "The divergent instruction symlink will be replaced without following its target."
                            },
                        )
                        .with_context("target", target.as_str()),
                    );
                }
                let operation_id =
                    instruction_operation_id(concrete_scope, "bridge", target.as_str());
                let operation_action = if repair_operation {
                    OperationAction::InstructionRepair
                } else {
                    OperationAction::InstructionSetup
                };
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
                    operation_id.clone(),
                    target.clone(),
                    bridge_resource,
                    operation_action,
                    bridge.clone(),
                    canonical_dependency
                        .clone()
                        .into_iter()
                        .map(skilltap_core::domain::OperationDependency::new),
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The instruction bridge operation was invalid.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    InstructionEntry {
                        path: bridge.clone(),
                        write,
                        action: operation_action,
                        backup: (repair_operation && bridge_kind == Some(FileKind::RegularFile))
                            .then(|| instruction_backup_path(&paths, &bridge)),
                    },
                );
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
                "The instruction inventory could not be published safely.",
            ));
        }
        if operations.is_empty() {
            if let Err(()) = seed_state_if_missing(self.state, &seeds) {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The instruction state could not be recorded safely.",
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
                    "The instruction operation plan was invalid.",
                ));
            }
        };
        let port = InstructionPort {
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
        if report.changed && outcome.errors.is_empty() && outcome.warnings.is_empty() {
            outcome.result = ResultClass::Completed;
        }
        outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed)
    }
}
