//! Deterministic user-service definitions for one bounded daemon cycle.

use crate::{
    domain::AbsolutePath,
    foreground_update::ForegroundUpdatePlan,
    storage::{UpdateInterval, UpdateIntervalUnit},
};

pub const SERVICE_LABEL: &str = "com.skilltap.daemon";
pub const SYSTEMD_UNIT: &str = "skilltap-update.service";
pub const SYSTEMD_TIMER: &str = "skilltap-update.timer";
pub const SERVICE_MARKER: &str = "skilltap-managed-v3";

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonCyclePlan {
    safe: Vec<crate::domain::ResourceKey>,
    pending: Vec<crate::domain::ResourceKey>,
}

impl DaemonCyclePlan {
    pub fn safe(&self) -> &[crate::domain::ResourceKey] {
        &self.safe
    }

    pub fn pending(&self) -> &[crate::domain::ResourceKey] {
        &self.pending
    }
}

/// Reduce a foreground update plan to the only work a daemon may apply. The
/// daemon has no acknowledgment set, so every non-safe decision remains
/// pending and visible to status.
pub fn plan_daemon_cycle(plan: &ForegroundUpdatePlan) -> DaemonCyclePlan {
    let mut safe = Vec::new();
    let mut pending = Vec::new();
    for entry in plan.entries() {
        if entry.is_safe() {
            safe.push(entry.resource().clone());
        } else if entry.available_revision().is_some() {
            pending.push(entry.resource().clone());
        }
    }
    DaemonCyclePlan { safe, pending }
}

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
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>{SERVICE_LABEL}</string>\n  <key>SkilltapManaged</key><string>{SERVICE_MARKER}</string>\n  <key>ProgramArguments</key>\n  <array><string>{}</string><string>daemon</string><string>run</string></array>\n  <key>StartInterval</key><integer>{seconds}</integer>\n  <key>RunAtLoad</key><false/>\n</dict>\n</plist>\n",
        xml_escape(executable.as_str())
    )
}

fn systemd_service_contents(executable: &AbsolutePath) -> String {
    format!(
        "# {SERVICE_MARKER}\n[Unit]\nDescription=skilltap safe update cycle\n\n[Service]\nType=oneshot\nExecStart={} daemon run\n",
        systemd_escape(executable.as_str())
    )
}

fn systemd_timer_contents(seconds: u64) -> String {
    format!(
        "# {SERVICE_MARKER}\n[Unit]\nDescription=skilltap safe update timer\n\n[Timer]\nOnBootSec={seconds}s\nOnUnitActiveSec={seconds}s\nPersistent=true\nUnit={SYSTEMD_UNIT}\n\n[Install]\nWantedBy=timers.target\n"
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
        format!(
            "\"{}\"",
            value
                .replace('%', "%%")
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        )
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

    #[test]
    fn systemd_paths_escape_specifier_percent() {
        let definition = render_service(&DaemonServiceSpec {
            platform: ServicePlatform::SystemdUser,
            interval: UpdateInterval::new(1, UpdateIntervalUnit::Hours).unwrap(),
            executable: AbsolutePath::new("/tmp/skill%tap/bin/skilltap").unwrap(),
        })
        .unwrap();
        assert!(
            definition.files()[0]
                .contents()
                .contains("ExecStart=\"/tmp/skill%%tap/bin/skilltap\" daemon run")
        );
    }

    #[test]
    fn daemon_cycle_never_selects_non_safe_entries() {
        let resource = crate::domain::ResourceKey::new(
            crate::domain::ResourceId::new("skill:demo").unwrap(),
            crate::domain::Scope::Global,
        );
        let candidate = crate::updates::UpdateCandidate {
            resource: resource.clone(),
            current_revision: None,
            available_revision: None,
            resolution_error: Some(crate::updates::ResolutionError::UnreachableSource),
            pinned: false,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
            intent: crate::domain::UpdateIntent::Track,
            acknowledgment_selectors: std::collections::BTreeSet::new(),
        };
        let desired = crate::domain::DesiredResource::new(
            resource,
            crate::domain::ResourceKind::StandaloneSkill,
            crate::domain::HarnessSet::new([crate::domain::HarnessId::new("codex").unwrap()])
                .unwrap(),
            crate::domain::DesiredOrigin::Direct,
            None,
            crate::domain::UpdateIntent::Track,
            crate::domain::ComponentGraph::new([]).unwrap(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeSet::new(),
        )
        .unwrap();
        let plan = crate::foreground_update::plan_foreground_updates(
            crate::foreground_update::ForegroundUpdateRequest {
                resources: &[desired],
                candidates: &[candidate],
                mode: crate::storage::UpdateMode::ApplySafe,
            },
        )
        .unwrap();
        let cycle = plan_daemon_cycle(&plan);
        assert!(cycle.safe().is_empty());
        assert!(cycle.pending().is_empty());
    }
}
