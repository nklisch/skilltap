//! Deterministic user-service definitions for one bounded daemon cycle.

use crate::{
    domain::AbsolutePath,
    storage::{UpdateInterval, UpdateIntervalUnit},
};

pub const SERVICE_LABEL: &str = "com.skilltap.daemon";
pub const SYSTEMD_UNIT: &str = "skilltap-update.service";
pub const SYSTEMD_TIMER: &str = "skilltap-update.timer";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServicePlatform {
    Launchd,
    SystemdUser,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonServiceSpec {
    pub platform: ServicePlatform,
    pub interval: UpdateInterval,
    pub executable: AbsolutePath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceFile {
    name: String,
    contents: String,
}

impl ServiceFile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceDefinition {
    platform: ServicePlatform,
    files: Vec<ServiceFile>,
}

impl ServiceDefinition {
    pub const fn platform(&self) -> ServicePlatform {
        self.platform
    }

    pub fn files(&self) -> &[ServiceFile] {
        &self.files
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceRenderError {
    InvalidExecutable,
}

impl std::fmt::Display for ServiceRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("daemon executable must be a normalized absolute path")
    }
}

impl std::error::Error for ServiceRenderError {}

pub fn render_service(spec: &DaemonServiceSpec) -> Result<ServiceDefinition, ServiceRenderError> {
    if spec.executable.as_str().is_empty() || !spec.executable.as_str().starts_with('/') {
        return Err(ServiceRenderError::InvalidExecutable);
    }
    let seconds = interval_seconds(spec.interval);
    let files = match spec.platform {
        ServicePlatform::Launchd => vec![ServiceFile {
            name: format!("{SERVICE_LABEL}.plist"),
            contents: launchd_contents(&spec.executable, seconds),
        }],
        ServicePlatform::SystemdUser => vec![
            ServiceFile {
                name: SYSTEMD_UNIT.to_owned(),
                contents: systemd_service_contents(&spec.executable),
            },
            ServiceFile {
                name: SYSTEMD_TIMER.to_owned(),
                contents: systemd_timer_contents(seconds),
            },
        ],
    };
    Ok(ServiceDefinition {
        platform: spec.platform,
        files,
    })
}

fn interval_seconds(interval: UpdateInterval) -> u64 {
    let multiplier = match interval.unit() {
        UpdateIntervalUnit::Seconds => 1,
        UpdateIntervalUnit::Minutes => 60,
        UpdateIntervalUnit::Hours => 60 * 60,
        UpdateIntervalUnit::Days => 24 * 60 * 60,
    };
    interval.value().saturating_mul(multiplier)
}

fn launchd_contents(executable: &AbsolutePath, seconds: u64) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>{SERVICE_LABEL}</string>\n  <key>ProgramArguments</key>\n  <array><string>{}</string><string>daemon</string><string>run</string></array>\n  <key>StartInterval</key><integer>{seconds}</integer>\n  <key>RunAtLoad</key><false/>\n</dict>\n</plist>\n",
        xml_escape(executable.as_str())
    )
}

fn systemd_service_contents(executable: &AbsolutePath) -> String {
    format!(
        "[Unit]\nDescription=skilltap safe update cycle\n\n[Service]\nType=oneshot\nExecStart={} daemon run\n",
        systemd_escape(executable.as_str())
    )
}

fn systemd_timer_contents(seconds: u64) -> String {
    format!(
        "[Unit]\nDescription=skilltap safe update timer\n\n[Timer]\nOnBootSec={seconds}s\nOnUnitActiveSec={seconds}s\nPersistent=true\nUnit={SYSTEMD_UNIT}\n\n[Install]\nWantedBy=timers.target\n"
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn systemd_escape(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "._/-".contains(character))
    {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::UpdateIntervalUnit;

    fn spec(platform: ServicePlatform) -> DaemonServiceSpec {
        DaemonServiceSpec {
            platform,
            interval: UpdateInterval::new(6, UpdateIntervalUnit::Hours).unwrap(),
            executable: AbsolutePath::new("/usr/local/bin/skilltap").unwrap(),
        }
    }

    #[test]
    fn launchd_definition_is_stable_and_finite() {
        let first = render_service(&spec(ServicePlatform::Launchd)).unwrap();
        let second = render_service(&spec(ServicePlatform::Launchd)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.files().len(), 1);
        let contents = first.files()[0].contents();
        assert!(contents.contains("<string>daemon</string><string>run</string>"));
        assert!(contents.contains("<integer>21600</integer>"));
        assert!(contents.contains("<false/>"));
    }

    #[test]
    fn systemd_definition_has_one_service_and_timer() {
        let definition = render_service(&spec(ServicePlatform::SystemdUser)).unwrap();
        assert_eq!(definition.files().len(), 2);
        assert!(definition.files()[0].contents().contains("Type=oneshot"));
        assert!(definition.files()[0].contents().contains("daemon run"));
        assert!(
            definition.files()[1]
                .contents()
                .contains("OnUnitActiveSec=21600s")
        );
    }
}
