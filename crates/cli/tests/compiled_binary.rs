use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::SystemTime,
};

use serde_json::Value;
use skilltap_core::VERSION;
use skilltap_test_support::{
    FakeHarnessProfile, FakeLifecycleAction, FakeNativeMode, FakeNativeProcess, IsolatedMachine,
    captured_stderr, captured_stdout, compiled_binary,
};

const ENABLED_CONFIG: &str = r#"schema = 1

[harnesses.codex]
enabled = true
binary = "codex"

[harnesses.claude]
enabled = true
binary = "claude"

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"

[bootstrap]
mode = "off"
allow_major = false
"#;

fn machine() -> IsolatedMachine {
    IsolatedMachine::new("skilltap-compiled-cli").expect("create isolated machine")
}

struct InstalledFakeHarness {
    _fixture: FakeNativeProcess,
    executable: PathBuf,
}

impl InstalledFakeHarness {
    fn executable(&self) -> &Path {
        &self.executable
    }
}

fn fake_harness(machine: &IsolatedMachine, profile: &FakeHarnessProfile) -> InstalledFakeHarness {
    // Executable resolution canonicalizes symlinks. Publish the profile's
    // ordinary alias so the sealed wrapper still finds its sibling behavior
    // file after canonicalization.
    let fixture = profile
        .builder(FakeNativeMode::VersionKnown)
        .build()
        .expect("build isolated fake harness");
    let destination = machine.working_directory().join(profile.id());
    let executable = fixture
        .install_alias(&destination, profile.id())
        .expect("install isolated fake harness alias");
    InstalledFakeHarness {
        _fixture: fixture,
        executable,
    }
}

fn binary() -> std::path::PathBuf {
    compiled_binary(env!("CARGO_BIN_EXE_skilltap")).expect("resolve compiled skilltap binary")
}

fn config_root(machine: &IsolatedMachine) -> std::path::PathBuf {
    machine.configuration_home().join("skilltap")
}

fn native_config(codex: &Path, claude: &Path) -> String {
    format!(
        r#"schema = 1

[harnesses.codex]
enabled = true
binary = {}

[harnesses.claude]
enabled = true
binary = {}

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"

[bootstrap]
mode = "off"
allow_major = false
"#,
        toml_string(codex),
        toml_string(claude),
    )
}

fn native_config_with_gemini(codex: &Path, claude: &Path, gemini: &Path) -> String {
    format!(
        "{}\n[harnesses.gemini]\nenabled = true\nbinary = {}\n",
        native_config(codex, claude),
        toml_string(gemini),
    )
}

fn write_gemini_harness(machine: &IsolatedMachine, version: &str) -> PathBuf {
    let executable = machine.working_directory().join("gemini");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf '%s\\n' {}\n",
            toml_string(Path::new(version))
        ),
    )
    .expect("write Gemini version fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&executable)
            .expect("read Gemini version fixture")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions)
            .expect("make Gemini version fixture executable");
    }
    executable
}

fn write_gemini_marketplace(machine: &IsolatedMachine) -> PathBuf {
    let source = machine.home().join("gemini-marketplace");
    fs::create_dir_all(source.join(".agents/plugins")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/.codex-plugin")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/skills/demo/scripts")).unwrap();
    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{"name":"team","plugins":[{"name":"demo","source":{"source":"local","path":"./plugins/demo"}}]}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/plugin.json"),
        r#"{"name":"demo","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"demo-docs":{"command":"demo-mcp","args":["serve"]}}}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: Gemini compiled fixture\n---\nbody\n",
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/scripts/run.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .unwrap();
    source
}

fn write_owned(machine: &IsolatedMachine, name: &str, contents: &str) {
    let root = config_root(machine);
    fs::create_dir_all(&root).expect("create configuration root");
    fs::write(root.join(name), contents).expect("write owned document");
}

fn run(machine: &IsolatedMachine, arguments: &[&str]) -> Output {
    machine
        .run(&binary(), arguments)
        .expect("run compiled skilltap binary")
}

fn stdout(output: &Output) -> &str {
    captured_stdout(output).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    captured_stderr(output).expect("stderr is UTF-8")
}

fn json(output: &Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "JSON wrote stderr: {}",
        stderr(output)
    );
    let document = stdout(output);
    assert_eq!(document.lines().count(), 1, "JSON must be one document");
    serde_json::from_str(document).expect("stdout is one JSON document")
}

#[derive(Debug, Eq, PartialEq)]
struct NativeEntrySnapshot {
    relative: PathBuf,
    kind: &'static str,
    bytes: Option<Vec<u8>>,
    link: Option<PathBuf>,
    modified: Option<SystemTime>,
}

fn snapshot_native_tree(root: &Path) -> Vec<NativeEntrySnapshot> {
    fn visit(root: &Path, current: &Path, entries: &mut Vec<NativeEntrySnapshot>) {
        let metadata = fs::symlink_metadata(current).expect("read native metadata");
        let file_type = metadata.file_type();
        let kind = if file_type.is_dir() {
            "directory"
        } else if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };
        entries.push(NativeEntrySnapshot {
            relative: current
                .strip_prefix(root)
                .expect("native entry is beneath root")
                .to_owned(),
            kind,
            bytes: file_type
                .is_file()
                .then(|| fs::read(current).expect("read native file")),
            link: file_type
                .is_symlink()
                .then(|| fs::read_link(current).expect("read native link")),
            modified: metadata.modified().ok(),
        });
        if file_type.is_dir() {
            let mut children = fs::read_dir(current)
                .expect("read native directory")
                .map(|entry| entry.expect("read native entry").path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(root, &child, entries);
            }
        }
    }

    if !root.exists() {
        return Vec::new();
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries
}

fn assert_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        stderr(output)
    );
}

#[test]
fn release_binary_exposes_version_help_and_the_complete_leaf_grammar() {
    let machine = machine();
    let version = run(&machine, &["--version"]);
    assert_code(&version, 0);
    assert_eq!(stdout(&version).trim(), format!("skilltap {VERSION}"));
    assert!(version.stderr.is_empty());

    let root_help = run(&machine, &["--help"]);
    assert_code(&root_help, 0);
    assert!(stdout(&root_help).contains("Registered harnesses: codex|claude"));

    for arguments in [
        vec!["--help"],
        vec!["harness", "--help"],
        vec!["harness", "enable", "--help"],
        vec!["adopt", "--help"],
        vec!["status", "--help"],
        vec!["plan", "--help"],
        vec!["sync", "--help"],
        vec!["bootstrap", "--help"],
        vec!["marketplace", "--help"],
        vec!["plugin", "--help"],
        vec!["skill", "--help"],
        vec!["instructions", "--help"],
        vec!["daemon", "--help"],
    ] {
        let output = run(&machine, &arguments);
        assert_code(&output, 0);
        assert!(
            stdout(&output).contains("Usage:"),
            "arguments: {arguments:?}"
        );
        assert!(output.stderr.is_empty(), "arguments: {arguments:?}");
    }

    let leaves: &[(&[&str], &str)] = &[
        (&["harness", "list"], "harness list"),
        (&["harness", "enable", "codex"], "harness enable"),
        (&["harness", "disable", "codex"], "harness disable"),
        (&["adopt"], "adopt"),
        (&["plan"], "plan"),
        (&["sync"], "sync"),
        (
            &["marketplace", "add", "https://example.invalid/team.git"],
            "marketplace add",
        ),
        (&["marketplace", "remove", "team"], "marketplace remove"),
        (&["marketplace", "update"], "marketplace update"),
        (&["marketplace", "list"], "marketplace list"),
        (&["plugin", "install", "format@team"], "plugin install"),
        (&["plugin", "remove", "format@team"], "plugin remove"),
        (&["plugin", "update"], "plugin update"),
        (&["plugin", "list"], "plugin list"),
        (
            &["skill", "install", "https://example.invalid/skill.git"],
            "skill install",
        ),
        (&["skill", "remove", "release-helper"], "skill remove"),
        (&["skill", "update"], "skill update"),
        (&["skill", "list"], "skill list"),
        (&["instructions", "setup"], "instructions setup"),
        (&["instructions", "status"], "instructions status"),
        (&["instructions", "repair"], "instructions repair"),
        (&["daemon", "enable"], "daemon enable"),
        (&["daemon", "disable"], "daemon disable"),
        (&["daemon", "status"], "daemon status"),
        (&["daemon", "run"], "daemon run"),
    ];
    for (arguments, command) in leaves {
        let mut arguments = arguments.to_vec();
        if *command != "daemon run" {
            arguments.push("--json");
        }
        let output = if *command == "harness list" {
            machine
                .run_with_path(&binary(), &arguments, machine.working_directory())
                .expect("run harness list with isolated PATH")
        } else {
            run(&machine, &arguments)
        };
        if command.starts_with("harness ") {
            assert_code(&output, if *command == "harness list" { 2 } else { 0 });
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(
                value["result"],
                if *command == "harness list" {
                    "attention_required"
                } else {
                    "completed"
                }
            );
        } else if *command == "daemon run" {
            assert!(output.status.code() == Some(0) || output.status.code() == Some(2));
            assert!(stderr(&output).is_empty());
        } else if command.starts_with("daemon ") {
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert!(
                value["result"] == "completed" || value["result"] == "attention_required",
                "arguments: {arguments:?}, output: {value}"
            );
        } else if *command == "skill update" {
            assert_code(&output, 0);
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(value["result"], "completed");
            assert_eq!(value["summary"]["changed"], false);
        } else if matches!(
            *command,
            "plan"
                | "sync"
                | "skill list"
                | "marketplace list"
                | "plugin list"
                | "instructions status"
                | "marketplace add"
                | "marketplace remove"
                | "marketplace update"
                | "plugin install"
                | "plugin remove"
                | "plugin update"
                | "skill install"
                | "skill remove"
                | "instructions setup"
                | "instructions repair"
        ) {
            assert_code(&output, 2);
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(value["result"], "attention_required");
        } else if *command == "adopt" {
            assert_code(&output, 2);
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(value["result"], "attention_required");
            assert_eq!(value["errors"][0]["code"], "no_enabled_harnesses");
        } else {
            assert_code(&output, 1);
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(value["errors"][0]["code"], "capability_unavailable");
        }
    }
}

#[test]
fn compiled_leaf_help_is_the_agent_discovery_contract() {
    let machine = machine();
    let leaves: &[&[&str]] = &[
        &["harness", "list"],
        &["harness", "enable"],
        &["harness", "disable"],
        &["adopt"],
        &["status"],
        &["plan"],
        &["sync"],
        &["bootstrap"],
        &["marketplace", "add"],
        &["marketplace", "remove"],
        &["marketplace", "update"],
        &["marketplace", "list"],
        &["plugin", "install"],
        &["plugin", "remove"],
        &["plugin", "update"],
        &["plugin", "list"],
        &["skill", "install"],
        &["skill", "remove"],
        &["skill", "update"],
        &["skill", "list"],
        &["instructions", "setup"],
        &["instructions", "status"],
        &["instructions", "repair"],
        &["daemon", "enable"],
        &["daemon", "disable"],
        &["daemon", "status"],
        &["daemon", "run"],
    ];

    for path in leaves {
        let mut arguments = path.to_vec();
        arguments.push("--help");
        let output = run(&machine, &arguments);
        assert_code(&output, 0);
        assert!(output.stderr.is_empty(), "arguments: {arguments:?}");
        let help = stdout(&output);
        assert!(help.contains("Usage:"), "arguments: {arguments:?}");
        assert!(
            help.contains("Exit status: 0 completed"),
            "arguments: {arguments:?}"
        );
        assert!(help.contains("-h, --help"), "arguments: {arguments:?}");
    }
}

#[test]
fn compiled_invalid_invocations_use_safe_channels_and_boundaries() {
    let machine = machine();

    let remove_help = run(&machine, &["plugin", "remove", "--help"]);
    assert_code(&remove_help, 0);
    assert!(stdout(&remove_help).contains("PLUGIN@MARKETPLACE"));

    let remove_plain = run(&machine, &["plugin", "remove", "formatter"]);
    assert_code(&remove_plain, 1);
    assert!(remove_plain.stdout.is_empty());
    assert!(stderr(&remove_plain).contains("skilltap plugin remove --help"));

    let remove_json = run(&machine, &["plugin", "remove", "formatter", "--json"]);
    assert_code(&remove_json, 1);
    assert_eq!(json(&remove_json)["command"], "plugin remove");
    assert_eq!(
        json(&remove_json)["next_actions"][0]["command"],
        "skilltap plugin remove --help"
    );

    let plain = run(&machine, &["status", "--target", "pi"]);
    assert_code(&plain, 1);
    assert!(plain.stdout.is_empty());
    let plain_error = stderr(&plain);
    assert!(plain_error.contains("Code: target_not_registered"));
    assert!(plain_error.contains("harness  pi"));

    let json_output = run(&machine, &["status", "--target", "pi", "--json"]);
    assert_code(&json_output, 1);
    let value = json(&json_output);
    assert_eq!(value["command"], "status");
    assert_eq!(value["errors"][0]["code"], "target_not_registered");
    assert_eq!(value["errors"][0]["context"]["harness"], "pi");

    let source = run(
        &machine,
        &[
            "marketplace",
            "add",
            "https://user:token@example.invalid/repo.git",
            "--json",
        ],
    );
    assert_code(&source, 1);
    let source_value = json(&source);
    assert_eq!(source_value["command"], "marketplace add");
    assert!(!stdout(&source).contains("user:token"));
    assert!(!stdout(&source).contains("example.invalid"));
}

#[test]
fn unregistered_harness_is_rejected_before_state_creation() {
    let machine = machine();
    let result = run(&machine, &["harness", "enable", "not-registered", "--json"]);

    assert_code(&result, 1);
    let value = json(&result);
    assert_eq!(value["errors"][0]["code"], "target_not_registered");
    assert_eq!(value["errors"][0]["context"]["harness"], "not-registered");
    assert!(!config_root(&machine).exists());
}

