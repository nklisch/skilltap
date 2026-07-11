use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::SystemTime,
};

use serde_json::Value;
use skilltap_core::VERSION;
use skilltap_test_support::{
    FakeNativeMode, FakeNativeProcess, IsolatedMachine, captured_stderr, captured_stdout,
    compiled_binary,
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
"#;

fn machine() -> IsolatedMachine {
    IsolatedMachine::new("skilltap-compiled-cli").expect("create isolated machine")
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
"#,
        toml_string(codex),
        toml_string(claude),
    )
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
        "stderr: {}",
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

    for arguments in [
        vec!["--help"],
        vec!["harness", "--help"],
        vec!["harness", "enable", "--help"],
        vec!["adopt", "--help"],
        vec!["status", "--help"],
        vec!["plan", "--help"],
        vec!["sync", "--help"],
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
        (&["plugin", "remove", "format"], "plugin remove"),
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
        let output = run(&machine, &arguments);
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
            assert!(stdout(&output).is_empty());
            assert!(stderr(&output).contains("capability_unavailable"));
        } else if matches!(
            *command,
            "plan"
                | "sync"
                | "skill list"
                | "marketplace list"
                | "plugin list"
                | "instructions status"
                | "marketplace add"
                | "plugin install"
                | "skill install"
                | "instructions setup"
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
fn harness_policy_commands_are_non_interactive_idempotent_and_first_use_read_only() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();

    let first_list = run(&machine, &["harness", "list", "--json"]);
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

    let binary = fixture.executable();
    let binary_text = binary.to_str().expect("fake binary path is UTF-8");
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

    let list_plain = run(&machine, &["harness", "list"]);
    assert_code(&list_plain, 2);
    assert!(list_plain.stderr.is_empty());
    assert!(stdout(&list_plain).contains("codex  enabled"));
    assert!(stdout(&list_plain).contains("claude  disabled"));
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

    let output = run(&machine, &["status", "--json"]);
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
fn status_resolves_current_explicit_and_all_scopes_independently_from_targets() {
    let machine = machine();
    write_owned(&machine, "config.toml", ENABLED_CONFIG);
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
    assert_code(&current, 2);
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
    assert_code(&explicit, 2);
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
        warning["code"] == "native_detection_failed" && warning["context"]["harness"] == "claude"
    }));

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
        warning["code"] == "native_detection_failed" && warning["context"]["harness"] == "claude"
    }));
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
    assert!(stderr(&invalid).contains("Code: invalid_arguments"));
    assert!(!stderr(&invalid).contains("pi"));

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
