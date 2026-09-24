use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn disk_filesystems_json_emits_schema_envelope() {
    let output = bbt()
        .args(["disk", "filesystems", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.disk.filesystems.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert!(json["data"]["warnings"].is_array());

    let filesystems = json["data"]["filesystems"]
        .as_array()
        .expect("filesystems array");
    assert!(!filesystems.is_empty());
    for filesystem in filesystems {
        assert!(filesystem["source"].is_string());
        assert!(filesystem["target"].is_string());
        assert!(filesystem["fstype"].is_string());
        assert!(filesystem["total_bytes"].is_u64() || filesystem["total_bytes"].is_null());
        assert!(filesystem["available_bytes"].is_u64() || filesystem["available_bytes"].is_null());
        assert!(filesystem["used_percent"].is_number() || filesystem["used_percent"].is_null());
    }
}

#[test]
fn disk_filesystems_json_filters_pseudo_filesystem_noise() {
    let output = bbt()
        .args(["disk", "filesystems", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let filesystems = json["data"]["filesystems"]
        .as_array()
        .expect("filesystems array");
    assert!(filesystems
        .iter()
        .all(|filesystem| !matches!(filesystem["fstype"].as_str(), Some("proc" | "sysfs"))));
}

#[test]
fn disk_filesystems_matches_host_snapshot_filesystem_list() {
    let filesystems_output = bbt()
        .args(["disk", "filesystems", "--json"])
        .output()
        .expect("run bbt disk filesystems");
    let snapshot_output = bbt()
        .args(["host", "snapshot", "--json"])
        .output()
        .expect("run bbt host snapshot");

    assert!(filesystems_output.status.success());
    assert!(snapshot_output.status.success());

    let filesystems: serde_json::Value =
        serde_json::from_slice(&filesystems_output.stdout).expect("valid filesystems json");
    let snapshot: serde_json::Value =
        serde_json::from_slice(&snapshot_output.stdout).expect("valid snapshot json");

    let targets = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_array()
            .expect("filesystems array")
            .iter()
            .map(|fs| fs["target"].as_str().expect("target string").to_owned())
            .collect()
    };

    assert_eq!(
        targets(&filesystems["data"]["filesystems"]),
        targets(&snapshot["data"]["filesystems"]),
        "disk filesystems and host snapshot should report identically filtered mounts"
    );
}

#[test]
fn disk_filesystems_agent_emits_agent_report() {
    let output = bbt()
        .args(["disk", "filesystems", "--agent"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "disk");
    assert_eq!(json["subject"]["command"], "filesystems");
    let facts = json["facts"].as_array().expect("facts array");
    assert!(facts.iter().any(|fact| fact["key"] == "filesystem_count"));
    assert!(!json["safe_next_steps"]
        .as_array()
        .expect("safe next steps")
        .is_empty());
    assert!(json["risky_next_steps"]
        .as_array()
        .expect("risky next steps")
        .is_empty());
}

#[test]
fn disk_filesystems_human_output_is_readable() {
    let output = bbt()
        .args(["disk", "filesystems"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Mounted filesystems:"));
    assert!(stdout.contains("TARGET"));
    assert!(stdout.contains("Safe next observations:"));
}

#[test]
fn disk_filesystems_yaml_emits_schema_envelope() {
    let output = bbt()
        .args(["disk", "filesystems", "--yaml"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("schema: bbt.disk.filesystems.v1"));
    assert!(stdout.contains("filesystems:"));
}

#[test]
fn disk_filesystems_rejects_path_argument() {
    let output = bbt()
        .args(["disk", "filesystems", "/"])
        .output()
        .expect("run bbt");

    assert!(!output.status.success());
}