#[test]
fn harness_policy_commands_are_non_interactive_idempotent_and_first_use_read_only() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let claude_fixture = fake_harness(&machine, &FakeHarnessProfile::claude());

    let first_list = machine
        .run_with_path(
            &binary(),
            &["harness", "list", "--json"],
            machine.working_directory(),
        )
        .expect("run first-use harness list with isolated PATH");
    assert_code(&first_list, 2);
    let first_value = json(&first_list);
    assert_eq!(first_value["command"], "harness list");
    assert_eq!(first_value["result"], "attention_required");
    assert!(
        first_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == "codex" && entry["status"] == "disabled" })
    );
    assert!(
        first_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == "claude" && entry["status"] == "disabled" })
    );
    assert!(!config_root(&machine).exists());

    let fixture_binary = fixture.executable();
    let binary_text = fixture_binary.to_str().expect("fake binary path is UTF-8");
    let enable = run(
        &machine,
        &[
            "harness",
            "enable",
            "codex",
            "--binary",
            binary_text,
            "--json",
        ],
    );
    assert_code(&enable, 0);
    assert_eq!(json(&enable)["result"], "completed");
    let config_path = config_root(&machine).join("config.toml");
    assert!(config_path.is_file());
    let configured = fs::read_to_string(&config_path).unwrap().replace(
        "binary = \"claude\"",
        &format!("binary = {}", toml_string(claude_fixture.executable())),
    );
    fs::write(&config_path, configured).unwrap();
    let initial_bytes = fs::read(&config_path).expect("read enabled config");
    assert!(String::from_utf8_lossy(&initial_bytes).contains(binary_text));
    let initial_mtime = fs::metadata(&config_path)
        .expect("stat enabled config")
        .modified()
        .expect("config mtime");

    let repeat_enable = run(
        &machine,
        &[
            "harness",
            "enable",
            "codex",
            "--binary",
            binary_text,
            "--json",
        ],
    );
    assert_code(&repeat_enable, 0);
    assert_eq!(json(&repeat_enable)["result"], "completed");
    assert_eq!(fs::read(&config_path).unwrap(), initial_bytes);
    assert_eq!(
        fs::metadata(&config_path).unwrap().modified().unwrap(),
        initial_mtime
    );

    let list_plain = machine
        .run_with_path(&binary(), &["harness", "list"], machine.working_directory())
        .expect("run harness list with isolated PATH");
    assert_code(&list_plain, 2);
    assert!(list_plain.stderr.is_empty());
    assert!(stdout(&list_plain).contains("codex  enabled"));
    assert!(stdout(&list_plain).contains("claude  disabled"));
    assert!(stdout(&list_plain).contains("gemini  disabled"));
    assert!(stdout(&list_plain).contains("Result: attention required"));

    let disable = run(&machine, &["harness", "disable", "codex"]);
    assert_code(&disable, 0);
    assert!(disable.stderr.is_empty());
    assert!(stdout(&disable).contains("codex  disabled"));
    assert!(stdout(&disable).contains("Result: completed"));
    let final_bytes = fs::read(&config_path).expect("read disabled config");
    assert!(String::from_utf8_lossy(&final_bytes).contains("enabled = false"));

    let final_list = run(&machine, &["harness", "list", "--json"]);
    assert_code(&final_list, 2);
    let final_value = json(&final_list);
    assert_eq!(final_value["result"], "attention_required");
    assert!(
        final_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == "codex" && entry["status"] == "disabled" })
    );

    let repeat_disable = run(&machine, &["harness", "disable", "codex", "--json"]);
    assert_code(&repeat_disable, 1);
    assert_eq!(
        json(&repeat_disable)["errors"][0]["code"],
        "harness_already_disabled"
    );
}

#[test]
fn native_marketplace_add_uses_bounded_lifecycle_and_journals_state() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()),
    );
    fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/skills")).unwrap();

    let output = run(
        &machine,
        &[
            "marketplace",
            "add",
            "https://example.invalid/team.git",
            "--name",
            "team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&output, 0);
    let value = json(&output);
    assert_eq!(value["command"], "marketplace add");
    assert_eq!(value["result"], "completed");
    assert_eq!(value["summary"]["changed"], true);
    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory.contains("marketplace:team"));
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("lifecycle:codex:"));

    let repeat = run(
        &machine,
        &[
            "marketplace",
            "add",
            "https://example.invalid/team.git",
            "--name",
            "team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    let repeat_value = json(&repeat);
    assert_eq!(repeat_value["summary"]["changed"], false);
    assert_eq!(
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap(),
        inventory
    );
    let repeated_state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(repeated_state.contains("\"status\": \"no_change\""));

    let update = run(
        &machine,
        &[
            "marketplace",
            "update",
            "team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&update, 0);
    let update_value = json(&update);
    assert_eq!(update_value["result"], "completed");
    assert_eq!(update_value["summary"]["changed"], true);
    assert!(
        update_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["status"] == "observed")
    );

    let second = run(
        &machine,
        &[
            "marketplace",
            "add",
            "https://example.invalid/other.git",
            "--name",
            "other",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&second, 0);
    assert_eq!(json(&second)["result"], "completed");

    let update_all = run(
        &machine,
        &["marketplace", "update", "--target", "codex", "--json"],
    );
    assert_code(&update_all, 0);
    let update_all_value = json(&update_all);
    assert_eq!(update_all_value["result"], "completed");
    assert_eq!(update_all_value["summary"]["changed"], true);
    assert_eq!(update_all_value["summary"]["operations"], 2);

    let remove = run(
        &machine,
        &[
            "marketplace",
            "remove",
            "team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    assert_eq!(json(&remove)["result"], "completed");
    assert!(
        !fs::read_to_string(config_root(&machine).join("inventory.toml"))
            .unwrap()
            .contains("marketplace:team")
    );
}

#[test]
fn native_lifecycle_projects_each_detection_failure_without_sensitive_context() {
    fn assert_failure(claude: &Path, warning_code: &str, action_code: &str) {
        let machine = machine();
        let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
        write_owned(
            &machine,
            "config.toml",
            &native_config(codex.executable(), claude),
        );

        let output = run(
            &machine,
            &[
                "marketplace",
                "add",
                "https://example.invalid/team.git",
                "--name",
                "team",
                "--target",
                "claude",
                "--json",
            ],
        );
        assert_code(&output, 2);
        let value = json(&output);
        assert_eq!(value["result"], "attention_required");
        assert!(value["warnings"].as_array().unwrap().iter().any(|warning| {
            warning["code"] == warning_code && warning["context"]["harness"] == "claude"
        }));
        let action = value["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| action["code"] == action_code)
            .unwrap_or_else(|| panic!("missing {action_code} in {value}"));
        if action_code == "inspect_harness_version" {
            assert_eq!(action["command"], format!("{} --version", claude.display()));
        }
        assert!(
            !value["warnings"].as_array().unwrap().iter().any(|warning| {
                warning["code"] == "native_profile_unavailable"
                    || warning["code"] == "native_detection_failed"
            })
        );
        let rendered = serde_json::to_string(&value).unwrap();
        for forbidden in ["secret-native-output", "argv", "environment"] {
            assert!(!rendered.contains(forbidden));
        }

        let plan = run(&machine, &["plan", "--target", "claude", "--json"]);
        assert_code(&plan, 2);
        let plan_value = json(&plan);
        assert!(
            plan_value["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|warning| {
                    warning["code"] == warning_code && warning["context"]["harness"] == "claude"
                })
        );
        assert!(
            plan_value["next_actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate == action),
            "plan dropped typed recovery action: {plan_value}"
        );
    }

    let missing_root = machine();
    assert_failure(
        &missing_root.home().join("missing-claude"),
        "native_executable_not_found",
        "configure_harness_binary",
    );

    let invalid = FakeNativeProcess::new(FakeNativeMode::MalformedJson).unwrap();
    assert_failure(
        invalid.executable(),
        "native_version_invalid",
        "inspect_harness_version",
    );

    let nonzero = FakeNativeProcess::new(FakeNativeMode::Exit(17)).unwrap();
    assert_failure(
        nonzero.executable(),
        "native_version_command_failed",
        "inspect_harness_version",
    );

    let bounded = FakeNativeProcess::new(FakeNativeMode::Flood {
        stdout_bytes: 300_000,
        stderr_bytes: 0,
    })
    .unwrap();
    assert_failure(
        bounded.executable(),
        "native_detection_bounded",
        "inspect_harness_version",
    );
}

#[test]
fn codex_project_lifecycle_materializes_owned_plugin_without_cache_mutation() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    let project = machine.home().join("managed-project");
    fs::create_dir_all(&project).unwrap();
    let source = machine.home().join("managed-marketplace.git");
    fs::create_dir_all(source.join(".agents/plugins")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/.codex-plugin")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/skills/demo/scripts")).unwrap();
    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "team",
  "future": {"preserve": true},
  "plugins": [{
    "name": "demo",
    "source": {"source": "local", "path": "./plugins/demo"},
    "futurePluginField": "keep"
  }]
}
"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/plugin.json"),
        r#"{"name":"demo","version":"1.0.0","future":true}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"demo-docs":{"command":"demo-mcp","args":["serve"]}}}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: managed project fixture\n---\nbody\n",
    )
    .unwrap();
    let script = source.join("plugins/demo/skills/demo/scripts/run.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).unwrap();
    }

    let add = run(
        &machine,
        &[
            "marketplace",
            "add",
            source.to_str().unwrap(),
            "--name",
            "team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&add, 0);
    assert_eq!(json(&add)["summary"]["changed"], true);
    let catalog_path = project.join(".agents/plugins/marketplace.json");
    let added_catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
    assert_eq!(added_catalog["future"]["preserve"], true);

    let install = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_eq!(json(&install)["summary"]["changed"], true);
    let installed = project.join(".agents/skills/demo");
    assert_eq!(
        fs::read_to_string(installed.join("SKILL.md")).unwrap(),
        "---\nname: demo\ndescription: managed project fixture\n---\nbody\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            fs::metadata(installed.join("scripts/run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    let installed_catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
    assert_eq!(installed_catalog["future"]["preserve"], true);
    assert_eq!(installed_catalog["plugins"][0]["futurePluginField"], "keep");
    assert_eq!(
        installed_catalog["plugins"][0]["source"]["path"],
        "./plugins/demo"
    );
    let codex_config = fs::read_to_string(project.join(".codex/config.toml")).unwrap();
    assert!(codex_config.contains("[mcp_servers.demo-docs]"));
    assert!(codex_config.contains("command = \"demo-mcp\""));
    assert!(!machine.codex_home().join("plugins/cache").exists());
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("\"provenance\": \"materialized\""));
    assert!(state.contains("\"ownership\": \"skilltap\""));
    assert!(state.contains("managed_projections"), "state: {state}");

    let repeat = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    let catalog_update = run(
        &machine,
        &[
            "marketplace",
            "update",
            "team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&catalog_update, 0);
    assert_eq!(json(&catalog_update)["summary"]["changed"], false);

    fs::rename(
        source.join("plugins/demo/skills/demo"),
        source.join("plugins/demo/skills/renamed"),
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"renamed-docs":{"command":"demo-mcp","args":["serve"]}}}"#,
    )
    .unwrap();
    let evolved = run(
        &machine,
        &[
            "plugin",
            "update",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&evolved, 0);
    assert!(
        !project.join(".agents/skills/demo/SKILL.md").exists(),
        "evolved output: {}",
        stdout(&evolved)
    );
    assert!(project.join(".agents/skills/renamed/SKILL.md").is_file());
    let evolved_config = fs::read_to_string(project.join(".codex/config.toml")).unwrap();
    assert!(!evolved_config.contains("demo-docs"));
    assert!(evolved_config.contains("renamed-docs"));
    let evolved_repeat = run(
        &machine,
        &[
            "plugin",
            "update",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&evolved_repeat, 0);
    assert_eq!(json(&evolved_repeat)["summary"]["changed"], false);

    fs::rename(
        source.join("plugins/demo/skills/renamed"),
        source.join("plugins/demo/skills/demo"),
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"demo-docs":{"command":"demo-mcp","args":["serve"]}}}"#,
    )
    .unwrap();
    let restored = run(
        &machine,
        &[
            "plugin",
            "update",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&restored, 0);

    fs::write(installed.join("SKILL.md"), "drift\n").unwrap();
    let drifted = run(
        &machine,
        &[
            "plugin",
            "update",
            "demo@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&drifted, 2);
    assert_eq!(
        json(&drifted)["errors"][0]["code"],
        "managed_project_drifted"
    );
    assert_eq!(
        fs::read_to_string(installed.join("SKILL.md")).unwrap(),
        "drift\n"
    );

    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"demo-docs":{"command":"demo-mcp","args":["serve"]},"plugin-relative":{"command":"${CLAUDE_PLUGIN_ROOT}/bin/server"}}}"#,
    )
    .unwrap();
    for (component, file) in [
        ("hooks/first", "hook.sh"),
        ("hooks/second", "hook.sh"),
        ("lsp-servers/rust", "server.json"),
        ("output_styles/warm", "style.md"),
    ] {
        let root = source.join("plugins/demo").join(component);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(file), "fixture\n").unwrap();
    }

    for arguments in [
        vec!["init"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "user.name", "Fixture"],
        vec!["add", "."],
        vec!["commit", "-m", "fixture"],
    ] {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&source)
            .output()
            .unwrap();
        assert!(output.status.success(), "git fixture command failed");
    }
    let revision = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&source)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let git_project = machine.home().join("git-managed-project");
    fs::create_dir_all(&git_project).unwrap();
    let git_url = format!("file://{}", source.display());
    let git_add = run(
        &machine,
        &[
            "marketplace",
            "add",
            &git_url,
            "--name",
            "git-team",
            "--project",
            git_project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&git_add, 0);
    let partial = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@git-team",
            "--project",
            git_project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&partial, 2);
    assert_eq!(
        json(&partial)["errors"][0]["code"],
        "partial_operation_requires_acknowledgment"
    );
    fs::create_dir_all(git_project.join(".codex")).unwrap();
    let foreign_config = "[mcp_servers.demo-docs]\ncommand = \"foreign\"\n";
    fs::write(git_project.join(".codex/config.toml"), foreign_config).unwrap();
    let foreign = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@git-team",
            "--project",
            git_project.to_str().unwrap(),
            "--target",
            "codex",
            "--yes",
            "--json",
        ],
    );
    assert_code(&foreign, 2);
    assert_eq!(
        json(&foreign)["errors"][0]["code"],
        "managed_project_unowned"
    );
    assert_eq!(
        fs::read_to_string(git_project.join(".codex/config.toml")).unwrap(),
        foreign_config
    );
    fs::remove_file(git_project.join(".codex/config.toml")).unwrap();
    let git_install = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@git-team",
            "--project",
            git_project.to_str().unwrap(),
            "--target",
            "codex",
            "--yes",
            "--json",
        ],
    );
    assert_code(&git_install, 0);
    assert!(
        json(&git_install)["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["id"] == "omitted:mcp:plugin-relative"
                && resource["fields"]["consequence"] == "plugin_root_relative_mcp_omitted")
    );
    for component in ["hook:first", "hook:second", "lsp:rust", "output-style:warm"] {
        assert!(
            json(&git_install)["resources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|resource| resource["id"] == format!("omitted:{component}"))
        );
    }
    assert!(git_project.join(".agents/skills/demo/SKILL.md").is_file());
    let git_config = fs::read_to_string(git_project.join(".codex/config.toml")).unwrap();
    assert!(git_config.contains("demo-docs"));
    assert!(!git_config.contains("plugin-relative"));
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains(revision.trim()));
    assert!(state.contains("plugin_root_relative_mcp_omitted"));
    assert!(config_root(&machine).join("managed/sources").is_dir());

    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{"name":"team","plugins":[]}"#,
    )
    .unwrap();
    let git_remove = run(
        &machine,
        &[
            "plugin",
            "remove",
            "demo@git-team",
            "--project",
            git_project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&git_remove, 0);
    assert!(!git_project.join(".agents/skills/demo/SKILL.md").exists());
    assert!(!git_project.join(".codex/config.toml").exists());

    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{"name":"team","plugins":[{"name":"demo","source":{"source":"local","path":"./plugins/demo"}}]}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"plugin-relative":{"command":"${CLAUDE_PLUGIN_ROOT}/bin/server"}}}"#,
    )
    .unwrap();
    let omission_project = machine.home().join("omission-only-project");
    fs::create_dir_all(&omission_project).unwrap();
    let omission_add = run(
        &machine,
        &[
            "marketplace",
            "add",
            source.to_str().unwrap(),
            "--name",
            "omission-team",
            "--project",
            omission_project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&omission_add, 0);
    let omission_install = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@omission-team",
            "--project",
            omission_project.to_str().unwrap(),
            "--target",
            "codex",
            "--yes",
            "--json",
        ],
    );
    assert_code(&omission_install, 0);
    assert!(
        omission_project
            .join(".agents/skills/demo/SKILL.md")
            .is_file()
    );
    assert!(!omission_project.join(".codex/config.toml").exists());
    assert!(
        json(&omission_install)["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["id"] == "omitted:mcp:plugin-relative")
    );
}

