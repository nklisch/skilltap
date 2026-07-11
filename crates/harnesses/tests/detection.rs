use std::{collections::BTreeMap, ffi::OsString, fs, path::Path};

use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, HarnessReachability, Scope, UnreachableReason},
    runtime::{
        Environment, EnvironmentVariable, ExecutableResolutionRequest, ExecutableResolver,
        ExternalTreeLimits, JsonLimits, NativeProcessRequest, ObservationRuntimeError,
        PlatformPaths, ProcessLimits, SupportedPlatform, SystemExecutableResolver,
    },
};
use skilltap_harnesses::{
    CodexConfigError, DetectionError, HarnessKind, ProbeError, detect_configured_installation,
    detect_installation, observe_codex_config, probe_profile, select_profile,
    unreachable_installation,
};
use skilltap_test_support::{FakeNativeMode, FakeNativeProcess, TempRoot};

#[derive(Default)]
struct TestEnvironment(BTreeMap<&'static str, OsString>);

impl TestEnvironment {
    fn with(mut self, variable: EnvironmentVariable, value: &str) -> Self {
        self.0.insert(variable.as_str(), OsString::from(value));
        self
    }
}

impl Environment for TestEnvironment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
        self.0.get(variable.as_str()).cloned()
    }
}

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
fn configured_absolute_binary_is_used_for_detection_without_path_lookup() {
    let (root, _fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (process_limits, json_limits) = limits();
    let configured = ConfiguredBinary::absolute(
        AbsolutePath::new(root.join("codex").to_str().unwrap()).unwrap(),
    );
    let installation = detect_configured_installation(
        HarnessKind::Codex,
        configured,
        None,
        process_limits,
        json_limits,
    )
    .unwrap();
    assert!(matches!(
        installation.reachability(),
        HarnessReachability::Reachable { .. }
    ));
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

#[test]
fn codex_paths_preserve_global_and_project_instruction_inputs() {
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, "/home/user")
        .with(EnvironmentVariable::XdgConfigHome, "/config/user")
        .with(EnvironmentVariable::CodexHome, "/opt/codex");
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let project = Scope::Project(AbsolutePath::new("/workspace/project").unwrap());
    let inputs = skilltap_harnesses::codex_observation_paths(&platform, &project).unwrap();
    assert_eq!(inputs.codex_home.as_str(), "/opt/codex");
    assert_eq!(inputs.global_agents.as_str(), "/home/user/AGENTS.md");
    assert_eq!(
        inputs.project_agents.as_ref().unwrap().as_str(),
        "/workspace/project/AGENTS.md"
    );
    assert_eq!(
        inputs.project_override.as_ref().unwrap().as_str(),
        "/workspace/project/AGENTS.override.md"
    );
}

#[test]
fn codex_config_observation_is_bounded_unknown_field_tolerant_and_redacted() {
    let observed = observe_codex_config(
        br#"marketplaces = ["secret-marketplace"]
plugins = ["one"]
trust = true
"#,
    )
    .unwrap();
    assert_eq!(observed.marketplace_count, 1);
    assert_eq!(observed.plugin_count, 1);
    assert!(observed.trust_policy_present);
    assert!(!format!("{observed:?}").contains("example.invalid"));
    assert_eq!(
        observe_codex_config(b"not = [valid"),
        Err(CodexConfigError::Malformed)
    );
}

#[test]
fn codex_resource_observation_reads_complete_skill_trees_without_writing() {
    let root = TempRoot::new("skilltap-codex-resources").unwrap();
    let home = root.join("home");
    let codex_home = root.join("codex");
    let skill = codex_home.join("skills/example");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), b"name: example\n").unwrap();
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(EnvironmentVariable::CodexHome, codex_home.to_str().unwrap());
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let paths = skilltap_harnesses::codex_observation_paths(&platform, &Scope::Global).unwrap();
    let snapshot = skilltap_harnesses::observe_codex_resources(
        &paths,
        ExternalTreeLimits::new(8, 32, 4096, 8192, 1024).unwrap(),
    )
    .unwrap();
    assert!(
        snapshot
            .entries()
            .iter()
            .any(|entry| entry.path().as_str() == "skills/example/SKILL.md")
    );
    assert_eq!(
        std::fs::read(skill.join("SKILL.md")).unwrap(),
        b"name: example\n"
    );
}

#[test]
fn claude_paths_keep_global_and_personal_project_inputs_separate() {
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, "/home/user")
        .with(EnvironmentVariable::XdgConfigHome, "/config/user");
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let project = Scope::Project(AbsolutePath::new("/workspace/personal").unwrap());
    let inputs = skilltap_harnesses::claude_observation_paths(&platform, &project).unwrap();
    assert_eq!(inputs.claude_home.as_str(), "/home/user/.claude");
    assert_eq!(
        inputs.global_settings.as_str(),
        "/home/user/.claude/settings.json"
    );
    assert_eq!(inputs.global_plugins.as_str(), "/home/user/.claude/plugins");
    assert_eq!(inputs.global_skills.as_str(), "/home/user/.claude/skills");
    assert_eq!(
        inputs.project_settings.as_ref().unwrap().as_str(),
        "/workspace/personal/.claude/settings.json"
    );
}

#[test]
fn claude_settings_preserve_qualified_identity_counts_and_shared_project_state() {
    let observed = skilltap_harnesses::observe_claude_settings(
        br#"{
  "enabledPlugins": ["lint@anthropic", "local-tool"],
  "trust": {"consent": true},
  "sharedProject": true,
  "unknown": "secret"
}"#,
        JsonLimits::new(4096, 16).unwrap(),
    )
    .unwrap();
    assert_eq!(observed.enabled_plugin_count, 2);
    assert_eq!(observed.qualified_plugin_count, 1);
    assert!(observed.trust_policy_present);
    assert!(observed.shared_project);
    assert!(!format!("{observed:?}").contains("anthropic"));
}

#[test]
fn claude_resource_observation_keeps_complete_skills_and_cache_evidence_read_only() {
    let root = TempRoot::new("skilltap-claude-resources").unwrap();
    let home = root.join("home");
    let claude_home = home.join(".claude");
    let skill = claude_home.join("skills/example");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), b"name: example\n").unwrap();
    let environment =
        TestEnvironment::default().with(EnvironmentVariable::Home, home.to_str().unwrap());
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let paths = skilltap_harnesses::claude_observation_paths(&platform, &Scope::Global).unwrap();
    let snapshot = skilltap_harnesses::observe_claude_resources(
        &paths,
        ExternalTreeLimits::new(8, 32, 4096, 8192, 1024).unwrap(),
    )
    .unwrap();
    assert!(
        snapshot
            .entries()
            .iter()
            .any(|entry| entry.path().as_str() == "skills/example/SKILL.md")
    );
    assert_eq!(
        std::fs::read(skill.join("SKILL.md")).unwrap(),
        b"name: example\n"
    );
}
