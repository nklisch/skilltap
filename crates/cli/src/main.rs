use std::process::ExitCode;

fn main() -> ExitCode {
    let execution = skilltap::run_from(std::env::args_os());
    match execution.channel {
        skilltap::OutputChannel::Stdout => print!("{}", execution.document),
        skilltap::OutputChannel::Stderr => eprint!("{}", execution.document),
    }
    ExitCode::from(execution.exit_code)
}
