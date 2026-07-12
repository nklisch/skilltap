//! Private command boundary for bootstrap publication and daemon binary policy.
//!
//! This module owns bootstrap composition, publication, rollback, and daemon
//! policy projection; runtime ports remain in skilltap-core.

use super::*;
pub(super) fn execute_system_bootstrap(args: &BootstrapArgs) -> Outcome {
    use skilltap_core::domain::{ConfiguredBinary, TargetSelection};
    use skilltap_harnesses::{
        HarnessBootstrapPolicy, HarnessSetupResult, setup_first_party_plugin,
    };

    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let filesystem = SystemFileSystem;
    let repository = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let config = match repository.load() {
        Ok(DocumentState::Missing) => ConfigDocument::defaults(),
        Ok(DocumentState::Present(value)) => value,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let selected = args.target.clone().unwrap_or(TargetSelection::All);
    let includes = |kind: HarnessKind| match &selected {
        TargetSelection::All => true,
        TargetSelection::Only(target) => target.as_str() == kind.id(),
    };
    let binary = execute_binary_bootstrap(args, &paths);
    // Do not mutate native harness state when the binary boundary did not
    // complete. Bootstrap's binary and harness phases are reported separately,
    // but a failed/blocked release must remain a read-only attention result.
    if binary.attention {
        return compose_bootstrap_outcome(args, binary, Vec::new());
    }
    let search_path = std::env::var_os("PATH");
    let process_limits =
        skilltap_core::runtime::ProcessLimits::new(30_000, 64 * 1024, 64 * 1024, 128 * 1024)
            .expect("bootstrap process limits are valid");
    let json_limits = skilltap_core::runtime::JsonLimits::new(128 * 1024, 32)
        .expect("bootstrap JSON limits are valid");
    let mut harness_results = Vec::new();
    for (kind, policy) in [
        (HarnessKind::Codex, &config.harnesses().codex),
        (HarnessKind::Claude, &config.harnesses().claude),
    ] {
        if !includes(kind) {
            continue;
        }
        let configured = if std::path::Path::new(policy.binary.as_str()).is_absolute() {
            match AbsolutePath::new(policy.binary.as_str()) {
                Ok(path) => ConfiguredBinary::absolute(path),
                Err(_) => {
                    harness_results.push((
                        kind,
                        HarnessSetupResult::Unavailable {
                            harness: kind,
                            reason: skilltap_harnesses::SetupReason::InvalidVersion,
                        },
                    ));
                    continue;
                }
            }
        } else {
            match NativeId::new(policy.binary.as_str()).and_then(ConfiguredBinary::path_lookup) {
                Ok(binary) => binary,
                Err(_) => {
                    harness_results.push((
                        kind,
                        HarnessSetupResult::Unavailable {
                            harness: kind,
                            reason: skilltap_harnesses::SetupReason::InvalidVersion,
                        },
                    ));
                    continue;
                }
            }
        };
        let bootstrap_policy = HarnessBootstrapPolicy {
            configured,
            search_path: search_path.clone(),
            process_limits,
            json_limits,
            plugin_name: NativeId::new("skilltap").expect("canonical plugin id is valid"),
            canonical_source: Some(
                skilltap_core::domain::SourceLocator::new(
                    "https://github.com/nklisch/skilltap/tree/main/plugin",
                )
                .expect("canonical source is valid"),
            ),
        };
        let result = setup_first_party_plugin(kind, &bootstrap_policy);
        harness_results.push((kind, result));
    }
    compose_bootstrap_outcome(args, binary, harness_results)
}

fn compose_bootstrap_outcome(
    args: &BootstrapArgs,
    binary: BinaryBootstrapResult,
    harness_results: Vec<(HarnessKind, skilltap_harnesses::HarnessSetupResult)>,
) -> Outcome {
    use skilltap_core::domain::TargetSelection;
    use skilltap_harnesses::HarnessSetupResult;

    let mut outcome = Outcome::new("bootstrap", ResultClass::Completed)
        .with_scope(crate::OutputScope::Global)
        .with_summary("binary", "pending")
        .with_summary("version", skilltap_core::VERSION)
        .with_summary("allow_major", args.allow_major)
        .with_resource(binary.entry);
    if binary.attention {
        outcome.result = ResultClass::AttentionRequired;
    }
    for warning in binary.warnings {
        outcome = outcome.with_warning(warning);
    }
    for action in binary.next_actions {
        outcome = outcome.with_next_action(action);
    }
    if binary.attention {
        return outcome;
    }
    let selected = args.target.clone().unwrap_or(TargetSelection::All);
    for (kind, result) in harness_results {
        let included = match &selected {
            TargetSelection::All => true,
            TargetSelection::Only(target) => target.as_str() == kind.id(),
        };
        if !included {
            continue;
        }
        let (status, attention, next_action) = match &result {
            HarnessSetupResult::Installed { .. } => ("installed", false, None),
            HarnessSetupResult::AlreadyPresent { .. } => ("already-present", false, None),
            HarnessSetupResult::Unavailable { reason, .. } => {
                ("unavailable", true, Some(reason.to_string()))
            }
            HarnessSetupResult::Unsupported { next_action, .. } => {
                ("unsupported", true, Some(next_action.clone()))
            }
            HarnessSetupResult::Failed { reason, .. } => ("failed", true, Some(reason.to_string())),
        };
        outcome = outcome.with_resource(OutputEntry::new(kind.id(), status));
        if let Some(next_action) = next_action {
            outcome = outcome.with_next_action(NextAction::new(
                format!("bootstrap_{}", kind.id()),
                next_action,
            ));
        }
        if attention {
            outcome.result = ResultClass::AttentionRequired;
        }
    }
    outcome
}

struct BinaryBootstrapResult {
    entry: OutputEntry,
    attention: bool,
    pending: bool,
    changed: bool,
    warnings: Vec<crate::Warning>,
    next_actions: Vec<crate::NextAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryExecutionMode {
    Check,
    ApplySafe,
}

struct BinaryUpdateTarget {
    destination: AbsolutePath,
    lock_path: AbsolutePath,
}

fn execute_binary_bootstrap(
    args: &BootstrapArgs,
    paths: &skilltap_core::runtime::PlatformPaths,
) -> BinaryBootstrapResult {
    execute_binary_bootstrap_mode(args, paths, BinaryExecutionMode::ApplySafe)
}

fn execute_binary_bootstrap_mode(
    args: &BootstrapArgs,
    paths: &skilltap_core::runtime::PlatformPaths,
    mode: BinaryExecutionMode,
) -> BinaryBootstrapResult {
    use skilltap_core::{
        bootstrap::ArtifactKey,
        runtime::{SystemArtifactFetcher, SystemBinaryInstaller, SystemReleaseResolver},
    };
    let destination = match std::env::var_os("SKILLTAP_INSTALL")
        .and_then(|value| value.into_string().ok())
        .map(AbsolutePath::new)
        .transpose()
    {
        Ok(Some(path)) => path,
        Ok(None) => {
            match AbsolutePath::new(format!("{}/.local/bin/skilltap", paths.home().as_str())) {
                Ok(path) => path,
                Err(_) => {
                    return binary_attention(
                        "invalid-destination",
                        "The user-local skilltap install destination is invalid.",
                    );
                }
            }
        }
        Err(_) => {
            return binary_attention(
                "invalid-destination",
                "SKILLTAP_INSTALL must be a normalized absolute path.",
            );
        }
    };
    let key = match ArtifactKey::current() {
        Ok(key) => key,
        Err(_) => {
            return binary_attention(
                "unsupported-platform",
                "This platform has no published skilltap bootstrap artifact.",
            );
        }
    };
    let resolver = SystemReleaseResolver::current(key);
    let fetcher = SystemArtifactFetcher;
    let installer = SystemBinaryInstaller;
    let lock_path = match AbsolutePath::new(format!(
        "{}/skilltap.lock",
        paths.skilltap_config().as_str()
    )) {
        Ok(path) => path,
        Err(_) => {
            return binary_attention(
                "lock_path_invalid",
                "The skilltap binary update lock path is invalid.",
            );
        }
    };
    execute_binary_bootstrap_with_lock(
        args,
        BinaryUpdateTarget {
            destination,
            lock_path,
        },
        &resolver,
        &fetcher,
        &installer,
        &SystemConfigurationLock,
        mode,
    )
}

/// Run a binary publication while holding the same cooperative configuration
/// lock used by all foreground mutations.  The daemon and foreground paths
/// both use this boundary, so a resolver never fetches an artifact that cannot
/// be published to the current installation.
fn execute_binary_bootstrap_with_lock<R, F, I, L>(
    args: &BootstrapArgs,
    target: BinaryUpdateTarget,
    resolver: &R,
    fetcher: &F,
    installer: &I,
    lock: &L,
    mode: BinaryExecutionMode,
) -> BinaryBootstrapResult
where
    R: skilltap_core::runtime::ReleaseResolver,
    F: skilltap_core::runtime::ArtifactFetcher,
    I: skilltap_core::runtime::BinaryInstaller,
    L: ConfigurationLock,
{
    if let Some(parent) = std::path::Path::new(target.lock_path.as_str()).parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return binary_attention(
            "configuration_lock_path_unavailable",
            "The skilltap configuration directory could not be prepared for binary updates.",
        );
    }
    let guard = match lock.try_acquire(&target.lock_path) {
        Ok(guard) => guard,
        Err(skilltap_core::runtime::RuntimeError::LockContended { .. }) => {
            return binary_pending(
                "configuration_locked",
                "Another skilltap update is publishing the configured binary; no network or mutation was attempted.",
            );
        }
        Err(_) => {
            return binary_attention(
                "configuration_lock_failed",
                "The skilltap binary update lock could not be acquired safely.",
            );
        }
    };
    let mut result = execute_binary_bootstrap_with_mode(
        args,
        target.destination,
        resolver,
        fetcher,
        installer,
        mode,
    );
    if guard.release().is_err() {
        result.attention = true;
        result.pending = true;
        result.warnings.push(crate::Warning::new(
            "configuration_lock_release_failed",
            "The skilltap binary update completed, but the configuration lock could not be released safely.",
        ));
    }
    result
}

/// Test-only composition is provided by passing in the release, transport,
/// and publication ports.  The shipped command above always constructs the
/// canonical HTTPS resolver and system ports; no environment variable can
/// replace those production boundaries.
#[cfg(test)]
fn execute_binary_bootstrap_with<R, F, I>(
    args: &BootstrapArgs,
    destination: AbsolutePath,
    resolver: &R,
    fetcher: &F,
    installer: &I,
) -> BinaryBootstrapResult
where
    R: skilltap_core::runtime::ReleaseResolver,
    F: skilltap_core::runtime::ArtifactFetcher,
    I: skilltap_core::runtime::BinaryInstaller,
{
    execute_binary_bootstrap_with_mode(
        args,
        destination,
        resolver,
        fetcher,
        installer,
        BinaryExecutionMode::ApplySafe,
    )
}

fn execute_binary_bootstrap_with_mode<R, F, I>(
    args: &BootstrapArgs,
    destination: AbsolutePath,
    resolver: &R,
    fetcher: &F,
    installer: &I,
    mode: BinaryExecutionMode,
) -> BinaryBootstrapResult
where
    R: skilltap_core::runtime::ReleaseResolver,
    F: skilltap_core::runtime::ArtifactFetcher,
    I: skilltap_core::runtime::BinaryInstaller,
{
    use skilltap_core::bootstrap::{ArtifactKey, BinaryDecision, choose_binary_decision};
    let key = match ArtifactKey::current() {
        Ok(key) => key,
        Err(_) => {
            return binary_attention(
                "unsupported-platform",
                "This platform has no published skilltap bootstrap artifact.",
            );
        }
    };
    let manifest = match resolver.latest() {
        Ok(manifest) => manifest,
        Err(error) => return binary_attention("release_manifest_failed", &error.to_string()),
    };
    let artifact = match manifest.artifact(key) {
        Ok(artifact) => artifact,
        Err(error) => return binary_attention("release_asset_failed", &error.to_string()),
    };
    let installed = match installer.inspect(&destination) {
        Ok(value) => value,
        Err(error) => return binary_attention("binary_inspection_failed", &error.to_string()),
    };
    let installed_version = installed
        .as_ref()
        .and_then(|_| probe_installed_version(&destination));
    if installed.is_some() && installed_version.is_none() {
        return binary_attention(
            "unknown_version",
            "The existing skilltap executable version could not be verified; no replacement was attempted.",
        );
    }
    let decision = choose_binary_decision(
        installed_version.as_ref(),
        &manifest.version,
        args.allow_major,
    );
    if decision == BinaryDecision::MajorUpgradeBlocked {
        return BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "major-upgrade-blocked")
                .with_field("available_version", manifest.version.to_string())
                .with_field("policy", binary_policy_label(mode))
                .with_field("path_role", "user-local-bin/skilltap"),
            attention: true,
            pending: true,
            changed: false,
            warnings: vec![crate::Warning::new(
                "major_upgrade_blocked",
                "A newer major skilltap binary is available; no existing binary was changed.",
            )],
            next_actions: vec![crate::NextAction::new(
                "allow_major",
                "Rerun with --allow-major to accept the major-version consequence.",
            )],
        };
    }
    if decision == BinaryDecision::Noop {
        return BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "no-op")
                .with_field("version", manifest.version.to_string())
                .with_field("policy", binary_policy_label(mode))
                .with_field("path_role", "user-local-bin/skilltap"),
            attention: false,
            pending: false,
            changed: false,
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };
    }
    if mode == BinaryExecutionMode::Check {
        return BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "update-available")
                .with_field("available_version", manifest.version.to_string())
                .with_field("policy", binary_policy_label(mode))
                .with_field("path_role", "user-local-bin/skilltap"),
            attention: true,
            pending: true,
            changed: false,
            warnings: vec![crate::Warning::new(
                "binary_update_available",
                "A compatible skilltap binary update is available; the check policy did not publish it.",
            )],
            next_actions: vec![crate::NextAction::new(
                "apply_binary_update",
                "Run `skilltap bootstrap` or set bootstrap.mode = \"apply-safe\" to apply the verified update.",
            )],
        };
    }
    let parent = std::path::Path::new(destination.as_str())
        .parent()
        .unwrap_or(std::path::Path::new("/"));
    if std::fs::create_dir_all(parent).is_err() {
        return binary_attention(
            "destination_unavailable",
            "The user-local binary directory could not be created safely.",
        );
    }
    let (temporary_workspace, temporary) = match private_bootstrap_temp(parent) {
        Ok(paths) => paths,
        Err(_) => {
            return binary_attention(
                "temporary_path_failed",
                "The private bootstrap temporary path is invalid.",
            );
        }
    };
    let fetch_result = fetcher.fetch(artifact.download_url().as_str(), &temporary);
    if fetch_result.is_err() {
        let _ = std::fs::remove_dir_all(temporary_workspace.as_str());
        return binary_attention(
            "release_download_failed",
            "The release artifact could not be downloaded; the existing binary was preserved.",
        );
    }
    if let Err(error) = prepare_downloaded_release(&temporary, artifact) {
        let _ = std::fs::remove_dir_all(temporary_workspace.as_str());
        return match error {
            skilltap_core::runtime::ArtifactError::ChecksumMismatch => binary_attention(
                "release_checksum_failed",
                "The downloaded release artifact did not match the signed release checksum; the existing binary was preserved.",
            ),
            _ => binary_attention(
                "release_permissions_failed",
                "The downloaded release artifact could not be validated and made runnable safely; the existing binary was preserved.",
            ),
        };
    }
    if probe_installed_version(&temporary).as_ref() != Some(&manifest.version) {
        let _ = std::fs::remove_dir_all(temporary_workspace.as_str());
        return binary_attention(
            "release_identity_failed",
            "The downloaded executable did not report the verified release version; the existing binary was preserved.",
        );
    }
    let previous = std::fs::read(destination.as_str()).ok();
    #[cfg(unix)]
    let previous_mode = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(destination.as_str())
            .ok()
            .map(|metadata| metadata.permissions().mode())
    };
    let result = installer.install_verified(&temporary, &destination, artifact);
    let _ = std::fs::remove_dir_all(temporary_workspace.as_str());
    if let Err(error) = result {
        return binary_attention("binary_install_failed", &error.to_string());
    }
    // Capture the identity immediately after publication.  Passing a fresh
    // stat result into rollback would bless a replacement that arrived after
    // publication and let recovery overwrite an unrelated executable.
    let published_identity = binary_file_identity(&destination);
    if probe_installed_version(&destination).as_ref() != Some(&manifest.version) {
        let rollback = if let Some(previous) = previous {
            #[cfg(unix)]
            {
                restore_previous_binary(&destination, published_identity, &previous, previous_mode)
            }
            #[cfg(not(unix))]
            {
                restore_previous_binary(&destination, published_identity, &previous)
            }
        } else {
            remove_published_binary(&destination, published_identity)
        };
        let detail = match rollback {
            RollbackResult::Restored | RollbackResult::Removed => {
                "The published executable did not report the verified release version; the previous binary was restored."
            }
            RollbackResult::ReplacementPreserved => {
                "The published executable failed its identity check, but a replacement arrived before rollback; it was preserved and recovery needs attention."
            }
            RollbackResult::Failed => {
                "The published executable failed its identity check and could not be rolled back safely; recovery needs attention."
            }
        };
        return binary_attention("post_install_identity_failed", detail);
    }
    BinaryBootstrapResult {
        entry: OutputEntry::new(
            "binary",
            match decision {
                BinaryDecision::Install => "installed",
                _ => "updated",
            },
        )
        .with_field("version", manifest.version.to_string())
        .with_field("policy", binary_policy_label(mode))
        .with_field("path_role", "user-local-bin/skilltap"),
        attention: false,
        pending: false,
        changed: true,
        warnings: Vec::new(),
        next_actions: Vec::new(),
    }
}

