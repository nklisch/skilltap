use std::{cell::RefCell, collections::BTreeMap, io};

use serde_json::Value;

use super::*;

#[test]
fn plan_composes_as_an_attention_report_and_other_unavailable_commands_remain_explicit() {
    let plain = run_from(["skilltap", "plan"]);
    assert_eq!(plain.exit_code, 2);
    assert_eq!(plain.channel, OutputChannel::Stdout);
    assert!(plain.document.contains("Result: attention required"));

    let json = run_from(["skilltap", "plugin", "install", "format@team", "--json"]);
    assert_eq!(json.exit_code, 2);
    assert_eq!(json.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&json.document).unwrap();
    assert_eq!(value["command"], "plugin install");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["errors"][0]["code"], "no_enabled_harnesses");
}

#[test]
fn first_use_plain_status_is_an_attention_report_on_stdout() {
    let outcome =
        Outcome::new("status", ResultClass::AttentionRequired).with_warning(crate::Warning::new(
            "native_observation_unavailable",
            "Native harness observation is not available in this build.",
        ));

    let execution = render(outcome, false, OutputChannel::Stdout);

    assert_eq!(execution.exit_code, 2);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    assert!(execution.document.contains("Result: attention required"));
}

#[test]
fn parse_failures_are_normalized_as_one_json_document_when_requested() {
    let execution = run_from(["skilltap", "status", "--target", "pi", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["result"], "invalid");
    assert_eq!(value["errors"][0]["code"], "invalid_arguments");
    assert!(!execution.document.contains("pi"));
    assert_eq!(execution.document.lines().count(), 1);
}

#[test]
fn missing_command_is_a_stable_input_error() {
    let execution = run_from(["skilltap"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stderr);
    assert!(execution.document.contains("Code: missing_command"));
    assert!(execution.document.contains("Usage: skilltap <COMMAND>"));
}

#[test]
fn json_requested_without_a_command_remains_one_normalized_document() {
    let execution = run_from(["skilltap", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    assert_eq!(execution.document.lines().count(), 1);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["command"], "skilltap");
    assert_eq!(value["errors"][0]["code"], "invalid_arguments");
    assert!(!execution.document.contains("--json"));
    assert!(!execution.document.contains("Usage:"));
}

#[test]
fn help_and_version_complete_on_stdout() {
    for arguments in [
        &["skilltap", "--help"][..],
        &["skilltap", "--version"][..],
        &["skilltap", "status", "--help"][..],
    ] {
        let execution = run_from(arguments.iter().copied());
        assert_eq!(execution.exit_code, 0);
        assert_eq!(execution.channel, OutputChannel::Stdout);
        assert!(!execution.document.is_empty());
    }
}

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
    ) -> Result<skilltap_core::runtime::FileMetadata, skilltap_core::runtime::RuntimeError> {
        unimplemented!("publication helper does not inspect")
    }

    fn canonicalize(
        &self,
        _path: &skilltap_core::domain::AbsolutePath,
    ) -> Result<skilltap_core::domain::AbsolutePath, skilltap_core::runtime::RuntimeError> {
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
