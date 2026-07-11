#![cfg(unix)]

use std::{cell::Cell, collections::BTreeMap, fs, io, path::PathBuf};

use skilltap_test_support::TempRoot;

use super::*;
use crate::{
    domain::FingerprintAlgorithm,
    runtime::{
        DirectoryContentState, DirectoryPathState, DirectoryPublishOutcome, DirectorySyncState,
        SystemFileSystem,
    },
    storage::SchemaError,
};

fn setup() -> (TempRoot, FileManagedArtifactRepository<'static>) {
    let temporary = TempRoot::new("skilltap-managed-artifacts-test").unwrap();
    let root = AbsolutePath::new(temporary.path().to_str().unwrap()).unwrap();
    let filesystem = Box::leak(Box::new(SystemFileSystem));
    let repository = FileManagedArtifactRepository::new(filesystem, root).unwrap();
    (temporary, repository)
}

fn owner(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn fingerprint(byte: char) -> Fingerprint {
    Fingerprint::new(FingerprintAlgorithm::Sha256, byte.to_string().repeat(64)).unwrap()
}

fn skill_tree() -> ArtifactTree {
    ArtifactTree::new([
        ("SKILL.md", b"not semantically validated".to_vec()),
        ("scripts/run.sh", b"#!/bin/sh\nexit 0\n".to_vec()),
        ("references/guide.md", vec![0, 1, 2, 255]),
    ])
    .unwrap()
}

fn absolute(
    repository: &FileManagedArtifactRepository<'_>,
    path: &RelativeArtifactPath,
) -> PathBuf {
    PathBuf::from(repository.managed_root().as_str()).join(path.as_str())
}

include!("tests/tree_contract.rs");
include!("tests/lifecycle_security.rs");
include!("tests/failure_mapping.rs");