#[test]
fn gemini_exact_profile_manages_global_and_project_plugins_but_unknown_versions_only_observe() {
    let machine = machine();
    let gemini = write_gemini_harness(&machine, "0.50.0");
    let config = native_config_with_gemini(&gemini, &gemini, &gemini)
        .replace(
            "[harnesses.codex]\nenabled = true",
            "[harnesses.codex]\nenabled = false",
        )
        .replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        );
    write_owned(&machine, "config.toml", &config);
    let source = write_gemini_marketplace(&machine);
    let project = machine.home().join("gemini-project");
    fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
    fs::create_dir_all(project.join(".agents/skills")).unwrap();

    for scope in [None, Some(project.as_path())] {
        let mut add = vec![
            "marketplace",
            "add",
            source.to_str().unwrap(),
            "--name",
            "team",
        ];
        if let Some(project) = scope {
            add.extend(["--project", project.to_str().unwrap()]);
        }
        add.extend(["--target", "gemini", "--json"]);
        let output = run(&machine, &add);
        assert_code(&output, 0);
        assert_eq!(json(&output)["result"], "completed");

        let mut install = vec!["plugin", "install", "demo@team"];
        if let Some(project) = scope {
            install.extend(["--project", project.to_str().unwrap()]);
        }
        install.extend(["--target", "gemini", "--json"]);
        let output = run(&machine, &install);
        assert_code(&output, 0);
        assert_eq!(json(&output)["result"], "completed");
    }

    assert!(
        machine
            .home()
            .join(".agents/skills/demo/SKILL.md")
            .is_file()
    );
    assert!(project.join(".agents/skills/demo/SKILL.md").is_file());
    let global_settings: Value =
        serde_json::from_slice(&fs::read(machine.home().join(".gemini/settings.json")).unwrap())
            .unwrap();
    let project_settings: Value =
        serde_json::from_slice(&fs::read(project.join(".gemini/settings.json")).unwrap()).unwrap();
    assert_eq!(
        global_settings["mcpServers"]["demo-docs"]["command"],
        "demo-mcp"
    );
    assert_eq!(
        project_settings["mcpServers"]["demo-docs"]["args"][0],
        "serve"
    );
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(
        state.contains("gemini"),
        "Gemini target state was not recorded: {state}"
    );

    for version in ["0.50.1", "0.49.0"] {
        let machine = IsolatedMachine::new("skilltap-compiled-gemini-unknown")
            .expect("create isolated unknown-version machine");
        let gemini = write_gemini_harness(&machine, version);
        let config = native_config_with_gemini(&gemini, &gemini, &gemini)
            .replace(
                "[harnesses.codex]\nenabled = true",
                "[harnesses.codex]\nenabled = false",
            )
            .replace(
                "[harnesses.claude]\nenabled = true",
                "[harnesses.claude]\nenabled = false",
            );
        write_owned(&machine, "config.toml", &config);
        let source = write_gemini_marketplace(&machine);
        let project = machine.home().join("gemini-unknown-project");
        fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
        fs::create_dir_all(project.join(".agents/skills")).unwrap();

        for scope in [None, Some(project.as_path())] {
            let mut add = vec![
                "marketplace",
                "add",
                source.to_str().unwrap(),
                "--name",
                "team",
            ];
            if let Some(project) = scope {
                add.extend(["--project", project.to_str().unwrap()]);
            }
            add.extend(["--target", "gemini", "--json"]);
            let output = run(&machine, &add);
            assert_code(&output, 2);
            let value = json(&output);
            assert_eq!(value["result"], "attention_required");
            assert!(
                value["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|warning| warning["code"] == "native_capability_unverified")
            );
        }

        let global_roots = [
            machine.home().join(".agents/skills"),
            machine.home().join(".gemini"),
        ];
        let project_roots = [project.join(".agents/skills"), project.join(".gemini")];
        let before = global_roots
            .iter()
            .chain(project_roots.iter())
            .map(|root| snapshot_native_tree(root))
            .collect::<Vec<_>>();
        assert!(!config_root(&machine).join("state.json").exists());

        for scope in [None, Some(project.as_path())] {
            let mut install = vec!["plugin", "install", "demo@team"];
            if let Some(project) = scope {
                install.extend(["--project", project.to_str().unwrap()]);
            }
            install.extend(["--target", "gemini", "--json"]);
            let output = run(&machine, &install);
            assert_code(&output, 2);
            let value = json(&output);
            assert_eq!(value["result"], "attention_required");
            assert_eq!(value["summary"]["changed"], false);
            assert!(
                value["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|warning| warning["code"] == "native_capability_unverified")
            );
        }

        let after = global_roots
            .iter()
            .chain(project_roots.iter())
            .map(|root| snapshot_native_tree(root))
            .collect::<Vec<_>>();
        assert_eq!(
            after, before,
            "unknown Gemini {version} wrote a target surface"
        );
        assert!(!config_root(&machine).join("state.json").exists());
    }
}

#[test]
fn gemini_managed_revalidation_rejects_a_version_change_before_writing() {
    let machine = machine();
    let gemini = write_gemini_harness(&machine, "0.50.0");
    let config = native_config_with_gemini(&gemini, &gemini, &gemini)
        .replace(
            "[harnesses.codex]\nenabled = true",
            "[harnesses.codex]\nenabled = false",
        )
        .replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        );
    write_owned(&machine, "config.toml", &config);
    let source = write_gemini_marketplace(&machine);
    fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
    let add = run(
        &machine,
        &[
            "marketplace",
            "add",
            source.to_str().unwrap(),
            "--name",
            "team",
            "--target",
            "gemini",
            "--json",
        ],
    );
    assert_code(&add, 0);
    let state_path = config_root(&machine).join("state.json");
    let state_before = fs::read(&state_path).expect("source registration state");

    let marker = machine.working_directory().join("gemini-version-flipped");
    let marker_literal = marker.to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        &gemini,
        format!(
            "#!/bin/sh\nif [ -f '{marker}' ]; then printf '%s\\n' '0.50.1'; else : > '{marker}'; printf '%s\\n' '0.50.0'; fi\n",
            marker = marker_literal
        ),
    )
    .unwrap();

    let before = [
        machine.home().join(".agents/skills"),
        machine.home().join(".gemini"),
    ]
    .map(|root| snapshot_native_tree(&root));
    let install = run(
        &machine,
        &[
            "plugin",
            "install",
            "demo@team",
            "--target",
            "gemini",
            "--json",
        ],
    );
    assert_code(&install, 2);
    let value = json(&install);
    assert_eq!(value["result"], "attention_required");
    assert!(
        value["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["code"] == "native_command_failed")
    );
    let after = [
        machine.home().join(".agents/skills"),
        machine.home().join(".gemini"),
    ]
    .map(|root| snapshot_native_tree(&root));
    assert_eq!(after, before);
    assert_eq!(fs::read(&state_path).unwrap(), state_before);
}

#[test]
fn unsupported_only_managed_project_plugin_stays_blocked_with_acknowledgment() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    let project = machine.home().join("unsupported-only-project");
    fs::create_dir_all(&project).unwrap();
    let source = machine.home().join("unsupported-only-marketplace");
    fs::create_dir_all(source.join(".agents/plugins")).unwrap();
    fs::create_dir_all(source.join("plugins/unsupported/.codex-plugin")).unwrap();
    fs::create_dir_all(source.join("plugins/unsupported/hooks/first")).unwrap();
    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{"name":"team","plugins":[{"name":"unsupported","source":{"source":"local","path":"./plugins/unsupported"}}]}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/unsupported/.codex-plugin/plugin.json"),
        r#"{"name":"unsupported","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/unsupported/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"plugin-relative":{"command":"${CLAUDE_PLUGIN_ROOT}/bin/server"}}}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/unsupported/hooks/first/hook.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .unwrap();

    let add = run(
        &machine,
        &[
            "marketplace",
            "add",
            source.to_str().unwrap(),
            "--name",
            "team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&add, 0);
    let project_before = snapshot_native_tree(&project);
    let state_before = snapshot_native_tree(&config_root(&machine));

    for acknowledgment in [None, Some("--yes")] {
        let mut arguments = vec![
            "plugin",
            "install",
            "unsupported@team",
            "--project",
            project.to_str().unwrap(),
            "--target",
            "codex",
        ];
        if let Some(flag) = acknowledgment {
            arguments.push(flag);
        }
        arguments.push("--json");
        let output = run(&machine, &arguments);
        assert_code(&output, 2);
        let value = json(&output);
        assert_eq!(value["result"], "attention_required");
        assert_eq!(value["summary"]["changed"], false);
        assert_eq!(snapshot_native_tree(&project), project_before);
        assert_eq!(snapshot_native_tree(&config_root(&machine)), state_before);
        assert!(!project.join(".agents/skills").exists());
        assert!(!project.join(".codex/config.toml").exists());
        assert!(!machine.codex_home().join("plugins/cache").exists());
    }
}

#[test]
fn targeted_native_remove_preserves_unselected_harness() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/skills")).unwrap();
    let install = run(
        &machine,
        &[
            "marketplace",
            "add",
            "https://example.invalid/team.git",
            "--name",
            "team",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&install, 0);
    let remove = run(
        &machine,
        &[
            "marketplace",
            "remove",
            "team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory.contains("marketplace:team"));
    assert!(inventory.contains("targets = [\"claude\"]"));
}

#[test]
fn local_skill_install_publishes_the_complete_canonical_tree() {
    let machine = machine();
    let source = machine.home().join("source-skill");
    fs::create_dir_all(source.join("docs")).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: demo\ndescription: test skill\n---\nbody\n",
    )
    .unwrap();
    fs::write(source.join("docs/example.txt"), "sibling").unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source_text = source.to_str().unwrap();

    let mismatch = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "other",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&mismatch, 1);
    assert_eq!(json(&mismatch)["errors"][0]["code"], "skill_name_mismatch");

    let output = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&output, 0);
    let value = json(&output);
    assert_eq!(value["result"], "completed");
    assert_eq!(value["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/source-skill/SKILL.md")).unwrap(),
        "---\nname: demo\ndescription: test skill\n---\nbody\n"
    );
    assert_eq!(
        fs::read_to_string(
            machine
                .home()
                .join(".agents/skills/source-skill/docs/example.txt")
        )
        .unwrap(),
        "sibling"
    );
    assert!(
        fs::read_to_string(config_root(&machine).join("inventory.toml"))
            .unwrap()
            .contains("skill:source-skill")
    );
    assert!(
        fs::read_to_string(config_root(&machine).join("state.json"))
            .unwrap()
            .contains("skill:codex:")
    );

    let repeat = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    fs::write(
        source.join("SKILL.md"),
        "---\nname: demo\ndescription: updated skill\n---\nupdated\n",
    )
    .unwrap();
    let replace = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--target",
            "codex",
            "--yes",
            "--json",
        ],
    );
    assert_code(&replace, 2);
    assert_eq!(json(&replace)["summary"]["changed"], false);

    let first_update = run(
        &machine,
        &[
            "skill",
            "update",
            "source-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&first_update, 0);
    assert_eq!(json(&first_update)["summary"]["changed"], true);

    fs::write(
        source.join("SKILL.md"),
        "---\nname: demo\ndescription: updated again\n---\nupdated again\n",
    )
    .unwrap();
    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "source-skill",
            "--target",
            "codex",
            "--yes",
            "--json",
        ],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["summary"]["changed"], true);
    assert!(
        fs::read_dir(config_root(&machine).join("managed"))
            .unwrap()
            .next()
            .is_some()
    );

    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "source-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    assert_eq!(json(&remove)["result"], "completed");
    assert!(!machine.home().join(".agents/skills/source-skill").exists());
    assert!(
        !fs::read_to_string(config_root(&machine).join("inventory.toml"))
            .unwrap()
            .contains("skill:source-skill")
    );
}

