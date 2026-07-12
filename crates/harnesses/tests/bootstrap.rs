use skilltap_core::domain::{
    AbsolutePath, ConfiguredBinary, ExecutableFileIdentity, ExecutableIdentity, HarnessId,
    HarnessInstallation, HarnessReachability, NativeVersion,
};
use skilltap_harnesses::{
    HarnessBootstrapPolicy, HarnessKind, HarnessSetupResult, setup_detected_plugin,
};

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
