use skilltap_core::domain::{
    AbsolutePath, ConfiguredBinary, ExecutableFileIdentity, ExecutableIdentity, HarnessId,
    HarnessInstallation, HarnessReachability, NativeVersion,
};
use skilltap_harnesses::{
    HarnessBootstrapPolicy, HarnessKind, HarnessSetupResult, setup_detected_plugin,
};
use skilltap_test_support::TempRoot;
use std::fs;

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
    use std::os::unix::fs::PermissionsExt;

    let root = TempRoot::new("harness-bootstrap").unwrap();
    let binary = root.path().join("claude");
    let log = root.path().join("calls.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$1\" = \"--version\" ]; then printf '{{\\\"version\\\":\\\"3.0.0\\\"}}'; else printf '{{\\\"marketplaces\\\":[],\\\"plugins\\\":[]}}'; fi\n",
        log.display()
    );
    fs::write(&binary, script).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let configured = ConfiguredBinary::absolute(
        AbsolutePath::new(binary.to_string_lossy().into_owned()).unwrap(),
    );
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