fn prepare_downloaded_release(
    path: &AbsolutePath,
    expected: &skilltap_core::bootstrap::ReleaseArtifact,
) -> Result<(), skilltap_core::runtime::ArtifactError> {
    use skilltap_core::runtime::ArtifactError;

    let path = std::path::Path::new(path.as_str());
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ArtifactError::InvalidArtifact)?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_file()
        || metadata.len() > 64 * 1024 * 1024
    {
        return Err(ArtifactError::InvalidArtifact);
    }
    let digest = binary_file_digest(path).ok_or(ArtifactError::InvalidArtifact)?;
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if digest != expected.sha256() {
        return Err(ArtifactError::ChecksumMismatch);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| ArtifactError::InvalidArtifact)?;
    }
    Ok(())
}

fn binary_policy_label(mode: BinaryExecutionMode) -> &'static str {
    match mode {
        BinaryExecutionMode::Check => "check",
        BinaryExecutionMode::ApplySafe => "apply-safe",
    }
}

fn private_bootstrap_temp(
    parent: &std::path::Path,
) -> Result<(AbsolutePath, AbsolutePath), std::io::Error> {
    for attempt in 0..64u32 {
        let workspace_path = parent.join(format!(
            ".skilltap-bootstrap-{}-{}",
            std::process::id(),
            attempt
        ));
        match std::fs::create_dir(&workspace_path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(
                        &workspace_path,
                        std::fs::Permissions::from_mode(0o700),
                    )?;
                }
                let workspace = AbsolutePath::new(workspace_path.to_string_lossy().into_owned())
                    .map_err(|_| std::io::Error::other("invalid temporary workspace"))?;
                let payload = AbsolutePath::new(
                    workspace_path
                        .join("payload")
                        .to_string_lossy()
                        .into_owned(),
                )
                .map_err(|_| std::io::Error::other("invalid temporary payload"))?;
                return Ok((workspace, payload));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "temporary workspace exhausted",
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RollbackResult {
    Restored,
    Removed,
    ReplacementPreserved,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BinaryIdentity {
    location: (u64, u64),
    digest: [u8; 32],
}

/// Restore a prior executable through an atomic exchange.  A stat-then-rename
/// sequence is not sufficient here: another process can replace the path
/// between those operations and be overwritten by recovery.  Exchange lets us
/// inspect the inode that was actually displaced and put an unrelated one
/// back without clobbering it.
#[cfg(unix)]
fn restore_previous_binary(
    path: &AbsolutePath,
    expected: Option<BinaryIdentity>,
    bytes: &[u8],
    mode: Option<u32>,
) -> RollbackResult {
    restore_previous_binary_with_hook(path, expected, bytes, mode, || {})
}

#[cfg(unix)]
fn restore_previous_binary_with_hook(
    path: &AbsolutePath,
    expected: Option<BinaryIdentity>,
    bytes: &[u8],
    mode: Option<u32>,
    after_exchange: impl FnOnce(),
) -> RollbackResult {
    use std::os::unix::fs::PermissionsExt;
    let destination = std::path::Path::new(path.as_str());
    let Some(parent) = destination.parent() else {
        return RollbackResult::Failed;
    };
    if binary_file_identity_absolute(destination) != expected {
        return RollbackResult::ReplacementPreserved;
    }
    let temporary = parent.join(format!(
        ".skilltap-restore-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("worker")
    ));
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
    {
        Ok(file) => file,
        Err(_) => return RollbackResult::Failed,
    };
    use std::io::Write;
    if file.write_all(bytes).is_err()
        || file.sync_all().is_err()
        || std::fs::set_permissions(
            &temporary,
            std::fs::Permissions::from_mode(mode.unwrap_or(0o700)),
        )
        .is_err()
    {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return RollbackResult::Failed;
    }
    drop(file);
    let prior_identity = binary_file_identity_absolute(&temporary);
    if exchange_paths_cli(&temporary, destination).is_err() {
        let _ = std::fs::remove_file(&temporary);
        return RollbackResult::ReplacementPreserved;
    }
    // The first exchange is atomic, but another writer can still replace the
    // destination immediately afterwards.  Observe the destination inode
    // before classifying the rollback so a replacement is always surfaced as
    // recovery attention rather than a clean restoration.
    after_exchange();
    if binary_file_identity_absolute(destination) != prior_identity {
        let _ = std::fs::remove_file(&temporary);
        return RollbackResult::ReplacementPreserved;
    }
    let displaced = binary_file_identity_absolute(&temporary);
    if displaced == expected {
        let _ = std::fs::remove_file(&temporary);
        RollbackResult::Restored
    } else if exchange_paths_cli(&temporary, destination).is_ok() {
        // The displaced inode was not the one published by skilltap.  The
        // second exchange restores that unrelated replacement at the path;
        // the prior bytes remain private and can be removed safely.
        let _ = std::fs::remove_file(&temporary);
        RollbackResult::ReplacementPreserved
    } else {
        // Keep the residual rather than deleting a path whose identity is no
        // longer known.  The destination remains whatever the exchange left.
        RollbackResult::Failed
    }
}

#[cfg(not(unix))]
fn restore_previous_binary(
    _path: &AbsolutePath,
    _expected: Option<BinaryIdentity>,
    _bytes: &[u8],
) -> RollbackResult {
    // This publication boundary has no atomic no-replace/exchange primitive
    // on unsupported platforms.  Fail closed rather than overwrite a race.
    RollbackResult::Failed
}

#[cfg(unix)]
fn remove_published_binary(
    path: &AbsolutePath,
    expected: Option<BinaryIdentity>,
) -> RollbackResult {
    remove_published_binary_with_hooks(path, expected, || {}, |path| std::fs::remove_file(path))
}

#[cfg(unix)]
fn remove_published_binary_with_hooks(
    path: &AbsolutePath,
    expected: Option<BinaryIdentity>,
    after_rename: impl FnOnce(),
    remove_file: impl Fn(&std::path::Path) -> std::io::Result<()>,
) -> RollbackResult {
    let Some(expected) = expected else {
        return RollbackResult::Failed;
    };
    let destination = std::path::Path::new(path.as_str());
    let Some(parent) = destination.parent() else {
        return RollbackResult::Failed;
    };
    let marker = parent.join(format!(
        ".skilltap-rollback-cleanup-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("worker")
    ));
    if rename_noreplace_cli(destination, &marker).is_err() {
        return RollbackResult::ReplacementPreserved;
    }
    after_rename();
    if binary_file_identity_absolute(&marker) == Some(expected) {
        let destination_state = match std::fs::symlink_metadata(destination) {
            Ok(_) => Some(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return RollbackResult::Failed,
        };
        if destination_state.is_some() {
            // A replacement won the path after the no-replace move.  The
            // marker is still the expected published inode and can be
            // removed without touching the replacement.
            return match remove_file(&marker) {
                Ok(()) => RollbackResult::ReplacementPreserved,
                Err(_) => RollbackResult::Failed,
            };
        }
        match remove_file(&marker) {
            Ok(()) => RollbackResult::Removed,
            Err(_) => RollbackResult::Failed,
        }
    } else if rename_noreplace_cli(&marker, destination).is_ok() {
        RollbackResult::ReplacementPreserved
    } else {
        RollbackResult::Failed
    }
}

#[cfg(not(unix))]
fn remove_published_binary(
    _path: &AbsolutePath,
    _expected: Option<BinaryIdentity>,
) -> RollbackResult {
    RollbackResult::Failed
}

#[cfg(unix)]
fn binary_file_identity_absolute(path: &std::path::Path) -> Option<BinaryIdentity> {
    binary_file_identity_path(path)
}

fn binary_file_identity_path(path: &std::path::Path) -> Option<BinaryIdentity> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let digest = binary_file_digest(path)?;
    #[cfg(unix)]
    let location = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino())
    };
    #[cfg(not(unix))]
    let location = (0, metadata.len());
    Some(BinaryIdentity { location, digest })
}

#[cfg(unix)]
fn rename_noreplace_cli(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let result = -1;
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn exchange_paths_cli(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let result = -1;
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn binary_file_identity(path: &AbsolutePath) -> Option<BinaryIdentity> {
    binary_file_identity_path(std::path::Path::new(path.as_str()))
}

fn binary_file_digest(path: &std::path::Path) -> Option<[u8; 32]> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).ok()?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            return Some(digest.finalize().into());
        }
        digest.update(&buffer[..read]);
    }
}

