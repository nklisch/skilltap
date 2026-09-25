//! Regressions from the 2026-09-24 repository audit.
use super::*;

fn source(machine: &IsolatedMachine, parent: &str, body: &str) -> PathBuf {
    let root = machine.working_directory().join(parent).join("audit-demo");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("SKILL.md"),
        format!("---\nname: audit-demo\ndescription: Audit regression\n---\n{body}\n"),
    )
    .unwrap();
    fs::write(
        root.join("scripts/helper.txt"),
        "complete directory resource",
    )
    .unwrap();
    root
}

fn inventory_targets(machine: &IsolatedMachine) -> Vec<String> {
    let inventory: toml::Value =
        toml::from_str(&fs::read_to_string(config_root(machine).join("inventory.toml")).unwrap())
            .unwrap();
    inventory["resources"][0]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn targeted_global_removal_retains_canonical_tree_for_disabled_sibling() {
    for removed in ["claude", "codex"] {
        let m = machine();
        let _h = enable_default_harnesses(&m);
        let src = source(&m, "source", "v1");
        assert_code(
            &run(&m, &["skill", "install", src.to_str().unwrap(), "--json"]),
            0,
        );
        let sibling = if removed == "claude" {
            "codex"
        } else {
            "claude"
        };
        assert_code(&run(&m, &["harness", "disable", sibling, "--json"]), 0);
        assert_code(
            &run(
                &m,
                &[
                    "skill",
                    "remove",
                    "audit-demo",
                    "--target",
                    removed,
                    "--json",
                ],
            ),
            0,
        );
        assert!(
            m.home()
                .join(".agents/skills/audit-demo/SKILL.md")
                .is_file()
        );
        assert_eq!(inventory_targets(&m), [sibling]);
        let before = snapshot_tree_for_audit(m.home());
        let repeat = run(
            &m,
            &[
                "skill",
                "remove",
                "audit-demo",
                "--target",
                removed,
                "--json",
            ],
        );
        assert_eq!(json(&repeat)["summary"]["changed"], false);
        assert_eq!(before, snapshot_tree_for_audit(m.home()));
    }
}

fn snapshot_tree_for_audit(root: &Path) -> Vec<NativeEntrySnapshot> {
    snapshot_native_tree(root)
}

#[test]
fn partial_global_skill_publication_never_reports_success() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let src = source(&m, "source", "v1");
    let destination = m.home().join(".claude/skills/audit-demo");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, "user file").unwrap();
    for _ in 0..2 {
        let output = run(
            &m,
            &[
                "skill",
                "install",
                src.to_str().unwrap(),
                "--target",
                "claude",
                "--json",
            ],
        );
        assert_code(&output, 3);
        let value = json(&output);
        assert_eq!(value["result"], "partial_apply");
        assert!(!value["errors"].as_array().unwrap().is_empty());
        assert!(!value["next_actions"].as_array().unwrap().is_empty());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "user file");
    }
}

#[test]
fn relative_local_skill_sources_and_subdirectories_install_idempotently() {
    for (relative, subdirectory) in [
        ("./local source/audit-demo", None),
        ("local source", Some("audit-demo")),
        ("./other/../local source/audit-demo", None),
    ] {
        let m = machine();
        let _h = enable_default_harnesses(&m);
        source(&m, "local source", "v1");
        fs::create_dir_all(m.working_directory().join("other")).unwrap();
        let mut args = vec!["skill", "install", relative, "--target", "codex", "--json"];
        if let Some(subdirectory) = subdirectory {
            args.extend(["--path", subdirectory]);
        }
        assert_code(&run(&m, &args), 0);
        let repeat = run(&m, &args);
        assert_code(&repeat, 0);
        assert_eq!(json(&repeat)["summary"]["changed"], false);
        assert!(
            m.home()
                .join(".agents/skills/audit-demo/scripts/helper.txt")
                .is_file()
        );
    }
}

