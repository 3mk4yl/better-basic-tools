//! Filesystem domain for Better Basic Tools.
//!
//! Commands: `bbt fs inspect PATH`, `bbt fs list PATH`, and `bbt fs tree PATH`.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::{CString, OsStr};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::time::SystemTime;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsInspection {
    pub path: String,
    pub path_bytes_hex: String,
    pub absolute_path: Option<String>,
    pub canonical_path: Option<String>,
    pub exists: bool,
    pub kind: FileKind,
    pub size_bytes: Option<u64>,
    pub owner: Option<Owner>,
    pub group: Option<Group>,
    pub mode: Option<Mode>,
    pub access: Access,
    pub symlink_target: Option<String>,
    pub timestamps: Timestamps,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    Missing,
    RegularFile,
    Directory,
    Symlink,
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub uid: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub gid: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mode {
    pub octal: String,
    pub symbolic: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Access {
    pub readable: Option<bool>,
    pub writable: Option<bool>,
    pub executable: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timestamps {
    pub modified: Option<String>,
    pub accessed: Option<String>,
    pub changed: Option<String>,
    pub created: Option<String>,
}

pub fn inspect_path(path: impl AsRef<Path>) -> io::Result<FsInspection> {
    let path = path.as_ref();
    let path_text = path.display().to_string();
    let path_bytes_hex = bytes_hex(path.as_os_str().as_bytes());
    let absolute_path = absolute_path(path)
        .ok()
        .map(|path| path.display().to_string());

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(FsInspection {
                path: path_text,
                path_bytes_hex,
                absolute_path,
                canonical_path: None,
                exists: false,
                kind: FileKind::Missing,
                size_bytes: None,
                owner: None,
                group: None,
                mode: None,
                access: Access::default(),
                symlink_target: None,
                timestamps: Timestamps::default(),
                warnings: Vec::new(),
            });
        }
        Err(error) => {
            return Ok(FsInspection {
                path: path_text,
                path_bytes_hex,
                absolute_path,
                canonical_path: None,
                exists: false,
                kind: FileKind::Unknown,
                size_bytes: None,
                owner: None,
                group: None,
                mode: None,
                access: Access::default(),
                symlink_target: None,
                timestamps: Timestamps::default(),
                warnings: vec![format!(
                    "metadata unavailable for {}: {error}",
                    path.display()
                )],
            });
        }
    };

    let file_type = metadata.file_type();
    let kind = classify_file_type(&file_type);
    let mode_bits = metadata.mode();
    let symlink_target = if kind == FileKind::Symlink {
        fs::read_link(path)
            .ok()
            .map(|target| target.display().to_string())
    } else {
        None
    };

    Ok(FsInspection {
        path: path_text,
        path_bytes_hex,
        absolute_path,
        canonical_path: fs::canonicalize(path)
            .ok()
            .map(|path| path.display().to_string()),
        exists: true,
        kind,
        size_bytes: Some(metadata.len()),
        owner: Some(Owner {
            uid: metadata.uid(),
            name: None,
        }),
        group: Some(Group {
            gid: metadata.gid(),
            name: None,
        }),
        mode: Some(Mode {
            octal: format!("{:04o}", mode_bits & 0o7777),
            symbolic: symbolic_mode(mode_bits),
        }),
        access: if kind == FileKind::Symlink {
            Access::default()
        } else {
            access_for_current_user(path)
        },
        symlink_target,
        timestamps: Timestamps {
            modified: metadata.modified().ok().and_then(format_system_time),
            accessed: metadata.accessed().ok().and_then(format_system_time),
            changed: Some(format_unix_timestamp(
                metadata.ctime(),
                metadata.ctime_nsec(),
            )),
            created: metadata.created().ok().and_then(format_system_time),
        },
        warnings: Vec::new(),
    })
}

