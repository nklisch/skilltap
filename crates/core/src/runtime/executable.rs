//! Canonical executable resolution and last-moment identity revalidation.

use std::{fs, io, path::Path};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use crate::domain::{AbsolutePath, ConfiguredBinary, ExecutableFileIdentity, ExecutableIdentity};

use super::{ExecutableResolutionRequest, ExecutableResolver, ObservationRuntimeError};

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemExecutableResolver;

impl ExecutableResolver for SystemExecutableResolver {
    fn resolve(
        &self,
        request: &ExecutableResolutionRequest,
    ) -> Result<ExecutableIdentity, ObservationRuntimeError> {
        match request.configured_binary() {
            ConfiguredBinary::Absolute(path) => resolve_candidate(Path::new(path.as_str())),
            ConfiguredBinary::PathLookup(name) => {
                let search_path = request
                    .search_path()
                    .ok_or(ObservationRuntimeError::InvalidSearchPath)?;
                let search_path = search_path
                    .to_str()
                    .ok_or(ObservationRuntimeError::InvalidSearchPath)?;
                if search_path.is_empty() {
                    return Err(ObservationRuntimeError::InvalidSearchPath);
                }

                let directories = std::env::split_paths(search_path).collect::<Vec<_>>();
                for directory in &directories {
                    if directory.as_os_str().is_empty()
                        || !directory.is_absolute()
                        || directory == Path::new(".")
                    {
                        return Err(ObservationRuntimeError::InvalidSearchPath);
                    }
                }
                for directory in directories {
                    let candidate = directory.join(name.as_str());
                    match fs::symlink_metadata(&candidate) {
                        Ok(_) => return resolve_candidate(&candidate),
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => return Err(map_resolution_error(&error)),
                    }
                }
                Err(ObservationRuntimeError::ExecutableNotFound)
            }
        }
    }

    /// Narrows the replacement race immediately before spawn. A path can still
    /// change between this check and the operating system's exec boundary.
    fn revalidate(&self, executable: &ExecutableIdentity) -> Result<(), ObservationRuntimeError> {
        let canonical = fs::canonicalize(executable.path().as_str()).map_err(|error| {
            if error.kind() == io::ErrorKind::PermissionDenied {
                ObservationRuntimeError::ExecutableInaccessible
            } else {
                ObservationRuntimeError::ExecutableChanged
            }
        })?;
        if canonical.to_str() != Some(executable.path().as_str()) {
            return Err(ObservationRuntimeError::ExecutableChanged);
        }
        let metadata = fs::metadata(&canonical).map_err(|error| {
            if error.kind() == io::ErrorKind::PermissionDenied {
                ObservationRuntimeError::ExecutableInaccessible
            } else {
                ObservationRuntimeError::ExecutableChanged
            }
        })?;
        if !metadata.is_file() || !is_executable(&metadata) {
            return Err(ObservationRuntimeError::ExecutableChanged);
        }
        if file_identity(&metadata) != executable.file() {
            return Err(ObservationRuntimeError::ExecutableChanged);
        }
        Ok(())
    }
}

fn resolve_candidate(candidate: &Path) -> Result<ExecutableIdentity, ObservationRuntimeError> {
    let canonical = fs::canonicalize(candidate).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ObservationRuntimeError::ExecutableNotFound,
        io::ErrorKind::PermissionDenied => ObservationRuntimeError::ExecutableInaccessible,
        _ => ObservationRuntimeError::ExecutableResolutionFailed,
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| map_resolution_error(&error))?;
    if !metadata.is_file() {
        return Err(ObservationRuntimeError::ExecutableNotRegular);
    }
    if !is_executable(&metadata) {
        return Err(ObservationRuntimeError::ExecutableNotRunnable);
    }
    let canonical = canonical
        .to_str()
        .ok_or(ObservationRuntimeError::ExecutableResolutionFailed)?;
    let path = AbsolutePath::new(canonical)
        .map_err(|_| ObservationRuntimeError::ExecutableResolutionFailed)?;
    Ok(ExecutableIdentity::new(path, file_identity(&metadata)))
}

