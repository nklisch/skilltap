use std::{collections::BTreeSet, ffi::OsString, fs};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityScope, ComponentId, HarnessId, NativeId, ResourceId,
        ResourceKey, ResourceKind, Scope, Source, SourceKind, SourceLocator,
    },
    managed_projection::ResolvedSourceCheckout,
    mutation_authority::{
        CapabilityRequirement, ManagedSurfaceKind, MutationAuthorityRequest, MutationChannel,
        authorize_mutation,
    },
    runtime::{
        Environment, EnvironmentVariable, JsonLimits, PlatformPaths, SupportedPlatform,
        SystemFileSystem,
    },
};
use skilltap_harnesses::{
    CopilotAdapter, CopilotManagedProjection, CopilotPolicyHealth, ManagedLifecycleKind,
    ManagedProjectionContext, ManagedProjectionInput, NativeLifecycleAction,
    NativeLifecycleRequest,
};
use skilltap_test_support::TempRoot;

struct TestEnvironment(OsString);

impl Environment for TestEnvironment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
        (variable == EnvironmentVariable::Home).then(|| self.0.clone())
    }
}

fn limits() -> JsonLimits {
    JsonLimits::new(64 * 1024, 32).unwrap()
}

fn source(root: &TempRoot) -> Source {
    Source::new(
        SourceKind::Local,
        SourceLocator::new(root.path().to_string_lossy()).unwrap(),
        None,
    )
    .unwrap()
}

fn write_catalog(root: &TempRoot) {
    fs::create_dir_all(root.join(".claude-plugin")).unwrap();
    fs::write(
        root.join(".claude-plugin/marketplace.json"),
        br#"{"name":"team","plugins":[{"name":"demo","source":"./plugins/demo"}]}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("plugins/demo/.claude-plugin")).unwrap();
    fs::write(
        root.join("plugins/demo/.claude-plugin/plugin.json"),
        br#"{"name":"demo"}"#,
    )
    .unwrap();
}

fn context<'a>(
    paths: &'a PlatformPaths,
    checkout: &'a ResolvedSourceCheckout,
    scope: &'a Scope,
    resource_key: &'a ResourceKey,
    request: &'a NativeLifecycleRequest,
    target: &'a HarnessId,
) -> ManagedProjectionContext<'a> {
    ManagedProjectionContext {
        target,
        scope,
        paths,
        resource_key,
        resource_kind: ResourceKind::Plugin,
        request,
        kind: ManagedLifecycleKind::PluginInstall,
        input: ManagedProjectionInput::Apply { checkout },
        prior: &[],
        filesystem: &SystemFileSystem,
        json_limits: limits(),
    }
}