pub fn agent_report(inspection: &FsInspection, generated_at: impl Into<String>) -> AgentReport {
    let severity = if !inspection.warnings.is_empty() {
        Severity::Warning
    } else if inspection.exists {
        Severity::Ok
    } else {
        Severity::Warning
    };
    let summary = if inspection.exists {
        format!("{} is a {}.", inspection.path, inspection.kind.label())
    } else if inspection.kind == FileKind::Missing {
        format!("{} does not exist.", inspection.path)
    } else {
        format!("{} could not be inspected.", inspection.path)
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("fs", "inspect", Some(inspection.path.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "path_exists",
        serde_json::json!(inspection.exists),
        if inspection.exists {
            Severity::Info
        } else {
            Severity::Warning
        },
        vec![if inspection.exists {
            format!("{} exists", inspection.path)
        } else if inspection.kind == FileKind::Missing {
            format!("{} was not found", inspection.path)
        } else {
            format!("{} could not be inspected", inspection.path)
        }],
    ));

    if inspection.exists {
        report = report
            .with_fact(Fact::new(
                "file_kind",
                serde_json::json!(inspection.kind),
                Severity::Info,
                vec![format!("filesystem type is {}", inspection.kind.label())],
            ))
            .with_fact(Fact::new(
                "current_user_readable",
                serde_json::json!(inspection.access.readable),
                if inspection.access.readable == Some(false) {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                vec!["checked read access for the current process user".to_owned()],
            ))
            .with_safe_next_step(NextStep::read_only(
                format!("bbt --json fs inspect -- {}", shell_quote(&inspection.path)),
                "Fetch complete raw filesystem metadata",
            ));
    }

    if !inspection.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} filesystem inspection warnings occurred; permissions or path races may limit completeness.",
            inspection.warnings.len()
        ));
    }

    report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsListing {
    pub path: String,
    pub path_bytes_hex: String,
    pub absolute_path: Option<String>,
    pub exists: bool,
    pub is_directory: bool,
    pub entry_count: usize,
    pub entries: Vec<FsListEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsListEntry {
    pub name: String,
    pub name_bytes_hex: String,
    pub kind: FileKind,
    pub size_bytes: u64,
    pub owner: Owner,
    pub group: Group,
    pub mode: Mode,
    pub modified: Option<String>,
    pub symlink_target: Option<String>,
}

pub fn list_directory(path: impl AsRef<Path>) -> FsListing {
    let path = path.as_ref();
    let mut listing = FsListing {
        path: path.display().to_string(),
        path_bytes_hex: bytes_hex(path.as_os_str().as_bytes()),
        absolute_path: absolute_path(path)
            .ok()
            .map(|path| path.display().to_string()),
        exists: false,
        is_directory: false,
        entry_count: 0,
        entries: Vec::new(),
        warnings: Vec::new(),
    };

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            listing
                .warnings
                .push(format!("{} does not exist", listing.path));
            return listing;
        }
        Err(error) => {
            listing.warnings.push(format!(
                "metadata unavailable for {}: {error}",
                listing.path
            ));
            return listing;
        }
    };

    listing.exists = true;

    if !metadata.is_dir() {
        listing.warnings.push(format!(
            "{} is not a directory; reporting the path itself as its only entry",
            listing.path
        ));
        listing
            .entries
            .push(list_entry(path, path.as_os_str(), &metadata));
        listing.entry_count = 1;
        return listing;
    }

    listing.is_directory = true;

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            listing
                .warnings
                .push(format!("cannot read directory {}: {error}", listing.path));
            return listing;
        }
    };

    let mut names = Vec::new();
    for entry in read_dir {
        match entry {
            Ok(entry) => names.push(entry.file_name()),
            Err(error) => listing.warnings.push(format!(
                "cannot read a directory entry in {}: {error}",
                listing.path
            )),
        }
    }
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

    for name in names {
        let entry_path = path.join(&name);
        match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => listing
                .entries
                .push(list_entry(&entry_path, &name, &metadata)),
            Err(error) => listing.warnings.push(format!(
                "metadata unavailable for {}: {error}",
                entry_path.display()
            )),
        }
    }

    listing.entry_count = listing.entries.len();
    listing
}

fn list_entry(path: &Path, name: &OsStr, metadata: &fs::Metadata) -> FsListEntry {
    let kind = classify_file_type(&metadata.file_type());
    let mode_bits = metadata.mode();

    FsListEntry {
        name: name.to_string_lossy().into_owned(),
        name_bytes_hex: bytes_hex(name.as_bytes()),
        kind,
        size_bytes: metadata.len(),
        owner: Owner {
            uid: metadata.uid(),
            name: None,
        },
        group: Group {
            gid: metadata.gid(),
            name: None,
        },
        mode: Mode {
            octal: format!("{:04o}", mode_bits & 0o7777),
            symbolic: symbolic_mode(mode_bits),
        },
        modified: metadata.modified().ok().and_then(format_system_time),
        symlink_target: if kind == FileKind::Symlink {
            fs::read_link(path)
                .ok()
                .map(|target| target.display().to_string())
        } else {
            None
        },
    }
}

