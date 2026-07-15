mod claude;
mod codex;
mod codex_managed;
mod configuration_constrained;
mod copilot;
mod copilot_managed;
mod cursor;
mod factory;
mod factory_managed;
mod file_managed;
mod gemini;
mod gemini_managed;
mod kilo;
mod kimi;
mod kiro;
mod kiro_managed;
mod opencode;
mod opencode_managed;
mod pi;
mod pi_profile;
mod pi_settings;
mod qwen;
mod qwen_managed;
mod trust_interactive;
mod vibe;
mod zcode;
mod zoo;

pub use claude::{ClaudeAdapter, ClaudeInstructionBridge, ClaudeLifecycle, ClaudeSkillProjection};
pub use codex::{CodexAdapter, CodexInstructionBridge, CodexLifecycle, CodexSkillProjection};
pub use codex_managed::CodexManagedProjection;
pub use copilot::{
    CopilotAdapter, CopilotEffectiveMcpObservation, CopilotEffectiveStateProbe,
    CopilotPolicyHealth, CopilotSkillProjection, copilot_policy_finding,
};
pub use copilot_managed::CopilotManagedProjection;
pub use cursor::CursorAdapter;
pub use factory::{
    FactoryAdapter, FactoryLifecycle, FactoryNativeDistribution, FactorySkillProjection,
    decode_factory_plugin_list,
};
pub use factory_managed::FactoryManagedProjection;
pub use gemini::{GeminiAdapter, GeminiEffectiveStateProbe, GeminiSkillProjection};
pub use gemini_managed::GeminiManagedProjection;
pub use kilo::{KiloAdapter, KiloManagedProjection, KiloSkillProjection};
pub use kimi::{KimiAdapter, KimiManagedProjection, KimiSkillProjection};
pub use kiro::{KiroAdapter, KiroSkillProjection};
pub use kiro_managed::KiroManagedProjection;
pub use opencode::{OpenCodeAdapter, OpenCodeEffectiveStateProbe, OpenCodeSkillProjection};
pub use opencode_managed::OpenCodeManagedProjection;
pub use pi::{PiAdapter, PiSkillProjection};
pub use pi_profile::PiConditionalProfile;
pub use qwen::{
    QwenAdapter, QwenEffectiveStateProbe, QwenExtensionRecord, QwenLifecycle,
    QwenNativeDistribution, QwenSkillProjection, decode_qwen_extensions, decode_qwen_mcp_status,
};
pub use qwen_managed::QwenManagedProjection;
pub use trust_interactive::{
    AmpAdapter, AmpDeclaredListError, AmpDeclaredServer, AmpDeclaredSource, AmpManagedProjection,
    AmpSkillProjection, JunieAdapter, JunieManagedProjection, JunieSkillProjection,
    declared_list_arguments, decode_declared_mcp_list,
};
pub use vibe::{VibeAdapter, VibeManagedProjection, VibeSkillProjection};
pub use zcode::{ZCodeAdapter, ZCodeReadOnlyTarget};
pub use zoo::{ZooAdapter, ZooReadOnlyTarget};

#[cfg(test)]
mod tests {
    use skilltap_core::domain::{
        CapabilityId, CapabilityScope, CapabilitySupport, HarnessId, NativeVersion,
    };

    use super::*;
    use crate::TargetRegistry;

