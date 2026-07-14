//! Deterministic user-service definitions for one bounded daemon cycle.

use std::collections::BTreeMap;

use crate::{
    domain::{
        AbsolutePath, DesiredResource, HarnessId, HarnessSet, NativeId, ResourceId, ResourceKey,
        ResourceKind, Scope, UpdateIntent,
    },
    foreground_update::ForegroundUpdatePlan,
    marketplace::PluginSelector,
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

/// The exact identity of a daemon marketplace prerequisite. Scope belongs to
/// the resource key and the harness belongs to this key, so equal native names
/// can never authorize one another across either boundary.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DaemonMarketplaceRefreshKey {
    resource: ResourceKey,
    target: HarnessId,
}

impl DaemonMarketplaceRefreshKey {
    pub fn new(resource: ResourceKey, target: HarnessId) -> Self {
        Self { resource, target }
    }

    pub const fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub const fn target(&self) -> &HarnessId {
        &self.target
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonMarketplaceRefreshTask {
    key: DaemonMarketplaceRefreshKey,
    name: NativeId,
}

impl DaemonMarketplaceRefreshTask {
    pub const fn key(&self) -> &DaemonMarketplaceRefreshKey {
        &self.key
    }

    pub const fn name(&self) -> &NativeId {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonPluginUpdateTask {
    resource: ResourceKey,
    target: HarnessId,
    selector: PluginSelector,
    refresh: DaemonMarketplaceRefreshKey,
}

impl DaemonPluginUpdateTask {
    pub const fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub const fn target(&self) -> &HarnessId {
        &self.target
    }

    pub const fn selector(&self) -> &PluginSelector {
        &self.selector
    }

    pub const fn refresh(&self) -> &DaemonMarketplaceRefreshKey {
        &self.refresh
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DaemonPluginBlockReason {
    InvalidSelector,
    MarketplaceMissing,
    MarketplaceTargetMissing,
    MarketplaceUpdateDisabled,
    MarketplacePinned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonBlockedPluginUpdate {
    resource: ResourceKey,
    target: HarnessId,
    reason: DaemonPluginBlockReason,
}

impl DaemonBlockedPluginUpdate {
    pub const fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub const fn target(&self) -> &HarnessId {
        &self.target
    }

    pub const fn reason(&self) -> DaemonPluginBlockReason {
        self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonNativeUpdatePlan {
    refreshes: Vec<DaemonMarketplaceRefreshTask>,
    plugins: Vec<DaemonPluginUpdateTask>,
    blocked_plugins: Vec<DaemonBlockedPluginUpdate>,
}

impl DaemonNativeUpdatePlan {
    pub fn refreshes(&self) -> &[DaemonMarketplaceRefreshTask] {
        &self.refreshes
    }

    pub fn plugins(&self) -> &[DaemonPluginUpdateTask] {
        &self.plugins
    }

    pub fn blocked_plugins(&self) -> &[DaemonBlockedPluginUpdate] {
        &self.blocked_plugins
    }
}

/// Build the native part of one daemon update cycle from desired inventory.
///
/// Marketplace refreshes are deduplicated by their full resource key and
/// target. Plugin selectors are resolved only against an exact marketplace
/// resource in the same scope; no missing registration or target is invented.
pub fn plan_daemon_native_updates<'a>(
    resources: impl IntoIterator<Item = &'a DesiredResource>,
) -> DaemonNativeUpdatePlan {
    let resources = resources.into_iter().collect::<Vec<_>>();
    let marketplaces = resources
        .iter()
        .filter(|resource| resource.kind() == ResourceKind::Marketplace)
        .map(|resource| (resource.key().clone(), *resource))
        .collect::<BTreeMap<_, _>>();

    let mut refreshes =
        BTreeMap::<DaemonMarketplaceRefreshKey, DaemonMarketplaceRefreshTask>::new();
    for resource in marketplaces.values().copied() {
        if resource.update() != UpdateIntent::Track {
            continue;
        }
        let name = resource
            .id()
            .as_str()
            .strip_prefix("marketplace:")
            .and_then(|name| NativeId::new(name).ok());
        let Some(name) = name else {
            // Desired marketplace ids are validated at the inventory boundary;
            // retaining no executable task is safer than guessing a native id
            // if a future schema permits another marketplace identity shape.
            continue;
        };
        for target in resource.targets().iter() {
            let key = DaemonMarketplaceRefreshKey::new(resource.key().clone(), target.clone());
            refreshes.insert(
                key.clone(),
                DaemonMarketplaceRefreshTask {
                    key,
                    name: name.clone(),
                },
            );
        }
    }

    let mut plugins = Vec::new();
    let mut blocked_plugins = Vec::new();
    for resource in resources
        .iter()
        .filter(|resource| resource.kind() == ResourceKind::Plugin)
        .filter(|resource| resource.update() == UpdateIntent::Track)
    {
        for target in resource.targets().iter() {
            let selector = resource
                .id()
                .as_str()
                .strip_prefix("plugin:")
                .and_then(|value| PluginSelector::parse(value).ok());
            let Some(selector) = selector else {
                blocked_plugins.push(DaemonBlockedPluginUpdate {
                    resource: resource.key().clone(),
                    target: target.clone(),
                    reason: DaemonPluginBlockReason::InvalidSelector,
                });
                continue;
            };
            let marketplace_key = ResourceKey::new(
                match ResourceId::new(format!("marketplace:{}", selector.marketplace())) {
                    Ok(id) => id,
                    Err(_) => {
                        blocked_plugins.push(DaemonBlockedPluginUpdate {
                            resource: resource.key().clone(),
                            target: target.clone(),
                            reason: DaemonPluginBlockReason::InvalidSelector,
                        });
                        continue;
                    }
                },
                resource.scope().clone(),
            );
            let Some(marketplace) = marketplaces.get(&marketplace_key).copied() else {
                blocked_plugins.push(DaemonBlockedPluginUpdate {
                    resource: resource.key().clone(),
                    target: target.clone(),
                    reason: DaemonPluginBlockReason::MarketplaceMissing,
                });
                continue;
            };
            if !marketplace.targets().contains(target) {
                blocked_plugins.push(DaemonBlockedPluginUpdate {
                    resource: resource.key().clone(),
                    target: target.clone(),
                    reason: DaemonPluginBlockReason::MarketplaceTargetMissing,
                });
                continue;
            }
            let reason = match marketplace.update() {
                UpdateIntent::Disabled => Some(DaemonPluginBlockReason::MarketplaceUpdateDisabled),
                UpdateIntent::Pinned => Some(DaemonPluginBlockReason::MarketplacePinned),
                UpdateIntent::Track => None,
            };
            if let Some(reason) = reason {
                blocked_plugins.push(DaemonBlockedPluginUpdate {
                    resource: resource.key().clone(),
                    target: target.clone(),
                    reason,
                });
                continue;
            }
            let refresh =
                DaemonMarketplaceRefreshKey::new(marketplace.key().clone(), target.clone());
            if !refreshes.contains_key(&refresh) {
                blocked_plugins.push(DaemonBlockedPluginUpdate {
                    resource: resource.key().clone(),
                    target: target.clone(),
                    reason: DaemonPluginBlockReason::MarketplaceMissing,
                });
                continue;
            }
            plugins.push(DaemonPluginUpdateTask {
                resource: resource.key().clone(),
                target: target.clone(),
                selector,
                refresh,
            });
        }
    }

    plugins.sort_by(|left, right| {
        (&left.resource, &left.target).cmp(&(&right.resource, &right.target))
    });
    blocked_plugins.sort_by(|left, right| {
        (&left.resource, &left.target, left.reason).cmp(&(
            &right.resource,
            &right.target,
            right.reason,
        ))
    });
    DaemonNativeUpdatePlan {
        refreshes: refreshes.into_values().collect(),
        plugins,
        blocked_plugins,
    }
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

    fn desired(
        kind: ResourceKind,
        id: &str,
        scope: Scope,
        targets: &[&str],
        update: UpdateIntent,
    ) -> DesiredResource {
        DesiredResource::new(
            ResourceKey::new(ResourceId::new(id).unwrap(), scope),
            kind,
            HarnessSet::new(
                targets
                    .iter()
                    .map(|target| HarnessId::new(*target).unwrap()),
            )
            .unwrap(),
            crate::domain::DesiredOrigin::Direct,
            None,
            update,
            crate::domain::ComponentGraph::new([]).unwrap(),
            BTreeMap::new(),
            BTreeMap::new(),
            std::collections::BTreeSet::new(),
        )
        .unwrap()
    }

    #[test]
    fn daemon_native_plan_deduplicates_exact_marketplace_prerequisites() {
        let marketplace = desired(
            ResourceKind::Marketplace,
            "marketplace:team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let first = desired(
            ResourceKind::Plugin,
            "plugin:formatter@team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let second = desired(
            ResourceKind::Plugin,
            "plugin:review@team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let plan = plan_daemon_native_updates([&second, &marketplace, &first]);
        assert_eq!(plan.refreshes().len(), 1);
        assert_eq!(plan.plugins().len(), 2);
        assert!(plan.plugins().iter().all(|task| {
            task.refresh().resource() == marketplace.key()
                && task.refresh().target().as_str() == "codex"
        }));
        assert_eq!(plan.refreshes()[0].name().as_str(), "team");
    }

    #[test]
    fn daemon_native_plan_keeps_scope_and_target_in_refresh_identity() {
        let global = desired(
            ResourceKind::Marketplace,
            "marketplace:team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let project = desired(
            ResourceKind::Marketplace,
            "marketplace:team",
            Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            &["claude"],
            UpdateIntent::Track,
        );
        let plan = plan_daemon_native_updates([&project, &global]);
        assert_eq!(plan.refreshes().len(), 2);
        assert_ne!(plan.refreshes()[0].key(), plan.refreshes()[1].key());
        assert_eq!(plan.refreshes()[0].name(), plan.refreshes()[1].name());
    }

    #[test]
    fn daemon_native_plan_blocks_only_unresolvable_plugin_relationships() {
        let tracked = desired(
            ResourceKind::Marketplace,
            "marketplace:tracked",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let pinned = desired(
            ResourceKind::Marketplace,
            "marketplace:pinned",
            Scope::Global,
            &["codex"],
            UpdateIntent::Pinned,
        );
        let disabled = desired(
            ResourceKind::Marketplace,
            "marketplace:disabled",
            Scope::Global,
            &["codex"],
            UpdateIntent::Disabled,
        );
        let target_mismatch = desired(
            ResourceKind::Marketplace,
            "marketplace:other-target",
            Scope::Global,
            &["claude"],
            UpdateIntent::Track,
        );
        let valid = desired(
            ResourceKind::Plugin,
            "plugin:valid@tracked",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let missing = desired(
            ResourceKind::Plugin,
            "plugin:missing@unknown",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let mismatch = desired(
            ResourceKind::Plugin,
            "plugin:mismatch@other-target",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let pinned_plugin = desired(
            ResourceKind::Plugin,
            "plugin:pinned@pinned",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let disabled_plugin = desired(
            ResourceKind::Plugin,
            "plugin:disabled@disabled",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let malformed = desired(
            ResourceKind::Plugin,
            "plugin:malformed",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let plan = plan_daemon_native_updates([
            &malformed,
            &disabled_plugin,
            &pinned_plugin,
            &mismatch,
            &missing,
            &valid,
            &target_mismatch,
            &disabled,
            &pinned,
            &tracked,
        ]);
        assert_eq!(plan.plugins().len(), 1);
        assert_eq!(plan.plugins()[0].resource(), valid.key());
        let reasons = plan
            .blocked_plugins()
            .iter()
            .map(|blocked| blocked.reason())
            .collect::<Vec<_>>();
        assert!(reasons.contains(&DaemonPluginBlockReason::InvalidSelector));
        assert!(reasons.contains(&DaemonPluginBlockReason::MarketplaceMissing));
        assert!(reasons.contains(&DaemonPluginBlockReason::MarketplaceTargetMissing));
        assert!(reasons.contains(&DaemonPluginBlockReason::MarketplacePinned));
        assert!(reasons.contains(&DaemonPluginBlockReason::MarketplaceUpdateDisabled));
    }

    #[test]
    fn daemon_native_plan_is_independent_of_inventory_order() {
        let marketplace = desired(
            ResourceKind::Marketplace,
            "marketplace:team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let plugin = desired(
            ResourceKind::Plugin,
            "plugin:formatter@team",
            Scope::Global,
            &["codex"],
            UpdateIntent::Track,
        );
        let first = plan_daemon_native_updates([&marketplace, &plugin]);
        let second = plan_daemon_native_updates([&plugin, &marketplace]);
        assert_eq!(first, second);
    }
}