fn probe_installed_version(
    path: &AbsolutePath,
) -> Option<skilltap_core::bootstrap::ReleaseVersion> {
    use skilltap_core::runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessRequest, NativeProcessRunner,
        ProcessLimits, SystemExecutableResolver, SystemNativeProcessRunner,
    };
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            skilltap_core::domain::ConfiguredBinary::absolute(path.clone()),
            None,
        ))
        .ok()?;
    let limits = ProcessLimits::new(5_000, 4 * 1024, 4 * 1024, 8 * 1024).ok()?;
    let output = SystemNativeProcessRunner
        .run(&NativeProcessRequest::new(
            executable,
            [std::ffi::OsString::from("--version")],
            std::collections::BTreeMap::new(),
            None,
            limits,
        ))
        .ok()?;
    if !output.status().success() {
        return None;
    }
    let text = String::from_utf8(output.stdout().to_vec()).ok()?;
    let mut fields = text.split_whitespace();
    if fields.next() != Some("skilltap") {
        return None;
    }
    let version = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    version.trim_start_matches('v').parse().ok()
}

fn binary_attention(code: &str, detail: &str) -> BinaryBootstrapResult {
    BinaryBootstrapResult {
        entry: OutputEntry::new("binary", "unavailable").with_field("policy", "latest-compatible"),
        attention: true,
        pending: true,
        changed: false,
        warnings: vec![crate::Warning::new(code, detail)],
        next_actions: vec![crate::NextAction::new(
            "bootstrap_help",
            "Run `skilltap bootstrap --help` for the release and platform requirements.",
        )],
    }
}

