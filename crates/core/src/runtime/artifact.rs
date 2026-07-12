//! Bounded release transport and atomic user-level binary publication.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};

use crate::{
    bootstrap::{ArtifactKey, ReleaseArtifact, ReleaseVersion},
    domain::{AbsolutePath, Fingerprint, FingerprintAlgorithm, SourceLocator},
};

use super::{CommandRequest, CommandRunner, SystemCommandRunner};

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
        let path = std::env::temp_dir().join(format!(
            ".skilltap-release-manifest-{}-{}",
            std::process::id(),
            ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let path = AbsolutePath::new(path.to_string_lossy().into_owned())
            .map_err(|_| ArtifactError::DownloadFailed)?;
        let result = self.fetcher.fetch(self.url.as_str(), &path).and_then(|()| {
            let bytes = fs::read(path.as_str()).map_err(|_| ArtifactError::DownloadFailed)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
                ArtifactError::InvalidManifest("release manifest is not valid JSON")
            })?;
            if value.get("tag_name").is_some() {
                self.parse_github_release(&value)
            } else {
                ReleaseManifest::parse(&value, self.key)
            }
        });
        let _ = fs::remove_file(path.as_str());
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
        let asset = assets
            .iter()
            .find(|asset| {
                asset.get("name").and_then(serde_json::Value::as_str) == Some(expected_name)
            })
            .ok_or(ArtifactError::UnsupportedAsset)?;
        let download_url = asset
            .get("browser_download_url")
            .and_then(serde_json::Value::as_str)
            .ok_or(ArtifactError::InvalidManifest(
                "GitHub release asset URL is missing",
            ))?;
        let checksums_url = format!(
            "https://github.com/nklisch/skilltap/releases/download/v{version}/checksums.txt"
        );
        let checksum_path = AbsolutePath::new(
            std::env::temp_dir()
                .join(format!(
                    ".skilltap-release-checksums-{}-{}",
                    std::process::id(),
                    ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ))
                .to_string_lossy()
                .into_owned(),
        )
        .map_err(|_| ArtifactError::DownloadFailed)?;
        let checksum = self
            .fetcher
            .fetch(&checksums_url, &checksum_path)
            .and_then(|()| {
                let contents = fs::read_to_string(checksum_path.as_str())
                    .map_err(|_| ArtifactError::DownloadFailed)?;
                parse_checksum(&contents, expected_name)
            });
        let _ = fs::remove_file(checksum_path.as_str());
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
        let executable =
            crate::domain::NativeId::new("curl").map_err(|_| ArtifactError::InvalidLocator)?;
        let request = CommandRequest::new(
            executable,
            [
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-time",
                "30",
                "--output",
                destination.as_str(),
                "--",
                url,
            ]
            .into_iter()
            .map(std::ffi::OsString::from),
            None,
        );
        let output = SystemCommandRunner
            .run(&request)
            .map_err(|_| ArtifactError::DownloadFailed)?;
        if output.status().success() {
            Ok(())
        } else {
            Err(ArtifactError::DownloadFailed)
        }
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
        let bytes = fs::read(path.as_str()).map_err(|_| ArtifactError::InstallFailed)?;
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
        let bytes = fs::read(artifact.as_str()).map_err(|_| ArtifactError::InstallFailed)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != expected.sha256() {
            return Err(ArtifactError::ChecksumMismatch);
        }
        let metadata =
            fs::metadata(artifact.as_str()).map_err(|_| ArtifactError::InvalidArtifact)?;
        if !metadata.is_file() {
            return Err(ArtifactError::InvalidArtifact);
        }
        let destination_path = Path::new(destination.as_str());
        let parent = destination_path
            .parent()
            .ok_or(ArtifactError::InstallFailed)?;
        fs::create_dir_all(parent).map_err(|_| ArtifactError::InstallFailed)?;
        let prior_identity = destination_identity(destination_path)?;
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
            fs::rename(&temporary, destination_path).map_err(|_| ArtifactError::InstallFailed)?;
            sync_parent(parent).map_err(|_| ArtifactError::InstallFailed)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
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
        return Ok(Some((metadata.dev(), metadata.ino())));
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
}
