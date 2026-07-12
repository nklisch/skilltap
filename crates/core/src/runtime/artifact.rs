//! Bounded release transport and atomic user-level binary publication.

use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::ffi::OsStrExt;

use sha2::{Digest, Sha256};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::{
    bootstrap::{ArtifactKey, ReleaseArtifact, ReleaseVersion},
    domain::{AbsolutePath, Fingerprint, FingerprintAlgorithm, SourceLocator},
};

use super::{
    ExecutableResolutionRequest, ExecutableResolver, NativeProcessRequest, NativeProcessRunner,
    ProcessLimits, SystemExecutableResolver, SystemNativeProcessRunner,
};

static ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactError {
    InvalidManifest(&'static str),
    InvalidLocator,
    UnsupportedAsset,
    ChecksumMismatch,
    DownloadFailed,
    InstallFailed,
    DestinationChanged,
    InvalidArtifact,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidManifest(detail) => detail,
            Self::InvalidLocator => "release locator is not an allowed HTTPS GitHub release URL",
            Self::UnsupportedAsset => {
                "the release does not contain exactly one supported platform asset"
            }
            Self::ChecksumMismatch => {
                "downloaded release asset checksum does not match the signed release manifest"
            }
            Self::DownloadFailed => "release asset download failed",
            Self::InstallFailed => "verified release binary could not be published atomically",
            Self::DestinationChanged => {
                "binary destination changed while the release was being installed"
            }
            Self::InvalidArtifact => "release artifact is not a regular executable file",
        })
    }
}

impl std::error::Error for ArtifactError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseManifest {
    pub version: ReleaseVersion,
    artifacts: Vec<ReleaseArtifact>,
}

impl ReleaseManifest {
    pub fn new(
        version: ReleaseVersion,
        artifacts: impl IntoIterator<Item = ReleaseArtifact>,
    ) -> Result<Self, ArtifactError> {
        let artifacts = artifacts.into_iter().collect::<Vec<_>>();
        if artifacts.is_empty() {
            return Err(ArtifactError::InvalidManifest(
                "release manifest contains no assets",
            ));
        }
        Ok(Self { version, artifacts })
    }

    pub fn parse(value: &serde_json::Value, key: ArtifactKey) -> Result<Self, ArtifactError> {
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_str)
            .ok_or(ArtifactError::InvalidManifest(
                "release manifest is missing a version",
            ))?
            .parse()
            .map_err(|_| ArtifactError::InvalidManifest("release version is malformed"))?;
        let entries = value
            .get("assets")
            .and_then(serde_json::Value::as_array)
            .ok_or(ArtifactError::InvalidManifest(
                "release manifest assets must be an array",
            ))?;
        let mut matches = Vec::new();
        for entry in entries {
            let object = entry.as_object().ok_or(ArtifactError::InvalidManifest(
                "release asset entry must be an object",
            ))?;
            let platform = object
                .get("platform")
                .and_then(serde_json::Value::as_str)
                .ok_or(ArtifactError::InvalidManifest(
                    "release asset platform is missing",
                ))?;
            let arch = object
                .get("arch")
                .and_then(serde_json::Value::as_str)
                .ok_or(ArtifactError::InvalidManifest(
                    "release asset architecture is missing",
                ))?;
            let candidate_key = ArtifactKey {
                platform: match platform {
                    "linux" => super::SupportedPlatform::Linux,
                    "macos" => super::SupportedPlatform::MacOs,
                    _ => continue,
                },
                arch: match arch {
                    "x86_64" => crate::bootstrap::ArtifactArch::X86_64,
                    "aarch64" => crate::bootstrap::ArtifactArch::Aarch64,
                    _ => continue,
                },
            };
            if candidate_key != key {
                continue;
            }
            let name = object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or(ArtifactError::InvalidManifest(
                    "release asset name is missing",
                ))?;
            let sha256 = object
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or(ArtifactError::InvalidManifest(
                    "release asset sha256 is missing",
                ))?;
            let url = object
                .get("download_url")
                .and_then(serde_json::Value::as_str)
                .ok_or(ArtifactError::InvalidManifest(
                    "release asset download URL is missing",
                ))?;
            let locator = SourceLocator::new(url).map_err(|_| {
                ArtifactError::InvalidManifest("release asset download URL is invalid")
            })?;
            matches.push(
                ReleaseArtifact::new(version, candidate_key, name, sha256, locator).map_err(
                    |_| ArtifactError::InvalidManifest("release asset metadata is invalid"),
                )?,
            );
        }
        if matches.len() != 1 {
            return Err(ArtifactError::UnsupportedAsset);
        }
        Self::new(version, matches)
    }

    pub fn artifact(&self, key: ArtifactKey) -> Result<&ReleaseArtifact, ArtifactError> {
        let mut matches = self
            .artifacts
            .iter()
            .filter(|artifact| artifact.key() == &key);
        let first = matches.next().ok_or(ArtifactError::UnsupportedAsset)?;
        if matches.next().is_some() {
            return Err(ArtifactError::UnsupportedAsset);
        }
        Ok(first)
    }
}

