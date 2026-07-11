use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};
use skilltap_core::{
    domain::{
        HarnessId, NativeId, RelativeArtifactPath, RequestedRevision, SourceLocator,
        TargetSelection,
    },
    storage::{ClaudeInstructionMode, HarnessBinary, UpdateInterval},
};

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
    Adopt(AdoptArgs),
    /// Inspect desired, observed, and managed state.
    Status(StatusArgs),
    /// Show the operations required to reach desired state.
    Plan(PlanArgs),
    /// Reconcile managed resources with desired state.
    Sync(SyncArgs),
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
    List(OutputArgs),
    /// Enable one harness.
    Enable(HarnessEnableArgs),
    /// Disable one harness without uninstalling its resources.
    Disable(HarnessChangeArgs),
}

#[derive(Debug, Args)]
pub struct HarnessEnableArgs {
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
pub struct MarketplaceArgs {
    #[command(subcommand)]
    pub command: MarketplaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum MarketplaceCommand {
    /// Register an explicit marketplace source.
    Add(MarketplaceAddArgs),
    /// Remove a registered marketplace.
    Remove(MarketplaceNamedArgs),
    /// Update one marketplace or all selected marketplaces.
    Update(MarketplaceUpdateArgs),
    /// List registered marketplaces without browsing their contents.
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct MarketplaceAddArgs {
    #[arg(value_parser = parse_source_locator)]
    pub source: SourceLocator,
    #[arg(long, value_parser = parse_native_id)]
    pub name: Option<NativeId>,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct MarketplaceNamedArgs {
    #[arg(value_parser = parse_native_id)]
    pub name: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct MarketplaceUpdateArgs {
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
    Install(PluginInstallArgs),
    /// Remove one managed plugin.
    Remove(PluginNamedArgs),
    /// Update one plugin or all selected plugins.
    Update(PluginUpdateArgs),
    /// List installed and desired plugins.
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct PluginInstallArgs {
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
    #[arg(value_parser = parse_native_id)]
    pub plugin: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct PluginUpdateArgs {
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
    Install(SkillInstallArgs),
    /// Remove one managed standalone skill.
    Remove(SkillNamedArgs),
    /// Update one skill or all selected skills.
    Update(SkillUpdateArgs),
    /// List installed and desired standalone skills.
    List(ScopedTargetArgs),
}

#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    #[arg(value_parser = parse_source_locator)]
    pub source: SourceLocator,
    #[arg(long, value_parser = parse_native_id)]
    pub name: Option<NativeId>,
    #[arg(long = "ref", value_name = "GIT_REF", value_parser = parse_requested_revision)]
    pub requested_revision: Option<RequestedRevision>,
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
    #[arg(value_parser = parse_native_id)]
    pub skill: NativeId,
    #[command(flatten)]
    pub common: ScopedTargetArgs,
}

#[derive(Debug, Args)]
pub struct SkillUpdateArgs {
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
    Setup(InstructionsSetupArgs),
    /// Inspect canonical instructions and harness bridges.
    Status(ScopedOutputArgs),
    /// Repair bridges already managed by skilltap.
    Repair(InstructionsRepairArgs),
}

#[derive(Debug, Args)]
pub struct InstructionsSetupArgs {
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
    Enable(DaemonEnableArgs),
    /// Stop and remove the user update service.
    Disable(OutputArgs),
    /// Inspect the user update service.
    Status(OutputArgs),
    /// Run one foreground daemon cycle.
    Run,
}

#[derive(Debug, Args)]
pub struct DaemonEnableArgs {
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
    /// Acknowledge the exact partial or lossy result reported by the plan.
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
