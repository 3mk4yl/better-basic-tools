use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Command;

fn bbt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bbt"))
}

/// Three directory levels below the root, mixed file sizes, a symlink to a
/// directory, and a dangling symlink — 10 lstat entries in total.
fn tree_fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    std::fs::write(root.join("top.txt"), vec![b'a'; 5]).expect("write top");
    let level1 = root.join("level1");
    std::fs::create_dir(&level1).expect("level1");
    std::fs::write(level1.join("mid.bin"), vec![b'b'; 4096]).expect("write mid");
    let level2 = level1.join("level2");
    std::fs::create_dir(&level2).expect("level2");
    std::fs::write(level2.join("deep.txt"), vec![b'c'; 300]).expect("write deep");
    let level3 = level2.join("level3");
    std::fs::create_dir(&level3).expect("level3");
    std::fs::write(level3.join("deepest.txt"), vec![b'd'; 65]).expect("write deepest");
    std::os::unix::fs::symlink(&level1, root.join("dir-link")).expect("dir symlink");
    std::os::unix::fs::symlink(root.join("missing"), root.join("dangling"))
        .expect("dangling symlink");
    temp
}

/// Independent lstat-based recursive (allocated, apparent) totals.
fn expected_subtree_sizes(path: &Path) -> (u64, u64) {
    let metadata = std::fs::symlink_metadata(path).expect("lstat");
    let mut total = metadata.blocks() * 512;
    let mut apparent = metadata.len();
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).expect("read dir") {
            let (child_total, child_apparent) =
                expected_subtree_sizes(&entry.expect("dir entry").path());
            total += child_total;
            apparent += child_apparent;
        }
    }
    (total, apparent)
}

fn run_json(args: &[&str]) -> serde_json::Value {
    let output = bbt().args(args).arg("--json").output().expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("valid json")
}

