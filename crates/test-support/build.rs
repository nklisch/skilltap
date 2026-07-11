use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=fixtures/escaped_pipe_holder.c");
    if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() != Ok("unix") {
        return;
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR"))
        .join("escaped-pipe-holder");
    let compiler = env::var_os("CC").unwrap_or_else(|| "cc".into());
    let status = Command::new(compiler)
        .args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "fixtures/escaped_pipe_holder.c",
            "-o",
        ])
        .arg(&output)
        .status()
        .expect("compile escaped pipe-holder fixture");
    assert!(status.success(), "escaped pipe-holder fixture must compile");
}