pub trait ReleaseResolver {
    fn latest(&self) -> Result<ReleaseManifest, ArtifactError>;
}

/// Resolves a release manifest from an isolated local file.  The CLI uses
/// this only for deterministic fixture/bootstrap environments; production
/// callers use [`SystemReleaseResolver`].
#[derive(Clone, Debug)]
pub struct FileReleaseResolver {
    path: AbsolutePath,
    key: ArtifactKey,
}

impl FileReleaseResolver {
    pub const fn new(path: AbsolutePath, key: ArtifactKey) -> Self {
        Self { path, key }
    }
}

impl ReleaseResolver for FileReleaseResolver {
    fn latest(&self) -> Result<ReleaseManifest, ArtifactError> {
        let bytes = fs::read(self.path.as_str()).map_err(|_| ArtifactError::DownloadFailed)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| ArtifactError::InvalidManifest("release manifest is not valid JSON"))?;
        ReleaseManifest::parse(&value, self.key)
    }
}

/// Bounded production resolver for the canonical GitHub latest-release
/// endpoint. The fetched document is removed before returning to the caller.
#[derive(Clone, Debug)]
pub struct SystemReleaseResolver<F = SystemArtifactFetcher> {
    url: SourceLocator,
    key: ArtifactKey,
    fetcher: F,
}

impl SystemReleaseResolver<SystemArtifactFetcher> {
    pub fn current(key: ArtifactKey) -> Self {
        Self {
            url: SourceLocator::new(
                "https://api.github.com/repos/nklisch/skilltap/releases/latest",
            )
            .expect("canonical release endpoint is valid"),
            key,
            fetcher: SystemArtifactFetcher,
        }
    }
}

impl<F> SystemReleaseResolver<F> {
    pub fn with_fetcher(url: SourceLocator, key: ArtifactKey, fetcher: F) -> Self {
        Self { url, key, fetcher }
    }
}

impl<F: ArtifactFetcher> ReleaseResolver for SystemReleaseResolver<F> {
    fn latest(&self) -> Result<ReleaseManifest, ArtifactError> {
        let (workspace, path) = private_temp_file("manifest")?;
        let result = self.fetcher.fetch(self.url.as_str(), &path).and_then(|()| {
            let bytes = read_bounded(&path, 4 * 1024 * 1024)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
                ArtifactError::InvalidManifest("release manifest is not valid JSON")
            })?;
            if value.get("tag_name").is_some() {
                self.parse_github_release(&value)
            } else {
                ReleaseManifest::parse(&value, self.key)
            }
        });
        let _ = fs::remove_dir_all(workspace.as_str());
        result
    }
}