fn map_resolution_error(error: &io::Error) -> ObservationRuntimeError {
    match error.kind() {
        io::ErrorKind::NotFound => ObservationRuntimeError::ExecutableNotFound,
        io::ErrorKind::PermissionDenied => ObservationRuntimeError::ExecutableInaccessible,
        _ => ObservationRuntimeError::ExecutableResolutionFailed,
    }
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(unix)]
fn file_identity(metadata: &fs::Metadata) -> ExecutableFileIdentity {
    ExecutableFileIdentity::new(metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
compile_error!("executable resolution is supported only on Unix platforms");

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt};

    use skilltap_test_support::{FakeNativeMode, FakeNativeProcess, TempRoot};

    use super::*;
    use crate::domain::NativeId;

    fn lookup(name: &str, path: OsString) -> ExecutableResolutionRequest {
        ExecutableResolutionRequest::new(
            ConfiguredBinary::path_lookup(NativeId::new(name).unwrap()).unwrap(),
            Some(path),
        )
    }

    fn absolute(path: &Path) -> ExecutableResolutionRequest {
        ExecutableResolutionRequest::new(
            ConfiguredBinary::absolute(AbsolutePath::new(path.to_str().unwrap()).unwrap()),
            None,
        )
    }

    fn copy_executable(source: &Path, destination: &Path) {
        fs::copy(source, destination).unwrap();
        let mut permissions = fs::metadata(destination).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions).unwrap();
    }

    #[test]
    fn absolute_resolution_returns_canonical_path_and_exact_file_identity() {
        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let expected_path = fs::canonicalize(fixture.executable()).unwrap();
        let metadata = fs::metadata(&expected_path).unwrap();

        let resolved = SystemExecutableResolver
            .resolve(&absolute(fixture.executable()))
            .unwrap();

        assert_eq!(resolved.path().as_str(), expected_path.to_str().unwrap());
        assert_eq!(resolved.file().device(), metadata.dev());
        assert_eq!(resolved.file().inode(), metadata.ino());
        SystemExecutableResolver.revalidate(&resolved).unwrap();
    }

    #[test]
    fn path_lookup_uses_explicit_order_and_missing_directories_fall_through() {
        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let root = TempRoot::new("skilltap-executable-path").unwrap();
        let missing = root.join("missing");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        copy_executable(fixture.executable(), &first.join("codex"));
        copy_executable(fixture.executable(), &second.join("codex"));
        let path = std::env::join_paths([&missing, &second, &first]).unwrap();

        let resolved = SystemExecutableResolver
            .resolve(&lookup("codex", path))
            .unwrap();

        assert_eq!(
            resolved.path().as_str(),
            fs::canonicalize(second.join("codex"))
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn final_symlinks_resolve_to_the_regular_executable() {
        use std::os::unix::fs::symlink;

        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let root = TempRoot::new("skilltap-executable-symlink").unwrap();
        let link = root.join("codex");
        symlink(fixture.executable(), &link).unwrap();

        let resolved = SystemExecutableResolver.resolve(&absolute(&link)).unwrap();

        assert_eq!(
            resolved.path().as_str(),
            fs::canonicalize(fixture.executable())
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn replacement_is_detected_by_identity_revalidation() {
        let first = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let second = FakeNativeProcess::new(FakeNativeMode::Exit(1)).unwrap();
        let root = TempRoot::new("skilltap-executable-replacement").unwrap();
        let candidate = root.join("codex");
        copy_executable(first.executable(), &candidate);
        let resolved = SystemExecutableResolver
            .resolve(&absolute(&candidate))
            .unwrap();

        fs::remove_file(&candidate).unwrap();
        copy_executable(second.executable(), &candidate);

        assert_eq!(
            SystemExecutableResolver.revalidate(&resolved),
            Err(ObservationRuntimeError::ExecutableChanged)
        );
    }

    #[test]
    fn invalid_search_paths_fail_without_implicit_current_directory() {
        for path in [
            None,
            Some(OsString::new()),
            Some(OsString::from(":")),
            Some(OsString::from(".")),
            Some(OsString::from("relative/bin")),
            Some(OsString::from_vec(vec![b'/', b't', b'm', b'p', 0xff])),
        ] {
            let request = ExecutableResolutionRequest::new(
                ConfiguredBinary::path_lookup(NativeId::new("codex").unwrap()).unwrap(),
                path,
            );
            assert_eq!(
                SystemExecutableResolver.resolve(&request),
                Err(ObservationRuntimeError::InvalidSearchPath)
            );
        }

        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let root = TempRoot::new("skilltap-invalid-path-tail").unwrap();
        copy_executable(fixture.executable(), &root.join("codex"));
        for path in [
            OsString::from(format!("{}:", root.display())),
            OsString::from(format!("{}:.", root.display())),
        ] {
            assert_eq!(
                SystemExecutableResolver.resolve(&lookup("codex", path)),
                Err(ObservationRuntimeError::InvalidSearchPath)
            );
        }
    }

    #[test]
    fn missing_non_file_and_non_executable_candidates_are_distinct() {
        let root = TempRoot::new("skilltap-executable-invalid").unwrap();
        let missing = root.join("missing");
        assert_eq!(
            SystemExecutableResolver.resolve(&absolute(&missing)),
            Err(ObservationRuntimeError::ExecutableNotFound)
        );

        let directory = root.join("directory");
        fs::create_dir(&directory).unwrap();
        assert_eq!(
            SystemExecutableResolver.resolve(&absolute(&directory)),
            Err(ObservationRuntimeError::ExecutableNotRegular)
        );

        let file = root.join("file");
        fs::write(&file, b"not executable").unwrap();
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&file, permissions).unwrap();
        assert_eq!(
            SystemExecutableResolver.resolve(&absolute(&file)),
            Err(ObservationRuntimeError::ExecutableNotRunnable)
        );
    }

    #[test]
    fn first_existing_path_candidate_is_authoritative() {
        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(0)).unwrap();
        let root = TempRoot::new("skilltap-executable-precedence").unwrap();
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("codex"), b"blocked").unwrap();
        copy_executable(fixture.executable(), &second.join("codex"));

        assert_eq!(
            SystemExecutableResolver.resolve(&lookup(
                "codex",
                std::env::join_paths([first, second]).unwrap(),
            )),
            Err(ObservationRuntimeError::ExecutableNotRunnable)
        );
    }

    #[test]
    fn permission_and_path_errors_never_render_caller_paths() {
        const SECRET: &str = "secret-executable-canary";
        let root = TempRoot::new(SECRET).unwrap();
        let directory = root.join(SECRET);
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("codex"), b"blocked").unwrap();
        let mut permissions = fs::metadata(&directory).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&directory, permissions).unwrap();

        let result = SystemExecutableResolver.resolve(&lookup(
            "codex",
            std::env::join_paths([&directory]).unwrap(),
        ));
        let mut permissions = fs::metadata(&directory).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&directory, permissions).unwrap();

        let error = if unsafe { libc::geteuid() } == 0 {
            ObservationRuntimeError::ExecutableInaccessible
        } else {
            assert_eq!(result, Err(ObservationRuntimeError::ExecutableInaccessible));
            result.unwrap_err()
        };
        assert!(!error.to_string().contains(SECRET));
        assert!(!format!("{error:?}").contains(SECRET));
        assert!(!serde_json::to_string(&error).unwrap().contains(SECRET));
    }
}
