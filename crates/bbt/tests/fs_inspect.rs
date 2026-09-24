use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

#[test]
fn fs_inspect_json_emits_schema_envelope() {
    let output = bbt()
        .args(["fs", "inspect", "Cargo.toml", "--json"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["schema"], "bbt.fs.inspect.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert!(json["command"]["cwd"].is_string());
    assert!(json["command"]["effective_uid"].is_number());
    assert_eq!(json["data"]["exists"], true);
    assert_eq!(json["data"]["kind"], "regular-file");
    assert_eq!(json["data"]["path_bytes_hex"], "436172676f2e746f6d6c");
    assert!(json["data"]["mode"]["symbolic"].is_string());
}

#[test]
fn fs_inspect_agent_emits_agent_report() {
    let output = bbt()
        .args(["fs", "inspect", "Cargo.toml", "--agent"])
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
    assert_eq!(json["subject"]["command"], "inspect");
    assert!(json["facts"]
        .as_array()
        .expect("facts array")
        .iter()
        .any(|fact| fact["key"] == "path_exists"));
}

#[test]
fn fs_inspect_human_output_is_readable() {
    let output = bbt()
        .args(["fs", "inspect", "Cargo.toml"])
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("Type:"));
    assert!(stdout.contains("Access for current user:"));
}

#[test]
fn fs_inspect_json_preserves_non_utf8_path_and_argv_bytes() {
    let mut file_name = format!("bbt-nonutf-{}-", std::process::id()).into_bytes();
    file_name.push(0xff);

    let path = std::env::temp_dir().join(OsString::from_vec(file_name));
    std::fs::write(&path, "non-utf8 fixture").expect("write fixture");

    let output = bbt()
        .arg("fs")
        .arg("inspect")
        .arg(&path)
        .arg("--json")
        .output()
        .expect("run bbt");

    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let path_hex = bytes_hex(path.as_os_str().as_bytes());

    assert_eq!(json["data"]["path_bytes_hex"], path_hex);
    assert!(json["command"]["argv_bytes_hex"]
        .as_array()
        .expect("argv bytes")
        .iter()
        .any(|value| value.as_str() == Some(path_hex.as_str())));
}

#[test]
fn fs_inspect_permission_denied_returns_structured_warning_without_failing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let locked_dir = temp.path().join("locked");
    std::fs::create_dir(&locked_dir).expect("create locked dir");
    let hidden_path = locked_dir.join("hidden.txt");
    std::fs::write(&hidden_path, "hidden").expect("write hidden fixture");
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o000))
        .expect("lock dir");

    let output = bbt()
        .arg("fs")
        .arg("inspect")
        .arg(&hidden_path)
        .arg("--json")
        .output()
        .expect("run bbt");

    let _ = std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o700));

    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["data"]["exists"], false);
    assert_eq!(json["data"]["kind"], "unknown");
    assert!(json["data"]["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("Permission denied")));
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
