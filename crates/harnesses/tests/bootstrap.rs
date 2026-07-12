use skilltap_core::domain::{
    AbsolutePath, ConfiguredBinary, ExecutableFileIdentity, ExecutableIdentity, HarnessId,
    HarnessInstallation, HarnessReachability, NativeVersion,
};
use skilltap_harnesses::{
    HarnessBootstrapPolicy, HarnessKind, HarnessSetupResult, setup_detected_plugin,
};
use skilltap_test_support::TempRoot;
use std::fs;

#[cfg(unix)]
fn write_fake_claude(
    root: &TempRoot,
    marketplace_payload: &str,
    plugin_payload: &str,
) -> (skilltap_core::domain::ConfiguredBinary, std::path::PathBuf) {
    write_fake_claude_version(
        root,
        r#"{"version":"3.0.0"}"#,
        marketplace_payload,
        plugin_payload,
    )
}

#[cfg(unix)]
fn write_fake_claude_version(
    root: &TempRoot,
    version_payload: &str,
    marketplace_payload: &str,
    plugin_payload: &str,
) -> (skilltap_core::domain::ConfiguredBinary, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let binary = root.path().join("claude");
    let log = root.path().join("calls.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\nif [ \"$1\" = \"--version\" ]; then printf '%s' '{version}'; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"marketplace\" ]; then printf '%s' '{marketplace_payload}'; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"list\" ]; then printf '%s' '{plugin_payload}'; exit 0; fi\nexit 0\n",
        log = log.display(),
        version = version_payload,
        marketplace_payload = marketplace_payload,
        plugin_payload = plugin_payload,
    );
    fs::write(&binary, script).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    (
        skilltap_core::domain::ConfiguredBinary::absolute(
            skilltap_core::domain::AbsolutePath::new(binary.to_string_lossy().into_owned())
                .unwrap(),
        ),
        log,
    )
}

fn installation(target: HarnessKind) -> HarnessInstallation {
    HarnessInstallation::new(
        HarnessId::new(target.id()).unwrap(),
        ConfiguredBinary::absolute(AbsolutePath::new("/tmp/skilltap-fake-harness").unwrap()),
        HarnessReachability::Reachable {
            executable: ExecutableIdentity::new(
                AbsolutePath::new("/tmp/skilltap-fake-harness").unwrap(),
                ExecutableFileIdentity::new(1, 1),
            ),
            native_version: NativeVersion::new("3.0.0").unwrap(),
        },
    )
}

#[test]
fn codex_plugin_setup_preserves_the_interactive_contract_gap() {
    let policy = HarnessBootstrapPolicy::skilltap(
        ConfiguredBinary::absolute(AbsolutePath::new("/tmp/skilltap-fake-harness").unwrap()),
        None,
    );
    let result = setup_detected_plugin(
        HarnessKind::Codex,
        &installation(HarnessKind::Codex),
        &policy,
    );
    assert!(matches!(result, HarnessSetupResult::Unsupported { .. }));
}

#[cfg(unix)]
#[test]
fn claude_setup_uses_marketplace_then_qualified_plugin_vectors() {
    let root = TempRoot::new("harness-bootstrap").unwrap();
    let (configured, log) = write_fake_claude(&root, r#"{"marketplaces":[]}"#, r#"{"plugins":[]}"#);
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    let result = skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy);
    assert!(matches!(result, HarnessSetupResult::Installed { .. }));
    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.contains("plugin marketplace list --json --scope user"));
    assert!(
        calls.contains(
            "plugin marketplace add https://github.com/nklisch/skilltap/tree/main/plugin"
        )
    );
    assert!(calls.contains("plugin install skilltap@skilltap --scope user"));
}

#[cfg(unix)]
#[test]
fn claude_bootstrap_presence_matrix_is_read_first_and_target_isolated() {
    let root = TempRoot::new("harness-bootstrap-presence").unwrap();
    let (configured, log) = write_fake_claude(
        &root,
        r#"{"marketplaces":["skilltap"]}"#,
        r#"{"plugins":["skilltap@skilltap"]}"#,
    );
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    let result = skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy);
    assert!(matches!(result, HarnessSetupResult::AlreadyPresent { .. }));
    let calls = fs::read_to_string(&log).unwrap();
    assert!(calls.contains("--version --json"));
    assert!(calls.contains("plugin marketplace list --json --scope user"));
    assert!(calls.contains("plugin list --json --scope user"));
    assert!(!calls.contains("marketplace add"));
    assert!(!calls.contains("plugin install"));
    assert!(!calls.contains("codex"));
    let mut entries = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    entries.sort();
    assert_eq!(
        entries,
        [
            std::ffi::OsString::from("calls.log"),
            std::ffi::OsString::from("claude")
        ]
    );
}

