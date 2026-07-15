use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileId, CapabilityProfileSelection,
        CapabilityScope, CapabilitySet, CapabilitySupport, ConditionalComponentReport,
        ConditionalProfileError, HarnessId, NativeId, NativeVersion, ObservationField,
        ObservationFields, ObservationFinding, ObservationFindingCode, ObservationSeverity,
        ObservationSubject, ObservationSummary, Ownership, ProfileComponentActivation,
        ProfileComponentCompatibility, ProfileComponentObservation, ProfileComponentPresence,
        ProfileComponentRole, ProfileComponentSet, Scope, ScopedCapabilitySets,
    },
    runtime::{ConfinedFileSystem, JsonLimits, StrictJson, StrictJsonDecoder},
};

use crate::conditional_profile::{ConditionalProfileContext, ConditionalProfilePort};

use super::pi_settings::{
    HookDeclaration, PackageDeclaration, PiSettings, SettingsReadState, package_manifest_path,
    package_root,
};

const CORE_VERSION: &str = "0.80.6";
const MCP_PACKAGE: &str = "pi-mcp-adapter";
const MCP_VERSION: &str = "2.11.0";
const MCP_ENTRYPOINT: &str = "./index.ts";
const HOOK_PACKAGE: &str = "@hsingjui/pi-hooks";
const HOOK_VERSION: &str = "0.0.2";
const HOOK_ENTRYPOINT: &str = "./src/pi-hooks.ts";
const CORE_PROFILE_ID: &str = "pi-0-80-6";
const COMPOUND_PROFILE_ID: &str = "pi-0-80-6-mcp-2-11-0-hooks-0-0-2";

pub struct PiConditionalProfile;

impl PiConditionalProfile {
    pub fn static_ref() -> &'static dyn ConditionalProfilePort {
        static PROFILE: PiConditionalProfile = PiConditionalProfile;
        &PROFILE
    }
}

impl ConditionalProfilePort for PiConditionalProfile {
    fn inspect_components(
        &self,
        context: &ConditionalProfileContext<'_>,
    ) -> Result<ConditionalComponentReport, ConditionalProfileError> {
        let settings = super::pi_settings::read(context);
        let target = ObservationSubject::Harness {
            harness: HarnessId::new("pi").expect("Pi harness id is valid"),
            scope: context.scope.clone(),
        };
        let mut findings = Vec::new();
        let mcp = observe_component(
            context,
            &settings,
            &target,
            ComponentSpec {
                package: MCP_PACKAGE,
                role: ProfileComponentRole::McpCompanion,
                expected_version: MCP_VERSION,
                expected_entrypoint: MCP_ENTRYPOINT,
            },
            &mut findings,
        );
        let hooks = observe_component(
            context,
            &settings,
            &target,
            ComponentSpec {
                package: HOOK_PACKAGE,
                role: ProfileComponentRole::HookCompanion,
                expected_version: HOOK_VERSION,
                expected_entrypoint: HOOK_ENTRYPOINT,
            },
            &mut findings,
        );
        findings.push(ObservationFinding::new(
            ObservationFindingCode::CompoundProfileUnavailable,
            ObservationSummary::CompoundProfileUnavailable,
            ObservationSeverity::Blocking,
            target,
            ObservationFields::default(),
        ));
        ConditionalComponentReport::from_components([mcp, hooks], base_capabilities(), findings)
    }

    fn select_compiled_profile(
        &self,
        runtime_version: &NativeVersion,
        components: &ProfileComponentSet,
    ) -> CapabilityProfileSelection {
        if runtime_version.as_str() == CORE_VERSION && exact_components(components) {
            CapabilityProfileSelection::verified_observe_only(
                CapabilityProfileId::new(COMPOUND_PROFILE_ID)
                    .expect("compiled Pi profile id is valid"),
                base_capabilities(),
            )
        } else {
            CapabilityProfileSelection::unknown_version(base_capabilities())
        }
    }
}

pub(super) fn select_core_profile(version: &NativeVersion) -> CapabilityProfileSelection {
    if version.as_str() == CORE_VERSION {
        CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new(CORE_PROFILE_ID).expect("compiled Pi profile id is valid"),
            base_capabilities(),
        )
    } else {
        CapabilityProfileSelection::unknown_version(base_capabilities())
    }
}

