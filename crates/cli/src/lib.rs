mod application;
pub mod command;
mod dispatch;
mod entrypoint;
pub mod outcome;
pub mod output;

pub use entrypoint::{CommandExecution, OutputChannel, run_from};

pub use outcome::{
    ErrorDetail, NextAction, OperationOutcome, Outcome, OutputEntry, OutputScope, OutputValue,
    ResourceOutcome, ResultClass, Warning,
};
pub use output::{ExitCode, JsonRenderer, PlainRenderer, RenderError, Renderer};
