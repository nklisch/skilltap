//! Bounded, descriptor-relative observation of harness-owned trees.

use super::{
    ExternalTreeEntry, ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest,
    ExternalTreeSnapshot, ObservationRuntimeError,
};
use crate::domain::{AbsolutePath, RelativeArtifactPath};
use std::{
    ffi::{CStr, CString, OsString},
    fs::File,
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd, RawFd},
        unix::ffi::OsStringExt,
    },
    path::{Component, Path},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemExternalTreeObserver;

impl ExternalTreeObserver for SystemExternalTreeObserver {
    fn observe(
        &self,
        request: &ExternalTreeRequest,
    ) -> Result<ExternalTreeSnapshot, ObservationRuntimeError> {
        self.observe_with(request, &mut |_| Ok(()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HookPoint<'a> {
    RootOpened,
    BeforeOpen(&'a str),
    AfterRead(&'a str),
}

impl SystemExternalTreeObserver {
    fn observe_with(
        &self,
        request: &ExternalTreeRequest,
        hook: &mut impl FnMut(HookPoint<'_>) -> Result<(), ObservationRuntimeError>,
    ) -> Result<ExternalTreeSnapshot, ObservationRuntimeError> {
        let (root, bindings, root_identity) = open_root(request.root())?;
        hook(HookPoint::RootOpened)?;
        let mut state = WalkState {
            limits: request.limits(),
            entries: Vec::new(),
            entry_count: 0,
            total_bytes: 0,
        };
        walk(&root, None, 0, &mut state, hook)?;
        if file_identity(&root).map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?
            != root_identity
        {
            return Err(ObservationRuntimeError::TreeEntryChanged);
        }
        for binding in bindings.iter().rev() {
            verify_at(binding.parent.as_raw_fd(), &binding.name, binding.identity)?;
        }
        ExternalTreeSnapshot::new(state.entries, request.limits())
    }
}

struct WalkState {
    limits: ExternalTreeLimits,
    entries: Vec<ExternalTreeEntry>,
    entry_count: u64,
    total_bytes: u64,
}

fn walk(
    directory: &File,
    prefix: Option<&str>,
    parent_depth: u32,
    state: &mut WalkState,
    hook: &mut impl FnMut(HookPoint<'_>) -> Result<(), ObservationRuntimeError>,
) -> Result<(), ObservationRuntimeError> {
    let names = directory_names(
        directory,
        state.limits.entries().saturating_sub(state.entry_count),
    )?;
    state.entry_count = state
        .entry_count
        .checked_add(
            u64::try_from(names.len())
                .map_err(|_| ObservationRuntimeError::TreeEntryLimitExceeded)?,
        )
        .ok_or(ObservationRuntimeError::TreeEntryLimitExceeded)?;
    for name in names {
        let relative = prefix.map_or_else(|| name.clone(), |prefix| format!("{prefix}/{name}"));
        let depth = parent_depth
            .checked_add(1)
            .ok_or(ObservationRuntimeError::TreeDepthLimitExceeded)?;
        if depth > state.limits.depth() {
            return Err(ObservationRuntimeError::TreeDepthLimitExceeded);
        }
        let path = RelativeArtifactPath::new(relative.clone())
            .map_err(|_| ObservationRuntimeError::TreeEntryNonUtf8)?;
        let name = CString::new(name).map_err(|_| ObservationRuntimeError::TreeEntryNonUtf8)?;
        let before = stat_at(directory.as_raw_fd(), &name)
            .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
        let identity = stat_identity(&before)?;
        match before.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                hook(HookPoint::BeforeOpen(&relative))?;
                let child = open_entry(directory.as_raw_fd(), &name, libc::O_DIRECTORY)?;
                require_opened(&child, identity, libc::S_IFDIR)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                state.entries.push(ExternalTreeEntry::directory(path));
                walk(&child, Some(&relative), depth, state, hook)?;
                verify_opened_and_path(&child, directory.as_raw_fd(), &name, identity)?;
            }
            libc::S_IFREG => {
                if stat_size(&before)? > state.limits.file_bytes() {
                    return Err(ObservationRuntimeError::TreeFileLimitExceeded);
                }
                hook(HookPoint::BeforeOpen(&relative))?;
                let mut file = open_entry(directory.as_raw_fd(), &name, 0)?;
                require_opened(&file, identity, libc::S_IFREG)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                let bytes = read_file_bounded(&mut file, state.limits.file_bytes())?;
                hook(HookPoint::AfterRead(&relative))?;
                verify_opened_and_path(&file, directory.as_raw_fd(), &name, identity)?;
                add_total(state, bytes.len())?;
                state.entries.push(ExternalTreeEntry::file(path, bytes));
            }
            libc::S_IFLNK => {
                hook(HookPoint::BeforeOpen(&relative))?;
                let target = read_link_bounded(
                    directory.as_raw_fd(),
                    &name,
                    state.limits.symlink_target_bytes(),
                )
                .map_err(|error| match stat_at(directory.as_raw_fd(), &name) {
                    Ok(after) if stat_identity(&after).ok() != Some(identity) => {
                        ObservationRuntimeError::TreeEntryChanged
                    }
                    _ => error,
                })?;
                hook(HookPoint::AfterRead(&relative))?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                add_total(state, target.len())?;
                state.entries.push(ExternalTreeEntry::symlink(path, target));
            }
            _ => return Err(ObservationRuntimeError::TreeEntryUnsupported),
        }
    }
    Ok(())
}

fn add_total(state: &mut WalkState, bytes: usize) -> Result<(), ObservationRuntimeError> {
    state.total_bytes = state
        .total_bytes
        .checked_add(
            u64::try_from(bytes).map_err(|_| ObservationRuntimeError::TreeTotalLimitExceeded)?,
        )
        .ok_or(ObservationRuntimeError::TreeTotalLimitExceeded)?;
    if state.total_bytes > state.limits.total_bytes() {
        return Err(ObservationRuntimeError::TreeTotalLimitExceeded);
    }
    Ok(())
}

struct RootBinding {
    parent: File,
    name: CString,
    identity: Identity,
}

fn open_root(
    path: &AbsolutePath,
) -> Result<(File, Vec<RootBinding>, Identity), ObservationRuntimeError> {
    let mut directory =
        File::open("/").map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
    let mut bindings = Vec::new();
    for component in Path::new(path.as_str()).components() {
        let Component::Normal(component) = component else {
            continue;
        };
        let name = CString::new(component.as_encoded_bytes())
            .map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
        let before = stat_at(directory.as_raw_fd(), &name)
            .map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
        if before.st_mode & libc::S_IFMT != libc::S_IFDIR {
            return Err(ObservationRuntimeError::TreeRootUnavailable);
        }
        let identity =
            stat_identity(&before).map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
        let child = open_entry(directory.as_raw_fd(), &name, libc::O_DIRECTORY)
            .map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
        require_opened(&child, identity, libc::S_IFDIR)?;
        verify_at(directory.as_raw_fd(), &name, identity)?;
        bindings.push(RootBinding {
            parent: directory,
            name,
            identity,
        });
        directory = child;
    }
    let identity =
        file_identity(&directory).map_err(|_| ObservationRuntimeError::TreeRootUnavailable)?;
    Ok((directory, bindings, identity))
}

fn open_entry(
    parent: RawFd,
    name: &CStr,
    extra: libc::c_int,
) -> Result<File, ObservationRuntimeError> {
    let fd = cvt(unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | extra,
            0,
        )
    })
    .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn read_file_bounded(file: &mut File, limit: u64) -> Result<Vec<u8>, ObservationRuntimeError> {
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
    if u64::try_from(bytes.len()).map_err(|_| ObservationRuntimeError::TreeFileLimitExceeded)?
        > limit
    {
        return Err(ObservationRuntimeError::TreeFileLimitExceeded);
    }
    Ok(bytes)
}

fn read_link_bounded(
    parent: RawFd,
    name: &CStr,
    limit: u64,
) -> Result<Vec<u8>, ObservationRuntimeError> {
    let capacity = usize::try_from(limit.saturating_add(1))
        .map_err(|_| ObservationRuntimeError::TreeSymlinkTargetLimitExceeded)?;
    let mut target = vec![0_u8; capacity];
    let length = unsafe {
        libc::readlinkat(
            parent,
            name.as_ptr(),
            target.as_mut_ptr().cast(),
            target.len(),
        )
    };
    if length < 0 {
        return Err(ObservationRuntimeError::TreeEntryUnreadable);
    }
    let length =
        usize::try_from(length).map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
    if u64::try_from(length).map_err(|_| ObservationRuntimeError::TreeSymlinkTargetLimitExceeded)?
        > limit
    {
        return Err(ObservationRuntimeError::TreeSymlinkTargetLimitExceeded);
    }
    target.truncate(length);
    Ok(target)
}

fn directory_names(
    directory: &File,
    remaining: u64,
) -> Result<Vec<String>, ObservationRuntimeError> {
    let duplicate = cvt(unsafe { libc::dup(directory.as_raw_fd()) })
        .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(ObservationRuntimeError::TreeEntryUnreadable);
    }
    let result = (|| {
        let mut names = Vec::new();
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                if io::Error::last_os_error().raw_os_error() == Some(0) {
                    break;
                }
                return Err(ObservationRuntimeError::TreeEntryUnreadable);
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            if u64::try_from(names.len()).unwrap_or(u64::MAX) >= remaining {
                return Err(ObservationRuntimeError::TreeEntryLimitExceeded);
            }
            names.push(
                OsString::from_vec(bytes.to_vec())
                    .into_string()
                    .map_err(|_| ObservationRuntimeError::TreeEntryNonUtf8)?,
            );
        }
        names.sort();
        Ok(names)
    })();
    unsafe { libc::closedir(stream) };
    result
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Identity {
    device: u64,
    inode: u64,
}
fn stat_identity(metadata: &libc::stat) -> Result<Identity, ObservationRuntimeError> {
    normalize_identity(metadata.st_dev, metadata.st_ino)
        .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)
}
fn stat_size(metadata: &libc::stat) -> Result<u64, ObservationRuntimeError> {
    metadata
        .st_size
        .try_into()
        .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)
}
fn file_identity(file: &File) -> io::Result<Identity> {
    let mut metadata = std::mem::MaybeUninit::uninit();
    cvt(unsafe { libc::fstat(file.as_raw_fd(), metadata.as_mut_ptr()) })?;
    let metadata = unsafe { metadata.assume_init() };
    normalize_identity(metadata.st_dev, metadata.st_ino)
}
fn normalize_identity<D, I>(device: D, inode: I) -> io::Result<Identity>
where
    D: TryInto<u64>,
    I: TryInto<u64>,
{
    Ok(Identity {
        device: device
            .try_into()
            .map_err(|_| io::Error::other("invalid device identity"))?,
        inode: inode
            .try_into()
            .map_err(|_| io::Error::other("invalid inode identity"))?,
    })
}
fn require_opened(
    file: &File,
    expected: Identity,
    kind: libc::mode_t,
) -> Result<(), ObservationRuntimeError> {
    let mut metadata = std::mem::MaybeUninit::uninit();
    cvt(unsafe { libc::fstat(file.as_raw_fd(), metadata.as_mut_ptr()) })
        .map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)?;
    let metadata = unsafe { metadata.assume_init() };
    if metadata.st_mode & libc::S_IFMT != kind || stat_identity(&metadata)? != expected {
        return Err(ObservationRuntimeError::TreeEntryChanged);
    }
    Ok(())
}
fn verify_opened_and_path(
    file: &File,
    parent: RawFd,
    name: &CStr,
    identity: Identity,
) -> Result<(), ObservationRuntimeError> {
    if file_identity(file).map_err(|_| ObservationRuntimeError::TreeEntryUnreadable)? != identity {
        return Err(ObservationRuntimeError::TreeEntryChanged);
    }
    verify_at(parent, name, identity)
}
fn verify_at(
    parent: RawFd,
    name: &CStr,
    identity: Identity,
) -> Result<(), ObservationRuntimeError> {
    let after = stat_at(parent, name).map_err(|_| ObservationRuntimeError::TreeEntryChanged)?;
    if stat_identity(&after)? != identity {
        return Err(ObservationRuntimeError::TreeEntryChanged);
    }
    Ok(())
}
fn stat_at(parent: RawFd, name: &CStr) -> io::Result<libc::stat> {
    let mut metadata = std::mem::MaybeUninit::uninit();
    cvt(unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    Ok(unsafe { metadata.assume_init() })
}
fn cvt(result: libc::c_int) -> io::Result<libc::c_int> {
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}
#[cfg(target_vendor = "apple")]
fn clear_errno() {
    unsafe { *libc::__error() = 0 };
}
#[cfg(not(target_vendor = "apple"))]
fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt, path::Path};

    use skilltap_test_support::ExternalTreeFixture;

    use super::*;

    fn limits(depth: u32, entries: u64, file: u64, total: u64, link: u64) -> ExternalTreeLimits {
        ExternalTreeLimits::new(depth, entries, file, total, link).unwrap()
    }

    fn request(tree: &ExternalTreeFixture, limits: ExternalTreeLimits) -> ExternalTreeRequest {
        ExternalTreeRequest::new(
            AbsolutePath::new(tree.root().to_str().unwrap()).unwrap(),
            limits,
        )
    }

    #[test]
    fn observes_sorted_directories_files_and_opaque_links_without_following() {
        const SECRET: &str = "identifier-valid-secret-target";
        let tree = ExternalTreeFixture::new().unwrap();
        fs::create_dir(tree.root().join("b-dir")).unwrap();
        fs::write(tree.root().join("b-dir/file"), b"payload").unwrap();
        tree.live_symlink(Path::new("a-link"), Path::new(SECRET))
            .unwrap();
        tree.dangling_symlink(Path::new("z-dangling")).unwrap();

        let snapshot = SystemExternalTreeObserver
            .observe(&request(&tree, limits(3, 8, 16, 128, 64)))
            .unwrap();
        let paths = snapshot
            .entries()
            .iter()
            .map(|entry| entry.path().as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths, ["a-link", "b-dir", "b-dir/file", "z-dangling"]);
        assert_eq!(
            snapshot.entries()[0].symlink_target(),
            Some(SECRET.as_bytes())
        );
        assert_eq!(
            snapshot.entries()[2].file_bytes(),
            Some(b"payload".as_slice())
        );
        assert!(!format!("{snapshot:?}").contains(SECRET));
        assert!(!format!("{:?}", snapshot.entries()[0]).contains(SECRET));
    }

    #[test]
    fn rejects_depth_entry_file_total_and_link_boundaries() {
        let tree = ExternalTreeFixture::new().unwrap();
        fs::create_dir(tree.root().join("dir")).unwrap();
        fs::write(tree.root().join("dir/file"), b"1234").unwrap();
        fs::write(tree.root().join("other"), b"12").unwrap();
        tree.live_symlink(Path::new("link"), Path::new("abcd"))
            .unwrap();

        let observer = SystemExternalTreeObserver;
        assert_eq!(
            observer.observe(&request(&tree, limits(1, 8, 8, 32, 8))),
            Err(ObservationRuntimeError::TreeDepthLimitExceeded)
        );
        assert_eq!(
            observer.observe(&request(&tree, limits(3, 3, 8, 32, 8))),
            Err(ObservationRuntimeError::TreeEntryLimitExceeded)
        );
        assert_eq!(
            observer.observe(&request(&tree, limits(3, 8, 3, 32, 8))),
            Err(ObservationRuntimeError::TreeFileLimitExceeded)
        );
        assert_eq!(
            observer.observe(&request(&tree, limits(3, 8, 4, 5, 4))),
            Err(ObservationRuntimeError::TreeTotalLimitExceeded)
        );
        assert_eq!(
            observer.observe(&request(&tree, limits(3, 8, 8, 32, 3))),
            Err(ObservationRuntimeError::TreeSymlinkTargetLimitExceeded)
        );

        let exact = ExternalTreeFixture::new().unwrap();
        fs::write(exact.root().join("file"), b"1234").unwrap();
        exact
            .live_symlink(Path::new("link"), Path::new("abcd"))
            .unwrap();
        assert!(
            observer
                .observe(&request(&exact, limits(1, 2, 4, 8, 4)))
                .is_ok()
        );
    }

    #[test]
    fn rejects_special_and_non_utf8_entries_without_blocking_or_rendering() {
        let observer = SystemExternalTreeObserver;
        for special in ["fifo", "socket"] {
            let tree = ExternalTreeFixture::new().unwrap();
            let _socket = if special == "fifo" {
                tree.fifo(Path::new("special")).unwrap();
                None
            } else {
                Some(tree.live_socket(Path::new("special")).unwrap())
            };
            let error = observer
                .observe(&request(&tree, limits(1, 2, 8, 8, 8)))
                .unwrap_err();
            assert_eq!(error, ObservationRuntimeError::TreeEntryUnsupported);
        }

        let tree = ExternalTreeFixture::new().unwrap();
        let name = OsString::from_vec(vec![b's', 0xff]);
        fs::write(tree.root().join(name), b"secret").unwrap();
        let error = observer
            .observe(&request(&tree, limits(1, 2, 8, 8, 8)))
            .unwrap_err();
        assert_eq!(error, ObservationRuntimeError::TreeEntryNonUtf8);
        assert_eq!(
            error.to_string(),
            "an external tree entry name is not valid UTF-8"
        );
    }

    #[test]
    fn deterministic_hooks_detect_file_and_root_replacement_and_injected_denial() {
        let observer = SystemExternalTreeObserver;
        let tree = ExternalTreeFixture::new().unwrap();
        fs::write(tree.root().join("file"), b"old").unwrap();
        fs::write(tree.root().join("staged"), b"new").unwrap();
        let mut replaced = false;
        let error = observer
            .observe_with(&request(&tree, limits(1, 4, 8, 16, 8)), &mut |point| {
                if point == HookPoint::BeforeOpen("file") && !replaced {
                    fs::rename(tree.root().join("staged"), tree.root().join("file")).unwrap();
                    replaced = true;
                }
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, ObservationRuntimeError::TreeEntryChanged);

        let after_read = ExternalTreeFixture::new().unwrap();
        fs::write(after_read.root().join("file"), b"old").unwrap();
        fs::write(after_read.root().join("staged"), b"new").unwrap();
        let mut replaced = false;
        let error = observer
            .observe_with(
                &request(&after_read, limits(1, 4, 8, 16, 8)),
                &mut |point| {
                    if point == HookPoint::AfterRead("file") && !replaced {
                        fs::rename(
                            after_read.root().join("staged"),
                            after_read.root().join("file"),
                        )
                        .unwrap();
                        replaced = true;
                    }
                    Ok(())
                },
            )
            .unwrap_err();
        assert_eq!(error, ObservationRuntimeError::TreeEntryChanged);

        let denied = ExternalTreeFixture::new().unwrap();
        fs::write(denied.root().join("file"), b"bytes").unwrap();
        let error = observer
            .observe_with(&request(&denied, limits(1, 2, 8, 8, 8)), &mut |point| {
                if point == HookPoint::BeforeOpen("file") {
                    Err(ObservationRuntimeError::TreeEntryUnreadable)
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(error, ObservationRuntimeError::TreeEntryUnreadable);

        let parent = ExternalTreeFixture::new().unwrap();
        let root = parent.root().join("root");
        let staged = parent.root().join("staged-root");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&staged).unwrap();
        fs::write(root.join("old"), b"old").unwrap();
        fs::write(staged.join("new"), b"new").unwrap();
        let request = ExternalTreeRequest::new(
            AbsolutePath::new(root.to_str().unwrap()).unwrap(),
            limits(1, 2, 8, 8, 8),
        );
        let mut swapped = false;
        let error = observer
            .observe_with(&request, &mut |point| {
                if point == HookPoint::RootOpened && !swapped {
                    fs::rename(&root, parent.root().join("old-root")).unwrap();
                    fs::rename(&staged, &root).unwrap();
                    swapped = true;
                }
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, ObservationRuntimeError::TreeEntryChanged);
    }

    #[test]
    fn missing_file_root_and_symlink_root_are_unavailable() {
        let tree = ExternalTreeFixture::new().unwrap();
        fs::write(tree.root().join("file"), b"x").unwrap();
        tree.live_symlink(Path::new("link"), Path::new("."))
            .unwrap();
        for root in [
            tree.root().join("missing"),
            tree.root().join("file"),
            tree.root().join("link"),
        ] {
            let request = ExternalTreeRequest::new(
                AbsolutePath::new(root.to_str().unwrap()).unwrap(),
                limits(1, 2, 8, 8, 8),
            );
            assert_eq!(
                SystemExternalTreeObserver.observe(&request),
                Err(ObservationRuntimeError::TreeRootUnavailable)
            );
        }
    }
}