#[cfg(unix)]
#[test]
fn whole_skill_modes_are_normalized_for_global_and_project_codex_and_claude() {
    use std::os::unix::fs::PermissionsExt;

    fn write_skill(root: &Path, name: &str) {
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join("references")).unwrap();
        fs::write(
            root.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: mode test\n---\n"),
        )
        .unwrap();
        fs::write(root.join("scripts/run.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::write(root.join("references/plain.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            root.join("scripts/run.sh"),
            fs::Permissions::from_mode(0o7777),
        )
        .unwrap();
        fs::set_permissions(
            root.join("references/plain.sh"),
            fs::Permissions::from_mode(0o6666),
        )
        .unwrap();
    }

    fn assert_normalized(root: &Path) {
        assert_eq!(
            fs::metadata(root.join("SKILL.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        assert_eq!(
            fs::metadata(root.join("scripts/run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
        assert_eq!(
            fs::metadata(root.join("references/plain.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
    }

    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let global_source = machine.home().join("global-mode-skill");
    write_skill(&global_source, "global-mode-skill");
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            global_source.to_str().unwrap(),
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&install, 0);
    for destination in [
        machine.home().join(".agents/skills/global-mode-skill"),
        machine.home().join(".claude/skills/global-mode-skill"),
    ] {
        assert_normalized(&destination);
    }

    fs::set_permissions(
        global_source.join("scripts/run.sh"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "global-mode-skill",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["summary"]["changed"], true);
    for destination in [
        machine
            .home()
            .join(".agents/skills/global-mode-skill/scripts/run.sh"),
        machine
            .home()
            .join(".claude/skills/global-mode-skill/scripts/run.sh"),
    ] {
        assert_eq!(
            fs::metadata(destination).unwrap().permissions().mode() & 0o7777,
            0o600
        );
    }
    let repeat = run(
        &machine,
        &[
            "skill",
            "update",
            "global-mode-skill",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    let drifted = machine
        .home()
        .join(".agents/skills/global-mode-skill/scripts/run.sh");
    fs::set_permissions(&drifted, fs::Permissions::from_mode(0o700)).unwrap();
    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "global-mode-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove, 2);
    assert!(
        json(&remove)["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill_destination_drifted_requires_acknowledgment")
    );
    fs::set_permissions(&drifted, fs::Permissions::from_mode(0o600)).unwrap();

    let project = machine.working_directory().join("mode-project");
    fs::create_dir_all(&project).unwrap();
    let project_source = machine.home().join("project-mode-skill");
    write_skill(&project_source, "project-mode-skill");
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            project_source.to_str().unwrap(),
            "--target",
            "all",
            "--project",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_normalized(&project.join(".agents/skills/project-mode-skill"));
    let claude_link = project.join(".claude/skills/project-mode-skill");
    assert!(
        fs::symlink_metadata(&claude_link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(&claude_link).unwrap(),
        PathBuf::from("../../.agents/skills/project-mode-skill")
    );
    assert_normalized(&claude_link);
}

#[cfg(unix)]
#[test]
fn project_skill_links_are_canonical_idempotent_and_ownership_safe() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    let project = machine.working_directory().join("nested/project");
    fs::create_dir_all(&project).unwrap();
    let source = machine.home().join("project-links-source");
    fs::create_dir_all(source.join("scripts")).unwrap();
    fs::create_dir_all(source.join("references")).unwrap();
    fs::create_dir_all(source.join("assets")).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: project-links\ndescription: project link fixture\nmetadata:\n  future: preserved\n---\nbody\n",
    )
    .unwrap();
    fs::write(source.join("scripts/run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::write(source.join("references/example.md"), "reference\n").unwrap();
    fs::write(source.join("assets/data.bin"), [1_u8, 2, 3, 4]).unwrap();
    #[cfg(unix)]
    fs::set_permissions(
        source.join("scripts/run.sh"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let source_text = source.to_str().unwrap();
    let project_text = project.to_str().unwrap();
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_eq!(json(&install)["summary"]["changed"], true);

    let canonical = project.join(".agents/skills/project-links");
    let claude_link = project.join(".claude/skills/project-links");
    assert!(canonical.join("SKILL.md").is_file());
    assert!(canonical.join("scripts/run.sh").is_file());
    assert_eq!(
        fs::read(canonical.join("assets/data.bin")).unwrap(),
        [1, 2, 3, 4]
    );
    assert!(
        fs::symlink_metadata(&claude_link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(&claude_link).unwrap(),
        PathBuf::from("../../.agents/skills/project-links")
    );
    let original_link_inode = fs::symlink_metadata(&claude_link).unwrap().ino();

    let repeat = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
    assert_eq!(
        fs::symlink_metadata(&claude_link).unwrap().ino(),
        original_link_inode
    );

    fs::remove_file(&claude_link).unwrap();
    let repaired = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&repaired, 0);
    assert!(
        fs::symlink_metadata(&claude_link)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    fs::remove_dir_all(&canonical).unwrap();
    let restored = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&restored, 0);
    assert!(canonical.join("SKILL.md").is_file());
    assert!(
        fs::symlink_metadata(&claude_link)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    fs::remove_file(&claude_link).unwrap();
    symlink(Path::new("../../elsewhere"), &claude_link).unwrap();
    let divergent = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&divergent, 0);
    assert_eq!(
        fs::read_link(&claude_link).unwrap(),
        PathBuf::from("../../.agents/skills/project-links")
    );

    fs::remove_file(&claude_link).unwrap();
    let conflict_bytes = b"unmanaged conflict\n";
    fs::write(&claude_link, conflict_bytes).unwrap();
    let conflict = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&conflict, 2);
    assert_eq!(json(&conflict)["summary"]["changed"], false);
    assert_eq!(fs::read(&claude_link).unwrap(), conflict_bytes);
    assert!(
        json(&conflict)["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill_destination_unmanaged")
    );

    fs::remove_file(&claude_link).unwrap();
    let repair = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "project-links",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&repair, 0);
    let remove_target = run(
        &machine,
        &[
            "skill",
            "remove",
            "project-links",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&remove_target, 0);
    assert!(canonical.join("SKILL.md").is_file());
    assert!(!claude_link.exists());

    let final_remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "project-links",
            "--project",
            project_text,
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&final_remove, 0);
    assert!(!canonical.exists());
}

#[cfg(unix)]
#[test]
fn project_skill_status_exposes_independent_health_and_preserves_read_only_state() {
    use std::os::unix::fs::symlink;

    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    let project = machine.working_directory().join("status-project");
    fs::create_dir_all(project.join(".agents/skills/status-skill")).unwrap();
    fs::write(
        project.join(".agents/skills/status-skill/SKILL.md"),
        "---\nname: status-skill\ndescription: status fixture\n---\nbody\n",
    )
    .unwrap();
    let project_text = project.to_str().unwrap();
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            project
                .join(".agents/skills/status-skill/SKILL.md")
                .parent()
                .unwrap()
                .to_str()
                .unwrap(),
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&install, 0);
    let link = project.join(".claude/skills/status-skill");
    fs::remove_file(&link).unwrap();
    let before = snapshot_native_tree(&config_root(&machine));

    let missing = run(
        &machine,
        &[
            "status",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&missing, 2);
    let missing_value = json(&missing);
    assert!(
        missing_value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill.link.missing")
    );
    let missing_resource = missing_value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| {
            resource["id"].as_str().is_some_and(|id| {
                id == format!("claude:skill:status-skill [project:{project_text}]")
            })
        })
        .unwrap();
    assert_eq!(missing_resource["fields"]["conformance"], "conforming");
    assert_eq!(missing_resource["fields"]["compatibility"], "compatible");
    assert_eq!(missing_resource["fields"]["loadability"], "loadable");
    assert_eq!(missing_resource["fields"]["projection"], "missing");
    assert_eq!(snapshot_native_tree(&config_root(&machine)), before);

    fs::create_dir_all(project.join(".claude/skills")).unwrap();
    symlink(
        Path::new("../../.agents/skills/status-skill"),
        project.join(".claude/skills/status-skill"),
    )
    .unwrap();
    fs::remove_dir_all(project.join(".agents/skills/status-skill")).unwrap();
    let broken = run(
        &machine,
        &[
            "status",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&broken, 2);
    assert!(
        json(&broken)["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill.link.broken")
    );

    fs::create_dir_all(project.join(".agents/skills/status-skill")).unwrap();
    fs::write(
        project.join(".agents/skills/status-skill/SKILL.md"),
        "---\nname: status-skill\ndescription: status fixture\n---\nbody\n",
    )
    .unwrap();
    fs::remove_file(project.join(".claude/skills/status-skill")).unwrap();
    symlink(
        Path::new("../../elsewhere"),
        project.join(".claude/skills/status-skill"),
    )
    .unwrap();
    let divergent = run(
        &machine,
        &[
            "status",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&divergent, 2);
    assert!(
        json(&divergent)["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill.link.divergent")
    );

    fs::remove_file(project.join(".claude/skills/status-skill")).unwrap();
    fs::write(project.join(".claude/skills/status-skill"), b"foreign").unwrap();
    let conflict = run(
        &machine,
        &[
            "status",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&conflict, 2);
    assert_eq!(
        fs::read(project.join(".claude/skills/status-skill")).unwrap(),
        b"foreign"
    );
}

#[test]
fn source_less_project_adoption_links_without_deleting_adopted_canonical_content() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    let project = machine.working_directory().join("adopt-project");
    let canonical = project.join(".agents/skills/adopted-skill");
    fs::create_dir_all(&canonical).unwrap();
    fs::write(
        canonical.join("SKILL.md"),
        "---\nname: adopted-skill\ndescription: adopted fixture\n---\nbody\n",
    )
    .unwrap();
    let project_text = project.to_str().unwrap();

    let adopt = run(
        &machine,
        &[
            "adopt",
            "--from",
            "claude",
            "--project",
            project_text,
            "--json",
        ],
    );
    assert_code(&adopt, 0);
    assert!(
        fs::read_to_string(config_root(&machine).join("inventory.toml"))
            .unwrap()
            .contains("skill:adopted-skill")
    );
    assert!(!config_root(&machine).join("state.json").exists());

    let sync = run(
        &machine,
        &[
            "sync",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&sync, 0);
    let link = project.join(".claude/skills/adopted-skill");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let repeat = run(
        &machine,
        &[
            "sync",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "adopted-skill",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    assert!(!link.exists());
    assert!(canonical.join("SKILL.md").is_file());
}

#[test]
fn malformed_unmanaged_project_skill_is_reported_without_adoption_or_mutation() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    let project = machine.working_directory().join("malformed-project");
    let canonical = project.join(".agents/skills/malformed-skill");
    fs::create_dir_all(&canonical).unwrap();
    fs::write(
        canonical.join("SKILL.md"),
        "---\nname: malformed-skill\n---\nbody\n",
    )
    .unwrap();
    let project_text = project.to_str().unwrap();
    let status = run(
        &machine,
        &[
            "status",
            "--project",
            project_text,
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&status, 2);
    let value = json(&status);
    assert!(
        value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "skill.format.invalid")
    );
    let unmanaged = value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| {
            resource["id"].as_str().is_some_and(|id| {
                id == format!("skill:malformed-skill [project:{project_text}]:unmanaged")
            })
        })
        .unwrap();
    assert_eq!(unmanaged["fields"]["adoptable"], false);
    assert!(!config_root(&machine).join("inventory.toml").exists());
}

#[test]
fn project_skill_content_update_requires_all_desired_targets() {
    let machine = machine();
    let source = machine.home().join("shared-project-source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: shared-project\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let project = machine.working_directory().join("shared-project");
    fs::create_dir_all(&project).unwrap();
    let source_text = source.to_str().unwrap();
    let project_text = project.to_str().unwrap();
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "shared-project",
            "--project",
            project_text,
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&install, 0);
    fs::write(
        source.join("SKILL.md"),
        "---\nname: shared-project\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    let blocked = run(
        &machine,
        &[
            "skill",
            "update",
            "shared-project",
            "--project",
            project_text,
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&blocked, 2);
    assert_eq!(json(&blocked)["summary"]["changed"], false);
    assert_eq!(
        fs::read_to_string(project.join(".agents/skills/shared-project/SKILL.md")).unwrap(),
        "---\nname: shared-project\ndescription: v1\n---\nv1\n"
    );
    assert!(
        json(&blocked)["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "project_skill_shared_content_requires_all_targets")
    );

    let accepted = run(
        &machine,
        &[
            "skill",
            "update",
            "shared-project",
            "--project",
            project_text,
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&accepted, 0);
    assert_eq!(json(&accepted)["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(project.join(".agents/skills/shared-project/SKILL.md")).unwrap(),
        "---\nname: shared-project\ndescription: v2\n---\nv2\n"
    );
}

#[test]
fn skill_install_requires_generic_yes_for_loadable_partial_frontmatter() {
    let machine = machine();
    let source = machine.home().join("partial-source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: partial\ndescription: loadable but unterminated\nbody\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source = source.to_str().unwrap();

    let blocked = run(
        &machine,
        &["skill", "install", source, "--target", "codex", "--json"],
    );
    assert_code(&blocked, 2);
    let blocked_value = json(&blocked);
    assert_eq!(blocked_value["result"], "attention_required");
    assert!(
        blocked_value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| { warning["code"] == "partial_operation_requires_acknowledgment" })
    );
    assert!(
        !machine
            .home()
            .join(".agents/skills/partial-source")
            .exists()
    );

    let accepted = run(
        &machine,
        &[
            "skill", "install", source, "--target", "codex", "--yes", "--json",
        ],
    );
    assert_code(&accepted, 0);
    assert_eq!(json(&accepted)["summary"]["changed"], true);
    assert!(
        machine
            .home()
            .join(".agents/skills/partial-source/SKILL.md")
            .is_file()
    );
}

#[test]
fn claude_only_skill_install_keeps_canonical_and_harness_projection() {
    let machine = machine();
    let source = machine.home().join("claude-only-skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: claude-only\ndescription: test\n---\nbody\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source_text = source.to_str().unwrap();

    let install = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "claude-only",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_eq!(json(&install)["result"], "completed");
    assert!(
        machine
            .home()
            .join(".agents/skills/claude-only/SKILL.md")
            .is_file(),
        "{}",
        stdout(&install)
    );
    assert!(
        machine
            .home()
            .join(".claude/skills/claude-only/SKILL.md")
            .is_file()
    );

    let repeat = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "claude-only",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    fs::remove_dir_all(machine.home().join(".agents/skills/claude-only")).unwrap();
    let restore = run(
        &machine,
        &[
            "skill",
            "install",
            source_text,
            "--name",
            "claude-only",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&restore, 0);
    assert_eq!(json(&restore)["summary"]["changed"], true);
    assert!(
        machine
            .home()
            .join(".agents/skills/claude-only/SKILL.md")
            .is_file()
    );

    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "claude-only",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    assert_eq!(json(&remove)["result"], "completed");
    assert!(!machine.home().join(".agents/skills/claude-only").exists());
    assert!(!machine.home().join(".claude/skills/claude-only").exists());
}

#[test]
fn git_skill_install_clones_a_bounded_source_and_records_the_commit() {
    let machine = machine();
    let repository = machine.home().join("git-skill-source");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: git-demo\ndescription: git skill\n---\nbody\n",
    )
    .unwrap();
    let init = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(init.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            "SKILL.md",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source = format!("file://{}", repository.to_str().unwrap());
    let output = run(
        &machine,
        &["skill", "install", &source, "--target", "codex", "--json"],
    );
    assert_code(&output, 0);
    assert_eq!(json(&output)["result"], "completed");
    assert!(
        machine
            .home()
            .join(".agents/skills/git-skill-source/SKILL.md")
            .is_file(),
        "{}",
        stdout(&output)
    );
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("installed_revision"));
    assert!(state.contains("git_commit"));

    fs::write(
        repository.join("SKILL.md"),
        "---\nname: git-demo\ndescription: git skill v2\n---\nupdated\n",
    )
    .unwrap();
    let add = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            "SKILL.md",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(add.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());
    let new_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repository)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "git-skill-source",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["summary"]["changed"], true);
    let updated_state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(updated_state.contains(&new_sha));

    let update_all = run(
        &machine,
        &["skill", "update", "--target", "codex", "--json"],
    );
    assert_code(&update_all, 0);
    assert_eq!(json(&update_all)["result"], "completed");
    assert_eq!(json(&update_all)["summary"]["changed"], false);
    assert_eq!(json(&update_all)["summary"]["operations"], 1);
}

#[test]
fn git_skill_subdirectory_is_reused_by_unnamed_update() {
    let machine = machine();
    let repository = machine.home().join("nested-repository");
    fs::create_dir_all(repository.join("skills/demo")).unwrap();
    fs::write(
        repository.join("skills/demo/SKILL.md"),
        "---\nname: subdir-demo\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    let init = Command::new("git")
        .args(["init", "--quiet", "--initial-branch", "main"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(init.status.success());
    for args in [
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ][..],
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ][..],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(result.status.success());
    }
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source = format!("file://{}", repository.to_str().unwrap());
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            &source,
            "--path",
            "skills/demo",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_eq!(json(&install)["result"], "completed");

    fs::write(
        repository.join("skills/demo/SKILL.md"),
        "---\nname: subdir-demo\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    let add = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(add.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());

    let update = run(
        &machine,
        &["skill", "update", "--target", "codex", "--json"],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/demo/SKILL.md")).unwrap(),
        "---\nname: subdir-demo\ndescription: v2\n---\nv2\n"
    );
}

#[test]
fn explicitly_named_git_skill_update_preserves_the_managed_name() {
    let machine = machine();
    let repository = machine.home().join("source-skill");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: example-skill\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    for args in [
        &["init", "--quiet"][..],
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ][..],
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ][..],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(result.status.success());
    }
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source = format!("file://{}", repository.to_str().unwrap());

    let local = run(
        &machine,
        &[
            "skill",
            "install",
            repository.to_str().unwrap(),
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&local, 0);
    let named = run(
        &machine,
        &[
            "skill",
            "install",
            &source,
            "--name",
            "example-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&named, 0);

    fs::write(
        repository.join("SKILL.md"),
        "---\nname: example-skill\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());

    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "example-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["result"], "completed");
    assert_eq!(json(&update)["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/example-skill/SKILL.md")).unwrap(),
        "---\nname: example-skill\ndescription: v2\n---\nv2\n"
    );
}

#[test]
fn targeted_skill_remove_preserves_unselected_target_inventory() {
    let machine = machine();
    let source = machine.home().join("targeted-skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: targeted-skill\ndescription: test\n---\nbody\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source_text = source.to_str().unwrap();
    let install = run(
        &machine,
        &["skill", "install", source_text, "--target", "all", "--json"],
    );
    assert_code(&install, 0);

    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "targeted-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove, 0);
    assert!(
        !machine
            .home()
            .join(".agents/skills/targeted-skill")
            .exists()
    );
    assert!(
        machine
            .home()
            .join(".claude/skills/targeted-skill")
            .exists()
    );
    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory.contains("skill:targeted-skill"));
    assert!(inventory.contains("targets = [\"claude\"]"));
}

#[test]
fn targeted_skill_update_preserves_unselected_target_and_native_ids() {
    let machine = machine();
    let source = machine.home().join("targeted-update");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: targeted-update\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source_text = source.to_str().unwrap();
    let install = run(
        &machine,
        &["skill", "install", source_text, "--target", "all", "--json"],
    );
    assert_code(&install, 0);
    fs::write(
        source.join("SKILL.md"),
        "---\nname: targeted-update\ndescription: v2\n---\nv2\n",
    )
    .unwrap();

    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "targeted-update",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&update, 0);
    assert_eq!(json(&update)["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(
            machine
                .home()
                .join(".agents/skills/targeted-update/SKILL.md")
        )
        .unwrap(),
        "---\nname: targeted-update\ndescription: v2\n---\nv2\n"
    );
    assert_eq!(
        fs::read_to_string(
            machine
                .home()
                .join(".claude/skills/targeted-update/SKILL.md")
        )
        .unwrap(),
        "---\nname: targeted-update\ndescription: v1\n---\nv1\n"
    );
    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory.contains("skill:targeted-update"));
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(config_root(&machine).join("state.json")).unwrap())
            .unwrap();
    let resource = state["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["key"]["id"] == "skill:targeted-update")
        .unwrap();
    let binding = |harness: &str| {
        resource["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|target| target["target"] == harness)
            .unwrap()["binding"]
            .clone()
    };
    let codex = binding("codex");
    let claude = binding("claude");
    assert_eq!(claude["native_id"], "targeted-update");
    assert_eq!(codex["native_id"], "targeted-update");
    assert_ne!(codex["fingerprint"], claude["fingerprint"]);
}

#[test]
fn non_default_git_ref_resolves_and_same_tree_commit_advances_sha() {
    let machine = machine();
    let repository = machine.home().join("ref-skill");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: ref-skill\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    for args in [
        &["init", "--quiet", "--initial-branch", "main"][..],
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ][..],
        &[
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ][..],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(result.status.success());
    }
    let branch = Command::new("git")
        .args(["branch", "feature"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(branch.status.success());
    let checkout_feature = Command::new("git")
        .args(["switch", "feature"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(checkout_feature.status.success());
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source = format!("file://{}", repository.to_str().unwrap());

    let install = run(
        &machine,
        &[
            "skill", "install", &source, "--ref", "feature", "--target", "codex", "--json",
        ],
    );
    assert_code(&install, 0);
    let state_before = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    let old_sha = state_before
        .split("\"value\": \"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .unwrap()
        .to_owned();

    let empty = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--allow-empty",
            "--quiet",
            "-m",
            "same-tree",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(empty.status.success());
    let new_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repository)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    assert_ne!(old_sha, new_sha);

    let update = run(
        &machine,
        &[
            "skill",
            "update",
            "ref-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&update, 0);
    let value = json(&update);
    assert_eq!(value["summary"]["changed"], true);
    assert_eq!(value["summary"]["old_revision"], format!("git:{old_sha}"));
    assert_eq!(value["summary"]["new_revision"], format!("git:{new_sha}"));
    let state_after = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state_after.contains(&new_sha));
}

#[test]
fn skill_remove_blocks_all_targets_when_one_target_is_drifted() {
    let machine = machine();
    let source = machine.home().join("drifted-skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: drifted-skill\ndescription: test\n---\nbody\n",
    )
    .unwrap();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let source_text = source.to_str().unwrap();
    let install = run(
        &machine,
        &["skill", "install", source_text, "--target", "all", "--json"],
    );
    assert_code(&install, 0);
    let inventory_before =
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    fs::write(
        machine.home().join(".agents/skills/drifted-skill/SKILL.md"),
        "---\nname: drifted-skill\ndescription: externally changed\n---\nchanged\n",
    )
    .unwrap();

    let remove = run(
        &machine,
        &[
            "skill",
            "remove",
            "drifted-skill",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&remove, 2);
    let value = json(&remove);
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["summary"]["changed"], false);
    assert_eq!(value["summary"]["operations"], 0);
    assert!(
        value["warnings"].as_array().unwrap().iter().any(|warning| {
            warning["code"] == "skill_destination_drifted_requires_acknowledgment"
        })
    );
    assert!(machine.home().join(".agents/skills/drifted-skill").exists());
    assert!(machine.home().join(".claude/skills/drifted-skill").exists());
    assert_eq!(
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap(),
        inventory_before
    );
}

#[test]
fn instruction_setup_creates_canonical_global_file_and_bridges() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);

    let output = run(&machine, &["instructions", "setup", "--json"]);
    assert_code(&output, 0);
    assert_eq!(json(&output)["result"], "completed");
    assert!(machine.home().join("AGENTS.md").is_file());
    assert_eq!(
        fs::read_link(machine.home().join(".codex/AGENTS.md")).unwrap(),
        PathBuf::from("../AGENTS.md")
    );
    assert_eq!(
        fs::read_link(machine.home().join(".claude/CLAUDE.md")).unwrap(),
        PathBuf::from("../AGENTS.md")
    );
    assert!(
        fs::read_to_string(config_root(&machine).join("inventory.toml"))
            .unwrap()
            .contains("instructions:global")
    );
    assert!(
        fs::read_to_string(config_root(&machine).join("state.json"))
            .unwrap()
            .contains("instructions:")
    );

    let status = run(&machine, &["instructions", "status", "--json"]);
    assert_code(&status, 0);
    let status_value = json(&status);
    assert_eq!(status_value["result"], "completed");
    assert!(
        status_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["status"] == "managed")
    );

    let repeat = run(&machine, &["instructions", "setup", "--json"]);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    fs::remove_file(machine.home().join(".claude/CLAUDE.md")).unwrap();
    fs::write(machine.home().join(".claude/CLAUDE.md"), "legacy bridge\n").unwrap();
    let repair = run(&machine, &["instructions", "repair", "--yes", "--json"]);
    assert_code(&repair, 0);
    let repair_value = json(&repair);
    assert_eq!(repair_value["result"], "completed");
    assert_eq!(repair_value["summary"]["changed"], true);
    let backup = repair_value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["status"] == "preserved")
        .and_then(|entry| entry["fields"]["path"].as_str())
        .expect("successful repair discloses its recoverable backup");
    assert_eq!(fs::read_to_string(backup).unwrap(), "legacy bridge\n");
    assert!(
        fs::symlink_metadata(machine.home().join(".claude/CLAUDE.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let backup_root = config_root(&machine).join("managed/backups/instructions");
    let backup_count = || fs::read_dir(&backup_root).unwrap().count();
    assert_eq!(backup_count(), 1);
    let repeat_repair = run(&machine, &["instructions", "repair", "--yes", "--json"]);
    assert_code(&repeat_repair, 0);
    assert_eq!(json(&repeat_repair)["summary"]["changed"], false);
    assert_eq!(backup_count(), 1);

    let plain_repeat = run(&machine, &["instructions", "repair", "--yes"]);
    assert_code(&plain_repeat, 0);
    assert!(stdout(&plain_repeat).contains("Result: completed"));
    assert_eq!(backup_count(), 1);
}

#[test]
fn reconciliation_repairs_instruction_bridge_drift_with_target_and_scope_boundaries() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".agents/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/skills")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/skills")).unwrap();

    let setup = run(&machine, &["instructions", "setup", "--json"]);
    assert_code(&setup, 0);

    let codex_bridge = machine.home().join(".codex/AGENTS.md");
    let claude_bridge = machine.home().join(".claude/CLAUDE.md");
    fs::remove_file(&codex_bridge).unwrap();
    fs::write(&codex_bridge, b"unmanaged codex content\n").unwrap();

    let plan = run(&machine, &["plan", "--target", "codex", "--json"]);
    assert_code(&plan, 2);
    let plan_value = json(&plan);
    assert!(
        plan_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operation| operation["status"] == "blocked")
    );
    assert!(
        plan_value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "instruction_bridge_conflict")
    );

    let blocked = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&blocked, 2);
    assert_eq!(json(&blocked)["summary"]["changed"], false);
    assert_eq!(
        fs::read(&codex_bridge).unwrap(),
        b"unmanaged codex content\n"
    );
    assert!(
        fs::read_link(&claude_bridge).is_ok(),
        "targeted Codex sync must not alter the Claude bridge"
    );

    let repaired = run(&machine, &["sync", "--target", "codex", "--yes", "--json"]);
    assert_code(&repaired, 0);
    let repaired_value = json(&repaired);
    assert_eq!(repaired_value["result"], "completed");
    assert_eq!(repaired_value["summary"]["changed"], true);
    assert_eq!(
        fs::read_link(&codex_bridge).unwrap(),
        PathBuf::from("../AGENTS.md")
    );

    let repeat = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);

    let project = machine.working_directory().join("project");
    fs::create_dir_all(&project).unwrap();
    let project_setup = run_in(
        &machine,
        &project,
        &["instructions", "setup", "--project", "--json"],
    );
    assert_code(&project_setup, 0);
    let project_claude = project.join("CLAUDE.md");
    fs::remove_file(&project_claude).unwrap();
    fs::write(&project_claude, b"project drift\n").unwrap();

    let project_plan = run_in(
        &machine,
        &project,
        &["plan", "--project", "--target", "claude", "--json"],
    );
    assert_code(&project_plan, 2);
    assert!(
        json(&project_plan)["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operation| operation["status"] == "blocked")
    );
    let project_sync = run_in(
        &machine,
        &project,
        &["sync", "--project", "--target", "claude", "--yes", "--json"],
    );
    assert_code(&project_sync, 0);
    let project_sync_value = json(&project_sync);
    assert_eq!(project_sync_value["result"], "completed");
    assert_eq!(project_sync_value["summary"]["changed"], true);
    assert_eq!(
        fs::read_link(&project_claude).unwrap(),
        PathBuf::from("AGENTS.md")
    );
}

#[test]
fn reconciliation_plan_and_sync_preserve_nested_project_claude_bridge() {
    for (mode, expected_bytes) in [
        ("symlink", None),
        ("import", Some(b"@../AGENTS.md\n".as_slice())),
    ] {
        let machine = machine();
        let fixture = fake_harness(&machine, &FakeHarnessProfile::claude());
        let mut config = native_config(fixture.executable(), fixture.executable());
        config = config.replace(
            "claude_mode = \"symlink\"",
            &format!("claude_mode = \"{mode}\""),
        );
        write_owned(&machine, "config.toml", &config);

        let project = machine.working_directory().to_owned();
        fs::write(project.join("AGENTS.md"), b"canonical\n").unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        match expected_bytes {
            None => std::os::unix::fs::symlink("../AGENTS.md", project.join(".claude/CLAUDE.md"))
                .unwrap(),
            Some(contents) => fs::write(project.join(".claude/CLAUDE.md"), contents).unwrap(),
        }

        let setup = run(&machine, &["instructions", "setup", "--project", "--json"]);
        assert_code(&setup, 0);
        assert_eq!(json(&setup)["summary"]["changed"], false);
        assert!(!project.join("CLAUDE.md").exists());

        let plan = run(
            &machine,
            &["plan", "--project", "--target", "claude", "--json"],
        );
        assert_code(&plan, 2);
        let plan_value = json(&plan);
        let operation = plan_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|operation| operation["status"] == "no_change")
            .expect("nested managed bridge is represented as no_change");
        assert_eq!(
            operation["fields"]["path"],
            project.join(".claude/CLAUDE.md").to_str().unwrap()
        );

        let sync = run(
            &machine,
            &["sync", "--project", "--target", "claude", "--json"],
        );
        assert_code(&sync, 0);
        assert_eq!(json(&sync)["summary"]["changed"], false);

        let repeat = run(
            &machine,
            &["sync", "--project", "--target", "claude", "--json"],
        );
        assert_code(&repeat, 0);
        assert_eq!(json(&repeat)["summary"]["changed"], false);
        match expected_bytes {
            None => assert_eq!(
                fs::read_link(project.join(".claude/CLAUDE.md")).unwrap(),
                PathBuf::from("../AGENTS.md")
            ),
            Some(contents) => assert_eq!(
                fs::read(project.join(".claude/CLAUDE.md")).unwrap(),
                contents
            ),
        }
    }
}

#[test]
fn daemon_run_accepts_json_output() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);

    let output = run(&machine, &["daemon", "run", "--json"]);

    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["command"], "daemon run");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["summary"]["pending_operations"], 0);
}

#[test]
fn daemon_enable_is_idempotent_in_an_isolated_config_root() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);

    let first = run(&machine, &["daemon", "enable", "--json"]);
    assert!(first.status.code() == Some(0) || first.status.code() == Some(2));
    let first_value = json(&first);
    #[cfg(target_os = "macos")]
    let root = machine.home().join("Library/LaunchAgents");
    #[cfg(not(target_os = "macos"))]
    let root = machine.configuration_home().join("systemd/user");
    #[cfg(target_os = "macos")]
    let service = root.join("com.skilltap.daemon.plist");
    #[cfg(not(target_os = "macos"))]
    let service = root.join("skilltap-update.service");
    #[cfg(not(target_os = "macos"))]
    let timer = root.join("skilltap-update.timer");
    assert!(service.is_file());
    #[cfg(not(target_os = "macos"))]
    assert!(timer.is_file());
    let service_bytes = fs::read(&service).unwrap();
    #[cfg(not(target_os = "macos"))]
    let timer_bytes = fs::read(&timer).unwrap();
    let service_mtime = fs::metadata(&service).unwrap().modified().unwrap();
    #[cfg(not(target_os = "macos"))]
    let timer_mtime = fs::metadata(&timer).unwrap().modified().unwrap();

    let second = run(&machine, &["daemon", "enable", "--json"]);
    assert!(second.status.code() == Some(0) || second.status.code() == Some(2));
    let second_value = json(&second);
    assert_eq!(second_value["summary"]["changed"], false);
    assert_eq!(first_value["command"], "daemon enable");
    assert_eq!(fs::read(&service).unwrap(), service_bytes);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(fs::read(&timer).unwrap(), timer_bytes);
    assert_eq!(
        fs::metadata(&service).unwrap().modified().unwrap(),
        service_mtime
    );
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        fs::metadata(&timer).unwrap().modified().unwrap(),
        timer_mtime
    );
}

#[test]
fn daemon_status_surfaces_malformed_state_instead_of_reporting_disabled() {
    let machine = machine();
    write_owned(&machine, "state.json", "not valid state");
    let output = run(&machine, &["daemon", "status", "--json"]);
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["warnings"][0]["code"], "daemon_state_unavailable");
}

#[test]
fn bare_invocation_prints_concise_help_and_fails_as_input() {
    let machine = machine();
    let output = run(&machine, &[]);

    assert_code(&output, 1);
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("Usage: skilltap <COMMAND>"));
    assert!(stderr(&output).contains("Code: missing_command"));
    assert!(!stderr(&output).contains("\u{1b}["));
}

#[test]
fn first_use_status_is_read_only_and_reports_no_enabled_harnesses() {
    let machine = machine();
    assert!(!config_root(&machine).exists());

    let output = machine
        .run_with_path(
            &binary(),
            &["status", "--json"],
            machine.working_directory(),
        )
        .expect("run first-use status with an isolated executable path");
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["command"], "status");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["scope"]["kind"], "global");
    assert_eq!(value["summary"]["targets"], 0);
    assert_eq!(value["errors"][0]["code"], "no_enabled_harnesses");
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| { resource["id"] == "codex" && resource["status"] == "unreachable" })
    );
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| { resource["id"] == "claude" && resource["status"] == "unreachable" })
    );
    assert!(!config_root(&machine).exists());
}

