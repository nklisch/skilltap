use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;
use skilltap_core::VERSION;
use skilltap_test_support::{IsolatedMachine, captured_stderr, captured_stdout, compiled_binary};

fn machine() -> IsolatedMachine {
    IsolatedMachine::new("skilltap-compiled-cli").expect("create isolated machine")
}

fn binary() -> std::path::PathBuf {
    compiled_binary(env!("CARGO_BIN_EXE_skilltap")).expect("resolve compiled skilltap binary")
}

fn config_root(machine: &IsolatedMachine) -> std::path::PathBuf {
    machine.configuration_home().join("skilltap")
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
        (&["harness", "disable", "claude"], "harness disable"),
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
        assert_code(&output, 1);
        if *command == "daemon run" {
            assert!(stdout(&output).is_empty());
            assert!(stderr(&output).contains("capability_unavailable"));
        } else {
            let value = json(&output);
            assert_eq!(value["command"], *command, "arguments: {arguments:?}");
            assert_eq!(value["errors"][0]["code"], "capability_unavailable");
        }
    }
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
fn first_use_status_is_read_only_and_uses_global_defaults() {
    let machine = machine();
    assert!(!config_root(&machine).exists());

    let output = run(&machine, &["status", "--json"]);
    assert_code(&output, 2);
    let value = json(&output);
    assert_eq!(value["command"], "status");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["scope"]["kind"], "global");
    assert_eq!(value["summary"]["targets"], 2);
    assert_eq!(
        value["warnings"][0]["code"],
        "native_observation_unavailable"
    );
    assert!(!config_root(&machine).exists());
}

#[test]
fn status_resolves_current_explicit_and_all_scopes_independently_from_targets() {
    let machine = machine();
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
    assert_eq!(value["resources"].as_array().unwrap().len(), 4);
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

    let unavailable = run(&machine, &["plan"]);
    assert_code(&unavailable, 1);
    assert!(unavailable.stdout.is_empty());
    assert!(stderr(&unavailable).contains("Code: capability_unavailable"));
    assert!(!stderr(&unavailable).contains("\u{1b}["));

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
