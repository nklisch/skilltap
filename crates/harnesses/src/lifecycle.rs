//! Native lifecycle command vectors for verified Codex and Claude contracts.

use std::{ffi::OsString, fmt};

use skilltap_core::{
    domain::{ConfiguredBinary, NativeId, Scope, SourceLocator},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessOutput, NativeProcessRequest,
        NativeProcessRunner, ObservationRuntimeError, ProcessLimits, SystemExecutableResolver,
        SystemNativeProcessRunner,
    },
};

use crate::HarnessKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeLifecycleAction {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLifecycleRequest {
    pub harness: HarnessKind,
    pub action: NativeLifecycleAction,
    pub scope: Scope,
    pub name: NativeId,
    pub source: Option<SourceLocator>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeLifecycleError {
    MissingSource,
    UnsupportedProjectScope,
    Runtime(ObservationRuntimeError),
}

impl fmt::Display for NativeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingSource => "marketplace add requires an explicit source",
            Self::UnsupportedProjectScope => {
                "the native harness has no verified project-scoped lifecycle command"
            }
            Self::Runtime(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for NativeLifecycleError {}

impl From<ObservationRuntimeError> for NativeLifecycleError {
    fn from(error: ObservationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// Build a direct native argument vector. The caller still owns executable
/// resolution, profile authority, bounded execution, and post-mutation
/// observation; this function never shells out.
pub fn native_arguments(
    request: &NativeLifecycleRequest,
) -> Result<Vec<OsString>, NativeLifecycleError> {
    let project = matches!(request.scope, Scope::Project(_));
    match request.harness {
        HarnessKind::Codex if project => Err(NativeLifecycleError::UnsupportedProjectScope),
        HarnessKind::Claude => {
            let scope = if project { "local" } else { "user" };
            Ok(claude_arguments(request, scope))
        }
        HarnessKind::Codex => Ok(codex_arguments(request)),
    }
}

/// Execute one already-authorized lifecycle vector through the bounded native
/// process boundary. Profile selection and post-mutation observation remain
/// caller responsibilities.
pub fn run_native_lifecycle(
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    request: &NativeLifecycleRequest,
    limits: ProcessLimits,
) -> Result<NativeProcessOutput, NativeLifecycleError> {
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(configured, search_path))?;
    let working_directory = match &request.scope {
        Scope::Global => None,
        Scope::Project(path) => Some(path.clone()),
    };
    Ok(SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable,
        native_arguments(request)?,
        std::collections::BTreeMap::new(),
        working_directory,
        limits,
    ))?)
}

fn codex_arguments(request: &NativeLifecycleRequest) -> Vec<OsString> {
    let mut args = vec![OsString::from("plugin")];
    match request.action {
        NativeLifecycleAction::MarketplaceAdd => {
            args.extend(["marketplace", "add"].into_iter().map(OsString::from));
            args.push(OsString::from(
                request.source.as_ref().expect("validated source").as_str(),
            ));
        }
        NativeLifecycleAction::MarketplaceRemove => args.extend(
            ["marketplace", "remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::MarketplaceUpdate => args.extend(
            ["marketplace", "upgrade", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginInstall => args.extend(
            ["add", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginRemove => args.extend(
            ["remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginUpdate => args.extend(
            ["update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
    }
    args
}

fn claude_arguments(request: &NativeLifecycleRequest, scope: &str) -> Vec<OsString> {
    let mut args = vec![OsString::from("plugin")];
    match request.action {
        NativeLifecycleAction::MarketplaceAdd => {
            args.extend(["marketplace", "add"].into_iter().map(OsString::from));
            args.push(OsString::from(
                request.source.as_ref().expect("validated source").as_str(),
            ));
        }
        NativeLifecycleAction::MarketplaceRemove => args.extend(
            ["marketplace", "remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::MarketplaceUpdate => args.extend(
            ["marketplace", "update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginInstall => args.extend(
            ["install", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginRemove => args.extend(
            ["uninstall", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginUpdate => args.extend(
            ["update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
    }
    args.extend(["--scope", scope].into_iter().map(OsString::from));
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::AbsolutePath;

    fn request(
        harness: HarnessKind,
        action: NativeLifecycleAction,
        scope: Scope,
    ) -> NativeLifecycleRequest {
        NativeLifecycleRequest {
            harness,
            action,
            scope,
            name: NativeId::new("formatter@team").unwrap(),
            source: Some(SourceLocator::new("https://example.invalid/team.git").unwrap()),
        }
    }

    #[test]
    fn native_vectors_use_direct_arguments_and_scope_mapping() {
        let claude = native_arguments(&request(
            HarnessKind::Claude,
            NativeLifecycleAction::PluginInstall,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            claude,
            ["plugin", "install", "formatter@team", "--scope", "user"].map(OsString::from)
        );
        let codex = native_arguments(&request(
            HarnessKind::Codex,
            NativeLifecycleAction::MarketplaceAdd,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            codex,
            [
                "plugin",
                "marketplace",
                "add",
                "https://example.invalid/team.git"
            ]
            .map(OsString::from)
        );
        assert!(matches!(
            native_arguments(&request(
                HarnessKind::Codex,
                NativeLifecycleAction::PluginInstall,
                Scope::Project(AbsolutePath::new("/tmp/project").unwrap())
            )),
            Err(NativeLifecycleError::UnsupportedProjectScope)
        ));
    }
}
