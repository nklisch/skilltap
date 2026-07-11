use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "skilltap", version = skilltap_core::VERSION, about = "Manage local agent environments")]
struct Cli {}

fn main() {
    Cli::parse();
}
