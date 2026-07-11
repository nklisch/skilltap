use std::path::Path;

use crate::domain::AbsolutePath;

use super::{PathRole, RuntimeError};

pub(super) fn absolute_path(path: &Path, role: PathRole) -> Result<AbsolutePath, RuntimeError> {
    let value = path
        .to_str()
        .ok_or(RuntimeError::NonUtf8Path { role })?
        .to_owned();
    AbsolutePath::new(value).map_err(|source| RuntimeError::InvalidPath { role, source })
}
