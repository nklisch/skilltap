use std::collections::{BTreeMap, BTreeSet};

use skilltap_core::{
    domain::{AbsolutePath, NativeId, RelativeArtifactPath, ResourceKind},
    instructions::fingerprint_contents,
    managed_projection::{ManagedFileWrite, ManagedProjectionError, ManagedProjectionPlan},
    runtime::{StrictJson, StrictJsonDecoder},
    storage::ManagedProjection,
};

use crate::managed_projection::{
    ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort,
};

use super::super::configuration_constrained::{
    AuthenticationRequirement, PortableMcpServer, PortableRemoteTransport, SelectedPortablePlugin,
    common::{evidence, plan_skills, read_optional_file},
    load_selected_plugin,
};
use super::junie::junie_home;

const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];

static PROJECTION: JunieManagedProjection = JunieManagedProjection;

pub struct JunieManagedProjection;

impl JunieManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for JunieManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
        match context.resource_kind {
            ResourceKind::Marketplace => Ok(ManagedProjectionPlan::default()),
            ResourceKind::Plugin => plan_plugin(context),
            _ => Err(ManagedProjectionError::UnsupportedResourceKind),
        }
    }
}

fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => Some(load_selected_plugin(
            context,
            checkout,
            MARKETPLACE_DOCUMENTS,
        )?),
        ManagedProjectionInput::Remove => None,
    };
    let skill_root = match context.scope {
        skilltap_core::domain::Scope::Global => {
            AbsolutePath::new(format!("{}/skills", junie_home(context.paths).as_str()))
        }
        skilltap_core::domain::Scope::Project(project) => {
            AbsolutePath::new(format!("{}/.junie/skills", project.as_str()))
        }
    }
    .map_err(|_| destination_error())?;
    let (trees, mut current_parts, mut desired_parts, mut manifest) =
        plan_skills(&skill_root, context, plugin.as_ref())?;
    let (mcp_write, mcp_manifest) = plan_mcp(
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;
    manifest.extend(mcp_manifest);
    manifest.sort();
    manifest.dedup();
    if trees.is_empty() && mcp_write.is_none() {
        return Err(ManagedProjectionError::Other {
            code: "junie_managed_plugin_unsupported",
            summary: "The plugin has no faithful Junie skill or MCP projection.",
        });
    }
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if removal {
        manifest.clear();
    }
    Ok(ManagedProjectionPlan {
        trees,
        files: mcp_write.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}

fn plan_mcp(
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&SelectedPortablePlugin>,
    fingerprints: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<(Option<ManagedFileWrite>, Vec<ManagedProjection>), ManagedProjectionError> {
    let (root, destination) = match context.scope {
        skilltap_core::domain::Scope::Global => {
            (junie_home(context.paths), relative("mcp/mcp.json")?)
        }
        skilltap_core::domain::Scope::Project(project) => {
            (project.clone(), relative(".junie/mcp/mcp.json")?)
        }
    };
    let expected = read_optional_file(
        context.filesystem,
        &root,
        &destination,
        context.json_limits.bytes(),
        "The Junie MCP document could not be read safely.",
    )?;
    let mut document = match expected.as_deref() {
        Some(bytes) => StrictJson
            .decode(bytes, context.json_limits)
            .map_err(|_| mcp_invalid("The Junie MCP document is invalid JSON."))?
            .value()
            .as_object()
            .cloned()
            .ok_or_else(|| mcp_invalid("The Junie MCP document must be an object."))?,
        None => serde_json::Map::new(),
    };
    let current_servers = servers(&document)?;

    // Junie does not document a same-name user/project merge rule. Treating
    // both declarations as a hard conflict is safer than guessing which one
    // the interactive loader will choose.
    if let skilltap_core::domain::Scope::Project(project) = context.scope {
        let user_root = junie_home(context.paths);
        let user_destination = relative("mcp/mcp.json")?;
        let user_bytes = read_optional_file(
            context.filesystem,
            &user_root,
            &user_destination,
            context.json_limits.bytes(),
            "The Junie user MCP document could not be read safely.",
        )?;
        if let Some(user_bytes) = user_bytes {
            let user_document = StrictJson
                .decode(&user_bytes, context.json_limits)
                .map_err(|_| mcp_invalid("The Junie user MCP document is invalid JSON."))?;
            let user_servers = user_document
                .value()
                .as_object()
                .ok_or_else(|| mcp_invalid("The Junie user MCP document must be an object."))?;
            let user_servers = user_servers
                .get("mcpServers")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| {
                    mcp_invalid("The Junie user mcpServers member must be an object.")
                })?;
            let source_names = plugin
                .map(|plugin| {
                    plugin
                        .mcp
                        .keys()
                        .map(NativeId::as_str)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            if user_servers.keys().any(|name| {
                current_servers.contains_key(name) || source_names.contains(name.as_str())
            }) {
                let _ = project;
                return Err(ManagedProjectionError::McpConflict);
            }
        }
    }

    let mut names = BTreeSet::new();
    if let Some(plugin) = plugin {
        names.extend(plugin.mcp.keys().cloned());
    }
    names.extend(
        context
            .prior
            .iter()
            .filter_map(|projection| match projection {
                ManagedProjection::Mcp { id, .. } => Some(id.clone()),
                _ => None,
            }),
    );

    let mut manifest = Vec::new();
    let mut touched = false;
    for id in names {
        let name = id.as_str();
        let current = current_servers.get(name).cloned();
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
            if current.as_ref().map(json_fingerprint).as_ref() != Some(expected_fingerprint) {
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned Junie MCP server is missing or was replaced.",
                });
            }
            if let Some(current) = &current {
                fingerprints.0.extend(json_fingerprint_bytes(current));
            }
        } else if current.is_some() && !matches!(context.input, ManagedProjectionInput::Remove) {
            return Err(ManagedProjectionError::McpConflict);
        }

        let source = plugin.and_then(|plugin| plugin.mcp.get(&id));
        let Some(source) = source else {
            if prior.is_some()
                && document
                    .get_mut("mcpServers")
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|servers| servers.remove(name).is_some())
            {
                touched = true;
            }
            continue;
        };
        let mapped = match map_junie_server(source) {
            Ok(mapped) => mapped,
            Err(_) if is_required_mcp(&plugin.expect("source exists").declarations, name) => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: component_id(&id)?,
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                continue;
            }
        };
        let mapped = merge_object(current.as_ref(), mapped);
        touched |= current.as_ref() != Some(&mapped);
        fingerprints.1.extend(json_fingerprint_bytes(&mapped));
        manifest.push(ManagedProjection::Mcp {
            id: id.clone(),
            fingerprint: json_fingerprint(&mapped),
        });
        document
            .entry("mcpServers".to_owned())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| mcp_invalid("The Junie mcpServers member must be an object."))?
            .insert(name.to_owned(), mapped);
    }
    if !touched {
        return Ok((None, manifest));
    }
    let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(document))
        .map_err(|_| mcp_invalid("The Junie MCP document could not be encoded."))?;
    bytes.push(b'\n');
    Ok((
        Some(ManagedFileWrite {
            root,
            destination,
            expected,
            desired: Some(bytes),
        }),
        manifest,
    ))
}

