//! Bounded readers for Pi's documented settings files.
//!
//! The reader keeps only the two compound-profile package identities and the
//! hook activation bit. Unknown settings are intentionally discarded at this
//! boundary so native configuration and secrets cannot become profile output.

use std::path::Path;

use serde_json::Value;
use skilltap_core::{
    domain::RelativeArtifactPath,
    runtime::{ConfinedFileSystem, JsonLimits, PlatformPaths, StrictJson, StrictJsonDecoder},
};

use crate::conditional_profile::ConditionalProfileContext;

const GLOBAL_SETTINGS: &str = "settings.json";
const PROJECT_SETTINGS: &str = ".pi/settings.json";
const MCP_PACKAGE: &str = "npm:pi-mcp-adapter";
const HOOK_PACKAGE: &str = "npm:@hsingjui/pi-hooks";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SettingsReadState {
    Missing,
    Present,
    Malformed,
    Unreadable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PackageDeclaration {
    Absent,
    Declared { autoload: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HookDeclaration {
    Absent,
    Configured,
}

impl HookDeclaration {
    pub(super) const fn is_configured(self) -> bool {
        matches!(self, Self::Configured)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SettingsSnapshot {
    pub(super) state: SettingsReadState,
    pub(super) mcp_package: PackageDeclaration,
    pub(super) hook_package: PackageDeclaration,
    pub(super) hooks: HookDeclaration,
}

impl SettingsSnapshot {
    const fn missing() -> Self {
        Self {
            state: SettingsReadState::Missing,
            mcp_package: PackageDeclaration::Absent,
            hook_package: PackageDeclaration::Absent,
            hooks: HookDeclaration::Absent,
        }
    }

    const fn unreadable() -> Self {
        Self {
            state: SettingsReadState::Unreadable,
            ..Self::missing()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PiSettings {
    pub(super) global: SettingsSnapshot,
    pub(super) project: Option<SettingsSnapshot>,
}

pub(super) fn read(context: &ConditionalProfileContext<'_>) -> PiSettings {
    let global = read_document(
        context.filesystem,
        context.paths.pi_home(),
        GLOBAL_SETTINGS,
        context.json_limits,
    );
    let project = match context.scope {
        skilltap_core::domain::Scope::Global => None,
        skilltap_core::domain::Scope::Project(project) => Some(read_document(
            context.filesystem,
            project,
            PROJECT_SETTINGS,
            context.json_limits,
        )),
    };
    PiSettings { global, project }
}

pub(super) fn package_manifest_path(package: &str) -> RelativeArtifactPath {
    RelativeArtifactPath::new(format!("node_modules/{package}/package.json"))
        .expect("compiled Pi package manifest path is valid")
}

pub(super) fn package_root(
    paths: &PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    declared_global: bool,
) -> Option<skilltap_core::domain::AbsolutePath> {
    if declared_global {
        return Some(paths.pi_package_dir().clone());
    }
    match scope {
        skilltap_core::domain::Scope::Global => None,
        skilltap_core::domain::Scope::Project(project) => {
            skilltap_core::domain::AbsolutePath::new(format!("{}/.pi/npm", project.as_str())).ok()
        }
    }
}

fn read_document(
    filesystem: &dyn ConfinedFileSystem,
    root: &skilltap_core::domain::AbsolutePath,
    destination: &str,
    limits: JsonLimits,
) -> SettingsSnapshot {
    let destination = RelativeArtifactPath::new(destination).expect("static Pi settings path");
    let bytes = match filesystem.read_regular_bounded_no_follow(root, &destination, limits.bytes())
    {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return SettingsSnapshot::missing(),
        Err(_) if !root_is_present(root) => return SettingsSnapshot::missing(),
        Err(_) => return SettingsSnapshot::unreadable(),
    };
    let value = match StrictJson.decode(&bytes, limits) {
        Ok(decoded) => decoded.into_value(),
        Err(_) => {
            return SettingsSnapshot {
                state: SettingsReadState::Malformed,
                ..SettingsSnapshot::missing()
            };
        }
    };
    let Some(object) = value.as_object() else {
        return SettingsSnapshot {
            state: SettingsReadState::Malformed,
            ..SettingsSnapshot::missing()
        };
    };

    let (mcp_package, hook_package) = match object.get("packages") {
        None => (PackageDeclaration::Absent, PackageDeclaration::Absent),
        Some(packages) => match parse_packages(packages) {
            Ok(packages) => packages,
            Err(()) => {
                return SettingsSnapshot {
                    state: SettingsReadState::Malformed,
                    ..SettingsSnapshot::missing()
                };
            }
        },
    };
    let hooks = match object.get("hooks") {
        None => HookDeclaration::Absent,
        Some(hooks) => match parse_hooks(hooks) {
            Ok(hooks) => hooks,
            Err(()) => {
                return SettingsSnapshot {
                    state: SettingsReadState::Malformed,
                    ..SettingsSnapshot::missing()
                };
            }
        },
    };
    SettingsSnapshot {
        state: SettingsReadState::Present,
        mcp_package,
        hook_package,
        hooks,
    }
}

fn parse_packages(value: &Value) -> Result<(PackageDeclaration, PackageDeclaration), ()> {
    let packages = value.as_array().ok_or(())?;
    let mut mcp = PackageDeclaration::Absent;
    let mut hooks = PackageDeclaration::Absent;
    for package in packages {
        let (source, autoload) = match package {
            Value::String(source) => (source.as_str(), true),
            Value::Object(object) => {
                let source = object.get("source").and_then(Value::as_str).ok_or(())?;
                let autoload = match object.get("autoload") {
                    None => true,
                    Some(value) => value.as_bool().ok_or(())?,
                };
                (source, autoload)
            }
            _ => return Err(()),
        };
        match source {
            MCP_PACKAGE => mcp = PackageDeclaration::Declared { autoload },
            HOOK_PACKAGE => hooks = PackageDeclaration::Declared { autoload },
            _ => {}
        }
    }
    Ok((mcp, hooks))
}

fn parse_hooks(value: &Value) -> Result<HookDeclaration, ()> {
    let hooks = value.as_object().ok_or(())?;
    let mut configured = false;
    for groups in hooks.values() {
        let groups = groups.as_array().ok_or(())?;
        configured |= !groups.is_empty();
    }
    Ok(if configured {
        HookDeclaration::Configured
    } else {
        HookDeclaration::Absent
    })
}

fn root_is_present(root: &skilltap_core::domain::AbsolutePath) -> bool {
    match std::fs::symlink_metadata(Path::new(root.as_str())) {
        Ok(metadata) => !metadata.file_type().is_symlink(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::runtime::{
        Environment, EnvironmentVariable, SupportedPlatform, SystemFileSystem,
    };
    use skilltap_test_support::TempRoot;
    use std::{ffi::OsString, fs};

    struct TestEnvironment {
        home: String,
    }

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            (variable == EnvironmentVariable::Home).then(|| self.home.clone().into())
        }
    }

    fn paths(root: &TempRoot) -> PlatformPaths {
        PlatformPaths::resolve_for(
            SupportedPlatform::Linux,
            &TestEnvironment {
                home: root.join("home").to_string_lossy().into_owned(),
            },
        )
        .unwrap()
    }

    fn snapshot(root: &TempRoot, contents: &str) -> SettingsSnapshot {
        let paths = paths(root);
        let pi_home = std::path::Path::new(paths.pi_home().as_str());
        fs::create_dir_all(pi_home).unwrap();
        fs::write(pi_home.join("settings.json"), contents).unwrap();
        read_document(
            &SystemFileSystem,
            paths.pi_home(),
            GLOBAL_SETTINGS,
            JsonLimits::new(16 * 1024, 16).unwrap(),
        )
    }

    #[test]
    fn settings_reader_extracts_only_supported_package_declarations() {
        let root = TempRoot::new("pi-settings-reader").unwrap();
        let snapshot = snapshot(
            &root,
            r#"{"packages":["npm:pi-mcp-adapter",{"source":"npm:@hsingjui/pi-hooks","autoload":false},"npm:unrelated"],"future":{"token":"secret"}}"#,
        );
        assert_eq!(snapshot.state, SettingsReadState::Present);
        assert_eq!(
            snapshot.mcp_package,
            PackageDeclaration::Declared { autoload: true }
        );
        assert_eq!(
            snapshot.hook_package,
            PackageDeclaration::Declared { autoload: false }
        );
    }

    #[test]
    fn malformed_settings_fail_closed_without_rendering_payload() {
        let root = TempRoot::new("pi-settings-malformed").unwrap();
        let snapshot = snapshot(&root, r#"{"packages":{"token":"secret"}}"#);
        assert_eq!(snapshot.state, SettingsReadState::Malformed);
        assert!(!format!("{snapshot:?}").contains("secret"));
    }

    #[test]
    fn hook_reader_distinguishes_absent_empty_and_configured_arrays() {
        let root = TempRoot::new("pi-settings-hooks").unwrap();
        assert_eq!(
            snapshot(&root, r#"{"hooks":{}}"#).hooks,
            HookDeclaration::Absent
        );
        let root = TempRoot::new("pi-settings-hooks-empty").unwrap();
        assert_eq!(
            snapshot(&root, r#"{"hooks":{"PreToolUse":[]}}"#).hooks,
            HookDeclaration::Absent
        );
        let root = TempRoot::new("pi-settings-hooks-configured").unwrap();
        assert_eq!(
            snapshot(&root, r#"{"hooks":{"PreToolUse":[{"matcher":"*"}]}}"#).hooks,
            HookDeclaration::Configured
        );
    }

    #[test]
    fn project_settings_are_read_separately_from_global_settings() {
        let root = TempRoot::new("pi-settings-scopes").unwrap();
        let paths = paths(&root);
        fs::create_dir_all(paths.pi_home().as_str()).unwrap();
        fs::write(
            std::path::Path::new(paths.pi_home().as_str()).join("settings.json"),
            br#"{"packages":["npm:pi-mcp-adapter"],"hooks":{"PreToolUse":[]}}"#,
        )
        .unwrap();
        let project = root.join("project");
        fs::create_dir_all(project.join(".pi")).unwrap();
        fs::write(
            project.join(".pi/settings.json"),
            br#"{"packages":["npm:@hsingjui/pi-hooks"],"hooks":{"Stop":[{}]}}"#,
        )
        .unwrap();
        let project = skilltap_core::domain::AbsolutePath::new(project.to_str().unwrap()).unwrap();
        let context = ConditionalProfileContext {
            scope: &skilltap_core::domain::Scope::Project(project),
            paths: &paths,
            filesystem: &SystemFileSystem,
            json_limits: JsonLimits::new(16 * 1024, 16).unwrap(),
            maximum_manifest_bytes: 16 * 1024,
        };
        let settings = read(&context);
        assert!(matches!(
            settings.global.mcp_package,
            PackageDeclaration::Declared { .. }
        ));
        assert!(matches!(
            settings.project.unwrap().hook_package,
            PackageDeclaration::Declared { .. }
        ));
    }

    #[test]
    fn package_manifest_paths_are_confined_and_scoped() {
        assert_eq!(
            package_manifest_path("@hsingjui/pi-hooks").as_str(),
            "node_modules/@hsingjui/pi-hooks/package.json"
        );
    }
}