#[test]
fn named_and_bulk_updates_do_not_install_unselected_resource_targets() {
    for named in [false, true] {
        let m = machine();
        let _h = enable_default_harnesses(&m);
        let src = source(&m, "source", "v1");
        assert_code(
            &run(
                &m,
                &[
                    "skill",
                    "install",
                    src.to_str().unwrap(),
                    "--target",
                    "codex",
                    "--json",
                ],
            ),
            0,
        );
        let mut args = vec!["skill", "update"];
        if named {
            args.push("audit-demo");
        }
        args.push("--json");
        for _ in 0..2 {
            assert_code(&run(&m, &args), 0);
        }
        assert!(!m.home().join(".claude/skills/audit-demo").exists());
        assert_eq!(inventory_targets(&m), ["codex"]);
    }
}

#[test]
fn sequential_global_installs_extend_desired_targets_and_sync_repairs_them() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let src = source(&m, "source", "v1");
    for target in ["codex", "claude"] {
        let args = [
            "skill",
            "install",
            src.to_str().unwrap(),
            "--target",
            target,
            "--json",
        ];
        assert_code(&run(&m, &args), 0);
        assert_eq!(json(&run(&m, &args))["summary"]["changed"], false);
    }
    assert_eq!(inventory_targets(&m), ["claude", "codex"]);
    let destination = m.home().join(".claude/skills/audit-demo");
    fs::remove_dir_all(&destination).unwrap();
    assert_code(&run(&m, &["sync", "--target", "claude", "--json"]), 0);
    assert!(destination.join("SKILL.md").exists());
    assert_eq!(
        json(&run(&m, &["sync", "--target", "claude", "--json"]))["summary"]["changed"],
        false
    );
}

fn git_commit(root: &Path) {
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Audit",
            "-c",
            "user.email=audit@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}

#[test]
fn named_all_scopes_update_preserves_each_git_source_and_repeats_cleanly() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let global = source(&m, "global-source", "global-v1");
    let local = source(&m, "project-source", "project-v1");
    let project = m.working_directory().join("project");
    fs::create_dir_all(&project).unwrap();
    for (root, scope) in [(&global, None), (&local, Some(project.to_str().unwrap()))] {
        assert!(
            Command::new("git")
                .args(["init", "--quiet", "--initial-branch", "main"])
                .current_dir(root)
                .output()
                .unwrap()
                .status
                .success()
        );
        git_commit(root);
        let url = format!("file://{}", root.display());
        let mut args = vec!["skill", "install", &url, "--target", "codex", "--json"];
        if let Some(scope) = scope {
            args.extend(["--project", scope]);
        }
        assert_code(&run(&m, &args), 0);
        let skill = root.join("SKILL.md");
        fs::write(
            &skill,
            fs::read_to_string(&skill).unwrap().replace("v1", "v2"),
        )
        .unwrap();
        git_commit(root);
    }
    let args = [
        "skill",
        "update",
        "audit-demo",
        "--all-scopes",
        "--target",
        "codex",
        "--json",
    ];
    assert_code(&run(&m, &args), 0);
    assert!(
        fs::read_to_string(m.home().join(".agents/skills/audit-demo/SKILL.md"))
            .unwrap()
            .contains("global-v2")
    );
    assert!(
        fs::read_to_string(project.join(".agents/skills/audit-demo/SKILL.md"))
            .unwrap()
            .contains("project-v2")
    );
    let repeat = run(&m, &args);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
}

