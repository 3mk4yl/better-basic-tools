use std::path::PathBuf;
use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
}

fn assert_schema_file_matches_command(schema_file: &str, expected_schema_id: &str, args: &[&str]) {
    let schema_path = schemas_dir().join(schema_file);
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", schema_path.display()));
    let schema_json: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema file is valid JSON");
    assert_eq!(
        schema_json["properties"]["schema"]["const"], expected_schema_id,
        "{} should declare the command schema const",
        schema_file
    );

    let output = bbt().args(args).output().expect("run bbt");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let command_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("command output is valid JSON");
    assert_eq!(command_json["schema"], expected_schema_id);
}

#[test]
fn every_json_command_has_a_matching_schema_file() {
    assert_schema_file_matches_command(
        "host-snapshot.v1.schema.json",
        "bbt.host.snapshot.v1",
        &["host", "snapshot", "--json"],
    );
    assert_schema_file_matches_command(
        "fs-inspect.v1.schema.json",
        "bbt.fs.inspect.v1",
        &["fs", "inspect", "Cargo.toml", "--json"],
    );
    assert_schema_file_matches_command(
        "fs-list.v1.schema.json",
        "bbt.fs.list.v1",
        &["fs", "list", ".", "--json"],
    );
    assert_schema_file_matches_command(
        "fs-tree.v1.schema.json",
        "bbt.fs.tree.v1",
        &["fs", "tree", "docs", "--depth", "1", "--json"],
    );
    assert_schema_file_matches_command(
        "disk-usage.v1.schema.json",
        "bbt.disk.usage.v1",
        &["disk", "usage", ".", "--json"],
    );
    assert_schema_file_matches_command(
        "disk-filesystems.v1.schema.json",
        "bbt.disk.filesystems.v1",
        &["disk", "filesystems", "--json"],
    );
    assert_schema_file_matches_command(
        "disk-pressure.v1.schema.json",
        "bbt.disk.pressure.v1",
        &["disk", "pressure", "--json"],
    );
    assert_schema_file_matches_command(
        "proc-inspect.v1.schema.json",
        "bbt.proc.inspect.v1",
        &["proc", "inspect", &std::process::id().to_string(), "--json"],
    );
    assert_schema_file_matches_command(
        "proc-list.v1.schema.json",
        "bbt.proc.list.v1",
        &["proc", "list", "--json"],
    );
    assert_schema_file_matches_command(
        "net-listeners.v1.schema.json",
        "bbt.net.listeners.v1",
        &["net", "listeners", "--json"],
    );
    assert_schema_file_matches_command(
        "service-inspect.v1.schema.json",
        "bbt.service.inspect.v1",
        &[
            "service",
            "inspect",
            "bbt-definitely-missing-test-service.service",
            "--json",
        ],
    );
}
