#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Output};

use serde_json::Value;
use skilltap_test_support::{IsolatedMachine, captured_stderr, captured_stdout, compiled_binary};

fn binary() -> std::path::PathBuf {
    compiled_binary(env!("CARGO_BIN_EXE_skilltap")).expect("resolve compiled skilltap binary")
}

fn config_root(machine: &IsolatedMachine) -> std::path::PathBuf {
    machine.configuration_home().join("skilltap")
}

fn json(output: &Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "JSON wrote stderr: {}",
        captured_stderr(output).unwrap()
    );
    serde_json::from_str(captured_stdout(output).unwrap()).expect("stdout is JSON")
}

fn install(machine: &IsolatedMachine) -> Output {
    machine
        .run(
            &binary(),
            &[
                "plugin",
                "install",
                "formatter@team",
                "--target",
                "claude",
                "--json",
            ],
        )
        .expect("run isolated lifecycle command")
}

fn install_plain(machine: &IsolatedMachine) -> Output {
    machine
        .run(
            &binary(),
            &["plugin", "install", "formatter@team", "--target", "claude"],
        )
        .expect("run isolated plain lifecycle command")
}

fn remove(machine: &IsolatedMachine) -> Output {
    machine
        .run(
            &binary(),
            &[
                "plugin",
                "remove",
                "formatter@team",
                "--target",
                "claude",
                "--json",
            ],
        )
        .expect("run isolated lifecycle removal")
}

fn write_fixture(
    machine: &IsolatedMachine,
    mode: &Path,
    present: &Path,
    count: &Path,
) -> std::path::PathBuf {
    let executable = machine.home().join("claude-postcondition-fixture");
    let quote = |path: &Path| path.to_string_lossy().replace('\'', "'\\''");
    fs::write(
        &executable,
        format!(
            r#"#!/bin/sh
if [ "${{1-}}" = "--version" ]; then printf '%s\n' '2.1.201 (Claude Code)'; exit 0; fi
if [ "${{1-}} ${{2-}}" = "plugin install" ]; then
  count=0
  if [ -f '{count}' ]; then read count < '{count}'; fi
  count=$((count + 1))
  printf '%s' "$count" > '{count}'
  : > '{present}'
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin uninstall" ]; then
  count=0
  if [ -f '{count}' ]; then read count < '{count}'; fi
  count=$((count + 1))
  printf '%s' "$count" > '{count}'
  current=good
  if [ -f '{mode}' ]; then read current < '{mode}'; fi
  if [ "$current" != "expected_missing" ]; then /bin/rm -f '{present}'; fi
  exit 0
fi
if [ "${{1-}} ${{2-}}" = "plugin list" ]; then
  current=good
  if [ -f '{mode}' ]; then read current < '{mode}'; fi
  case "$current" in
    good)
      if [ -f '{present}' ]; then printf '%s' '{{"plugins":[{{"name":"formatter@team","scope":"user"}}]}}'; else printf '%s' '{{"plugins":[]}}'; fi
      exit 0 ;;
    command_failed) exit 17 ;;
    invalid_json) printf '%s' '{{malformed'; exit 0 ;;
    unsupported_shape) printf '%s' '{{"version":"2.1.201"}}'; exit 0 ;;
    ambiguous_scope) printf '%s' '{{"plugins":[{{"name":"formatter@team"}}]}}'; exit 0 ;;
    expected_present) printf '%s' '{{"plugins":[]}}'; exit 0 ;;
    expected_missing) printf '%s' '{{"plugins":[{{"name":"formatter@team","scope":"user"}}]}}'; exit 0 ;;
  esac
fi
exit 0
"#,
            mode = quote(mode),
            present = quote(present),
            count = quote(count),
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    executable
}

