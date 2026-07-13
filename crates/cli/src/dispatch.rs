use skilltap_core::domain::{HarnessId, TargetSelection};
use skilltap_harnesses::TargetRegistry;

use crate::{
    ErrorDetail, NextAction,
    command::{
        AdoptArgs, BootstrapArgs, Command, DaemonCommand, HarnessChangeArgs, HarnessCommand,
        HarnessEnableArgs, InstructionsCommand, InstructionsRepairArgs, InstructionsSetupArgs,
        MarketplaceAddArgs, MarketplaceCommand, MarketplaceNamedArgs, MarketplaceUpdateArgs,
        OutputArgs, PlanArgs, PluginCommand, PluginInstallArgs, PluginNamedArgs, PluginUpdateArgs,
        ScopedOutputArgs, ScopedTargetArgs, SkillCommand, SkillInstallArgs, SkillNamedArgs,
        SkillUpdateArgs, StatusArgs, SyncArgs,
    },
};

pub(crate) enum Dispatch {
    Status(StatusArgs),
    Adopt(AdoptArgs),
    Plan(PlanArgs),
    Sync(SyncArgs),
    Bootstrap(BootstrapArgs),
    SkillList(ScopedTargetArgs),
    MarketplaceList(ScopedTargetArgs),
    PluginList(ScopedTargetArgs),
    InstructionStatus(ScopedOutputArgs),
    MarketplaceAdd(MarketplaceAddArgs),
    MarketplaceRemove(MarketplaceNamedArgs),
    MarketplaceUpdate(MarketplaceUpdateArgs),
    PluginInstall(PluginInstallArgs),
    PluginRemove(PluginNamedArgs),
    PluginUpdate(PluginUpdateArgs),
    SkillInstall(SkillInstallArgs),
    SkillRemove(SkillNamedArgs),
    SkillUpdate(SkillUpdateArgs),
    InstructionSetup(InstructionsSetupArgs),
    InstructionRepair(InstructionsRepairArgs),
    HarnessList(OutputArgs),
    HarnessEnable(HarnessEnableArgs),
    HarnessDisable(HarnessChangeArgs),
    DaemonEnable(crate::command::DaemonEnableArgs),
    DaemonDisable(OutputArgs),
    DaemonStatus(OutputArgs),
    DaemonRun(OutputArgs),
}

impl Dispatch {
    pub(crate) fn from_command(command: Command) -> Self {
        match command {
            Command::Status(args) => Self::Status(args),
            Command::Adopt(args) => Self::Adopt(args),
            Command::Plan(args) => Self::Plan(args),
            Command::Sync(args) => Self::Sync(args),
            Command::Bootstrap(args) => Self::Bootstrap(args),
            Command::Harness(args) => match args.command {
                HarnessCommand::List(args) => Self::HarnessList(args),
                HarnessCommand::Enable(args) => Self::HarnessEnable(args),
                HarnessCommand::Disable(args) => Self::HarnessDisable(args),
            },
            Command::Marketplace(args) => match args.command {
                MarketplaceCommand::Add(args) => Self::MarketplaceAdd(args),
                MarketplaceCommand::Remove(args) => Self::MarketplaceRemove(args),
                MarketplaceCommand::Update(args) => Self::MarketplaceUpdate(args),
                MarketplaceCommand::List(args) => Self::MarketplaceList(args),
            },
            Command::Plugin(args) => match args.command {
                PluginCommand::Install(args) => Self::PluginInstall(args),
                PluginCommand::Remove(args) => Self::PluginRemove(args),
                PluginCommand::Update(args) => Self::PluginUpdate(args),
                PluginCommand::List(args) => Self::PluginList(args),
            },
            Command::Skill(args) => match args.command {
                SkillCommand::Install(args) => Self::SkillInstall(args),
                SkillCommand::Remove(args) => Self::SkillRemove(args),
                SkillCommand::Update(args) => Self::SkillUpdate(args),
                SkillCommand::List(args) => Self::SkillList(args),
            },
            Command::Instructions(args) => match args.command {
                InstructionsCommand::Setup(args) => Self::InstructionSetup(args),
                InstructionsCommand::Status(args) => Self::InstructionStatus(args),
                InstructionsCommand::Repair(args) => Self::InstructionRepair(args),
            },
            Command::Daemon(args) => match args.command {
                DaemonCommand::Enable(args) => Self::DaemonEnable(args),
                DaemonCommand::Disable(args) => Self::DaemonDisable(args),
                DaemonCommand::Status(args) => Self::DaemonStatus(args),
                DaemonCommand::Run(args) => Self::DaemonRun(args),
            },
        }
    }

