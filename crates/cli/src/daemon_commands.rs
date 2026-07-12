use std::{collections::BTreeMap, ffi::OsString};

use super::repository_composition_error;
use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, ResultClass, Warning, command::OutputArgs,
};
use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, NativeId},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessRequest, NativeProcessRunner,
        ProcessLimits, SystemExecutableResolver, SystemNativeProcessRunner,
    },
    runtime::{FileSystem, PlatformPaths, ProcessEnvironment, SystemFileSystem},
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository, FileStateRepository,
        StateRepository,
    },
};

pub(super) fn execute_system_daemon_enable(args: &crate::command::DaemonEnableArgs) -> Outcome {
    let command = "daemon enable";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let filesystem = SystemFileSystem;
    let config = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone())
        .and_then(|repository| match repository.load() {
            Ok(DocumentState::Present(config)) => Ok(config),
            Ok(DocumentState::Missing) => Ok(ConfigDocument::defaults()),
            Err(error) => Err(error),
        }) {
        Ok(config) => config,
        Err(_) => return repository_composition_error(command),
    };
    let interval = args.interval.unwrap_or(config.updates().interval);
    let platform = crate::daemon::platform(&paths);
    let executable = match std::env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
        .and_then(|path| AbsolutePath::new(path).ok())
    {
        Some(path) => path,
        None => {
            return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                "daemon_executable_unavailable",
                "The skilltap executable path could not be represented safely.",
            ));
        }
    };
    let definition =
        match skilltap_core::daemon::render_service(&skilltap_core::daemon::DaemonServiceSpec {
            platform,
            interval,
            executable,
        }) {
            Ok(definition) => definition,
            Err(_) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                    "daemon_definition_invalid",
                    "The daemon service definition could not be validated.",
                ));
            }
        };
    let files = crate::daemon::files(&paths, &definition);
    let root = crate::daemon::root(&paths, platform);
    if let Err(_error) = filesystem.create_directory_all(&root) {
        return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
            Warning::new(
                "daemon_definition_write_failed",
                "The daemon service directory could not be created.",
            )
            .with_context("path", root.as_str()),
        );
    }
    let mut existing = Vec::with_capacity(files.len());
    for (path, _file) in &files {
        match filesystem.read_regular_no_follow(path) {
            Ok(Some(contents))
                if crate::daemon::owns(platform, &contents)
                    && !crate::daemon::valid(platform, &contents) =>
            {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "malformed"))
                    .with_warning(Warning::new(
                        "daemon_definition_malformed",
                        "An owned daemon service definition is malformed; it was not replaced.",
                    ));
            }
            Ok(Some(contents)) if !crate::daemon::owns(platform, &contents) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "conflict"))
                    .with_warning(Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition already occupies the skilltap path.",
                    ));
            }
            Ok(value) => existing.push(value),
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_unreadable",
                        "The existing daemon service definition could not be read safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
        }
    }
    let changed_files = files
        .iter()
        .zip(existing.iter())
        .filter(|((_, file), current)| current.as_deref() != Some(file.contents().as_bytes()))
        .map(|((path, file), current)| {
            (
                path.clone(),
                file.contents().as_bytes().to_vec(),
                current.clone(),
            )
        })
        .collect::<Vec<_>>();
    if let Err((path, _error)) = publish_daemon_files(&filesystem, &changed_files) {
        return Outcome::new(command, ResultClass::AttentionRequired)
            .with_summary("changed", false)
            .with_warning(
                Warning::new(
                    "daemon_definition_write_failed",
                    "The daemon service definition could not be published atomically; prior files were restored.",
                )
                .with_context("path", path.as_str()),
            );
    }
    if run_service_manager(platform, ServiceManagerAction::Enable, &files[0].0).is_err() {
        return Outcome::new(command, ResultClass::AttentionRequired)
            .with_summary("changed", !changed_files.is_empty())
            .with_warning(Warning::new(
                "daemon_manager_unavailable",
                "The service definition was written, but the user service manager did not activate it.",
            ))
            .with_next_action(NextAction::new(
                "retry_daemon_enable",
                "Retry daemon enable after checking the user service manager.",
            ))
            .with_resource(OutputEntry::new(files[0].0.as_str(), "installed"));
    }
    Outcome::new(command, ResultClass::Completed)
        .with_resource(
            OutputEntry::new(files[0].0.as_str(), "enabled")
                .with_field("interval", interval.to_string())
                .with_field("platform", format!("{platform:?}").to_lowercase()),
        )
        .with_summary("changed", !changed_files.is_empty())
}

