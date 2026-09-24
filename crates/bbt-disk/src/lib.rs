//! Disk domain for Better Basic Tools.
//!
//! Commands: `bbt disk usage PATH`, `bbt disk filesystems`, and `bbt disk pressure`.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use bbt_core::filesystems::read_filesystems;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub use bbt_core::filesystems::FilesystemInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub path: String,
    pub path_bytes_hex: String,
    pub absolute_path: Option<String>,
    pub exists: bool,
    pub max_depth: Option<u64>,
    pub one_filesystem: bool,
    pub top_limit: usize,
    pub total_bytes: u64,
    pub apparent_bytes: u64,
    pub entries_seen: u64,
    pub directories_seen: u64,
    pub files_seen: u64,
    pub symlinks_seen: u64,
    pub unreadable_entries: u64,
    pub skipped_different_filesystem: u64,
    pub largest_children: Vec<UsageChild>,
    pub errors: Vec<UsageError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageOptions {
    max_depth: Option<u64>,
    one_filesystem: bool,
    top_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageDepthError {
    depth: u64,
}

impl std::fmt::Display for UsageDepthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "disk usage supports depth 0 (totals only) and depth 1 (immediate children), not {}",
            self.depth
        )
    }
}

impl std::error::Error for UsageDepthError {}

impl UsageOptions {
    pub fn new(
        max_depth: Option<u64>,
        one_filesystem: bool,
        top_limit: usize,
    ) -> Result<Self, UsageDepthError> {
        if let Some(depth) = max_depth.filter(|depth| *depth > 1) {
            return Err(UsageDepthError { depth });
        }
        Ok(Self {
            max_depth,
            one_filesystem,
            top_limit,
        })
    }
}