pub fn list_agent_report(listing: &FsListing, generated_at: impl Into<String>) -> AgentReport {
    let severity = if !listing.exists || !listing.warnings.is_empty() {
        Severity::Warning
    } else {
        Severity::Ok
    };
    let summary = if listing.is_directory {
        format!("{} contains {} entries.", listing.path, listing.entry_count)
    } else if listing.exists {
        format!("{} is not a directory.", listing.path)
    } else {
        format!("{} does not exist.", listing.path)
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("fs", "list", Some(listing.path.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "path_exists",
        serde_json::json!(listing.exists),
        if listing.exists {
            Severity::Info
        } else {
            Severity::Warning
        },
        vec![if listing.exists {
            format!("{} exists", listing.path)
        } else {
            format!("{} was not found", listing.path)
        }],
    ));

    if listing.exists {
        report = report.with_fact(Fact::new(
            "is_directory",
            serde_json::json!(listing.is_directory),
            if listing.is_directory {
                Severity::Info
            } else {
                Severity::Warning
            },
            vec!["lstat file type of the listed path".to_owned()],
        ));
    }

    if listing.is_directory {
        report = report
            .with_fact(Fact::new(
                "entry_count",
                serde_json::json!(listing.entry_count),
                Severity::Info,
                vec!["immediate entries with readable metadata; non-recursive".to_owned()],
            ))
            .with_fact(Fact::new(
                "hidden_entry_count",
                serde_json::json!(listing
                    .entries
                    .iter()
                    .filter(|entry| entry.name.starts_with('.'))
                    .count()),
                Severity::Info,
                vec!["entries whose name starts with a dot".to_owned()],
            ));

        if let Some(largest) = listing.entries.iter().max_by_key(|entry| entry.size_bytes) {
            report = report.with_fact(Fact::new(
                "largest_entry",
                serde_json::json!({
                    "name": largest.name,
                    "kind": largest.kind,
                    "size_bytes": largest.size_bytes,
                }),
                Severity::Info,
                vec![
                    "largest immediate entry by lstat size; directory sizes are not recursive"
                        .to_owned(),
                ],
            ));
        }
    }

    if !listing.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} directory listing warnings occurred; permissions or path races may limit completeness.",
            listing.warnings.len()
        ));
    }

    let quoted_path = shell_quote(&listing.path);
    report = report.with_safe_next_step(NextStep::read_only(
        format!("bbt --json fs list -- {quoted_path}"),
        "Fetch the complete structured directory listing",
    ));

    if listing.is_directory {
        if let Some(largest) = listing.entries.iter().max_by_key(|entry| entry.size_bytes) {
            report = report.with_safe_next_step(NextStep::read_only(
                format!(
                    "bbt --agent fs inspect -- {}",
                    shell_quote(
                        &Path::new(&listing.path)
                            .join(&largest.name)
                            .display()
                            .to_string()
                    )
                ),
                "Inspect the largest immediate entry in more detail",
            ));
        }
        report = report.with_safe_next_step(NextStep::read_only(
            format!("bbt --agent disk usage --depth 1 -- {quoted_path}"),
            "Calculate recursive size totals below this directory",
        ));
    } else if listing.exists {
        report = report.with_safe_next_step(NextStep::read_only(
            format!("bbt --agent fs inspect -- {quoted_path}"),
            "Inspect this non-directory path's full metadata",
        ));
    }

    report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsTree {
    pub path: String,
    pub path_bytes_hex: String,
    pub absolute_path: Option<String>,
    pub exists: bool,
    pub max_depth: Option<u64>,
    pub one_filesystem: bool,
    pub root: Option<TreeNode>,
    pub entries_seen: u64,
    pub max_depth_reached: Option<u64>,
    pub skipped_different_filesystem: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub name_bytes_hex: String,
    pub kind: FileKind,
    pub total_bytes: u64,
    pub apparent_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries_seen: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeOptions {
    pub max_depth: Option<u64>,
    pub one_filesystem: bool,
}

impl TreeOptions {
    pub fn new(max_depth: Option<u64>, one_filesystem: bool) -> Self {
        Self {
            max_depth,
            one_filesystem,
        }
    }
}

pub fn tree(path: impl AsRef<Path>) -> FsTree {
    tree_with_options(path, TreeOptions::default())
}

pub fn tree_with_options(path: impl AsRef<Path>, options: TreeOptions) -> FsTree {
    let path = path.as_ref();
    let mut tree = FsTree {
        path: path.display().to_string(),
        path_bytes_hex: bytes_hex(path.as_os_str().as_bytes()),
        absolute_path: absolute_path(path)
            .ok()
            .map(|path| path.display().to_string()),
        exists: false,
        max_depth: options.max_depth,
        one_filesystem: options.one_filesystem,
        root: None,
        entries_seen: 0,
        max_depth_reached: None,
        skipped_different_filesystem: 0,
        warnings: Vec::new(),
    };

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            tree.warnings.push(format!("{} does not exist", tree.path));
            return tree;
        }
        Err(error) => {
            tree.warnings
                .push(format!("metadata unavailable for {}: {error}", tree.path));
            return tree;
        }
    };

    tree.exists = true;
    if !metadata.is_dir() {
        tree.warnings.push(format!(
            "{} is not a directory; reporting the path itself as a single leaf node",
            tree.path
        ));
    }

    let mut walk = TreeWalk {
        options,
        root_device: metadata.dev(),
        seen_inodes: HashSet::new(),
        visited_directories: HashSet::new(),
        entries_seen: 0,
        max_depth_reached: 0,
        skipped_different_filesystem: 0,
        warnings: Vec::new(),
    };
    let root = walk.build_node(path, path.as_os_str(), &metadata, 0);

    tree.root = Some(root);
    tree.entries_seen = walk.entries_seen;
    tree.max_depth_reached = Some(walk.max_depth_reached);
    tree.skipped_different_filesystem = walk.skipped_different_filesystem;
    tree.warnings.append(&mut walk.warnings);
    tree
}

struct TreeWalk {
    options: TreeOptions,
    root_device: u64,
    seen_inodes: HashSet<(u64, u64)>,
    visited_directories: HashSet<(u64, u64)>,
    entries_seen: u64,
    max_depth_reached: u64,
    skipped_different_filesystem: u64,
    warnings: Vec<String>,
}

impl TreeWalk {
    fn build_node(
        &mut self,
        path: &Path,
        name: &OsStr,
        metadata: &fs::Metadata,
        depth: u64,
    ) -> TreeNode {
        self.entries_seen += 1;
        self.max_depth_reached = self.max_depth_reached.max(depth);

        if metadata.is_dir()
            && !self
                .visited_directories
                .insert((metadata.dev(), metadata.ino()))
        {
            // A same-device revisit (e.g. a bind mount forming a cycle) must not
            // recurse forever or double-count an already-summed subtree.
            self.warnings.push(format!(
                "{} was already visited during this traversal; skipped to avoid double counting",
                path.display()
            ));
            return TreeNode {
                name: name.to_string_lossy().into_owned(),
                name_bytes_hex: bytes_hex(name.as_bytes()),
                kind: classify_file_type(&metadata.file_type()),
                total_bytes: 0,
                apparent_bytes: 0,
                entries_seen: None,
                children: None,
            };
        }

        let already_seen =
            !metadata.is_dir() && !self.seen_inodes.insert((metadata.dev(), metadata.ino()));
        let mut node = TreeNode {
            name: name.to_string_lossy().into_owned(),
            name_bytes_hex: bytes_hex(name.as_bytes()),
            kind: classify_file_type(&metadata.file_type()),
            total_bytes: if already_seen {
                0
            } else {
                allocated_bytes(metadata)
            },
            apparent_bytes: if already_seen { 0 } else { metadata.len() },
            entries_seen: None,
            children: None,
        };

        if !metadata.is_dir() {
            return node;
        }

        let report_children = match self.options.max_depth {
            None => true,
            Some(max_depth) => depth < max_depth,
        };
        let mut children = if report_children {
            Some(Vec::new())
        } else {
            None
        };
        let mut subtree_entries = 1u64;

        match fs::read_dir(path) {
            Ok(read_dir) => {
                let mut names = Vec::new();
                for entry in read_dir {
                    match entry {
                        Ok(entry) => names.push(entry.file_name()),
                        Err(error) => self.warnings.push(format!(
                            "cannot read a directory entry in {}: {error}",
                            path.display()
                        )),
                    }
                }
                names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

                for child_name in names {
                    let child_path = path.join(&child_name);
                    let child_metadata = match fs::symlink_metadata(&child_path) {
                        Ok(child_metadata) => child_metadata,
                        Err(error) => {
                            self.warnings.push(format!(
                                "metadata unavailable for {}: {error}",
                                child_path.display()
                            ));
                            continue;
                        }
                    };
                    if self.options.one_filesystem && child_metadata.dev() != self.root_device {
                        self.skipped_different_filesystem += 1;
                        continue;
                    }
                    let child_node =
                        self.build_node(&child_path, &child_name, &child_metadata, depth + 1);
                    subtree_entries += child_node.entries_seen.unwrap_or(1);
                    node.total_bytes = node.total_bytes.saturating_add(child_node.total_bytes);
                    node.apparent_bytes = node
                        .apparent_bytes
                        .saturating_add(child_node.apparent_bytes);
                    if let Some(children) = children.as_mut() {
                        children.push(child_node);
                    }
                }
            }
            Err(error) => self
                .warnings
                .push(format!("cannot read directory {}: {error}", path.display())),
        }

        node.entries_seen = Some(subtree_entries);
        node.children = children;
        node
    }
}

