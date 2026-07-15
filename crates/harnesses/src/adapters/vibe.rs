use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    str::FromStr,
};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        ComponentKind, ComponentRequiredness, HarnessId, NativeId, NativeVersion, Scope,
        ScopedCapabilitySets,
    },
    instructions::fingerprint_contents,
    mutation_authority::{ManagedDeclarationContract, ManagedSurfaceKind},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, ObservationRuntimeError,
        PlatformPaths, SystemExternalTreeObserver,
    },
    storage::ManagedProjection,
};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, value};

use super::configuration_constrained::common::{evidence, plan_skills, read_optional_file};
use super::configuration_constrained::{
    AuthenticationRequirement, PortableMcpServer, PortableRemoteTransport, SelectedPortablePlugin,
    load_selected_plugin,
};
use crate::{
    adapter_helpers,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

const VERIFIED_VERSION: &str = "2.19.1";
const PROFILE_ID: &str = "vibe-2-19-1";
const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
static ADAPTER: VibeAdapter = VibeAdapter;
static SKILLS: VibeSkillProjection = VibeSkillProjection;
static PROJECTION: VibeManagedProjection = VibeManagedProjection;
static DECLARATION_CONTRACT: std::sync::LazyLock<ManagedDeclarationContract> =
    std::sync::LazyLock::new(|| {
        ManagedDeclarationContract::new([
            ManagedSurfaceKind::ManagedDocument,
            ManagedSurfaceKind::CompleteSkillTree,
        ])
        .expect("Vibe declaration contract is non-empty")
    });

pub struct VibeAdapter;
pub struct VibeSkillProjection;
pub struct VibeManagedProjection;

impl VibeAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn capabilities() -> ScopedCapabilitySets {
    let cap = |id: &str, support: CapabilitySupport| {
        (
            CapabilityId::new(id).expect("Vibe capability is valid"),
            support,
        )
    };
    let make = || {
        CapabilitySet::new([
            cap("harness.observe", CapabilitySupport::Supported),
            cap("managed.projection", CapabilitySupport::Unverified),
            cap("component.skill", CapabilitySupport::Supported),
            cap("component.mcp", CapabilitySupport::Unverified),
            cap("skill.install", CapabilitySupport::Supported),
            cap("skill.update", CapabilitySupport::Supported),
            cap("skill.remove", CapabilitySupport::Supported),
        ])
    };
    ScopedCapabilitySets::new(make(), make())
}

impl HarnessAdapter for VibeAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("vibe").expect("static harness id is valid"),
            display_name: "Mistral Vibe",
            default_binary: "vibe",
            distribution_surface: DistributionSurface::Managed,
        }
    }
    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text =
            std::str::from_utf8(stdout).map_err(|_| crate::DetectionError::InvalidVersion)?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let version = text
            .strip_prefix("vibe ")
            .filter(|value| !value.is_empty() && !value.chars().any(char::is_whitespace))
            .ok_or(crate::DetectionError::InvalidVersion)?;
        NativeVersion::new(version).map_err(|_| crate::DetectionError::InvalidVersion)
    }
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(version, VERIFIED_VERSION, PROFILE_ID, capabilities())
    }
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let (canonical_root, native_root, config_root, config_child, config_label) = match scope {
            Scope::Global => (
                AbsolutePath::new(format!("{}/.agents/skills", paths.home().as_str()))?,
                adapter_helpers::absolute_child(paths.vibe_home(), "skills"),
                paths.vibe_home().clone(),
                "config.toml",
                "vibe.global.config",
            ),
            Scope::Project(project) => (
                AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))?,
                adapter_helpers::absolute_child(project, ".vibe/skills"),
                project.clone(),
                ".vibe/config.toml",
                "vibe.project.config",
            ),
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = None;
        for (label, root) in [
            ("vibe.agents.skills", Some(canonical_root)),
            ("vibe.native.skills", native_root),
        ] {
            let Some(root) = root else { continue };
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    if label == "vibe.agents.skills" && matches!(scope, Scope::Project(_)) {
                        project_entry_count = Some(snapshot.entries().len());
                    }
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        let labels = if std::fs::symlink_metadata(
            std::path::Path::new(config_root.as_str()).join(config_child),
        )
        .is_ok()
        {
            vec![config_label]
        } else {
            Vec::new()
        };
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count,
            surface_labels: labels,
        })
    }
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(&PROJECTION)
    }
    fn managed_declaration_contract(
        &self,
        _scope: skilltap_core::domain::CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
    }
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(paths.vibe_home().clone())
    }
}

impl SkillProjectionPort for VibeSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