type DaemonChangedFile = (AbsolutePath, Vec<u8>, Option<Vec<u8>>);

/// Publish all changed service definitions as one recoverable pair. If a
/// later definition fails, every earlier write is restored to its prior bytes
/// (or removed when it did not previously exist).
fn publish_daemon_files(
    filesystem: &dyn FileSystem,
    changed_files: &[DaemonChangedFile],
) -> Result<(), (AbsolutePath, skilltap_core::runtime::RuntimeError)> {
    let mut written: Vec<&DaemonChangedFile> = Vec::new();
    for changed in changed_files {
        let (path, contents, _previous) = changed;
        if let Err(error) = filesystem.atomic_write(path, contents) {
            for (written_path, _, written_previous) in written.iter().rev().copied() {
                match written_previous {
                    Some(previous) => {
                        let _ = filesystem.atomic_write(written_path, previous);
                    }
                    None => {
                        let _ = filesystem.remove(written_path);
                    }
                }
            }
            return Err((path.clone(), error));
        }
        written.push(changed);
    }
    Ok(())
}

pub(super) fn execute_system_daemon_disable(_args: &OutputArgs) -> Outcome {
    let command = "daemon disable";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let platform = crate::daemon::platform(&paths);
    let root = crate::daemon::root(&paths, platform);
    let names = match platform {
        skilltap_core::daemon::ServicePlatform::Launchd => {
            vec![format!("{}.plist", skilltap_core::daemon::SERVICE_LABEL)]
        }
        skilltap_core::daemon::ServicePlatform::SystemdUser => vec![
            skilltap_core::daemon::SYSTEMD_UNIT.to_owned(),
            skilltap_core::daemon::SYSTEMD_TIMER.to_owned(),
        ],
    };
    let files = names
        .iter()
        .map(|name| AbsolutePath::new(format!("{}/{}", root.as_str(), name)).unwrap())
        .collect::<Vec<_>>();
    let mut owned_present = false;
    for path in &files {
        match SystemFileSystem.read_regular_no_follow(path) {
            Ok(Some(contents))
                if crate::daemon::owns(platform, &contents)
                    && !crate::daemon::valid(platform, &contents) =>
            {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "malformed"))
                    .with_warning(Warning::new(
                        "daemon_definition_malformed",
                        "An owned daemon service definition is malformed; it was not removed.",
                    ));
            }
            Ok(Some(contents)) if crate::daemon::owns(platform, &contents) => {
                owned_present = true;
            }
            Ok(Some(_)) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition occupies the skilltap path; it was not removed.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
            Ok(None) => {}
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_unreadable",
                        "The daemon service definition could not be inspected safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
        }
    }
    if !owned_present {
        return Outcome::new(command, ResultClass::Completed)
            .with_resource(OutputEntry::new(root.as_str(), "disabled"))
            .with_summary("changed", false);
    }
    if run_service_manager(platform, ServiceManagerAction::Disable, &files[0]).is_err() {
        return Outcome::new(command, ResultClass::AttentionRequired).with_warning(Warning::new(
            "daemon_manager_unavailable",
            "The user service manager did not disable the daemon; owned definitions were retained.",
        ));
    }
    let mut changed = false;
    for path in &files {
        if let Ok(Some(contents)) = SystemFileSystem.read_regular_no_follow(path)
            && crate::daemon::owns(platform, &contents)
        {
            if SystemFileSystem.remove(path).is_err() {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_remove_failed",
                        "The owned daemon service definition could not be removed safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
            changed = true;
        }
    }
    Outcome::new(command, ResultClass::Completed)
        .with_resource(OutputEntry::new(root.as_str(), "disabled"))
        .with_summary("changed", changed)
}

