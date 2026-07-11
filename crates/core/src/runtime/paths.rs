use std::{ffi::OsString, path::PathBuf};

use crate::domain::AbsolutePath;

use super::{EnvironmentVariable, PathRole, RuntimeError, path_value::absolute_path};

pub trait Environment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessEnvironment;

impl Environment for ProcessEnvironment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
        std::env::var_os(variable.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportedPlatform {
    Linux,
    MacOs,
}

impl SupportedPlatform {
    pub fn detect(platform: &str) -> Result<Self, RuntimeError> {
        match platform {
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            platform => Err(RuntimeError::UnsupportedPlatform {
                platform: platform.to_owned(),
            }),
        }
    }

    pub fn current() -> Result<Self, RuntimeError> {
        Self::detect(std::env::consts::OS)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformPaths {
    platform: SupportedPlatform,
    home: AbsolutePath,
    config_home: AbsolutePath,
    skilltap_config: AbsolutePath,
    global_agents: AbsolutePath,
    codex_home: AbsolutePath,
    claude_home: AbsolutePath,
}

impl PlatformPaths {
    pub fn resolve(environment: &impl Environment) -> Result<Self, RuntimeError> {
        Self::resolve_for(SupportedPlatform::current()?, environment)
    }

    pub fn resolve_for(
        platform: SupportedPlatform,
        environment: &impl Environment,
    ) -> Result<Self, RuntimeError> {
        let home = required_environment_path(environment, EnvironmentVariable::Home)?;
        let config_home =
            optional_environment_path(environment, EnvironmentVariable::XdgConfigHome)?
                .map_or_else(|| join(&home, ".config", PathRole::ConfigHome), Ok)?;

        Ok(Self {
            platform,
            skilltap_config: join(&config_home, "skilltap", PathRole::SkilltapConfig)?,
            global_agents: join(&home, "AGENTS.md", PathRole::GlobalAgents)?,
            codex_home: join(&home, ".codex", PathRole::CodexHome)?,
            claude_home: join(&home, ".claude", PathRole::ClaudeHome)?,
            home,
            config_home,
        })
    }

    pub const fn platform(&self) -> SupportedPlatform {
        self.platform
    }

    pub const fn home(&self) -> &AbsolutePath {
        &self.home
    }

    pub const fn config_home(&self) -> &AbsolutePath {
        &self.config_home
    }

    pub const fn skilltap_config(&self) -> &AbsolutePath {
        &self.skilltap_config
    }

    pub const fn global_agents(&self) -> &AbsolutePath {
        &self.global_agents
    }

    pub const fn codex_home(&self) -> &AbsolutePath {
        &self.codex_home
    }

    pub const fn claude_home(&self) -> &AbsolutePath {
        &self.claude_home
    }
}

fn required_environment_path(
    environment: &impl Environment,
    variable: EnvironmentVariable,
) -> Result<AbsolutePath, RuntimeError> {
    let value = environment
        .value(variable)
        .filter(|value| !value.is_empty())
        .ok_or(RuntimeError::MissingEnvironment { variable })?;
    parse_environment_path(value, variable)
}

fn optional_environment_path(
    environment: &impl Environment,
    variable: EnvironmentVariable,
) -> Result<Option<AbsolutePath>, RuntimeError> {
    environment
        .value(variable)
        .filter(|value| !value.is_empty())
        .map(|value| parse_environment_path(value, variable))
        .transpose()
}

fn parse_environment_path(
    value: OsString,
    variable: EnvironmentVariable,
) -> Result<AbsolutePath, RuntimeError> {
    let value = value
        .into_string()
        .map_err(|_| RuntimeError::NonUtf8Environment { variable })?;
    AbsolutePath::new(value)
        .map_err(|source| RuntimeError::InvalidEnvironmentPath { variable, source })
}

fn join(base: &AbsolutePath, child: &str, role: PathRole) -> Result<AbsolutePath, RuntimeError> {
    let path = PathBuf::from(base.as_str()).join(child);
    absolute_path(&path, role)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[derive(Default)]
    struct TestEnvironment(BTreeMap<&'static str, OsString>);

    impl TestEnvironment {
        fn with(mut self, variable: EnvironmentVariable, value: impl Into<OsString>) -> Self {
            self.0.insert(variable.as_str(), value.into());
            self
        }
    }

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            self.0.get(variable.as_str()).cloned()
        }
    }

    #[test]
    fn xdg_override_resolves_every_required_location() {
        let environment = TestEnvironment::default()
            .with(EnvironmentVariable::Home, "/home/nathan")
            .with(EnvironmentVariable::XdgConfigHome, "/var/config/nathan");
        let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();

        assert_eq!(paths.home().as_str(), "/home/nathan");
        assert_eq!(paths.config_home().as_str(), "/var/config/nathan");
        assert_eq!(
            paths.skilltap_config().as_str(),
            "/var/config/nathan/skilltap"
        );
        assert_eq!(paths.global_agents().as_str(), "/home/nathan/AGENTS.md");
        assert_eq!(paths.codex_home().as_str(), "/home/nathan/.codex");
        assert_eq!(paths.claude_home().as_str(), "/home/nathan/.claude");
    }

    #[test]
    fn absent_or_empty_xdg_uses_home_config_fallback() {
        for environment in [
            TestEnvironment::default().with(EnvironmentVariable::Home, "/Users/nathan"),
            TestEnvironment::default()
                .with(EnvironmentVariable::Home, "/Users/nathan")
                .with(EnvironmentVariable::XdgConfigHome, ""),
        ] {
            let paths = PlatformPaths::resolve_for(SupportedPlatform::MacOs, &environment).unwrap();
            assert_eq!(paths.config_home().as_str(), "/Users/nathan/.config");
            assert_eq!(
                paths.skilltap_config().as_str(),
                "/Users/nathan/.config/skilltap"
            );
        }
    }

    #[test]
    fn missing_relative_and_noncanonical_environment_paths_fail_fast() {
        let missing =
            PlatformPaths::resolve_for(SupportedPlatform::Linux, &TestEnvironment::default())
                .unwrap_err();
        assert!(matches!(
            missing,
            RuntimeError::MissingEnvironment {
                variable: EnvironmentVariable::Home
            }
        ));

        for (variable, value) in [
            (EnvironmentVariable::Home, "relative/home"),
            (EnvironmentVariable::Home, "/home/nathan/../other"),
            (EnvironmentVariable::XdgConfigHome, "relative/config"),
            (EnvironmentVariable::XdgConfigHome, "/var//config"),
        ] {
            let environment = TestEnvironment::default()
                .with(EnvironmentVariable::Home, "/home/nathan")
                .with(variable, value);
            let error =
                PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap_err();
            assert!(matches!(
                error,
                RuntimeError::InvalidEnvironmentPath {
                    variable: actual,
                    ..
                } if actual == variable
            ));
            assert!(!error.to_string().contains(value));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_environment_paths_are_rejected_without_rendering_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let invalid = OsString::from_vec(vec![b'/', b't', b'm', b'p', 0xff]);
        let environment = TestEnvironment::default()
            .with(EnvironmentVariable::Home, "/home/nathan")
            .with(EnvironmentVariable::XdgConfigHome, invalid);
        let error = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap_err();

        assert!(matches!(
            error,
            RuntimeError::NonUtf8Environment {
                variable: EnvironmentVariable::XdgConfigHome
            }
        ));
        assert_eq!(
            error.to_string(),
            "environment variable `XDG_CONFIG_HOME` is not valid UTF-8"
        );
    }

    #[test]
    fn resolution_does_not_create_paths() {
        let unique = format!(
            "/tmp/skilltap-runtime-path-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let environment = TestEnvironment::default()
            .with(EnvironmentVariable::Home, unique.clone())
            .with(
                EnvironmentVariable::XdgConfigHome,
                format!("{unique}/config"),
            );
        assert!(!std::path::Path::new(&unique).exists());

        let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();

        assert!(!std::path::Path::new(paths.home().as_str()).exists());
        assert!(!std::path::Path::new(paths.skilltap_config().as_str()).exists());
    }

    #[test]
    fn unsupported_platforms_are_explicit() {
        let error = SupportedPlatform::detect("windows").unwrap_err();
        assert_eq!(
            error.boundary(),
            super::super::RuntimeBoundary::UnsupportedPlatform
        );
        assert_eq!(error.to_string(), "unsupported platform `windows`");
    }
}
