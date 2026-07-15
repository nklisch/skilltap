#![cfg(unix)]

use std::{collections::BTreeMap, ffi::OsString, fs, os::unix::fs::PermissionsExt};

use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, NativeId, Scope},
    runtime::{JsonLimits, ProcessLimits},
};
use skilltap_harnesses::{
    ClaudeAdapter, FactoryAdapter, NativeLifecycleAction, NativeLifecycleDispatch,
    NativeLifecycleRequest, NativeObservationFailure, NativeResourceObservation, QwenAdapter,
    QwenExtensionRecord, decode_factory_plugin_list, decode_qwen_extensions,
    observe_native_resource,
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
fn factory_native_lifecycle_and_human_postcondition_are_scope_exact() {
    let adapter = FactoryAdapter::static_ref();
    let project = Scope::Project(AbsolutePath::new("/tmp/factory-project").unwrap());
    let request = NativeLifecycleRequest {
        action: NativeLifecycleAction::PluginInstall,
        scope: project.clone(),
        name: NativeId::new("demo@market").unwrap(),
        source: None,
    };
    let dispatch = NativeLifecycleDispatch::new(
        adapter.identity().id,
        adapter.native_lifecycle().unwrap(),
        request,
    );
    assert_eq!(
        skilltap_harnesses::native_arguments(&dispatch).unwrap(),
        ["plugin", "install", "demo@market", "--scope", "project"].map(OsString::from)
    );
    assert_eq!(
        adapter
            .native_lifecycle()
            .unwrap()
            .observation_arguments(dispatch.request())
            .unwrap(),
        ["plugin", "list", "--scope", "project"].map(OsString::from)
    );
    assert_eq!(
        decode_factory_plugin_list(
            b"Installed plugins:\nActive:\n  demo@market  [project]  e8801fa\n",
            "demo@market",
            "project",
            JsonLimits::new(4096, 16).unwrap(),
        ),
        NativeResourceObservation::Present {
            scope: Some(skilltap_core::domain::CapabilityScope::Project),
            revision: Some(skilltap_core::domain::ResolvedRevision::Native(
                NativeId::new("e8801fa").unwrap(),
            )),
        }
    );
}

#[test]
fn qwen_native_vectors_and_human_enablement_observation_preserve_workspace_scope() {
    let adapter = QwenAdapter::static_ref();
    let project = Scope::Project(AbsolutePath::new("/tmp/qwen-project").unwrap());
    let request = NativeLifecycleRequest {
        action: NativeLifecycleAction::PluginInstall,
        scope: project.clone(),
        name: NativeId::new("fixture").unwrap(),
        source: Some(skilltap_core::domain::SourceLocator::new("/tmp/qwen-source").unwrap()),
    };
    let dispatch = NativeLifecycleDispatch::new(
        adapter.identity().id,
        adapter.native_lifecycle().unwrap(),
        request.clone(),
    );
    assert_eq!(
        skilltap_harnesses::native_arguments(&dispatch),
        Ok([
            "extensions",
            "install",
            "/tmp/qwen-source:fixture",
            "--scope",
            "workspace"
        ]
        .map(OsString::from)
        .to_vec())
    );
    assert_eq!(
        adapter
            .native_lifecycle()
            .unwrap()
            .observation_arguments(&request)
            .unwrap(),
        ["extensions", "list", "--scope", "workspace"].map(OsString::from)
    );
    let records = decode_qwen_extensions(b"Installed extensions:\n  fixture\n    Version: 1.0.0\n    Enabled (User): false\n    Enabled (Workspace): true\n", JsonLimits::new(4096, 16).unwrap()).unwrap();
    assert_eq!(
        records,
        vec![QwenExtensionRecord {
            name: "fixture".to_owned(),
            version: Some(NativeId::new("1.0.0").unwrap()),
            path: None,
            source: None,
            source_type: None,
            enabled_user: false,
            enabled_workspace: true,
            components: Default::default()
        }]
    );
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
            revision: None,
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
