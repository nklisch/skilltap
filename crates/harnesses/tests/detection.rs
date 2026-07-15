use std::{collections::BTreeMap, ffi::OsString, fs, path::Path};

use skilltap_core::{
    domain::{
        AbsolutePath, ConfiguredBinary, HarnessId, HarnessInstallation, HarnessReachability, Scope,
        UnreachableReason,
    },
    runtime::{
        Environment, EnvironmentVariable, ExecutableResolutionRequest, ExecutableResolver,
        ExternalTreeLimits, JsonLimits, NativeProcessRequest, ObservationRuntimeError,
        PlatformPaths, ProcessLimits, SupportedPlatform, SystemExecutableResolver,
    },
};
use skilltap_harnesses::{
    ClaudeAdapter, CodexAdapter, CodexConfigError, DetectionError, HarnessAdapter, PiAdapter,
    ProbeError, TargetRegistry,
    detect_configured_installation as detect_configured_installation_with_environment,
    detect_installation as detect_installation_with_environment, observe_codex_canonical_resources,
    observe_codex_config, probe_profile, unreachable_installation,
};
use skilltap_test_support::{
    BLOCKED_CANDIDATES, CandidateAdmissionReport, CandidateDisposition, FakeHarnessProfile,
    FakeNativeMode, FakeNativeProcess, TempRoot, blocked_candidate_admission_reports,
};

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

