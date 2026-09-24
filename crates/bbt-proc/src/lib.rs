//! Process domain for Better Basic Tools.
//!
//! First target: `bbt proc inspect PID`.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcInspection {
    pub pid: u32,
    pub exists: bool,
    pub name: Option<String>,
    pub state: Option<String>,
    pub parent_pid: Option<u32>,
    pub child_pids: Vec<u32>,
    pub command_line: Vec<String>,
    pub command_line_bytes_hex: Vec<String>,
    pub executable: Option<String>,
    pub cwd: Option<String>,
    pub uid: Option<u32>,
    pub user: Option<String>,
    pub gid: Option<u32>,
    pub threads: Option<u64>,
    pub memory: ProcMemory,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcMemory {
    pub vm_peak_bytes: Option<u64>,
    pub vm_size_bytes: Option<u64>,
    pub vm_rss_bytes: Option<u64>,
    pub vm_hwm_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcList {
    pub processes: Vec<ProcSummary>,
    pub process_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcSummary {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: Option<String>,
    pub state: Option<String>,
    pub uid: Option<u32>,
    pub user: Option<String>,
    pub vm_rss_bytes: Option<u64>,
    pub command_line: Vec<String>,
}

pub fn list() -> ProcList {
    let mut warnings = Vec::new();
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("/proc scan unavailable: {error}"));
            return ProcList {
                processes: Vec::new(),
                process_count: 0,
                warnings,
            };
        }
    };
    let mut processes = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        match summarize_process(pid) {
            Ok(summary) => processes.push(summary),
            Err(error) => warnings.push(format!("pid {pid} unavailable during list: {error}")),
        }
    }
    processes.sort_by_key(|process| process.pid);
    let process_count = processes.len();
    ProcList {
        processes,
        process_count,
        warnings,
    }
}

pub fn list_agent_report(list: &ProcList, generated_at: impl Into<String>) -> AgentReport {
    let severity = if list.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };
    let root_process_count = list
        .processes
        .iter()
        .filter(|process| process.uid == Some(0))
        .count();
    let zombie_process_count = list
        .processes
        .iter()
        .filter(|process| {
            process
                .state
                .as_deref()
                .is_some_and(|state| state.starts_with('Z'))
        })
        .count();
    let top_rss_process = list
        .processes
        .iter()
        .filter_map(|process| process.vm_rss_bytes.map(|rss| (process, rss)))
        .max_by_key(|(_, rss)| *rss);

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("proc", "list", Some("local")),
        format!("Observed {} visible local processes.", list.process_count),
        severity,
    )
    .with_fact(Fact::new(
        "process_count",
        serde_json::json!(list.process_count),
        Severity::Info,
        vec!["counted numeric entries under /proc with readable process metadata"],
    ))
    .with_fact(Fact::new(
        "current_user_process_count",
        serde_json::json!(current_uid().map(|uid| list
            .processes
            .iter()
            .filter(|process| process.uid == Some(uid))
            .count())),
        Severity::Info,
        vec!["compared process UIDs against current effective UID"],
    ))
    .with_fact(Fact::new(
        "root_process_count",
        serde_json::json!(root_process_count),
        Severity::Info,
        vec!["counted visible process summaries with uid 0"],
    ))
    .with_fact(Fact::new(
        "zombie_process_count",
        serde_json::json!(zombie_process_count),
        if zombie_process_count > 0 {
            Severity::Warning
        } else {
            Severity::Info
        },
        vec!["counted process states beginning with Z from /proc status data"],
    ));

    if let Some((process, rss)) = top_rss_process {
        report = report.with_fact(Fact::new(
            "top_rss_process",
            serde_json::json!({
                "pid": process.pid,
                "name": process.name,
                "uid": process.uid,
                "user": process.user,
                "vm_rss_bytes": rss
            }),
            Severity::Info,
            vec!["selected largest VmRSS value among visible process summaries".to_owned()],
        ));
    }
    if !list.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} process list warnings occurred; short-lived processes or permissions may limit completeness.",
            list.warnings.len()
        ));
    }
    report
        .with_safe_next_step(NextStep::read_only(
            "bbt --json proc list",
            "Fetch complete structured visible process inventory",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent net listeners",
            "Correlate processes with listening sockets",
        ))
}

fn summarize_process(pid: u32) -> io::Result<ProcSummary> {
    summarize_process_at(&PathBuf::from(format!("/proc/{pid}")), pid)
}

/// Read a procfs text file as bytes and decode lossily: kernel-provided
/// fields such as `comm` are not guaranteed to be valid UTF-8, and a bad
/// byte must not make the whole process invisible.
fn read_procfs_lossy(path: &Path) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

