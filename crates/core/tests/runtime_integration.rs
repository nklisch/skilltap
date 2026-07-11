use std::{collections::BTreeMap, ffi::OsString, fs, path::Path};

use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, NativeId},
    runtime::{
        Environment, EnvironmentVariable, ExecutableResolutionRequest, ExecutableResolver,
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        NativeProcessRequest, NativeProcessRunner, PlatformPaths, ProcessLimits, StrictJson,
        StrictJsonDecoder, SupportedPlatform, SystemExecutableResolver, SystemExternalTreeObserver,
        SystemNativeProcessRunner,
    },
};
use skilltap_test_support::{ExternalTreeFixture, TempRoot};

fn printf_path() -> &'static Path {
    ["/usr/bin/printf", "/bin/printf"]
        .into_iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .expect("a native printf executable is required for runtime integration")
}

fn printf_resolution() -> skilltap_core::domain::ExecutableIdentity {
    let path = printf_path();
    let directory = path.parent().unwrap();
    SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            ConfiguredBinary::path_lookup(NativeId::new("printf").unwrap()).unwrap(),
            Some(OsString::from(directory)),
        ))
        .unwrap()
}

fn process_limits() -> ProcessLimits {
    ProcessLimits::new(1_000, 4_096, 4_096, 8_192).unwrap()
}

#[test]
fn resolve_run_decode_is_repeatable_and_secret_safe() {
    let executable = printf_resolution();
    let source = br#"{"answer":42,"message":"integration"}"#;
    let request = NativeProcessRequest::new(
        executable,
        [OsString::from(String::from_utf8(source.to_vec()).unwrap())],
        BTreeMap::new(),
        None,
        process_limits(),
    );
    let decoder = StrictJson;
    let limits = JsonLimits::new(4_096, 16).unwrap();

    let first = SystemNativeProcessRunner.run(&request).unwrap();
    let second = SystemNativeProcessRunner.run(&request).unwrap();
    assert!(first.status().success());
    assert_eq!(first.stdout(), second.stdout());
    assert_eq!(first.stderr(), second.stderr());
    assert_eq!(
        decoder.decode(first.stdout(), limits).unwrap().value()["answer"],
        42
    );
    assert!(!format!("{first:?}").contains("integration"));
    assert!(
        !format!("{:?}", decoder.decode(first.stdout(), limits).unwrap()).contains("integration")
    );

    let duplicate = br#"{"secret":"do-not-render","secret":true}"#;
    let error = decoder.decode(duplicate, limits).unwrap_err();
    assert_eq!(
        error,
        skilltap_core::runtime::ObservationRuntimeError::JsonDuplicateKey
    );
    assert!(!format!("{error:?}").contains("do-not-render"));
}

#[derive(Default)]
struct TestEnvironment(BTreeMap<&'static str, OsString>);

impl TestEnvironment {
    fn with(mut self, variable: EnvironmentVariable, value: impl Into<OsString>) -> Self {
        self.0.insert(variable.as_str(), value.into());
        self
    }
}

impl Environment for TestEnvironment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
        self.0.get(variable.as_str()).cloned()
    }
}

#[test]
fn path_resolution_keeps_codex_home_and_global_agents_isolated() {
    let root = TempRoot::new("skilltap-runtime-paths").unwrap();
    let home = root.join("home");
    let xdg = root.join("xdg");
    let codex = root.join("custom-codex");
    let environment = TestEnvironment::default()
        .with(EnvironmentVariable::Home, home.to_str().unwrap())
        .with(EnvironmentVariable::XdgConfigHome, xdg.to_str().unwrap())
        .with(EnvironmentVariable::CodexHome, codex.to_str().unwrap());

    let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    assert_eq!(
        paths.global_agents().as_str(),
        home.join("AGENTS.md").to_str().unwrap()
    );
    assert_eq!(paths.codex_home().as_str(), codex.to_str().unwrap());
    assert_eq!(paths.config_home().as_str(), xdg.to_str().unwrap());
    assert_eq!(
        paths.skilltap_config().as_str(),
        xdg.join("skilltap").to_str().unwrap()
    );
    assert!(!home.exists());
    assert!(!xdg.exists());
    assert!(!codex.exists());
}

#[test]
fn external_tree_snapshot_is_bounded_repeatable_and_read_only() {
    let fixture = ExternalTreeFixture::new().unwrap();
    let file = fixture
        .resolve(Path::new("skills/example/SKILL.md"))
        .unwrap();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"name: example\n").unwrap();
    let before = fs::read(&file).unwrap();
    let root = AbsolutePath::new(fixture.root().to_str().unwrap()).unwrap();
    let request = ExternalTreeRequest::new(
        root,
        ExternalTreeLimits::new(8, 32, 4_096, 8_192, 1_024).unwrap(),
    );

    let first = SystemExternalTreeObserver.observe(&request).unwrap();
    let second = SystemExternalTreeObserver.observe(&request).unwrap();
    assert_eq!(first, second);
    assert!(
        first
            .entries()
            .iter()
            .any(|entry| entry.path().as_str() == "skills/example/SKILL.md")
    );
    assert_eq!(fs::read(file).unwrap(), before);
    assert!(!format!("{first:?}").contains("name: example"));
}
