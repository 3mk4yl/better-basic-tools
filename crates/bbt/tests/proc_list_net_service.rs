use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn proc_list_json_emits_current_process() {
    let output = bbt()
        .args(["proc", "list", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.proc.list.v1");
    assert!(json["generated_at"].is_string());
    let processes = json["data"]["processes"]
        .as_array()
        .expect("processes array");
    assert!(processes
        .iter()
        .any(|process| { process["pid"].as_u64() == Some(std::process::id() as u64) }));
    assert!(json["data"]["process_count"].as_u64().unwrap_or_default() > 0);
}

#[test]
fn proc_list_agent_emits_agent_report() {
    let output = bbt()
        .args(["proc", "list", "--agent"])
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
    assert_eq!(json["subject"]["command"], "list");
    assert!(json["facts"]
        .as_array()
        .expect("facts")
        .iter()
        .any(|fact| { fact["key"] == "process_count" }));
}

#[test]
fn net_listeners_json_emits_listener_collection() {
    let output = bbt()
        .args(["net", "listeners", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.net.listeners.v1");
    assert!(json["data"]["listeners"].is_array());
    assert!(json["data"]["listener_count"].as_u64().is_some());
}

#[test]
fn net_listeners_agent_emits_agent_report() {
    let output = bbt()
        .args(["net", "listeners", "--agent"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "net");
    assert_eq!(json["subject"]["command"], "listeners");
}

#[test]
fn service_inspect_json_reports_missing_service_without_failing() {
    let output = bbt()
        .args([
            "service",
            "inspect",
            "bbt-definitely-missing-test-service.service",
            "--json",
        ])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.service.inspect.v1");
    assert_eq!(
        json["data"]["name"],
        "bbt-definitely-missing-test-service.service"
    );
    assert_eq!(json["data"]["exists"], false);
    assert!(json["data"]["warnings"].is_array());
}

#[test]
fn service_inspect_agent_emits_agent_report() {
    let output = bbt()
        .args([
            "service",
            "inspect",
            "bbt-definitely-missing-test-service.service",
            "--agent",
        ])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "service");
    assert_eq!(json["subject"]["command"], "inspect");
    assert!(json["facts"]
        .as_array()
        .expect("facts")
        .iter()
        .any(|fact| { fact["key"] == "service_exists" }));
}