fn summarize_process_at(proc_path: &Path, pid: u32) -> io::Result<ProcSummary> {
    let content = read_procfs_lossy(&proc_path.join("status"))?;
    let mut summary = ProcSummary {
        pid,
        parent_pid: None,
        name: None,
        state: None,
        uid: None,
        user: None,
        vm_rss_bytes: None,
        command_line: Vec::new(),
    };
    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key {
                "Name" => summary.name = Some(value.to_owned()),
                "State" => summary.state = Some(value.to_owned()),
                "PPid" => summary.parent_pid = value.parse().ok(),
                "Uid" => summary.uid = value.split_whitespace().next().and_then(|v| v.parse().ok()),
                "VmRSS" => summary.vm_rss_bytes = parse_status_kib(value),
                _ => {}
            }
        }
    }
    summary.user = summary.uid.and_then(username_for_uid);
    if let Ok(bytes) = fs::read(proc_path.join("cmdline")) {
        summary.command_line = split_nul_arguments(&bytes)
            .iter()
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect();
    }
    Ok(summary)
}

fn current_uid() -> Option<u32> {
    Some(unsafe { libc::geteuid() })
}

pub fn inspect(pid: u32) -> ProcInspection {
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    let mut inspection = ProcInspection {
        pid,
        exists: proc_path.exists(),
        name: None,
        state: None,
        parent_pid: None,
        child_pids: Vec::new(),
        command_line: Vec::new(),
        command_line_bytes_hex: Vec::new(),
        executable: None,
        cwd: None,
        uid: None,
        user: None,
        gid: None,
        threads: None,
        memory: ProcMemory::default(),
        warnings: Vec::new(),
    };

    if !inspection.exists {
        inspection.warnings.push(format!(
            "/proc/{pid} does not exist; process is not visible"
        ));
        return inspection;
    }

    if let Err(error) = read_status(&proc_path.join("status"), &mut inspection) {
        inspection
            .warnings
            .push(format!("status unavailable for pid {pid}: {error}"));
    }

    if let Err(error) = read_stat(&proc_path.join("stat"), &mut inspection) {
        inspection
            .warnings
            .push(format!("stat unavailable for pid {pid}: {error}"));
    }

    match fs::read(proc_path.join("cmdline")) {
        Ok(bytes) => {
            let parts = split_nul_arguments(&bytes);
            inspection.command_line = parts
                .iter()
                .map(|part| String::from_utf8_lossy(part).into_owned())
                .collect();
            inspection.command_line_bytes_hex = parts.iter().map(|part| bytes_hex(part)).collect();
        }
        Err(error) => inspection
            .warnings
            .push(format!("cmdline unavailable for pid {pid}: {error}")),
    }

    inspection.executable = read_link_lossy(&proc_path.join("exe"), &mut inspection.warnings);
    inspection.cwd = read_link_lossy(&proc_path.join("cwd"), &mut inspection.warnings);
    inspection.child_pids = child_pids(pid, &mut inspection.warnings);
    inspection.user = inspection.uid.and_then(username_for_uid);

    inspection
}

pub fn agent_report(inspection: &ProcInspection, generated_at: impl Into<String>) -> AgentReport {
    let severity = if !inspection.exists {
        Severity::Warning
    } else if inspection.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };
    let process_label = inspection
        .name
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown process");
    let parent_label = inspection
        .parent_pid
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "unknown parent".to_owned());
    let summary = if inspection.exists {
        format!(
            "PID {} ({}) is {} with parent {}.",
            inspection.pid,
            process_label,
            inspection.state.as_deref().unwrap_or("unknown state"),
            parent_label
        )
    } else {
        format!("PID {} is not visible in /proc.", inspection.pid)
    };

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("proc", "inspect", Some(inspection.pid.to_string())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "process_exists",
        serde_json::json!(inspection.exists),
        Severity::Info,
        vec![format!(
            "/proc/{} exists: {}",
            inspection.pid, inspection.exists
        )],
    ))
    .with_fact(Fact::new(
        "process_state",
        serde_json::json!(inspection.state),
        Severity::Info,
        vec!["parsed from /proc/<pid>/status or /proc/<pid>/stat".to_owned()],
    ))
    .with_fact(Fact::new(
        "parent_pid",
        serde_json::json!(inspection.parent_pid),
        Severity::Info,
        vec!["parsed from /proc/<pid>/status or /proc/<pid>/stat".to_owned()],
    ))
    .with_fact(Fact::new(
        "child_pid_count",
        serde_json::json!(inspection.child_pids.len()),
        Severity::Info,
        vec!["scanned visible /proc numeric entries for matching PPid".to_owned()],
    ))
    .with_fact(Fact::new(
        "rss_bytes",
        serde_json::json!(inspection.memory.vm_rss_bytes),
        Severity::Info,
        vec!["parsed VmRSS from /proc/<pid>/status".to_owned()],
    ));

    if !inspection.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} process fields had collection warnings; process may have exited or permissions may be limited.",
            inspection.warnings.len()
        ));
    }

    report
        .with_safe_next_step(NextStep::read_only(
            format!("bbt --json proc inspect {}", inspection.pid),
            "Fetch complete structured process inspection data",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent host snapshot",
            "Inspect surrounding host pressure and context",
        ))
}

