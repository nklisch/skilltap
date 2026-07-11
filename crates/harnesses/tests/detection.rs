use std::path::Path;

use skilltap_core::{
    domain::{HarnessReachability, UnreachableReason},
    runtime::{JsonLimits, ObservationRuntimeError, ProcessLimits},
};
use skilltap_harnesses::{
    DetectionError, HarnessKind, detect_installation, unreachable_installation,
};
use skilltap_test_support::{FakeNativeMode, FakeNativeProcess, TempRoot};

fn limits() -> (ProcessLimits, JsonLimits) {
    (
        ProcessLimits::new(1_000, 4_096, 4_096, 8_192).unwrap(),
        JsonLimits::new(4_096, 16).unwrap(),
    )
}

fn install(mode: FakeNativeMode, name: &str) -> (TempRoot, FakeNativeProcess) {
    let fixture = FakeNativeProcess::new(mode).unwrap();
    let root = TempRoot::new("skilltap-detection").unwrap();
    fixture.install_alias(root.path(), name).unwrap();
    (root, fixture)
}

#[test]
fn known_and_unknown_versions_are_reachable_without_profile_guessing() {
    let (known_root, _known_fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (process_limits, json_limits) = limits();
    let known = detect_installation(
        HarnessKind::Codex,
        known_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    match known.reachability() {
        HarnessReachability::Reachable { native_version, .. } => {
            assert_eq!(native_version.as_str(), "3.0.0")
        }
        HarnessReachability::Unreachable { .. } => panic!("known fixture is reachable"),
    }

    let (unknown_root, _unknown_fixture) = install(FakeNativeMode::VersionUnknown, "codex");
    let unknown = detect_installation(
        HarnessKind::Codex,
        unknown_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    match unknown.reachability() {
        HarnessReachability::Reachable { native_version, .. } => {
            assert_eq!(native_version.as_str(), "99.0.0")
        }
        HarnessReachability::Unreachable { .. } => panic!("unknown fixture is reachable"),
    }
}

#[test]
fn malformed_native_output_is_typed_and_secret_safe() {
    let (root, _fixture) = install(FakeNativeMode::DuplicateJson, "claude");
    let (process_limits, json_limits) = limits();
    let error = detect_installation(
        HarnessKind::Claude,
        root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap_err();
    assert_eq!(
        error,
        DetectionError::Runtime(ObservationRuntimeError::JsonDuplicateKey)
    );
    assert!(!format!("{error:?}").contains("3.0.1"));
}

#[test]
fn missing_binary_and_explicit_unreachable_results_do_not_probe() {
    let (process_limits, json_limits) = limits();
    let missing = TempRoot::new("skilltap-detection-missing").unwrap();
    let error = detect_installation(
        HarnessKind::Codex,
        missing.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap_err();
    assert_eq!(
        error,
        DetectionError::Runtime(ObservationRuntimeError::ExecutableNotFound)
    );

    let unavailable = unreachable_installation(HarnessKind::Claude, UnreachableReason::NotFound);
    assert!(matches!(
        unavailable.reachability(),
        HarnessReachability::Unreachable {
            reason: UnreachableReason::NotFound
        }
    ));
    assert!(!Path::new("/tmp/skilltap").exists());
}
