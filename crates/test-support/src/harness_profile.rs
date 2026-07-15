use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    ConditionalTargetProfile, FakeNativeBuilder, FakeNativeMode, FakeNativeProcess,
    IsolatedMachine, ManagedProjectionProfile, snapshot_tree,
};

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
    Factory,
    None,
}

/// Registry-shaped identity and native behavior for one fake harness.
///
/// This crate intentionally stores a validated static id rather than importing
/// `HarnessId`: production crates use test-support as a dev dependency, so a
/// core or harnesses dependency here would create a package cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LayoutBase {
    Home,
    Codex,
    Claude,
    Factory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AcceptanceLayoutSpec {
    global_skill_base: LayoutBase,
    global_mcp_base: LayoutBase,
    global_skill: &'static str,
    global_mcp: &'static str,
    project_skill: &'static str,
    project_mcp: &'static str,
    mcp_initial: &'static [u8],
    mcp_reloaded: &'static [u8],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeHarnessProfile {
    id: &'static str,
    version_response: VersionResponse,
    lifecycle_dialect: LifecycleDialect,
    managed_projection: Option<ManagedProjectionProfile>,
    conditional_profile: Option<ConditionalTargetProfile>,
    layout: AcceptanceLayoutSpec,
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
            managed_projection: Some(ManagedProjectionProfile::codex()),
            conditional_profile: None,
            layout: AcceptanceLayoutSpec {
                global_skill_base: LayoutBase::Home,
                global_mcp_base: LayoutBase::Codex,
                global_skill: ".agents/skills/contract-skill",
                global_mcp: "config.toml",
                project_skill: ".agents/skills/contract-skill",
                project_mcp: ".codex/config.toml",
                mcp_initial: b"[mcp_servers.contract]\ncommand = \"contract-server\"\n",
                mcp_reloaded: b"[mcp_servers.contract]\ncommand = \"contract-server-v2\"\n",
            },
        }
    }

    pub const fn pi() -> Self {
        Self::pi_with_version("0.80.6")
    }

    /// Build the same isolated Pi layout with a deliberately non-attested core
    /// version for unknown-version detection tests.
    pub const fn pi_with_version(version: &'static str) -> Self {
        Self {
            id: "pi",
            version_response: VersionResponse::TextSuffix {
                version,
                suffix: "",
            },
            lifecycle_dialect: LifecycleDialect::None,
            managed_projection: None,
            conditional_profile: Some(ConditionalTargetProfile::pi()),
            layout: AcceptanceLayoutSpec {
                global_skill_base: LayoutBase::Home,
                global_mcp_base: LayoutBase::Home,
                global_skill: ".agents/skills/contract-skill",
                global_mcp: ".pi/agent/mcp.json",
                project_skill: ".agents/skills/contract-skill",
                project_mcp: ".mcp.json",
                mcp_initial: b"{}",
                mcp_reloaded: b"{}",
            },
        }
    }

    pub const fn droid() -> Self {
        Self::droid_with_version("0.171.0")
    }

    pub const fn droid_with_version(version: &'static str) -> Self {
        Self {
            id: "droid",
            version_response: VersionResponse::TextSuffix {
                version,
                suffix: "",
            },
            lifecycle_dialect: LifecycleDialect::Factory,
            managed_projection: Some(ManagedProjectionProfile::new(
                "droid",
                &[],
                Some(".factory/mcp.json"),
                ".factory/skills",
            )),
            conditional_profile: None,
            layout: AcceptanceLayoutSpec {
                global_skill_base: LayoutBase::Factory,
                global_mcp_base: LayoutBase::Factory,
                global_skill: "skills/contract-skill",
                global_mcp: "mcp.json",
                project_skill: ".factory/skills/contract-skill",
                project_mcp: ".factory/mcp.json",
                mcp_initial: br#"{"mcpServers":{"contract":{"command":"contract-server"}}}"#,
                mcp_reloaded: br#"{"mcpServers":{"contract":{"command":"contract-server-v2"}}}"#,
            },
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
            managed_projection: None,
            conditional_profile: None,
            layout: AcceptanceLayoutSpec {
                global_skill_base: LayoutBase::Claude,
                global_mcp_base: LayoutBase::Claude,
                global_skill: "skills/contract-skill",
                global_mcp: "settings.json",
                project_skill: ".claude/skills/contract-skill",
                project_mcp: ".claude/settings.local.json",
                mcp_initial:
                    br#"{\"mcpServers\":{\"contract\":{\"command\":\"contract-server\"}}}"#,
                mcp_reloaded:
                    br#"{\"mcpServers\":{\"contract\":{\"command\":\"contract-server-v2\"}}}"#,
            },
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

    /// Managed fallback acceptance contract, when this harness opts in.
    pub const fn managed_projection(&self) -> Option<&ManagedProjectionProfile> {
        self.managed_projection.as_ref()
    }

    /// Conditional-target layout data, when this harness has separately
    /// observed companion evidence.
    pub const fn conditional_profile(&self) -> Option<&ConditionalTargetProfile> {
        self.conditional_profile.as_ref()
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
        let skill_base = match self.layout.global_skill_base {
            LayoutBase::Home => machine.home(),
            LayoutBase::Codex => machine.codex_home(),
            LayoutBase::Claude => machine.claude_home(),
            LayoutBase::Factory => machine.factory_home(),
        };
        let mcp_base = match self.layout.global_mcp_base {
            LayoutBase::Home => machine.home(),
            LayoutBase::Codex => machine.codex_home(),
            LayoutBase::Claude => machine.claude_home(),
            LayoutBase::Factory => machine.factory_home(),
        };
        Ok(AcceptanceLayout {
            global: ScopeLayout {
                skill_root: skill_base.join(self.layout.global_skill),
                mcp_document: mcp_base.join(self.layout.global_mcp),
            },
            project: ScopeLayout {
                skill_root: project.join(self.layout.project_skill),
                mcp_document: project.join(self.layout.project_mcp),
            },
            mcp_initial: self.layout.mcp_initial,
            mcp_reloaded: self.layout.mcp_reloaded,
        })
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
    use std::thread;

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
    fn codex_claude_and_droid_pass_the_reusable_acceptance_matrix() {
        for profile in [
            FakeHarnessProfile::codex(),
            FakeHarnessProfile::claude(),
            FakeHarnessProfile::droid(),
        ] {
            let machine = IsolatedMachine::new("skilltap-acceptance-matrix").unwrap();
            let report = acceptance_matrix(&profile, &machine).unwrap();
            assert_eq!(report.profile_id(), profile.id());
            assert_eq!(
                report.version_output(),
                profile.version_response().render().as_bytes()
            );
            assert!(report.passed());
        }
        assert_eq!(
            FakeHarnessProfile::codex()
                .managed_projection()
                .map(ManagedProjectionProfile::id),
            Some("codex")
        );
        assert_eq!(
            FakeHarnessProfile::droid()
                .managed_projection()
                .map(ManagedProjectionProfile::id),
            Some("droid")
        );
        assert!(FakeHarnessProfile::claude().managed_projection().is_none());
    }

    #[test]
    fn droid_fixture_preserves_exact_scoped_human_lifecycle_output() {
        let root = TempRoot::new("skilltap-droid-lifecycle-fixture").unwrap();
        let native = FakeHarnessProfile::droid()
            .build(root.path(), FakeNativeMode::VersionKnown)
            .unwrap();
        let empty = native
            .command()
            .args(["plugin", "list", "--scope", "user"])
            .output()
            .unwrap();
        assert_eq!(empty.stdout, b"No plugins installed in user scope.\n");
        assert!(
            native
                .command()
                .args(["plugin", "install", "demo@market", "--scope", "user"])
                .status()
                .unwrap()
                .success()
        );
        let listed = native
            .command()
            .args(["plugin", "list", "--scope", "user"])
            .output()
            .unwrap();
        assert_eq!(
            listed.stdout,
            b"Installed plugins:\nActive:\n  demo@market  [user]  e8801fa\n"
        );
        assert!(
            native
                .command()
                .args(["plugin", "uninstall", "demo@market", "--scope", "user"])
                .status()
                .unwrap()
                .success()
        );
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

    #[test]
    fn profile_publication_survives_parallel_build_and_immediate_exec() {
        const WORKERS: usize = 8;
        const BUILDS_PER_WORKER: usize = 24;

        thread::scope(|scope| {
            let workers = (0..WORKERS)
                .map(|worker| {
                    scope.spawn(move || {
                        for iteration in 0..BUILDS_PER_WORKER {
                            let profile = if (worker + iteration) % 2 == 0 {
                                FakeHarnessProfile::codex()
                            } else {
                                FakeHarnessProfile::claude()
                            };
                            let root = TempRoot::new("skilltap-profile-publication").unwrap();
                            let native = profile
                                .build(root.path(), FakeNativeMode::VersionKnown)
                                .unwrap();
                            let output = native.command().arg("--version").output().unwrap_or_else(
                                |error| {
                                    panic!(
                                        "immediate exec failed for {}: {error}",
                                        native.executable().display()
                                    )
                                },
                            );
                            assert!(output.status.success());
                            assert_eq!(
                                output.stdout,
                                profile.version_response().render().as_bytes()
                            );
                            assert!(output.stderr.is_empty());
                        }
                    })
                })
                .collect::<Vec<_>>();
            for worker in workers {
                worker.join().unwrap();
            }
        });
    }
}