fn exact_components(components: &ProfileComponentSet) -> bool {
    if components.len() != 2 {
        return false;
    }
    let Some(mcp) = components.get(&native(MCP_PACKAGE)) else {
        return false;
    };
    let Some(hooks) = components.get(&native(HOOK_PACKAGE)) else {
        return false;
    };
    mcp.role == ProfileComponentRole::McpCompanion
        && mcp.package == native(MCP_PACKAGE)
        && mcp.declared_scope.is_some()
        && mcp.presence == ProfileComponentPresence::Present
        && mcp
            .version
            .as_ref()
            .is_some_and(|version| version.as_str() == MCP_VERSION)
        && mcp.compatibility == ProfileComponentCompatibility::Compatible
        && hooks.role == ProfileComponentRole::HookCompanion
        && hooks.package == native(HOOK_PACKAGE)
        && hooks.declared_scope.is_some()
        && hooks.presence == ProfileComponentPresence::Present
        && hooks
            .version
            .as_ref()
            .is_some_and(|version| version.as_str() == HOOK_VERSION)
        && hooks.compatibility == ProfileComponentCompatibility::Partial
}

fn base_capabilities() -> ScopedCapabilitySets {
    let capabilities = [
        ("harness.observe", CapabilitySupport::Supported),
        ("skill.observe", CapabilitySupport::Supported),
        ("skill.install", CapabilitySupport::Unsupported),
        ("skill.remove", CapabilitySupport::Unsupported),
        ("skill.update", CapabilitySupport::Unsupported),
        ("plugin.install", CapabilitySupport::Unsupported),
        ("plugin.remove", CapabilitySupport::Unsupported),
        ("plugin.update", CapabilitySupport::Unsupported),
        ("marketplace.register", CapabilitySupport::Unsupported),
        ("marketplace.remove", CapabilitySupport::Unsupported),
        ("marketplace.update", CapabilitySupport::Unsupported),
        ("managed.projection", CapabilitySupport::Unsupported),
        ("component.skill", CapabilitySupport::Supported),
        ("component.mcp", CapabilitySupport::Unverified),
        ("component.hook", CapabilitySupport::Unsupported),
    ]
    .into_iter()
    .map(|(id, support)| {
        (
            CapabilityId::new(id).expect("compiled Pi capability id is valid"),
            support,
        )
    })
    .collect::<Vec<_>>();
    let set = CapabilitySet::new(capabilities);
    ScopedCapabilitySets::new(set.clone(), set)
}

#[derive(Clone, Copy)]
struct ComponentSpec {
    package: &'static str,
    role: ProfileComponentRole,
    expected_version: &'static str,
    expected_entrypoint: &'static str,
}