impl<F: ArtifactFetcher> SystemReleaseResolver<F> {
    fn parse_github_release(
        &self,
        value: &serde_json::Value,
    ) -> Result<ReleaseManifest, ArtifactError> {
        let version_text = value
            .get("tag_name")
            .and_then(serde_json::Value::as_str)
            .ok_or(ArtifactError::InvalidManifest(
                "GitHub release tag is missing",
            ))?;
        let version = version_text
            .strip_prefix('v')
            .unwrap_or(version_text)
            .parse()
            .map_err(|_| ArtifactError::InvalidManifest("GitHub release tag is malformed"))?;
        let expected_name = github_asset_name(self.key);
        let assets = value
            .get("assets")
            .and_then(serde_json::Value::as_array)
            .ok_or(ArtifactError::InvalidManifest(
                "GitHub release assets are missing",
            ))?;
        let matches = assets
            .iter()
            .filter(|asset| {
                asset.get("name").and_then(serde_json::Value::as_str) == Some(expected_name)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(ArtifactError::UnsupportedAsset);
        }
        let asset = matches[0];
        let download_url = asset
            .get("browser_download_url")
            .and_then(serde_json::Value::as_str)
            .ok_or(ArtifactError::InvalidManifest(
                "GitHub release asset URL is missing",
            ))?;
        let checksums_url = format!(
            "https://github.com/nklisch/skilltap/releases/download/v{version}/checksums.txt"
        );
        let (checksum_workspace, checksum_path) = private_temp_file("checksums")?;
        let checksum = self
            .fetcher
            .fetch(&checksums_url, &checksum_path)
            .and_then(|()| {
                let contents = String::from_utf8(read_bounded(&checksum_path, 1024 * 1024)?)
                    .map_err(|_| ArtifactError::DownloadFailed)?;
                parse_checksum(&contents, expected_name)
            });
        let _ = fs::remove_dir_all(checksum_workspace.as_str());
        let checksum = checksum?;
        let artifact = ReleaseArtifact::new(
            version,
            self.key,
            expected_name,
            checksum,
            SourceLocator::new(download_url)
                .map_err(|_| ArtifactError::InvalidManifest("GitHub asset URL is invalid"))?,
        )
        .map_err(|_| ArtifactError::InvalidManifest("GitHub release asset metadata is invalid"))?;
        ReleaseManifest::new(version, [artifact])
    }
}

fn private_temp_file(label: &str) -> Result<(AbsolutePath, AbsolutePath), ArtifactError> {
    for _ in 0..64 {
        let workspace_path = std::env::temp_dir().join(format!(
            ".skilltap-{label}-{}-{}",
            std::process::id(),
            ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&workspace_path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&workspace_path, fs::Permissions::from_mode(0o700))
                        .map_err(|_| ArtifactError::DownloadFailed)?;
                }
                let workspace = AbsolutePath::new(workspace_path.to_string_lossy().into_owned())
                    .map_err(|_| ArtifactError::DownloadFailed)?;
                let file = AbsolutePath::new(
                    workspace_path
                        .join("payload")
                        .to_string_lossy()
                        .into_owned(),
                )
                .map_err(|_| ArtifactError::DownloadFailed)?;
                return Ok((workspace, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(ArtifactError::DownloadFailed),
        }
    }
    Err(ArtifactError::DownloadFailed)
}

fn read_bounded(path: &AbsolutePath, limit: u64) -> Result<Vec<u8>, ArtifactError> {
    let metadata =
        fs::symlink_metadata(path.as_str()).map_err(|_| ArtifactError::DownloadFailed)?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_file()
        || metadata.len() > limit
    {
        return Err(ArtifactError::DownloadFailed);
    }
    let file = fs::File::open(path.as_str()).map_err(|_| ArtifactError::DownloadFailed)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ArtifactError::DownloadFailed)?;
    if bytes.len() as u64 > limit {
        return Err(ArtifactError::DownloadFailed);
    }
    Ok(bytes)
}

fn github_asset_name(key: ArtifactKey) -> &'static str {
    match (key.platform, key.arch) {
        (super::SupportedPlatform::Linux, crate::bootstrap::ArtifactArch::X86_64) => {
            "skilltap-linux-x64"
        }
        (super::SupportedPlatform::Linux, crate::bootstrap::ArtifactArch::Aarch64) => {
            "skilltap-linux-arm64"
        }
        (super::SupportedPlatform::MacOs, crate::bootstrap::ArtifactArch::X86_64) => {
            "skilltap-darwin-x64"
        }
        (super::SupportedPlatform::MacOs, crate::bootstrap::ArtifactArch::Aarch64) => {
            "skilltap-darwin-arm64"
        }
    }
}