fn read_status(path: &Path, inspection: &mut ProcInspection) -> io::Result<()> {
    let content = read_procfs_lossy(path)?;
    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key {
                "Name" => inspection.name = Some(value.to_owned()),
                "State" => inspection.state = Some(value.to_owned()),
                "PPid" => inspection.parent_pid = value.parse().ok(),
                "Uid" => {
                    inspection.uid = value.split_whitespace().next().and_then(|v| v.parse().ok())
                }
                "Gid" => {
                    inspection.gid = value.split_whitespace().next().and_then(|v| v.parse().ok())
                }
                "Threads" => inspection.threads = value.parse().ok(),
                "VmPeak" => inspection.memory.vm_peak_bytes = parse_status_kib(value),
                "VmSize" => inspection.memory.vm_size_bytes = parse_status_kib(value),
                "VmRSS" => inspection.memory.vm_rss_bytes = parse_status_kib(value),
                "VmHWM" => inspection.memory.vm_hwm_bytes = parse_status_kib(value),
                _ => {}
            }
        }
    }
    Ok(())
}

fn read_stat(path: &Path, inspection: &mut ProcInspection) -> io::Result<()> {
    let content = read_procfs_lossy(path)?;
    let Some(close) = content.rfind(')') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing comm terminator",
        ));
    };
    let before = &content[..close];
    if inspection.name.is_none() {
        if let Some(open) = before.find('(') {
            inspection.name = Some(before[open + 1..].to_owned());
        }
    }
    let mut rest = content[close + 1..].split_whitespace();
    if inspection.state.is_none() {
        inspection.state = rest.next().map(|value| value.to_owned());
    } else {
        rest.next();
    }
    if inspection.parent_pid.is_none() {
        inspection.parent_pid = rest.next().and_then(|value| value.parse().ok());
    }
    Ok(())
}

fn split_nul_arguments(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut parts: Vec<Vec<u8>> = bytes
        .split(|byte| *byte == 0)
        .map(|part| part.to_vec())
        .collect();
    if parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    parts
}

pub fn escape_human(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

pub fn escape_human_argument(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_owned()
    } else {
        escape_human(value)
    }
}

fn parse_status_kib(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kib| kib.saturating_mul(1024))
}

fn read_link_lossy(path: &Path, warnings: &mut Vec<String>) -> Option<String> {
    match fs::read_link(path) {
        Ok(target) => Some(target.display().to_string()),
        Err(error) => {
            warnings.push(format!("{} unavailable: {error}", path.display()));
            None
        }
    }
}

fn child_pids(pid: u32, warnings: &mut Vec<String>) -> Vec<u32> {
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("/proc scan unavailable: {error}"));
            return Vec::new();
        }
    };

    let mut children = Vec::new();
    for entry in entries.flatten() {
        let Some(name) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let status_path = entry.path().join("status");
        if let Ok(content) = read_procfs_lossy(&status_path) {
            let parent = content.lines().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key == "PPid")
                    .then(|| value.trim().parse::<u32>().ok())
                    .flatten()
            });
            if parent == Some(pid) {
                children.push(name);
            }
        }
    }
    children.sort_unstable();
    children
}