pub fn tree_agent_report(tree: &FsTree, generated_at: impl Into<String>) -> AgentReport {
    let severity = if !tree.exists || !tree.warnings.is_empty() {
        Severity::Warning
    } else {
        Severity::Ok
    };
    let summary = match &tree.root {
        Some(root) if root.kind == FileKind::Directory => format!(
            "{} holds {} across {} entries.",
            tree.path,
            human_bytes(root.total_bytes),
            tree.entries_seen
        ),
        Some(root) => format!("{} is a {}, not a directory.", tree.path, root.kind.label()),
        None => format!("{} does not exist.", tree.path),
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("fs", "tree", Some(tree.path.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "path_exists",
        serde_json::json!(tree.exists),
        if tree.exists {
            Severity::Info
        } else {
            Severity::Warning
        },
        vec![if tree.exists {
            format!("{} exists", tree.path)
        } else {
            format!("{} was not found", tree.path)
        }],
    ));

    if let Some(root) = &tree.root {
        report = report
            .with_fact(Fact::new(
                "total_bytes",
                serde_json::json!(root.total_bytes),
                Severity::Info,
                vec!["recursively calculated allocated bytes for the root node".to_owned()],
            ))
            .with_fact(Fact::new(
                "entries_seen",
                serde_json::json!(tree.entries_seen),
                Severity::Info,
                vec!["entries observed during the full recursive walk; --depth limits reporting only".to_owned()],
            ))
            .with_fact(Fact::new(
                "max_depth_reached",
                serde_json::json!(tree.max_depth_reached),
                Severity::Info,
                vec![format!(
                    "deepest level observed during traversal; requested reporting depth: {}",
                    tree.max_depth
                        .map(|depth| depth.to_string())
                        .unwrap_or_else(|| "unlimited".to_owned())
                )],
            ));

        let largest_subtree = root
            .children
            .as_ref()
            .and_then(|children| children.iter().max_by_key(|child| child.total_bytes));
        if let Some(largest) = largest_subtree {
            report = report.with_fact(Fact::new(
                "largest_subtree",
                serde_json::json!({
                    "name": largest.name,
                    "kind": largest.kind,
                    "total_bytes": largest.total_bytes,
                    "apparent_bytes": largest.apparent_bytes,
                }),
                Severity::Info,
                vec![
                    "largest immediate child by recursively calculated allocated bytes".to_owned(),
                ],
            ));
        }
    }

    if !tree.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} tree traversal warnings occurred; permissions or path races may limit completeness.",
            tree.warnings.len()
        ));
    }

    let quoted_path = shell_quote(&tree.path);
    report = report.with_safe_next_step(NextStep::read_only(
        format!("bbt --json fs tree -- {quoted_path}"),
        "Fetch the complete structured directory tree",
    ));

    if let Some(root) = &tree.root {
        if root.kind == FileKind::Directory {
            if let Some(largest) = root
                .children
                .as_ref()
                .and_then(|children| children.iter().max_by_key(|child| child.total_bytes))
                .filter(|child| child.kind == FileKind::Directory)
            {
                report = report.with_safe_next_step(NextStep::read_only(
                    format!(
                        "bbt --agent disk usage --depth 1 -- {}",
                        shell_quote(
                            &Path::new(&tree.path)
                                .join(&largest.name)
                                .display()
                                .to_string()
                        )
                    ),
                    "Break down the largest subtree in more detail",
                ));
            }
            report = report.with_safe_next_step(NextStep::read_only(
                format!("bbt --agent fs list -- {quoted_path}"),
                "List this directory's flat contents with per-entry ownership",
            ));
        } else {
            report = report.with_safe_next_step(NextStep::read_only(
                format!("bbt --agent fs inspect -- {quoted_path}"),
                "Inspect this non-directory path's full metadata",
            ));
        }
    }

    report
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn allocated_bytes(metadata: &fs::Metadata) -> u64 {
    metadata.blocks().saturating_mul(512)
}

impl FileKind {
    pub fn label(self) -> &'static str {
        match self {
            FileKind::Missing => "missing path",
            FileKind::RegularFile => "regular file",
            FileKind::Directory => "directory",
            FileKind::Symlink => "symbolic link",
            FileKind::Socket => "socket",
            FileKind::Fifo => "FIFO",
            FileKind::BlockDevice => "block device",
            FileKind::CharDevice => "character device",
            FileKind::Unknown => "unknown filesystem object",
        }
    }

    pub fn indicator(self) -> char {
        match self {
            FileKind::RegularFile => '-',
            FileKind::Directory => 'd',
            FileKind::Symlink => 'l',
            FileKind::Socket => 's',
            FileKind::Fifo => 'p',
            FileKind::BlockDevice => 'b',
            FileKind::CharDevice => 'c',
            FileKind::Missing | FileKind::Unknown => '?',
        }
    }
}