fn binary_pending(code: &str, detail: &str) -> BinaryBootstrapResult {
    BinaryBootstrapResult {
        entry: OutputEntry::new("binary", "pending").with_field("policy", "apply-safe"),
        attention: true,
        pending: true,
        changed: false,
        warnings: vec![crate::Warning::new(code, detail)],
        next_actions: vec![crate::NextAction::new(
            "retry_binary_update",
            "Retry the update after the current skilltap mutation finishes.",
        )],
    }
}

/// Apply the persisted self-update policy once per daemon cycle.  This uses
/// the same resolver/fetcher/installer boundary as foreground bootstrap; the
/// application service only merges its bounded result with resource updates.
pub(super) fn execute_system_daemon_binary_policy() -> Outcome {
    use skilltap_core::bootstrap::BootstrapUpdateMode;
    let command = "daemon run";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let filesystem = SystemFileSystem;
    let repository = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(command),
    };
    let config = match repository.load() {
        Ok(DocumentState::Missing) => ConfigDocument::defaults(),
        Ok(DocumentState::Present(config)) => config,
        Err(_) => return repository_composition_error(command),
    };
    let policy = config.bootstrap();
    let result = match policy.mode {
        BootstrapUpdateMode::Off => BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "disabled").with_field("policy", "off"),
            attention: false,
            pending: false,
            changed: false,
            warnings: Vec::new(),
            next_actions: Vec::new(),
        },
        BootstrapUpdateMode::Check | BootstrapUpdateMode::ApplySafe => {
            let destination = match daemon_binary_destination(&paths) {
                Ok(destination) => destination,
                Err(detail) => return binary_policy_attention(detail),
            };
            let args = BootstrapArgs {
                target: None,
                allow_major: policy.allow_major,
                output: OutputArgs::default(),
            };
            let key = match skilltap_core::bootstrap::ArtifactKey::current() {
                Ok(key) => key,
                Err(_) => {
                    return binary_policy_attention(
                        "This platform has no published skilltap bootstrap artifact.",
                    );
                }
            };
            let resolver = skilltap_core::runtime::SystemReleaseResolver::current(key);
            let fetcher = skilltap_core::runtime::SystemArtifactFetcher;
            let installer = skilltap_core::runtime::SystemBinaryInstaller;
            let lock_path = match AbsolutePath::new(format!(
                "{}/skilltap.lock",
                paths.skilltap_config().as_str()
            )) {
                Ok(path) => path,
                Err(_) => {
                    return binary_policy_attention(
                        "The skilltap binary update lock path is invalid.",
                    );
                }
            };
            execute_binary_bootstrap_with_lock(
                &args,
                BinaryUpdateTarget {
                    destination,
                    lock_path,
                },
                &resolver,
                &fetcher,
                &installer,
                &SystemConfigurationLock,
                match policy.mode {
                    BootstrapUpdateMode::Check => BinaryExecutionMode::Check,
                    BootstrapUpdateMode::ApplySafe => BinaryExecutionMode::ApplySafe,
                    BootstrapUpdateMode::Off => unreachable!(),
                },
            )
        }
    };
    let mut outcome = Outcome::new(command, ResultClass::Completed)
        .with_scope(OutputScope::Global)
        .with_resource(result.entry)
        .with_summary("binary_changed", result.changed)
        .with_summary("binary_pending", result.pending)
        .with_summary("binary_policy", format!("{:?}", policy.mode).to_lowercase());
    if result.attention {
        outcome.result = ResultClass::AttentionRequired;
    }
    for warning in result.warnings {
        outcome = outcome.with_warning(warning);
    }
    for action in result.next_actions {
        outcome = outcome.with_next_action(action);
    }
    outcome
}