fn parse_checksum(contents: &str, expected_name: &str) -> Result<String, ArtifactError> {
    let mut found = None;
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(digest) = fields.next() else {
            continue;
        };
        let Some(name) = fields.next().map(|name| name.trim_start_matches('*')) else {
            continue;
        };
        if name == expected_name {
            if found.is_some() {
                return Err(ArtifactError::InvalidManifest(
                    "release checksums contain duplicate assets",
                ));
            }
            found = Some(digest.to_owned());
        }
    }
    found.ok_or(ArtifactError::UnsupportedAsset)
}

pub trait ArtifactFetcher {
    fn fetch(&self, url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError>;
}

pub trait BinaryInstaller {
    fn inspect(&self, path: &AbsolutePath) -> Result<Option<InstalledBinary>, ArtifactError>;
    fn install_verified(
        &self,
        artifact: &AbsolutePath,
        destination: &AbsolutePath,
        expected: &ReleaseArtifact,
    ) -> Result<(), ArtifactError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledBinary {
    pub path: AbsolutePath,
    pub fingerprint: Fingerprint,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemArtifactFetcher;

impl ArtifactFetcher for SystemArtifactFetcher {
    fn fetch(&self, url: &str, destination: &AbsolutePath) -> Result<(), ArtifactError> {
        validate_release_url(url)?;
        let configured = crate::domain::ConfiguredBinary::path_lookup(
            crate::domain::NativeId::new("curl").map_err(|_| ArtifactError::InvalidLocator)?,
        )
        .map_err(|_| ArtifactError::InvalidLocator)?;
        let executable = SystemExecutableResolver
            .resolve(&ExecutableResolutionRequest::new(
                configured,
                std::env::var_os("PATH"),
            ))
            .map_err(|_| ArtifactError::DownloadFailed)?;
        let limits = ProcessLimits::new(30_000, 4 * 1024, 4 * 1024, 8 * 1024)
            .map_err(|_| ArtifactError::DownloadFailed)?;
        let mut current = url.to_owned();
        for _ in 0..6 {
            validate_release_url(&current)?;
            let output = SystemNativeProcessRunner
                .run(&NativeProcessRequest::new(
                    executable.clone(),
                    [
                        "--fail",
                        "--silent",
                        "--show-error",
                        "--max-redirs",
                        "0",
                        "--proto",
                        "=https",
                        "--proto-redir",
                        "=https",
                        "--max-time",
                        "30",
                        "--max-filesize",
                        "67108864",
                        "--output",
                        destination.as_str(),
                        "--write-out",
                        "%{http_code}\n%{redirect_url}",
                        "--",
                        &current,
                    ]
                    .into_iter()
                    .map(std::ffi::OsString::from),
                    std::collections::BTreeMap::new(),
                    None,
                    limits,
                ))
                .map_err(|_| ArtifactError::DownloadFailed)?;
            if !output.status().success() {
                return Err(ArtifactError::DownloadFailed);
            }
            let response =
                std::str::from_utf8(output.stdout()).map_err(|_| ArtifactError::DownloadFailed)?;
            let mut lines = response.lines().rev();
            let redirect = lines.next().unwrap_or_default().trim();
            let status = lines
                .next()
                .and_then(|value| value.trim().parse::<u16>().ok())
                .ok_or(ArtifactError::DownloadFailed)?;
            match redirect_target(status, redirect)? {
                Some(next) => {
                    current = next;
                    continue;
                }
                None => return Ok(()),
            }
        }
        Err(ArtifactError::InvalidLocator)
    }
}

fn validate_release_url(url: &str) -> Result<(), ArtifactError> {
    let allowed = [
        "https://github.com/",
        "https://api.github.com/",
        "https://objects.githubusercontent.com/",
    ];
    if url.starts_with('-') || !allowed.iter().any(|prefix| url.starts_with(prefix)) {
        return Err(ArtifactError::InvalidLocator);
    }
    Ok(())
}

fn redirect_target(status: u16, redirect: &str) -> Result<Option<String>, ArtifactError> {
    if (300..400).contains(&status) {
        if redirect.is_empty() {
            return Err(ArtifactError::InvalidLocator);
        }
        validate_release_url(redirect)?;
        return Ok(Some(redirect.to_owned()));
    }
    if (200..300).contains(&status) {
        return Ok(None);
    }
    Err(ArtifactError::DownloadFailed)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemBinaryInstaller;

impl BinaryInstaller for SystemBinaryInstaller {
    fn inspect(&self, path: &AbsolutePath) -> Result<Option<InstalledBinary>, ArtifactError> {
        let metadata = match fs::symlink_metadata(path.as_str()) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ArtifactError::InstallFailed),
        };
        if !metadata.file_type().is_file() {
            return Err(ArtifactError::InvalidArtifact);
        }
        let bytes = read_bounded(path, 64 * 1024 * 1024)?;
        let digest = format!("{:x}", Sha256::digest(bytes));
        Ok(Some(InstalledBinary {
            path: path.clone(),
            fingerprint: Fingerprint::new(FingerprintAlgorithm::Sha256, digest)
                .expect("sha256 digest is valid"),
        }))
    }

    fn install_verified(
        &self,
        artifact: &AbsolutePath,
        destination: &AbsolutePath,
        expected: &ReleaseArtifact,
    ) -> Result<(), ArtifactError> {
        let bytes =
            read_bounded(artifact, 64 * 1024 * 1024).map_err(|_| ArtifactError::InstallFailed)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != expected.sha256() {
            return Err(ArtifactError::ChecksumMismatch);
        }
        let metadata =
            fs::metadata(artifact.as_str()).map_err(|_| ArtifactError::InvalidArtifact)?;
        if !metadata.is_file() {
            return Err(ArtifactError::InvalidArtifact);
        }
        #[cfg(unix)]
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(ArtifactError::InvalidArtifact);
        }
        let destination_path = Path::new(destination.as_str());
        let parent = destination_path
            .parent()
            .ok_or(ArtifactError::InstallFailed)?;
        fs::create_dir_all(parent).map_err(|_| ArtifactError::InstallFailed)?;
        let prior_identity = destination_identity(destination_path)?;
        let prior_bytes = if prior_identity.is_some() {
            Some(
                read_bounded(destination, 64 * 1024 * 1024)
                    .map_err(|_| ArtifactError::InstallFailed)?,
            )
        } else {
            None
        };
        #[cfg(unix)]
        let prior_mode = fs::metadata(destination_path)
            .ok()
            .map(|metadata| metadata.permissions().mode());
        let sequence = ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.skilltap-artifact-{sequence}",
            destination_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("binary")
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| ArtifactError::InstallFailed)?;
            file.write_all(&bytes)
                .map_err(|_| ArtifactError::InstallFailed)?;
            file.sync_all().map_err(|_| ArtifactError::InstallFailed)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))
                    .map_err(|_| ArtifactError::InstallFailed)?;
            }
            if destination_identity(destination_path)? != prior_identity {
                return Err(ArtifactError::DestinationChanged);
            }
            publish_destination(&temporary, destination_path, prior_identity)?;
            let published_identity = destination_identity(destination_path)?;
            if sync_parent(parent).is_err() {
                restore_destination(
                    destination_path,
                    published_identity,
                    prior_bytes.as_deref(),
                    {
                        #[cfg(unix)]
                        {
                            prior_mode
                        }
                        #[cfg(not(unix))]
                        {
                            None
                        }
                    },
                );
                return Err(ArtifactError::InstallFailed);
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn restore_destination(
    path: &Path,
    expected: Option<(u64, u64)>,
    prior: Option<&[u8]>,
    #[cfg(unix)] mode: Option<u32>,
    #[cfg(not(unix))] _mode: Option<u32>,
) {
    match prior {
        Some(bytes) => {
            let Some(parent) = path.parent() else { return };
            for attempt in 0..64u32 {
                let temporary = parent.join(format!(
                    ".skilltap-restore-{}-{attempt}",
                    std::process::id()
                ));
                let Ok(mut file) = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)
                else {
                    continue;
                };
                if file.write_all(bytes).is_ok() {
                    #[cfg(unix)]
                    let _ = fs::set_permissions(
                        &temporary,
                        fs::Permissions::from_mode(mode.unwrap_or(0o700)),
                    );
                    let _ = file.sync_all();
                    // Exchange the restore payload with the published file and
                    // inspect the path that was moved out.  If another actor
                    // replaced the destination after publication, exchanging
                    // back preserves that replacement instead of clobbering it.
                    match exchange_paths(&temporary, path) {
                        Ok(()) if destination_identity(&temporary).ok() == Some(expected) => {
                            let _ = fs::remove_file(&temporary);
                        }
                        Ok(()) => {
                            let _ = exchange_paths(&temporary, path);
                            let _ = fs::remove_file(&temporary);
                        }
                        Err(_) => {
                            let _ = fs::remove_file(&temporary);
                        }
                    }
                } else {
                    let _ = fs::remove_file(&temporary);
                }
                return;
            }
        }
        None => {
            if let Some(expected) = expected {
                remove_published_if_identity(path, expected);
            }
        }
    }
}