pub fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn absolute_path(path: &Path) -> io::Result<std::path::PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn classify_file_type(file_type: &fs::FileType) -> FileKind {
    if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_file() {
        FileKind::RegularFile
    } else if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_socket() {
        FileKind::Socket
    } else if file_type.is_fifo() {
        FileKind::Fifo
    } else if file_type.is_block_device() {
        FileKind::BlockDevice
    } else if file_type.is_char_device() {
        FileKind::CharDevice
    } else {
        FileKind::Unknown
    }
}

fn symbolic_mode(mode: u32) -> String {
    let mut chars = ['-'; 9];
    for (index, bit, marker) in [
        (0, 0o400, 'r'),
        (1, 0o200, 'w'),
        (2, 0o100, 'x'),
        (3, 0o040, 'r'),
        (4, 0o020, 'w'),
        (5, 0o010, 'x'),
        (6, 0o004, 'r'),
        (7, 0o002, 'w'),
        (8, 0o001, 'x'),
    ] {
        if mode & bit != 0 {
            chars[index] = marker;
        }
    }

    chars[2] = special_exec(chars[2], mode & 0o4000 != 0, 's', 'S');
    chars[5] = special_exec(chars[5], mode & 0o2000 != 0, 's', 'S');
    chars[8] = special_exec(chars[8], mode & 0o1000 != 0, 't', 'T');

    chars.into_iter().collect()
}

fn special_exec(
    current: char,
    special: bool,
    executable_marker: char,
    non_executable_marker: char,
) -> char {
    if !special {
        current
    } else if current == 'x' {
        executable_marker
    } else {
        non_executable_marker
    }
}

fn access_for_current_user(path: &Path) -> Access {
    let Some(path) = CString::new(path.as_os_str().as_bytes()).ok() else {
        return Access::default();
    };

    Access {
        readable: Some(check_access(&path, libc::R_OK)),
        writable: Some(check_access(&path, libc::W_OK)),
        executable: Some(check_access(&path, libc::X_OK)),
    }
}

fn check_access(path: &CString, mode: i32) -> bool {
    unsafe { libc::access(path.as_ptr(), mode) == 0 }
}

fn format_system_time(time: SystemTime) -> Option<String> {
    OffsetDateTime::from(time).format(&Rfc3339).ok()
}

