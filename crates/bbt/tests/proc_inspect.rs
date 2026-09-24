use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn proc_inspect_json_emits_schema_envelope_for_current_process() {
    let pid = std::process::id().to_string();
    let output = bbt()
        .args(["proc", "inspect", &pid, "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.proc.inspect.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert_eq!(
        json["data"]["pid"].as_u64(),
        Some(std::process::id() as u64)
    );
    assert_eq!(json["data"]["exists"], true);
    assert!(json["data"]["parent_pid"].as_u64().unwrap_or_default() > 0);
    assert!(json["data"]["state"].is_string());
    assert!(json["data"]["uid"].as_u64().is_some());
    assert!(json["data"]["command_line"].is_array());
    assert!(json["data"]["command_line_bytes_hex"].is_array());
}

#[test]
fn proc_inspect_agent_emits_agent_report() {
    let pid = std::process::id().to_string();
    let output = bbt()
        .args(["proc", "inspect", &pid, "--agent"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "proc");
    assert_eq!(json["subject"]["command"], "inspect");
    let summary = json["summary"].as_str().expect("summary string");
    assert!(summary.contains("with parent "));
    assert!(!summary.contains("Some("));
    assert!(json["facts"]
        .as_array()
        .expect("facts array")
        .iter()
        .any(|fact| fact["key"] == "process_state"));
}

#[test]
fn proc_inspect_human_output_is_readable() {
    let pid = std::process::id().to_string();
    let output = bbt()
        .args(["proc", "inspect", &pid])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Process inspect:"));
    assert!(stdout.contains("State:"));
    assert!(stdout.contains("Parent PID:"));
    assert!(stdout.contains("Safe next observations:"));
}

#[test]
fn proc_inspect_missing_pid_reports_not_found_without_failing() {
    let output = bbt()
        .args(["proc", "inspect", "999999999", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.proc.inspect.v1");
    assert_eq!(json["data"]["pid"].as_u64(), Some(999999999));
    assert_eq!(json["data"]["exists"], false);
    assert!(!json["data"]["warnings"]
        .as_array()
        .expect("warnings")
        .is_empty());
}
