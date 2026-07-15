use std::{collections::BTreeSet, ffi::OsString};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileId, CapabilityProfileSelection,
        CapabilityScope, CapabilitySet, CapabilitySupport, HarnessId, NativeId, NativeVersion,
        Scope, ScopedCapabilitySets,
    },
    materialization::{MaterializationSupport, plan_materialization},
    plugin_graph::normalize,
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        ObservationRuntimeError, PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    lifecycle::{
        NativeLifecycleAction, NativeLifecycleError, NativeLifecycleRequest,
        NativeObservationFailure, NativeResourceObservation,
    },
    managed_projection::ManagedProjectionPort,
    native_distribution::{
        NativeDistributionAssessment, NativeDistributionContext, NativeDistributionError,
        NativeDistributionPort,
    },
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, NativeLifecycleVector,
        ObservationPathError, SkillProjectionPort, TargetIdentity,
    },
};

use super::factory_managed::{FactoryManagedProjection, read_source_plugin};

const VERIFIED_VERSION: &str = "0.171.0";
const PROFILE_ID: &str = "factory-droid-0-171-0";
const FACTORY_HOME: &str = ".factory";

pub struct FactoryAdapter;
pub struct FactoryLifecycle;
pub struct FactorySkillProjection;
pub struct FactoryNativeDistribution;

static ADAPTER: FactoryAdapter = FactoryAdapter;
static LIFECYCLE: FactoryLifecycle = FactoryLifecycle;
static SKILLS: FactorySkillProjection = FactorySkillProjection;
static DISTRIBUTION: FactoryNativeDistribution = FactoryNativeDistribution;

impl FactoryAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for FactoryAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("droid").expect("static harness id is valid"),
            display_name: "Factory Droid",
            default_binary: "droid",
            distribution_surface: DistributionSurface::Managed,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| crate::DetectionError::InvalidVersion)?
            .strip_suffix('\n')
            .unwrap_or_else(|| std::str::from_utf8(stdout).expect("UTF-8 was checked"));
        let text = text.strip_suffix('\r').unwrap_or(text);
        if text.is_empty()
            || text
                .chars()
                .any(|character| character.is_control() || character.is_whitespace())
        {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(text).map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn decode_version_with_limits(
        &self,
        stdout: &[u8],
        _limits: JsonLimits,
    ) -> Result<NativeVersion, crate::DetectionError> {
        self.decode_version(stdout)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        let capabilities = factory_capabilities();
        if version.as_str() == VERIFIED_VERSION {
            CapabilityProfileSelection::verified(
                CapabilityProfileId::new(PROFILE_ID).expect("compiled profile id is valid"),
                capabilities,
            )
        } else {
            let unknown = ScopedCapabilitySets::new(
                unknown_set(capabilities.for_scope_kind(CapabilityScope::Global)),
                unknown_set(capabilities.for_scope_kind(CapabilityScope::Project)),
            );
            CapabilityProfileSelection::unknown_version(unknown)
        }
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let roots = match scope {
            Scope::Global => vec![("factory.home", factory_home(paths.home()))],
            Scope::Project(project) => vec![("project.factory", factory_home(project))],
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = 0usize;
        for (label, root) in roots {
            let Some(root) = root else {
                continue;
            };
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    project_entry_count =
                        project_entry_count.saturating_add(snapshot.entries().len());
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }

        let surface_labels = match scope {
            Scope::Global => factory_surface_labels(paths.home()),
            Scope::Project(project) => factory_surface_labels(project),
        };
        if canonical.is_empty() && surface_labels.is_empty() {
            return Err(ObservationPathError::Runtime(
                ObservationRuntimeError::TreeRootUnavailable,
            ));
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(project_entry_count),
            surface_labels,
        })
    }

    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        Some(&LIFECYCLE)
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }

    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort> {
        Some(&DISTRIBUTION)
    }

    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(FactoryManagedProjection::static_ref())
    }

    fn supports_managed_projection(&self, _scope: CapabilityScope) -> bool {
        true
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        factory_home(paths.home())
    }
}