#[test]
fn native_remove_requires_missing_postcondition_and_repeats_without_mutation() {
    let machine = IsolatedMachine::new("skilltap-native-remove-postcondition").unwrap();
    let mode = machine.home().join("mode");
    let present = machine.home().join("present");
    let count = machine.home().join("mutation-count");
    fs::write(&mode, "good").unwrap();
    let fixture = write_fixture(&machine, &mode, &present, &count);
    configure(&machine, &fixture);

    let installed = install(&machine);
    assert_eq!(
        installed.status.code(),
        Some(0),
        "{}",
        captured_stdout(&installed).unwrap()
    );
    fs::write(&mode, "expected_missing").unwrap();
    let failed = remove(&machine);
    assert_eq!(failed.status.code(), Some(2));
    assert_eq!(
        json(&failed)["errors"][0]["code"],
        "native.postcondition.expected_missing"
    );
    assert!(present.is_file());

    fs::write(&mode, "good").unwrap();
    let removed = remove(&machine);
    assert_eq!(removed.status.code(), Some(0));
    assert_eq!(json(&removed)["summary"]["changed"], true);
    assert!(!present.exists());
    let count_after_remove = fs::read_to_string(&count).unwrap();

    let repeat = remove(&machine);
    assert_eq!(repeat.status.code(), Some(0));
    assert_eq!(json(&repeat)["summary"]["changed"], false);
    assert_eq!(fs::read_to_string(&count).unwrap(), count_after_remove);
}

fn configure(machine: &IsolatedMachine, executable: &Path) {
    fs::create_dir_all(machine.claude_home().join("plugins")).unwrap();
    fs::create_dir_all(machine.claude_home().join("skills")).unwrap();
    let root = config_root(machine);
    fs::create_dir_all(&root).unwrap();
    let binary = executable
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    fs::write(
        root.join("config.toml"),
        format!(
            r#"schema = 1

[harnesses.codex]
enabled = false
binary = "codex"

[harnesses.claude]
enabled = true
binary = "{binary}"

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"

[bootstrap]
mode = "off"
allow_major = false
"#
        ),
    )
    .unwrap();
}

fn configure_invalid_binaries_for_all_scopes(machine: &IsolatedMachine) {
    let root = config_root(machine);
    fs::create_dir_all(&root).unwrap();
    let mut binaries = Vec::new();
    for harness in ["codex", "claude"] {
        let executable = machine.home().join(format!("{harness}-invalid-version"));
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' 'not-a-supported-version'\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        binaries.push(
            executable
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\""),
        );
    }
    fs::write(
        root.join("config.toml"),
        format!(
            r#"schema = 1

[harnesses.codex]
enabled = true
binary = "{}"

[harnesses.claude]
enabled = true
binary = "{}"

[instructions]
claude_mode = "symlink"

[updates]
mode = "apply-safe"
interval = "6h"

[bootstrap]
mode = "off"
allow_major = false
"#,
            binaries[0], binaries[1]
        ),
    )
    .unwrap();
    let project = machine.home().join("project");
    fs::create_dir_all(&project).unwrap();
    let project = project
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    fs::write(
        root.join("inventory.toml"),
        format!("schema = 1\nprojects = [\"{project}\"]\nresources = []\n"),
    )
    .unwrap();
}

#[test]
fn multi_target_all_scope_diagnostics_have_plain_and_json_parity() {
    let structured_machine = IsolatedMachine::new("skilltap-diagnostic-parity-json").unwrap();
    configure_invalid_binaries_for_all_scopes(&structured_machine);
    let structured = structured_machine
        .run(
            &binary(),
            &["status", "--all-scopes", "--target", "all", "--json"],
        )
        .unwrap();
    assert_eq!(structured.status.code(), Some(2));
    let value = json(&structured);
    assert_eq!(value["scope"]["kind"], "all");
    assert_eq!(value["summary"]["scopes"], 2);
    assert_eq!(value["summary"]["targets"], 2);
    assert_eq!(
        value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|warning| warning["code"] == "native_version_invalid")
            .count(),
        2
    );
    assert_eq!(
        value["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["code"] == "inspect_harness_version")
            .count(),
        2
    );

    let plain_machine = IsolatedMachine::new("skilltap-diagnostic-parity-plain").unwrap();
    configure_invalid_binaries_for_all_scopes(&plain_machine);
    let plain = plain_machine
        .run(&binary(), &["status", "--all-scopes", "--target", "all"])
        .unwrap();
    assert_eq!(plain.status.code(), Some(2));
    assert!(plain.stderr.is_empty());
    let rendered = captured_stdout(&plain).unwrap();
    assert!(rendered.contains("Scope  all scopes"));
    assert_eq!(rendered.matches("native_version_invalid:").count(), 2);
    assert_eq!(rendered.matches("--version").count(), 2);
    assert!(rendered.ends_with("Result: attention required\n"));
}