#[test]
fn skill_plan_distinguishes_satisfied_missing_and_drifted_without_writes() {
    let m = machine();
    let opencode = fake_harness(&m, &FakeHarnessProfile::opencode());
    write_owned(
        &m,
        "config.toml",
        &constrained_config("opencode", opencode.executable())
            .replace("mode = \"apply-safe\"", "mode = \"off\""),
    );
    let src = source(&m, "source", "v1");
    assert_code(
        &run(&m, &["skill", "install", src.to_str().unwrap(), "--json"]),
        0,
    );
    let before = owned_document_bytes(&m, "state.json");
    for _ in 0..2 {
        let plan = run(&m, &["plan", "--json"]);
        assert_code(&plan, 0);
        assert!(json(&plan)["operations"].as_array().unwrap().is_empty());
    }
    assert_eq!(owned_document_bytes(&m, "state.json"), before);
    assert_code(&run(&m, &["status", "--json"]), 0);
    let skill = m.home().join(".agents/skills/audit-demo/SKILL.md");
    fs::write(
        &skill,
        "---\nname: audit-demo\ndescription: changed\n---\nUSER EDIT\n",
    )
    .unwrap();
    let drift = run(&m, &["plan", "--json"]);
    assert_code(&drift, 2);
    assert_eq!(json(&drift)["operations"][0]["status"], "blocked");
    fs::remove_dir_all(skill.parent().unwrap()).unwrap();
    let missing = run(&m, &["plan", "--json"]);
    assert_code(&missing, 2);
    assert_eq!(json(&missing)["operations"][0]["status"], "repair");
    assert!(!skill.exists());
    assert_eq!(owned_document_bytes(&m, "state.json"), before);
}

#[test]
fn selected_global_updates_preserve_sibling_native_content_and_plan() {
    for selected in ["claude", "codex"] {
        let m = machine();
        let _h = enable_default_harnesses(&m);
        let src = source(&m, "source", "v1");
        assert_code(
            &run(&m, &["skill", "install", src.to_str().unwrap(), "--json"]),
            0,
        );
        fs::write(
            src.join("SKILL.md"),
            "---\nname: audit-demo\ndescription: Audit regression\n---\nv2\n",
        )
        .unwrap();
        let other = if selected == "claude" {
            "codex"
        } else {
            "claude"
        };
        let other_root = if other == "codex" {
            ".agents"
        } else {
            ".claude"
        };
        let sibling = m.home().join(other_root).join("skills/audit-demo/SKILL.md");
        let before = fs::read(&sibling).unwrap();
        let args = [
            "skill",
            "update",
            "audit-demo",
            "--target",
            selected,
            "--json",
        ];
        assert_code(&run(&m, &args), 2);
        assert_eq!(fs::read(&sibling).unwrap(), before);
        assert_code(&run(&m, &["plan", "--target", other, "--json"]), 0);
        let repeat = run(&m, &args);
        assert_code(&repeat, 2);
        assert_eq!(json(&repeat)["summary"]["changed"], false);
        let all = ["skill", "update", "audit-demo", "--target", "all", "--json"];
        assert_code(&run(&m, &all), 0);
        assert_eq!(json(&run(&m, &all))["summary"]["changed"], false);
        assert_code(
            &run(
                &m,
                &[
                    "skill",
                    "remove",
                    "audit-demo",
                    "--target",
                    "codex",
                    "--json",
                ],
            ),
            0,
        );
        assert_code(&run(&m, &["plan", "--target", "claude", "--json"]), 0);
        assert_code(
            &run(
                &m,
                &[
                    "skill",
                    "remove",
                    "audit-demo",
                    "--target",
                    "claude",
                    "--json",
                ],
            ),
            0,
        );
    }
}

#[cfg(unix)]
#[test]
fn relative_skill_source_preserves_symlink_parent_semantics() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let right = source(&m, "right", "RIGHT");
    let _wrong = source(&m, ".", "WRONG");
    std::os::unix::fs::symlink(&right, m.working_directory().join("link")).unwrap();
    let args = [
        "skill",
        "install",
        "./link/../audit-demo",
        "--target",
        "codex",
        "--json",
    ];
    assert_code(&run(&m, &args), 0);
    let installed = m.home().join(".agents/skills/audit-demo/SKILL.md");
    assert!(fs::read_to_string(installed).unwrap().ends_with("RIGHT\n"));
    let repeat = run(&m, &args);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
}