#[test]
fn instruction_status_reports_duplicate_project_claude_bridges() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    fs::write(
        machine.working_directory().join("AGENTS.md"),
        b"canonical\n",
    )
    .unwrap();
    fs::write(machine.working_directory().join("CLAUDE.md"), b"root\n").unwrap();
    fs::create_dir_all(machine.working_directory().join(".claude")).unwrap();
    fs::write(
        machine.working_directory().join(".claude/CLAUDE.md"),
        b"nested\n",
    )
    .unwrap();

    let output = run(&machine, &["instructions", "status", "--project", "--json"]);
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["result"], "attention_required");
    assert!(
        value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "instruction_duplicate_claude_bridge")
    );
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["status"] == "duplicate")
    );
}

#[test]
fn instruction_setup_preserves_existing_nested_project_claude_bridge() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    fs::write(
        machine.working_directory().join("AGENTS.md"),
        b"canonical\n",
    )
    .unwrap();
    fs::create_dir_all(machine.working_directory().join(".claude")).unwrap();
    std::os::unix::fs::symlink(
        "../AGENTS.md",
        machine.working_directory().join(".claude/CLAUDE.md"),
    )
    .unwrap();

    let output = run(&machine, &["instructions", "setup", "--project", "--json"]);
    assert_code(&output, 0);
    assert_eq!(json(&output)["result"], "completed");
    assert!(!machine.working_directory().join("CLAUDE.md").exists());
    assert_eq!(
        fs::read_link(machine.working_directory().join(".claude/CLAUDE.md")).unwrap(),
        PathBuf::from("../AGENTS.md")
    );
}

