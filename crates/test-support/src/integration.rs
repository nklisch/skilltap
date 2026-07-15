use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::IsolatedMachine;

#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeEntry {
    relative: PathBuf,
    kind: &'static str,
    bytes: Option<Vec<u8>>,
    link: Option<PathBuf>,
    modified: Option<SystemTime>,
}

/// Stable, no-follow snapshot of one native tree for mutation assertions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTreeSnapshot(Vec<NativeEntry>);

/// Snapshot one path without following symlinks.
pub fn snapshot_tree(root: &Path) -> io::Result<NativeTreeSnapshot> {
    let mut entries = Vec::new();
    if root.exists() {
        visit(root, root, &mut entries)?;
    }
    Ok(NativeTreeSnapshot(entries))
}

/// Snapshot the isolated machine's native and skilltap-owned roots together.
pub fn snapshot_native_roots(machine: &IsolatedMachine) -> io::Result<NativeTreeSnapshot> {
    let mut entries = Vec::new();
    for root in [
        machine.home().join(".codex"),
        machine.home().join(".claude"),
        machine.home().join(".copilot"),
        machine.home().join(".agents"),
        machine.configuration_home().join("skilltap"),
    ] {
        if root.exists() {
            let label = root
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("root"));
            let snapshot = snapshot_tree(&root)?;
            entries.extend(snapshot.0.into_iter().map(|mut entry| {
                entry.relative = label.join(entry.relative);
                entry
            }));
        }
    }
    entries.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(NativeTreeSnapshot(entries))
}

fn visit(root: &Path, current: &Path, entries: &mut Vec<NativeEntry>) -> io::Result<()> {
    let metadata = fs::symlink_metadata(current)?;
    let file_type = metadata.file_type();
    let kind = if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_file() {
        "file"
    } else {
        "other"
    };
    entries.push(NativeEntry {
        relative: current.strip_prefix(root).unwrap_or(current).to_owned(),
        kind,
        bytes: file_type.is_file().then(|| fs::read(current)).transpose()?,
        link: file_type
            .is_symlink()
            .then(|| fs::read_link(current))
            .transpose()?,
        modified: metadata.modified().ok(),
    });
    if file_type.is_dir() {
        let mut children = fs::read_dir(current)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            visit(root, &child, entries)?;
        }
    }
    Ok(())
}