#[test]
fn fs_tree_json_emits_schema_envelope_with_recursive_totals() {
    let temp = tree_fixture();
    let path = temp.path().to_string_lossy().into_owned();
    let (expected_total, expected_apparent) = expected_subtree_sizes(temp.path());

    let json = run_json(&["fs", "tree", &path]);

    assert_eq!(json["schema"], "bbt.fs.tree.v1");
    assert!(json["generated_at"].is_string());
    assert!(json["command"]["argv"].is_array());
    assert!(json["command"]["argv_bytes_hex"].is_array());
    assert!(json["command"]["cwd"].is_string());
    assert!(json["command"]["effective_uid"].is_number());
    assert_eq!(json["data"]["exists"], true);
    assert_eq!(json["data"]["max_depth"], serde_json::Value::Null);
    assert_eq!(json["data"]["one_filesystem"], false);
    assert_eq!(json["data"]["entries_seen"], 10);
    assert_eq!(json["data"]["max_depth_reached"], 4);
    assert_eq!(json["data"]["skipped_different_filesystem"], 0);
    assert!(json["data"]["warnings"]
        .as_array()
        .expect("warnings")
        .is_empty());

    let root = &json["data"]["root"];
    assert_eq!(root["kind"], "directory");
    assert_eq!(root["total_bytes"], expected_total);
    assert_eq!(root["apparent_bytes"], expected_apparent);
    assert_eq!(root["entries_seen"], 10);
    assert!(root["name_bytes_hex"].is_string());

    let children = root["children"].as_array().expect("children array");
    let names: Vec<_> = children
        .iter()
        .map(|child| child["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["dangling", "dir-link", "level1", "top.txt"]);

    let (level1_total, level1_apparent) = expected_subtree_sizes(&temp.path().join("level1"));
    let level1 = &children[2];
    assert_eq!(level1["kind"], "directory");
    assert_eq!(level1["total_bytes"], level1_total);
    assert_eq!(level1["apparent_bytes"], level1_apparent);
    assert_eq!(level1["entries_seen"], 6);
    assert!(level1["children"].is_array());
}

#[test]
fn fs_tree_depth_zero_keeps_recursive_root_total_without_children() {
    let temp = tree_fixture();
    let path = temp.path().to_string_lossy().into_owned();
    let (expected_total, expected_apparent) = expected_subtree_sizes(temp.path());

    let json = run_json(&["fs", "tree", &path, "--depth", "0"]);

    assert_eq!(json["data"]["max_depth"], 0);
    let root = &json["data"]["root"];
    assert_eq!(
        root["total_bytes"], expected_total,
        "--depth 0 should limit reporting, not recursive root totals"
    );
    assert_eq!(root["apparent_bytes"], expected_apparent);
    assert_eq!(json["data"]["entries_seen"], 10);
    assert!(
        root.get("children").is_none(),
        "root children must be omitted at --depth 0"
    );
}

#[test]
fn fs_tree_depth_one_reports_children_with_full_recursive_totals() {
    let temp = tree_fixture();
    let path = temp.path().to_string_lossy().into_owned();
    let (level1_total, level1_apparent) = expected_subtree_sizes(&temp.path().join("level1"));

    let json = run_json(&["fs", "tree", &path, "--depth", "1"]);

    assert_eq!(json["data"]["max_depth"], 1);
    let children = json["data"]["root"]["children"]
        .as_array()
        .expect("children at depth 1");
    let level1 = children
        .iter()
        .find(|child| child["name"] == "level1")
        .expect("level1 node");
    assert_eq!(
        level1["total_bytes"], level1_total,
        "depth-limited nodes still carry full recursive subtree totals"
    );
    assert_eq!(level1["apparent_bytes"], level1_apparent);
    assert!(
        level1.get("children").is_none(),
        "children below the depth cutoff must be omitted"
    );
}

#[test]
fn fs_tree_reports_directory_symlink_as_leaf() {
    let temp = tree_fixture();
    let path = temp.path().to_string_lossy().into_owned();

    let json = run_json(&["fs", "tree", &path]);

    let children = json["data"]["root"]["children"]
        .as_array()
        .expect("children");
    let dir_link = children
        .iter()
        .find(|child| child["name"] == "dir-link")
        .expect("dir-link node");
    assert_eq!(dir_link["kind"], "symlink");
    assert!(
        dir_link.get("children").is_none(),
        "symlinks to directories must not be recursed into"
    );
    assert!(dir_link.get("entries_seen").is_none());
}

#[test]
fn fs_tree_json_reports_missing_path_as_warning_with_exit_zero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");
    let path = missing.to_string_lossy().into_owned();

    let json = run_json(&["fs", "tree", &path]);

    assert_eq!(json["data"]["exists"], false);
    assert_eq!(json["data"]["root"], serde_json::Value::Null);
    assert_eq!(json["data"]["entries_seen"], 0);
    assert_eq!(json["data"]["max_depth_reached"], serde_json::Value::Null);
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
fn fs_tree_json_on_file_reports_single_leaf_node_with_warning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("plain.txt");
    std::fs::write(&file, "plain").expect("write fixture");
    let path = file.to_string_lossy().into_owned();

    let json = run_json(&["fs", "tree", &path]);

    assert_eq!(json["data"]["exists"], true);
    let root = &json["data"]["root"];
    assert_eq!(root["kind"], "regular-file");
    assert_eq!(root["apparent_bytes"], 5);
    assert!(root.get("children").is_none());
    assert_eq!(json["data"]["entries_seen"], 1);
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
fn fs_tree_path_defaults_to_current_directory() {
    let temp = tree_fixture();

    let output = bbt()
        .args(["fs", "tree", "--json"])
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
    assert_eq!(json["data"]["entries_seen"], 10);
}

#[test]
fn fs_tree_one_filesystem_flag_is_reflected_in_json() {
    let temp = tree_fixture();
    let path = temp.path().to_string_lossy().into_owned();

    let json = run_json(&["fs", "tree", &path, "--one-filesystem"]);

    assert_eq!(json["data"]["one_filesystem"], true);
    assert_eq!(json["data"]["skipped_different_filesystem"], 0);
    assert_eq!(json["data"]["entries_seen"], 10);
}

#[test]
fn fs_tree_agent_emits_agent_report_with_read_only_steps_only() {
    let temp = tree_fixture();

    let output = bbt()
        .arg("fs")
        .arg("tree")
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
    assert_eq!(json["subject"]["command"], "tree");
    let facts = json["facts"].as_array().expect("facts array");
    for key in [
        "path_exists",
        "total_bytes",
        "entries_seen",
        "max_depth_reached",
        "largest_subtree",
    ] {
        assert!(
            facts.iter().any(|fact| fact["key"] == key),
            "missing fact {key}"
        );
    }
    let largest = facts
        .iter()
        .find(|fact| fact["key"] == "largest_subtree")
        .expect("largest_subtree fact");
    assert_eq!(largest["value"]["name"], "level1");
    let safe_steps = json["safe_next_steps"].as_array().expect("safe next steps");
    assert!(!safe_steps.is_empty());
    assert!(safe_steps.iter().all(|step| step["risk"] == "read-only"));
    assert!(json["risky_next_steps"]
        .as_array()
        .expect("risky next steps")
        .is_empty());
}

#[test]
fn fs_tree_human_output_draws_connectors_with_size_annotations() {
    let temp = tree_fixture();

    let output = bbt()
        .arg("fs")
        .arg("tree")
        .arg(temp.path())
        .output()
        .expect("run bbt");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Directory tree:"));
    assert!(stdout.contains("Entries seen:        10"));
    assert!(stdout.contains("├── "));
    assert!(stdout.contains("└── "));
    assert!(stdout.contains("│   "));
    assert!(stdout.contains("level1  ["));
    assert!(stdout.contains("Safe next observations:"));
}

#[test]
fn fs_tree_yaml_emits_schema_envelope() {
    let temp = tree_fixture();

    let output = bbt()
        .arg("fs")
        .arg("tree")
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
    assert!(stdout.contains("schema: bbt.fs.tree.v1"));
    assert!(stdout.contains("root:"));
    assert!(stdout.contains("entries_seen: 10"));
}
