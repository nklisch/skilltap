use crate::command::{
    AdoptArgs, Command, DaemonCommand, HarnessChangeArgs, HarnessCommand, HarnessEnableArgs,
    InstructionsCommand, InstructionsRepairArgs, InstructionsSetupArgs, MarketplaceAddArgs,
    MarketplaceCommand, MarketplaceNamedArgs, MarketplaceUpdateArgs, OutputArgs, PlanArgs,
    PluginCommand, PluginInstallArgs, PluginNamedArgs, PluginUpdateArgs, ScopedOutputArgs,
    ScopedTargetArgs, SkillCommand, SkillInstallArgs, SkillNamedArgs, StatusArgs, SyncArgs,
};

pub(crate) enum Dispatch {
    Status(StatusArgs),
    Adopt(AdoptArgs),
    Plan(PlanArgs),
    Sync(SyncArgs),
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
    InstructionSetup(InstructionsSetupArgs),
    InstructionRepair(InstructionsRepairArgs),
    HarnessList(OutputArgs),
    HarnessEnable(HarnessEnableArgs),
    HarnessDisable(HarnessChangeArgs),
    Unavailable { command: &'static str, json: bool },
}

impl Dispatch {
    pub(crate) fn from_command(command: Command) -> Self {
        match command {
            Command::Status(args) => Self::Status(args),
            Command::Adopt(args) => Self::Adopt(args),
            Command::Plan(args) => Self::Plan(args),
            Command::Sync(args) => Self::Sync(args),
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
                SkillCommand::Update(args) => unavailable("skill update", args.output.json),
                SkillCommand::List(args) => Self::SkillList(args),
            },
            Command::Instructions(args) => match args.command {
                InstructionsCommand::Setup(args) => Self::InstructionSetup(args),
                InstructionsCommand::Status(args) => Self::InstructionStatus(args),
                InstructionsCommand::Repair(args) => Self::InstructionRepair(args),
            },
            Command::Daemon(args) => match args.command {
                DaemonCommand::Enable(args) => unavailable("daemon enable", args.output.json),
                DaemonCommand::Disable(args) => unavailable("daemon disable", args.json),
                DaemonCommand::Status(args) => unavailable("daemon status", args.json),
                DaemonCommand::Run => unavailable("daemon run", false),
            },
        }
    }

    pub(crate) const fn json(&self) -> bool {
        match self {
            Self::Status(args) => args.output.json,
            Self::Adopt(args) => args.output.json,
            Self::Plan(args) => args.output.json,
            Self::Sync(args) => args.output.json,
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
            Self::InstructionSetup(args) => args.output.json,
            Self::InstructionRepair(args) => args.output.json,
            Self::HarnessList(args) => args.json,
            Self::HarnessEnable(args) => args.output.json,
            Self::HarnessDisable(args) => args.output.json,
            Self::Unavailable { json, .. } => *json,
        }
    }
}

const fn unavailable(command: &'static str, json: bool) -> Dispatch {
    Dispatch::Unavailable { command, json }
}
