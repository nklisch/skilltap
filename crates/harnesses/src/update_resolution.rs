//! Non-mutating revision resolvers for foreground and daemon update checks.

use std::{collections::BTreeMap, ffi::OsString, str};

use skilltap_core::{
    domain::{
        ConfiguredBinary, DesiredResource, ExecutableIdentity, GitCommit, HarnessId,
        HarnessObservationOutcome, NativeId, ObservationLayer, ObservedEnvironment,
        ResolvedRevision, Source, SourceKind,
    },
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessRequest, NativeProcessRunner,
        ProcessLimits, SystemExecutableResolver, SystemNativeProcessRunner,
    },
    updates::{NativeRevisionResolver, ResolutionError, SourceRevisionResolver},
};

/// Resolves a Git ref through a bounded `git ls-remote` invocation. It never
/// checks out a tree or mutates a managed source cache.
pub struct GitSourceRevisionResolver<R> {
    runner: R,
    executable: ExecutableIdentity,
    limits: ProcessLimits,
}

impl GitSourceRevisionResolver<SystemNativeProcessRunner> {
    pub fn system(limits: ProcessLimits) -> Result<Self, ResolutionError> {
        let configured = ConfiguredBinary::path_lookup(
            NativeId::new("git").map_err(|_| ResolutionError::UnreachableSource)?,
        )
        .map_err(|_| ResolutionError::UnreachableSource)?;
        let executable = SystemExecutableResolver
            .resolve(&ExecutableResolutionRequest::new(
                configured,
                std::env::var_os("PATH"),
            ))
            .map_err(|_| ResolutionError::UnreachableSource)?;
        Ok(Self {
            runner: SystemNativeProcessRunner,
            executable,
            limits,
        })
    }
}

impl<R> GitSourceRevisionResolver<R> {
    pub const fn new(runner: R, executable: ExecutableIdentity, limits: ProcessLimits) -> Self {
        Self {
            runner,
            executable,
            limits,
        }
    }
}

impl<R: NativeProcessRunner> SourceRevisionResolver for GitSourceRevisionResolver<R> {
    fn resolve(&self, source: &Source) -> Result<ResolvedRevision, ResolutionError> {
        if source.kind() != SourceKind::Git {
            return Err(ResolutionError::UnsupportedSourceKind(source.kind()));
        }
        let requested = source
            .requested_revision()
            .map_or("HEAD", |revision| revision.as_str());
        let output = self
            .runner
            .run(&NativeProcessRequest::new(
                self.executable.clone(),
                [
                    OsString::from("ls-remote"),
                    OsString::from(source.locator().as_str()),
                    OsString::from(requested),
                ],
                BTreeMap::new(),
                None,
                self.limits,
            ))
            .map_err(|_| ResolutionError::UnreachableSource)?;
        if !output.status().success() {
            return Err(ResolutionError::UnreachableSource);
        }
        parse_git_ls_remote(output.stdout())
    }
}

/// Resolves native revisions from one fresh normalized observation snapshot.
pub struct ObservedNativeRevisionResolver<'a> {
    environment: &'a ObservedEnvironment,
}

impl<'a> ObservedNativeRevisionResolver<'a> {
    pub const fn new(environment: &'a ObservedEnvironment) -> Self {
        Self { environment }
    }
}

impl NativeRevisionResolver for ObservedNativeRevisionResolver<'_> {
    fn resolve(
        &self,
        resource: &DesiredResource,
        target: &HarnessId,
    ) -> Result<Option<ResolvedRevision>, ResolutionError> {
        let mut selected: Option<(ObservationLayer, ResolvedRevision)> = None;
        let mut saw_target = false;
        for (_, outcome) in self.environment.iter() {
            if outcome.target().harness() != target || outcome.request().scope() != resource.scope()
            {
                continue;
            }
            saw_target = true;
            let HarnessObservationOutcome::Observed { observation } = outcome else {
                return Err(ResolutionError::NativeObservationUnavailable);
            };
            for observed in observation.resources().values() {
                if observed.key().resource() != resource.key() {
                    continue;
                }
                let Some(revision) = observed.revision() else {
                    continue;
                };
                let candidate = (observed.key().layer(), revision.clone());
                match &selected {
                    None => selected = Some(candidate),
                    Some((layer, existing)) if candidate.0 > *layer => selected = Some(candidate),
                    Some((layer, existing))
                        if candidate.0 == *layer && existing != &candidate.1 =>
                    {
                        return Err(ResolutionError::TargetDisagreement);
                    }
                    Some(_) => {}
                }
            }
        }
        if !saw_target {
            return Err(ResolutionError::NativeObservationUnavailable);
        }
        Ok(selected.map(|(_, revision)| revision))
    }
}

fn parse_git_ls_remote(output: &[u8]) -> Result<ResolvedRevision, ResolutionError> {
    let text = str::from_utf8(output).map_err(|_| ResolutionError::InvalidRequestedRevision)?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(line) = lines.next() else {
        return Err(ResolutionError::InvalidRequestedRevision);
    };
    if lines.next().is_some() {
        return Err(ResolutionError::InvalidRequestedRevision);
    }
    let Some(sha) = line.split_whitespace().next() else {
        return Err(ResolutionError::InvalidRequestedRevision);
    };
    GitCommit::new(sha.to_owned())
        .map(ResolvedRevision::GitCommit)
        .map_err(|_| ResolutionError::InvalidRequestedRevision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_output_requires_one_valid_commit() {
        let sha = "a".repeat(40);
        assert_eq!(
            parse_git_ls_remote(format!("{sha}\trefs/heads/main\n").as_bytes()),
            Ok(ResolvedRevision::GitCommit(GitCommit::new(&sha).unwrap()))
        );
        assert_eq!(
            parse_git_ls_remote(format!("{sha}\n{sha}\n").as_bytes()),
            Err(ResolutionError::InvalidRequestedRevision)
        );
        assert_eq!(
            parse_git_ls_remote(b"not-a-sha\trefs/heads/main\n"),
            Err(ResolutionError::InvalidRequestedRevision)
        );
    }
}
