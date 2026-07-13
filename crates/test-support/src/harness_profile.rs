use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{FakeNativeBuilder, FakeNativeMode, FakeNativeProcess, IsolatedMachine, snapshot_tree};

/// How a fake harness responds to its version probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VersionResponse {
    TextPrefix {
        prefix: &'static str,
        version: &'static str,
    },
    TextSuffix {
        version: &'static str,
        suffix: &'static str,
    },
    Json {
        version: &'static str,
    },
}

impl VersionResponse {
    pub(crate) fn render(&self) -> String {
        match self {
            Self::TextPrefix { prefix, version } => format!("{prefix}{version}\n"),
            Self::TextSuffix { version, suffix } => format!("{version}{suffix}\n"),
            Self::Json { version } => format!(r#"{{"version":"{version}"}}"#),
        }
    }
}

/// Which native lifecycle command family the fake executable emulates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleDialect {
    Codex,
    Claude,
    None,
}

/// Registry-shaped identity and native behavior for one fake harness.
///
/// This crate intentionally stores a validated static id rather than importing
/// `HarnessId`: production crates use test-support as a dev dependency, so a
/// core or harnesses dependency here would create a package cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeHarnessProfile {
    id: &'static str,
    version_response: VersionResponse,
    lifecycle_dialect: LifecycleDialect,
}

impl FakeHarnessProfile {
    pub const fn codex() -> Self {
        Self {
            id: "codex",
            version_response: VersionResponse::TextPrefix {
                prefix: "codex-cli ",
                version: "0.144.1",
            },
            lifecycle_dialect: LifecycleDialect::Codex,
        }
    }

    pub const fn claude() -> Self {
        Self {
            id: "claude",
            version_response: VersionResponse::TextSuffix {
                version: "2.1.201",
                suffix: " (Claude Code)",
            },
            lifecycle_dialect: LifecycleDialect::Claude,
        }
    }

    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn version_response(&self) -> &VersionResponse {
        &self.version_response
    }

    pub const fn lifecycle_dialect(&self) -> LifecycleDialect {
        self.lifecycle_dialect
    }

    /// Compose this harness identity with an orthogonal process behavior.
    pub fn builder(&self, behavior: FakeNativeMode) -> FakeNativeBuilder {
        FakeNativeBuilder::new(behavior).for_harness(
            self.version_response.clone(),
            self.lifecycle_dialect,
            self.id,
        )
    }

    /// Materialize the fake beneath a caller-owned isolated root.
    pub fn build(&self, root: &Path, behavior: FakeNativeMode) -> io::Result<FakeNativeProcess> {
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "fake harness parent root does not exist",
            ));
        }
        self.builder(behavior).build_in(root)
    }

    fn layout(&self, machine: &IsolatedMachine) -> io::Result<AcceptanceLayout> {
        let project = machine.working_directory().join("acceptance-project");
        fs::create_dir_all(&project)?;
        match self.id {
            "codex" => Ok(AcceptanceLayout {
                global: ScopeLayout {
                    skill_root: machine.home().join(".agents/skills/contract-skill"),
                    mcp_document: machine.codex_home().join("config.toml"),
                },
                project: ScopeLayout {
                    skill_root: project.join(".agents/skills/contract-skill"),
                    mcp_document: project.join(".codex/config.toml"),
                },
                mcp_initial: b"[mcp_servers.contract]\ncommand = \"contract-server\"\n",
                mcp_reloaded: b"[mcp_servers.contract]\ncommand = \"contract-server-v2\"\n",
            }),
            "claude" => Ok(AcceptanceLayout {
                global: ScopeLayout {
                    skill_root: machine.claude_home().join("skills/contract-skill"),
                    mcp_document: machine.claude_home().join("settings.json"),
                },
                project: ScopeLayout {
                    skill_root: project.join(".claude/skills/contract-skill"),
                    mcp_document: project.join(".claude/settings.local.json"),
                },
                mcp_initial: br#"{"mcpServers":{"contract":{"command":"contract-server"}}}"#,
                mcp_reloaded: br#"{"mcpServers":{"contract":{"command":"contract-server-v2"}}}"#,
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "fake harness has no acceptance load-surface layout",
            )),
        }
    }
}