#[test]
fn copilot_mcp_only_projection_is_supported_and_skill_projection_is_acknowledged() {
    let root = TempRoot::new("skilltap-copilot-mixed").unwrap();
    write_catalog(&root);
    fs::write(
        root.join("plugins/demo/.mcp.json"),
        br#"{"mcpServers":{"docs":{"type":"stdio","command":"node","env":{"TOKEN":"${MCP_TOKEN}"},"future":{"keep":true}}}}"#,
    )
    .unwrap();
    let environment = TestEnvironment(OsString::from(root.path()));
    let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    let source = source(&root);
    let checkout = ResolvedSourceCheckout::new(
        AbsolutePath::new(root.path().to_string_lossy()).unwrap(),
        source.clone(),
        None,
    );
    let scope = Scope::Global;
    let target = HarnessId::new("copilot").unwrap();
    let key = ResourceKey::new(ResourceId::new("plugin:demo@team").unwrap(), scope.clone());
    let request = NativeLifecycleRequest {
        action: NativeLifecycleAction::PluginInstall,
        scope: scope.clone(),
        name: NativeId::new("demo@team").unwrap(),
        source: None,
    };
    let plan = CopilotManagedProjection::static_ref()
        .plan(&context(&paths, &checkout, &scope, &key, &request, &target))
        .unwrap();
    assert!(plan.trees.is_empty());
    assert_eq!(plan.files.len(), 1);
    assert!(plan.manifest.iter().any(|projection| matches!(
        projection,
        skilltap_core::storage::ManagedProjection::Mcp { id, .. } if id.as_str() == "docs"
    )));

    let selected = CopilotAdapter::static_ref()
        .select_profile(&skilltap_core::domain::NativeVersion::new("1.0.70").unwrap());
    let mcp_requirement = CapabilityRequirement::new(
        CapabilityId::new("component.mcp").unwrap(),
        [ComponentId::new("mcp:docs").unwrap()],
    );
    let supported = authorize_mutation(MutationAuthorityRequest {
        profile: &selected,
        scope: &scope,
        channel: MutationChannel::ManagedProjection,
        required: &[
            CapabilityRequirement::new(CapabilityId::new("managed.projection").unwrap(), []),
            mcp_requirement,
        ],
        surfaces: &BTreeSet::from([ManagedSurfaceKind::ManagedDocument]),
        declaration: CopilotAdapter::static_ref()
            .managed_declaration_contract(CapabilityScope::Global),
    })
    .unwrap();
    assert_eq!(
        supported,
        skilltap_core::mutation_authority::MutationAuthorization::Supported
    );

    fs::create_dir_all(root.join("plugins/demo/skills/demo")).unwrap();
    fs::write(
        root.join("plugins/demo/skills/demo/SKILL.md"),
        b"---\nname: demo\ndescription: demo\n---\n",
    )
    .unwrap();
    let skill_plan = CopilotManagedProjection::static_ref()
        .plan(&context(&paths, &checkout, &scope, &key, &request, &target))
        .unwrap();
    assert_eq!(skill_plan.files.len(), 1);
    assert_eq!(skill_plan.trees.len(), 1);
    let skill_requirement = CapabilityRequirement::new(
        CapabilityId::new("component.skill").unwrap(),
        [ComponentId::new("skill:demo").unwrap()],
    );
    let declaration = CopilotAdapter::static_ref()
        .managed_declaration_contract(CapabilityScope::Global)
        .unwrap();
    let partial = authorize_mutation(MutationAuthorityRequest {
        profile: &selected,
        scope: &scope,
        channel: MutationChannel::ManagedProjection,
        required: &[
            CapabilityRequirement::new(CapabilityId::new("managed.projection").unwrap(), []),
            skill_requirement,
        ],
        surfaces: &BTreeSet::from([ManagedSurfaceKind::CompleteSkillTree]),
        declaration: Some(declaration),
    })
    .unwrap();
    assert!(partial.is_declaration_managed());
}

#[test]
fn copilot_canonical_skill_roots_are_both_scoped_and_no_linked_copy_is_needed() {
    let adapter = CopilotAdapter::static_ref();
    let home = AbsolutePath::new("/home/user").unwrap();
    let project = AbsolutePath::new("/home/user/project").unwrap();
    let environment = TestEnvironment(home.as_str().into());
    let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap();
    assert_eq!(
        adapter
            .skill_projection()
            .unwrap()
            .destination(&paths, &Scope::Global)
            .unwrap()
            .as_str(),
        "/home/user/.agents/skills"
    );
    assert_eq!(
        adapter
            .skill_projection()
            .unwrap()
            .destination(&paths, &Scope::Project(project.clone()))
            .unwrap()
            .as_str(),
        "/home/user/project/.agents/skills"
    );
    assert!(
        !adapter
            .skill_projection()
            .unwrap()
            .destination(&paths, &Scope::Project(project))
            .unwrap()
            .as_str()
            .contains(".github/skills")
    );
}

#[test]
fn copilot_has_no_native_plugin_invocation_even_when_the_source_overlaps() {
    let adapter = CopilotAdapter::static_ref();
    assert!(adapter.native_lifecycle().is_none());
    assert!(adapter.native_distribution().is_none());
    assert!(matches!(
        CopilotPolicyHealth::EnterpriseBlocked,
        CopilotPolicyHealth::EnterpriseBlocked
    ));
}
