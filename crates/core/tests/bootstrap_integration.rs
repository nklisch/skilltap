use std::fs;

use sha2::{Digest, Sha256};
use skilltap_core::{
    bootstrap::{ArtifactArch, ArtifactKey, ReleaseArtifact, ReleaseVersion},
    domain::{AbsolutePath, SourceLocator},
    runtime::{BinaryInstaller, ReleaseManifest, SystemBinaryInstaller},
};
use skilltap_test_support::TempRoot;

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