/// Remove a published file without unlinking a replacement that appeared
/// after publication.  Swapping a private marker into the destination first
/// gives us an atomic identity check on the displaced inode.  If the path is
/// already occupied by an unrelated replacement, the published inode is
/// cleaned up through the private marker and the replacement is left intact.
fn remove_published_if_identity(path: &Path, expected: (u64, u64)) {
    remove_published_if_identity_with(path, expected, || {});
}

fn remove_published_if_identity_with(
    path: &Path,
    expected: (u64, u64),
    after_exchange: impl FnOnce(),
) {
    after_exchange();
    let Some(parent) = path.parent() else { return };
    let marker = parent.join(format!(
        ".skilltap-rollback-cleanup-{}",
        ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    // Atomically move the observed destination into a private name without
    // replacing anything. A replacement that wins before this operation is
    // moved to the marker, inspected, and restored only with another
    // no-replace rename; the destination is never unlinked by a stale name.
    if rename_noreplace(path, &marker).is_err() {
        return;
    }
    if destination_identity(&marker).ok().flatten() == Some(expected) {
        let _ = fs::remove_file(&marker);
    } else if rename_noreplace(&marker, path).is_err() {
        // Preserve an unrelated replacement at `path`; leave the private
        // residual for an explicit repair rather than deleting either inode.
    }
}

/// Publish a verified payload without overwriting a destination that changed
/// after it was observed.  Linux provides the required compare-and-swap-like
/// directory operations through `renameat2`: `RENAME_NOREPLACE` handles first
/// install and `RENAME_EXCHANGE` lets updates inspect the inode displaced by
/// the exchange before deciding whether to keep it.
fn publish_destination(
    temporary: &Path,
    destination: &Path,
    prior: Option<(u64, u64)>,
) -> Result<(), ArtifactError> {
    match prior {
        Some(expected) => {
            exchange_paths(temporary, destination).map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    ArtifactError::DestinationChanged
                } else {
                    ArtifactError::InstallFailed
                }
            })?;
            if destination_identity(temporary)? != Some(expected) {
                // The destination was replaced between observation and the
                // exchange.  Put the unrelated replacement back at its path.
                exchange_paths(temporary, destination)
                    .map_err(|_| ArtifactError::DestinationChanged)?;
                return Err(ArtifactError::DestinationChanged);
            }
            Ok(())
        }
        None => match rename_noreplace(temporary, destination) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(ArtifactError::DestinationChanged)
            }
            Err(_) => Err(ArtifactError::InstallFailed),
        },
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(source: &Path, destination: &Path) -> std::io::Result<()> {
    rename_with_flags(source, destination, libc::RENAME_NOREPLACE)
}