#[derive(Debug)]
struct AcceptanceLayout {
    global: ScopeLayout,
    project: ScopeLayout,
    mcp_initial: &'static [u8],
    mcp_reloaded: &'static [u8],
}

#[derive(Debug)]
struct ScopeLayout {
    skill_root: PathBuf,
    mcp_document: PathBuf,
}

/// Evidence emitted by the dependency-neutral fixture acceptance contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptanceReport {
    profile_id: &'static str,
    version_output: Vec<u8>,
    scopes_exercised: usize,
    complete_skill_files_observed: usize,
    mcp_documents_observed: usize,
    reloads_observed: usize,
    drifts_detected: usize,
    removals_verified: usize,
    repeat_no_changes: usize,
}

impl AcceptanceReport {
    pub fn profile_id(&self) -> &str {
        self.profile_id
    }

    pub fn version_output(&self) -> &[u8] {
        &self.version_output
    }

    pub fn passed(&self) -> bool {
        self.scopes_exercised == 2
            && self.complete_skill_files_observed == 6
            && self.mcp_documents_observed == 2
            && self.reloads_observed == 2
            && self.drifts_detected == 2
            && self.removals_verified == 4
            && self.repeat_no_changes == 2
    }
}

/// Exercise the fixture-level contract shared by adapter acceptance tests.
///
/// Production-bound detection, observation, and CLI mutation stay in the
/// harnesses and CLI crates to preserve dependency direction. This routine
/// supplies one reusable, isolated scenario for the documented global/project
/// load surfaces and verifies complete skills, MCP freshness, drift, removal,
/// and immediate-repeat idempotency without duplicating those end-to-end tests.
pub fn acceptance_matrix(
    profile: &FakeHarnessProfile,
    machine: &IsolatedMachine,
) -> io::Result<AcceptanceReport> {
    let native = profile.build(machine.working_directory(), FakeNativeMode::VersionKnown)?;
    let version = native.command().arg("--version").output()?;
    if !version.status.success() || !version.stderr.is_empty() {
        return Err(contract_error("version detection fixture failed"));
    }
    let expected_version = profile.version_response.render().into_bytes();
    if version.stdout != expected_version {
        return Err(contract_error("version detection bytes changed"));
    }

    let layout = profile.layout(machine)?;
    let mut report = AcceptanceReport {
        profile_id: profile.id,
        version_output: version.stdout,
        scopes_exercised: 0,
        complete_skill_files_observed: 0,
        mcp_documents_observed: 0,
        reloads_observed: 0,
        drifts_detected: 0,
        removals_verified: 0,
        repeat_no_changes: 0,
    };

    for scope in [&layout.global, &layout.project] {
        let first_changed = materialize_scope(scope, layout.mcp_initial)?;
        let before_repeat = snapshot_tree(&scope.skill_root)?;
        let repeat_changed = materialize_scope(scope, layout.mcp_initial)?;
        let after_repeat = snapshot_tree(&scope.skill_root)?;
        if !first_changed || repeat_changed || before_repeat != after_repeat {
            return Err(contract_error("immediate repeat was not idempotent"));
        }
        report.repeat_no_changes += 1;

        let observed_files = observe_complete_skill(&scope.skill_root)?;
        if observed_files != 3 {
            return Err(contract_error("complete skill directory was not observed"));
        }
        report.complete_skill_files_observed += observed_files;
        if fs::read(&scope.mcp_document)? != layout.mcp_initial {
            return Err(contract_error("MCP document was not freshly observable"));
        }
        report.mcp_documents_observed += 1;

        write_if_changed(&scope.mcp_document, layout.mcp_reloaded)?;
        if fs::read(&scope.mcp_document)? != layout.mcp_reloaded {
            return Err(contract_error("MCP reload did not expose fresh state"));
        }
        report.reloads_observed += 1;
        write_if_changed(&scope.mcp_document, layout.mcp_initial)?;

        let clean = snapshot_tree(&scope.skill_root)?;
        fs::write(
            scope.skill_root.join("scripts/run.sh"),
            b"#!/bin/sh\nexit 23\n",
        )?;
        let drifted = snapshot_tree(&scope.skill_root)?;
        if clean == drifted {
            return Err(contract_error("skill drift was not detected"));
        }
        report.drifts_detected += 1;
        materialize_scope(scope, layout.mcp_initial)?;
        report.scopes_exercised += 1;
    }

    for scope in [&layout.global, &layout.project] {
        fs::remove_dir_all(&scope.skill_root)?;
        fs::remove_file(&scope.mcp_document)?;
        if scope.skill_root.exists() || scope.mcp_document.exists() {
            return Err(contract_error(
                "managed acceptance resources remained after removal",
            ));
        }
        report.removals_verified += 2;
    }

    if !report.passed() {
        return Err(contract_error("acceptance evidence was incomplete"));
    }
    Ok(report)
}