fn binary_policy_attention(detail: &str) -> Outcome {
    Outcome::new("daemon run", ResultClass::AttentionRequired)
        .with_scope(OutputScope::Global)
        .with_resource(OutputEntry::new("binary", "unavailable"))
        .with_summary("binary_changed", false)
        .with_summary("binary_pending", true)
        .with_warning(crate::Warning::new(
            "daemon_binary_target_unavailable",
            detail,
        ))
}

fn daemon_binary_destination(paths: &PlatformPaths) -> Result<AbsolutePath, &'static str> {
    let platform = crate::daemon::platform(paths);
    let root = crate::daemon::root(paths, platform);
    let name = match platform {
        skilltap_core::daemon::ServicePlatform::Launchd => {
            format!("{}.plist", skilltap_core::daemon::SERVICE_LABEL)
        }
        skilltap_core::daemon::ServicePlatform::SystemdUser => {
            skilltap_core::daemon::SYSTEMD_UNIT.to_owned()
        }
    };
    let path = AbsolutePath::new(format!("{}/{}", root.as_str(), name))
        .map_err(|_| "The daemon service definition path is invalid.")?;
    let contents = SystemFileSystem
        .read_regular_no_follow(&path)
        .map_err(|_| "The daemon service definition could not be read safely.")?
        .ok_or("The daemon service is not enabled, so its binary destination is unknown.")?;
    crate::daemon::executable_from_service(platform, &contents)
        .ok_or("The managed daemon service definition is malformed or has no executable target.")
}

