use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::IsolatedMachine;

/// Data-only layout for a compound target whose companions are observed but
/// never owned by the test harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConditionalTargetProfile {
    id: &'static str,
    core_version: &'static str,
    pi_home: &'static str,
    global_package_root: &'static str,
    project_package_root: &'static str,
    global_settings: &'static str,
    project_settings: &'static str,
    global_skill_root: &'static str,
    project_skill_root: &'static str,
}

impl ConditionalTargetProfile {
    pub const fn pi() -> Self {
        Self {
            id: "pi",
            core_version: "0.80.6",
            pi_home: ".pi/agent",
            global_package_root: ".pi/agent/npm",
            project_package_root: ".pi/npm",
            global_settings: ".pi/agent/settings.json",
            project_settings: ".pi/settings.json",
            global_skill_root: ".agents/skills",
            project_skill_root: ".agents/skills",
        }
    }

    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn core_version(&self) -> &'static str {
        self.core_version
    }

    pub const fn pi_home(&self) -> &'static str {
        self.pi_home
    }

    pub const fn global_package_root(&self) -> &'static str {
        self.global_package_root
    }

    pub const fn project_package_root(&self) -> &'static str {
        self.project_package_root
    }

    pub const fn global_settings(&self) -> &'static str {
        self.global_settings
    }

    pub const fn project_settings(&self) -> &'static str {
        self.project_settings
    }

    pub const fn global_skill_root(&self) -> &'static str {
        self.global_skill_root
    }

    pub const fn project_skill_root(&self) -> &'static str {
        self.project_skill_root
    }
}

/// Native boundary cases required by the conditional-target acceptance matrix.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConditionalFixtureCase {
    Exact,
    HooksConfigured,
    MissingMcp,
    MissingHooks,
    MismatchedPackage,
    UnknownMcpVersion,
    UnknownHookVersion,
    MalformedSettings,
    MalformedManifest,
    ProjectTrust,
}

impl ConditionalFixtureCase {
    pub const ALL: [Self; 10] = [
        Self::Exact,
        Self::HooksConfigured,
        Self::MissingMcp,
        Self::MissingHooks,
        Self::MismatchedPackage,
        Self::UnknownMcpVersion,
        Self::UnknownHookVersion,
        Self::MalformedSettings,
        Self::MalformedManifest,
        Self::ProjectTrust,
    ];
}

/// A caller-owned, dependency-neutral Pi fixture. The fixture writes only
/// documented native surfaces beneath the supplied isolated machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConditionalTargetFixture {
    profile: ConditionalTargetProfile,
    case: ConditionalFixtureCase,
}

impl ConditionalTargetFixture {
    pub const fn new(profile: ConditionalTargetProfile, case: ConditionalFixtureCase) -> Self {
        Self { profile, case }
    }

    pub const fn pi(case: ConditionalFixtureCase) -> Self {
        Self::new(ConditionalTargetProfile::pi(), case)
    }

    pub const fn profile(&self) -> ConditionalTargetProfile {
        self.profile
    }

    pub const fn case(&self) -> ConditionalFixtureCase {
        self.case
    }

    /// Materialize one global/project fixture without invoking a package
    /// manager, Pi, Node, a browser, or any other external dependency.
    pub fn install(&self, machine: &IsolatedMachine) -> io::Result<ConditionalFixtureRoots> {
        let project = machine.working_directory().join("conditional-project");
        let pi_home = machine.home().join(self.profile.pi_home);
        let global_package_root = machine.home().join(self.profile.global_package_root);
        let project_package_root = project.join(self.profile.project_package_root);
        fs::create_dir_all(&pi_home)?;
        fs::create_dir_all(&project)?;

        let roots = ConditionalFixtureRoots {
            pi_home,
            global_package_root,
            project_package_root,
            global_settings: machine.home().join(self.profile.global_settings),
            project_settings: project.join(self.profile.project_settings),
            canonical_global_skills: machine.home().join(self.profile.global_skill_root),
            canonical_project_skills: project.join(self.profile.project_skill_root),
            project,
        };
        self.write_settings(&roots)?;
        self.write_manifests(&roots)?;
        Ok(roots)
    }

    fn write_settings(&self, roots: &ConditionalFixtureRoots) -> io::Result<()> {
        if self.case == ConditionalFixtureCase::MalformedSettings {
            write_bytes(&roots.global_settings, b"{malformed settings")?;
            return Ok(());
        }

        let hooks = if self.case == ConditionalFixtureCase::HooksConfigured {
            r#"{"PreToolUse":[{"matcher":"*"}]}"#
        } else {
            "{}"
        };
        let global = format!(
            r#"{{"packages":["npm:pi-mcp-adapter","npm:@hsingjui/pi-hooks"],"hooks":{hooks}}}"#
        );
        write_bytes(&roots.global_settings, global.as_bytes())?;

        if self.case == ConditionalFixtureCase::ProjectTrust {
            let project =
                br#"{"packages":[{"source":"npm:@hsingjui/pi-hooks"}],"hooks":{"Stop":[{}]}}"#;
            write_bytes(&roots.project_settings, project)?;
        }
        Ok(())
    }

