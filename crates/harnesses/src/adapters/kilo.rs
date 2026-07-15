use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
};

use serde_json::Value;
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

const VERIFIED_VERSION: &str = "7.4.7";
const PROFILE_ID: &str = "kilo-7-4-7";
const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
static ADAPTER: KiloAdapter = KiloAdapter;
static SKILLS: KiloSkillProjection = KiloSkillProjection;
static PROJECTION: KiloManagedProjection = KiloManagedProjection;
static DECLARATION_CONTRACT: std::sync::LazyLock<ManagedDeclarationContract> =
    std::sync::LazyLock::new(|| {
        ManagedDeclarationContract::new([
            ManagedSurfaceKind::ManagedDocument,
            ManagedSurfaceKind::CompleteSkillTree,
        ])
        .expect("Kilo declaration contract is non-empty")
    });

pub struct KiloAdapter;
pub struct KiloSkillProjection;
pub struct KiloManagedProjection;

impl KiloAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn capabilities() -> ScopedCapabilitySets {
    let cap = |id: &str, support: CapabilitySupport| {
        (
            CapabilityId::new(id).expect("Kilo capability is valid"),
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

impl HarnessAdapter for KiloAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("kilo").expect("static harness id is valid"),
            display_name: "Kilo Code",
            default_binary: Some("kilo"),
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: crate::TargetIdentityBoundary::Executable,
        }
    }
    fn version_arguments(&self) -> Option<Vec<OsString>> {
        Some(vec![OsString::from("--version")])
    }
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text =
            std::str::from_utf8(stdout).map_err(|_| crate::DetectionError::InvalidVersion)?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        if text.is_empty() || text.chars().any(char::is_whitespace) {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(text).map_err(|_| crate::DetectionError::InvalidVersion)
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
        let (canonical_root, native_root, _config_root, labels) = match scope {
            Scope::Global => (
                AbsolutePath::new(format!("{}/.agents/skills", paths.home().as_str()))?,
                adapter_helpers::absolute_child(paths.config_home(), "kilo/skills"),
                paths.config_home().clone(),
                vec!["kilo.global.config"],
            ),
            Scope::Project(project) => (
                AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))?,
                adapter_helpers::absolute_child(project, ".kilo/skills"),
                project.clone(),
                vec!["kilo.project.config"],
            ),
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = None;
        for (label, root) in [
            ("kilo.agents.skills", Some(canonical_root)),
            ("kilo.native.skills", native_root),
        ] {
            let Some(root) = root else { continue };
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    if label == "kilo.agents.skills" && matches!(scope, Scope::Project(_)) {
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
        let config_exists = {
            let system_fs = skilltap_core::runtime::SystemFileSystem;
            resolve_document(paths, scope, &system_fs, 64 * 1024)
                .ok()
                .flatten()
                .is_some_and(|(root, path)| {
                    adapter_helpers::child_path_exists(&root, path.as_str())
                })
        };
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count,
            surface_labels: if config_exists { labels } else { Vec::new() },
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
        adapter_helpers::absolute_child(paths.config_home(), "kilo")
    }
}

impl SkillProjectionPort for KiloSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

impl ManagedProjectionPort for KiloManagedProjection {
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
        plan_skills(&skill_root, context, plugin.as_ref())?;
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
                code: "kilo_managed_plugin_unsupported",
                summary: "The plugin has no faithful Kilo skill or MCP projection.",
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

#[derive(Clone, Debug)]
struct KiloJsoncDocument {
    source: Vec<u8>,
    value: Value,
    root_start: usize,
    root_end: usize,
}
#[derive(Clone, Debug)]
struct JsonMember {
    key: String,
    key_start: usize,
    value_start: usize,
    value_end: usize,
}

impl KiloJsoncDocument {
    fn parse(
        bytes: Option<&[u8]>,
    ) -> Result<Self, skilltap_core::managed_projection::ManagedProjectionError> {
        let source = bytes.unwrap_or(b"{}").to_vec();
        let clean = strip_jsonc(&source);
        let value: Value = serde_json::from_slice(&strip_trailing_commas(&clean))
            .map_err(|_| mcp_invalid("The Kilo JSONC document is invalid."))?;
        let object = value
            .as_object()
            .ok_or_else(|| mcp_invalid("The Kilo configuration must be an object."))?;
        validate_schema(object)?;
        let (root_start, root_end) = object_span(&source, 0)
            .ok_or_else(|| mcp_invalid("The Kilo JSONC root object is invalid."))?;
        if let Some(mcp) = object.get("mcp")
            && !mcp.is_object()
        {
            return Err(mcp_invalid("The Kilo mcp member must be an object."));
        }
        Ok(Self {
            source,
            value,
            root_start,
            root_end,
        })
    }
    fn servers(
        &self,
    ) -> Result<BTreeMap<NativeId, Value>, skilltap_core::managed_projection::ManagedProjectionError>
    {
        let Some(mcp) = self.value.get("mcp") else {
            return Ok(BTreeMap::new());
        };
        mcp.as_object()
            .ok_or_else(|| mcp_invalid("The Kilo mcp member must be an object."))?
            .iter()
            .map(|(name, value)| {
                Ok((
                    NativeId::new(name).map_err(|_| mcp_invalid("A Kilo MCP name is invalid."))?,
                    value.clone(),
                ))
            })
            .collect()
    }
    fn upsert(
        &mut self,
        id: &NativeId,
        mapped: Value,
    ) -> Result<bool, skilltap_core::managed_projection::ManagedProjectionError> {
        let encoded = serde_json::to_string(&mapped)
            .map_err(|_| mcp_invalid("The Kilo MCP value could not be encoded."))?;
        if self.value.get("mcp").is_some() {
            let span = object_span_for_key(&self.source, self.root_start, self.root_end, "mcp")
                .ok_or_else(|| mcp_invalid("The Kilo mcp object could not be located."))?;
            let members = object_members(&self.source, span.0, span.1);
            if let Some(member) = members.iter().find(|member| member.key == id.as_str()) {
                let old = self.source[member.value_start..member.value_end].to_vec();
                if old == encoded.as_bytes() {
                    self.set_value(id, mapped);
                    return Ok(false);
                }
                self.source
                    .splice(member.value_start..member.value_end, encoded.bytes());
                self.reindex();
                self.set_value(id, mapped);
                return Ok(true);
            }
            let insert = insertion_for_object(
                &self.source,
                span.0,
                span.1,
                &format!("\"{}\": {}", escape_json(id.as_str()), encoded),
            );
            self.source.splice(
                insert..insert,
                insert_bytes(
                    &self.source,
                    span.0,
                    span.1,
                    &format!("\"{}\": {}", escape_json(id.as_str()), encoded),
                ),
            );
            self.reindex();
            self.set_value(id, mapped);
            return Ok(true);
        }
        let member = format!("\"mcp\": {{\"{}\": {encoded}}}", escape_json(id.as_str()));
        let insert = insertion_for_object(&self.source, self.root_start, self.root_end, &member);
        self.source.splice(
            insert..insert,
            insert_bytes(&self.source, self.root_start, self.root_end, &member),
        );
        self.reindex();
        self.set_value(id, mapped);
        Ok(true)
    }
    fn remove(
        &mut self,
        id: &NativeId,
    ) -> Result<bool, skilltap_core::managed_projection::ManagedProjectionError> {
        let Some(mcp_span) =
            object_span_for_key(&self.source, self.root_start, self.root_end, "mcp")
        else {
            return Ok(false);
        };
        let members = object_members(&self.source, mcp_span.0, mcp_span.1);
        let Some(member) = members.into_iter().find(|member| member.key == id.as_str()) else {
            return Ok(false);
        };
        let (start, end) = member_removal_span(&self.source, mcp_span.0, mcp_span.1, &member);
        self.source.drain(start..end);
        self.reindex();
        if let Some(mcp) = self.value.get_mut("mcp").and_then(Value::as_object_mut) {
            mcp.remove(id.as_str());
        }
        Ok(true)
    }
    fn encode(self) -> Option<Vec<u8>> {
        (self
            .value
            .get("mcp")
            .and_then(Value::as_object)
            .is_some_and(|mcp| !mcp.is_empty())
            || self
                .value
                .as_object()
                .is_some_and(|object| object.len() > 1 || !object.contains_key("mcp")))
        .then_some(self.source)
    }
    fn set_value(&mut self, id: &NativeId, mapped: Value) {
        if let Some(mcp) = self
            .value
            .as_object_mut()
            .and_then(|object| object.get_mut("mcp"))
            .and_then(Value::as_object_mut)
        {
            mcp.insert(id.as_str().to_owned(), mapped);
        }
    }
    fn reindex(&mut self) {
        if let Some((start, end)) = object_span(&self.source, 0) {
            self.root_start = start;
            self.root_end = end;
        }
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
    let (root, destination) = resolve_document(
        context.paths,
        context.scope,
        context.filesystem,
        context.json_limits.bytes(),
    )?
    .ok_or_else(|| mcp_invalid("No valid Kilo configuration location is available."))?;
    let expected = read_optional_file(
        context.filesystem,
        &root,
        &destination,
        context.json_limits.bytes(),
        "The Kilo JSONC document could not be read safely.",
    )?;
    let mut doc = KiloJsoncDocument::parse(expected.as_deref())?;
    let existing = doc.servers()?;
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
            if current.as_ref().map(json_fingerprint).as_ref() != Some(expected_fingerprint) {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::Drifted {
                        detail: "An owned Kilo MCP server is missing or was replaced.",
                    },
                );
            }
            if let Some(value) = &current {
                fingerprints.0.extend(json_fingerprint_bytes(value));
            }
        } else if current.is_some() && !matches!(context.input, ManagedProjectionInput::Remove) {
            return Err(skilltap_core::managed_projection::ManagedProjectionError::McpConflict);
        }
        let source = plugin.and_then(|plugin| plugin.mcp.get(&id));
        let Some(source) = source else {
            if prior.is_some() {
                touched |= doc.remove(&id)?;
            }
            continue;
        };
        let mapped = match map_kilo_server(source) {
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
        touched |= doc.upsert(&id, mapped.clone())?;
        let fp = json_fingerprint(&mapped);
        fingerprints.1.extend(json_fingerprint_bytes(&mapped));
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
            desired,
        }),
        manifest,
    ))
}

