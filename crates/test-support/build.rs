#[cfg(unix)]
use std::{env, fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

const FAKE_NATIVE: &str = r#"#!/bin/sh
# Behavior scripts return explicit lifecycle statuses; bookkeeping must not
# turn a successful version probe into a wrapper failure.
set -u
case "$0" in
  */*) fixture_directory=${0%/*} ;;
  *) fixture_directory=. ;;
esac
fixture_directory=$(cd "$fixture_directory" && pwd -P)
. "$fixture_directory/behavior"
"#;

#[cfg(unix)]
fn main() {
    println!("cargo:rerun-if-changed=fixtures/escaped_pipe_holder.c");
    if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() != Ok("unix") {
        return;
    }

    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR"));
    let output = output_directory.join("escaped-pipe-holder");
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

    let fake_native = output_directory.join("fake-native");
    fs::write(&fake_native, FAKE_NATIVE).expect("write generic fake-native fixture");
    let mut permissions = fs::metadata(&fake_native)
        .expect("stat generic fake-native fixture")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_native, permissions)
        .expect("make generic fake-native fixture executable");
    println!(
        "cargo:rustc-env=SKILLTAP_FAKE_NATIVE_EXECUTABLE={}",
        fake_native.display()
    );
    println!(
        "cargo:rustc-env=SKILLTAP_ESCAPED_PIPE_HOLDER={}",
        output.display()
    );
}

#[cfg(not(unix))]
fn main() {
    println!("cargo:rerun-if-changed=fixtures/escaped_pipe_holder.c");
}