impl Default for UsageOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            one_filesystem: false,
            top_limit: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageChild {
    pub path: String,
    pub path_bytes_hex: String,
    pub kind: UsageKind,
    pub total_bytes: u64,
    pub apparent_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageError {
    pub path: String,
    pub path_bytes_hex: String,
    pub kind: UsageErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageKind {
    RegularFile,
    Directory,
    Symlink,
    Other,
    Missing,
}

impl UsageKind {
    pub fn label(self) -> &'static str {
        match self {
            UsageKind::RegularFile => "regular file",
            UsageKind::Directory => "directory",
            UsageKind::Symlink => "symlink",
            UsageKind::Other => "other",
            UsageKind::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageErrorKind {
    NotFound,
    PermissionDenied,
    ReadDirectory,
    Metadata,
    DirectoryCycle,
}

pub fn usage(path: impl AsRef<Path>) -> DiskUsage {
    usage_with_options(path, UsageOptions::default())
}

pub fn usage_with_options(path: impl AsRef<Path>, options: UsageOptions) -> DiskUsage {
    let path = path.as_ref();
    let display_path = path.display().to_string();
    let path_bytes_hex = bytes_hex(path.as_os_str().as_bytes());
    let absolute_path = absolute_path(path)
        .ok()
        .map(|path| path.display().to_string());

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            let kind = if error.kind() == io::ErrorKind::NotFound {
                UsageErrorKind::NotFound
            } else if error.kind() == io::ErrorKind::PermissionDenied {
                UsageErrorKind::PermissionDenied
            } else {
                UsageErrorKind::Metadata
            };

            return DiskUsage {
                path: display_path.clone(),
                path_bytes_hex: path_bytes_hex.clone(),
                absolute_path,
                exists: false,
                max_depth: options.max_depth,
                one_filesystem: options.one_filesystem,
                top_limit: options.top_limit,
                total_bytes: 0,
                apparent_bytes: 0,
                entries_seen: 0,
                directories_seen: 0,
                files_seen: 0,
                symlinks_seen: 0,
                unreadable_entries: 1,
                skipped_different_filesystem: 0,
                largest_children: Vec::new(),
                errors: vec![UsageError::new(path, kind, error)],
            };
        }
    };

    let mut usage = DiskUsage {
        path: display_path,
        path_bytes_hex,
        absolute_path,
        exists: true,
        max_depth: options.max_depth,
        one_filesystem: options.one_filesystem,
        top_limit: options.top_limit,
        total_bytes: 0,
        apparent_bytes: 0,
        entries_seen: 0,
        directories_seen: 0,
        files_seen: 0,
        symlinks_seen: 0,
        unreadable_entries: 0,
        skipped_different_filesystem: 0,
        largest_children: Vec::new(),
        errors: Vec::new(),
    };

    let mut seen_inodes = HashSet::new();
    let mut visited_directories = HashSet::new();
    let root_summary = summarize_path(
        path,
        &metadata,
        &mut usage,
        &mut seen_inodes,
        &mut visited_directories,
        TraversalContext {
            root_device: metadata.dev(),
            collect_children: options.max_depth != Some(0),
        },
    );
    usage.total_bytes = root_summary.total_bytes;
    usage.apparent_bytes = root_summary.apparent_bytes;
    usage
        .largest_children
        .sort_by_key(|child| std::cmp::Reverse(child.total_bytes));
    usage.largest_children.truncate(usage.top_limit);

    usage
}

pub fn agent_report(usage: &DiskUsage, generated_at: impl Into<String>) -> AgentReport {
    let severity = if !usage.exists {
        Severity::Warning
    } else if usage.unreadable_entries > 0 {
        Severity::Info
    } else {
        Severity::Ok
    };

    let summary = if usage.exists {
        format!(
            "{} uses {} across {} entries.",
            usage.path,
            human_bytes(usage.total_bytes),
            usage.entries_seen
        )
    } else {
        format!("{} does not exist or cannot be inspected.", usage.path)
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("disk", "usage", Some(usage.path.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "path_exists",
        serde_json::json!(usage.exists),
        Severity::Info,
        vec![format!("{} exists: {}", usage.path, usage.exists)],
    ))
    .with_fact(Fact::new(
        "total_bytes",
        serde_json::json!(usage.total_bytes),
        Severity::Info,
        vec![format!("Total allocated bytes: {}", usage.total_bytes)],
    ))
    .with_fact(Fact::new(
        "entries_seen",
        serde_json::json!(usage.entries_seen),
        Severity::Info,
        vec![format!("Entries seen: {}", usage.entries_seen)],
    ));

    if let Some(child) = usage.largest_children.first() {
        report = report.with_fact(Fact::new(
            "largest_child",
            serde_json::json!({
                "path": child.path,
                "kind": child.kind,
                "total_bytes": child.total_bytes,
                "apparent_bytes": child.apparent_bytes
            }),
            Severity::Info,
            vec!["largest immediate child by recursively calculated allocated bytes".to_owned()],
        ));
    }

    if usage.unreadable_entries > 0 {
        report = report.with_interpretation(format!(
            "{} entries could not be inspected; totals may be lower than real usage.",
            usage.unreadable_entries
        ));
    }

    let quoted_path = shell_quote(&usage.path);
    let mut report = report
        .with_safe_next_step(NextStep::read_only(
            format!("bbt --json disk usage -- {quoted_path}"),
            "Fetch complete structured disk usage data",
        ))
        .with_safe_next_step(NextStep::read_only(
            format!("bbt --agent fs inspect -- {quoted_path}"),
            "Inspect metadata and access details for the same path",
        ));

    if let Some(child) = usage.largest_children.first() {
        report = report.with_safe_next_step(NextStep::read_only(
            format!(
                "bbt --agent disk usage --depth 1 -- {}",
                shell_quote(&child.path)
            ),
            "Inspect the largest immediate child in more detail",
        ));
    }

    report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemReport {
    pub filesystems: Vec<FilesystemInfo>,
    pub warnings: Vec<String>,
}

pub fn filesystems() -> FilesystemReport {
    let mut warnings = Vec::new();
    let filesystems = read_filesystems(&mut warnings);
    FilesystemReport {
        filesystems,
        warnings,
    }
}

pub fn filesystems_agent_report(
    report: &FilesystemReport,
    generated_at: impl Into<String>,
) -> AgentReport {
    let severity = if report.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };

    let fullest = report
        .filesystems
        .iter()
        .filter_map(|fs| fs.used_percent.map(|used| (used, fs)))
        .max_by(|left, right| left.0.total_cmp(&right.0));

    let summary = match fullest {
        Some((used, fs)) => format!(
            "{} mounted filesystems visible; fullest is {} at {used:.1}% used.",
            report.filesystems.len(),
            fs.target
        ),
        None => format!("{} mounted filesystems visible.", report.filesystems.len()),
    };

    let mut agent_report = AgentReport::new(
        generated_at,
        Subject::new("disk", "filesystems", None::<String>),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "filesystem_count",
        serde_json::json!(report.filesystems.len()),
        Severity::Info,
        vec!["/proc/mounts entries after pseudo-filesystem filtering".to_owned()],
    ));

    if let Some((used, fs)) = fullest {
        agent_report = agent_report.with_fact(Fact::new(
            "fullest_filesystem",
            serde_json::json!({
                "target": fs.target,
                "fstype": fs.fstype,
                "total_bytes": fs.total_bytes,
                "available_bytes": fs.available_bytes,
                "used_percent": fs.used_percent,
            }),
            Severity::Info,
            vec!["statvfs-derived used percentage per mount target".to_owned()],
        ));

        let level = if used >= 90.0 {
            "high"
        } else if used >= 75.0 {
            "elevated"
        } else {
            "normal"
        };
        agent_report = agent_report.with_interpretation(format!(
            "Fullest filesystem {} is {level} at {used:.1}% used.",
            fs.target
        ));
    }

    if !report.warnings.is_empty() {
        agent_report = agent_report.with_interpretation(format!(
            "{} mounts had collection warnings.",
            report.warnings.len()
        ));
    }

    agent_report = agent_report.with_safe_next_step(NextStep::read_only(
        "bbt --json disk filesystems",
        "Fetch complete structured filesystem list",
    ));

    if let Some((_, fs)) = fullest {
        agent_report = agent_report.with_safe_next_step(NextStep::read_only(
            format!(
                "bbt --agent disk usage --depth 1 -- {}",
                shell_quote(&fs.target)
            ),
            "Inspect usage under the fullest mounted filesystem",
        ));
    }

    agent_report.with_safe_next_step(NextStep::read_only(
        "bbt --agent host snapshot",
        "Correlate filesystem pressure with overall host state",
    ))
}

pub const PRESSURE_IO_PATH: &str = "/proc/pressure/io";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureInfo {
    pub available: bool,
    pub some: Option<PressureLine>,
    pub full: Option<PressureLine>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureLine {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
    pub total_stalled_usec: u64,
}

pub fn pressure() -> PressureInfo {
    pressure_from_path(Path::new(PRESSURE_IO_PATH))
}

pub fn pressure_from_path(path: &Path) -> PressureInfo {
    match fs::read_to_string(path) {
        Ok(content) => pressure_from_str(&content),
        Err(error) => PressureInfo {
            available: false,
            some: None,
            full: None,
            warnings: vec![format!(
                "{} unreadable: {error}; PSI requires Linux 4.20+ with CONFIG_PSI enabled",
                path.display()
            )],
        },
    }
}

pub fn pressure_from_str(content: &str) -> PressureInfo {
    let mut some = None;
    let mut full = None;
    let mut warnings = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let slot = match line.split_whitespace().next() {
            Some("some") => &mut some,
            Some("full") => &mut full,
            _ => {
                warnings.push(format!("unrecognized pressure line: {line}"));
                continue;
            }
        };
        match parse_pressure_line(line) {
            Some(parsed) => *slot = Some(parsed),
            None => warnings.push(format!("malformed pressure line: {line}")),
        }
    }

    let available = some.is_some() || full.is_some();
    if !available {
        warnings.push("no parseable PSI data found".to_owned());
    }

    PressureInfo {
        available,
        some,
        full,
        warnings,
    }
}

fn parse_pressure_line(line: &str) -> Option<PressureLine> {
    let mut avg10 = None;
    let mut avg60 = None;
    let mut avg300 = None;
    let mut total = None;

    for field in line.split_whitespace().skip(1) {
        let (key, value) = field.split_once('=')?;
        match key {
            "avg10" => avg10 = Some(value.parse::<f64>().ok()?),
            "avg60" => avg60 = Some(value.parse::<f64>().ok()?),
            "avg300" => avg300 = Some(value.parse::<f64>().ok()?),
            "total" => total = Some(value.parse::<u64>().ok()?),
            _ => {}
        }
    }

    Some(PressureLine {
        avg10: avg10?,
        avg60: avg60?,
        avg300: avg300?,
        total_stalled_usec: total?,
    })
}

pub fn pressure_agent_report(info: &PressureInfo, generated_at: impl Into<String>) -> AgentReport {
    let severity = if info.available && info.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };

    let summary = match &info.some {
        Some(some) => format!(
            "I/O pressure (some): {:.2}% avg10, {:.2}% avg60, {:.2}% avg300.",
            some.avg10, some.avg60, some.avg300
        ),
        None => "I/O pressure information (PSI) is unavailable on this system.".to_owned(),
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("disk", "pressure", None::<String>),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "psi_available",
        serde_json::json!(info.available),
        Severity::Info,
        vec!["/proc/pressure/io readability and parse result".to_owned()],
    ));

    if let Some(some) = &info.some {
        report = report
            .with_fact(Fact::new(
                "some_avg10",
                serde_json::json!(some.avg10),
                Severity::Info,
                vec!["% of the last 10s at least one task stalled on I/O".to_owned()],
            ))
            .with_fact(Fact::new(
                "some_avg60",
                serde_json::json!(some.avg60),
                Severity::Info,
                vec!["% of the last 60s at least one task stalled on I/O".to_owned()],
            ))
            .with_fact(Fact::new(
                "some_avg300",
                serde_json::json!(some.avg300),
                Severity::Info,
                vec!["% of the last 300s at least one task stalled on I/O".to_owned()],
            ));

        let level = if some.avg10 >= 25.0 {
            "high"
        } else if some.avg10 >= 10.0 {
            "elevated"
        } else {
            "normal"
        };
        report = report.with_interpretation(format!(
            "I/O stall pressure is {level} at {:.2}% (some, 10s window).",
            some.avg10
        ));
    }

    if let Some(full) = &info.full {
        report = report.with_fact(Fact::new(
            "full_pressure",
            serde_json::json!({
                "avg10": full.avg10,
                "avg60": full.avg60,
                "avg300": full.avg300,
                "total_stalled_usec": full.total_stalled_usec,
            }),
            Severity::Info,
            vec!["% of time all non-idle tasks stalled on I/O simultaneously".to_owned()],
        ));
    }

    if !info.available {
        report = report.with_interpretation(
            "PSI is unavailable; this kernel is older than 4.20 or has CONFIG_PSI disabled."
                .to_owned(),
        );
    } else if !info.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} PSI lines had collection warnings.",
            info.warnings.len()
        ));
    }

    report
        .with_safe_next_step(NextStep::read_only(
            "bbt --json disk pressure",
            "Fetch complete structured I/O pressure data",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent proc list",
            "Correlate I/O pressure with the processes currently running",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent disk filesystems",
            "Correlate I/O pressure with filesystem capacity",
        ))
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

pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[derive(Debug, Clone, Copy)]
struct UsageSummary {
    total_bytes: u64,
    apparent_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
struct TraversalContext {
    root_device: u64,
    collect_children: bool,
}

fn summarize_path(
    path: &Path,
    metadata: &fs::Metadata,
    usage: &mut DiskUsage,
    seen_inodes: &mut HashSet<(u64, u64)>,
    visited_directories: &mut HashSet<(u64, u64)>,
    context: TraversalContext,
) -> UsageSummary {
    usage.entries_seen += 1;
    record_kind(metadata, usage);
    if metadata.is_dir() && !visited_directories.insert((metadata.dev(), metadata.ino())) {
        // A same-device revisit (e.g. a bind mount forming a cycle) must not
        // recurse forever or double-count an already-summed subtree.
        usage.errors.push(UsageError {
            path: path.display().to_string(),
            path_bytes_hex: bytes_hex(path.as_os_str().as_bytes()),
            kind: UsageErrorKind::DirectoryCycle,
            message: "directory was already visited during this traversal; skipped to avoid double counting".to_owned(),
        });
        return UsageSummary {
            total_bytes: 0,
            apparent_bytes: 0,
        };
    }
    let already_seen = !metadata.is_dir() && !seen_inodes.insert((metadata.dev(), metadata.ino()));

    let mut summary = UsageSummary {
        total_bytes: if already_seen {
            0
        } else {
            allocated_bytes(metadata)
        },
        apparent_bytes: if already_seen { 0 } else { metadata.len() },
    };

    if !metadata.is_dir() {
        return summary;
    }

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            usage.unreadable_entries += 1;
            usage
                .errors
                .push(UsageError::new(path, UsageErrorKind::ReadDirectory, error));
            return summary;
        }
    };

    for entry in read_dir {
        match entry {
            Ok(entry) => {
                let child_path = entry.path();
                match fs::symlink_metadata(&child_path) {
                    Ok(child_metadata) => {
                        if usage.one_filesystem && child_metadata.dev() != context.root_device {
                            usage.skipped_different_filesystem += 1;
                            continue;
                        }
                        let child_summary = summarize_path(
                            &child_path,
                            &child_metadata,
                            usage,
                            seen_inodes,
                            visited_directories,
                            TraversalContext {
                                root_device: context.root_device,
                                collect_children: false,
                            },
                        );
                        if context.collect_children {
                            usage.largest_children.push(UsageChild {
                                path: child_path.display().to_string(),
                                path_bytes_hex: bytes_hex(child_path.as_os_str().as_bytes()),
                                kind: classify_kind(&child_metadata),
                                total_bytes: child_summary.total_bytes,
                                apparent_bytes: child_summary.apparent_bytes,
                            });
                        }
                        summary.total_bytes = summary
                            .total_bytes
                            .saturating_add(child_summary.total_bytes);
                        summary.apparent_bytes = summary
                            .apparent_bytes
                            .saturating_add(child_summary.apparent_bytes);
                    }
                    Err(error) => {
                        usage.unreadable_entries += 1;
                        usage.errors.push(UsageError::new(
                            &child_path,
                            UsageErrorKind::Metadata,
                            error,
                        ));
                    }
                }
            }
            Err(error) => {
                usage.unreadable_entries += 1;
                usage.errors.push(UsageError {
                    path: path.display().to_string(),
                    path_bytes_hex: bytes_hex(path.as_os_str().as_bytes()),
                    kind: UsageErrorKind::ReadDirectory,
                    message: error.to_string(),
                });
            }
        }
    }

    summary
}