fn resolve_document(
    paths: &PlatformPaths,
    scope: &Scope,
    filesystem: &dyn skilltap_core::runtime::ConfinedFileSystem,
    max_bytes: u64,
) -> Result<
    Option<(AbsolutePath, skilltap_core::domain::RelativeArtifactPath)>,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    match scope {
        Scope::Global => {
            let jsonc =
                skilltap_core::domain::RelativeArtifactPath::new("kilo/kilo.jsonc").unwrap();
            let json = skilltap_core::domain::RelativeArtifactPath::new("kilo/kilo.json").unwrap();
            let jsonc_exists = read_optional_file(
                filesystem,
                paths.config_home(),
                &jsonc,
                max_bytes,
                "The Kilo global configuration could not be read safely.",
            )?
            .is_some();
            let json_exists = read_optional_file(
                filesystem,
                paths.config_home(),
                &json,
                max_bytes,
                "The Kilo global configuration could not be read safely.",
            )?
            .is_some();
            if jsonc_exists && json_exists {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::Other {
                        code: "configuration.higher-precedence",
                        summary: "Multiple Kilo global configuration locations conflict.",
                    },
                );
            }
            let child = if json_exists {
                "kilo/kilo.json"
            } else {
                "kilo/kilo.jsonc"
            };
            Ok(Some((
                paths.config_home().clone(),
                skilltap_core::domain::RelativeArtifactPath::new(child).unwrap(),
            )))
        }
        Scope::Project(project) => {
            let candidates = [
                "kilo.jsonc",
                "kilo.json",
                ".kilo/kilo.jsonc",
                ".kilo/kilo.json",
            ];
            let mut existing = Vec::new();
            for child in candidates {
                let path = skilltap_core::domain::RelativeArtifactPath::new(child).unwrap();
                let bytes = read_optional_file(
                    filesystem,
                    project,
                    &path,
                    max_bytes,
                    "The Kilo project configuration could not be read safely.",
                )?;
                if bytes.is_some() {
                    existing.push(child);
                }
            }
            if existing.len() > 1 {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::Other {
                        code: "configuration.higher-precedence",
                        summary: "Multiple Kilo project configuration locations conflict.",
                    },
                );
            }
            let child = existing.first().copied().unwrap_or("kilo.jsonc");
            Ok(Some((
                project.clone(),
                skilltap_core::domain::RelativeArtifactPath::new(child).unwrap(),
            )))
        }
    }
}

