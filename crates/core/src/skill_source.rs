//! Explicit standalone skill source validation.

use std::fmt;

use crate::domain::{
    AbsolutePath, NativeId, RelativeArtifactPath, Source, SourceKind, SourceLocator,
    ValidationError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillSourceRequest {
    locator: SourceLocator,
    kind: SourceKind,
    requested_revision: Option<crate::domain::RequestedRevision>,
    subdirectory: Option<RelativeArtifactPath>,
    expected_name: Option<NativeId>,
}

impl SkillSourceRequest {
    pub fn new(
        kind: SourceKind,
        locator: SourceLocator,
        requested_revision: Option<crate::domain::RequestedRevision>,
        subdirectory: Option<RelativeArtifactPath>,
        expected_name: Option<NativeId>,
    ) -> Result<Self, SkillSourceError> {
        Source::new_with_subdirectory(
            kind,
            locator.clone(),
            requested_revision.clone(),
            subdirectory.clone(),
        )
        .map_err(SkillSourceError::InvalidSource)?;
        Ok(Self {
            locator,
            kind,
            requested_revision,
            subdirectory,
            expected_name,
        })
    }

    pub const fn kind(&self) -> SourceKind {
        self.kind
    }
    pub const fn locator(&self) -> &SourceLocator {
        &self.locator
    }
    pub const fn requested_revision(&self) -> Option<&crate::domain::RequestedRevision> {
        self.requested_revision.as_ref()
    }
    pub const fn subdirectory(&self) -> Option<&RelativeArtifactPath> {
        self.subdirectory.as_ref()
    }
    pub const fn expected_name(&self) -> Option<&NativeId> {
        self.expected_name.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSkillSource {
    source: Source,
    subdirectory: Option<RelativeArtifactPath>,
    expected_name: Option<NativeId>,
}

impl ResolvedSkillSource {
    pub fn from_request(request: SkillSourceRequest) -> Result<Self, SkillSourceError> {
        let source = Source::new_with_subdirectory(
            request.kind,
            request.locator,
            request.requested_revision,
            request.subdirectory.clone(),
        )
        .map_err(SkillSourceError::InvalidSource)?;
        Ok(Self {
            source,
            subdirectory: request.subdirectory,
            expected_name: request.expected_name,
        })
    }

    pub const fn source(&self) -> &Source {
        &self.source
    }
    pub const fn subdirectory(&self) -> Option<&RelativeArtifactPath> {
        self.subdirectory.as_ref()
    }
    pub const fn expected_name(&self) -> Option<&NativeId> {
        self.expected_name.as_ref()
    }

    /// Resolve a local source root without touching the filesystem. The
    /// filesystem observer owns existence and tree-integrity checks.
    pub fn local_root(&self) -> Result<Option<AbsolutePath>, SkillSourceError> {
        if self.source.kind() != SourceKind::Local {
            return Ok(None);
        }
        let mut root = self.source.locator().as_str().to_owned();
        if let Some(subdirectory) = &self.subdirectory {
            root.push('/');
            root.push_str(subdirectory.as_str());
        }
        AbsolutePath::new(root)
            .map(Some)
            .map_err(SkillSourceError::InvalidPath)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillSourceError {
    InvalidSource(ValidationError),
    InvalidPath(ValidationError),
}

impl fmt::Display for SkillSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource(error) => write!(formatter, "invalid skill source: {error}"),
            Self::InvalidPath(error) => write!(formatter, "invalid local skill path: {error}"),
        }
    }
}

impl std::error::Error for SkillSourceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RequestedRevision, SourceKind};

    #[test]
    fn local_sources_keep_explicit_subdirectory_and_name() {
        let request = SkillSourceRequest::new(
            SourceKind::Local,
            SourceLocator::new("/tmp/skills/demo").unwrap(),
            None,
            Some(RelativeArtifactPath::new("nested").unwrap()),
            Some(NativeId::new("demo").unwrap()),
        )
        .unwrap();
        let resolved = ResolvedSkillSource::from_request(request).unwrap();
        assert_eq!(
            resolved.local_root().unwrap().unwrap().as_str(),
            "/tmp/skills/demo/nested"
        );
        assert_eq!(resolved.source().subdirectory().unwrap().as_str(), "nested");
        assert_eq!(resolved.expected_name().unwrap().as_str(), "demo");
    }

    #[test]
    fn local_sources_reject_revision_and_git_sources_do_not_become_local_paths() {
        assert!(
            SkillSourceRequest::new(
                SourceKind::Local,
                SourceLocator::new("/tmp/demo").unwrap(),
                Some(RequestedRevision::new("main").unwrap()),
                None,
                None,
            )
            .is_err()
        );
        let request = SkillSourceRequest::new(
            SourceKind::Git,
            SourceLocator::new("https://example.invalid/skills.git").unwrap(),
            Some(RequestedRevision::new("main").unwrap()),
            None,
            None,
        )
        .unwrap();
        let resolved = ResolvedSkillSource::from_request(request).unwrap();
        assert!(resolved.local_root().unwrap().is_none());
    }
}