#[test]
fn failed_update_check_requires_attention_without_native_drift() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let src = source(&m, "repository", "v1");
    assert!(
        Command::new("git")
            .args(["init", "--quiet", "--initial-branch", "main"])
            .current_dir(&src)
            .status()
            .unwrap()
            .success()
    );
    git_commit(&src);
    let url = format!("file://{}", src.display());
    assert_code(
        &run(
            &m,
            &["skill", "install", &url, "--target", "codex", "--json"],
        ),
        0,
    );
    fs::remove_dir_all(src).unwrap();
    for _ in 0..2 {
        let output = run(&m, &["status", "--target", "codex", "--json"]);
        assert_code(&output, 2);
        assert!(
            json(&output)["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|warning| warning["code"] == "update_resolution_unavailable")
        );
    }
}

#[test]
fn shared_native_directory_updates_once_and_records_every_target() {
    let m = machine();
    let codex = fake_harness(&m, &FakeHarnessProfile::codex());
    let opencode = fake_harness(&m, &FakeHarnessProfile::opencode());
    let config = native_config_with_opencode(
        codex.executable(),
        codex.executable(),
        opencode.executable(),
    )
    .replace(
        "[harnesses.claude]\nenabled = true",
        "[harnesses.claude]\nenabled = false",
    );
    write_owned(&m, "config.toml", &config);
    let src = source(&m, "source", "v1");
    assert_code(
        &run(&m, &["skill", "install", src.to_str().unwrap(), "--json"]),
        0,
    );
    fs::write(
        src.join("SKILL.md"),
        "---\nname: audit-demo\ndescription: Audit regression\n---\nv2\n",
    )
    .unwrap();
    assert_code(
        &run(
            &m,
            &[
                "skill",
                "update",
                "audit-demo",
                "--target",
                "codex",
                "--json",
            ],
        ),
        2,
    );
    let args = ["skill", "update", "audit-demo", "--json"];
    assert_code(&run(&m, &args), 0);
    assert_code(&run(&m, &["plan", "--json"]), 0);
    let repeat = run(&m, &args);
    assert_code(&repeat, 0);
    assert_eq!(json(&repeat)["summary"]["changed"], false);
    assert_code(&run(&m, &["skill", "remove", "audit-demo", "--json"]), 0);
}

#[test]
fn missing_shared_directory_allows_same_content_repair_but_not_selected_replacement() {
    let m = machine();
    let _h = enable_default_harnesses(&m);
    let src = source(&m, "source", "v1");
    assert_code(
        &run(&m, &["skill", "install", src.to_str().unwrap(), "--json"]),
        0,
    );
    let canonical = m.home().join(".agents/skills/audit-demo");
    fs::remove_dir_all(&canonical).unwrap();
    let args = [
        "skill",
        "update",
        "audit-demo",
        "--target",
        "codex",
        "--json",
    ];
    assert_code(&run(&m, &args), 0);
    assert_eq!(json(&run(&m, &args))["summary"]["changed"], false);
    fs::remove_dir_all(&canonical).unwrap();
    fs::write(
        src.join("SKILL.md"),
        "---\nname: audit-demo\ndescription: Audit regression\n---\nv2\n",
    )
    .unwrap();
    for _ in 0..2 {
        let output = run(&m, &args);
        assert_code(&output, 2);
        assert_eq!(json(&output)["summary"]["changed"], false);
        assert!(!canonical.exists());
    }
    assert!(
        fs::read_to_string(m.home().join(".claude/skills/audit-demo/SKILL.md"))
            .unwrap()
            .ends_with("v1\n")
    );
    assert_code(&run(&m, &["skill", "update", "audit-demo", "--json"]), 0);
    assert_code(&run(&m, &["plan", "--json"]), 0);
}