fn map_kilo_server(source: &PortableMcpServer) -> Result<Value, ()> {
    match source {
        PortableMcpServer::Stdio {
            command,
            args,
            environment,
            enabled,
            ..
        } => {
            let mut command_vec = vec![Value::String(command.clone())];
            command_vec.extend(args.iter().cloned().map(Value::String));
            Ok(
                serde_json::json!({"type":"local","command":command_vec,"environment":environment,"enabled":enabled}),
            )
        }
        PortableMcpServer::Remote {
            transport,
            url,
            headers,
            authentication,
            enabled,
            ..
        } => {
            if matches!(authentication, AuthenticationRequirement::OAuth)
                || !matches!(transport, PortableRemoteTransport::Http)
            {
                return Err(());
            }
            Ok(serde_json::json!({"type":"remote","url":url,"headers":headers,"enabled":enabled}))
        }
    }
}

fn validate_schema(
    object: &serde_json::Map<String, Value>,
) -> Result<(), skilltap_core::managed_projection::ManagedProjectionError> {
    // Keep this admission list to the keys directly evidenced by the locked
    // Kilo 7.4.7 contract: `mcp` in the documented config forms and
    // `username` in the isolated effective-config capture. Other valid Kilo
    // settings remain blocked until the release-sensitive schema records
    // them; accepting an ungrounded key could make Kilo reject the edited
    // document even though the bytes were otherwise preserved.
    const ATTESTED_TOP_LEVEL_KEYS: &[&str] = &["mcp", "username"];
    if object
        .keys()
        .any(|key| !ATTESTED_TOP_LEVEL_KEYS.contains(&key.as_str()))
    {
        return Err(
            skilltap_core::managed_projection::ManagedProjectionError::Other {
                code: "configuration.unknown-schema-key",
                summary: "The Kilo configuration contains an unknown schema key.",
            },
        );
    }
    Ok(())
}

