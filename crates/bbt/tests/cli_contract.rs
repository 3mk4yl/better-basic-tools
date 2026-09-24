use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn version_command_prints_package_version() {
    let output = bbt().arg("version").output().expect("run bbt version");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("bbt"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn missing_inspection_target_is_structured_success_for_json_consumers() {
    let output = bbt()
        .args(["fs", "inspect", "bbt-definitely-missing-path", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["exists"], false);
}

#[test]
fn invalid_cli_usage_returns_nonzero() {
    let output = bbt()
        .args(["fs", "inspect"])
        .output()
        .expect("run bbt invalid invocation");

    assert!(!output.status.success());
}