fn format_unix_timestamp(seconds: i64, nanoseconds: i64) -> String {
    match OffsetDateTime::from_unix_timestamp(seconds)
        .and_then(|time| time.replace_nanosecond(nanoseconds as u32))
    {
        Ok(time) => time
            .format(&Rfc3339)
            .unwrap_or_else(|_| format!("{seconds}.{nanoseconds:09}Z")),
        Err(_) => format!("{seconds}.{nanoseconds:09}Z"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    #[test]
    fn inspects_regular_file_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sample.txt");
        fs::write(&path, "hello").expect("write fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("set mode");

        let inspected = inspect_path(&path).expect("inspect path");

        assert!(inspected.exists);
        assert_eq!(inspected.kind, FileKind::RegularFile);
        assert_eq!(inspected.size_bytes, Some(5));
        assert_eq!(inspected.mode.as_ref().expect("mode").octal, "0640");
        assert_eq!(inspected.mode.as_ref().expect("mode").symbolic, "rw-r-----");
        assert_eq!(
            inspected.owner.as_ref().expect("owner").uid,
            fs::metadata(&path).expect("metadata").uid()
        );
        assert!(inspected.timestamps.modified.is_some());
    }

    #[test]
    fn reports_missing_path_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("missing.txt");

        let inspected = inspect_path(&path).expect("inspect missing path");

        assert!(!inspected.exists);
        assert_eq!(inspected.kind, FileKind::Missing);
        assert_eq!(inspected.path, path.display().to_string());
    }

    #[test]
    fn reports_symlink_target() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "target").expect("write fixture");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let inspected = inspect_path(&link).expect("inspect symlink");

        assert_eq!(inspected.kind, FileKind::Symlink);
        assert_eq!(inspected.symlink_target, Some(target.display().to_string()));
    }

    fn mixed_fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("file.txt"), "hello").expect("write file");
        fs::set_permissions(
            temp.path().join("file.txt"),
            fs::Permissions::from_mode(0o640),
        )
        .expect("set mode");
        fs::create_dir(temp.path().join("subdir")).expect("subdir");
        std::os::unix::fs::symlink(temp.path().join("file.txt"), temp.path().join("link"))
            .expect("symlink");
        std::os::unix::fs::symlink(temp.path().join("missing"), temp.path().join("dangling"))
            .expect("dangling symlink");
        temp
    }

    #[test]
    fn lists_mixed_directory_sorted_by_name_without_following_symlinks() {
        let temp = mixed_fixture();

        let listing = list_directory(temp.path());

        assert!(listing.exists);
        assert!(listing.is_directory);
        assert!(listing.warnings.is_empty());
        assert_eq!(listing.entry_count, 4);
        assert_eq!(
            listing
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["dangling", "file.txt", "link", "subdir"]
        );

        let file = &listing.entries[1];
        assert_eq!(file.kind, FileKind::RegularFile);
        assert_eq!(file.size_bytes, 5);
        assert_eq!(file.mode.octal, "0640");
        assert_eq!(file.mode.symbolic, "rw-r-----");
        assert_eq!(file.name_bytes_hex, bytes_hex(b"file.txt"));
        assert_eq!(
            file.owner.uid,
            fs::metadata(temp.path().join("file.txt"))
                .expect("metadata")
                .uid()
        );
        assert_eq!(
            file.group.gid,
            fs::metadata(temp.path().join("file.txt"))
                .expect("metadata")
                .gid()
        );
        assert!(file.modified.is_some());
        assert!(file.symlink_target.is_none());

        let subdir = &listing.entries[3];
        assert_eq!(subdir.kind, FileKind::Directory);

        let link = &listing.entries[2];
        assert_eq!(link.kind, FileKind::Symlink);
        assert_eq!(
            link.symlink_target,
            Some(temp.path().join("file.txt").display().to_string())
        );
        assert_eq!(
            link.size_bytes,
            fs::symlink_metadata(temp.path().join("link"))
                .expect("lstat")
                .len()
        );

        let dangling = &listing.entries[0];
        assert_eq!(dangling.kind, FileKind::Symlink);
        assert_eq!(
            dangling.symlink_target,
            Some(temp.path().join("missing").display().to_string())
        );
    }

    #[test]
    fn lists_non_utf8_entry_name_as_bytes_hex_without_panicking() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut raw_name = b"weird-".to_vec();
        raw_name.push(0xff);
        let name = std::ffi::OsString::from_vec(raw_name.clone());
        fs::write(temp.path().join(&name), "payload").expect("write non-utf8 fixture");

        let listing = list_directory(temp.path());

        assert_eq!(listing.entry_count, 1);
        assert!(listing.warnings.is_empty());
        let entry = &listing.entries[0];
        assert_eq!(entry.name_bytes_hex, bytes_hex(&raw_name));
        assert_eq!(entry.name, name.to_string_lossy());
        assert_eq!(entry.size_bytes, 7);
    }

    #[test]
    fn list_reports_missing_path_as_warning_with_exit_worthy_structure() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing");

        let listing = list_directory(&missing);

        assert!(!listing.exists);
        assert!(!listing.is_directory);
        assert_eq!(listing.entry_count, 0);
        assert!(listing.entries.is_empty());
        assert_eq!(listing.warnings.len(), 1);
        assert!(listing.warnings[0].contains("does not exist"));
    }

    #[test]
    fn list_on_non_directory_reports_the_path_itself_as_single_entry() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("plain.txt");
        fs::write(&file, "plain").expect("write fixture");

        let listing = list_directory(&file);

        assert!(listing.exists);
        assert!(!listing.is_directory);
        assert_eq!(listing.entry_count, 1);
        assert_eq!(listing.entries[0].name, file.display().to_string());
        assert_eq!(listing.entries[0].kind, FileKind::RegularFile);
        assert_eq!(listing.entries[0].size_bytes, 5);
        assert_eq!(listing.warnings.len(), 1);
        assert!(listing.warnings[0].contains("is not a directory"));
    }

    #[test]
    fn list_degrades_to_warning_when_directory_is_unreadable() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let temp = tempfile::tempdir().expect("tempdir");
        let locked = temp.path().join("locked");
        fs::create_dir(&locked).expect("create locked dir");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock dir");

        let listing = list_directory(&locked);

        let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o700));

        assert!(listing.exists);
        assert!(listing.is_directory);
        assert!(listing.entries.is_empty());
        assert_eq!(listing.entry_count, 0);
        assert!(listing
            .warnings
            .iter()
            .any(|warning| warning.contains("cannot read directory")));
    }

    #[test]
    fn list_skips_entries_with_unreadable_metadata_and_keeps_the_rest() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        // Readable but non-searchable directory: read_dir succeeds, but
        // lstat on every child fails with permission denied.
        let temp = tempfile::tempdir().expect("tempdir");
        let readable_only = temp.path().join("readable-only");
        fs::create_dir(&readable_only).expect("create dir");
        fs::write(readable_only.join("blocked.txt"), "blocked").expect("write fixture");
        fs::set_permissions(&readable_only, fs::Permissions::from_mode(0o444))
            .expect("remove search bit");

        let listing = list_directory(&readable_only);

        let _ = fs::set_permissions(&readable_only, fs::Permissions::from_mode(0o700));

        assert!(listing.is_directory);
        assert!(listing.entries.is_empty());
        assert_eq!(listing.entry_count, 0);
        assert!(listing
            .warnings
            .iter()
            .any(|warning| warning.contains("blocked.txt")
                && warning.contains("metadata unavailable")));
    }

    #[test]
    fn list_agent_report_carries_triage_facts_and_read_only_steps_only() {
        let temp = mixed_fixture();

        let listing = list_directory(temp.path());
        let report = list_agent_report(&listing, "2026-01-01T00:00:00Z");

        assert_eq!(report.subject.domain, "fs");
        assert_eq!(report.subject.command, "list");
        for key in [
            "path_exists",
            "is_directory",
            "entry_count",
            "hidden_entry_count",
            "largest_entry",
        ] {
            assert!(
                report.facts.iter().any(|fact| fact.key == key),
                "missing fact {key}"
            );
        }
        let largest = report
            .facts
            .iter()
            .find(|fact| fact.key == "largest_entry")
            .expect("largest_entry fact");
        let expected_largest = listing
            .entries
            .iter()
            .max_by_key(|entry| entry.size_bytes)
            .expect("non-empty listing");
        assert_eq!(largest.value["name"], expected_largest.name.as_str());
        assert_eq!(
            largest.value["size_bytes"],
            serde_json::json!(expected_largest.size_bytes)
        );
        assert!(!report.safe_next_steps.is_empty());
        assert!(report
            .safe_next_steps
            .iter()
            .all(|step| matches!(step.risk, bbt_agent::Risk::ReadOnly)));
        assert!(report.risky_next_steps.is_empty());
    }

    #[test]
    fn list_agent_report_flags_missing_path_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");

        let listing = list_directory(temp.path().join("missing"));
        let report = list_agent_report(&listing, "2026-01-01T00:00:00Z");

        assert!(report.summary.contains("does not exist"));
        assert!(matches!(report.severity, Severity::Warning));
        assert!(report.risky_next_steps.is_empty());
    }

    /// Three directory levels below the root, mixed file sizes, a symlink to a
    /// directory, and a dangling symlink — 10 lstat entries in total.
    fn tree_fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        fs::write(root.join("top.txt"), vec![b'a'; 5]).expect("write top");
        let level1 = root.join("level1");
        fs::create_dir(&level1).expect("level1");
        fs::write(level1.join("mid.bin"), vec![b'b'; 4096]).expect("write mid");
        let level2 = level1.join("level2");
        fs::create_dir(&level2).expect("level2");
        fs::write(level2.join("deep.txt"), vec![b'c'; 300]).expect("write deep");
        let level3 = level2.join("level3");
        fs::create_dir(&level3).expect("level3");
        fs::write(level3.join("deepest.txt"), vec![b'd'; 65]).expect("write deepest");
        std::os::unix::fs::symlink(&level1, root.join("dir-link")).expect("dir symlink");
        std::os::unix::fs::symlink(root.join("missing"), root.join("dangling"))
            .expect("dangling symlink");
        temp
    }

    /// Independent lstat-based recursive (allocated, apparent) totals.
    fn expected_subtree_sizes(path: &Path) -> (u64, u64) {
        let metadata = fs::symlink_metadata(path).expect("lstat");
        let mut total = allocated_bytes(&metadata);
        let mut apparent = metadata.len();
        if metadata.is_dir() {
            for entry in fs::read_dir(path).expect("read dir") {
                let (child_total, child_apparent) =
                    expected_subtree_sizes(&entry.expect("dir entry").path());
                total += child_total;
                apparent += child_apparent;
            }
        }
        (total, apparent)
    }

    #[test]
    fn tree_reports_exact_recursive_totals_at_every_depth() {
        let temp = tree_fixture();
        let (expected_total, expected_apparent) = expected_subtree_sizes(temp.path());

        for max_depth in [None, Some(0), Some(1), Some(2), Some(10)] {
            let tree = tree_with_options(temp.path(), TreeOptions::new(max_depth, false));

            assert!(tree.exists, "depth {max_depth:?}");
            assert!(tree.warnings.is_empty(), "depth {max_depth:?}");
            let root = tree.root.as_ref().expect("root node");
            assert_eq!(root.kind, FileKind::Directory);
            assert_eq!(root.total_bytes, expected_total, "depth {max_depth:?}");
            assert_eq!(
                root.apparent_bytes, expected_apparent,
                "depth {max_depth:?}"
            );
            assert_eq!(tree.entries_seen, 10, "depth {max_depth:?}");
            assert_eq!(root.entries_seen, Some(10), "depth {max_depth:?}");
            assert_eq!(tree.max_depth_reached, Some(4), "depth {max_depth:?}");
            assert_eq!(tree.skipped_different_filesystem, 0);
            assert_eq!(
                root.children.is_some(),
                max_depth != Some(0),
                "depth {max_depth:?}"
            );
        }
    }

    #[test]
    fn tree_depth_limited_nodes_still_carry_full_recursive_totals() {
        let temp = tree_fixture();
        let (expected_total, expected_apparent) =
            expected_subtree_sizes(&temp.path().join("level1"));

        let tree = tree_with_options(temp.path(), TreeOptions::new(Some(1), false));

        let root = tree.root.as_ref().expect("root node");
        let children = root.children.as_ref().expect("children at depth 1");
        assert_eq!(
            children
                .iter()
                .map(|child| child.name.as_str())
                .collect::<Vec<_>>(),
            vec!["dangling", "dir-link", "level1", "top.txt"]
        );

        let level1 = &children[2];
        assert_eq!(level1.kind, FileKind::Directory);
        assert_eq!(level1.total_bytes, expected_total);
        assert_eq!(level1.apparent_bytes, expected_apparent);
        assert_eq!(level1.entries_seen, Some(6));
        assert!(
            level1.children.is_none(),
            "children below the depth cutoff must be omitted"
        );
    }

    #[test]
    fn tree_reports_directory_symlink_as_leaf_without_recursing() {
        let temp = tree_fixture();

        let tree = tree(temp.path());

        let root = tree.root.as_ref().expect("root node");
        let children = root.children.as_ref().expect("children");
        let dir_link = children
            .iter()
            .find(|child| child.name == "dir-link")
            .expect("dir-link node");
        assert_eq!(dir_link.kind, FileKind::Symlink);
        assert!(dir_link.children.is_none());
        assert!(dir_link.entries_seen.is_none());
        assert_eq!(
            dir_link.apparent_bytes,
            fs::symlink_metadata(temp.path().join("dir-link"))
                .expect("lstat")
                .len()
        );

        let dangling = children
            .iter()
            .find(|child| child.name == "dangling")
            .expect("dangling node");
        assert_eq!(dangling.kind, FileKind::Symlink);
        assert!(dangling.children.is_none());
    }

    #[test]
    fn tree_counts_hard_links_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let original = temp.path().join("original.bin");
        fs::write(&original, vec![42; 4096]).expect("write original");
        fs::hard_link(&original, temp.path().join("linked.bin")).expect("hard link");

        let tree = tree(temp.path());

        let directory_metadata = fs::metadata(temp.path()).expect("dir metadata");
        let original_metadata = fs::metadata(&original).expect("metadata");
        let root = tree.root.as_ref().expect("root node");
        assert_eq!(
            root.total_bytes,
            allocated_bytes(&directory_metadata) + allocated_bytes(&original_metadata)
        );
        assert_eq!(
            root.apparent_bytes,
            directory_metadata.len() + original_metadata.len()
        );
    }

    #[test]
    fn tree_revisited_directory_is_skipped_with_bounded_warning_and_no_duplicate_totals() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("payload.bin"), vec![7u8; 4096]).expect("write payload");
        let metadata = fs::symlink_metadata(temp.path()).expect("lstat");
        let mut walk = TreeWalk {
            options: TreeOptions::default(),
            root_device: metadata.dev(),
            seen_inodes: HashSet::new(),
            visited_directories: HashSet::new(),
            entries_seen: 0,
            max_depth_reached: 0,
            skipped_different_filesystem: 0,
            warnings: Vec::new(),
        };

        let first = walk.build_node(temp.path(), temp.path().as_os_str(), &metadata, 0);
        assert!(first.total_bytes > 0);
        assert!(walk.warnings.is_empty());

        let second = walk.build_node(temp.path(), temp.path().as_os_str(), &metadata, 0);

        assert_eq!(
            second.total_bytes, 0,
            "a directory revisit must not contribute duplicate totals"
        );
        assert_eq!(second.apparent_bytes, 0);
        assert!(
            second.children.is_none(),
            "a revisited directory must not be descended into again"
        );
        assert_eq!(
            walk.warnings.len(),
            1,
            "exactly one bounded warning per skipped directory revisit"
        );
        assert!(walk.warnings[0].contains("already visited"));
    }

    #[test]
    fn tree_preserves_non_utf8_names_as_bytes_hex() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut raw_name = b"weird-".to_vec();
        raw_name.push(0xff);
        let name = std::ffi::OsString::from_vec(raw_name.clone());
        fs::write(temp.path().join(&name), "payload").expect("write non-utf8 fixture");

        let tree = tree(temp.path());

        assert!(tree.warnings.is_empty());
        let root = tree.root.as_ref().expect("root node");
        let children = root.children.as_ref().expect("children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name_bytes_hex, bytes_hex(&raw_name));
        assert_eq!(children[0].name, name.to_string_lossy());
        assert_eq!(children[0].apparent_bytes, 7);
    }

    #[test]
    fn tree_reports_missing_path_as_warning() {
        let temp = tempfile::tempdir().expect("tempdir");

        let tree = tree(temp.path().join("missing"));

        assert!(!tree.exists);
        assert!(tree.root.is_none());
        assert_eq!(tree.entries_seen, 0);
        assert_eq!(tree.max_depth_reached, None);
        assert_eq!(tree.warnings.len(), 1);
        assert!(tree.warnings[0].contains("does not exist"));
    }

    #[test]
    fn tree_on_non_directory_reports_single_leaf_node() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("plain.txt");
        fs::write(&file, "plain").expect("write fixture");

        let tree = tree(&file);

        assert!(tree.exists);
        let root = tree.root.as_ref().expect("root node");
        assert_eq!(root.kind, FileKind::RegularFile);
        assert_eq!(root.name, file.display().to_string());
        assert_eq!(root.apparent_bytes, 5);
        assert!(root.children.is_none());
        assert_eq!(tree.entries_seen, 1);
        assert_eq!(tree.warnings.len(), 1);
        assert!(tree.warnings[0].contains("is not a directory"));
    }

    #[test]
    fn tree_degrades_to_warning_when_subdirectory_is_unreadable() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }

        let temp = tree_fixture();
        let locked = temp.path().join("locked");
        fs::create_dir(&locked).expect("create locked dir");
        fs::write(locked.join("hidden.txt"), "hidden").expect("write hidden");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("lock dir");

        let tree = tree(temp.path());

        let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o700));

        assert!(tree.exists);
        assert!(
            tree.warnings
                .iter()
                .any(|warning| warning.contains("cannot read directory")
                    && warning.contains("locked"))
        );

        let root = tree.root.as_ref().expect("root node");
        let children = root.children.as_ref().expect("children");
        let locked_node = children
            .iter()
            .find(|child| child.name == "locked")
            .expect("locked node still reported");
        assert_eq!(locked_node.kind, FileKind::Directory);
        assert_eq!(
            locked_node.children.as_ref().map(Vec::len),
            Some(0),
            "unreadable directory reports an empty children array"
        );
        assert!(
            children.iter().any(|child| child.name == "level1"),
            "the rest of the tree is still traversed"
        );
    }

    #[test]
    fn tree_agent_report_carries_triage_facts_and_read_only_steps_only() {
        let temp = tree_fixture();

        let tree = tree(temp.path());
        let report = tree_agent_report(&tree, "2026-01-01T00:00:00Z");

        assert_eq!(report.subject.domain, "fs");
        assert_eq!(report.subject.command, "tree");
        for key in [
            "path_exists",
            "total_bytes",
            "entries_seen",
            "max_depth_reached",
            "largest_subtree",
        ] {
            assert!(
                report.facts.iter().any(|fact| fact.key == key),
                "missing fact {key}"
            );
        }
        let largest = report
            .facts
            .iter()
            .find(|fact| fact.key == "largest_subtree")
            .expect("largest_subtree fact");
        assert_eq!(largest.value["name"], "level1");
        assert!(!report.safe_next_steps.is_empty());
        assert!(report
            .safe_next_steps
            .iter()
            .all(|step| matches!(step.risk, bbt_agent::Risk::ReadOnly)));
        assert!(report.risky_next_steps.is_empty());
    }

    #[test]
    fn tree_agent_report_flags_missing_path_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");

        let tree = tree(temp.path().join("missing"));
        let report = tree_agent_report(&tree, "2026-01-01T00:00:00Z");

        assert!(report.summary.contains("does not exist"));
        assert!(matches!(report.severity, Severity::Warning));
        assert!(report.risky_next_steps.is_empty());
    }
}