fn detect_installation(
    harness: &'static dyn HarnessAdapter,
    search_path: OsString,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError> {
    detect_installation_with_environment(
        harness,
        search_path,
        &BTreeMap::new(),
        process_limits,
        json_limits,
    )
}

fn detect_configured_installation(
    harness: &'static dyn HarnessAdapter,
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError> {
    detect_configured_installation_with_environment(
        harness,
        configured,
        search_path,
        &BTreeMap::new(),
        process_limits,
        json_limits,
    )
}

fn install(mode: FakeNativeMode, name: &str) -> (TempRoot, FakeNativeProcess) {
    let fixture = FakeNativeProcess::new(mode).unwrap();
    let root = TempRoot::new("skilltap-detection").unwrap();
    fixture.install_alias(root.path(), name).unwrap();
    (root, fixture)
}

fn install_profile(profile: &FakeHarnessProfile, name: &str) -> (TempRoot, FakeNativeProcess) {
    let root = TempRoot::new("skilltap-profile-detection").unwrap();
    let fixture = profile
        .build(root.path(), FakeNativeMode::VersionKnown)
        .unwrap();
    fixture.install_alias(root.path(), name).unwrap();
    (root, fixture)
}

#[test]
fn detection_forwards_only_the_explicit_native_environment() {
    let fixture = FakeHarnessProfile::codex()
        .builder(FakeNativeMode::VersionKnown)
        .capture_environment([
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "CODEX_HOME",
            "CLAUDE_CONFIG_DIR",
            "PATH",
            "UNLISTED",
        ])
        .unwrap()
        .build()
        .unwrap();
    let configured = ConfiguredBinary::absolute(
        AbsolutePath::new(fixture.executable().to_string_lossy()).unwrap(),
    );
    let environment = BTreeMap::from([
        (OsString::from("HOME"), OsString::from("/isolated/home")),
        (
            OsString::from("XDG_CONFIG_HOME"),
            OsString::from("/isolated/config"),
        ),
        (
            OsString::from("XDG_CACHE_HOME"),
            OsString::from("/isolated/cache"),
        ),
        (
            OsString::from("CODEX_HOME"),
            OsString::from("/isolated/codex"),
        ),
        (
            OsString::from("CLAUDE_CONFIG_DIR"),
            OsString::from("/isolated/claude"),
        ),
        (OsString::from("PATH"), OsString::from("/isolated/bin")),
    ]);

    detect_configured_installation_with_environment(
        CodexAdapter::static_ref(),
        configured,
        None,
        &environment,
        limits().0,
        limits().1,
    )
    .unwrap();

    let capture = fixture.captured_invocation().unwrap();
    for (name, expected) in &environment {
        let name = name.to_str().unwrap();
        assert_eq!(
            capture.environment().get(name).unwrap().as_deref(),
            Some(expected.as_encoded_bytes())
        );
    }
    assert_eq!(capture.environment().get("UNLISTED"), Some(&None));
}

#[test]
fn known_and_unknown_versions_are_reachable_without_profile_guessing() {
    let (known_root, _known_fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (process_limits, json_limits) = limits();
    let known = detect_installation(
        CodexAdapter::static_ref(),
        known_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    match known.reachability() {
        HarnessReachability::Reachable { native_version, .. } => {
            assert_eq!(native_version.as_str(), "0.144.1")
        }
        HarnessReachability::Unreachable { .. } => panic!("known fixture is reachable"),
    }

    let (unknown_root, _unknown_fixture) = install(FakeNativeMode::VersionUnknown, "codex");
    let unknown = detect_installation(
        CodexAdapter::static_ref(),
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
fn exact_real_versions_are_reachable_and_select_exact_profiles() {
    for (harness, profile, binary, expected) in [
        (
            CodexAdapter::static_ref(),
            FakeHarnessProfile::codex(),
            "codex",
            "0.144.1",
        ),
        (
            ClaudeAdapter::static_ref(),
            FakeHarnessProfile::claude(),
            "claude",
            "2.1.201",
        ),
    ] {
        let (root, _fixture) = install_profile(&profile, binary);
        let (process_limits, json_limits) = limits();
        let installation = detect_installation(
            harness,
            root.path().as_os_str().to_os_string(),
            process_limits,
            json_limits,
        )
        .unwrap();
        let HarnessReachability::Reachable { native_version, .. } = installation.reachability()
        else {
            panic!("real version fixture must be reachable");
        };
        assert_eq!(native_version.as_str(), expected);
        assert!(
            harness
                .select_profile(native_version)
                .mutation_capabilities()
                .is_some()
        );
    }
}

#[test]
fn pi_profiles_preserve_exact_bytes_and_unknown_versions_remain_observe_only() {
    for (profile, expected_version, mutation_authorized) in [
        (FakeHarnessProfile::pi(), "0.80.6", false),
        (
            FakeHarnessProfile::pi_with_version("0.80.7"),
            "0.80.7",
            false,
        ),
    ] {
        let (root, fixture) = install_profile(&profile, "pi");
        let (process_limits, json_limits) = limits();
        let installation = detect_installation(
            PiAdapter::static_ref(),
            root.path().as_os_str().to_os_string(),
            process_limits,
            json_limits,
        )
        .unwrap();
        let HarnessReachability::Reachable { native_version, .. } = installation.reachability()
        else {
            panic!("Pi fixture must remain reachable");
        };
        assert_eq!(native_version.as_str(), expected_version);
        assert_eq!(
            fixture.captured_invocation().unwrap().arguments(),
            &[b"--version".to_vec()]
        );
        assert_eq!(
            PiAdapter::static_ref()
                .select_profile(native_version)
                .mutation_capabilities()
                .is_some(),
            mutation_authorized
        );
        assert!(PiAdapter::static_ref().conditional_profile().is_some());
        assert!(PiAdapter::static_ref().native_lifecycle().is_none());
    }
}

#[test]
fn configured_absolute_binary_is_used_for_detection_without_path_lookup() {
    let (root, fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (process_limits, json_limits) = limits();
    let configured = ConfiguredBinary::absolute(
        AbsolutePath::new(root.join("codex").to_str().unwrap()).unwrap(),
    );
    let installation = detect_configured_installation(
        CodexAdapter::static_ref(),
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
    assert_eq!(
        fixture.captured_invocation().unwrap().arguments(),
        &[b"--version".to_vec()]
    );
}

#[test]
fn cross_harness_and_extra_document_versions_are_rejected() {
    for (harness, profile, binary) in [
        (
            CodexAdapter::static_ref(),
            FakeHarnessProfile::claude(),
            "codex",
        ),
        (
            ClaudeAdapter::static_ref(),
            FakeHarnessProfile::codex(),
            "claude",
        ),
    ] {
        let (root, _fixture) = install_profile(&profile, binary);
        let (process_limits, json_limits) = limits();
        assert_eq!(
            detect_installation(
                harness,
                root.path().as_os_str().to_os_string(),
                process_limits,
                json_limits,
            )
            .unwrap_err(),
            DetectionError::InvalidVersion
        );
    }

    let (root, _fixture) = install(FakeNativeMode::ExtraJsonDocument, "codex");
    let (process_limits, json_limits) = limits();
    assert_eq!(
        detect_installation(
            CodexAdapter::static_ref(),
            root.path().as_os_str().to_os_string(),
            process_limits,
            json_limits,
        )
        .unwrap_err(),
        DetectionError::InvalidVersion
    );
}

#[test]
fn malformed_native_output_is_typed_and_secret_safe() {
    let (root, _fixture) = install(FakeNativeMode::DuplicateJson, "claude");
    let (process_limits, json_limits) = limits();
    let error = detect_installation(
        ClaudeAdapter::static_ref(),
        root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap_err();
    assert_eq!(error, DetectionError::InvalidVersion);
    assert!(!format!("{error:?}").contains("3.0.1"));
}

#[test]
fn flood_native_output_is_bounded_and_secret_safe() {
    let (root, _fixture) = install(
        FakeNativeMode::Flood {
            stdout_bytes: 32_768,
            stderr_bytes: 32_768,
        },
        "codex",
    );
    let (process_limits, json_limits) = limits();
    let error = detect_installation(
        CodexAdapter::static_ref(),
        root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DetectionError::Runtime(ObservationRuntimeError::ProcessOutputLimitExceeded { .. })
            | DetectionError::Runtime(ObservationRuntimeError::ProcessDeadlineExceeded)
    ));
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("xxxxxxxx"));
    assert!(!rendered.contains("yyyyyyyy"));
}

#[test]
fn nonzero_and_timeout_version_commands_remain_distinct_failures() {
    let (nonzero_root, _fixture) = install(FakeNativeMode::Exit(17), "codex");
    let (process_limits, json_limits) = limits();
    assert_eq!(
        detect_installation(
            CodexAdapter::static_ref(),
            nonzero_root.path().as_os_str().to_os_string(),
            process_limits,
            json_limits,
        )
        .unwrap_err(),
        DetectionError::NonZeroExit
    );

    let (timeout_root, _fixture) = install(FakeNativeMode::Hang, "claude");
    let timeout_limits = ProcessLimits::new(50, 4_096, 4_096, 8_192).unwrap();
    assert_eq!(
        detect_installation(
            ClaudeAdapter::static_ref(),
            timeout_root.path().as_os_str().to_os_string(),
            timeout_limits,
            json_limits,
        )
        .unwrap_err(),
        DetectionError::Runtime(ObservationRuntimeError::ProcessDeadlineExceeded)
    );
}

#[test]
fn missing_binary_and_explicit_unreachable_results_do_not_probe() {
    let (process_limits, json_limits) = limits();
    let missing = TempRoot::new("skilltap-detection-missing").unwrap();
    let error = detect_installation(
        CodexAdapter::static_ref(),
        missing.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap_err();
    assert_eq!(
        error,
        DetectionError::Runtime(ObservationRuntimeError::ExecutableNotFound)
    );

    let unavailable =
        unreachable_installation(ClaudeAdapter::static_ref(), UnreachableReason::NotFound);
    assert!(matches!(
        unavailable.reachability(),
        HarnessReachability::Unreachable {
            reason: UnreachableReason::NotFound
        }
    ));
    assert!(!Path::new("/tmp/skilltap").exists());
}

#[test]
fn blocked_candidate_reports_match_registry_absence_and_first_party_bootstrap_scope() {
    let reports = blocked_candidate_admission_reports();
    assert_eq!(
        reports
            .iter()
            .map(CandidateAdmissionReport::candidate)
            .collect::<Vec<_>>(),
        BLOCKED_CANDIDATES.to_vec()
    );
    assert!(
        reports
            .iter()
            .all(|report| report.disposition() == CandidateDisposition::Blocked)
    );

    let registry = TargetRegistry::canonical();
    assert_eq!(
        registry.ids().map(HarnessId::as_str).collect::<Vec<_>>(),
        ["codex", "claude", "gemini", "opencode", "pi"]
    );
    assert_eq!(
        registry
            .first_party_targets()
            .map(|adapter| adapter.identity().id.to_string())
            .collect::<Vec<_>>(),
        ["codex".to_owned(), "claude".to_owned()]
    );

    for report in reports {
        let id = HarnessId::new(report.candidate()).unwrap();
        assert!(!registry.contains(&id));
        assert!(registry.adapter(&id).is_none());
        assert!(
            registry
                .iter()
                .all(|adapter| adapter.identity().id.as_str() != report.candidate())
        );
    }
}

#[test]
fn known_profiles_grant_mutation_and_unknown_versions_remain_observe_only() {
    let known = skilltap_core::domain::NativeVersion::new("0.144.1").unwrap();
    let unknown = skilltap_core::domain::NativeVersion::new("99.0.0").unwrap();
    let known_profile = CodexAdapter::static_ref().select_profile(&known);
    assert!(known_profile.mutation_capabilities().is_some());
    assert_eq!(
        known_profile.profile_id().unwrap().as_str(),
        "codex-0-144-1"
    );

    for version in ["3.0.0", "0.144.0", "0.144.2", "2.1.201"] {
        assert!(
            CodexAdapter::static_ref()
                .select_profile(&skilltap_core::domain::NativeVersion::new(version).unwrap())
                .mutation_capabilities()
                .is_none(),
            "{version} must not select the exact Codex profile"
        );
    }

    let unknown_profile = CodexAdapter::static_ref().select_profile(&unknown);
    assert!(unknown_profile.mutation_capabilities().is_none());
    assert!(unknown_profile.profile_id().is_none());
}

#[test]
fn probe_narrowing_is_strict_and_never_widens_profiles() {
    let known = skilltap_core::domain::NativeVersion::new("0.144.1").unwrap();
    let profile = CodexAdapter::static_ref().select_profile(&known);
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
    assert_eq!(probed.profile_id().unwrap().as_str(), "codex-0-144-1");
}

#[test]
fn detection_pipeline_is_repeatable_and_keeps_sibling_failures_isolated() {
    let (codex_root, _codex_fixture) = install(FakeNativeMode::VersionKnown, "codex");
    let (claude_root, _claude_fixture) = install(FakeNativeMode::MalformedJson, "claude");
    let (process_limits, json_limits) = limits();
    let codex_before = fs::read_dir(codex_root.path()).unwrap().count();
    let first = detect_installation(
        CodexAdapter::static_ref(),
        codex_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    )
    .unwrap();
    let second = detect_installation(
        CodexAdapter::static_ref(),
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
        ClaudeAdapter::static_ref(),
        claude_root.path().as_os_str().to_os_string(),
        process_limits,
        json_limits,
    );
    assert!(matches!(claude, Err(DetectionError::InvalidVersion)));
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
fn canonical_codex_observation_names_only_documented_roots() {
    let root = TempRoot::new("skilltap-codex-canonical").unwrap();
    let home = root.join("home");
    let codex_home = root.join("codex");
    fs::create_dir_all(home.join(".agents/skills/shared")).unwrap();
    fs::create_dir_all(codex_home.join("skills/example")).unwrap();
    fs::create_dir_all(codex_home.join("plugins/sample")).unwrap();
    fs::write(
        codex_home.join("skills/example/SKILL.md"),
        b"name: example\n",
    )
    .unwrap();
    fs::write(codex_home.join("unrelated.txt"), b"must not be observed").unwrap();
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(EnvironmentVariable::CodexHome, codex_home.to_str().unwrap());
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let paths = skilltap_harnesses::codex_observation_paths(&platform, &Scope::Global).unwrap();
    let roots = observe_codex_canonical_resources(
        &paths,
        &Scope::Global,
        ExternalTreeLimits::new(8, 64, 4096, 8192, 1024).unwrap(),
    )
    .unwrap();
    let names = roots
        .iter()
        .map(|root| root.root.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["agents.skills", "codex.skills", "codex.plugins"]
    );
    assert!(roots.iter().all(|root| {
        root.snapshot
            .entries()
            .iter()
            .all(|entry| !entry.path().as_str().contains("unrelated"))
    }));
}

#[test]
fn project_resource_observation_stays_inside_documented_native_roots() {
    let root = TempRoot::new("skilltap-project-resources").unwrap();
    let home = root.join("home");
    let project = root.join("project");
    let skill = project.join(".agents/skills/example");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), b"name: example\n").unwrap();
    std::fs::write(project.join("unrelated.txt"), b"not observed").unwrap();
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(
            EnvironmentVariable::CodexHome,
            root.join("codex").to_str().unwrap(),
        );
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let limits = ExternalTreeLimits::new(8, 64, 4096, 8192, 1024).unwrap();
    let paths = skilltap_harnesses::codex_observation_paths(
        &platform,
        &Scope::Project(AbsolutePath::new(project.to_str().unwrap()).unwrap()),
    )
    .unwrap();
    let count = skilltap_harnesses::observe_codex_project_resources(&paths, limits).unwrap();
    assert!(count >= 2);
}

#[test]
fn empty_project_canonical_observation_is_healthy_and_empty() {
    let root = TempRoot::new("skilltap-empty-project-canonical").unwrap();
    let home = root.join("home");
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(
            EnvironmentVariable::CodexHome,
            root.join("codex").to_str().unwrap(),
        );
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let paths = skilltap_harnesses::codex_observation_paths(
        &platform,
        &Scope::Project(AbsolutePath::new(project.to_str().unwrap()).unwrap()),
    )
    .unwrap();
    let observations = observe_codex_canonical_resources(
        &paths,
        &Scope::Project(AbsolutePath::new(project.to_str().unwrap()).unwrap()),
        ExternalTreeLimits::new(8, 64, 4096, 8192, 1024).unwrap(),
    )
    .unwrap();
    assert!(observations.is_empty());
}

#[test]
fn codex_canonical_global_observation_stays_inside_codex_roots() {
    let root = TempRoot::new("skilltap-codex-canonical").unwrap();
    let home = root.join("home");
    let codex_home = root.join("codex");
    std::fs::create_dir_all(home.join(".agents/skills/example")).unwrap();
    std::fs::create_dir_all(codex_home.join("skills")).unwrap();
    std::fs::write(
        home.join(".agents/skills/example/SKILL.md"),
        b"---\nname: example\n---\n",
    )
    .unwrap();
    std::fs::write(home.join("AGENTS.md"), b"instructions\n").unwrap();
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(EnvironmentVariable::CodexHome, codex_home.to_str().unwrap());
    let platform = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let paths = skilltap_harnesses::codex_observation_paths(&platform, &Scope::Global).unwrap();
    let observations = skilltap_harnesses::observe_codex_canonical_resources(
        &paths,
        &Scope::Global,
        ExternalTreeLimits::new(8, 64, 4096, 8192, 1024).unwrap(),
    )
    .unwrap();
    assert!(observations.iter().all(|value| {
        value.root == "agents.skills"
            || value.root == "codex.skills"
            || value.root == "codex.plugins"
    }));
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