#[cfg(unix)]
#[test]
fn claude_bootstrap_missing_resources_uses_exact_native_vectors() {
    let root = TempRoot::new("harness-bootstrap-missing").unwrap();
    let (configured, log) = write_fake_claude(&root, r#"{"marketplaces":[]}"#, r#"{"plugins":[]}"#);
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    let result = skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy);
    assert!(matches!(result, HarnessSetupResult::Installed { .. }));
    let calls = fs::read_to_string(log).unwrap();
    assert!(
        calls.contains(
            "plugin marketplace add https://github.com/nklisch/skilltap/tree/main/plugin"
        )
    );
    assert!(calls.contains("plugin install skilltap@skilltap --scope user"));
}

#[cfg(unix)]
#[test]
fn malformed_or_unknown_native_lists_block_mutation_and_codex_stays_unsupported() {
    let root = TempRoot::new("harness-bootstrap-malformed").unwrap();
    let (configured, log) = write_fake_claude(&root, "{malformed", r#"{"plugins":[]}"#);
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    let result = skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy);
    assert!(matches!(
        result,
        HarnessSetupResult::Failed {
            reason: skilltap_harnesses::SetupReason::UnknownNativeState,
            ..
        }
    ));
    let calls = fs::read_to_string(log).unwrap();
    assert!(!calls.contains("marketplace add"));
    assert!(!calls.contains("plugin install"));

    let codex = setup_detected_plugin(
        HarnessKind::Codex,
        &installation(HarnessKind::Codex),
        &HarnessBootstrapPolicy::skilltap(
            ConfiguredBinary::absolute(AbsolutePath::new("/tmp/fake-codex").unwrap()),
            None,
        ),
    );
    assert!(matches!(codex, HarnessSetupResult::Unsupported { .. }));
}

#[cfg(unix)]
#[test]
fn malformed_version_is_invalid_and_unknown_version_narrows_capabilities() {
    let malformed_root = TempRoot::new("harness-bootstrap-version-malformed").unwrap();
    let (configured, _log) = write_fake_claude_version(
        &malformed_root,
        "{malformed",
        r#"{"marketplaces":[]}"#,
        r#"{"plugins":[]}"#,
    );
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    assert!(matches!(
        skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy),
        HarnessSetupResult::Unavailable {
            reason: skilltap_harnesses::SetupReason::InvalidVersion,
            ..
        }
    ));

    let unknown_root = TempRoot::new("harness-bootstrap-version-unknown").unwrap();
    let (configured, log) = write_fake_claude_version(
        &unknown_root,
        r#"{"version":"99.0.0"}"#,
        r#"{"marketplaces":[]}"#,
        r#"{"plugins":[]}"#,
    );
    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    assert!(matches!(
        skilltap_harnesses::setup_first_party_plugin(HarnessKind::Claude, &policy),
        HarnessSetupResult::Unsupported { .. }
    ));
    let calls = fs::read_to_string(log).unwrap();
    assert!(calls.contains("--version --json"));
    assert!(!calls.contains("marketplace list"));
    assert!(!calls.contains("marketplace add"));
    assert!(!calls.contains("plugin install"));
}

#[cfg(unix)]
#[test]
fn executable_replacement_after_detection_blocks_native_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempRoot::new("harness-bootstrap-replacement").unwrap();
    let (configured, log) = write_fake_claude(&root, r#"{"marketplaces":[]}"#, r#"{"plugins":[]}"#);
    let installation = skilltap_harnesses::detect_configured_installation(
        HarnessKind::Claude,
        configured.clone(),
        None,
        skilltap_core::runtime::ProcessLimits::new(30_000, 64 * 1024, 64 * 1024, 128 * 1024)
            .unwrap(),
        skilltap_core::runtime::JsonLimits::new(128 * 1024, 32).unwrap(),
    )
    .unwrap();
    let binary = root.path().join("claude");
    let replacement = root.path().join("replacement");
    let original = root.path().join("claude.original");
    fs::copy(&binary, &replacement).unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o700)).unwrap();
    // Keep the original inode alive while creating the replacement.  Removing
    // it first lets filesystems immediately recycle the inode, turning this
    // fixture into a false negative for identity revalidation.
    fs::rename(&binary, &original).unwrap();
    fs::rename(replacement, &binary).unwrap();
    fs::remove_file(original).unwrap();

    let policy = HarnessBootstrapPolicy::skilltap(configured, None);
    let result = setup_detected_plugin(HarnessKind::Claude, &installation, &policy);
    assert!(matches!(result, HarnessSetupResult::Failed { .. }));
    let calls = fs::read_to_string(log).unwrap();
    assert!(!calls.contains("marketplace add"));
    assert!(!calls.contains("plugin install"));
}