fn observe_component(
    context: &ConditionalProfileContext<'_>,
    settings: &PiSettings,
    target: &ObservationSubject,
    spec: ComponentSpec,
    findings: &mut Vec<ObservationFinding>,
) -> ProfileComponentObservation {
    let (declaration, declared_scope) = effective_package(settings, context.scope, spec.package);
    add_settings_findings(settings, target, spec.package, findings);

    let roots = package_roots(context, declared_scope);
    let mut manifest = ManifestRead::Missing;
    for root in roots {
        manifest = read_manifest(
            context.filesystem,
            &root,
            package_manifest_path(spec.package),
            spec.package,
            spec.expected_entrypoint,
            context.json_limits,
            context.maximum_manifest_bytes,
        );
        if !matches!(manifest, ManifestRead::Missing) {
            break;
        }
    }
    let declared = declaration.is_some();
    let (presence, version, identity_matches, entrypoint_matches) = match manifest {
        ManifestRead::Missing => {
            findings.push(component_finding(
                target,
                ObservationFindingCode::ProfileComponentMissing,
                ObservationSeverity::Blocking,
                spec.package,
            ));
            (ProfileComponentPresence::Missing, None, false, false)
        }
        ManifestRead::Unreadable => {
            findings.push(component_finding(
                target,
                ObservationFindingCode::NativeStateUnreadable,
                ObservationSeverity::Error,
                spec.package,
            ));
            (ProfileComponentPresence::Present, None, false, false)
        }
        ManifestRead::Malformed => {
            findings.push(component_finding(
                target,
                ObservationFindingCode::NativeEntryMalformed,
                ObservationSeverity::Error,
                spec.package,
            ));
            (ProfileComponentPresence::Present, None, false, false)
        }
        ManifestRead::Present {
            version,
            identity_matches,
            entrypoint_matches,
        } => (
            ProfileComponentPresence::Present,
            version,
            identity_matches,
            entrypoint_matches,
        ),
    };

    let exact_version = version
        .as_ref()
        .is_some_and(|version| version.as_str() == spec.expected_version);
    if presence == ProfileComponentPresence::Present && !exact_version {
        findings.push(component_finding(
            target,
            ObservationFindingCode::ProfileComponentVersionUnverified,
            ObservationSeverity::Warning,
            spec.package,
        ));
    }
    if presence == ProfileComponentPresence::Present && (!identity_matches || !entrypoint_matches) {
        findings.push(component_finding(
            target,
            ObservationFindingCode::ProfileComponentIncompatible,
            ObservationSeverity::Error,
            spec.package,
        ));
    }

    let compatibility = if presence != ProfileComponentPresence::Present {
        ProfileComponentCompatibility::Unverified
    } else if !identity_matches || !entrypoint_matches {
        ProfileComponentCompatibility::Incompatible
    } else if !exact_version {
        ProfileComponentCompatibility::Unverified
    } else if spec.role == ProfileComponentRole::HookCompanion {
        findings.push(component_finding(
            target,
            ObservationFindingCode::ProfileComponentIncompatible,
            ObservationSeverity::Warning,
            spec.package,
        ));
        ProfileComponentCompatibility::Partial
    } else {
        ProfileComponentCompatibility::Compatible
    };

    let project_declared = matches!(declared_scope, Some(CapabilityScope::Project));
    let activation = if !declared {
        if presence == ProfileComponentPresence::Present {
            findings.push(component_finding(
                target,
                ObservationFindingCode::ProfileComponentInactive,
                ObservationSeverity::Warning,
                spec.package,
            ));
            ProfileComponentActivation::Inert
        } else {
            ProfileComponentActivation::Unverified
        }
    } else {
        match spec.role {
            ProfileComponentRole::McpCompanion => {
                if presence != ProfileComponentPresence::Present
                    || !identity_matches
                    || !entrypoint_matches
                {
                    ProfileComponentActivation::Unverified
                } else if project_declared {
                    ProfileComponentActivation::TrustRequired
                } else {
                    ProfileComponentActivation::Unverified
                }
            }
            ProfileComponentRole::HookCompanion => {
                let (hooks, project_hooks) = effective_hooks(settings, context.scope);
                let configured = hooks.is_configured();
                if presence != ProfileComponentPresence::Present
                    || !identity_matches
                    || !entrypoint_matches
                {
                    ProfileComponentActivation::Unverified
                } else if project_hooks || project_declared {
                    ProfileComponentActivation::TrustRequired
                } else if configured {
                    ProfileComponentActivation::ConfiguredUnverified
                } else {
                    findings.push(component_finding(
                        target,
                        ObservationFindingCode::ProfileComponentInactive,
                        ObservationSeverity::Warning,
                        spec.package,
                    ));
                    ProfileComponentActivation::Inert
                }
            }
        }
    };

    ProfileComponentObservation::new(
        native(spec.package),
        native(spec.package),
        spec.role,
        true,
        declared_scope,
        presence,
        version,
        activation,
        compatibility,
        Ownership::Harness,
    )
}

fn effective_package(
    settings: &PiSettings,
    scope: &Scope,
    package: &str,
) -> (Option<PackageDeclaration>, Option<CapabilityScope>) {
    let global = package_declaration(settings.global, package);
    match scope {
        Scope::Global => (global, global.map(|_| CapabilityScope::Global)),
        Scope::Project(_) => {
            let Some(project) = settings.project else {
                return (global, global.map(|_| CapabilityScope::Global));
            };
            if matches!(project.state, SettingsReadState::Present)
                && package_declaration(project, package).is_some()
            {
                (
                    package_declaration(project, package),
                    Some(CapabilityScope::Project),
                )
            } else if matches!(
                project.state,
                SettingsReadState::Malformed | SettingsReadState::Unreadable
            ) {
                (None, None)
            } else {
                (global, global.map(|_| CapabilityScope::Global))
            }
        }
    }
}