    fn write_manifests(&self, roots: &ConditionalFixtureRoots) -> io::Result<()> {
        let mcp_manifest = match self.case {
            ConditionalFixtureCase::MissingMcp => None,
            ConditionalFixtureCase::MismatchedPackage => {
                Some(br#"{"name":"wrong-package","version":"2.11.0","pi":{"extensions":["./index.ts"]}}"#.as_slice())
            }
            ConditionalFixtureCase::UnknownMcpVersion => {
                Some(br#"{"name":"pi-mcp-adapter","version":"2.12.0","pi":{"extensions":["./index.ts"]}}"#.as_slice())
            }
            ConditionalFixtureCase::MalformedManifest => Some(b"{malformed manifest".as_slice()),
            _ => Some(
                br#"{"name":"pi-mcp-adapter","version":"2.11.0","pi":{"extensions":["./index.ts"]}}"#.as_slice(),
            ),
        };
        let hook_manifest = match self.case {
            ConditionalFixtureCase::MissingHooks => None,
            ConditionalFixtureCase::UnknownHookVersion => Some(
                br#"{"name":"@hsingjui/pi-hooks","version":"0.0.3","pi":{"extensions":["./src/pi-hooks.ts"]}}"#.as_slice(),
            ),
            _ => Some(
                br#"{"name":"@hsingjui/pi-hooks","version":"0.0.2","pi":{"extensions":["./src/pi-hooks.ts"]}}"#.as_slice(),
            ),
        };
        if let Some(manifest) = mcp_manifest {
            write_package(&roots.global_package_root, "pi-mcp-adapter", manifest)?;
        }
        if let Some(manifest) = hook_manifest {
            let package_root = if self.case == ConditionalFixtureCase::ProjectTrust {
                &roots.project_package_root
            } else {
                &roots.global_package_root
            };
            write_package(package_root, "@hsingjui/pi-hooks", manifest)?;
        }
        Ok(())
    }
}

/// Paths created by [`ConditionalTargetFixture::install`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalFixtureRoots {
    pi_home: PathBuf,
    global_package_root: PathBuf,
    project_package_root: PathBuf,
    global_settings: PathBuf,
    project_settings: PathBuf,
    canonical_global_skills: PathBuf,
    canonical_project_skills: PathBuf,
    project: PathBuf,
}

impl ConditionalFixtureRoots {
    pub fn pi_home(&self) -> &Path {
        &self.pi_home
    }

    pub fn global_package_root(&self) -> &Path {
        &self.global_package_root
    }

    pub fn project_package_root(&self) -> &Path {
        &self.project_package_root
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn global_settings(&self) -> &Path {
        &self.global_settings
    }

    pub fn project_settings(&self) -> &Path {
        &self.project_settings
    }

    pub fn canonical_global_skills(&self) -> &Path {
        &self.canonical_global_skills
    }

    pub fn canonical_project_skills(&self) -> &Path {
        &self.canonical_project_skills
    }

    pub fn native_project_skills(&self) -> PathBuf {
        self.project.join(".pi/skills")
    }
}

fn write_package(root: &Path, package: &str, manifest: &[u8]) -> io::Result<()> {
    let path = root.join("node_modules").join(package).join("package.json");
    write_bytes(&path, manifest)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IsolatedMachine;

    #[test]
    fn matrix_has_distinct_boundary_cases_and_uses_profile_data() {
        assert_eq!(ConditionalFixtureCase::ALL.len(), 10);
        let profile = ConditionalTargetProfile::pi();
        assert_eq!(profile.id(), "pi");
        assert_eq!(profile.core_version(), "0.80.6");
        assert_eq!(profile.global_package_root(), ".pi/agent/npm");
        assert_eq!(profile.project_package_root(), ".pi/npm");
    }

    #[test]
    fn exact_fixture_is_isolated_and_contains_only_documented_companion_surfaces() {
        let machine = IsolatedMachine::new("conditional-fixture").unwrap();
        let roots = ConditionalTargetFixture::pi(ConditionalFixtureCase::Exact)
            .install(&machine)
            .unwrap();
        assert!(roots.global_settings().is_file());
        assert!(
            roots
                .global_package_root()
                .join("node_modules/pi-mcp-adapter/package.json")
                .is_file()
        );
        assert!(
            roots
                .global_package_root()
                .join("node_modules/@hsingjui/pi-hooks/package.json")
                .is_file()
        );
        assert!(!roots.project_settings().exists());
        assert!(!machine.home().join(".agents/skills").exists());
    }

    #[test]
    fn missing_and_malformed_cases_preserve_the_sibling_fixture_surface() {
        for case in [
            ConditionalFixtureCase::MissingMcp,
            ConditionalFixtureCase::MissingHooks,
            ConditionalFixtureCase::MalformedSettings,
            ConditionalFixtureCase::MalformedManifest,
        ] {
            let machine = IsolatedMachine::new("conditional-fixture-boundary").unwrap();
            let roots = ConditionalTargetFixture::pi(case)
                .install(&machine)
                .unwrap();
            assert!(roots.pi_home().is_dir());
            assert!(roots.global_package_root().is_dir());
        }
    }
}