#[test]
fn native_postcondition_failures_are_typed_and_never_journal_success() {
    for (mode_name, expected_code) in [
        ("command_failed", "native_observation_command_failed"),
        ("invalid_json", "native_observation_invalid_json"),
        ("unsupported_shape", "native_observation_unsupported_shape"),
        ("ambiguous_scope", "native_observation_ambiguous_scope"),
        ("expected_present", "native.postcondition.expected_present"),
    ] {
        let machine = IsolatedMachine::new("skilltap-native-postcondition").unwrap();
        let mode = machine.home().join("mode");
        let present = machine.home().join("present");
        let count = machine.home().join("mutation-count");
        fs::write(&mode, mode_name).unwrap();
        let fixture = write_fixture(&machine, &mode, &present, &count);
        configure(&machine, &fixture);

        let output = install(&machine);
        assert_eq!(output.status.code(), Some(2), "mode={mode_name}");
        let value = json(&output);
        assert_eq!(value["result"], "attention_required", "mode={mode_name}");
        assert_eq!(
            value["errors"][0]["code"], expected_code,
            "mode={mode_name}"
        );
        assert_eq!(
            value["errors"][0]["next_actions"][0]["code"], "reobserve_before_retry",
            "mode={mode_name}"
        );
        let state = fs::read_to_string(config_root(&machine).join("state.json")).unwrap();
        assert!(state.contains("\"status\": \"failed\""), "mode={mode_name}");
        assert!(
            !state.contains("\"status\": \"applied\""),
            "mode={mode_name}"
        );
    }
}

#[test]
fn native_postcondition_diagnostic_has_plain_and_json_parity() {
    fn failing_machine(label: &str) -> IsolatedMachine {
        let machine = IsolatedMachine::new(label).unwrap();
        let mode = machine.home().join("mode");
        let present = machine.home().join("present");
        let count = machine.home().join("mutation-count");
        fs::write(&mode, "invalid_json").unwrap();
        let fixture = write_fixture(&machine, &mode, &present, &count);
        configure(&machine, &fixture);
        machine
    }

    let json_machine = failing_machine("skilltap-native-postcondition-json");
    let structured = install(&json_machine);
    assert_eq!(structured.status.code(), Some(2));
    let value = json(&structured);
    assert_eq!(
        value["errors"][0]["code"],
        "native_observation_invalid_json"
    );
    assert_eq!(value["next_actions"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["next_actions"][0]["command"],
        "skilltap status --json"
    );

    let plain_machine = failing_machine("skilltap-native-postcondition-plain");
    let plain = install_plain(&plain_machine);
    assert_eq!(plain.status.code(), Some(2));
    assert!(plain.stderr.is_empty());
    let rendered = captured_stdout(&plain).unwrap();
    assert!(rendered.contains("Code: native_observation_invalid_json"));
    assert_eq!(rendered.matches("skilltap status --json").count(), 1);
    assert!(rendered.ends_with("Result: attention required\n"));
}

#[test]
fn prior_success_with_indeterminate_observation_does_not_repeat_mutation() {
    let machine = IsolatedMachine::new("skilltap-native-postcondition-repeat").unwrap();
    let mode = machine.home().join("mode");
    let present = machine.home().join("present");
    let count = machine.home().join("mutation-count");
    fs::write(&mode, "good").unwrap();
    let fixture = write_fixture(&machine, &mode, &present, &count);
    configure(&machine, &fixture);

    let first = install(&machine);
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        captured_stdout(&first).unwrap()
    );
    assert_eq!(json(&first)["summary"]["changed"], true);
    let state = fs::read(config_root(&machine).join("state.json")).unwrap();
    assert_eq!(fs::read_to_string(&count).unwrap(), "1");

    let repeat = install(&machine);
    assert_eq!(repeat.status.code(), Some(0));
    assert_eq!(json(&repeat)["summary"]["changed"], false);
    assert_eq!(fs::read_to_string(&count).unwrap(), "1");

    fs::write(&mode, "invalid_json").unwrap();
    let uncertain = install(&machine);
    assert_eq!(uncertain.status.code(), Some(2));
    let value = json(&uncertain);
    assert!(
        value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| { warning["code"] == "native_observation_invalid_json" })
    );
    assert_eq!(value["next_actions"][0]["code"], "reobserve_before_retry");
    assert_eq!(fs::read_to_string(&count).unwrap(), "1");
    assert_eq!(
        fs::read(config_root(&machine).join("state.json")).unwrap(),
        state
    );
}
