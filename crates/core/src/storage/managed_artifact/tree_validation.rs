use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::domain::RelativeArtifactPath;

use super::{ArtifactTree, ArtifactTreeError};

pub(super) fn validate<P, B>(
    files: impl IntoIterator<Item = (P, B)>,
) -> Result<BTreeMap<RelativeArtifactPath, Vec<u8>>, ArtifactTreeError>
where
    P: Into<String>,
    B: Into<Vec<u8>>,
{
    let mut collected = BTreeMap::new();
    for (path, contents) in files {
        let raw = path.into();
        let path =
            RelativeArtifactPath::new(raw.clone()).map_err(|_| ArtifactTreeError::InvalidPath)?;
        if collected.insert(path, contents.into()).is_some() {
            return Err(ArtifactTreeError::DuplicatePath { path: raw });
        }
    }
    if collected.is_empty() {
        return Err(ArtifactTreeError::Empty);
    }
    for path in collected.keys() {
        let mut ancestor = Path::new(path.as_str()).parent().map(PathBuf::from);
        while let Some(candidate) = ancestor {
            if candidate.as_os_str().is_empty() {
                break;
            }
            let candidate = RelativeArtifactPath::new(candidate.to_string_lossy().into_owned())
                .map_err(|_| ArtifactTreeError::InvalidPath)?;
            if collected.contains_key(&candidate) {
                return Err(ArtifactTreeError::FileIsAncestor { path: candidate });
            }
            ancestor = Path::new(candidate.as_str()).parent().map(PathBuf::from);
        }
    }
    Ok(collected)
}

impl ArtifactTree {
    pub(super) fn from_validated(
        files: BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<Self, ArtifactTreeError> {
        Self::new(
            files
                .into_iter()
                .map(|(path, contents)| (path.as_str().to_owned(), contents)),
        )
    }
}