#[cfg(test)]
mod bootstrap_tests {
    use std::{fs, path::Path, sync::Arc};

    use sha2::Digest;
    use skilltap_core::{
        bootstrap::{ArtifactKey, ReleaseArtifact, ReleaseVersion},
        domain::{AbsolutePath, HarnessId, SourceLocator, TargetSelection},
        runtime::{
            ArtifactError, ArtifactFetcher, BinaryInstaller, ConfigurationLock,
            ConfigurationLockGuard, InstalledBinary, ReleaseManifest, ReleaseResolver,
            RuntimeError, SystemBinaryInstaller, SystemConfigurationLock,
        },
    };
    use skilltap_test_support::TempRoot;

    use super::{
        BinaryBootstrapResult, BinaryExecutionMode, BinaryUpdateTarget, BootstrapArgs, OutputArgs,
        RollbackResult, compose_bootstrap_outcome, execute_binary_bootstrap_with,
        execute_binary_bootstrap_with_lock, execute_binary_bootstrap_with_mode,
        remove_published_binary, remove_published_binary_with_hooks, restore_previous_binary,
        restore_previous_binary_with_hook,
    };
    use crate::{JsonRenderer, PlainRenderer, Renderer};

    #[derive(Clone)]
    struct FixtureResolver {
        manifest: ReleaseManifest,
    }

    impl ReleaseResolver for FixtureResolver {
        fn latest(&self) -> Result<ReleaseManifest, ArtifactError> {
            Ok(self.manifest.clone())
        }
    }

    #[derive(Clone)]
    struct FixtureFetcher {
        bytes: Arc<Vec<u8>>,
    }

    impl ArtifactFetcher for FixtureFetcher {
        fn fetch(&self, _url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError> {
            fs::write(destination.as_str(), self.bytes.as_ref())
                .map_err(|_| ArtifactError::DownloadFailed)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(destination.as_str(), fs::Permissions::from_mode(0o700))
                    .map_err(|_| ArtifactError::DownloadFailed)?;
            }
            Ok(())
        }
    }

    struct NonExecutableFetcher {
        bytes: Vec<u8>,
    }

    impl ArtifactFetcher for NonExecutableFetcher {
        fn fetch(&self, _url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError> {
            fs::write(destination.as_str(), &self.bytes).map_err(|_| ArtifactError::DownloadFailed)
        }
    }

    #[cfg(unix)]
    struct ObservedChecksumMismatchFetcher {
        bytes: Vec<u8>,
        observed: std::path::PathBuf,
    }

    #[cfg(unix)]
    impl ArtifactFetcher for ObservedChecksumMismatchFetcher {
        fn fetch(&self, _url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError> {
            use std::os::unix::fs::PermissionsExt;

            fs::write(destination.as_str(), &self.bytes)
                .map_err(|_| ArtifactError::DownloadFailed)?;
            fs::set_permissions(destination.as_str(), fs::Permissions::from_mode(0o600))
                .map_err(|_| ArtifactError::DownloadFailed)?;
            fs::hard_link(destination.as_str(), &self.observed)
                .map_err(|_| ArtifactError::DownloadFailed)
        }
    }

    struct WrongPublisher;

    struct ContendedLock;

    impl ConfigurationLock for ContendedLock {
        type Guard = NeverGuard;

        fn try_acquire(&self, path: &AbsolutePath) -> Result<Self::Guard, RuntimeError> {
            Err(RuntimeError::LockContended { path: path.clone() })
        }
    }

    struct NeverGuard;

    impl ConfigurationLockGuard for NeverGuard {
        fn path(&self) -> &AbsolutePath {
            panic!("a contended lock never returns a guard")
        }

        fn release(self) -> Result<(), RuntimeError> {
            Ok(())
        }
    }

    impl BinaryInstaller for WrongPublisher {
        fn inspect(&self, path: &AbsolutePath) -> Result<Option<InstalledBinary>, ArtifactError> {
            SystemBinaryInstaller.inspect(path)
        }

        fn install_verified(
            &self,
            _artifact: &AbsolutePath,
            destination: &AbsolutePath,
            _expected: &ReleaseArtifact,
        ) -> Result<(), ArtifactError> {
            let path = Path::new(destination.as_str());
            fs::write(path, b"#!/bin/sh\nprintf 'skilltap 2.0.0\\n'\n")
                .map_err(|_| ArtifactError::InstallFailed)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|_| ArtifactError::InstallFailed)?;
            }
            Ok(())
        }
    }

    fn args(allow_major: bool) -> BootstrapArgs {
        BootstrapArgs {
            target: None,
            allow_major,
            output: OutputArgs { json: false },
        }
    }

    fn key() -> ArtifactKey {
        ArtifactKey::current().expect("tests run on a supported release host")
    }

    fn fixture(version: &str) -> (FixtureResolver, FixtureFetcher) {
        let version = version.parse::<ReleaseVersion>().unwrap();
        let bytes = format!("#!/bin/sh\nprintf 'skilltap {version}\\n'\n").into_bytes();
        let key = key();
        let artifact = ReleaseArtifact::new(
            version,
            key,
            "skilltap-fixture",
            format!("{:x}", sha2::Sha256::digest(&bytes)),
            SourceLocator::new(
                "https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap-fixture",
            )
            .unwrap(),
        )
        .unwrap();
        (
            FixtureResolver {
                manifest: ReleaseManifest::new(version, [artifact]).unwrap(),
            },
            FixtureFetcher {
                bytes: Arc::new(bytes),
            },
        )
    }

    #[test]
    fn isolated_matrix_covers_install_noop_update_major_block_and_opt_in() {
        let root = TempRoot::new("bootstrap-command-matrix").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();

        let (resolver, fetcher) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "installed");
        assert!(result.changed);
        assert!(!result.attention);
        assert!(!result.pending);
        assert!(result.warnings.is_empty());
        assert!(result.next_actions.is_empty());