impl NativeLifecycleVector for FactoryLifecycle {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        let mut args = vec![OsString::from("plugin")];
        match request.action {
            NativeLifecycleAction::MarketplaceAdd => {
                args.extend(["marketplace", "add"].into_iter().map(OsString::from));
                args.push(OsString::from(
                    request
                        .source
                        .as_ref()
                        .ok_or(NativeLifecycleError::MissingSource)?
                        .as_str(),
                ));
            }
            NativeLifecycleAction::MarketplaceRemove => args.extend(
                ["marketplace", "remove", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::MarketplaceUpdate => args.extend(
                ["marketplace", "update", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginInstall => args.extend(
                [
                    "install",
                    request.name.as_str(),
                    "--scope",
                    factory_scope(request),
                ]
                .into_iter()
                .map(OsString::from),
            ),
            NativeLifecycleAction::PluginRemove => args.extend(
                [
                    "uninstall",
                    request.name.as_str(),
                    "--scope",
                    factory_scope(request),
                ]
                .into_iter()
                .map(OsString::from),
            ),
            NativeLifecycleAction::PluginUpdate => args.extend(
                [
                    "update",
                    request.name.as_str(),
                    "--scope",
                    factory_scope(request),
                ]
                .into_iter()
                .map(OsString::from),
            ),
        }
        Ok(args)
    }

    fn observation_scope(&self, scope: &Scope) -> Option<CapabilityScope> {
        Some(CapabilityScope::from(scope))
    }

    fn observation_arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        match request.action {
            NativeLifecycleAction::MarketplaceAdd
            | NativeLifecycleAction::MarketplaceRemove
            | NativeLifecycleAction::MarketplaceUpdate => Ok(["plugin", "marketplace", "list"]
                .into_iter()
                .map(OsString::from)
                .collect()),
            NativeLifecycleAction::PluginInstall
            | NativeLifecycleAction::PluginRemove
            | NativeLifecycleAction::PluginUpdate => {
                Ok(["plugin", "list", "--scope", factory_scope(request)]
                    .into_iter()
                    .map(OsString::from)
                    .collect())
            }
        }
    }

    fn decode_observation(
        &self,
        stdout: &[u8],
        dispatch: &crate::NativeLifecycleDispatch,
        limits: JsonLimits,
    ) -> NativeResourceObservation {
        let request = dispatch.request();
        match request.action {
            NativeLifecycleAction::PluginInstall
            | NativeLifecycleAction::PluginRemove
            | NativeLifecycleAction::PluginUpdate => decode_factory_plugin_list(
                stdout,
                request.name.as_str(),
                factory_scope(request),
                limits,
            ),
            NativeLifecycleAction::MarketplaceAdd
            | NativeLifecycleAction::MarketplaceRemove
            | NativeLifecycleAction::MarketplaceUpdate => {
                NativeResourceObservation::Indeterminate(NativeObservationFailure::UnsupportedShape)
            }
        }
    }
}

impl SkillProjectionPort for FactorySkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => absolute_child(paths.home(), ".factory/skills"),
            Scope::Project(project) => absolute_child(project, ".factory/skills"),
        }
    }
}

impl NativeDistributionPort for FactoryNativeDistribution {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError> {
        // Factory updates follow the marketplace's latest commit. A requested
        // revision therefore cannot be represented by its native lifecycle.
        if context.requested_revision.is_some() {
            return Ok(None);
        }
        let plugin = match read_source_plugin(
            context.filesystem,
            context.checkout.root(),
            context.checkout.source(),
            context.json_limits,
        ) {
            Ok(plugin) => plugin,
            Err(NativeDistributionError::UnsupportedSource) => return Ok(None),
            Err(error) => return Err(error),
        };
        let graph = normalize(context.checkout.source().clone(), plugin.declarations)
            .map_err(|_| NativeDistributionError::InvalidAssessment)?;
        if graph.components().is_empty() {
            return Ok(None);
        }
        let supported = graph
            .components()
            .iter()
            .filter(|(_, component)| {
                matches!(
                    component.kind,
                    skilltap_core::domain::ComponentKind::Skill
                        | skilltap_core::domain::ComponentKind::McpServer
                        | skilltap_core::domain::ComponentKind::Hook
                        | skilltap_core::domain::ComponentKind::Agent
                        | skilltap_core::domain::ComponentKind::Command
                )
            })
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        let plan = plan_materialization(
            graph.components(),
            &MaterializationSupport {
                target: context.target.clone(),
                supported,
            },
        );
        Ok(Some(NativeDistributionAssessment { graph, plan }))
    }
}

