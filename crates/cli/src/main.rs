use std::process::ExitCode;

use clap::{Parser, error::ErrorKind};

fn main() -> ExitCode {
    match skilltap::command::Cli::try_parse() {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            let exit = if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
            let _ = error.print();
            exit
        }
    }
}