#[cfg(target_os = "macos")]
fn rename_noreplace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename_noreplace(_source: &Path, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-replace publication is unavailable on this platform",
    ))
}

#[cfg(target_os = "linux")]
fn exchange_paths(source: &Path, destination: &Path) -> std::io::Result<()> {
    rename_with_flags(source, destination, libc::RENAME_EXCHANGE)
}

#[cfg(target_os = "macos")]
fn exchange_paths(source: &Path, destination: &Path) -> std::io::Result<()> {
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: both C strings are NUL-free owned path bytes and AT_FDCWD
    // scopes the operation to the process's current directory.
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn exchange_paths(_source: &Path, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic exchange publication is unavailable on this platform",
    ))
}

#[cfg(target_os = "linux")]
fn rename_with_flags(
    source: &Path,
    destination: &Path,
    flags: libc::c_uint,
) -> std::io::Result<()> {
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: both C strings are NUL-free owned path bytes and AT_FDCWD
    // scopes the operation to the process's current directory.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            flags,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn destination_identity(path: &Path) -> Result<Option<(u64, u64)>, ArtifactError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ArtifactError::InstallFailed),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(ArtifactError::DestinationChanged);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Some((metadata.dev(), metadata.ino())))
    }
    #[cfg(not(unix))]
    {
        Ok(Some((0, metadata.len())))
    }
}

