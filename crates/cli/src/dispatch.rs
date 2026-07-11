use crate::command::{
    AdoptArgs, Command, DaemonCommand, HarnessChangeArgs, HarnessCommand, HarnessEnableArgs,
    InstructionsCommand, MarketplaceCommand, OutputArgs, PluginCommand, SkillCommand, StatusArgs,
};

pub(crate) enum Dispatch {
    Status(StatusArgs),
    Adopt(AdoptArgs),
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
            Command::Plan(args) => unavailable("plan", args.output.json),
            Command::Sync(args) => unavailable("sync", args.output.json),
            Command::Harness(args) => match args.command {
                HarnessCommand::List(args) => Self::HarnessList(args),
                HarnessCommand::Enable(args) => Self::HarnessEnable(args),
                HarnessCommand::Disable(args) => Self::HarnessDisable(args),
            },
            Command::Marketplace(args) => match args.command {
                MarketplaceCommand::Add(args) => {
                    unavailable("marketplace add", args.common.output.json)
                }
                MarketplaceCommand::Remove(args) => {
                    unavailable("marketplace remove", args.common.output.json)
                }
                MarketplaceCommand::Update(args) => {
                    unavailable("marketplace update", args.common.output.json)
                }
                MarketplaceCommand::List(args) => unavailable("marketplace list", args.output.json),
            },
            Command::Plugin(args) => match args.command {
                PluginCommand::Install(args) => unavailable("plugin install", args.output.json),
                PluginCommand::Remove(args) => {
                    unavailable("plugin remove", args.common.output.json)
                }
                PluginCommand::Update(args) => unavailable("plugin update", args.output.json),
                PluginCommand::List(args) => unavailable("plugin list", args.output.json),
            },
            Command::Skill(args) => match args.command {
                SkillCommand::Install(args) => unavailable("skill install", args.output.json),
                SkillCommand::Remove(args) => unavailable("skill remove", args.common.output.json),
                SkillCommand::Update(args) => unavailable("skill update", args.output.json),
                SkillCommand::List(args) => unavailable("skill list", args.output.json),
            },
            Command::Instructions(args) => match args.command {
                InstructionsCommand::Setup(args) => {
                    unavailable("instructions setup", args.output.json)
                }
                InstructionsCommand::Status(args) => {
                    unavailable("instructions status", args.output.json)
                }
                InstructionsCommand::Repair(args) => {
                    unavailable("instructions repair", args.output.json)
                }
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