fn materialize_scope(scope: &ScopeLayout, mcp: &[u8]) -> io::Result<bool> {
    let mut changed = false;
    changed |= write_if_changed(
        &scope.skill_root.join("SKILL.md"),
        b"---\nname: contract-skill\ndescription: acceptance fixture\n---\n",
    )?;
    changed |= write_if_changed(
        &scope.skill_root.join("scripts/run.sh"),
        b"#!/bin/sh\nexit 0\n",
    )?;
    changed |= write_if_changed(
        &scope.skill_root.join("references/contract.md"),
        b"# Contract fixture\n",
    )?;
    changed |= write_if_changed(&scope.mcp_document, mcp)?;
    Ok(changed)
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<bool> {
    match fs::read(path) {
        Ok(existing) if existing == bytes => return Ok(false),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(true)
}

fn observe_complete_skill(root: &Path) -> io::Result<usize> {
    for relative in ["SKILL.md", "scripts/run.sh", "references/contract.md"] {
        if !root.join(relative).is_file() {
            return Err(contract_error("complete skill entry is missing"));
        }
    }
    Ok(3)
}

fn contract_error(message: &'static str) -> io::Error {
    io::Error::other(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TempRoot;

    #[test]
    fn profile_versions_and_lifecycle_scripts_preserve_native_bytes() {
        let root = TempRoot::new("skilltap-profile-version").unwrap();
        for (profile, expected, install_args, remove_args) in [
            (
                FakeHarnessProfile::codex(),
                b"codex-cli 0.144.1\n".as_slice(),
                ["plugin", "add", "formatter@team"].as_slice(),
                ["plugin", "remove", "formatter@team"].as_slice(),
            ),
            (
                FakeHarnessProfile::claude(),
                b"2.1.201 (Claude Code)\n".as_slice(),
                ["plugin", "install", "formatter@team"].as_slice(),
                ["plugin", "uninstall", "formatter@team"].as_slice(),
            ),
        ] {
            let native = profile
                .build(root.path(), FakeNativeMode::VersionKnown)
                .unwrap();
            let output = native.command().arg("--version").output().unwrap();
            assert!(output.status.success());
            assert_eq!(output.stdout, expected);
            assert!(output.stderr.is_empty());

            assert!(
                native
                    .command()
                    .args(install_args)
                    .status()
                    .unwrap()
                    .success()
            );
            let listed = native
                .command()
                .args(["plugin", "list", "--json"])
                .output()
                .unwrap();
            assert!(listed.status.success());
            assert!(
                String::from_utf8(listed.stdout)
                    .unwrap()
                    .contains("formatter@team")
            );
            assert!(
                native
                    .command()
                    .args(remove_args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
    }

    #[test]
    fn codex_and_claude_pass_the_reusable_acceptance_matrix() {
        for profile in [FakeHarnessProfile::codex(), FakeHarnessProfile::claude()] {
            let machine = IsolatedMachine::new("skilltap-acceptance-matrix").unwrap();
            let report = acceptance_matrix(&profile, &machine).unwrap();
            assert_eq!(report.profile_id(), profile.id());
            assert_eq!(
                report.version_output(),
                profile.version_response().render().as_bytes()
            );
            assert!(report.passed());
        }
    }

    #[test]
    fn profile_build_requires_a_caller_owned_isolated_root() {
        let root = TempRoot::new("skilltap-missing-profile-root").unwrap();
        let missing = root.join("missing");
        assert_eq!(
            FakeHarnessProfile::codex()
                .build(&missing, FakeNativeMode::VersionKnown)
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
}
