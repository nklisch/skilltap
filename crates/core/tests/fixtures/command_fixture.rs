use std::{env, process};

fn main() {
    println!("cwd={}", env::current_dir().unwrap().display());
    let mut exit_code = 0;
    for (index, argument) in env::args().skip(1).enumerate() {
        println!("arg[{index}]={argument}");
        if let Some(code) = argument.strip_prefix("--exit=") {
            exit_code = code.parse().unwrap();
        }
    }
    eprintln!("fixture-stderr");
    process::exit(exit_code);
}