/// Decode the exact human-only Factory `droid plugin list --scope ...` output
/// attested for Droid 0.171.0. This deliberately does not accept JSON or a
/// broader guessed grammar: the binary rejected `--json` during preflight.
pub fn decode_factory_plugin_list(
    stdout: &[u8],
    expected_name: &str,
    expected_scope: &str,
    limits: JsonLimits,
) -> NativeResourceObservation {
    if (stdout.len() as u64) > limits.bytes() {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::InvalidJson);
    }
    let Ok(text) = std::str::from_utf8(stdout) else {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    };
    let text = text.strip_suffix('\n').unwrap_or(text);
    let empty = format!("No plugins installed in {expected_scope} scope.");
    if text == empty {
        return NativeResourceObservation::Missing;
    }
    if !text.starts_with("Installed plugins:\nActive:\n") {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    }

    let mut matches = Vec::new();
    for line in text.lines().skip(2) {
        let Some(row) = line.strip_prefix("  ") else {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        };
        let mut columns = row.split("  ");
        let Some(name) = columns.next() else {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        };
        let Some(scope) = columns.next() else {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        };
        let Some(revision) = columns.next() else {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        };
        if columns.next().is_some()
            || !scope.starts_with('[')
            || !scope.ends_with(']')
            || revision.is_empty()
            || revision.chars().any(char::is_whitespace)
        {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        }
        let scope = &scope[1..scope.len() - 1];
        let Ok(revision) = NativeId::new(revision) else {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        };
        if name == expected_name && scope == expected_scope {
            matches.push(revision);
        }
    }
    match matches.len() {
        0 => NativeResourceObservation::Missing,
        1 => NativeResourceObservation::Present {
            scope: Some(if expected_scope == "user" {
                CapabilityScope::Global
            } else {
                CapabilityScope::Project
            }),
            revision: Some(skilltap_core::domain::ResolvedRevision::Native(
                matches.remove(0),
            )),
        },
        _ => NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
    }
}

fn factory_scope(request: &NativeLifecycleRequest) -> &'static str {
    if matches!(request.scope, Scope::Global) {
        "user"
    } else {
        "project"
    }
}

fn factory_home(base: &AbsolutePath) -> Option<AbsolutePath> {
    absolute_child(base, FACTORY_HOME)
}

fn absolute_child(base: &AbsolutePath, child: &str) -> Option<AbsolutePath> {
    AbsolutePath::new(format!("{}/{}", base.as_str(), child)).ok()
}

fn factory_surface_labels(base: &AbsolutePath) -> Vec<&'static str> {
    [
        ("factory.settings", ".factory/settings.json"),
        ("factory.mcp", ".factory/mcp.json"),
        (
            "factory.plugins.installed",
            ".factory/plugins/installed_plugins.json",
        ),
    ]
    .into_iter()
    .filter_map(|(label, path)| {
        std::fs::symlink_metadata(format!("{}/{}", base.as_str(), path))
            .is_ok()
            .then_some(label)
    })
    .collect()
}

fn unknown_set(set: &CapabilitySet) -> CapabilitySet {
    CapabilitySet::new(
        set.iter()
            .map(|(id, _)| (id.clone(), CapabilitySupport::Unverified)),
    )
}

