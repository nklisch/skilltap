#![cfg(unix)]

use std::{fs, path::Path, process::Output};

use serde_json::Value;
use skilltap_test_support::{IsolatedMachine, captured_stderr, compiled_binary};

const CODEX_ONLY_CONFIG: &str = r#"schema = 1

[harnesses.codex]
enabled = true
binary = "codex"

[harnesses.claude]
enabled = false
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

fn run(machine: &IsolatedMachine, codex_home: &Path, arguments: &[&str]) -> Output {
    machine
        .run_with_env(
            &compiled_binary(env!("CARGO_BIN_EXE_skilltap")).unwrap(),
            arguments,
            [("CODEX_HOME", codex_home)],
        )
        .unwrap()
}

fn assert_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        captured_stderr(output).unwrap(),
    );
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn custom_codex_home_uses_and_validates_the_effective_canonical_target() {
    let machine = IsolatedMachine::new("skilltap-custom-instruction-bridge").unwrap();
    let config_root = machine.configuration_home().join("skilltap");
    fs::create_dir_all(&config_root).unwrap();
    fs::write(config_root.join("config.toml"), CODEX_ONLY_CONFIG).unwrap();
    let isolated_root = machine.working_directory().parent().unwrap();
    let custom_codex_home = isolated_root.join("custom-codex");
    fs::create_dir_all(&custom_codex_home).unwrap();
    let bridge = custom_codex_home.join("AGENTS.md");
    let canonical = machine.home().join("AGENTS.md");

    let setup = run(
        &machine,
        &custom_codex_home,
        &["instructions", "setup", "--json"],
    );
    assert_code(&setup, 0);
    assert_eq!(
        fs::read_link(&bridge).unwrap(),
        Path::new("../home/AGENTS.md")
    );

    fs::remove_file(&bridge).unwrap();
    std::os::unix::fs::symlink("../AGENTS.md", &bridge).unwrap();
    let wrong_status = run(
        &machine,
        &custom_codex_home,
        &["instructions", "status", "--json"],
    );
    assert_code(&wrong_status, 2);
    assert!(
        json(&wrong_status)["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |resource| resource["fields"]["path"] == bridge.to_str().unwrap()
                    && resource["status"] != "managed"
            )
    );

    let repair = run(
        &machine,
        &custom_codex_home,
        &["instructions", "repair", "--yes", "--json"],
    );
    assert!(matches!(repair.status.code(), Some(0 | 2)));
    assert_eq!(
        fs::read_link(&bridge).unwrap(),
        Path::new("../home/AGENTS.md")
    );

    fs::remove_file(&bridge).unwrap();
    std::os::unix::fs::symlink(&canonical, &bridge).unwrap();
    let absolute_status = run(
        &machine,
        &custom_codex_home,
        &["instructions", "status", "--json"],
    );
    assert_code(&absolute_status, 2);
    assert!(
        json(&absolute_status)["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |resource| resource["fields"]["path"] == bridge.to_str().unwrap()
                    && resource["status"] == "broken"
            )
    );
}