fn strip_jsonc(source: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(source.len());
    let mut i = 0;
    let mut string = false;
    while i < source.len() {
        if string {
            output.push(source[i]);
            if source[i] == b'\\' && i + 1 < source.len() {
                i += 1;
                output.push(source[i]);
            } else if source[i] == b'"' {
                string = false;
            }
            i += 1;
            continue;
        }
        if source[i] == b'"' {
            string = true;
            output.push(source[i]);
            i += 1;
            continue;
        }
        if source[i] == b'/' && i + 1 < source.len() && source[i + 1] == b'/' {
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if source[i] == b'/' && i + 1 < source.len() && source[i + 1] == b'*' {
            i += 2;
            while i + 1 < source.len() && !(source[i] == b'*' && source[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(source.len());
            continue;
        }
        output.push(source[i]);
        i += 1;
    }
    output
}
fn strip_trailing_commas(source: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(source.len());
    let mut i = 0;
    while i < source.len() {
        if source[i] == b',' {
            let mut j = i + 1;
            while j < source.len() && source[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < source.len() && matches!(source[j], b'}' | b']') {
                i += 1;
                continue;
            }
        }
        output.push(source[i]);
        i += 1;
    }
    output
}
fn object_span(source: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i < source.len() && source[i].is_ascii_whitespace() {
        i += 1;
    }
    if source.get(i) != Some(&b'{') {
        return None;
    }
    Some((i, scan_value(source, i)?))
}
fn object_span_for_key(
    source: &[u8],
    start: usize,
    end: usize,
    key: &str,
) -> Option<(usize, usize)> {
    object_members(source, start, end)
        .into_iter()
        .find(|member| member.key == key)
        .and_then(|member| object_span(source, member.value_start))
}
fn object_members(source: &[u8], start: usize, end: usize) -> Vec<JsonMember> {
    let mut result = Vec::new();
    let mut i = start + 1;
    while i < end {
        i = skip_space_comments(source, i);
        if i >= end || source[i] == b'}' {
            break;
        }
        let key_start = i;
        let Some((key, next)) = scan_string(source, i) else {
            break;
        };
        i = skip_space_comments(source, next);
        if source.get(i) != Some(&b':') {
            break;
        }
        i = skip_space_comments(source, i + 1);
        let value_start = i;
        let Some(value_end) = scan_value(source, i) else {
            break;
        };
        result.push(JsonMember {
            key,
            key_start,
            value_start,
            value_end,
        });
        i = skip_space_comments(source, value_end);
        if source.get(i) == Some(&b',') {
            i += 1;
        } else if source.get(i) == Some(&b'}') {
            break;
        } else {
            let _ = key_start;
            break;
        }
    }
    result
}
fn skip_space_comments(source: &[u8], mut i: usize) -> usize {
    loop {
        while i < source.len() && source[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < source.len() && source[i] == b'/' && source[i + 1] == b'/' {
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < source.len() && source[i] == b'/' && source[i + 1] == b'*' {
            i += 2;
            while i + 1 < source.len() && !(source[i] == b'*' && source[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(source.len());
            continue;
        }
        break;
    }
    i
}
fn scan_string(source: &[u8], mut i: usize) -> Option<(String, usize)> {
    if source.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    let start = i;
    let mut bytes = Vec::new();
    while i < source.len() {
        match source[i] {
            b'\\' => {
                bytes.extend_from_slice(&source[start..i]);
                i += 1;
                if i < source.len() {
                    bytes.push(source[i]);
                    i += 1;
                }
            }
            b'"' => {
                bytes.extend_from_slice(&source[start..i]);
                let mut encoded = Vec::with_capacity(bytes.len() + 2);
                encoded.push(b'"');
                encoded.extend_from_slice(&bytes);
                encoded.push(b'"');
                return Some((serde_json::from_slice(&encoded).ok()?, i + 1));
            }
            _ => i += 1,
        }
    }
    None
}
fn scan_value(source: &[u8], i: usize) -> Option<usize> {
    if source.get(i) == Some(&b'"') {
        return scan_string(source, i).map(|(_, end)| end);
    }
    if matches!(source.get(i), Some(b'{') | Some(b'[')) {
        let open = source[i];
        let close = if open == b'{' { b'}' } else { b']' };
        let mut depth = 0;
        let mut j = i;
        let mut string = false;
        while j < source.len() {
            if string {
                if source[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if source[j] == b'"' {
                    string = false;
                }
                j += 1;
                continue;
            }
            if source[j] == b'"' {
                string = true;
                j += 1;
                continue;
            }
            if source[j] == open {
                depth += 1;
            }
            if source[j] == close {
                depth -= 1;
                if depth == 0 {
                    return Some(j + 1);
                }
            }
            j += 1;
        }
        None
    } else {
        let mut j = i;
        while j < source.len() && !matches!(source[j], b',' | b'}' | b']') {
            j += 1;
        }
        Some(j)
    }
}
fn insertion_for_object(source: &[u8], start: usize, end: usize, _member: &str) -> usize {
    let mut i = end.saturating_sub(1);
    while i > start && source[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    i
}
fn insert_bytes(source: &[u8], start: usize, end: usize, member: &str) -> Vec<u8> {
    let has = !object_members(source, start, end).is_empty();
    let mut before = end.saturating_sub(1);
    while before > start && source[before - 1].is_ascii_whitespace() {
        before -= 1;
    }
    let trailing_comma = before > start && source[before - 1] == b',';
    let prefix = if has && !trailing_comma { ", " } else { "" };
    format!("{prefix}{member}").into_bytes()
}
fn member_removal_span(
    source: &[u8],
    start: usize,
    end: usize,
    member: &JsonMember,
) -> (usize, usize) {
    let mut after = member.value_end;
    while after < end && source[after].is_ascii_whitespace() {
        after += 1;
    }
    if source.get(after) == Some(&b',') {
        (member.key_start, after + 1)
    } else {
        let mut before = member.key_start;
        while before > start && source[before - 1].is_ascii_whitespace() {
            before -= 1;
        }
        if before > start && source[before - 1] == b',' {
            (before - 1, member.value_end)
        } else {
            (member.key_start, member.value_end)
        }
    }
}
fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
fn json_fingerprint(value: &Value) -> skilltap_core::domain::Fingerprint {
    fingerprint_contents(&json_fingerprint_bytes(value))
}
fn json_fingerprint_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}
fn is_required_mcp(
    declarations: &[skilltap_core::plugin_graph::ComponentDeclaration],
    name: &str,
) -> bool {
    declarations.iter().any(|d| {
        d.kind == ComponentKind::McpServer
            && d.declared_name.as_deref() == Some(name)
            && d.requiredness == ComponentRequiredness::Required
    })
}
fn component(
    id: &NativeId,
) -> Result<
    skilltap_core::domain::ComponentId,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    skilltap_core::domain::ComponentId::new(format!("mcp:{}", id.as_str()))
        .map_err(|_| mcp_invalid("The Kilo MCP component id is invalid."))
}
fn destination_error() -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::Other {
        code: "kilo_managed_destination_invalid",
        summary: "The Kilo managed destination is invalid.",
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
    fn kilo_codec_keeps_comments_and_rejects_unknown_schema() {
        let source=b"{\n  // keep\n  \"mcp\": {\n    \"old\": {\"type\": \"local\"},\n  },\n  \"username\": \"nested-user\"\n}\n";
        let doc = KiloJsoncDocument::parse(Some(source)).unwrap();
        assert!(doc.value["username"] == "nested-user");
        assert!(KiloJsoncDocument::parse(Some(br#"{"unknownGlobal":true}"#)).is_err());
    }

    #[test]
    fn kilo_codec_admits_only_contract_grounded_top_level_keys() {
        for source in [br#"{"mcp":{}}"#.as_slice(), br#"{"username":"user"}"#] {
            assert!(KiloJsoncDocument::parse(Some(source)).is_ok());
        }
        for key in ["$schema", "theme", "model", "provider", "plugins"] {
            let source = format!(r#"{{"{key}":true}}"#);
            assert!(
                KiloJsoncDocument::parse(Some(source.as_bytes())).is_err(),
                "unattested Kilo key {key} must fail closed"
            );
        }
    }
    #[test]
    fn kilo_jsonc_targeted_patch_preserves_unrelated_bytes_and_removes_owned_entries() {
        let source = b"{\n  // keep this comment\n  \"mcp\": {\n    \"old\": {\"type\": \"local\"},\n  },\n  \"username\": \"nested-user\"\n}\n";
        let mut document = KiloJsoncDocument::parse(Some(source)).unwrap();
        let id = NativeId::new("new").unwrap();
        assert!(
            document
                .upsert(
                    &id,
                    serde_json::json!({"type":"remote","url":"https://example.invalid"})
                )
                .unwrap()
        );
        let encoded = document.clone().encode().unwrap();
        let encoded = String::from_utf8(encoded).unwrap();
        assert!(encoded.contains("keep this comment"));
        assert!(encoded.contains("\"old\""));
        assert!(encoded.contains("\"new\""));
        assert!(encoded.contains("\"username\": \"nested-user\""));
        assert!(document.servers().unwrap().contains_key(&id));
        assert!(document.remove(&id).unwrap());
        assert!(!document.servers().unwrap().contains_key(&id));
    }

    #[test]
    fn kilo_never_exposes_side_effectful_effective_probe_or_lifecycle() {
        let adapter = KiloAdapter;
        assert!(adapter.effective_state_probe().is_none());
        assert!(adapter.native_lifecycle().is_none());
        assert_eq!(
            adapter.decode_version(b"7.4.7\n").unwrap().as_str(),
            "7.4.7"
        );
        let profile = adapter.select_profile(&NativeVersion::new("7.4.7").unwrap());
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
    fn kilo_rejects_oauth_and_non_http_remote_transport() {
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
        assert!(map_kilo_server(&oauth).is_err());
        assert!(map_kilo_server(&sse).is_err());
    }
}