#[test]
fn instruction_repair_consolidates_duplicate_project_claude_bridges() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    fs::write(
        machine.working_directory().join("AGENTS.md"),
        b"canonical\n",
    )
    .unwrap();
    fs::write(
        machine.working_directory().join("CLAUDE.md"),
        b"root drift\n",
    )
    .unwrap();
    fs::create_dir_all(machine.working_directory().join(".claude")).unwrap();
    fs::write(
        machine.working_directory().join(".claude/CLAUDE.md"),
        b"nested drift\n",
    )
    .unwrap();

    let output = run(
        &machine,
        &["instructions", "repair", "--project", "--yes", "--json"],
    );
    assert_code(&output, 0);
    assert_eq!(json(&output)["result"], "completed");
    assert_eq!(
        fs::read_link(machine.working_directory().join("CLAUDE.md")).unwrap(),
        PathBuf::from("AGENTS.md")
    );
    assert!(
        !machine
            .working_directory()
            .join(".claude/CLAUDE.md")
            .exists()
    );
    let backups = config_root(&machine).join("managed/backups/instructions");
    assert!(backups.is_dir());
    assert!(fs::read_dir(backups).unwrap().next().is_some());
}

#[test]
fn instruction_repair_does_not_remove_broken_duplicate_bridge_entries() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    fs::write(
        machine.working_directory().join("AGENTS.md"),
        b"canonical\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("AGENTS.md", machine.working_directory().join("CLAUDE.md")).unwrap();
    fs::create_dir_all(machine.working_directory().join(".claude/CLAUDE.md")).unwrap();

    let output = run(
        &machine,
        &["instructions", "repair", "--project", "--yes", "--json"],
    );
    assert_code(&output, 2);
    assert_eq!(json(&output)["result"], "attention_required");
    assert!(
        machine
            .working_directory()
            .join(".claude/CLAUDE.md")
            .is_dir()
    );
}

#[test]
fn status_resolves_current_explicit_and_all_scopes_independently_from_targets() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    let project = machine.working_directory().join("project");
    let nested = project.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&project)
        .output()
        .expect("initialize Git fixture");
    assert!(git.status.success(), "{}", stderr(&git));

    let global = run(&machine, &["status", "--target", "codex", "--json"]);
    assert_code(&global, 2);
    let value = json(&global);
    assert_eq!(value["scope"]["kind"], "global");
    assert!(value["resources"].as_array().unwrap().len() >= 4);
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == "codex")
    );

    let current = run_in(
        &machine,
        &nested,
        &["status", "--project", "--target", "claude", "--json"],
    );
    assert_code(&current, 0);
    let value = json(&current);
    assert_eq!(value["scope"]["kind"], "project");
    assert_eq!(value["scope"]["path"], project.to_str().unwrap());
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == "claude")
    );

    let explicit = run(
        &machine,
        &[
            "status",
            "--project",
            nested.to_str().unwrap(),
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&explicit, 0);
    let value = json(&explicit);
    assert_eq!(value["scope"]["path"], project.to_str().unwrap());
    assert_eq!(value["summary"]["targets"], 2);

    write_owned(
        &machine,
        "inventory.toml",
        &format!(
            "schema = 1\nprojects = [{}]\nresources = []\n",
            toml_string(&project)
        ),
    );
    let all = run(
        &machine,
        &["status", "--all-scopes", "--target", "all", "--json"],
    );
    assert_code(&all, 2);
    let value = json(&all);
    assert_eq!(value["scope"]["kind"], "all");
    assert_eq!(value["summary"]["scopes"], 2);
}

#[test]
fn status_preserves_successful_sibling_observation_and_never_mutates_native_trees() {
    let machine = machine();
    let codex = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let claude = FakeNativeProcess::new(FakeNativeMode::MalformedJson).unwrap();
    let codex_home = machine.home().join(".codex");
    let claude_home = machine.home().join(".claude");
    fs::create_dir_all(codex_home.join("skills/example")).unwrap();
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
        codex_home.join("config.toml"),
        "[features]\nplugins = true\n",
    )
    .unwrap();
    fs::write(
        codex_home.join("skills/example/SKILL.md"),
        "---\nname: example\n---\n",
    )
    .unwrap();
    fs::write(
        claude_home.join("settings.json"),
        "{\"enabledPlugins\": []}\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("config.toml", codex_home.join("config-link"))
        .expect("create native symlink");
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );

    let before_codex = snapshot_native_tree(&codex_home);
    let before_claude = snapshot_native_tree(&claude_home);
    let output = run(&machine, &["status", "--target", "all", "--json"]);
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["summary"]["targets"], 2);
    assert_eq!(value["summary"]["observed_targets"], 1);
    assert_eq!(value["summary"]["failed_targets"], 1);
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == "codex:global" && entry["status"] == "observed" })
    );
    assert!(
        value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["id"] == "claude:global" && entry["status"] == "unreachable" })
    );
    assert!(value["warnings"].as_array().unwrap().iter().any(|warning| {
        warning["code"] == "native_version_invalid" && warning["context"]["harness"] == "claude"
    }));
    assert!(
        value["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["code"] == "inspect_harness_version"
                    && action["command"] == format!("{} --version", claude.executable().display())
            }),
        "unexpected adoption diagnostics: {value}"
    );

    let plain = run(&machine, &["status", "--target", "codex"]);
    assert_code(&plain, 0);
    assert!(plain.stderr.is_empty());
    assert!(stdout(&plain).contains("Result: completed"));
    assert!(stdout(&plain).contains("codex:global  observed"));

    assert_eq!(snapshot_native_tree(&codex_home), before_codex);
    assert_eq!(snapshot_native_tree(&claude_home), before_claude);
}

#[test]
fn adopt_publishes_inventory_and_is_idempotent_without_native_mutation() {
    let machine = machine();
    let codex = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let codex_home = machine.home().join(".codex");
    fs::create_dir_all(codex_home.join("skills/example")).unwrap();
    fs::write(
        codex_home.join("skills/example/SKILL.md"),
        "---\nname: example\ndescription: Example\n---\n",
    )
    .unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), codex.executable()),
    );
    let before = snapshot_native_tree(&codex_home);

    let first = run(&machine, &["adopt", "--from", "codex", "--json"]);
    assert_code(&first, 0);
    let first_value = json(&first);
    assert_eq!(first_value["command"], "adopt");
    assert_eq!(first_value["result"], "completed");
    assert!(first_value["summary"]["adopted"].as_u64().unwrap() > 0);
    let inventory = config_root(&machine).join("inventory.toml");
    assert!(inventory.is_file());
    let first_inventory = fs::read(&inventory).unwrap();

    let second = run(&machine, &["adopt", "--from", "codex", "--json"]);
    assert_code(&second, 0);
    let second_value = json(&second);
    assert_eq!(second_value["result"], "completed");
    assert!(second_value["summary"]["already_managed"].as_u64().unwrap() > 0);
    assert_eq!(fs::read(&inventory).unwrap(), first_inventory);
    assert_eq!(snapshot_native_tree(&codex_home), before);
}

#[test]
fn adopt_reports_partial_sibling_and_still_publishes_healthy_candidates() {
    let machine = machine();
    let codex = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let claude = FakeNativeProcess::new(FakeNativeMode::MalformedJson).unwrap();
    let codex_home = machine.home().join(".codex");
    fs::create_dir_all(codex_home.join("skills/example")).unwrap();
    fs::write(
        codex_home.join("skills/example/SKILL.md"),
        "---\nname: example\n---\n",
    )
    .unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );

    let output = run(&machine, &["adopt", "--from", "all", "--json"]);
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["result"], "attention_required");
    assert!(value["summary"]["adopted"].as_u64().unwrap() > 0);
    assert!(value["warnings"].as_array().unwrap().iter().any(|warning| {
        warning["code"] == "native_version_invalid" && warning["context"]["harness"] == "claude"
    }));
    assert!(
        value["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["code"] == "inspect_harness_version"
                    && action["command"] == format!("{} --version", claude.executable().display())
            }),
        "unexpected partial adoption diagnostics: {value}"
    );
    assert!(config_root(&machine).join("inventory.toml").is_file());
}

#[test]
fn adopt_project_and_all_scopes_preserve_project_inventory_scope() {
    let machine = machine();
    let codex = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let project = machine.working_directory().join("project");
    fs::create_dir_all(project.join(".agents/skills/example")).unwrap();
    fs::write(
        project.join(".agents/skills/example/SKILL.md"),
        "---\nname: project-example\n---\n",
    )
    .unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), codex.executable()),
    );

    let project_output = run(
        &machine,
        &[
            "adopt",
            "--from",
            "codex",
            "--project",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert_code(&project_output, 0);
    let project_value = json(&project_output);
    assert_eq!(project_value["scope"]["kind"], "project");
    assert!(project_value["summary"]["adopted"].as_u64().unwrap() > 0);
    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory.contains(project.to_str().unwrap()));

    let all = run(
        &machine,
        &["adopt", "--all-scopes", "--from", "codex", "--json"],
    );
    assert_code(&all, 2);
    let all_value = json(&all);
    assert_eq!(all_value["scope"]["kind"], "all");
    assert!(all_value["summary"]["already_managed"].as_u64().unwrap() > 0);
}