fn record_kind(metadata: &fs::Metadata, usage: &mut DiskUsage) {
    if metadata.is_dir() {
        usage.directories_seen += 1;
    } else if metadata.is_file() {
        usage.files_seen += 1;
    } else if metadata.file_type().is_symlink() {
        usage.symlinks_seen += 1;
    }
}

fn classify_kind(metadata: &fs::Metadata) -> UsageKind {
    if metadata.is_dir() {
        UsageKind::Directory
    } else if metadata.is_file() {
        UsageKind::RegularFile
    } else if metadata.file_type().is_symlink() {
        UsageKind::Symlink
    } else {
        UsageKind::Other
    }
}

fn allocated_bytes(metadata: &fs::Metadata) -> u64 {
    metadata.blocks().saturating_mul(512)
}

fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

impl UsageError {
    fn new(path: &Path, kind: UsageErrorKind, error: io::Error) -> Self {
        Self {
            path: path.display().to_string(),
            path_bytes_hex: bytes_hex(path.as_os_str().as_bytes()),
            kind,
            message: error.to_string(),
        }
    }
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn usage_options_reject_depth_greater_than_one() {
        let result = UsageOptions::new(Some(2), false, 10);

        assert!(result.is_err(), "depth values above one must be rejected");
    }

    #[test]
    fn filesystems_report_lists_mounts_with_capacity_data() {
        let report = filesystems();

        assert!(!report.filesystems.is_empty());
        assert!(report
            .filesystems
            .iter()
            .all(|fs| !fs.target.is_empty() && !fs.fstype.is_empty()));
        assert!(report.filesystems.iter().any(|fs| fs.total_bytes.is_some()));
    }

