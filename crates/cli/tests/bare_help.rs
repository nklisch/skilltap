use std::process::Command;

#[test]
fn bare_compiled_binary_reports_normalized_error_and_root_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_skilltap"))
        .output()
        .expect("run compiled skilltap binary");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = std::str::from_utf8(&output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("Code: missing_command"));
    assert!(stderr.contains("Usage: skilltap <COMMAND>"));
}