    #[test]
    fn canonical_adapter_profiles_preserve_verified_and_unknown_matrices() {
        let cases = [
            (
                CodexAdapter::static_ref(),
                "0.144.1",
                "codex-0-144-1",
                false,
                false,
            ),
            (
                ClaudeAdapter::static_ref(),
                "2.1.201",
                "claude-2-1-201",
                true,
                true,
            ),
            (
                GeminiAdapter::static_ref(),
                "0.50.0",
                "gemini-0-50-0",
                true,
                true,
            ),
            (
                OpenCodeAdapter::static_ref(),
                "1.18.1",
                "opencode-1-18-1",
                true,
                true,
            ),
            (
                QwenAdapter::static_ref(),
                "0.19.10",
                "qwen-0-19-10",
                true,
                true,
            ),
        ];

        for (adapter, version, profile_id, global_update, project_lifecycle) in cases {
            let known = adapter.select_profile(&NativeVersion::new(version).unwrap());
            assert_eq!(known.profile_id().unwrap().as_str(), profile_id);
            let known_sets = known.mutation_capabilities().unwrap();
            assert_support(
                known_sets,
                CapabilityScope::Global,
                "harness.observe",
                CapabilitySupport::Supported,
            );
            assert_support(
                known_sets,
                CapabilityScope::Global,
                "plugin.install",
                CapabilitySupport::Supported,
            );
            assert_support(
                known_sets,
                CapabilityScope::Global,
                "plugin.remove",
                CapabilitySupport::Supported,
            );
            assert_support(
                known_sets,
                CapabilityScope::Global,
                "plugin.update",
                support(global_update),
            );
            for capability in [
                "marketplace.register",
                "marketplace.remove",
                "marketplace.update",
            ] {
                assert_support(
                    known_sets,
                    CapabilityScope::Global,
                    capability,
                    CapabilitySupport::Supported,
                );
            }
            for capability in [
                "plugin.install",
                "plugin.remove",
                "plugin.update",
                "marketplace.register",
                "marketplace.remove",
                "marketplace.update",
            ] {
                assert_support(
                    known_sets,
                    CapabilityScope::Project,
                    capability,
                    support(project_lifecycle),
                );
            }
            assert_support(
                known_sets,
                CapabilityScope::Project,
                "harness.observe",
                CapabilitySupport::Supported,
            );

            let unknown = adapter.select_profile(&NativeVersion::new("99.0.0").unwrap());
            assert!(unknown.profile_id().is_none());
            assert!(unknown.mutation_capabilities().is_none());
            for scope in [CapabilityScope::Global, CapabilityScope::Project] {
                for capability in [
                    "harness.observe",
                    "plugin.install",
                    "plugin.remove",
                    "plugin.update",
                    "marketplace.register",
                    "marketplace.remove",
                    "marketplace.update",
                ] {
                    assert_support(
                        unknown.observation_capabilities(),
                        scope,
                        capability,
                        CapabilitySupport::Unverified,
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_registry_dispatches_the_concrete_singletons() {
        let registry = TargetRegistry::canonical();
        let codex = registry.adapter(&HarnessId::new("codex").unwrap()).unwrap();
        let claude = registry
            .adapter(&HarnessId::new("claude").unwrap())
            .unwrap();
        let droid = registry.adapter(&HarnessId::new("droid").unwrap()).unwrap();
        let copilot = registry
            .adapter(&HarnessId::new("copilot").unwrap())
            .unwrap();
        let gemini = registry
            .adapter(&HarnessId::new("gemini").unwrap())
            .unwrap();
        let opencode = registry
            .adapter(&HarnessId::new("opencode").unwrap())
            .unwrap();
        let kiro = registry.adapter(&HarnessId::new("kiro").unwrap()).unwrap();
        let qwen = registry.adapter(&HarnessId::new("qwen").unwrap()).unwrap();

        let pi = registry.adapter(&HarnessId::new("pi").unwrap()).unwrap();
        assert_eq!(pi.identity(), super::PiAdapter::static_ref().identity());
        assert!(pi.native_lifecycle().is_none());
        assert!(pi.managed_projection().is_none());
        assert!(pi.skill_projection().is_some());
        assert_eq!(codex.identity(), CodexAdapter::static_ref().identity());
        assert_eq!(claude.identity(), ClaudeAdapter::static_ref().identity());
        assert_eq!(gemini.identity(), GeminiAdapter::static_ref().identity());
        assert_eq!(droid.identity(), FactoryAdapter::static_ref().identity());
        assert!(droid.native_lifecycle().is_some());
        assert_eq!(copilot.identity(), CopilotAdapter::static_ref().identity());
        assert!(copilot.native_lifecycle().is_none());
        assert!(copilot.native_distribution().is_none());
        assert!(copilot.managed_projection().is_some());
        assert!(copilot.effective_state_probe().is_some());
        assert!(copilot.skill_projection().is_some());
        assert!(droid.skill_projection().is_some());
        assert!(droid.native_distribution().is_some());
        assert!(droid.managed_projection().is_some());
        assert!(codex.native_lifecycle().is_some());
        assert!(codex.instruction_bridge().is_some());
        assert!(codex.skill_projection().is_some());
        assert!(codex.managed_projection().is_some());
        assert!(claude.managed_projection().is_none());
        assert!(claude.native_lifecycle().is_some());
        assert!(claude.instruction_bridge().is_some());
        assert!(claude.skill_projection().is_some());
        assert!(gemini.native_lifecycle().is_none());
        assert!(gemini.instruction_bridge().is_none());
        assert!(gemini.skill_projection().is_some());
        assert!(gemini.managed_projection().is_some());
        assert!(gemini.effective_state_probe().is_some());
        assert_eq!(
            opencode.identity(),
            OpenCodeAdapter::static_ref().identity()
        );
        assert!(opencode.native_lifecycle().is_none());
        assert!(opencode.skill_projection().is_some());
        assert!(opencode.managed_projection().is_some());
        assert!(opencode.effective_state_probe().is_some());
        assert_eq!(kiro.identity(), KiroAdapter::static_ref().identity());
        assert!(kiro.native_lifecycle().is_none());
        assert!(kiro.skill_projection().is_some());
        assert!(kiro.managed_projection().is_some());
        assert!(kiro.effective_state_probe().is_none());
        assert!(
            kiro.managed_declaration_contract(CapabilityScope::Global)
                .is_some()
        );
        assert_eq!(qwen.identity(), QwenAdapter::static_ref().identity());
        assert!(qwen.native_lifecycle().is_some());
        assert!(qwen.native_distribution().is_some());
        assert!(qwen.managed_projection().is_some());
        assert!(qwen.effective_state_probe().is_some());
        assert!(qwen.skill_projection().is_some());
    }

    #[test]
    fn factory_profile_keeps_marketplace_project_scope_unsupported() {
        let profile =
            FactoryAdapter::static_ref().select_profile(&NativeVersion::new("0.171.0").unwrap());
        let capabilities = profile.mutation_capabilities().unwrap();
        assert_support(
            capabilities,
            CapabilityScope::Project,
            "plugin.install",
            CapabilitySupport::Supported,
        );
        for capability in [
            "marketplace.register",
            "marketplace.remove",
            "marketplace.update",
        ] {
            assert_support(
                capabilities,
                CapabilityScope::Project,
                capability,
                CapabilitySupport::Unsupported,
            );
        }
    }

    fn support(value: bool) -> CapabilitySupport {
        if value {
            CapabilitySupport::Supported
        } else {
            CapabilitySupport::Unverified
        }
    }

    fn assert_support(
        sets: &skilltap_core::domain::ScopedCapabilitySets,
        scope: CapabilityScope,
        capability: &str,
        expected: CapabilitySupport,
    ) {
        assert_eq!(
            sets.for_scope_kind(scope)
                .support(&CapabilityId::new(capability).unwrap()),
            Some(expected),
            "{scope:?} {capability}",
        );
    }
}