#[test]
fn native_plugin_and_marketplace_lifecycle_covers_both_harnesses_and_journal_repeats() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    for root in [
        machine.home().join(".codex"),
        machine.home().join(".claude"),
    ] {
        fs::create_dir_all(root.join("plugins")).unwrap();
        fs::create_dir_all(root.join("skills")).unwrap();
    }

    for target in ["codex", "claude"] {
        let add = run(
            &machine,
            &[
                "marketplace",
                "add",
                "https://example.invalid/team.git",
                "--name",
                "team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&add, 0);
        let add_value = json(&add);
        assert_eq!(add_value["result"], "completed", "target={target}");
        assert_eq!(add_value["summary"]["changed"], true, "target={target}");
        assert!(
            add_value["operations"]
                .as_array()
                .is_some_and(|operations| !operations.is_empty()),
            "target={target}, output={add_value}"
        );

        let add_repeat = run(
            &machine,
            &[
                "marketplace",
                "add",
                "https://example.invalid/team.git",
                "--name",
                "team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&add_repeat, 0);
        assert_eq!(json(&add_repeat)["summary"]["changed"], false);

        let marketplace_update = run(
            &machine,
            &[
                "marketplace",
                "update",
                "team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&marketplace_update, 0);
        assert_eq!(json(&marketplace_update)["result"], "completed");

        let plugin = run(
            &machine,
            &[
                "plugin",
                "install",
                "formatter@team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&plugin, 0);
        let plugin_value = json(&plugin);
        assert_eq!(plugin_value["result"], "completed", "target={target}");
        assert_eq!(plugin_value["summary"]["changed"], true, "target={target}");
        assert!(
            plugin_value["operations"]
                .as_array()
                .is_some_and(|operations| !operations.is_empty()),
            "target={target}, output={plugin_value}"
        );
        let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
        assert!(inventory.contains("plugin:formatter@team"));
        let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
        assert!(state.contains("formatter@team"));

        let plugin_repeat = run(
            &machine,
            &[
                "plugin",
                "install",
                "formatter@team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&plugin_repeat, 0);
        assert_eq!(json(&plugin_repeat)["summary"]["changed"], false);

        let plugin_update = run(
            &machine,
            &[
                "plugin",
                "update",
                "formatter@team",
                "--target",
                target,
                "--json",
            ],
        );
        if target == "codex" {
            assert_code(&plugin_update, 2);
            assert_eq!(json(&plugin_update)["result"], "attention_required");
        } else {
            assert_code(&plugin_update, 0);
            assert_eq!(json(&plugin_update)["result"], "completed");
        }

        let plugin_remove = run(
            &machine,
            &[
                "plugin",
                "remove",
                "formatter@team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&plugin_remove, 0);
        assert_eq!(json(&plugin_remove)["result"], "completed");

        let marketplace_remove = run(
            &machine,
            &[
                "marketplace",
                "remove",
                "team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&marketplace_remove, 0);
        assert_eq!(json(&marketplace_remove)["result"], "completed");
    }
}

#[test]
fn sequential_native_plugin_installs_widen_targets_and_preserve_bindings() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    for root in [machine.codex_home(), machine.claude_home()] {
        fs::create_dir_all(root.join("plugins")).unwrap();
        fs::create_dir_all(root.join("skills")).unwrap();
    }

    for target in ["codex", "claude"] {
        let install = run(
            &machine,
            &[
                "plugin",
                "install",
                "shared@team",
                "--target",
                target,
                "--json",
            ],
        );
        assert_code(&install, 0);
        assert_eq!(json(&install)["summary"]["changed"], true);
    }

    let inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    let shared = inventory
        .split("[[resources]]")
        .find(|section| section.contains("id = \"plugin:shared@team\""))
        .expect("shared desired resource exists");
    assert!(shared.contains("\"codex\"") && shared.contains("\"claude\""));

    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(config_root(&machine).join("state.json")).unwrap())
            .unwrap();
    let bindings = state["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["key"]["id"] == "plugin:shared@team")
        .and_then(|resource| resource["targets"].as_array())
        .expect("shared state has target bindings");
    assert!(
        bindings.iter().any(|binding| binding["target"] == "codex")
            && bindings.iter().any(|binding| binding["target"] == "claude")
    );

    let repeat = run(
        &machine,
        &[
            "plugin",
            "install",
            "shared@team",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
}

#[test]
fn native_mutations_keep_project_and_all_scope_boundaries() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    let project = machine.working_directory().join("project");
    fs::create_dir_all(&project).unwrap();
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        git.status.success(),
        "{}",
        String::from_utf8_lossy(&git.stderr)
    );

    let global = run(
        &machine,
        &[
            "plugin",
            "install",
            "global@team",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&global, 0);
    let project_install = run_in(
        &machine,
        &project,
        &[
            "plugin",
            "install",
            "project@team",
            "--project",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&project_install, 0);
    assert_eq!(json(&project_install)["scope"]["kind"], "project");
    assert_eq!(
        json(&project_install)["scope"]["path"],
        project.to_str().unwrap()
    );

    // Equal logical ids in global and project scopes must remain distinct
    // operations while an all-scopes request reconciles both instances.
    let same_global = run(
        &machine,
        &[
            "plugin",
            "install",
            "same@team",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&same_global, 0);
    let same_project = run_in(
        &machine,
        &project,
        &[
            "plugin",
            "install",
            "same@team",
            "--project",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&same_project, 0);
    fs::write(project.join("untouched.txt"), "keep me").unwrap();
    let native_before = snapshot_native_tree(&machine.home().join(".claude"));
    let state_before = fs::read(config_root(&machine).join("state.json")).unwrap();
    let same_remove = run(
        &machine,
        &[
            "plugin",
            "remove",
            "same@team",
            "--all-scopes",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&same_remove, 0);
    let same_inventory = fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(!same_inventory.contains("plugin:same@team"));
    assert!(same_inventory.contains("plugin:project@team"));
    let same_state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(!same_state.contains("plugin:same@team"));
    assert_ne!(same_state.as_bytes(), state_before.as_slice());
    assert_eq!(
        snapshot_native_tree(&machine.home().join(".claude")),
        native_before
    );
    assert_eq!(
        fs::read_to_string(project.join("untouched.txt")).unwrap(),
        "keep me"
    );

    let inventory_before =
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(inventory_before.contains("plugin:global@team"));
    assert!(inventory_before.contains("plugin:project@team"));
    let global_before = inventory_before.clone();

    let remove_global = run(
        &machine,
        &[
            "plugin",
            "remove",
            "global@team",
            "--all-scopes",
            "--target",
            "claude",
            "--json",
        ],
    );
    assert_code(&remove_global, 0);
    let after_global_remove =
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(!after_global_remove.contains("plugin:global@team"));
    assert!(after_global_remove.contains("plugin:project@team"));
    assert_ne!(after_global_remove, global_before);

    let codex_install = run(
        &machine,
        &[
            "plugin",
            "install",
            "shared@team",
            "--target",
            "all",
            "--json",
        ],
    );
    assert_code(&codex_install, 0);
    let before_target_remove =
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    let shared_section = before_target_remove
        .split("[[resources]]")
        .find(|section| section.contains("id = \"plugin:shared@team\""))
        .expect("shared resource is present in inventory");
    assert!(
        shared_section.contains("\"claude\"") && shared_section.contains("\"codex\""),
        "shared resource should retain both selected targets: {shared_section}"
    );

    let remove_codex = run(
        &machine,
        &[
            "plugin",
            "remove",
            "shared@team",
            "--all-scopes",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&remove_codex, 0);
    let after_target_remove =
        fs::read_to_string(config_root(&machine).join("inventory.toml")).unwrap();
    assert!(after_target_remove.contains("plugin:shared@team"));
    assert!(after_target_remove.contains("targets = [\"claude\"]"));
    assert!(after_target_remove.contains("plugin:project@team"));
}

#[test]
fn safe_update_cycle_reports_changed_git_revision_and_records_daemon_result() {
    let machine = machine();
    let repository = machine.home().join("daemon-skill-source");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: daemon-skill\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    for args in [
        vec!["init", "--quiet", "--initial-branch", "main"],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    write_owned(
        &machine,
        "config.toml",
        &ENABLED_CONFIG.replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    let source = format!("file://{}", repository.to_str().unwrap());
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            &source,
            "--name",
            "daemon-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    let first_daemon = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&first_daemon, 0);
    let first_value = json(&first_daemon);
    assert_eq!(first_value["result"], "completed");
    assert_eq!(first_value["summary"]["changed"], false);
    let first_state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(first_state.contains("\"daemon_run\""));
    assert!(first_state.contains("\"result\": \"completed\""));

    fs::write(
        repository.join("SKILL.md"),
        "---\nname: daemon-skill\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    let add = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(add.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());
    let status = run(&machine, &["status", "--target", "codex", "--json"]);
    assert_code(&status, 2);
    let status_value = json(&status);
    assert!(
        status_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"]
                .as_str()
                .unwrap_or_default()
                .starts_with("update:"))
    );

    let second_daemon = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&second_daemon, 0);
    let second_value = json(&second_daemon);
    assert_eq!(second_value["result"], "completed");
    assert_eq!(second_value["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/daemon-skill/SKILL.md")).unwrap(),
        "---\nname: daemon-skill\ndescription: v2\n---\nv2\n"
    );
    let second_state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(second_state.contains("\"daemon_run\""));
    assert!(second_state.contains("\"result\": \"completed\""));
}

#[test]
fn safe_update_policy_pins_drift_and_source_failures_remain_visible() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    let repository = machine.home().join("policy-skill-source");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: policy-skill\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    for args in [
        vec!["init", "--quiet", "--initial-branch", "main"],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    let source = format!("file://{}", repository.to_str().unwrap());
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            &source,
            "--name",
            "policy-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);

    fs::write(
        repository.join("SKILL.md"),
        "---\nname: policy-skill\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    let add = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(add.status.success());
    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(commit.status.success());

    let check_config = fs::read_to_string(config_root(&machine).join("config.toml"))
        .unwrap()
        .replace("mode = \"apply-safe\"", "mode = \"check\"");
    write_owned(&machine, "config.toml", &check_config);
    let check = run(&machine, &["status", "--target", "codex", "--json"]);
    assert_code(&check, 2);
    let check_value = json(&check);
    assert!(
        check_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["id"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("update:")
                    && entry["fields"]["reason"] == "check_only"
            })
    );

    let off_config = check_config.replace("mode = \"check\"", "mode = \"off\"");
    write_owned(&machine, "config.toml", &off_config);
    let off = run(&machine, &["status", "--target", "codex", "--json"]);
    assert_code(&off, 2);
    let off_value = json(&off);
    assert!(
        off_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["id"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("update:")
                    && entry["status"] == "up_to_date"
            }),
        "off status: {off_value}"
    );

    let apply_config = off_config.replace("mode = \"off\"", "mode = \"apply-safe\"");
    write_owned(&machine, "config.toml", &apply_config);
    fs::write(
        machine.home().join(".agents/skills/policy-skill/SKILL.md"),
        "---\nname: policy-skill\ndescription: local drift\n---\ndrift\n",
    )
    .unwrap();
    let drift = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&drift, 2);
    let drift_value = json(&drift);
    assert!(
        drift_value["summary"]["pending_operations"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        drift_value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| { warning["code"] == "skill_destination_drifted" })
    );
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/policy-skill/SKILL.md")).unwrap(),
        "---\nname: policy-skill\ndescription: local drift\n---\ndrift\n"
    );

    let pinned_repository = machine.home().join("pinned-skill-source");
    fs::create_dir_all(&pinned_repository).unwrap();
    fs::write(
        pinned_repository.join("SKILL.md"),
        "---\nname: pinned-skill\ndescription: pinned\n---\npinned\n",
    )
    .unwrap();
    for args in [
        vec!["init", "--quiet", "--initial-branch", "main"],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&pinned_repository)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let pinned_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&pinned_repository)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    let pinned_source = format!("file://{}", pinned_repository.to_str().unwrap());
    let pinned = run(
        &machine,
        &[
            "skill",
            "install",
            &pinned_source,
            "--name",
            "pinned-skill",
            "--ref",
            &pinned_sha,
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&pinned, 0);
    let pinned_status = run(&machine, &["status", "--target", "codex", "--json"]);
    assert_code(&pinned_status, 2);
    let pinned_value = json(&pinned_status);
    assert!(
        pinned_value["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["id"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("pinned-skill")
                    && entry["status"] == "blocked"
                    && entry["fields"]["reason"] == "resolution_failed"
            }),
        "pinned status: {pinned_value}"
    );

    fs::remove_dir_all(&repository).unwrap();
    fs::remove_dir_all(&pinned_repository).unwrap();
    let unavailable = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&unavailable, 2);
    let unavailable_value = json(&unavailable);
    assert!(
        unavailable_value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| {
                warning["code"] == "git_skill_source_unavailable"
                    || warning["code"] == "skill_source_unavailable"
            })
    );
    assert!(
        unavailable_value["summary"]["pending_operations"]
            .as_u64()
            .unwrap()
            >= 1
    );
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("\"daemon_run\""));
    assert!(state.contains("\"result\": \"pending\""));
}

#[cfg(target_os = "linux")]
#[test]
fn safe_update_lock_contention_records_pending_failure_and_recovers() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    let repository = machine.home().join("lock-skill-source");
    fs::create_dir_all(&repository).unwrap();
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: lock-skill\ndescription: v1\n---\nv1\n",
    )
    .unwrap();
    for args in [
        vec!["init", "--quiet", "--initial-branch", "main"],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let source = format!("file://{}", repository.to_str().unwrap());
    let install = run(
        &machine,
        &[
            "skill",
            "install",
            &source,
            "--name",
            "lock-skill",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    fs::write(
        repository.join("SKILL.md"),
        "---\nname: lock-skill\ndescription: v2\n---\nv2\n",
    )
    .unwrap();
    for args in [
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=skilltap-test",
            "-c",
            "user.email=skilltap@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "update",
        ],
    ] {
        let result = Command::new("git")
            .args(args)
            .current_dir(&repository)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let lock_path = config_root(&machine).join("skilltap.lock");
    let marker = machine.home().join("lock-held");
    let script = format!(
        "flock -n '{}' sh -c 'touch \"{}\"; sleep 1'",
        lock_path.to_str().unwrap(),
        marker.to_str().unwrap()
    );
    let mut holder = Command::new("sh").args(["-c", &script]).spawn().unwrap();
    for _ in 0..100 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(marker.exists(), "lock holder did not acquire the file lock");

    let blocked = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&blocked, 2);
    let blocked_value = json(&blocked);
    assert!(
        blocked_value["summary"]["pending_operations"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        blocked_value["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| { error["code"] == "configuration_locked" })
    );
    let _ = holder.wait();

    let recovered = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&recovered, 0);
    let recovered_value = json(&recovered);
    assert_eq!(recovered_value["result"], "completed");
    assert_eq!(recovered_value["summary"]["changed"], true);
    assert_eq!(
        fs::read_to_string(machine.home().join(".agents/skills/lock-skill/SKILL.md")).unwrap(),
        "---\nname: lock-skill\ndescription: v2\n---\nv2\n"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn daemon_service_failure_paths_preserve_unmanaged_and_nonregular_definitions() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
    let service_root = machine.configuration_home().join("systemd/user");
    fs::create_dir_all(&service_root).unwrap();
    let service = service_root.join("skilltap-update.service");
    let timer = service_root.join("skilltap-update.timer");

    let disable_empty = run(&machine, &["daemon", "disable", "--json"]);
    assert_code(&disable_empty, 0);
    assert_eq!(json(&disable_empty)["summary"]["changed"], false);

    fs::write(&service, "[Unit]\nDescription=unmanaged lookalike\n").unwrap();
    let conflict = run(&machine, &["daemon", "enable", "--json"]);
    assert_code(&conflict, 2);
    let conflict_value = json(&conflict);
    assert_eq!(
        conflict_value["warnings"][0]["code"],
        "daemon_definition_conflict"
    );
    assert_eq!(
        fs::read(&service).unwrap(),
        b"[Unit]\nDescription=unmanaged lookalike\n"
    );

    fs::remove_file(&service).unwrap();
    let malformed_definition = b"# skilltap-managed-v3\n[Unit]\nDescription=skilltap safe update cycle\n[Service]\nType=oneshot\nExecStart=/bin/skilltap daemon run\nExecStart=/bin/skilltap daemon run\n";
    fs::write(&service, malformed_definition).unwrap();
    let malformed = run(&machine, &["daemon", "enable", "--json"]);
    assert_code(&malformed, 2);
    assert_eq!(
        json(&malformed)["warnings"][0]["code"],
        "daemon_definition_malformed"
    );
    assert_eq!(fs::read(&service).unwrap(), malformed_definition);

    fs::remove_file(&service).unwrap();
    fs::create_dir(&timer).unwrap();
    let unreadable = run(&machine, &["daemon", "enable", "--json"]);
    assert_code(&unreadable, 2);
    let unreadable_value = json(&unreadable);
    assert_eq!(
        unreadable_value["warnings"][0]["code"],
        "daemon_definition_unreadable"
    );
    assert!(service.is_file() || !service.exists());
    assert!(timer.is_dir());

    fs::remove_dir(&timer).unwrap();
    let fake_manager_root = machine.home().join("fake-manager-bin");
    fs::create_dir_all(&fake_manager_root).unwrap();
    let manager = fake_manager_root.join("systemctl");
    fs::write(&manager, "#!/bin/sh\nexit 42\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&manager).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&manager, permissions).unwrap();
    }
    let manager_failure = machine
        .run_with_path(
            &binary(),
            &["daemon", "enable", "--json"],
            &fake_manager_root,
        )
        .unwrap();
    assert_code(&manager_failure, 2);
    let manager_value = json(&manager_failure);
    assert_eq!(
        manager_value["warnings"][0]["code"],
        "daemon_manager_unavailable"
    );
    assert!(service.is_file());
    assert!(timer.is_file());
}

#[test]
fn populated_plan_and_sync_apply_the_desired_inventory_resource() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();

    // Seed a valid desired inventory through the explicit lifecycle command,
    // then remove only apply provenance to simulate an unapplied desired state.
    let install = run(
        &machine,
        &[
            "plugin",
            "install",
            "formatter@team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    let inventory_path = config_root(&machine).join("inventory.toml");
    let inventory_before = fs::read(&inventory_path).unwrap();
    fs::remove_file(config_root(&machine).join("state.json")).unwrap();

    let plan = run(&machine, &["plan", "--target", "codex", "--json"]);
    assert_code(&plan, 2);
    let plan_value = json(&plan);
    assert_eq!(plan_value["summary"]["desired_resources"], 1);
    assert!(plan_value["summary"]["operations"].as_u64().unwrap() >= 1);
    assert!(
        plan_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                matches!(
                    entry["status"].as_str(),
                    Some("planned" | "pending" | "repair")
                )
            })
    );
    assert_eq!(fs::read(&inventory_path).unwrap(), inventory_before);
    assert!(!config_root(&machine).join("state.json").exists());

    let sync = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&sync, 0);
    let sync_value = json(&sync);
    assert_eq!(sync_value["result"], "completed");
    assert_eq!(sync_value["summary"]["changed"], true);
    assert!(sync_value["summary"]["operations"].as_u64().unwrap() >= 1);
    assert!(config_root(&machine).join("state.json").is_file());
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("formatter@team"));

    // The second reconciliation is idempotent and must not rewrite inventory.
    let repeat_inventory = fs::read(&inventory_path).unwrap();
    let repeat = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
    assert_eq!(fs::read(&inventory_path).unwrap(), repeat_inventory);

    // Keep the fixture binary alive through the whole test; lifecycle and
    // post-observation calls use this exact isolated executable.
    let _ = fixture;
}

#[test]
fn reconciliation_reobserves_missing_native_plugin_before_reusing_journal() {
    let machine = machine();
    let harness = machine.home().join("fake-codex");
    let marker = machine.home().join("native-plugin-present");
    let marker_literal = marker.to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        &harness,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf '%s' 'codex-cli 0.144.1'; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"list\" ]; then if [ -f '{marker}' ]; then printf '%s' '{{\"plugins\":[{{\"name\":\"formatter@team\"}}]}}'; else printf '%s' '{{\"plugins\":[]}}'; fi; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"add\" ]; then : > '{marker}'; exit 0; fi\nexit 0\n",
            marker = marker_literal
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&harness).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&harness, permissions).unwrap();
    }
    write_owned(
        &machine,
        "config.toml",
        &native_config(&harness, &harness).replace(
            "[harnesses.claude]\nenabled = true",
            "[harnesses.claude]\nenabled = false",
        ),
    );
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::write(&marker, b"present").unwrap();

    let install = run(
        &machine,
        &[
            "plugin",
            "install",
            "formatter@team",
            "--target",
            "codex",
            "--json",
        ],
    );
    assert_code(&install, 0);
    assert_eq!(json(&install)["summary"]["changed"], true);

    // The native plugin remains present even when skilltap's journal is
    // externally removed. Plan must show the provenance repair that sync will
    // perform instead of claiming a no-op from native presence alone.
    fs::remove_file(config_root(&machine).join("state.json")).unwrap();
    let unrecorded_plan = run(&machine, &["plan", "--target", "codex", "--json"]);
    assert_code(&unrecorded_plan, 2);
    let unrecorded_value = json(&unrecorded_plan);
    assert!(
        unrecorded_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["status"] == "repair"
                    && entry["fields"]["fresh_state"] == "present"
                    && entry["fields"]["recorded_state"] == "missing"
            })
    );
    let restore = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&restore, 0);
    assert_eq!(json(&restore)["summary"]["changed"], true);

    let healthy_plan = run(&machine, &["plan", "--target", "codex", "--json"]);
    assert_code(&healthy_plan, 2);
    let healthy_value = json(&healthy_plan);
    assert!(
        healthy_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["status"] == "no_change" && entry["fields"]["fresh_state"] == "present"
            })
    );

    fs::remove_file(&marker).unwrap();
    let drifted_plan = run(&machine, &["plan", "--target", "codex", "--json"]);
    assert_code(&drifted_plan, 2);
    let drifted_value = json(&drifted_plan);
    assert!(
        drifted_value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["status"] == "repair" && entry["fields"]["fresh_state"] == "missing"
            })
    );

    let repair = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&repair, 0);
    assert_eq!(json(&repair)["summary"]["changed"], true);
    assert!(marker.is_file());

    let repeat = run(&machine, &["sync", "--target", "codex", "--json"]);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
}

