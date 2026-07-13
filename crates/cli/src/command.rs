use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};
use skilltap_core::{
    domain::{
        HarnessId, NativeId, RelativeArtifactPath, RequestedRevision, SourceLocator,
        TargetSelection,
    },
    storage::{ClaudeInstructionMode, HarnessBinary, UpdateInterval},
};

/// Shared guidance printed on every executable leaf command.
///
/// Keep this text in the command grammar so compiled help, plugin guidance,
/// and website links all describe the same result/exit contract.
pub(crate) const EXIT_STATUS_HELP: &str = concat!(
    "Exit status: 0 completed; 1 invalid or pre-mutation failure; 2 attention ",
    "or user decision required; 3 partial mutation requiring recovery."
);

#[derive(Debug, Parser)]
#[command(
    name = "skilltap",
    version = skilltap_core::VERSION,
    about = "Manage local Codex and Claude Code environments",
    subcommand_required = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage enabled agent harnesses.
    Harness(HarnessArgs),
    /// Adopt existing native resources into desired state.
    #[command(after_help = EXIT_STATUS_HELP)]
    Adopt(AdoptArgs),
    /// Inspect desired, observed, and managed state.
    #[command(after_help = EXIT_STATUS_HELP)]
    Status(StatusArgs),
    /// Show the operations required to reach desired state.
    #[command(after_help = EXIT_STATUS_HELP)]
    Plan(PlanArgs),
    /// Reconcile managed resources with desired state.
    #[command(after_help = EXIT_STATUS_HELP)]
    Sync(SyncArgs),
    /// Install or repair the skilltap binary and first-party harness plugin.
    #[command(
        after_help = "Bootstrap is non-interactive. It detects Claude/Codex independently; use --allow-major to acknowledge an existing major-version upgrade.\nExit status: 0 completed; 1 invalid or pre-mutation failure; 2 attention or user decision required; 3 partial mutation requiring recovery."
    )]
    Bootstrap(BootstrapArgs),
    /// Manage registered native marketplaces.
    Marketplace(MarketplaceArgs),
    /// Manage plugins.
    Plugin(PluginArgs),
    /// Manage standalone skills.
    Skill(SkillArgs),
    /// Manage canonical AGENTS.md instructions and harness bridges.
    Instructions(InstructionsArgs),
    /// Manage the optional update daemon.
    Daemon(DaemonArgs),
}

#[derive(Debug, Args)]
pub struct HarnessArgs {
    #[command(subcommand)]
    pub command: HarnessCommand,
}

#[derive(Debug, Subcommand)]
pub enum HarnessCommand {
    /// List configured harnesses.
    #[command(after_help = EXIT_STATUS_HELP)]
    List(OutputArgs),
    /// Enable one harness.
    #[command(after_help = EXIT_STATUS_HELP)]
    Enable(HarnessEnableArgs),
    /// Disable one harness without uninstalling its resources.
    #[command(after_help = EXIT_STATUS_HELP)]
    Disable(HarnessChangeArgs),
}