    #[test]
    fn filesystems_agent_report_carries_count_and_safe_steps_only() {
        let report = filesystems();

        let agent_report = filesystems_agent_report(&report, "2026-01-01T00:00:00Z");

        assert_eq!(agent_report.subject.domain, "disk");
        assert_eq!(agent_report.subject.command, "filesystems");
        assert!(agent_report
            .facts
            .iter()
            .any(|fact| fact.key == "filesystem_count"));
        assert!(!agent_report.safe_next_steps.is_empty());
        assert!(agent_report.risky_next_steps.is_empty());
    }

    #[test]
    fn pressure_parses_reference_format_exactly() {
        let content = "some avg10=0.31 avg60=0.25 avg300=0.10 total=1216511894\n\
                       full avg10=0.31 avg60=0.25 avg300=0.10 total=1192581388\n";

        let info = pressure_from_str(content);

        assert!(info.available);
        assert!(info.warnings.is_empty());
        let some = info.some.expect("some line");
        assert_eq!(some.avg10, 0.31);
        assert_eq!(some.avg60, 0.25);
        assert_eq!(some.avg300, 0.10);
        assert_eq!(some.total_stalled_usec, 1216511894);
        let full = info.full.expect("full line");
        assert_eq!(full.avg10, 0.31);
        assert_eq!(full.avg60, 0.25);
        assert_eq!(full.avg300, 0.10);
        assert_eq!(full.total_stalled_usec, 1192581388);
    }