#[test]
fn claude_project_reobservation_does_not_borrow_same_name_user_presence() {
    let machine = machine();
    let harness = machine.home().join("fake-claude-scoped-presence");
    let project = machine.home().join("claude-project");
    let local_marker = machine.home().join("claude-local-plugin-present");
    fs::create_dir_all(&project).unwrap();
    let marker = local_marker.to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        &harness,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf '%s' '2.1.201 (Claude Code)'; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"list\" ]; then if [ -f '{marker}' ]; then printf '%s' '{{\"plugins\":[{{\"id\":\"formatter@team\",\"scope\":\"user\"}},{{\"id\":\"formatter@team\",\"scope\":\"local\"}}]}}'; else printf '%s' '{{\"plugins\":[{{\"id\":\"formatter@team\",\"scope\":\"user\"}}]}}'; fi; exit 0; fi\nif [ \"$1\" = \"plugin\" ] && [ \"$2\" = \"install\" ] && [ \"$5\" = \"local\" ]; then : > '{marker}'; exit 0; fi\nexit 0\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&harness).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&harness, permissions).unwrap();
    }
    write_owned(
        &machine,
        "config.toml",
        &native_config(&harness, &harness)
            .replace(
                "[harnesses.codex]\nenabled = true",
                "[harnesses.codex]\nenabled = false",
            )
            .replace(
                "[harnesses.claude]\nenabled = false",
                "[harnesses.claude]\nenabled = true",
            ),
    );
    fs::create_dir_all(machine.claude_home().join("plugins")).unwrap();

    let args = [
        "plugin",
        "install",
        "formatter@team",
        "--project",
        project.to_str().unwrap(),
        "--target",
        "claude",
        "--json",
    ];
    let install = run(&machine, &args);
    assert_code(&install, 0);
    assert_eq!(json(&install)["summary"]["changed"], true);
    assert!(local_marker.is_file());

    fs::remove_file(&local_marker).unwrap();
    let repair = run(&machine, &args);
    assert_code(&repair, 0);
    assert_eq!(json(&repair)["summary"]["changed"], true);
    assert!(
        local_marker.is_file(),
        "the missing local resource must be reapplied even while its user sibling exists"
    );
}

fn run_in(machine: &IsolatedMachine, cwd: &Path, arguments: &[&str]) -> Output {
    machine
        .run_in(&binary(), cwd, arguments)
        .expect("run compiled skilltap binary")
}

fn toml_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_str()
            .unwrap()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

#[test]
fn malformed_owned_documents_fail_safely_and_name_only_the_document() {
    for name in ["config.toml", "inventory.toml", "state.json"] {
        let machine = machine();
        write_owned(&machine, name, "not valid {{{ secret-marker");
        let output = run(&machine, &["status", "--json"]);
        assert_code(&output, 1);
        let value = json(&output);
        assert_eq!(value["result"], "invalid");
        assert_eq!(value["errors"][0]["code"], "owned_document_malformed");
        assert_eq!(
            value["errors"][0]["context"]["document"],
            name.trim_end_matches(".toml").trim_end_matches(".json")
        );
        assert!(!stdout(&output).contains("secret-marker"));
    }
}

#[test]
fn json_and_plain_modes_use_stable_channels_and_exit_classes() {
    let machine = machine();

    let attention = run(&machine, &["status"]);
    assert_code(&attention, 2);
    assert!(attention.stderr.is_empty());
    assert!(stdout(&attention).contains("Result: attention required"));
    assert!(!stdout(&attention).contains("\u{1b}["));

    let plan = run(&machine, &["plan"]);
    assert_code(&plan, 2);
    assert!(plan.stderr.is_empty());
    assert!(stdout(&plan).contains("Result: attention required"));
    assert!(!stdout(&plan).contains("\u{1b}["));

    let invalid = run(&machine, &["status", "--target", "pi"]);
    assert_code(&invalid, 1);
    assert!(invalid.stdout.is_empty());
    assert!(stderr(&invalid).contains("Code: target_not_registered"));
    assert!(stderr(&invalid).contains("harness  pi"));

    for arguments in [
        &["status", "--json"][..],
        &["plan", "--json"][..],
        &["status", "--target", "pi", "--json"][..],
        &["status", "--project", "--all-scopes", "--json"][..],
        &["status", "--yes", "--json"][..],
        &["plugin", "install", "not-a-selector", "--json"][..],
    ] {
        let output = run(&machine, arguments);
        let value = json(&output);
        assert_eq!(value["schema"], 1);
        assert!(!stdout(&output).contains("\u{1b}["));
    }
}

#[test]
fn daemon_refreshes_shared_marketplace_once_before_plugins_and_repeats_cleanly() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    assert_code(
        &run(
            &machine,
            &[
                "marketplace",
                "add",
                "https://example.invalid/team.git",
                "--name",
                "team",
                "--target",
                "claude",
                "--json",
            ],
        ),
        0,
    );
    for selector in ["formatter@team", "review@team"] {
        assert_code(
            &run(
                &machine,
                &[
                    "plugin", "install", selector, "--target", "claude", "--json",
                ],
            ),
            0,
        );
        claude._fixture.set_plugin_revision(selector, "1").unwrap();
        claude
            ._fixture
            .set_available_plugin_revision(selector, "2")
            .unwrap();
    }
    let first = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&first, 0);
    assert_eq!(json(&first)["summary"]["changed"], true);
    let invocations = claude._fixture.captured_invocations().unwrap();
    let arguments = invocations
        .iter()
        .map(|invocation| {
            invocation
                .arguments()
                .iter()
                .map(|argument| String::from_utf8_lossy(argument).into_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let refresh = arguments
        .iter()
        .position(|args| args == &["plugin", "marketplace", "update", "team"])
        .unwrap();
    assert_eq!(
        arguments
            .iter()
            .filter(|args| args == &&["plugin", "marketplace", "update", "team"])
            .count(),
        1
    );
    for selector in ["formatter@team", "review@team"] {
        let index = arguments
            .iter()
            .position(|args| args == &["plugin", "update", selector, "--scope", "user"])
            .unwrap();
        assert!(refresh < index);
    }
    let second = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&second, 0);
    assert_eq!(json(&second)["summary"]["changed"], false);
    assert_eq!(
        claude
            ._fixture
            .captured_invocations()
            .unwrap()
            .iter()
            .filter(|invocation| invocation.arguments()
                == [b"plugin".as_slice(), b"marketplace", b"update", b"team"])
            .count(),
        2
    );
}

#[test]
fn daemon_refresh_failure_is_target_local_and_status_redacts_native_details() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();
    for target in ["codex", "claude"] {
        assert_code(
            &run(
                &machine,
                &[
                    "marketplace",
                    "add",
                    "https://example.invalid/team.git",
                    "--name",
                    "team",
                    "--target",
                    target,
                    "--json",
                ],
            ),
            0,
        );
    }
    assert_code(
        &run(
            &machine,
            &[
                "plugin",
                "install",
                "formatter@team",
                "--target",
                "claude",
                "--json",
            ],
        ),
        0,
    );
    codex
        ._fixture
        .fail_lifecycle(FakeLifecycleAction::MarketplaceUpdate, "team")
        .unwrap();
    claude
        ._fixture
        .set_plugin_revision("formatter@team", "1")
        .unwrap();
    claude
        ._fixture
        .set_available_plugin_revision("formatter@team", "2")
        .unwrap();
    claude
        ._fixture
        .indeterminate_lifecycle(FakeLifecycleAction::PluginUpdate, "formatter@team")
        .unwrap();
    let daemon = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&daemon, 2);
    let value = json(&daemon);
    assert!(
        value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["fields"]["action"] == "marketplace_refresh"
                && entry["fields"]["target"] == "codex"),
        "{value}"
    );
    assert!(
        value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["fields"]["action"] == "plugin_update"
                && entry["fields"]["target"] == "claude")
    );
    assert!(!stdout(&daemon).contains("plugin marketplace upgrade"));
    assert!(!stdout(&daemon).contains("> 23"));
    assert!(
        codex
            ._fixture
            .captured_invocations()
            .unwrap()
            .iter()
            .any(|invocation| invocation.arguments()
                == [b"plugin".as_slice(), b"marketplace", b"upgrade", b"team"])
    );
    assert!(
        claude
            ._fixture
            .captured_invocations()
            .unwrap()
            .iter()
            .any(|invocation| invocation.arguments()
                == [
                    b"plugin".as_slice(),
                    b"update",
                    b"formatter@team",
                    b"--scope",
                    b"user"
                ])
    );
}

#[test]
fn daemon_refresh_failure_skips_same_target_plugin_and_persists_status_evidence() {
    let machine = machine();
    let codex = fake_harness(&machine, &FakeHarnessProfile::codex());
    let claude = fake_harness(&machine, &FakeHarnessProfile::claude());
    write_owned(
        &machine,
        "config.toml",
        &native_config(codex.executable(), claude.executable()),
    );
    fs::create_dir_all(machine.home().join(".codex/plugins")).unwrap();
    fs::create_dir_all(machine.home().join(".claude/plugins")).unwrap();

    assert_code(
        &run(
            &machine,
            &[
                "marketplace",
                "add",
                "https://example.invalid/team.git",
                "--name",
                "team",
                "--target",
                "claude",
                "--json",
            ],
        ),
        0,
    );
    assert_code(
        &run(
            &machine,
            &[
                "plugin",
                "install",
                "formatter@team",
                "--target",
                "claude",
                "--json",
            ],
        ),
        0,
    );
    claude
        ._fixture
        .set_plugin_revision("formatter@team", "1")
        .unwrap();
    claude
        ._fixture
        .set_available_plugin_revision("formatter@team", "2")
        .unwrap();
    claude
        ._fixture
        .fail_lifecycle(FakeLifecycleAction::MarketplaceUpdate, "team")
        .unwrap();

    let daemon = run(&machine, &["daemon", "run", "--json"]);
    assert_code(&daemon, 2);
    let daemon_value = json(&daemon);
    let operations = daemon_value["operations"].as_array().unwrap();
    assert!(
        operations.iter().any(|entry| {
            entry["fields"]["action"] == "marketplace_refresh"
                && entry["fields"]["target"] == "claude"
                && entry["status"] == "failed"
        }),
        "marketplace failure was not explicit: {daemon_value}"
    );
    assert!(
        operations.iter().any(|entry| {
            entry["fields"]["action"] == "plugin_update"
                && entry["fields"]["resource"] == "plugin:formatter@team [global]"
                && entry["fields"]["target"] == "claude"
                && entry["status"] == "skipped_dependency"
        }),
        "dependent plugin was not marked skipped: {daemon_value}"
    );
    assert!(
        daemon_value["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["code"] == "native.command_failed"),
        "refresh failure error was not explicit: {daemon_value}"
    );
    assert!(
        !claude
            ._fixture
            .captured_invocations()
            .unwrap()
            .iter()
            .any(|invocation| invocation.arguments()
                == [
                    b"plugin".as_slice(),
                    b"update",
                    b"formatter@team",
                    b"--scope",
                    b"user"
                ])
    );

    let status = run(&machine, &["status", "--target", "claude", "--json"]);
    assert_code(&status, 2);
    let status_value = json(&status);
    let resources = status_value["resources"].as_array().unwrap();
    assert!(
        resources.iter().any(|entry| {
            entry["status"] == "skipped_dependency"
                && entry["fields"]["phase"] == "plugin_update"
                && entry["fields"]["resource"] == "plugin:formatter@team [global]"
                && entry["fields"]["target"] == "claude"
        }),
        "status omitted dependency skip: {status_value}"
    );
    assert!(
        resources.iter().any(|entry| {
            entry["fields"]["phase"] == "marketplace_refresh" && entry["status"] == "failed"
        }),
        "status omitted refresh failure: {status_value}"
    );
}