    pub(crate) const fn command_name(&self) -> &'static str {
        match self {
            Self::Status(_) => "status",
            Self::Adopt(_) => "adopt",
            Self::Plan(_) => "plan",
            Self::Sync(_) => "sync",
            Self::Bootstrap(_) => "bootstrap",
            Self::SkillList(_) => "skill list",
            Self::MarketplaceList(_) => "marketplace list",
            Self::PluginList(_) => "plugin list",
            Self::InstructionStatus(_) => "instructions status",
            Self::MarketplaceAdd(_) => "marketplace add",
            Self::MarketplaceRemove(_) => "marketplace remove",
            Self::MarketplaceUpdate(_) => "marketplace update",
            Self::PluginInstall(_) => "plugin install",
            Self::PluginRemove(_) => "plugin remove",
            Self::PluginUpdate(_) => "plugin update",
            Self::SkillInstall(_) => "skill install",
            Self::SkillRemove(_) => "skill remove",
            Self::SkillUpdate(_) => "skill update",
            Self::InstructionSetup(_) => "instructions setup",
            Self::InstructionRepair(_) => "instructions repair",
            Self::HarnessList(_) => "harness list",
            Self::HarnessEnable(_) => "harness enable",
            Self::HarnessDisable(_) => "harness disable",
            Self::DaemonEnable(_) => "daemon enable",
            Self::DaemonDisable(_) => "daemon disable",
            Self::DaemonStatus(_) => "daemon status",
            Self::DaemonRun(_) => "daemon run",
        }
    }

    pub(crate) fn validate_targets(&self, registry: &TargetRegistry) -> Result<(), ErrorDetail> {
        if let Some(harness) = self.harness_argument()
            && !registry.contains(harness)
        {
            return Err(target_not_registered(harness, self.command_name()));
        }
        let Some(TargetSelection::Only(target)) = self.target_selection() else {
            return Ok(());
        };
        if !registry.contains(target) {
            return Err(target_not_registered(target, self.command_name()));
        }
        if matches!(self, Self::Bootstrap(_))
            && !registry
                .first_party_targets()
                .any(|adapter| adapter.identity().id == *target)
        {
            return Err(ErrorDetail::new(
                "bootstrap_target_unavailable",
                "The requested harness has no first-party skilltap plugin distribution.",
            )
            .with_context("harness", target.as_str()));
        }
        Ok(())
    }

    fn harness_argument(&self) -> Option<&HarnessId> {
        match self {
            Self::HarnessEnable(args) => Some(&args.harness),
            Self::HarnessDisable(args) => Some(&args.harness),
            _ => None,
        }
    }

    fn target_selection(&self) -> Option<&TargetSelection> {
        match self {
            Self::Status(args) => args.target.target.as_ref(),
            Self::Adopt(args) => args.from.as_ref(),
            Self::Plan(args) => args.target.target.as_ref(),
            Self::Sync(args) => args.target.target.as_ref(),
            Self::Bootstrap(args) => args.target.as_ref(),
            Self::SkillList(args) | Self::MarketplaceList(args) | Self::PluginList(args) => {
                args.target.target.as_ref()
            }
            Self::MarketplaceAdd(args) => args.common.target.target.as_ref(),
            Self::MarketplaceRemove(args) => args.common.target.target.as_ref(),
            Self::MarketplaceUpdate(args) => args.common.target.target.as_ref(),
            Self::PluginInstall(args) => args.target.target.as_ref(),
            Self::PluginRemove(args) => args.common.target.target.as_ref(),
            Self::PluginUpdate(args) => args.target.target.as_ref(),
            Self::SkillInstall(args) => args.target.target.as_ref(),
            Self::SkillRemove(args) => args.common.target.target.as_ref(),
            Self::SkillUpdate(args) => args.target.target.as_ref(),
            Self::InstructionStatus(_)
            | Self::InstructionSetup(_)
            | Self::InstructionRepair(_)
            | Self::HarnessList(_)
            | Self::HarnessEnable(_)
            | Self::HarnessDisable(_)
            | Self::DaemonEnable(_)
            | Self::DaemonDisable(_)
            | Self::DaemonStatus(_)
            | Self::DaemonRun(_) => None,
        }
    }

    pub(crate) const fn json(&self) -> bool {
        match self {
            Self::Status(args) => args.output.json,
            Self::Adopt(args) => args.output.json,
            Self::Plan(args) => args.output.json,
            Self::Sync(args) => args.output.json,
            Self::Bootstrap(args) => args.output.json,
            Self::SkillList(args) => args.output.json,
            Self::MarketplaceList(args) => args.output.json,
            Self::PluginList(args) => args.output.json,
            Self::InstructionStatus(args) => args.output.json,
            Self::MarketplaceAdd(args) => args.common.output.json,
            Self::MarketplaceRemove(args) => args.common.output.json,
            Self::MarketplaceUpdate(args) => args.common.output.json,
            Self::PluginInstall(args) => args.output.json,
            Self::PluginRemove(args) => args.common.output.json,
            Self::PluginUpdate(args) => args.output.json,
            Self::SkillInstall(args) => args.output.json,
            Self::SkillRemove(args) => args.common.output.json,
            Self::SkillUpdate(args) => args.output.json,
            Self::InstructionSetup(args) => args.output.json,
            Self::InstructionRepair(args) => args.output.json,
            Self::HarnessList(args) => args.json,
            Self::HarnessEnable(args) => args.output.json,
            Self::HarnessDisable(args) => args.output.json,
            Self::DaemonEnable(args) => args.output.json,
            Self::DaemonDisable(args) | Self::DaemonStatus(args) => args.json,
            Self::DaemonRun(args) => args.json,
        }
    }
}

fn target_not_registered(target: &HarnessId, command: &'static str) -> ErrorDetail {
    ErrorDetail::new(
        "target_not_registered",
        "The requested harness is not registered in this build.",
    )
    .with_context("harness", target.as_str())
    .with_next_action(
        NextAction::new(
            "inspect_registered_targets",
            "Inspect command help for the harnesses registered in this build.",
        )
        .with_command(format!("skilltap {command} --help")),
    )
}
