//! Exact, relaxed contracts for the Junie and Amp declaration surfaces.
//!
//! These values are deliberately narrow. They describe the one attested
//! executable identity and documented file surfaces that may authorize a
//! foreground declaration write; they do not imply that either harness loaded
//! or activated the resulting files.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VerifiedTrustInteractiveContract {
    pub verified_version: &'static str,
    pub profile_id: &'static str,
    pub default_binary: &'static str,
    pub version_arguments: &'static [&'static str],
    pub global_skill_root: &'static str,
    pub project_skill_root: &'static str,
    pub global_mcp_document: &'static str,
    pub project_mcp_document: &'static str,
    pub mcp_container: &'static str,
    pub declared_list_arguments: Option<&'static [&'static str]>,
    pub effective_probe: EffectiveProbeContract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EffectiveProbeContract {
    /// No deterministic effective observer is part of this relaxed contract.
    InteractiveOnly,
}

/// Junie's version identity combines the marketing version and JetBrains build.
/// `/mcp` is interactive and no cache or agent prompt is an observer.
pub(super) const JUNIE: VerifiedTrustInteractiveContract = VerifiedTrustInteractiveContract {
    verified_version: "26.6.29+2144.10",
    profile_id: "junie-26-6-29-build-2144-10",
    default_binary: "junie",
    version_arguments: &["--version"],
    global_skill_root: ".junie/skills",
    project_skill_root: ".junie/skills",
    global_mcp_document: ".junie/mcp/mcp.json",
    project_mcp_document: ".junie/mcp/mcp.json",
    mcp_container: "mcpServers",
    declared_list_arguments: None,
    effective_probe: EffectiveProbeContract::InteractiveOnly,
};

/// Amp's release timestamp is part of the identity. The relative age printed by
/// `--version` is intentionally excluded because it changes on every run.
pub(super) const AMP: VerifiedTrustInteractiveContract = VerifiedTrustInteractiveContract {
    verified_version: "0.0.1784073393-g9a3a12+2026-07-14T23:56:33.000Z",
    profile_id: "amp-0-0-1784073393-g9a3a12",
    default_binary: "amp",
    version_arguments: &["--version"],
    global_skill_root: ".agents/skills",
    project_skill_root: ".agents/skills",
    global_mcp_document: "amp/settings.json",
    project_mcp_document: ".amp/settings.json",
    mcp_container: "amp.mcpServers",
    declared_list_arguments: Some(&["mcp", "list", "--json"]),
    effective_probe: EffectiveProbeContract::InteractiveOnly,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_are_declaration_only_and_exactly_scoped() {
        assert_eq!(JUNIE.default_binary, "junie");
        assert_eq!(
            JUNIE.effective_probe,
            EffectiveProbeContract::InteractiveOnly
        );
        assert!(JUNIE.declared_list_arguments.is_none());
        assert_eq!(AMP.default_binary, "amp");
        assert_eq!(
            AMP.declared_list_arguments,
            Some(["mcp", "list", "--json"].as_slice())
        );
        assert_eq!(AMP.effective_probe, EffectiveProbeContract::InteractiveOnly);
    }
}