pub(super) fn execute_system_daemon_status(_args: &OutputArgs) -> Outcome {
    let command = "daemon status";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let platform = crate::daemon::platform(&paths);
    let root = crate::daemon::root(&paths, platform);
    let names = match platform {
        skilltap_core::daemon::ServicePlatform::Launchd => {
            vec![format!("{}.plist", skilltap_core::daemon::SERVICE_LABEL)]
        }
        skilltap_core::daemon::ServicePlatform::SystemdUser => vec![
            skilltap_core::daemon::SYSTEMD_UNIT.to_owned(),
            skilltap_core::daemon::SYSTEMD_TIMER.to_owned(),
        ],
    };
    let state_record =
        match FileStateRepository::new(&SystemFileSystem, paths.skilltap_config().clone())
            .and_then(|repository| repository.load())
        {
            Ok(DocumentState::Present(state)) => state.daemon_run().cloned(),
            Ok(DocumentState::Missing) => None,
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_warning(Warning::new(
                        "daemon_state_unavailable",
                        "The daemon state document could not be loaded safely.",
                    ))
                    .with_next_action(NextAction::new(
                        "repair_daemon_state",
                        "Repair or remove the malformed skilltap state document before retrying.",
                    ));
            }
        };
    let mut installed = true;
    for name in &names {
        let path = AbsolutePath::new(format!("{}/{}", root.as_str(), name)).unwrap();
        match SystemFileSystem.read_regular_no_follow(&path) {
            Ok(Some(contents))
                if crate::daemon::owns(platform, &contents)
                    && !crate::daemon::valid(platform, &contents) =>
            {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "malformed"))
                    .with_warning(Warning::new(
                        "daemon_definition_malformed",
                        "An owned daemon service definition is malformed; inspect it before retrying.",
                    ));
            }
            Ok(Some(contents)) if crate::daemon::owns(platform, &contents) => {}
            Ok(None) => installed = false,
            Ok(Some(_)) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "conflict"))
                    .with_warning(Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition occupies the skilltap path.",
                    ));
            }
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "unreadable"))
                    .with_warning(Warning::new(
                        "daemon_definition_unreadable",
                        "The daemon service definition could not be inspected safely.",
                    ));
            }
        }
    }
    if !installed {
        let mut entry = OutputEntry::new(root.as_str(), "disabled");
        entry = daemon_record_fields(entry, state_record.as_ref());
        return Outcome::new(command, ResultClass::Completed)
            .with_resource(entry)
            .with_next_action(
                NextAction::new(
                    "enable_daemon",
                    "Enable the optional user daemon before expecting automatic updates.",
                )
                .with_command("skilltap daemon enable"),
            );
    }
    let manager = run_service_manager(
        platform,
        ServiceManagerAction::Status,
        &AbsolutePath::new(format!("{}/{}", root.as_str(), names[0])).unwrap(),
    );
    if manager.is_err() {
        let entry = daemon_record_fields(
            OutputEntry::new(root.as_str(), "installed"),
            state_record.as_ref(),
        );
        return Outcome::new(command, ResultClass::AttentionRequired)
            .with_resource(entry)
            .with_warning(Warning::new(
                "daemon_manager_unavailable",
                "The owned daemon definition exists, but manager state could not be confirmed.",
            ))
            .with_next_action(
                NextAction::new(
                    "retry_daemon_enable",
                    "Retry daemon enable after checking the user service manager.",
                )
                .with_command("skilltap daemon enable"),
            );
    }
    let status = state_record.as_ref().map_or("enabled_never_run", |record| {
        daemon_result_label(record.result())
    });
    let entry = daemon_record_fields(
        OutputEntry::new(root.as_str(), status),
        state_record.as_ref(),
    );
    let mut outcome = Outcome::new(command, ResultClass::Completed).with_resource(entry);
    if let Some(record) = state_record.as_ref()
        && record.result() != skilltap_core::storage::DaemonRunResult::Completed
    {
        outcome = outcome.with_next_action(daemon_recovery_action(record.result()));
    } else if state_record.is_none() {
        outcome = outcome.with_next_action(
            NextAction::new(
                "run_daemon_cycle",
                "Run one bounded daemon cycle to establish update health.",
            )
            .with_command("skilltap daemon run"),
        );
    }
    outcome
}

