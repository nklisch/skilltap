use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use skilltap_test_support::TempRoot;

struct IsolatedMachine {
    _root: TempRoot,
    home: PathBuf,
    xdg: PathBuf,
    cwd: PathBuf,
}

impl IsolatedMachine {
    fn new() -> Self {
        let root = TempRoot::new("skilltap-compiled-cli").expect("create isolated machine");
        let home = root.join("home");
        let xdg = root.join("xdg");
        let cwd = root.join("work");
        fs::create_dir_all(&home).expect("create isolated home");
        fs::create_dir_all(&xdg).expect("create isolated configuration home");
        fs::create_dir_all(&cwd).expect("create isolated working directory");
        Self {
            _root: root,
            home,
            xdg,
            cwd,
        }
    }

    fn config_root(&self) -> PathBuf {
        self.xdg.join("skilltap")
    }

    fn write_owned(&self, name: &str, contents: &str) {
        let root = self.config_root();
        fs::create_dir_all(&root).expect("create configuration root");
        fs::write(root.join(name), contents).expect("write owned document");
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(binary())
            .args(arguments)
            .current_dir(&self.cwd)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env_remove("SKILLTAP_HOME")
            .output()
            .expect("run compiled skilltap binary")
    }
}

fn binary() -> PathBuf {
    env::var_os("SKILLTAP_TEST_BIN").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_skilltap")),
        |value| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                env::current_dir()
                    .expect("read test working directory")
                    .join(path)
            }
        },
    )
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr is UTF-8")
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
    let machine = IsolatedMachine::new();
    let version = machine.run(&["--version"]);
    assert_code(&version, 0);
    assert_eq!(stdout(&version).trim(), "skilltap 3.0.0");
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
        let output = machine.run(&arguments);
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
        let output = machine.run(&arguments);
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
    let machine = IsolatedMachine::new();
    let output = machine.run(&[]);

    assert_code(&output, 1);
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("Usage: skilltap <COMMAND>"));
    assert!(stderr(&output).contains("Code: missing_command"));
    assert!(!stderr(&output).contains("\u{1b}["));
}

#[test]
fn first_use_status_is_read_only_and_uses_global_defaults() {
    let machine = IsolatedMachine::new();
    assert!(!machine.config_root().exists());

    let output = machine.run(&["status", "--json"]);
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
    assert!(!machine.config_root().exists());
}

#[test]
fn status_resolves_current_explicit_and_all_scopes_independently_from_targets() {
    let machine = IsolatedMachine::new();
    let project = machine.cwd.join("project");
    let nested = project.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&project)
        .output()
        .expect("initialize Git fixture");
    assert!(git.status.success(), "{}", stderr(&git));

    let global = machine.run(&["status", "--target", "codex", "--json"]);
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

    let explicit = machine.run(&[
        "status",
        "--project",
        nested.to_str().unwrap(),
        "--target",
        "all",
        "--json",
    ]);
    assert_code(&explicit, 2);
    let value = json(&explicit);
    assert_eq!(value["scope"]["path"], project.to_str().unwrap());
    assert_eq!(value["summary"]["targets"], 2);

    machine.write_owned(
        "inventory.toml",
        &format!(
            "schema = 1\nprojects = [{}]\nresources = []\n",
            toml_string(&project)
        ),
    );
    let all = machine.run(&["status", "--all-scopes", "--target", "all", "--json"]);
    assert_code(&all, 2);
    let value = json(&all);
    assert_eq!(value["scope"]["kind"], "all");
    assert_eq!(value["summary"]["scopes"], 2);
}

fn run_in(machine: &IsolatedMachine, cwd: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(cwd)
        .env("HOME", &machine.home)
        .env("XDG_CONFIG_HOME", &machine.xdg)
        .env_remove("SKILLTAP_HOME")
        .output()
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
        let machine = IsolatedMachine::new();
        machine.write_owned(name, "not valid {{{ secret-marker");
        let output = machine.run(&["status", "--json"]);
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
    let machine = IsolatedMachine::new();

    let attention = machine.run(&["status"]);
    assert_code(&attention, 2);
    assert!(attention.stderr.is_empty());
    assert!(stdout(&attention).contains("Result: attention required"));
    assert!(!stdout(&attention).contains("\u{1b}["));

    let unavailable = machine.run(&["plan"]);
    assert_code(&unavailable, 1);
    assert!(unavailable.stdout.is_empty());
    assert!(stderr(&unavailable).contains("Code: capability_unavailable"));
    assert!(!stderr(&unavailable).contains("\u{1b}["));

    let invalid = machine.run(&["status", "--target", "pi"]);
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
        let output = machine.run(arguments);
        let value = json(&output);
        assert_eq!(value["schema"], 1);
        assert!(!stdout(&output).contains("\u{1b}["));
    }
}