#[derive(Debug, Args)]
pub struct HarnessEnableArgs {
    /// Harness to enable (`codex` or `claude`).
    #[arg(value_parser = parse_harness)]
    pub harness: HarnessId,
    /// Override the harness executable name or absolute path.
    #[arg(long, value_name = "PATH", value_parser = parse_harness_binary)]
    pub binary: Option<HarnessBinary>,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct HarnessChangeArgs {
    /// Harness to disable (`codex` or `claude`).
    #[arg(value_parser = parse_harness)]
    pub harness: HarnessId,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct AdoptArgs {
    /// Adopt from one harness or all enabled harnesses.
    #[arg(long, value_name = "TARGET", value_parser = parse_target)]
    pub from: Option<TargetSelection>,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct BootstrapArgs {
    /// Detect and set up Codex, Claude Code, or both harnesses.
    #[arg(long, value_name = "TARGET", value_parser = parse_target)]
    pub target: Option<TargetSelection>,
    /// Allow an existing binary to cross a major release boundary.
    #[arg(long)]
    pub allow_major: bool,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct MarketplaceArgs {
    #[command(subcommand)]
    pub command: MarketplaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum MarketplaceCommand {
    /// Register an explicit marketplace source.
    #[command(after_help = EXIT_STATUS_HELP)]
    Add(MarketplaceAddArgs),
    /// Remove a registered marketplace.
    #[command(after_help = EXIT_STATUS_HELP)]
    Remove(MarketplaceNamedArgs),
    /// Update one marketplace or all selected marketplaces.
    #[command(after_help = EXIT_STATUS_HELP)]
    Update(MarketplaceUpdateArgs),
    /// List registered marketplaces without browsing their contents.
    #[command(after_help = EXIT_STATUS_HELP)]
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct MarketplaceAddArgs {
    /// Marketplace source locator (for example a Git URL or local path).
    #[arg(value_parser = parse_source_locator)]
    pub source: SourceLocator,
    /// Stable native marketplace name to register.
    #[arg(long, value_parser = parse_native_id)]
    pub name: Option<NativeId>,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct MarketplaceNamedArgs {
    /// Registered marketplace name.
    #[arg(value_parser = parse_native_id)]
    pub name: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct MarketplaceUpdateArgs {
    /// Registered marketplace name; omit to update all selected marketplaces.
    #[arg(value_parser = parse_native_id)]
    pub name: Option<NativeId>,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Install an exact plugin@marketplace selector.
    #[command(after_help = EXIT_STATUS_HELP)]
    Install(PluginInstallArgs),
    /// Remove one exact plugin@marketplace selector.
    #[command(after_help = EXIT_STATUS_HELP)]
    Remove(PluginNamedArgs),
    /// Update one plugin or all selected plugins.
    #[command(after_help = EXIT_STATUS_HELP)]
    Update(PluginUpdateArgs),
    /// List installed and desired plugins.
    #[command(after_help = EXIT_STATUS_HELP)]
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct PluginInstallArgs {
    /// Exact plugin selector in `<plugin>@<marketplace>` form.
    #[arg(value_name = "PLUGIN@MARKETPLACE", value_parser = parse_plugin_selector)]
    pub plugin: NativeId,
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub selection: SelectionArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct PluginNamedArgs {
    /// Exact plugin selector in `<plugin>@<marketplace>` form.
    #[arg(value_name = "PLUGIN@MARKETPLACE", value_parser = parse_plugin_selector)]
    pub plugin: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct PluginUpdateArgs {
    /// Managed plugin name; omit to update all selected plugins.
    #[arg(value_parser = parse_native_id)]
    pub plugin: Option<NativeId>,
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// Install one explicit skill directory or source.
    #[command(after_help = EXIT_STATUS_HELP)]
    Install(SkillInstallArgs),
    /// Remove one managed standalone skill.
    #[command(after_help = EXIT_STATUS_HELP)]
    Remove(SkillNamedArgs),
    /// Update one skill or all selected skills.
    #[command(after_help = EXIT_STATUS_HELP)]
    Update(SkillUpdateArgs),
    /// List installed and desired standalone skills.
    #[command(after_help = EXIT_STATUS_HELP)]
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    /// Skill source locator (the complete directory must contain SKILL.md).
    #[arg(value_parser = parse_source_locator)]
    pub source: SourceLocator,
    /// Desired standalone skill name.
    #[arg(long, value_parser = parse_native_id)]
    pub name: Option<NativeId>,
    /// Git revision to fetch for the skill source.
    #[arg(long = "ref", value_name = "GIT_REF", value_parser = parse_requested_revision)]
    pub requested_revision: Option<RequestedRevision>,
    /// Relative subdirectory containing the skill.
    #[arg(long, value_name = "SUBDIRECTORY", value_parser = parse_relative_path)]
    pub path: Option<RelativeArtifactPath>,
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct SkillNamedArgs {
    /// Managed standalone skill name.
    #[arg(value_parser = parse_native_id)]
    pub skill: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct SkillUpdateArgs {
    /// Managed skill name; omit to update all selected skills.
    #[arg(value_parser = parse_native_id)]
    pub skill: Option<NativeId>,
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct InstructionsArgs {
    #[command(subcommand)]
    pub command: InstructionsCommand,
}

#[derive(Debug, Subcommand)]
pub enum InstructionsCommand {
    /// Create canonical instructions and required harness bridges.
    #[command(after_help = EXIT_STATUS_HELP)]
    Setup(InstructionsSetupArgs),
    /// Inspect canonical instructions and harness bridges.
    #[command(after_help = EXIT_STATUS_HELP)]
    Status(ScopedOutputArgs),
    /// Repair bridges already managed by skilltap.
    #[command(after_help = EXIT_STATUS_HELP)]
    Repair(InstructionsRepairArgs),
}

#[derive(Debug, Args)]
pub struct InstructionsSetupArgs {
    /// Claude instruction bridge mode (`symlink` or `import`).
    #[arg(long, value_name = "MODE", value_parser = parse_instruction_mode)]
    pub mode: Option<ClaudeInstructionMode>,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct InstructionsRepairArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub acknowledgment: AcknowledgmentArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Install and start the user update service.
    #[command(after_help = EXIT_STATUS_HELP)]
    Enable(DaemonEnableArgs),
    /// Stop and remove the user update service.
    #[command(after_help = EXIT_STATUS_HELP)]
    Disable(OutputArgs),
    /// Inspect the user update service.
    #[command(after_help = EXIT_STATUS_HELP)]
    Status(OutputArgs),
    /// Run one foreground daemon cycle.
    #[command(after_help = EXIT_STATUS_HELP)]
    Run(OutputArgs),
}

#[derive(Debug, Args)]
pub struct DaemonEnableArgs {
    /// Update interval for the user service (for example `6h`).
    #[arg(long, value_name = "DURATION", value_parser = parse_update_interval)]
    pub interval: Option<UpdateInterval>,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Clone, Debug, Default, Args)]
pub struct TargetArgs {
    /// Select Codex, Claude Code, or all enabled harnesses.
    #[arg(long, value_name = "TARGET", value_parser = parse_target)]
    pub target: Option<TargetSelection>,
}

#[derive(Clone, Debug, Default, Args)]
pub struct ScopeArgs {
    /// Select the current project or the project containing PATH.
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        value_parser = parse_project_path,
        conflicts_with = "all_scopes"
    )]
    pub project: Option<Option<PathBuf>>,
    /// Select global scope and every project recorded in inventory.
    #[arg(long, conflicts_with = "project")]
    pub all_scopes: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScopeArgument {
    Global,
    Project(Option<PathBuf>),
    AllScopes,
}

impl ScopeArgs {
    pub fn argument(&self) -> ScopeArgument {
        match (&self.project, self.all_scopes) {
            (Some(path), false) => ScopeArgument::Project(path.clone()),
            (None, true) => ScopeArgument::AllScopes,
            (None, false) => ScopeArgument::Global,
            (Some(_), true) => unreachable!("Clap rejects conflicting scope flags"),
        }
    }
}

#[derive(Clone, Debug, Default, Args)]
pub struct SelectionArgs {
    /// Include an exact resource or component selector.
    #[arg(long, value_name = "SELECTOR", action = clap::ArgAction::Append, value_parser = parse_native_id)]
    pub include: Vec<NativeId>,
    /// Exclude an exact resource or component selector.
    #[arg(long, value_name = "SELECTOR", action = clap::ArgAction::Append, value_parser = parse_native_id)]
    pub exclude: Vec<NativeId>,
}

#[derive(Clone, Copy, Debug, Default, Args)]
pub struct AcknowledgmentArgs {
    /// Accept every eligible partial or lossy consequence in this operation.
    /// Required, unsupported, drifted, and otherwise blocked work remains
    /// blocked even when this flag is present.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Clone, Copy, Debug, Default, Args)]
pub struct OutputArgs {
    /// Emit one stable JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ScopedTargetArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ScopedOutputArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

fn parse_harness(value: &str) -> Result<HarnessId, String> {
    match value {
        "codex" | "claude" => HarnessId::new(value).map_err(|error| error.to_string()),
        _ => Err("expected `codex` or `claude`".to_owned()),
    }
}

fn parse_target(value: &str) -> Result<TargetSelection, String> {
    match value {
        "all" => Ok(TargetSelection::All),
        "codex" | "claude" => parse_harness(value).map(TargetSelection::Only),
        _ => Err("expected `codex`, `claude`, or `all`".to_owned()),
    }
}

fn parse_harness_binary(value: &str) -> Result<HarnessBinary, String> {
    HarnessBinary::new(value).map_err(|error| error.to_string())
}

fn parse_native_id(value: &str) -> Result<NativeId, String> {
    NativeId::new(value).map_err(|error| error.to_string())
}

fn parse_plugin_selector(value: &str) -> Result<NativeId, String> {
    let Some((plugin, marketplace)) = value.split_once('@') else {
        return Err("expected an exact `<plugin>@<marketplace>` selector".to_owned());
    };
    if plugin.is_empty() || marketplace.is_empty() || marketplace.contains('@') {
        return Err("expected an exact `<plugin>@<marketplace>` selector".to_owned());
    }
    parse_native_id(value)
}

fn parse_source_locator(value: &str) -> Result<SourceLocator, String> {
    SourceLocator::new(value).map_err(|error| error.to_string())
}

fn parse_project_path(value: &str) -> Result<PathBuf, String> {
    SourceLocator::new(value)
        .map(|_| PathBuf::from(value))
        .map_err(|error| error.to_string())
}

fn parse_requested_revision(value: &str) -> Result<RequestedRevision, String> {
    RequestedRevision::new(value).map_err(|error| error.to_string())
}

fn parse_relative_path(value: &str) -> Result<RelativeArtifactPath, String> {
    RelativeArtifactPath::new(value).map_err(|error| error.to_string())
}

fn parse_instruction_mode(value: &str) -> Result<ClaudeInstructionMode, String> {
    match value {
        "symlink" => Ok(ClaudeInstructionMode::Symlink),
        "import" => Ok(ClaudeInstructionMode::Import),
        _ => Err("expected `symlink` or `import`".to_owned()),
    }
}

fn parse_update_interval(value: &str) -> Result<UpdateInterval, String> {
    UpdateInterval::from_str(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