fn daemon_result_label(result: skilltap_core::storage::DaemonRunResult) -> &'static str {
    match result {
        skilltap_core::storage::DaemonRunResult::Completed => "completed",
        skilltap_core::storage::DaemonRunResult::Pending => "pending",
        skilltap_core::storage::DaemonRunResult::Contended => "contended",
        skilltap_core::storage::DaemonRunResult::Failed => "failed",
    }
}

fn daemon_record_fields(
    mut entry: OutputEntry,
    record: Option<&skilltap_core::storage::DaemonRunRecord>,
) -> OutputEntry {
    let Some(record) = record else { return entry };
    entry = entry
        .with_field("last_run_seconds", record.at().seconds())
        .with_field("run_result", daemon_result_label(record.result()))
        .with_field("safe_operations", record.safe_operations())
        .with_field("pending_operations", record.pending_operations());
    if let Some(code) = record.failure_code() {
        entry = entry.with_field("failure", code.as_str());
    }
    entry
}

fn daemon_recovery_action(result: skilltap_core::storage::DaemonRunResult) -> NextAction {
    match result {
        skilltap_core::storage::DaemonRunResult::Pending => NextAction::new(
            "review_pending_updates",
            "Review pending updates and their decisions before foreground application.",
        )
        .with_command("skilltap status --all-scopes"),
        skilltap_core::storage::DaemonRunResult::Contended => NextAction::new(
            "retry_daemon_cycle",
            "Retry one bounded daemon cycle after the configuration lock is available.",
        )
        .with_command("skilltap daemon run"),
        skilltap_core::storage::DaemonRunResult::Failed => NextAction::new(
            "inspect_daemon_status",
            "Inspect daemon status and retry only after resolving the reported failure.",
        )
        .with_command("skilltap daemon status"),
        skilltap_core::storage::DaemonRunResult::Completed => NextAction::new(
            "run_daemon_cycle",
            "Run one bounded daemon cycle when another update check is needed.",
        )
        .with_command("skilltap daemon run"),
    }
}

#[derive(Clone, Copy)]
enum ServiceManagerAction {
    Enable,
    Disable,
    Status,
}

