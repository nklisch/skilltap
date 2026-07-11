use std::{collections::BTreeSet, fs, io, path::Path};

use crate::{
    domain::AbsolutePath,
    runtime::{
        DirectorySyncState, FileSystemAction, PublicationResidual, PublicationResidualRole,
        PublicationResiduals, RuntimeError,
    },
};

use super::unix_identity::{
    FileIdentity, descriptor_identity, descriptor_identity_io, open_read_no_follow, path_identity,
    verify_path_identity,
};
use super::{create_temporary, filesystem_error, sync_parent_io};

pub(super) trait Publication {
    fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    fn sync_parent(&self, destination: &Path) -> io::Result<()>;
}

pub(super) struct SystemPublication;

impl Publication for SystemPublication {
    fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
        fs::hard_link(temporary, destination)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn sync_parent(&self, destination: &Path) -> io::Result<()> {
        sync_parent_io(destination)
    }
}

pub(super) fn copy_recoverable_with(
    source: &AbsolutePath,
    destination: &AbsolutePath,
    publication: &impl Publication,
    after_source_open: impl FnOnce(),
) -> Result<(), RuntimeError> {
    let mut source_file = open_read_no_follow(source)?;
    let source_identity = descriptor_identity(&source_file, FileSystemAction::Copy, source)?;
    after_source_open();
    verify_path_identity(source, source_identity, FileSystemAction::Copy)?;
    if !source_file
        .metadata()
        .map_err(|error| filesystem_error(FileSystemAction::Copy, source, error))?
        .is_file()
    {
        return Err(filesystem_error(
            FileSystemAction::Copy,
            source,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "backup source is not a regular file",
            ),
        ));
    }

    let destination_path = Path::new(destination.as_str());
    let parent = destination_path.parent().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Copy,
            destination,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "destination has no parent directory",
            ),
        )
    })?;
    let name = destination_path.file_name().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Copy,
            destination,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"),
        )
    })?;
    let (temporary_path, mut temporary_file) = create_temporary(parent, name)
        .map_err(|error| filesystem_error(FileSystemAction::Copy, destination, error))?;
    let temporary_identity = match descriptor_identity_io(&temporary_file) {
        Ok(identity) => identity,
        Err(source) => {
            drop(temporary_file);
            return Err(RuntimeError::PartialPublication {
                path: destination.clone(),
                residuals: PublicationResiduals::new(
                    [publication_residual(
                        PublicationResidualRole::Temporary,
                        &temporary_path,
                    )],
                    DirectorySyncState::NotRequired,
                ),
                source,
                cleanup: io::Error::other(
                    "temporary identity unavailable; cleanup could not be proven safe",
                ),
            });
        }
    };

    let staged =
        io::copy(&mut source_file, &mut temporary_file).and_then(|_| temporary_file.sync_all());
    drop(temporary_file);
    if let Err(error) = staged {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = require_path_identity(&temporary_path, temporary_identity) {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = publication.publish_no_clobber(&temporary_path, destination_path) {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = require_path_identity(destination_path, temporary_identity) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            Some((&temporary_path, temporary_identity)),
            error,
            publication,
        ));
    }

    if let Err(error) = remove_if_identity(&temporary_path, temporary_identity, publication) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            Some((&temporary_path, temporary_identity)),
            error,
            publication,
        ));
    }

    if let Err(error) = publication.sync_parent(destination_path) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            None,
            error,
            publication,
        ));
    }
    Ok(())
}

fn clean_prepublication_failure(
    destination: &AbsolutePath,
    temporary: &Path,
    identity: FileIdentity,
    source: io::Error,
    publication: &impl Publication,
) -> RuntimeError {
    match remove_if_identity(temporary, identity, publication) {
        Ok(()) => filesystem_error(FileSystemAction::Copy, destination, source),
        Err(cleanup) => RuntimeError::PartialPublication {
            path: destination.clone(),
            residuals: PublicationResiduals::new(
                [publication_residual(
                    PublicationResidualRole::Temporary,
                    temporary,
                )],
                DirectorySyncState::NotRequired,
            ),
            source,
            cleanup,
        },
    }
}

fn rollback_publication(
    destination: &AbsolutePath,
    identity: FileIdentity,
    temporary: Option<(&Path, FileIdentity)>,
    source: io::Error,
    publication: &impl Publication,
) -> RuntimeError {
    let destination_path = Path::new(destination.as_str());
    let mut failures = Vec::new();
    let mut residual_paths = BTreeSet::new();
    if let Err(error) = remove_if_identity(destination_path, identity, publication) {
        residual_paths.insert(PublicationResidual::new(
            PublicationResidualRole::Destination,
            destination.clone(),
        ));
        failures.push(format!("destination rollback: {error}"));
    }
    if let Some((temporary_path, temporary_identity)) = temporary
        && let Err(error) = remove_if_identity(temporary_path, temporary_identity, publication)
    {
        residual_paths.insert(publication_residual(
            PublicationResidualRole::Temporary,
            temporary_path,
        ));
        failures.push(format!("temporary cleanup: {error}"));
    }
    let directory_sync = match publication.sync_parent(destination_path) {
        Ok(()) => DirectorySyncState::Synced,
        Err(error) => {
            failures.push(format!("rollback directory sync: {error}"));
            DirectorySyncState::Uncertain
        }
    };
    if residual_paths.is_empty() && directory_sync == DirectorySyncState::Synced {
        filesystem_error(FileSystemAction::Copy, destination, source)
    } else {
        RuntimeError::PartialPublication {
            path: destination.clone(),
            residuals: PublicationResiduals::new(residual_paths, directory_sync),
            source,
            cleanup: io::Error::other(failures.join("; ")),
        }
    }
}

fn publication_residual(role: PublicationResidualRole, path: &Path) -> PublicationResidual {
    let value = path
        .to_str()
        .expect("owned publication paths originate from validated UTF-8 paths");
    PublicationResidual::new(
        role,
        AbsolutePath::new(value)
            .expect("owned publication paths remain lexically normalized and absolute"),
    )
}

fn remove_if_identity(
    path: &Path,
    expected: FileIdentity,
    publication: &impl Publication,
) -> io::Result<()> {
    match path_identity(path) {
        Ok(actual) if actual == expected => publication.remove(path),
        Ok(_) => Err(io::Error::other("path identity changed before cleanup")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn require_path_identity(path: &Path, expected: FileIdentity) -> io::Result<()> {
    match path_identity(path) {
        Ok(actual) if actual == expected => Ok(()),
        Ok(_) => Err(io::Error::other("path identity changed")),
        Err(error) => Err(error),
    }
}
