use std::fmt;

/// A regular file in a skilltap-owned artifact tree.
///
/// Only contents and normalized owner-executable intent cross the managed
/// boundary. Source permission bits beyond whether any execute bit is set are
/// intentionally discarded.
#[derive(Clone, Eq, PartialEq)]
pub struct ArtifactFile {
    contents: Vec<u8>,
    executable: bool,
}

impl ArtifactFile {
    pub fn new(contents: impl Into<Vec<u8>>, executable: bool) -> Self {
        Self {
            contents: contents.into(),
            executable,
        }
    }

    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    pub const fn is_executable(&self) -> bool {
        self.executable
    }
}

impl From<Vec<u8>> for ArtifactFile {
    fn from(contents: Vec<u8>) -> Self {
        Self::new(contents, false)
    }
}

impl From<&[u8]> for ArtifactFile {
    fn from(contents: &[u8]) -> Self {
        Self::new(contents, false)
    }
}

impl fmt::Debug for ArtifactFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArtifactFile")
            .field("byte_count", &self.contents.len())
            .field("executable", &self.executable)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_omits_contents_and_reports_normalized_intent() {
        let file = ArtifactFile::new(b"secret".to_vec(), true);
        let rendered = format!("{file:?}");
        assert!(!rendered.contains("secret"));
        assert!(rendered.contains("byte_count: 6"));
        assert!(rendered.contains("executable: true"));
    }
}
