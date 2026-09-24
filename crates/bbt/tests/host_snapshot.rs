use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn host_snapshot_json_emits_schema_envelope() {
    let output = bbt()
        .args(["host", "snapshot", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.host.snapshot.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["data"]["hostname"].is_string());
    assert!(json["data"]["kernel"]["sysname"].is_string());
    assert!(json["data"]["current_user"]["effective_uid"]
        .as_u64()
        .is_some());
    assert!(
        json["data"]["cpu"]["logical_cpus"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        json["data"]["memory"]["total_bytes"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(!json["data"]["filesystems"]
        .as_array()
        .expect("filesystems")
        .is_empty());
}

#[test]
fn host_snapshot_agent_emits_agent_report() {
    let output = bbt()
        .args(["host", "snapshot", "--agent"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "host");
    assert_eq!(json["subject"]["command"], "snapshot");
    assert!(json["facts"]
        .as_array()
        .expect("facts array")
        .iter()
        .any(|fact| fact["key"] == "cpu_logical_cpus"));
    assert!(json["interpretation"].is_array());
}

#[test]
fn host_snapshot_human_output_is_readable() {
    let output = bbt().args(["host", "snapshot"]).output().expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Host snapshot:"));
    assert!(stdout.contains("Kernel:"));
    assert!(stdout.contains("User:"));
    assert!(stdout.contains("CPU:"));
    assert!(stdout.contains("Memory:"));
    assert!(stdout.contains("Filesystems:"));
}
