use super::{ServiceManagerAction, repository_composition_error, run_service_manager};
use crate::{ErrorDetail, NextAction, Outcome, OutputEntry, ResultClass, Warning};
use skilltap_core::{
    domain::AbsolutePath,
    runtime::{FileSystem, PlatformPaths, ProcessEnvironment, SystemFileSystem},
    storage::{ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository},
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
