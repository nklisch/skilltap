use std::{collections::BTreeMap, ffi::OsString, fs, path::Path};

use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, HarnessReachability, UnreachableReason},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, JsonLimits, NativeProcessRequest,
        ObservationRuntimeError, ProcessLimits, SystemExecutableResolver,
    },
};
use skilltap_harnesses::{
    DetectionError, HarnessKind, ProbeError, detect_installation, probe_profile, select_profile,
    unreachable_installation,
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

#[test]
fn known_profiles_grant_mutation_and_unknown_versions_remain_observe_only() {
    let known = skilltap_core::domain::NativeVersion::new("3.0.0").unwrap();
    let unknown = skilltap_core::domain::NativeVersion::new("99.0.0").unwrap();
    let known_profile = select_profile(HarnessKind::Codex, &known);
    assert!(known_profile.mutation_capabilities().is_some());
    assert_eq!(known_profile.profile_id().unwrap().as_str(), "codex-v3");

    let unknown_profile = select_profile(HarnessKind::Codex, &unknown);
    assert!(unknown_profile.mutation_capabilities().is_none());
    assert!(unknown_profile.profile_id().is_none());
}

#[test]
fn probe_narrowing_is_strict_and_never_widens_profiles() {
    let known = skilltap_core::domain::NativeVersion::new("3.0.0").unwrap();
    let profile = select_profile(HarnessKind::Codex, &known);
    let narrowed = skilltap_harnesses::narrow_profile(
        &profile,
        &serde_json::json!({
            "scope": "project",
            "capabilities": { "plugin.install": "unsupported" }
        }),
    )
    .unwrap();
    assert_eq!(
        narrowed
            .observation_capabilities()
            .for_scope_kind(skilltap_core::domain::CapabilityScope::Project)
            .support(&skilltap_core::domain::CapabilityId::new("plugin.install").unwrap()),
        Some(skilltap_core::domain::CapabilitySupport::Unsupported)
    );
    let drift = skilltap_harnesses::narrow_profile(
        &profile,
        &serde_json::json!({
            "scope": "project",
            "capabilities": { "future.capability": "supported" }
        }),
    )
    .unwrap_err();
    assert!(matches!(drift, ProbeError::Contract(_)));

    let (root, _fixture) = install(FakeNativeMode::ProbeNarrow, "codex");
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            ConfiguredBinary::absolute(
                AbsolutePath::new(root.join("codex").to_str().unwrap()).unwrap(),
            ),
            None,
        ))
        .unwrap();
    let request = NativeProcessRequest::new(
        executable,
        [OsString::from("probe")],
        BTreeMap::new(),
        None,
        ProcessLimits::new(1_000, 4_096, 4_096, 8_192).unwrap(),
    );
    let probed = probe_profile(&profile, &request, JsonLimits::new(4_096, 16).unwrap()).unwrap();
    assert_eq!(probed.profile_id().unwrap().as_str(), "codex-v3");
}

#[test]
fn detection_pipeline_is_repeatable_and_keeps_sibling_failures_isolated() {
    let (codex_root, _codex_fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (claude_root, _claude_fixture) = install(FakeNativeMode::MalformedJson, "claude");
    let (process_limits, json_limits) = limits();
    let codex_before = fs::read_dir(codex_root.path()).unwrap().count();
    let first = detect_installation(
        HarnessKind::Codex,
        codex_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    let second = detect_installation(
        HarnessKind::Codex,
        codex_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        fs::read_dir(codex_root.path()).unwrap().count(),
        codex_before
    );
    let claude = detect_installation(
        HarnessKind::Claude,
        claude_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    );
    assert!(matches!(claude, Err(DetectionError::Runtime(_))));
    assert_eq!(first.harness().as_str(), "codex");
}