        let (resolver, fetcher) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "no-op");
        assert!(!result.changed);
        assert!(!result.attention);
        assert!(!result.pending);
        assert!(result.warnings.is_empty());
        assert!(result.next_actions.is_empty());

        let (resolver, fetcher) = fixture("3.1.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "updated");

        let prior = fs::read(destination.as_str()).unwrap();
        let (resolver, fetcher) = fixture("4.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "major-upgrade-blocked");
        assert_eq!(fs::read(destination.as_str()).unwrap(), prior);

        let (resolver, fetcher) = fixture("4.0.0");
        let result = execute_binary_bootstrap_with(
            &args(true),
            destination,
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "updated");
    }

    #[test]
    fn non_executable_download_is_verified_before_being_made_runnable() {
        let root = TempRoot::new("bootstrap-non-executable-download").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        let (resolver, _) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &NonExecutableFetcher {
                bytes: b"#!/bin/sh\nprintf 'skilltap 3.0.0\\n'\n".to_vec(),
            },
            &SystemBinaryInstaller,
        );

        assert_eq!(result.entry.status, "installed");
        assert!(result.changed);
        assert!(!result.attention);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(destination.as_str())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn checksum_mismatch_is_neither_made_executable_nor_run() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempRoot::new("bootstrap-checksum-before-execution").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        let observed = root.path().join("downloaded-payload");
        let marker = root.path().join("payload-ran");
        let (resolver, _) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination,
            &resolver,
            &ObservedChecksumMismatchFetcher {
                bytes: format!(
                    "#!/bin/sh\ntouch '{}'\nprintf 'skilltap 3.0.0\\n'\n",
                    marker.display()
                )
                .into_bytes(),
                observed: observed.clone(),
            },
            &SystemBinaryInstaller,
        );

        assert_eq!(result.entry.status, "unavailable");
        assert_eq!(result.warnings[0].code, "release_checksum_failed");
        assert_eq!(
            fs::metadata(&observed).unwrap().permissions().mode() & 0o111,
            0
        );
        assert!(!marker.exists());
    }

    #[test]
    fn daemon_check_resolves_updates_without_fetching_or_publishing() {
        let root = TempRoot::new("bootstrap-daemon-check").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        let (resolver, fetcher) = fixture("3.0.0");
        let installed = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(installed.entry.status, "installed");
        let prior = fs::read(destination.as_str()).unwrap();
        let (resolver, fetcher) = fixture("3.1.0");
        let result = execute_binary_bootstrap_with_mode(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
            BinaryExecutionMode::Check,
        );
        assert_eq!(result.entry.status, "update-available");
        assert!(result.pending);
        assert!(!result.changed);
        assert_eq!(fs::read(destination.as_str()).unwrap(), prior);
    }

    #[test]
    fn daemon_binary_lock_contention_is_pending_without_resolver_or_fetcher() {
        let root = TempRoot::new("bootstrap-daemon-lock-contention").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("custom/skilltap").display().to_string()).unwrap();
        let lock_path =
            AbsolutePath::new(root.path().join("skilltap.lock").display().to_string()).unwrap();
        let (resolver, fetcher) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with_lock(
            &args(false),
            BinaryUpdateTarget {
                destination,
                lock_path,
            },
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
            &ContendedLock,
            BinaryExecutionMode::ApplySafe,
        );
        assert_eq!(result.entry.status, "pending");
        assert!(result.attention);
        assert!(result.pending);
        assert!(!result.changed);
        assert_eq!(result.warnings[0].code, "configuration_locked");
    }

    #[test]
    fn daemon_binary_update_uses_custom_destination_while_locked() {
        let root = TempRoot::new("bootstrap-daemon-custom-target").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("custom/skilltap").display().to_string()).unwrap();
        let lock_path =
            AbsolutePath::new(root.path().join("skilltap.lock").display().to_string()).unwrap();
        let (resolver, fetcher) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with_lock(
            &args(false),
            BinaryUpdateTarget {
                destination: destination.clone(),
                lock_path,
            },
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
            &SystemConfigurationLock,
            BinaryExecutionMode::ApplySafe,
        );
        assert_eq!(result.entry.status, "installed");
        assert!(Path::new(destination.as_str()).exists());
    }

    #[cfg(unix)]
    #[test]
    fn rollback_exchange_preserves_replacement_and_restores_matching_publish() {
        let root = TempRoot::new("bootstrap-cli-rollback").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::write(destination.as_str(), b"published").unwrap();
        let published = super::binary_file_identity(&destination);
        let replacement = b"replacement";
        let replacement_path = root.path().join("replacement");
        fs::write(&replacement_path, replacement).unwrap();
        fs::rename(&replacement_path, destination.as_str()).unwrap();
        let result = restore_previous_binary(&destination, published, b"prior", Some(0o700));
        assert_eq!(result, RollbackResult::ReplacementPreserved);
        assert_eq!(fs::read(destination.as_str()).unwrap(), replacement);

        fs::write(destination.as_str(), b"published-again").unwrap();
        let published = super::binary_file_identity(&destination);
        let result = restore_previous_binary(&destination, published, b"prior-again", Some(0o700));
        assert_eq!(result, RollbackResult::Restored);
        assert_eq!(fs::read(destination.as_str()).unwrap(), b"prior-again");
    }

    #[cfg(unix)]
    #[test]
    fn rollback_replacement_during_exchange_is_attention_and_preserved() {
        let root = TempRoot::new("bootstrap-cli-rollback-during").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::write(destination.as_str(), b"published").unwrap();
        let published = super::binary_file_identity(&destination);
        let replacement = root.path().join("replacement-during");
        fs::write(&replacement, b"replacement-during").unwrap();
        let result = restore_previous_binary_with_hook(
            &destination,
            published,
            b"prior",
            Some(0o700),
            || {
                fs::rename(&replacement, destination.as_str()).unwrap();
            },
        );
        assert_eq!(result, RollbackResult::ReplacementPreserved);
        assert_eq!(
            fs::read(destination.as_str()).unwrap(),
            b"replacement-during"
        );
    }

    #[cfg(unix)]
    #[test]
    fn first_install_cleanup_only_removes_expected_identity() {
        let root = TempRoot::new("bootstrap-cli-cleanup").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::write(destination.as_str(), b"published").unwrap();
        let published = super::binary_file_identity(&destination);
        assert_eq!(
            remove_published_binary(&destination, published),
            RollbackResult::Removed
        );
        assert!(!Path::new(destination.as_str()).exists());

        let replacement_path = root.path().join("replacement-2");
        fs::write(&replacement_path, b"replacement").unwrap();
        fs::rename(&replacement_path, destination.as_str()).unwrap();
        assert_eq!(
            remove_published_binary(&destination, published),
            RollbackResult::ReplacementPreserved
        );
        assert_eq!(fs::read(destination.as_str()).unwrap(), b"replacement");
    }

    #[cfg(unix)]
    #[test]
    fn first_install_cleanup_preserves_replacement_and_reports_residual() {
        let root = TempRoot::new("bootstrap-cli-cleanup-races").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::write(destination.as_str(), b"published").unwrap();
        let published = super::binary_file_identity(&destination);
        let replacement = root.path().join("replacement-during-cleanup");
        fs::write(&replacement, b"replacement-during-cleanup").unwrap();
        let result = remove_published_binary_with_hooks(
            &destination,
            published,
            || {
                fs::rename(&replacement, destination.as_str()).unwrap();
            },
            |path| fs::remove_file(path),
        );
        assert_eq!(result, RollbackResult::ReplacementPreserved);
        assert_eq!(
            fs::read(destination.as_str()).unwrap(),
            b"replacement-during-cleanup"
        );

        fs::write(destination.as_str(), b"published-residual").unwrap();
        let published = super::binary_file_identity(&destination);
        let result = remove_published_binary_with_hooks(
            &destination,
            published,
            || {},
            |_| Err(std::io::Error::other("test residual")),
        );
        assert_eq!(result, RollbackResult::Failed);
        assert!(
            fs::read_dir(root.path().join("bin"))
                .unwrap()
                .flatten()
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".skilltap-rollback-cleanup-"))
        );
    }

    #[test]
    fn wrong_release_identity_and_post_publish_identity_preserve_prior_binary() {
        let root = TempRoot::new("bootstrap-command-failures").unwrap();
        let destination =
            AbsolutePath::new(root.path().join("bin/skilltap").display().to_string()).unwrap();
        let (resolver, fetcher) = fixture("3.0.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "installed");
        let prior = fs::read(destination.as_str()).unwrap();

        let wrong_bytes = Arc::new(b"#!/bin/sh\nprintf 'skilltap 9.9.9\\n'\n".to_vec());
        let key = key();
        let version = "3.1.0".parse::<ReleaseVersion>().unwrap();
        let artifact = ReleaseArtifact::new(
            version,
            key,
            "skilltap-fixture",
            format!("{:x}", sha2::Sha256::digest(wrong_bytes.as_ref())),
            SourceLocator::new(
                "https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap-fixture",
            )
            .unwrap(),
        )
        .unwrap();
        let resolver = FixtureResolver {
            manifest: ReleaseManifest::new(version, [artifact]).unwrap(),
        };
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &FixtureFetcher { bytes: wrong_bytes },
            &SystemBinaryInstaller,
        );
        assert_eq!(result.entry.status, "unavailable");
        assert_eq!(fs::read(destination.as_str()).unwrap(), prior);

        let (resolver, fetcher) = fixture("3.1.0");
        let result = execute_binary_bootstrap_with(
            &args(false),
            destination.clone(),
            &resolver,
            &fetcher,
            &WrongPublisher,
        );
        assert_eq!(result.entry.status, "unavailable");
        assert_eq!(fs::read(destination.as_str()).unwrap(), prior);
    }

    fn completed_binary() -> BinaryBootstrapResult {
        BinaryBootstrapResult {
            entry: super::OutputEntry::new("binary", "no-op"),
            attention: false,
            pending: false,
            changed: false,
            warnings: Vec::new(),
            next_actions: Vec::new(),
        }
    }

    #[test]
    fn composed_bootstrap_contract_keeps_target_narrowing_and_json_schema_stable() {
        let args = BootstrapArgs {
            target: Some(TargetSelection::Only(HarnessId::new("claude").unwrap())),
            allow_major: false,
            output: OutputArgs { json: true },
        };
        let outcome = compose_bootstrap_outcome(
            &args,
            completed_binary(),
            vec![
                (
                    skilltap_harnesses::HarnessKind::Codex,
                    skilltap_harnesses::HarnessSetupResult::Installed {
                        harness: skilltap_harnesses::HarnessKind::Codex,
                        version: skilltap_core::domain::NativeVersion::new("3.0.0").unwrap(),
                    },
                ),
                (
                    skilltap_harnesses::HarnessKind::Claude,
                    skilltap_harnesses::HarnessSetupResult::AlreadyPresent {
                        harness: skilltap_harnesses::HarnessKind::Claude,
                        version: skilltap_core::domain::NativeVersion::new("3.0.0").unwrap(),
                    },
                ),
            ],
        );
        let json = JsonRenderer.render(&outcome).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema"], 1);
        assert_eq!(value["result"], "completed");
        assert_eq!(value["resources"].as_array().unwrap().len(), 2);
        assert_eq!(value["resources"][1]["id"], "claude");
        assert!(!json.contains("codex"));
        let plain = PlainRenderer.render(&outcome).unwrap();
        assert!(plain.contains("binary  no-op"));
        assert!(plain.contains("claude  already-present"));
        assert!(!plain.contains("codex"));
    }

    #[test]
    fn composed_bootstrap_contract_reports_absent_and_mixed_harness_attention() {
        let args = BootstrapArgs {
            target: None,
            allow_major: false,
            output: OutputArgs { json: false },
        };
        let outcome = compose_bootstrap_outcome(
            &args,
            completed_binary(),
            vec![
                (
                    skilltap_harnesses::HarnessKind::Claude,
                    skilltap_harnesses::HarnessSetupResult::Installed {
                        harness: skilltap_harnesses::HarnessKind::Claude,
                        version: skilltap_core::domain::NativeVersion::new("3.0.0").unwrap(),
                    },
                ),
                (
                    skilltap_harnesses::HarnessKind::Codex,
                    skilltap_harnesses::HarnessSetupResult::Unavailable {
                        harness: skilltap_harnesses::HarnessKind::Codex,
                        reason: skilltap_harnesses::SetupReason::NotInstalled,
                    },
                ),
            ],
        );
        assert_eq!(outcome.result, super::ResultClass::AttentionRequired);
        let json = JsonRenderer.render(&outcome).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema"], 1);
        let resources = value["resources"].as_array().unwrap();
        let codex = resources
            .iter()
            .find(|resource| resource["id"] == "codex")
            .unwrap();
        assert_eq!(codex["status"], "unavailable");
        assert_eq!(value["next_actions"][0]["code"], "bootstrap_codex");
        let plain = PlainRenderer.render(&outcome).unwrap();
        assert!(plain.contains("codex  unavailable"));
        assert!(plain.contains("not installed"));
    }
}