fn run_service_manager(
    platform: skilltap_core::daemon::ServicePlatform,
    action: ServiceManagerAction,
    definition: &AbsolutePath,
) -> Result<(), ()> {
    let (binary, arguments) = match (platform, action) {
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Enable) => {
            ("launchctl", vec!["load", "-w", definition.as_str()])
        }
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Disable) => {
            ("launchctl", vec!["unload", "-w", definition.as_str()])
        }
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Status) => (
            "launchctl",
            vec!["list", skilltap_core::daemon::SERVICE_LABEL],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Enable) => (
            "systemctl",
            vec![
                "--user",
                "enable",
                "--now",
                skilltap_core::daemon::SYSTEMD_TIMER,
            ],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Disable) => (
            "systemctl",
            vec![
                "--user",
                "disable",
                "--now",
                skilltap_core::daemon::SYSTEMD_TIMER,
            ],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Status) => (
            "systemctl",
            vec!["--user", "is-enabled", skilltap_core::daemon::SYSTEMD_TIMER],
        ),
    };
    let configured =
        ConfiguredBinary::path_lookup(NativeId::new(binary).map_err(|_| ())?).map_err(|_| ())?;
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured,
            std::env::var_os("PATH"),
        ))
        .map_err(|_| ())?;
    let limits = ProcessLimits::new(5_000, 64 * 1024, 64 * 1024, 128 * 1024).map_err(|_| ())?;
    let request = NativeProcessRequest::new(
        executable,
        arguments.into_iter().map(OsString::from),
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&request).map_err(|_| ())?;
    output.status().success().then_some(()).ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap, io};

    struct FailingPublicationFileSystem {
        files: RefCell<BTreeMap<String, Vec<u8>>>,
        fail_path: String,
    }

    impl FailingPublicationFileSystem {
        fn new(fail_path: &str) -> Self {
            Self {
                files: RefCell::new(BTreeMap::new()),
                fail_path: fail_path.to_owned(),
            }
        }
    }

    impl skilltap_core::runtime::FileSystem for FailingPublicationFileSystem {
        fn inspect(
            &self,
            _path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<skilltap_core::runtime::FileMetadata, skilltap_core::runtime::RuntimeError>
        {
            unimplemented!("publication helper does not inspect")
        }

        fn canonicalize(
            &self,
            _path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<skilltap_core::domain::AbsolutePath, skilltap_core::runtime::RuntimeError>
        {
            unimplemented!("publication helper does not canonicalize")
        }

        fn create_directory_all(
            &self,
            _path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<(), skilltap_core::runtime::RuntimeError> {
            unimplemented!("publication helper does not create directories")
        }

        fn read(
            &self,
            _path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<Vec<u8>, skilltap_core::runtime::RuntimeError> {
            unimplemented!("publication helper does not read")
        }

        fn read_regular_no_follow(
            &self,
            _path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<Option<Vec<u8>>, skilltap_core::runtime::RuntimeError> {
            unimplemented!("publication helper does not inspect regular files")
        }

        fn atomic_write(
            &self,
            path: &skilltap_core::domain::AbsolutePath,
            contents: &[u8],
        ) -> Result<(), skilltap_core::runtime::RuntimeError> {
            if path.as_str() == self.fail_path {
                return Err(skilltap_core::runtime::RuntimeError::FileSystem {
                    action: skilltap_core::runtime::FileSystemAction::Write,
                    path: path.clone(),
                    source: io::Error::other("injected second-write failure"),
                });
            }
            self.files
                .borrow_mut()
                .insert(path.as_str().to_owned(), contents.to_vec());
            Ok(())
        }

        fn copy_recoverable(
            &self,
            _source: &skilltap_core::domain::AbsolutePath,
            _destination: &skilltap_core::domain::AbsolutePath,
        ) -> Result<(), skilltap_core::runtime::RuntimeError> {
            unimplemented!("publication helper does not copy")
        }

        fn create_relative_symlink(
            &self,
            _target: &skilltap_core::runtime::RelativeSymlinkTarget,
            _link: &skilltap_core::domain::AbsolutePath,
        ) -> Result<(), skilltap_core::runtime::RuntimeError> {
            unimplemented!("publication helper does not symlink")
        }

        fn remove(
            &self,
            path: &skilltap_core::domain::AbsolutePath,
        ) -> Result<(), skilltap_core::runtime::RuntimeError> {
            self.files.borrow_mut().remove(path.as_str());
            Ok(())
        }
    }

    #[test]
    fn daemon_pair_publication_restores_earlier_service_files_on_later_failure() {
        let service = skilltap_core::domain::AbsolutePath::new("/tmp/skilltap-service").unwrap();
        let timer = skilltap_core::domain::AbsolutePath::new("/tmp/skilltap-timer").unwrap();
        let filesystem = FailingPublicationFileSystem::new(timer.as_str());
        filesystem
            .files
            .borrow_mut()
            .insert(service.as_str().to_owned(), b"old service".to_vec());

        let changed = vec![
            (
                service.clone(),
                b"new service".to_vec(),
                Some(b"old service".to_vec()),
            ),
            (timer.clone(), b"new timer".to_vec(), None),
        ];
        let error = publish_daemon_files(&filesystem, &changed).unwrap_err();
        assert_eq!(error.0, timer);
        let files = filesystem.files.borrow();
        assert_eq!(files.get(service.as_str()), Some(&b"old service".to_vec()));
        assert!(!files.contains_key(timer.as_str()));
    }
}
