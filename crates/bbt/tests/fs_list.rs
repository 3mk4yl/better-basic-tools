use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

fn mixed_fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("file.txt"), "hello").expect("write file");
    std::fs::set_permissions(
        temp.path().join("file.txt"),
        std::fs::Permissions::from_mode(0o640),
    )
    .expect("set mode");
    std::fs::create_dir(temp.path().join("subdir")).expect("subdir");
    std::os::unix::fs::symlink(temp.path().join("file.txt"), temp.path().join("link"))
        .expect("symlink");
    std::os::unix::fs::symlink(temp.path().join("missing"), temp.path().join("dangling"))
        .expect("dangling symlink");
    temp
}

fn run_json(path: &Path) -> serde_json::Value {
    let output = bbt()
        .arg("fs")
        .arg("list")
        .arg(path)
        .arg("--json")
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("valid json")
}

#[test]
fn fs_list_json_emits_schema_envelope_with_sorted_entries() {
    let temp = mixed_fixture();

    let json = run_json(temp.path());

    assert_eq!(json["schema"], "bbt.fs.list.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert!(json["command"]["cwd"].is_string());
    assert!(json["command"]["effective_uid"].is_number());
    assert_eq!(json["data"]["exists"], true);
    assert_eq!(json["data"]["is_directory"], true);
    assert_eq!(json["data"]["entry_count"], 4);
    assert!(json["data"]["warnings"]
        .as_array()
        .expect("warnings")
        .is_empty());

    let entries = json["data"]["entries"].as_array().expect("entries array");
    let names: Vec<_> = entries
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["dangling", "file.txt", "link", "subdir"]);

    for entry in entries {
        assert!(entry["name_bytes_hex"].is_string());
        assert!(entry["kind"].is_string());
        assert!(entry["size_bytes"].is_u64());
        assert!(entry["owner"]["uid"].is_u64());
        assert!(entry["group"]["gid"].is_u64());
        assert!(entry["mode"]["octal"].is_string());
        assert!(entry["mode"]["symbolic"].is_string());
        assert!(entry["modified"].is_string() || entry["modified"].is_null());
    }

    let file = &entries[1];
    assert_eq!(file["kind"], "regular-file");
    assert_eq!(file["size_bytes"], 5);
    assert_eq!(file["mode"]["octal"], "0640");
    assert_eq!(file["mode"]["symbolic"], "rw-r-----");
    assert_eq!(file["symlink_target"], serde_json::Value::Null);

    assert_eq!(entries[3]["kind"], "directory");
    assert_eq!(entries[2]["kind"], "symlink");
    assert_eq!(
        entries[2]["symlink_target"],
        temp.path().join("file.txt").display().to_string()
    );
    assert_eq!(entries[0]["kind"], "symlink");
    assert_eq!(
        entries[0]["symlink_target"],
        temp.path().join("missing").display().to_string()
    );
}

#[test]
fn fs_list_json_reports_missing_path_as_warning_with_exit_zero() {
    let temp = tempfile::tempdir().expect("tempdir");

    let json = run_json(&temp.path().join("missing"));

    assert_eq!(json["data"]["exists"], false);
    assert_eq!(json["data"]["is_directory"], false);
    assert_eq!(json["data"]["entry_count"], 0);
    assert!(json["data"]["entries"]
        .as_array()
        .expect("entries")
        .is_empty());
    assert!(json["data"]["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("does not exist")));
}

#[test]
fn fs_list_json_on_file_reports_the_path_itself_with_warning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("plain.txt");
    std::fs::write(&file, "plain").expect("write fixture");

    let json = run_json(&file);

    assert_eq!(json["data"]["exists"], true);
    assert_eq!(json["data"]["is_directory"], false);
    assert_eq!(json["data"]["entry_count"], 1);
    let entries = json["data"]["entries"].as_array().expect("entries");
    assert_eq!(entries[0]["name"], file.display().to_string());
    assert_eq!(entries[0]["kind"], "regular-file");
    assert!(json["data"]["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("is not a directory")));
}

#[test]
fn fs_list_path_defaults_to_current_directory() {
    let temp = mixed_fixture();

    let output = bbt()
        .args(["fs", "list", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["path"], ".");
    assert_eq!(json["data"]["entry_count"], 4);
}

#[test]
fn fs_list_agent_emits_agent_report_with_read_only_steps_only() {
    let temp = mixed_fixture();

    let output = bbt()
        .arg("fs")
        .arg("list")
        .arg(temp.path())
        .arg("--agent")
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.agent.report.v1");
    assert_eq!(json["subject"]["domain"], "fs");
    assert_eq!(json["subject"]["command"], "list");
    let facts = json["facts"].as_array().expect("facts array");
    for key in [
        "path_exists",
        "entry_count",
        "hidden_entry_count",
        "largest_entry",
    ] {
        assert!(
            facts.iter().any(|fact| fact["key"] == key),
            "missing fact {key}"
        );
    }
    let safe_steps = json["safe_next_steps"].as_array().expect("safe next steps");
    assert!(!safe_steps.is_empty());
    assert!(safe_steps.iter().all(|step| step["risk"] == "read-only"));
    assert!(json["risky_next_steps"]
        .as_array()
        .expect("risky next steps")
        .is_empty());
}

#[test]
fn fs_list_human_output_is_readable() {
    let temp = mixed_fixture();

    let output = bbt()
        .arg("fs")
        .arg("list")
        .arg(temp.path())
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Directory listing:"));
    assert!(stdout.contains("Entries: 4"));
    assert!(stdout.contains("MODE"));
    assert!(stdout.contains("file.txt"));
    assert!(stdout.contains("link -> "));
    assert!(stdout.contains("Safe next observations:"));
}

#[test]
fn fs_list_yaml_emits_schema_envelope() {
    let temp = mixed_fixture();

    let output = bbt()
        .arg("fs")
        .arg("list")
        .arg(temp.path())
        .arg("--yaml")
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("schema: bbt.fs.list.v1"));
    assert!(stdout.contains("entries:"));
    assert!(stdout.contains("entry_count: 4"));
}