fn sync_parent(parent: &Path) -> std::io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::ArtifactArch;
    use crate::runtime::SupportedPlatform;

    fn key() -> ArtifactKey {
        ArtifactKey {
            platform: SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        }
    }
    #[test]
    fn release_manifest_requires_one_matching_supported_asset() {
        let value = serde_json::json!({"version":"3.0.0","assets":[{"platform":"linux","arch":"x86_64","name":"skilltap","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","download_url":"https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap"}]});
        assert!(ReleaseManifest::parse(&value, key()).is_ok());
        let duplicate = serde_json::json!({"version":"3.0.0","assets":[{"platform":"linux","arch":"x86_64","name":"skilltap","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","download_url":"https://github.com/a"},{"platform":"linux","arch":"x86_64","name":"skilltap2","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","download_url":"https://github.com/b"}]});
        assert_eq!(
            ReleaseManifest::parse(&duplicate, key()),
            Err(ArtifactError::UnsupportedAsset)
        );
    }

    #[test]
    fn unsafe_release_hosts_are_rejected_before_process_execution() {
        assert_eq!(
            validate_release_url("https://evil.example/file"),
            Err(ArtifactError::InvalidLocator)
        );
        assert_eq!(
            validate_release_url("--url"),
            Err(ArtifactError::InvalidLocator)
        );
    }

    #[test]
    fn every_redirect_hop_is_attested_before_the_next_fetch() {
        assert_eq!(
            redirect_target(302, "https://evil.example/asset"),
            Err(ArtifactError::InvalidLocator)
        );
        assert_eq!(
            redirect_target(302, "https://objects.githubusercontent.com/asset").unwrap(),
            Some("https://objects.githubusercontent.com/asset".to_owned())
        );
        assert_eq!(redirect_target(200, "").unwrap(), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn publication_exchange_preserves_a_replacement_observed_after_initial_identity() {
        let root = skilltap_test_support::TempRoot::new("bootstrap-publication-race").unwrap();
        let destination = root.path().join("skilltap");
        let temporary = root.path().join("payload");
        let replacement = root.path().join("replacement");
        fs::write(&destination, b"prior").unwrap();
        let prior = destination_identity(&destination).unwrap();
        fs::write(&temporary, b"verified").unwrap();
        fs::write(&replacement, b"unrelated replacement").unwrap();
        fs::remove_file(&destination).unwrap();
        fs::rename(&replacement, &destination).unwrap();

        assert_eq!(
            publish_destination(&temporary, &destination, prior),
            Err(ArtifactError::DestinationChanged)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated replacement");
        assert!(temporary.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn first_install_uses_no_clobber_when_destination_appears_after_observation() {
        let root = skilltap_test_support::TempRoot::new("bootstrap-first-install-race").unwrap();
        let destination = root.path().join("skilltap");
        let temporary = root.path().join("payload");
        fs::write(&temporary, b"verified").unwrap();
        fs::write(&destination, b"unrelated replacement").unwrap();

        assert_eq!(
            publish_destination(&temporary, &destination, None),
            Err(ArtifactError::DestinationChanged)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated replacement");
        assert!(temporary.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rollback_exchange_preserves_a_replacement_before_rollback() {
        let root = skilltap_test_support::TempRoot::new("bootstrap-rollback-race").unwrap();
        let destination = root.path().join("skilltap");
        let replacement = root.path().join("replacement");
        fs::write(&destination, b"published").unwrap();
        let expected = destination_identity(&destination).unwrap().unwrap();
        fs::write(&replacement, b"unrelated replacement").unwrap();
        fs::remove_file(&destination).unwrap();
        fs::rename(&replacement, &destination).unwrap();

        restore_destination(&destination, Some(expected), Some(b"prior"), None);
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated replacement");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn no_prior_rollback_preserves_a_replacement_without_unlinking_by_stale_path() {
        let root =
            skilltap_test_support::TempRoot::new("bootstrap-rollback-no-prior-race").unwrap();
        let destination = root.path().join("skilltap");
        let replacement = root.path().join("replacement");
        fs::write(&destination, b"published").unwrap();
        let expected = destination_identity(&destination).unwrap().unwrap();
        fs::write(&replacement, b"unrelated replacement").unwrap();
        fs::remove_file(&destination).unwrap();
        fs::rename(&replacement, &destination).unwrap();

        remove_published_if_identity(&destination, expected);
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated replacement");
        assert_eq!(
            fs::read_dir(root.path())
                .unwrap()
                .filter_map(Result::ok)
                .count(),
            1
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn no_prior_rollback_removes_the_expected_destination_when_unchanged() {
        let root =
            skilltap_test_support::TempRoot::new("bootstrap-rollback-no-prior-cleanup").unwrap();
        let destination = root.path().join("skilltap");
        fs::write(&destination, b"published").unwrap();
        let expected = destination_identity(&destination).unwrap().unwrap();

        remove_published_if_identity(&destination, expected);
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn no_prior_rollback_preserves_replacement_arriving_after_exchange() {
        let root =
            skilltap_test_support::TempRoot::new("bootstrap-rollback-post-exchange").unwrap();
        let destination = root.path().join("skilltap");
        let replacement = root.path().join("replacement");
        fs::write(&destination, b"published").unwrap();
        let expected = destination_identity(&destination).unwrap().unwrap();
        fs::write(&replacement, b"unrelated replacement").unwrap();

        remove_published_if_identity_with(&destination, expected, || {
            fs::remove_file(&destination).unwrap();
            fs::rename(&replacement, &destination).unwrap();
        });
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated replacement");
    }
}