fn map_junie_server(source: &PortableMcpServer) -> Result<serde_json::Value, ()> {
    match source {
        PortableMcpServer::Stdio {
            command,
            args,
            environment,
            cwd,
            enabled,
            ..
        } => {
            if cwd.is_some() || !enabled {
                return Err(());
            }
            Ok(serde_json::json!({
                "command": command,
                "args": args,
                "env": environment,
            }))
        }
        PortableMcpServer::Remote {
            transport,
            url,
            headers,
            authentication,
            enabled,
            ..
        } => {
            if !matches!(transport, PortableRemoteTransport::Http)
                || !matches!(
                    authentication,
                    AuthenticationRequirement::None | AuthenticationRequirement::StaticReferences
                )
                || !enabled
            {
                return Err(());
            }
            Ok(serde_json::json!({ "url": url, "headers": headers }))
        }
    }
}

fn servers(
    document: &serde_json::Map<String, serde_json::Value>,
) -> Result<BTreeMap<String, serde_json::Value>, ManagedProjectionError> {
    match document.get("mcpServers") {
        None => Ok(BTreeMap::new()),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| mcp_invalid("The Junie mcpServers member must be an object."))
            .map(|servers| servers.into_iter().collect()),
    }
}

fn merge_object(
    current: Option<&serde_json::Value>,
    mapped: serde_json::Value,
) -> serde_json::Value {
    let Some(current) = current.and_then(serde_json::Value::as_object) else {
        return mapped;
    };
    let Some(mut mapped_object) = mapped.as_object().cloned() else {
        return mapped;
    };
    for (key, value) in current {
        mapped_object
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }
    serde_json::Value::Object(mapped_object)
}

fn relative(path: &str) -> Result<RelativeArtifactPath, ManagedProjectionError> {
    RelativeArtifactPath::new(path).map_err(|_| destination_error())
}

fn component_id(
    id: &NativeId,
) -> Result<skilltap_core::domain::ComponentId, ManagedProjectionError> {
    skilltap_core::domain::ComponentId::new(format!("mcp:{}", id.as_str()))
        .map_err(|_| mcp_invalid("The Junie MCP component id is invalid."))
}

fn is_required_mcp(
    declarations: &[skilltap_core::plugin_graph::ComponentDeclaration],
    name: &str,
) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == skilltap_core::domain::ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == skilltap_core::domain::ComponentRequiredness::Required
    })
}

fn destination_error() -> ManagedProjectionError {
    ManagedProjectionError::Other {
        code: "junie_managed_destination_invalid",
        summary: "The Junie managed destination is invalid.",
    }
}

fn mcp_invalid(detail: &'static str) -> ManagedProjectionError {
    ManagedProjectionError::McpInvalid { detail }
}

fn json_fingerprint(value: &serde_json::Value) -> skilltap_core::domain::Fingerprint {
    fingerprint_contents(&json_fingerprint_bytes(value))
}

fn json_fingerprint_bytes(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::configuration_constrained::AuthenticationRequirement;

    #[test]
    fn junie_maps_only_documented_enabled_transports() {
        let local = PortableMcpServer::Stdio {
            command: "/bin/true".into(),
            args: vec!["--safe".into()],
            environment: BTreeMap::from([(String::from("TOKEN"), String::from("${TOKEN}"))]),
            cwd: None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert_eq!(map_junie_server(&local).unwrap()["command"], "/bin/true");
        let oauth = PortableMcpServer::Remote {
            transport: PortableRemoteTransport::Http,
            url: "https://example.invalid".into(),
            headers: BTreeMap::new(),
            authentication: AuthenticationRequirement::OAuth,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert!(map_junie_server(&oauth).is_err());
    }
}