fn package_roots(
    context: &ConditionalProfileContext<'_>,
    declared_scope: Option<CapabilityScope>,
) -> Vec<AbsolutePath> {
    match declared_scope {
        Some(CapabilityScope::Global) => package_root(context.paths, context.scope, true)
            .into_iter()
            .collect(),
        Some(CapabilityScope::Project) => package_root(context.paths, context.scope, false)
            .into_iter()
            .collect(),
        None => match context.scope {
            Scope::Global => package_root(context.paths, context.scope, true)
                .into_iter()
                .collect(),
            Scope::Project(_) => package_root(context.paths, context.scope, false)
                .into_iter()
                .chain(package_root(context.paths, context.scope, true))
                .collect(),
        },
    }
}

fn package_declaration(
    snapshot: super::pi_settings::SettingsSnapshot,
    package: &str,
) -> Option<PackageDeclaration> {
    match package {
        MCP_PACKAGE => match snapshot.mcp_package {
            PackageDeclaration::Declared { autoload } => {
                Some(PackageDeclaration::Declared { autoload })
            }
            PackageDeclaration::Absent => None,
        },
        HOOK_PACKAGE => match snapshot.hook_package {
            PackageDeclaration::Declared { autoload } => {
                Some(PackageDeclaration::Declared { autoload })
            }
            PackageDeclaration::Absent => None,
        },
        _ => None,
    }
}

fn effective_hooks(settings: &PiSettings, scope: &Scope) -> (HookDeclaration, bool) {
    match scope {
        Scope::Global => (settings.global.hooks, false),
        Scope::Project(_) => {
            let project = settings
                .project
                .map(|settings| settings.hooks)
                .unwrap_or(HookDeclaration::Absent);
            (
                if settings.global.hooks.is_configured() || project.is_configured() {
                    HookDeclaration::Configured
                } else {
                    HookDeclaration::Absent
                },
                project.is_configured(),
            )
        }
    }
}