fn username_for_uid(uid: u32) -> Option<String> {
    let passwd = unsafe { libc::getpwuid(uid) };
    if passwd.is_null() {
        return None;
    }
    let name_ptr = unsafe { (*passwd).pw_name };
    if name_ptr.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(name_ptr) };
    Some(name.to_string_lossy().into_owned())
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

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_proc_cmdline_preserves_interior_empty_argument() {
        let args = split_nul_arguments(b"/bin/demo\0\0--flag\0");

        assert_eq!(
            args,
            vec![b"/bin/demo".to_vec(), Vec::new(), b"--flag".to_vec()]
        );
    }

    #[test]
    fn escapes_control_characters_for_human_output() {
        assert_eq!(escape_human("safe\u{1b}[31m"), "safe\\u{1b}[31m");
    }

    #[test]
    fn parses_status_kib_as_bytes() {
        assert_eq!(parse_status_kib("123 kB"), Some(125_952));
    }

    fn empty_inspection(pid: u32) -> ProcInspection {
        ProcInspection {
            pid,
            exists: true,
            name: None,
            state: None,
            parent_pid: None,
            child_pids: Vec::new(),
            command_line: Vec::new(),
            command_line_bytes_hex: Vec::new(),
            executable: None,
            cwd: None,
            uid: None,
            user: None,
            gid: None,
            threads: None,
            memory: ProcMemory::default(),
            warnings: Vec::new(),
        }
    }

    fn non_utf8_status_bytes() -> Vec<u8> {
        let mut content = b"Name:\tweird-".to_vec();
        content.push(0xff);
        content.extend_from_slice(
            b"proc\nState:\tS (sleeping)\nPPid:\t1\nUid:\t1000\t1000\t1000\t1000\nVmRSS:\t4 kB\n",
        );
        content
    }

    #[test]
    fn status_with_non_utf8_name_parses_lossily_instead_of_erroring() {
        let temp = tempfile::tempdir().expect("tempdir");
        let status_path = temp.path().join("status");
        fs::write(&status_path, non_utf8_status_bytes()).expect("write status fixture");
        let mut inspection = empty_inspection(4242);

        read_status(&status_path, &mut inspection)
            .expect("non-UTF-8 status content must parse with lossy replacement");

        assert_eq!(inspection.name.as_deref(), Some("weird-\u{fffd}proc"));
        assert_eq!(inspection.state.as_deref(), Some("S (sleeping)"));
        assert_eq!(inspection.parent_pid, Some(1));
        assert_eq!(inspection.memory.vm_rss_bytes, Some(4096));
    }

    #[test]
    fn stat_with_non_utf8_comm_parses_lossily_instead_of_erroring() {
        let temp = tempfile::tempdir().expect("tempdir");
        let stat_path = temp.path().join("stat");
        let mut content = b"4242 (weird-".to_vec();
        content.push(0xff);
        content.extend_from_slice(b"proc) S 1 4242 4242 0 -1 4194304 0 0 0 0\n");
        fs::write(&stat_path, content).expect("write stat fixture");
        let mut inspection = empty_inspection(4242);

        read_stat(&stat_path, &mut inspection)
            .expect("non-UTF-8 stat content must parse with lossy replacement");

        assert_eq!(inspection.name.as_deref(), Some("weird-\u{fffd}proc"));
        assert_eq!(inspection.state.as_deref(), Some("S"));
        assert_eq!(inspection.parent_pid, Some(1));
    }

    #[test]
    fn process_summary_keeps_process_with_non_utf8_name_visible() {
        let temp = tempfile::tempdir().expect("tempdir");
        let proc_dir = temp.path().join("4242");
        fs::create_dir(&proc_dir).expect("proc dir fixture");
        fs::write(proc_dir.join("status"), non_utf8_status_bytes()).expect("write status");
        fs::write(proc_dir.join("cmdline"), b"weird\0--flag\0").expect("write cmdline");

        let summary = summarize_process_at(&proc_dir, 4242)
            .expect("a non-UTF-8 process name must not drop the process from the list");

        assert_eq!(summary.pid, 4242);
        assert_eq!(summary.name.as_deref(), Some("weird-\u{fffd}proc"));
        assert_eq!(summary.parent_pid, Some(1));
        assert_eq!(summary.uid, Some(1000));
        assert_eq!(summary.vm_rss_bytes, Some(4096));
        assert_eq!(summary.command_line, vec!["weird", "--flag"]);
    }

    #[test]
    fn missing_pid_reports_warning_without_error() {
        let inspection = inspect(999_999_999);

        assert!(!inspection.exists);
        assert!(!inspection.warnings.is_empty());
    }

    #[test]
    fn proc_list_agent_report_includes_triage_facts() {
        let list = ProcList {
            processes: vec![
                ProcSummary {
                    pid: 10,
                    parent_pid: Some(1),
                    name: Some("root-worker".to_owned()),
                    state: Some("S (sleeping)".to_owned()),
                    uid: Some(0),
                    user: Some("root".to_owned()),
                    vm_rss_bytes: Some(1024),
                    command_line: vec!["root-worker".to_owned()],
                },
                ProcSummary {
                    pid: 11,
                    parent_pid: Some(1),
                    name: Some("zombie".to_owned()),
                    state: Some("Z (zombie)".to_owned()),
                    uid: Some(1000),
                    user: Some("user".to_owned()),
                    vm_rss_bytes: Some(4096),
                    command_line: vec!["zombie".to_owned()],
                },
            ],
            process_count: 2,
            warnings: Vec::new(),
        };

        let report = list_agent_report(&list, "2026-01-01T00:00:00Z");
        let value = serde_json::to_value(report).expect("report serializes");
        let facts = value["facts"].as_array().expect("facts");

        assert!(facts.iter().any(|fact| fact["key"] == "top_rss_process"));
        assert!(facts
            .iter()
            .any(|fact| fact["key"] == "zombie_process_count"));
        assert!(facts.iter().any(|fact| fact["key"] == "root_process_count"));
    }
}
