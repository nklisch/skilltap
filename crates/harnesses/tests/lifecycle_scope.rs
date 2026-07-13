#![cfg(unix)]

use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt};

use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, NativeId, Scope},
    runtime::{JsonLimits, ProcessLimits},
};
use skilltap_harnesses::{
    ClaudeAdapter, NativeLifecycleAction, NativeLifecycleDispatch, NativeLifecycleRequest,
    NativeObservationFailure, NativeResourceObservation, observe_native_resource,
};
use skilltap_test_support::TempRoot;

fn limits() -> (ProcessLimits, JsonLimits) {
    (
        ProcessLimits::new(1_000, 8_192, 8_192, 16_384).unwrap(),
        JsonLimits::new(8_192, 16).unwrap(),
    )
}

fn fake_claude(root: &TempRoot, payload: &str) -> ConfiguredBinary {
    let executable = root.join("claude");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nif [ \"$1 $2 $3\" = \"plugin list --json\" ]; then printf '%s' '{payload}'; exit 0; fi\nexit 9\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    ConfiguredBinary::absolute(AbsolutePath::new(executable.to_string_lossy()).unwrap())
}

fn request(scope: Scope) -> NativeLifecycleDispatch {
    let adapter = ClaudeAdapter::static_ref();
    NativeLifecycleDispatch::new(
        adapter.identity().id,
        adapter.native_lifecycle().unwrap(),
        NativeLifecycleRequest {
            action: NativeLifecycleAction::PluginInstall,
            scope,
            name: NativeId::new("formatter@team").unwrap(),
            source: None,
        },
    )
}

#[test]
fn same_name_user_plugin_does_not_satisfy_missing_local_plugin() {
    let root = TempRoot::new("skilltap-claude-scope-presence").unwrap();
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    let configured = fake_claude(
        &root,
        r#"{"plugins":[{"id":"formatter@team","scope":"user"}]}"#,
    );
    let (process_limits, json_limits) = limits();

    assert_eq!(
        observe_native_resource(
            configured.clone(),
            None,
            &BTreeMap::new(),
            &request(Scope::Global),
            process_limits,
            json_limits,
        )
        .unwrap(),
        NativeResourceObservation::Present {
            scope: Some(skilltap_core::domain::CapabilityScope::Global),
        }
    );
    assert_eq!(
        observe_native_resource(
            configured,
            None,
            &BTreeMap::new(),
            &request(Scope::Project(
                AbsolutePath::new(project.to_string_lossy()).unwrap(),
            )),
            process_limits,
            json_limits,
        )
        .unwrap(),
        NativeResourceObservation::Missing
    );
}

#[test]
fn scope_less_or_duplicate_native_entries_are_not_presence_evidence() {
    for (label, payload) in [
        ("missing", r#"{"plugins":[{"id":"formatter@team"}]}"#),
        (
            "duplicate",
            r#"{"plugins":[{"id":"formatter@team","scope":"local"},{"id":"formatter@team","scope":"local"}]}"#,
        ),
    ] {
        let root = TempRoot::new(&format!("skilltap-claude-scope-{label}")).unwrap();
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let configured = fake_claude(&root, payload);
        let (process_limits, json_limits) = limits();
        assert_eq!(
            observe_native_resource(
                configured,
                None,
                &BTreeMap::new(),
                &request(Scope::Project(
                    AbsolutePath::new(project.to_string_lossy()).unwrap(),
                )),
                process_limits,
                json_limits,
            )
            .unwrap(),
            NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
            "{label} scope evidence must fail closed"
        );
    }
}
