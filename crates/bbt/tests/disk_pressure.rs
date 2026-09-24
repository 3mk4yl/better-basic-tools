use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn disk_pressure_json_emits_schema_envelope() {
    let output = bbt()
        .args(["disk", "pressure", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.disk.pressure.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert!(json["data"]["available"].is_boolean());
    assert!(json["data"]["warnings"].is_array());

    for label in ["some", "full"] {
        let line = &json["data"][label];
        assert!(line.is_object() || line.is_null(), "{label} line shape");
        if line.is_object() {
            assert!(line["avg10"].is_number());
            assert!(line["avg60"].is_number());
            assert!(line["avg300"].is_number());
            assert!(line["total_stalled_usec"].is_u64());
        }
    }

    if json["data"]["available"] == false {
        assert!(!json["data"]["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty());
    }
}

#[test]
fn disk_pressure_agent_emits_agent_report_with_read_only_steps_only() {
    let output = bbt()
        .args(["disk", "pressure", "--agent"])
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
    assert_eq!(json["subject"]["command"], "pressure");
    let facts = json["facts"].as_array().expect("facts array");
    assert!(facts.iter().any(|fact| fact["key"] == "psi_available"));
    let safe_steps = json["safe_next_steps"].as_array().expect("safe next steps");
    assert!(!safe_steps.is_empty());
    assert!(safe_steps.iter().all(|step| step["risk"] == "read-only"));
    assert!(json["risky_next_steps"]
        .as_array()
        .expect("risky next steps")
        .is_empty());
}

#[test]
fn disk_pressure_human_output_is_readable() {
    let output = bbt().args(["disk", "pressure"]).output().expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Disk I/O pressure (PSI)"));
    assert!(stdout.contains("AVG10") || stdout.contains("PSI is unavailable"));
    assert!(stdout.contains("Safe next observations:"));
}

#[test]
fn disk_pressure_yaml_emits_schema_envelope() {
    let output = bbt()
        .args(["disk", "pressure", "--yaml"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("schema: bbt.disk.pressure.v1"));
    assert!(stdout.contains("available:"));
}

#[test]
fn disk_pressure_rejects_path_argument() {
    let output = bbt()
        .args(["disk", "pressure", "/"])
        .output()
        .expect("run bbt");

    assert!(!output.status.success());
}