    #[test]
    fn pressure_parses_zero_values() {
        let content = "some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n\
                       full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";

        let info = pressure_from_str(content);

        assert!(info.available);
        assert_eq!(info.some.expect("some line").avg10, 0.0);
        assert_eq!(info.full.expect("full line").total_stalled_usec, 0);
    }

    #[test]
    fn pressure_reports_malformed_content_as_unavailable_with_warnings() {
        let malformed = [
            "",
            "garbage\n",
            "some avg10=not-a-number avg60=0.25 avg300=0.10 total=1\n",
            "some avg10=0.31 avg60=0.25\n",
            "some avg10 avg60 avg300 total\n",
        ];

        for content in malformed {
            let info = pressure_from_str(content);

            assert!(!info.available, "content {content:?} should be unavailable");
            assert!(info.some.is_none());
            assert!(info.full.is_none());
            assert!(!info.warnings.is_empty());
        }
    }

    #[test]
    fn pressure_keeps_parseable_line_when_other_line_is_malformed() {
        let content = "some avg10=1.50 avg60=0.75 avg300=0.20 total=42\n\
                       full avg10=broken\n";

        let info = pressure_from_str(content);

        assert!(info.available);
        assert_eq!(info.some.expect("some line").total_stalled_usec, 42);
        assert!(info.full.is_none());
        assert_eq!(info.warnings.len(), 1);
    }

    #[test]
    fn pressure_missing_file_is_unavailable_without_panic() {
        let temp = tempfile::tempdir().expect("tempdir");

        let info = pressure_from_path(&temp.path().join("no-psi-here"));

        assert!(!info.available);
        assert!(info.some.is_none());
        assert!(info.full.is_none());
        assert!(!info.warnings.is_empty());
    }

    #[test]
    fn pressure_agent_report_carries_facts_and_read_only_steps_only() {
        let info = pressure_from_str(
            "some avg10=12.00 avg60=4.00 avg300=1.00 total=1000\n\
             full avg10=6.00 avg60=2.00 avg300=0.50 total=500\n",
        );

        let report = pressure_agent_report(&info, "2026-01-01T00:00:00Z");

        assert_eq!(report.subject.domain, "disk");
        assert_eq!(report.subject.command, "pressure");
        for key in ["psi_available", "some_avg10", "some_avg60", "some_avg300"] {
            assert!(
                report.facts.iter().any(|fact| fact.key == key),
                "missing fact {key}"
            );
        }
        assert!(report
            .interpretation
            .iter()
            .any(|line| line.contains("elevated")));
        assert!(!report.safe_next_steps.is_empty());
        assert!(report
            .safe_next_steps
            .iter()
            .all(|step| matches!(step.risk, bbt_agent::Risk::ReadOnly)));
        assert!(report.risky_next_steps.is_empty());
    }

    #[test]
    fn pressure_agent_report_flags_unavailable_psi_without_error() {
        let info = pressure_from_str("garbage\n");

        let report = pressure_agent_report(&info, "2026-01-01T00:00:00Z");

        assert!(report.summary.contains("unavailable"));
        assert!(report
            .facts
            .iter()
            .any(|fact| fact.key == "psi_available" && fact.value == serde_json::json!(false)));
        assert!(report.risky_next_steps.is_empty());
    }

