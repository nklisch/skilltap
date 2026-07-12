use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use sha2::{Digest, Sha256};
use skilltap_core::{
    bootstrap::{ArtifactArch, ArtifactKey, ReleaseArtifact, ReleaseVersion},
    domain::{AbsolutePath, SourceLocator},
    runtime::{
        ArtifactError, ArtifactFetcher, BinaryInstaller, ReleaseManifest, ReleaseResolver,
        SystemBinaryInstaller, SystemReleaseResolver,
    },
};
use skilltap_test_support::TempRoot;

#[derive(Clone)]
struct FixtureFetcher {
    payload: Vec<u8>,
    symlink: bool,
    workspace: Arc<Mutex<Option<PathBuf>>>,
    private_workspace: Arc<Mutex<bool>>,
}

impl ArtifactFetcher for FixtureFetcher {
    fn fetch(&self, _url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError> {
        let destination = std::path::Path::new(destination.as_str());
        let workspace = destination
            .parent()
            .expect("fixture destination has a private parent")
            .to_path_buf();
        *self.workspace.lock().unwrap() = Some(workspace.clone());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            *self.private_workspace.lock().unwrap() = fs::metadata(&workspace)
                .expect("private workspace metadata")
                .permissions()
                .mode()
                & 0o777
                == 0o700;
        }
        if self.symlink {
            let target = workspace.join("target");
            fs::write(&target, b"manifest target").unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, destination).unwrap();
            #[cfg(not(unix))]
            fs::write(destination, &self.payload).unwrap();
        } else {
            fs::write(destination, &self.payload).unwrap();
        }
        Ok(())
    }
}

fn artifact(version: ReleaseVersion, bytes: &[u8]) -> ReleaseArtifact {
    ReleaseArtifact::new(
        version,
        ArtifactKey {
            platform: skilltap_core::runtime::SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        },
        "skilltap-linux-x64",
        format!("{:x}", Sha256::digest(bytes)),
        SourceLocator::new(
            "https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap-linux-x64",
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn verified_install_is_atomic_and_rejects_checksum_mismatch() {
    let root = TempRoot::new("bootstrap-integration").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("bin").join("skilltap");
    let bytes = b"#!/bin/sh\nprintf 'skilltap 3.0.0\\n'\n";
    fs::write(&source, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let source = AbsolutePath::new(source.to_string_lossy().into_owned()).unwrap();
    let destination = AbsolutePath::new(destination.to_string_lossy().into_owned()).unwrap();
    let expected = artifact("3.0.0".parse().unwrap(), bytes);
    SystemBinaryInstaller
        .install_verified(&source, &destination, &expected)
        .unwrap();
    assert_eq!(fs::read(destination.as_str()).unwrap(), bytes);

    let prior = fs::read(destination.as_str()).unwrap();
    let wrong = artifact("3.0.1".parse().unwrap(), b"wrong");
    assert!(
        SystemBinaryInstaller
            .install_verified(&source, &destination, &wrong)
            .is_err()
    );
    assert_eq!(fs::read(destination.as_str()).unwrap(), prior);
}

#[cfg(unix)]
#[test]
fn verified_install_rejects_non_executable_payload_without_touching_destination() {
    let root = TempRoot::new("bootstrap-non-executable").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("bin").join("skilltap");
    let prior = b"#!/bin/sh\nprintf 'skilltap 2.0.0\\n'\n";
    let bytes = b"#!/bin/sh\nprintf 'skilltap 3.0.0\\n'\n";
    fs::write(&source, bytes).unwrap();
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, prior).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)).unwrap();
    let source = AbsolutePath::new(source.to_string_lossy().into_owned()).unwrap();
    let destination = AbsolutePath::new(destination.to_string_lossy().into_owned()).unwrap();
    let expected = artifact("3.0.0".parse().unwrap(), bytes);
    assert_eq!(
        SystemBinaryInstaller.install_verified(&source, &destination, &expected),
        Err(ArtifactError::InvalidArtifact)
    );
    assert_eq!(fs::read(destination.as_str()).unwrap(), prior);
}

#[test]
fn release_manifest_rejects_duplicate_selected_assets() {
    let key = ArtifactKey {
        platform: skilltap_core::runtime::SupportedPlatform::Linux,
        arch: ArtifactArch::X86_64,
    };
    let asset = |name: &str| {
        serde_json::json!({
            "platform":"linux", "arch":"x86_64", "name":name,
            "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "download_url":"https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap"
        })
    };
    let value =
        serde_json::json!({"version":"3.0.0", "assets":[asset("skilltap"), asset("skilltap-alt")]});
    assert!(ReleaseManifest::parse(&value, key).is_err());
}

#[test]
fn system_release_resolver_bounds_payloads_and_cleans_private_workspace() {
    let workspace = Arc::new(Mutex::new(None));
    let private_workspace = Arc::new(Mutex::new(false));
    let resolver = SystemReleaseResolver::with_fetcher(
        SourceLocator::new("https://github.com/nklisch/skilltap/releases/latest").unwrap(),
        ArtifactKey {
            platform: skilltap_core::runtime::SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        },
        FixtureFetcher {
            payload: vec![b'x'; 4 * 1024 * 1024 + 1],
            symlink: false,
            workspace: Arc::clone(&workspace),
            private_workspace: Arc::clone(&private_workspace),
        },
    );
    assert_eq!(resolver.latest(), Err(ArtifactError::DownloadFailed));
    assert!(*private_workspace.lock().unwrap());
    let workspace = workspace.lock().unwrap().clone().unwrap();
    assert!(
        !workspace.exists(),
        "private payload workspace must be removed"
    );
}

#[cfg(unix)]
#[test]
fn system_release_resolver_rejects_symlink_payloads_and_cleans_workspace() {
    let workspace = Arc::new(Mutex::new(None));
    let private_workspace = Arc::new(Mutex::new(false));
    let resolver = SystemReleaseResolver::with_fetcher(
        SourceLocator::new("https://github.com/nklisch/skilltap/releases/latest").unwrap(),
        ArtifactKey {
            platform: skilltap_core::runtime::SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        },
        FixtureFetcher {
            payload: b"unused".to_vec(),
            symlink: true,
            workspace: Arc::clone(&workspace),
            private_workspace: Arc::clone(&private_workspace),
        },
    );
    assert_eq!(resolver.latest(), Err(ArtifactError::DownloadFailed));
    assert!(*private_workspace.lock().unwrap());
    let workspace = workspace.lock().unwrap().clone().unwrap();
    assert!(
        !workspace.exists(),
        "private payload workspace must be removed"
    );
}

#[cfg(unix)]
#[test]
fn binary_install_rejects_symlink_destination_without_touching_target() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let root = TempRoot::new("bootstrap-destination-symlink").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("bin").join("skilltap");
    let target = root.path().join("unrelated");
    let bytes = b"#!/bin/sh\nprintf 'skilltap 3.0.0\\n'\n";
    fs::write(&source, bytes).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&target, b"unrelated").unwrap();
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    symlink(&target, &destination).unwrap();
    let source = AbsolutePath::new(source.to_string_lossy().into_owned()).unwrap();
    let destination = AbsolutePath::new(destination.to_string_lossy().into_owned()).unwrap();
    let expected = artifact("3.0.0".parse().unwrap(), bytes);
    assert_eq!(
        SystemBinaryInstaller.install_verified(&source, &destination, &expected),
        Err(ArtifactError::DestinationChanged)
    );
    assert_eq!(fs::read(&target).unwrap(), b"unrelated");
    assert_eq!(fs::read_link(destination.as_str()).unwrap(), target);
}