fn factory_capabilities() -> ScopedCapabilitySets {
    let capability = |name: &'static str, support: CapabilitySupport| {
        (
            CapabilityId::new(name).expect("compiled capability id is valid"),
            support,
        )
    };
    let global = CapabilitySet::new([
        capability("harness.observe", CapabilitySupport::Supported),
        capability("managed.projection", CapabilitySupport::Supported),
        capability("plugin.install", CapabilitySupport::Supported),
        capability("plugin.remove", CapabilitySupport::Supported),
        capability("plugin.update", CapabilitySupport::Supported),
        capability("marketplace.register", CapabilitySupport::Supported),
        capability("marketplace.remove", CapabilitySupport::Supported),
        capability("marketplace.update", CapabilitySupport::Supported),
    ]);
    let project = CapabilitySet::new([
        capability("harness.observe", CapabilitySupport::Supported),
        capability("managed.projection", CapabilitySupport::Supported),
        capability("plugin.install", CapabilitySupport::Supported),
        capability("plugin.remove", CapabilitySupport::Supported),
        capability("plugin.update", CapabilitySupport::Supported),
        capability("marketplace.register", CapabilitySupport::Unsupported),
        capability("marketplace.remove", CapabilitySupport::Unsupported),
        capability("marketplace.update", CapabilitySupport::Unsupported),
    ]);
    ScopedCapabilitySets::new(global, project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).unwrap()
    }

    #[test]
    fn exact_factory_version_and_adjacent_versions_are_narrowly_authorized() {
        let adapter = FactoryAdapter;
        assert_eq!(
            adapter.decode_version(b"0.171.0\n").unwrap().as_str(),
            "0.171.0"
        );
        assert_eq!(
            adapter
                .select_profile(&NativeVersion::new("0.171.0").unwrap())
                .profile_id()
                .unwrap()
                .as_str(),
            PROFILE_ID
        );
        for version in ["0.170.0", "0.171.1", "99.0.0"] {
            assert!(
                adapter
                    .select_profile(&NativeVersion::new(version).unwrap())
                    .mutation_capabilities()
                    .is_none(),
                "{version} must remain observe-only"
            );
        }
    }

    #[test]
    fn factory_lifecycle_preserves_unscoped_marketplaces_and_scoped_plugins() {
        let source =
            skilltap_core::domain::SourceLocator::new("https://example.invalid/m.git").unwrap();
        let global = NativeLifecycleRequest {
            action: NativeLifecycleAction::PluginInstall,
            scope: Scope::Global,
            name: NativeId::new("demo@market").unwrap(),
            source: None,
        };
        assert_eq!(
            LIFECYCLE.arguments(&global).unwrap(),
            ["plugin", "install", "demo@market", "--scope", "user"].map(OsString::from)
        );
        let marketplace = NativeLifecycleRequest {
            action: NativeLifecycleAction::MarketplaceAdd,
            scope: Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            name: NativeId::new("market").unwrap(),
            source: Some(source),
        };
        assert_eq!(
            LIFECYCLE.arguments(&marketplace).unwrap(),
            [
                "plugin",
                "marketplace",
                "add",
                "https://example.invalid/m.git"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn requested_revision_never_claims_factory_native_fidelity() {
        let source = skilltap_core::domain::Source::new(
            skilltap_core::domain::SourceKind::Git,
            skilltap_core::domain::SourceLocator::new("https://example.invalid/market.git")
                .unwrap(),
            Some(skilltap_core::domain::RequestedRevision::new("release-1").unwrap()),
        )
        .unwrap();
        let checkout = skilltap_core::managed_projection::ResolvedSourceCheckout::new(
            AbsolutePath::new("/tmp/factory-checkout").unwrap(),
            source,
            None,
        );
        let target = HarnessId::new("droid").unwrap();
        let filesystem = skilltap_core::runtime::SystemFileSystem;
        assert_eq!(
            DISTRIBUTION.assess(&NativeDistributionContext {
                target: &target,
                scope: &Scope::Global,
                checkout: &checkout,
                requested_revision: checkout.source().requested_revision(),
                filesystem: &filesystem,
                json_limits: limits(),
            }),
            Ok(None)
        );
    }

    #[test]
    fn exact_factory_human_plugin_fixtures_decode_scope_and_revision() {
        assert_eq!(
            decode_factory_plugin_list(
                b"No plugins installed in project scope.\n",
                "demo@market",
                "project",
                limits(),
            ),
            NativeResourceObservation::Missing
        );
        assert_eq!(
            decode_factory_plugin_list(
                b"Installed plugins:\nActive:\n  demo@market  [user]  e8801fa\n",
                "demo@market",
                "user",
                limits(),
            ),
            NativeResourceObservation::Present {
                scope: Some(CapabilityScope::Global),
                revision: Some(skilltap_core::domain::ResolvedRevision::Native(
                    NativeId::new("e8801fa").unwrap(),
                )),
            }
        );
        assert!(matches!(
            decode_factory_plugin_list(b"{\"plugins\":[]}\n", "demo@market", "user", limits()),
            NativeResourceObservation::Indeterminate(NativeObservationFailure::UnsupportedShape)
        ));
    }
}
