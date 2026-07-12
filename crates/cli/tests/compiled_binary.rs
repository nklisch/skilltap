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
    assert_eq!(
        fs::read_to_string(config_root(&machine).join("state.json")).unwrap(),
        state
    );

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
    assert_eq!(update_all_value["summary"]["operations"], 1);

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
fn targeted_native_remove_preserves_unselected_harness() {
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
    let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
    assert!(state.contains("\"claude\": \"targeted-update\""));
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
    assert_code(&repair, 2);
    assert!(
        fs::symlink_metadata(machine.home().join(".claude/CLAUDE.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_dir(config_root(&machine).join("managed/backups/instructions"))
            .unwrap()
            .next()
            .is_some()
    );
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
    let root = machine.configuration_home().join("systemd/user");
    let service = root.join("skilltap-update.service");
    let timer = root.join("skilltap-update.timer");
    assert!(service.is_file());
    assert!(timer.is_file());
    let service_bytes = fs::read(&service).unwrap();
    let timer_bytes = fs::read(&timer).unwrap();
    let service_mtime = fs::metadata(&service).unwrap().modified().unwrap();
    let timer_mtime = fs::metadata(&timer).unwrap().modified().unwrap();

    let second = run(&machine, &["daemon", "enable", "--json"]);
    assert!(second.status.code() == Some(0) || second.status.code() == Some(2));
    let second_value = json(&second);
    assert_eq!(second_value["summary"]["changed"], false);
    assert_eq!(first_value["command"], "daemon enable");
    assert_eq!(fs::read(&service).unwrap(), service_bytes);
    assert_eq!(fs::read(&timer).unwrap(), timer_bytes);
    assert_eq!(
        fs::metadata(&service).unwrap().modified().unwrap(),
        service_mtime
    );
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
    assert_code(&output, 2);
    assert_eq!(json(&output)["result"], "attention_required");
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

#[test]
fn native_plugin_and_marketplace_lifecycle_covers_both_harnesses_and_journal_repeats() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()),
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
        assert_code(&plugin_update, 0);
        assert_eq!(json(&plugin_update)["result"], "completed");

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
fn native_mutations_keep_project_and_all_scope_boundaries() {
    let machine = machine();
    let fixture = FakeNativeProcess::new(FakeNativeMode::VersionKnown).unwrap();
    write_owned(
        &machine,
        "config.toml",
        &native_config(fixture.executable(), fixture.executable()),
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
    fs::write(&service, b"# skilltap-managed-v3\nmalformed definition\n").unwrap();
    let malformed = run(&machine, &["daemon", "enable", "--json"]);
    assert_code(&malformed, 2);
    assert_eq!(
        json(&malformed)["warnings"][0]["code"],
        "daemon_definition_conflict"
    );
    assert_eq!(
        fs::read(&service).unwrap(),
        b"# skilltap-managed-v3\nmalformed definition\n"
    );

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
            .any(|entry| { entry["status"] == "planned" || entry["status"] == "pending" })
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