    #[test]
    fn revisited_directory_is_skipped_with_bounded_warning_and_no_duplicate_totals() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("payload.bin"), vec![7u8; 4096]).expect("write payload");
        let metadata = fs::symlink_metadata(temp.path()).expect("lstat");
        let mut usage = usage(temp.path());
        usage.errors.clear();
        let mut seen_inodes = HashSet::new();
        let mut visited_directories = HashSet::new();
        let context = TraversalContext {
            root_device: metadata.dev(),
            collect_children: false,
        };

        let first = summarize_path(
            temp.path(),
            &metadata,
            &mut usage,
            &mut seen_inodes,
            &mut visited_directories,
            context,
        );
        assert!(first.total_bytes > 0);
        assert!(usage.errors.is_empty());

        let second = summarize_path(
            temp.path(),
            &metadata,
            &mut usage,
            &mut seen_inodes,
            &mut visited_directories,
            context,
        );

        assert_eq!(
            second.total_bytes, 0,
            "a directory revisit must not contribute duplicate totals"
        );
        assert_eq!(second.apparent_bytes, 0);
        assert_eq!(
            usage.errors.len(),
            1,
            "exactly one bounded warning per skipped directory revisit"
        );
        assert_eq!(usage.errors[0].kind, UsageErrorKind::DirectoryCycle);
    }

    #[test]
    fn directory_cycle_error_kind_serializes_as_the_schema_value() {
        let value = serde_json::to_value(UsageErrorKind::DirectoryCycle)
            .expect("directory-cycle error kind serializes");

        assert_eq!(value, serde_json::json!("directory-cycle"));
    }

    #[test]
    fn reports_missing_path_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing");

        let usage = usage(&missing);

        assert!(!usage.exists);
        assert_eq!(usage.total_bytes, 0);
        assert_eq!(usage.errors[0].kind, UsageErrorKind::NotFound);
    }

    #[test]
    fn sums_directory_usage_without_following_symlink_targets() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("small.txt"), "small").expect("write small");
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).expect("nested dir");
        fs::write(nested.join("large.txt"), "a larger file body").expect("write large");
        symlink(nested.join("large.txt"), temp.path().join("large-link")).expect("symlink");

        let usage = usage(temp.path());

        assert!(usage.exists);
        assert!(usage.entries_seen >= 4);
        assert!(usage.files_seen >= 2);
        assert_eq!(usage.symlinks_seen, 1);
        assert!(usage.apparent_bytes >= "smalla larger file body".len() as u64);
        assert!(usage
            .largest_children
            .iter()
            .any(|child| child.path.ends_with("nested")));
    }

    #[test]
    fn total_bytes_uses_lstat_blocks_and_never_follows_symlinks() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");

        let big_target = outside.path().join("big-target.bin");
        fs::write(&big_target, vec![7u8; 4 * 1024 * 1024]).expect("write big target");
        let target_dir = outside.path().join("target-dir");
        fs::create_dir(&target_dir).expect("target dir");
        fs::write(target_dir.join("payload.bin"), vec![9u8; 2 * 1024 * 1024]).expect("payload");

        let root = temp.path();
        fs::write(root.join("real.txt"), b"real file contents").expect("write real");
        symlink(&big_target, root.join("link-to-big-file")).expect("file symlink");
        symlink(&target_dir, root.join("link-to-dir")).expect("dir symlink");
        symlink(root.join("does-not-exist"), root.join("dangling")).expect("dangling symlink");

        let usage = usage(root);

        let mut expected_total = 0;
        let mut expected_apparent = 0;
        for path in [
            root.to_path_buf(),
            root.join("real.txt"),
            root.join("link-to-big-file"),
            root.join("link-to-dir"),
            root.join("dangling"),
        ] {
            let metadata = fs::symlink_metadata(&path).expect("lstat");
            expected_total += allocated_bytes(&metadata);
            expected_apparent += metadata.len();
        }

        assert_eq!(usage.entries_seen, 5);
        assert_eq!(usage.files_seen, 1);
        assert_eq!(usage.directories_seen, 1);
        assert_eq!(usage.symlinks_seen, 3);
        assert_eq!(usage.unreadable_entries, 0);
        assert_eq!(usage.total_bytes, expected_total);
        assert_eq!(usage.apparent_bytes, expected_apparent);
        assert!(usage.total_bytes < 1024 * 1024);
        assert!(usage.apparent_bytes < 1024 * 1024);
    }

    #[test]
    fn hard_links_are_counted_once_for_allocated_and_apparent_bytes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let original = temp.path().join("original.bin");
        let linked = temp.path().join("linked.bin");
        fs::write(&original, vec![42; 4096]).expect("write original");
        fs::hard_link(&original, &linked).expect("hard link");

        let usage = usage(temp.path());
        let original_metadata = fs::metadata(&original).expect("metadata");

        let directory_metadata = fs::metadata(temp.path()).expect("dir metadata");

        assert_eq!(usage.files_seen, 2);
        assert_eq!(
            usage.apparent_bytes,
            directory_metadata.len() + original_metadata.len()
        );
        assert_eq!(
            usage.total_bytes,
            allocated_bytes(&directory_metadata) + allocated_bytes(&original_metadata)
        );
        assert!(usage
            .largest_children
            .iter()
            .filter(|child| child.path.ends_with(".bin"))
            .any(|child| child.total_bytes == 0));
    }
}