impl ManagedProjectionPort for VibeManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<
        skilltap_core::managed_projection::ManagedProjectionPlan,
        skilltap_core::managed_projection::ManagedProjectionError,
    > {
        match context.resource_kind {
            skilltap_core::domain::ResourceKind::Marketplace => Ok(Default::default()),
            skilltap_core::domain::ResourceKind::Plugin => plan_plugin(context),
            _ => Err(
                skilltap_core::managed_projection::ManagedProjectionError::UnsupportedResourceKind,
            ),
        }
    }
}

fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<
    skilltap_core::managed_projection::ManagedProjectionPlan,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => Some(load_selected_plugin(
            context,
            checkout,
            MARKETPLACE_DOCUMENTS,
        )?),
        ManagedProjectionInput::Remove => None,
    };
    let skill_root = AbsolutePath::new(match context.scope {
        Scope::Global => format!("{}/.agents/skills", context.paths.home().as_str()),
        Scope::Project(project) => format!("{}/.agents/skills", project.as_str()),
    })
    .map_err(|_| destination_error())?;
    let (trees, mut current_parts, mut desired_parts, mut manifest) =
        plan_skills(&skill_root, context, plugin.as_ref(), "Vibe")?;
    let (file, mcp_manifest) = plan_mcp(
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;
    manifest.extend(mcp_manifest);
    manifest.sort();
    manifest.dedup();
    if trees.is_empty() && file.is_none() {
        return Err(
            skilltap_core::managed_projection::ManagedProjectionError::Other {
                code: "vibe_managed_plugin_unsupported",
                summary: "The plugin has no faithful Vibe skill or MCP projection.",
            },
        );
    }
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if removal {
        manifest.clear();
    }
    Ok(skilltap_core::managed_projection::ManagedProjectionPlan {
        trees,
        files: file.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}

struct VibeConfigDocument {
    document: DocumentMut,
}
impl VibeConfigDocument {
    fn parse(
        bytes: Option<&[u8]>,
    ) -> Result<Self, skilltap_core::managed_projection::ManagedProjectionError> {
        let document = match bytes {
            Some(bytes) => DocumentMut::from_str(
                std::str::from_utf8(bytes)
                    .map_err(|_| mcp_invalid("The Vibe TOML document is not UTF-8."))?,
            )
            .map_err(|_| mcp_invalid("The Vibe TOML document is invalid."))?,
            None => DocumentMut::new(),
        };
        Ok(Self { document })
    }
    fn servers_mut(&mut self) -> &mut toml_edit::ArrayOfTables {
        if !self.document.contains_key("mcp_servers") {
            self.document["mcp_servers"] = Item::ArrayOfTables(ArrayOfTables::new());
        }
        self.document["mcp_servers"]
            .as_array_of_tables_mut()
            .expect("array-of-tables inserted above")
    }
    fn encode(self) -> Vec<u8> {
        self.document.to_string().into_bytes()
    }
}

fn plan_mcp(
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&SelectedPortablePlugin>,
    fingerprints: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<
    (
        Option<skilltap_core::managed_projection::ManagedFileWrite>,
        Vec<ManagedProjection>,
    ),
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let (root, destination) = match context.scope {
        Scope::Global => (
            context.paths.vibe_home().clone(),
            skilltap_core::domain::RelativeArtifactPath::new("config.toml").unwrap(),
        ),
        Scope::Project(project) => (
            project.clone(),
            skilltap_core::domain::RelativeArtifactPath::new(".vibe/config.toml").unwrap(),
        ),
    };
    let expected = read_optional_file(
        context.filesystem,
        &root,
        &destination,
        context.json_limits.bytes(),
        "The Vibe TOML document could not be read safely.",
    )?;
    let mut doc = VibeConfigDocument::parse(expected.as_deref())?;
    let existing = doc
        .document
        .get("mcp_servers")
        .and_then(Item::as_array_of_tables)
        .map(|servers| {
            servers
                .iter()
                .map(server_name)
                .collect::<Result<BTreeMap<_, _>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut names = BTreeSet::new();
    if let Some(plugin) = plugin {
        names.extend(plugin.mcp.keys().cloned());
    }
    names.extend(context.prior.iter().filter_map(|projection| {
        if let ManagedProjection::Mcp { id, .. } = projection {
            Some(id.clone())
        } else {
            None
        }
    }));
    let mut manifest = Vec::new();
    let mut touched = false;
    for id in names {
        let current = existing.get(&id).cloned();
        let prior = context
            .prior
            .iter()
            .find_map(|projection| match projection {
                ManagedProjection::Mcp {
                    id: prior_id,
                    fingerprint,
                } if prior_id == &id => Some(fingerprint),
                _ => None,
            });
        if let Some(expected_fingerprint) = prior {
            if current.as_ref().map(toml_item_fingerprint).as_ref() != Some(expected_fingerprint) {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::Drifted {
                        detail: "An owned Vibe MCP server is missing or was replaced.",
                    },
                );
            }
            if let Some(current) = &current {
                fingerprints.0.extend(toml_item_fingerprint_bytes(current));
            }
        } else if current.is_some() && !matches!(context.input, ManagedProjectionInput::Remove) {
            return Err(skilltap_core::managed_projection::ManagedProjectionError::McpConflict);
        }
        let source = plugin.and_then(|plugin| plugin.mcp.get(&id));
        let Some(source) = source else {
            if prior.is_some() {
                touched |= remove_server(doc.servers_mut(), &id);
            }
            continue;
        };
        let mut mapped = match map_vibe_server(source) {
            Ok(value) => value,
            Err(_)
                if is_required_mcp(&plugin.expect("source exists").declarations, id.as_str()) =>
            {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported,
                );
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: component(&id)?,
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                continue;
            }
        };
        mapped["name"] = value(id.as_str());
        let changed = upsert_server(doc.servers_mut(), &id, &mapped)?;
        touched |= changed;
        let fp = toml_item_fingerprint(&mapped);
        fingerprints.1.extend(toml_item_fingerprint_bytes(&mapped));
        manifest.push(ManagedProjection::Mcp {
            id: id.clone(),
            fingerprint: fp,
        });
    }
    if !touched {
        return Ok((None, manifest));
    }
    let desired = doc.encode();
    Ok((
        Some(skilltap_core::managed_projection::ManagedFileWrite {
            root,
            destination,
            expected,
            desired: Some(desired),
        }),
        manifest,
    ))
}

fn server_name(
    table: &Table,
) -> Result<(NativeId, Table), skilltap_core::managed_projection::ManagedProjectionError> {
    let name = table
        .get("name")
        .and_then(Item::as_str)
        .ok_or_else(|| mcp_invalid("A Vibe MCP table has no name."))?;
    Ok((
        NativeId::new(name).map_err(|_| mcp_invalid("A Vibe MCP name is invalid."))?,
        table.clone(),
    ))
}
fn upsert_server(
    servers: &mut ArrayOfTables,
    id: &NativeId,
    mapped: &Table,
) -> Result<bool, skilltap_core::managed_projection::ManagedProjectionError> {
    for index in 0..servers.len() {
        if servers
            .get(index)
            .and_then(|table| table.get("name"))
            .and_then(Item::as_str)
            == Some(id.as_str())
        {
            let table = servers.get_mut(index).expect("index checked");
            let before = table.to_string();
            for (key, value) in mapped.iter() {
                table[key] = value.clone();
            }
            return Ok(before != table.to_string());
        }
    }
    servers.push(mapped.clone());
    Ok(true)
}
fn remove_server(servers: &mut ArrayOfTables, id: &NativeId) -> bool {
    for index in 0..servers.len() {
        if servers
            .get(index)
            .and_then(|table| table.get("name"))
            .and_then(Item::as_str)
            == Some(id.as_str())
        {
            servers.remove(index);
            return true;
        }
    }
    false
}
fn map_vibe_server(source: &PortableMcpServer) -> Result<Table, ()> {
    let mut table = Table::new();
    match source {
        PortableMcpServer::Stdio {
            command,
            args,
            environment,
            cwd,
            enabled,
            timeout_ms,
            tools,
        } => {
            // Vibe's 2.19.1 contract does not attest a stdio `cwd` field. Do
            // not silently drop it or emit a declaration with changed
            // execution semantics; the caller classifies this error as an
            // optional omission or required blocker.
            if cwd.is_some() {
                return Err(());
            }
            table["name"] = value("");
            table["transport"] = value("stdio");
            table["command"] = value(command.clone());
            table["args"] = value(string_array(args));
            if !environment.is_empty() {
                table["env"] = value(string_map(environment));
            }
            if let Some(timeout) = timeout_ms {
                table["timeout"] = value(*timeout as i64);
            }
            if let Some(tools) = tools {
                table["tools"] = value(string_array(&tools.iter().cloned().collect::<Vec<_>>()));
            }
            table["enabled"] = value(*enabled);
        }
        PortableMcpServer::Remote {
            transport,
            url,
            headers,
            authentication,
            enabled,
            timeout_ms,
            tools,
        } => {
            if matches!(authentication, AuthenticationRequirement::OAuth) {
                return Err(());
            }
            table["name"] = value("");
            table["transport"] = value(match transport {
                PortableRemoteTransport::Http => "http",
                PortableRemoteTransport::StreamableHttp => "streamable-http",
                PortableRemoteTransport::Sse => return Err(()),
            });
            table["url"] = value(url.clone());
            if !headers.is_empty() {
                table["headers"] = value(string_map(headers));
            }
            if let Some(timeout) = timeout_ms {
                table["timeout"] = value(*timeout as i64);
            }
            if let Some(tools) = tools {
                table["tools"] = value(string_array(&tools.iter().cloned().collect::<Vec<_>>()));
            }
            table["enabled"] = value(*enabled);
        }
    }
    Ok(table)
}

fn string_array(values: &[String]) -> Array {
    let mut array = Array::new();
    for value in values {
        array.push(value.as_str());
    }
    array
}
fn string_map(values: &BTreeMap<String, String>) -> InlineTable {
    let mut table = InlineTable::new();
    for (key, item) in values {
        table.insert(key, item.as_str().into());
    }
    table
}

fn toml_item_fingerprint(table: &Table) -> skilltap_core::domain::Fingerprint {
    fingerprint_contents(&toml_item_fingerprint_bytes(table))
}
fn toml_item_fingerprint_bytes(table: &Table) -> Vec<u8> {
    table.to_string().into_bytes()
}
fn is_required_mcp(
    declarations: &[skilltap_core::plugin_graph::ComponentDeclaration],
    name: &str,
) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}
fn component(
    id: &NativeId,
) -> Result<
    skilltap_core::domain::ComponentId,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    skilltap_core::domain::ComponentId::new(format!("mcp:{}", id.as_str()))
        .map_err(|_| mcp_invalid("The Vibe MCP component id is invalid."))
}
fn destination_error() -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::Other {
        code: "vibe_managed_destination_invalid",
        summary: "The Vibe managed destination is invalid.",
    }
}
fn mcp_invalid(detail: &'static str) -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::McpInvalid { detail }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{CapabilityId, CapabilityScope, CapabilitySupport};
    #[test]
    fn no_probe_or_native_lifecycle_can_be_used_for_vibe() {
        let adapter = VibeAdapter;
        assert!(adapter.effective_state_probe().is_none());
        assert!(adapter.native_lifecycle().is_none());
        assert_eq!(
            adapter.decode_version(b"vibe 2.19.1\n").unwrap().as_str(),
            "2.19.1"
        );
        let profile = adapter.select_profile(&NativeVersion::new("2.19.1").unwrap());
        let capabilities = profile.mutation_capabilities().unwrap();
        for scope in [CapabilityScope::Global, CapabilityScope::Project] {
            assert_eq!(
                capabilities
                    .for_scope_kind(scope)
                    .support(&CapabilityId::new("managed.projection").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
        }
    }
    #[test]
    fn toml_document_retains_comments_and_unknown_tables() {
        let mut doc = VibeConfigDocument::parse(Some(b"# keep\n[future]\nvalue = 1\n")).unwrap();
        let mut mapped = map_vibe_server(&PortableMcpServer::Stdio {
            command: "node".into(),
            args: vec![],
            environment: BTreeMap::new(),
            cwd: None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        })
        .unwrap();
        mapped["name"] = value("demo");
        upsert_server(doc.servers_mut(), &NativeId::new("demo").unwrap(), &mapped).unwrap();
        let bytes = doc.encode();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("# keep"));
        assert!(text.contains("[future]"));
        assert!(text.contains("name = \"demo\""));
        let mut doc = VibeConfigDocument::parse(Some(&bytes)).unwrap();
        assert!(remove_server(
            doc.servers_mut(),
            &NativeId::new("demo").unwrap()
        ));
        assert!(
            !String::from_utf8(doc.encode())
                .unwrap()
                .contains("name = \"demo\"")
        );
    }

    #[test]
    fn vibe_rejects_stdio_cwd_without_emitting_unattested_semantics() {
        let server = PortableMcpServer::Stdio {
            command: "node".into(),
            args: vec!["server.js".into()],
            environment: BTreeMap::new(),
            cwd: Some("/opt/server".into()),
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert!(map_vibe_server(&server).is_err());
    }

    #[test]
    fn vibe_rejects_oauth_and_sse_without_process_probes() {
        let oauth = PortableMcpServer::Remote {
            transport: PortableRemoteTransport::Http,
            url: "https://example.invalid".into(),
            headers: BTreeMap::new(),
            authentication: AuthenticationRequirement::OAuth,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        let sse = PortableMcpServer::Remote {
            transport: PortableRemoteTransport::Sse,
            url: "https://example.invalid".into(),
            headers: BTreeMap::new(),
            authentication: AuthenticationRequirement::None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert!(map_vibe_server(&oauth).is_err());
        assert!(map_vibe_server(&sse).is_err());
    }
}
