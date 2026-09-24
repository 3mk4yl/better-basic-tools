use std::process::Command;

fn make_usage_fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("root.txt"), "root").expect("write root");
    std::fs::write(temp.path().join("middle.txt"), vec![b'm'; 128]).expect("write middle");
    std::fs::write(temp.path().join("large.txt"), vec![b'l'; 4096]).expect("write large");
    let nested = temp.path().join("nested");
    std::fs::create_dir(&nested).expect("create nested");
    std::fs::write(nested.join("deep.txt"), vec![b'd'; 4096]).expect("write deep");
    temp
}

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn disk_usage_json_emits_schema_envelope_for_directory() {
    let output = bbt()
        .args(["disk", "usage", ".", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.disk.usage.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert_eq!(json["data"]["path"], ".");
    assert_eq!(json["data"]["exists"], true);
    assert!(json["data"]["total_bytes"].as_u64().expect("total bytes") > 0);
    assert!(json["data"]["entries_seen"].as_u64().expect("entries seen") > 0);
    assert!(json["data"]["largest_children"].is_array());
}

#[test]
fn disk_usage_agent_emits_agent_report() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();
    let output = bbt()
        .args(["disk", "usage", &path, "--agent"])
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
    assert_eq!(json["subject"]["command"], "usage");
    let facts = json["facts"].as_array().expect("facts array");
    assert!(facts.iter().any(|fact| fact["key"] == "total_bytes"));
    assert!(facts.iter().any(|fact| fact["key"] == "largest_child"));
}

#[test]
fn disk_usage_human_output_is_readable() {
    let output = bbt()
        .args(["disk", "usage", "crates/bbt-disk"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Disk usage:"));
    assert!(stdout.contains("Total:"));
    assert!(stdout.contains("Largest children:"));
}

#[test]
fn disk_usage_missing_path_reports_without_error() {
    let output = bbt()
        .args(["disk", "usage", "definitely-not-a-bbt-path", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["exists"], false);
    assert_eq!(json["data"]["total_bytes"], 0);
    assert_eq!(json["data"]["errors"][0]["kind"], "not-found");
}

#[test]
fn disk_usage_top_limits_largest_children() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();
    let output = bbt()
        .args(["disk", "usage", &path, "--top", "2", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["top_limit"], 2);
    assert_eq!(
        json["data"]["largest_children"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn disk_usage_depth_zero_reports_recursive_root_total_without_children() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();

    let unlimited_output = bbt()
        .args(["disk", "usage", &path, "--json"])
        .output()
        .expect("run bbt unlimited");
    assert!(
        unlimited_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&unlimited_output.stderr)
    );
    let unlimited: serde_json::Value =
        serde_json::from_slice(&unlimited_output.stdout).expect("valid unlimited json");

    let depth_zero_output = bbt()
        .args(["disk", "usage", &path, "--depth", "0", "--json"])
        .output()
        .expect("run bbt depth zero");

    assert!(
        depth_zero_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&depth_zero_output.stderr)
    );

    let depth_zero: serde_json::Value =
        serde_json::from_slice(&depth_zero_output.stdout).expect("valid depth-zero json");
    assert_eq!(depth_zero["data"]["max_depth"], 0);
    assert_eq!(
        depth_zero["data"]["total_bytes"],
        unlimited["data"]["total_bytes"]
    );
    assert_eq!(
        depth_zero["data"]["apparent_bytes"],
        unlimited["data"]["apparent_bytes"]
    );
    assert_eq!(
        depth_zero["data"]["entries_seen"],
        unlimited["data"]["entries_seen"]
    );
    assert_eq!(
        depth_zero["data"]["largest_children"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn disk_usage_depth_one_reports_immediate_children_with_recursive_totals() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();

    let unlimited_output = bbt()
        .args(["disk", "usage", &path, "--json"])
        .output()
        .expect("run bbt unlimited");
    assert!(
        unlimited_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&unlimited_output.stderr)
    );
    let unlimited: serde_json::Value =
        serde_json::from_slice(&unlimited_output.stdout).expect("valid unlimited json");

    let depth_one_output = bbt()
        .args(["disk", "usage", &path, "--depth", "1", "--json"])
        .output()
        .expect("run bbt depth one");
    assert!(
        depth_one_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&depth_one_output.stderr)
    );
    let depth_one: serde_json::Value =
        serde_json::from_slice(&depth_one_output.stdout).expect("valid depth-one json");

    assert_eq!(depth_one["data"]["max_depth"], 1);
    assert_eq!(
        depth_one["data"]["total_bytes"], unlimited["data"]["total_bytes"],
        "--depth 1 should limit reported child depth, not recursive root totals"
    );
    assert_eq!(
        depth_one["data"]["apparent_bytes"], unlimited["data"]["apparent_bytes"],
        "--depth 1 should still include descendant apparent sizes in totals"
    );
    assert_eq!(
        depth_one["data"]["entries_seen"], unlimited["data"]["entries_seen"],
        "--depth 1 should still traverse descendants for recursive totals"
    );

    let depth_one_nested = depth_one["data"]["largest_children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["path"].as_str().unwrap().ends_with("nested"))
        .expect("nested child is reported at depth 1");
    let unlimited_nested = unlimited["data"]["largest_children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["path"].as_str().unwrap().ends_with("nested"))
        .expect("nested child is reported without depth limit");
    assert_eq!(
        depth_one_nested["total_bytes"], unlimited_nested["total_bytes"],
        "reported immediate child totals should include descendant usage"
    );
}

#[test]
fn disk_usage_depth_above_one_is_rejected_as_cli_usage_error() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();
    let output = bbt()
        .args(["disk", "usage", &path, "--depth", "2", "--json"])
        .output()
        .expect("run bbt depth two");

    assert!(
        !output.status.success(),
        "--depth 2 must fail as a usage error; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--depth 0"), "stderr: {stderr}");
    assert!(stderr.contains("--depth 1"), "stderr: {stderr}");
    assert!(stderr.contains("fs tree"), "stderr: {stderr}");
}

#[test]
fn disk_usage_one_filesystem_flag_is_reflected_in_json() {
    let fixture = make_usage_fixture();
    let path = fixture.path().to_string_lossy().into_owned();
    let output = bbt()
        .args(["disk", "usage", &path, "--one-filesystem", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["one_filesystem"], true);
    assert_eq!(json["data"]["skipped_different_filesystem"], 0);
}