fn add_settings_findings(
    settings: &PiSettings,
    target: &ObservationSubject,
    package: &str,
    findings: &mut Vec<ObservationFinding>,
) {
    for state in std::iter::once(settings.global.state)
        .chain(settings.project.into_iter().map(|settings| settings.state))
    {
        let (code, severity) = match state {
            SettingsReadState::Malformed => (
                ObservationFindingCode::NativeShapeUnsupported,
                ObservationSeverity::Error,
            ),
            SettingsReadState::Unreadable => (
                ObservationFindingCode::NativeStateUnreadable,
                ObservationSeverity::Error,
            ),
            SettingsReadState::Missing | SettingsReadState::Present => continue,
        };
        findings.push(component_finding(target, code, severity, package));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ManifestRead {
    Missing,
    Unreadable,
    Malformed,
    Present {
        version: Option<NativeVersion>,
        identity_matches: bool,
        entrypoint_matches: bool,
    },
}

fn read_manifest(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: skilltap_core::domain::RelativeArtifactPath,
    expected_package: &str,
    expected_entrypoint: &str,
    limits: JsonLimits,
    maximum_bytes: u64,
) -> ManifestRead {
    let maximum_bytes = maximum_bytes.min(limits.bytes());
    let bytes = match filesystem.read_regular_bounded_no_follow(root, &destination, maximum_bytes) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return ManifestRead::Missing,
        Err(_) if std::fs::symlink_metadata(root.as_str()).is_err() => {
            return ManifestRead::Missing;
        }
        Err(_) => return ManifestRead::Unreadable,
    };
    let value = match StrictJson.decode(&bytes, limits) {
        Ok(decoded) => decoded.into_value(),
        Err(_) => return ManifestRead::Malformed,
    };
    let Some(object) = value.as_object() else {
        return ManifestRead::Malformed;
    };
    let Some(name) = object.get("name").and_then(serde_json::Value::as_str) else {
        return ManifestRead::Malformed;
    };
    let Some(version) = object.get("version").and_then(serde_json::Value::as_str) else {
        return ManifestRead::Malformed;
    };
    let Some(pi) = object.get("pi").and_then(serde_json::Value::as_object) else {
        return ManifestRead::Malformed;
    };
    let Some(extensions) = pi.get("extensions").and_then(serde_json::Value::as_array) else {
        return ManifestRead::Malformed;
    };
    let identity_matches = name == expected_package;
    let entrypoint_matches = extensions.len() == 1
        && extensions[0]
            .as_str()
            .is_some_and(|entrypoint| entrypoint == expected_entrypoint);
    ManifestRead::Present {
        version: NativeVersion::new(version).ok(),
        identity_matches,
        entrypoint_matches,
    }
}

fn component_finding(
    target: &ObservationSubject,
    code: ObservationFindingCode,
    severity: ObservationSeverity,
    component: &str,
) -> ObservationFinding {
    ObservationFinding::new(
        code,
        summary_for(code),
        severity,
        target.clone(),
        ObservationFields::new([ObservationField::ProfileComponent(native(component))])
            .expect("static Pi finding field is unique"),
    )
}

fn summary_for(code: ObservationFindingCode) -> ObservationSummary {
    match code {
        ObservationFindingCode::NativeStateUnreadable => ObservationSummary::NativeStateUnreadable,
        ObservationFindingCode::NativeShapeUnsupported => {
            ObservationSummary::NativeShapeUnsupported
        }
        ObservationFindingCode::NativeEntryMalformed => ObservationSummary::MalformedNativeEntry,
        ObservationFindingCode::ProfileComponentMissing => {
            ObservationSummary::ProfileComponentMissing
        }
        ObservationFindingCode::ProfileComponentVersionUnverified => {
            ObservationSummary::ProfileComponentVersionUnverified
        }
        ObservationFindingCode::ProfileComponentInactive => {
            ObservationSummary::ProfileComponentInactive
        }
        ObservationFindingCode::ProfileComponentIncompatible => {
            ObservationSummary::ProfileComponentIncompatible
        }
        _ => unreachable!("Pi adapter uses only registered component findings"),
    }
}

fn native(value: &str) -> NativeId {
    NativeId::new(value).expect("static Pi native id is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::{
        domain::{CapabilityScope, ObservationFieldCode, Scope},
        runtime::{
            Environment, EnvironmentVariable, PlatformPaths, SupportedPlatform, SystemFileSystem,
        },
    };
    use skilltap_test_support::TempRoot;
    use std::{collections::BTreeMap, ffi::OsString, fs};

    #[derive(Default)]
    struct TestEnvironment(BTreeMap<&'static str, OsString>);

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            self.0.get(variable.as_str()).cloned()
        }
    }

    fn context<'a>(scope: &'a Scope, paths: &'a PlatformPaths) -> ConditionalProfileContext<'a> {
        ConditionalProfileContext {
            scope,
            paths,
            filesystem: &SystemFileSystem,
            json_limits: JsonLimits::new(64 * 1024, 32).unwrap(),
            maximum_manifest_bytes: 64 * 1024,
        }
    }

    fn paths(root: &TempRoot) -> PlatformPaths {
        let mut environment = TestEnvironment::default();
        environment.0.insert(
            EnvironmentVariable::Home.as_str(),
            OsString::from(root.join("home").to_string_lossy().as_ref()),
        );
        PlatformPaths::resolve_for(SupportedPlatform::Linux, &environment).unwrap()
    }

    fn write_package(root: &std::path::Path, package: &str, version: &str, entrypoint: &str) {
        let package_root = root.join(format!("node_modules/{package}"));
        fs::create_dir_all(&package_root).unwrap();
        fs::write(
            package_root.join("package.json"),
            format!(
                r#"{{"name":"{package}","version":"{version}","pi":{{"extensions":["{entrypoint}"]}}}}"#
            ),
        )
        .unwrap();
    }

    fn setup_global(root: &TempRoot, hooks: &str) -> PlatformPaths {
        let paths = paths(root);
        fs::create_dir_all(paths.pi_home().as_str()).unwrap();
        fs::create_dir_all(paths.pi_package_dir().as_str()).unwrap();
        fs::write(
            std::path::Path::new(paths.pi_home().as_str()).join("settings.json"),
            format!(r#"{{"packages":["npm:{MCP_PACKAGE}","npm:{HOOK_PACKAGE}"],"hooks":{hooks}}}"#),
        )
        .unwrap();
        write_package(
            std::path::Path::new(paths.pi_package_dir().as_str()),
            MCP_PACKAGE,
            MCP_VERSION,
            MCP_ENTRYPOINT,
        );
        write_package(
            std::path::Path::new(paths.pi_package_dir().as_str()),
            HOOK_PACKAGE,
            HOOK_VERSION,
            HOOK_ENTRYPOINT,
        );
        paths
    }

    #[test]
    fn exact_compound_tuple_is_known_but_mutation_unsupported() {
        let root = TempRoot::new("pi-profile-exact").unwrap();
        let paths = setup_global(&root, "{}");
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let profile = PiConditionalProfile.select_compiled_profile(
            &NativeVersion::new(CORE_VERSION).unwrap(),
            report.components(),
        );
        assert_eq!(profile.profile_id().unwrap().as_str(), COMPOUND_PROFILE_ID);
        assert!(profile.mutation_capabilities().is_none());
        assert_eq!(report.components().len(), 2);
        assert_eq!(
            report
                .components()
                .get(&native(MCP_PACKAGE))
                .unwrap()
                .compatibility,
            ProfileComponentCompatibility::Compatible
        );
        assert_eq!(
            report
                .components()
                .get(&native(HOOK_PACKAGE))
                .unwrap()
                .compatibility,
            ProfileComponentCompatibility::Partial
        );
    }

    #[test]
    fn hook_configuration_changes_activation_not_partial_compatibility() {
        let root = TempRoot::new("pi-profile-hooks").unwrap();
        let paths = setup_global(&root, r#"{"PreToolUse":[{}]}"#);
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let hook = report.components().get(&native(HOOK_PACKAGE)).unwrap();
        assert_eq!(
            hook.activation,
            ProfileComponentActivation::ConfiguredUnverified
        );
        assert_eq!(hook.compatibility, ProfileComponentCompatibility::Partial);
    }

    #[test]
    fn global_then_project_hooks_configure_independently_of_package_precedence() {
        let root = TempRoot::new("pi-profile-precedence").unwrap();
        let paths = setup_global(&root, r#"{"PreToolUse":[{}]}"#);
        let project_path = root.join("project");
        fs::create_dir_all(project_path.join(".pi/npm")).unwrap();
        fs::write(
            project_path.join(".pi/settings.json"),
            format!(
                r#"{{"packages":[{{"source":"npm:{HOOK_PACKAGE}"}}],"hooks":{{"Stop":[{{}}]}}}}"#
            ),
        )
        .unwrap();
        write_package(
            &project_path.join(".pi/npm"),
            HOOK_PACKAGE,
            HOOK_VERSION,
            HOOK_ENTRYPOINT,
        );
        let project = AbsolutePath::new(project_path.to_str().unwrap()).unwrap();
        let scope = Scope::Project(project);
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let hook = report.components().get(&native(HOOK_PACKAGE)).unwrap();
        assert_eq!(hook.declared_scope, Some(CapabilityScope::Project));
        assert_eq!(hook.activation, ProfileComponentActivation::TrustRequired);
        assert!(report.findings().iter().any(|finding| {
            finding
                .fields()
                .get(ObservationFieldCode::ProfileComponent)
                .is_some()
        }));
    }

    #[test]
    fn package_directory_without_settings_declaration_is_present_but_inert() {
        let root = TempRoot::new("pi-profile-inert-package").unwrap();
        let paths = setup_global(&root, "{}");
        fs::write(
            std::path::Path::new(paths.pi_home().as_str()).join("settings.json"),
            br#"{}"#,
        )
        .unwrap();
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let mcp = report.components().get(&native(MCP_PACKAGE)).unwrap();
        assert_eq!(mcp.presence, ProfileComponentPresence::Present);
        assert_eq!(mcp.declared_scope, None);
        assert_eq!(mcp.activation, ProfileComponentActivation::Inert);
        let profile = PiConditionalProfile.select_compiled_profile(
            &NativeVersion::new(CORE_VERSION).unwrap(),
            report.components(),
        );
        assert!(profile.profile_id().is_none());
        assert!(
            report.findings().iter().any(|finding| {
                finding.code() == ObservationFindingCode::ProfileComponentInactive
            })
        );
    }

    #[test]
    fn missing_one_companion_does_not_hide_the_other() {
        let root = TempRoot::new("pi-profile-missing").unwrap();
        let paths = setup_global(&root, "{}");
        fs::remove_dir_all(
            std::path::Path::new(paths.pi_package_dir().as_str())
                .join("node_modules")
                .join(HOOK_PACKAGE),
        )
        .unwrap();
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        assert_eq!(report.components().len(), 2);
        assert_eq!(
            report
                .components()
                .get(&native(MCP_PACKAGE))
                .unwrap()
                .presence,
            ProfileComponentPresence::Present
        );
        assert_eq!(
            report
                .components()
                .get(&native(HOOK_PACKAGE))
                .unwrap()
                .presence,
            ProfileComponentPresence::Missing
        );
        let profile = PiConditionalProfile.select_compiled_profile(
            &NativeVersion::new(CORE_VERSION).unwrap(),
            report.components(),
        );
        assert!(profile.profile_id().is_none());
        assert!(profile.mutation_capabilities().is_none());
    }

    #[test]
    fn mismatched_manifest_and_unknown_version_remain_unverified() {
        let root = TempRoot::new("pi-profile-mismatch").unwrap();
        let paths = setup_global(&root, "{}");
        write_package(
            std::path::Path::new(paths.pi_package_dir().as_str()),
            MCP_PACKAGE,
            "2.12.0",
            "./wrong.ts",
        );
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let mcp = report.components().get(&native(MCP_PACKAGE)).unwrap();
        assert_eq!(mcp.presence, ProfileComponentPresence::Present);
        assert_eq!(
            mcp.compatibility,
            ProfileComponentCompatibility::Incompatible
        );
        assert!(
            mcp.version
                .as_ref()
                .is_some_and(|version| version.as_str() == "2.12.0")
        );
        assert!(report.findings().iter().any(|finding| {
            finding.code() == ObservationFindingCode::ProfileComponentVersionUnverified
        }));
    }

    #[cfg(unix)]
    #[test]
    fn package_manifest_symlinks_are_unreadable_without_following_the_target() {
        use std::os::unix::fs::symlink;

        let root = TempRoot::new("pi-profile-no-follow").unwrap();
        let paths = setup_global(&root, "{}");
        let outside = root.join("outside-package.json");
        fs::write(&outside, br#"{"name":"pi-mcp-adapter","version":"2.11.0","pi":{"extensions":["./index.ts"]},"secret":"raw"}"#).unwrap();
        let manifest = std::path::Path::new(paths.pi_package_dir().as_str())
            .join("node_modules")
            .join(MCP_PACKAGE)
            .join("package.json");
        fs::remove_file(&manifest).unwrap();
        symlink(&outside, &manifest).unwrap();

        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        let mcp = report.components().get(&native(MCP_PACKAGE)).unwrap();
        assert_eq!(mcp.presence, ProfileComponentPresence::Present);
        assert!(
            report
                .findings()
                .iter()
                .any(|finding| { finding.code() == ObservationFindingCode::NativeStateUnreadable })
        );
        assert!(!format!("{report:?}").contains("raw"));
    }

    #[test]
    fn profile_component_findings_carry_no_settings_or_manifest_payload() {
        let root = TempRoot::new("pi-profile-findings").unwrap();
        let paths = setup_global(&root, "{}");
        let secret = "sk-test-secret";
        fs::write(
            std::path::Path::new(paths.pi_home().as_str()).join("settings.json"),
            format!(r#"{{"packages":["npm:{secret}"]}}"#),
        )
        .unwrap();
        let scope = Scope::Global;
        let report = PiConditionalProfile
            .inspect_components(&context(&scope, &paths))
            .unwrap();
        for finding in report.findings() {
            assert!(!format!("{finding:?}").contains(secret));
        }
    }
}
