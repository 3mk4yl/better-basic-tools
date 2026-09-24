//! Host snapshot domain for Better Basic Tools.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use bbt_core::filesystems::read_filesystems;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs;
use std::io;

pub use bbt_core::filesystems::FilesystemInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSnapshot {
    pub hostname: String,
    pub kernel: KernelInfo,
    pub os: OsInfo,
    pub current_user: UserInfo,
    pub cpu: CpuInfo,
    pub uptime: Option<UptimeInfo>,
    pub load_average: Option<LoadAverage>,
    pub memory: MemoryInfo,
    pub filesystems: Vec<FilesystemInfo>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub sysname: String,
    pub release: String,
    pub version: String,
    pub machine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub pretty_name: Option<String>,
    pub id: Option<String>,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: u32,
    pub effective_uid: u32,
    pub gid: u32,
    pub effective_gid: u32,
    pub username: Option<String>,
    pub home: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub logical_cpus: u64,
    pub model_name: Option<String>,
    pub vendor_id: Option<String>,
    pub cpu_mhz: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeInfo {
    pub seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub used_percent: Option<f64>,
}

pub fn snapshot() -> HostSnapshot {
    let mut warnings = Vec::new();
    let kernel = kernel_info().unwrap_or_else(|error| {
        warnings.push(format!("uname failed: {error}"));
        KernelInfo {
            sysname: "unknown".to_owned(),
            release: "unknown".to_owned(),
            version: "unknown".to_owned(),
            machine: "unknown".to_owned(),
        }
    });
    let hostname = hostname().unwrap_or_else(|| "unknown".to_owned());
    let os = os_info();
    let current_user = current_user_info();
    let cpu = read_cpu_info().unwrap_or_else(|error| {
        warnings.push(format!("cpu info unavailable: {error}"));
        CpuInfo {
            logical_cpus: 0,
            model_name: None,
            vendor_id: None,
            cpu_mhz: None,
        }
    });
    let uptime = read_uptime()
        .map_err(|error| warnings.push(format!("uptime unavailable: {error}")))
        .ok();
    let load_average = read_load_average()
        .map_err(|error| warnings.push(format!("load average unavailable: {error}")))
        .ok();
    let memory = read_memory_info().unwrap_or_else(|error| {
        warnings.push(format!("memory info unavailable: {error}"));
        MemoryInfo {
            total_bytes: 0,
            available_bytes: None,
            free_bytes: None,
            used_percent: None,
        }
    });
    let filesystems = read_filesystems(&mut warnings);

    HostSnapshot {
        hostname,
        kernel,
        os,
        current_user,
        cpu,
        uptime,
        load_average,
        memory,
        filesystems,
        warnings,
    }
}

pub fn agent_report(snapshot: &HostSnapshot, generated_at: impl Into<String>) -> AgentReport {
    let severity = if snapshot.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };
    let summary = format!(
        "{} is running {} {} on {} logical CPUs with {} memory used.",
        snapshot.hostname,
        snapshot.kernel.sysname,
        snapshot.kernel.release,
        snapshot.cpu.logical_cpus,
        snapshot
            .memory
            .used_percent
            .map(|value| format!("{value:.1}%"))
            .unwrap_or_else(|| "unknown".to_owned())
    );

    let mut report = AgentReport::new(
        generated_at,
        Subject::new("host", "snapshot", Some(snapshot.hostname.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "hostname",
        serde_json::json!(snapshot.hostname),
        Severity::Info,
        vec!["host snapshot hostname".to_owned()],
    ))
    .with_fact(Fact::new(
        "kernel_release",
        serde_json::json!(snapshot.kernel.release),
        Severity::Info,
        vec!["uname release".to_owned()],
    ))
    .with_fact(Fact::new(
        "cpu_logical_cpus",
        serde_json::json!(snapshot.cpu.logical_cpus),
        Severity::Info,
        vec!["counted processor entries from /proc/cpuinfo".to_owned()],
    ))
    .with_fact(Fact::new(
        "current_user",
        serde_json::json!({
            "uid": snapshot.current_user.uid,
            "effective_uid": snapshot.current_user.effective_uid,
            "username": snapshot.current_user.username,
        }),
        Severity::Info,
        vec!["queried current process uid/gid and passwd database".to_owned()],
    ))
    .with_fact(Fact::new(
        "memory_used_percent",
        serde_json::json!(snapshot.memory.used_percent),
        Severity::Info,
        vec!["derived from /proc/meminfo".to_owned()],
    ))
    .with_fact(Fact::new(
        "filesystem_count",
        serde_json::json!(snapshot.filesystems.len()),
        Severity::Info,
        vec!["/proc/mounts entries with statvfs data where available".to_owned()],
    ));

    if !snapshot.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} snapshot fields had collection warnings.",
            snapshot.warnings.len()
        ));
    }

    if let (Some(load), logical_cpus) = (&snapshot.load_average, snapshot.cpu.logical_cpus) {
        if logical_cpus > 0 {
            let load_per_cpu = load.one_minute / logical_cpus as f64;
            report = report.with_interpretation(format!(
                "1-minute load is {:.2} across {} logical CPUs ({:.2} per CPU).",
                load.one_minute, logical_cpus, load_per_cpu
            ));
        }
    }

    if let Some(memory_used) = snapshot.memory.used_percent {
        let level = if memory_used >= 90.0 {
            "high"
        } else if memory_used >= 75.0 {
            "elevated"
        } else {
            "normal"
        };
        report = report.with_interpretation(format!("Memory use is {level} at {memory_used:.1}%."));
    }

    report
        .with_safe_next_step(NextStep::read_only(
            "bbt --json host snapshot",
            "Fetch complete structured host snapshot data",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent disk usage --depth 1 -- /",
            "Inspect root filesystem disk usage with agent-focused output",
        ))
}

fn kernel_info() -> io::Result<KernelInfo> {
    let mut uts = std::mem::MaybeUninit::<libc::utsname>::uninit();
    if unsafe { libc::uname(uts.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let uts = unsafe { uts.assume_init() };
    Ok(KernelInfo {
        sysname: c_chars_to_string(&uts.sysname),
        release: c_chars_to_string(&uts.release),
        version: c_chars_to_string(&uts.version),
        machine: c_chars_to_string(&uts.machine),
    })
}

fn hostname() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn current_user_info() -> UserInfo {
    let uid = unsafe { libc::getuid() };
    let effective_uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getgid() };
    let effective_gid = unsafe { libc::getegid() };
    let (username, home, shell) = passwd_info(effective_uid);

    UserInfo {
        uid,
        effective_uid,
        gid,
        effective_gid,
        username,
        home,
        shell,
    }
}

fn passwd_info(uid: libc::uid_t) -> (Option<String>, Option<String>, Option<String>) {
    let passwd = unsafe { libc::getpwuid(uid) };
    if passwd.is_null() {
        return (None, None, None);
    }

    let username = c_string_field(unsafe { (*passwd).pw_name });
    let home = c_string_field(unsafe { (*passwd).pw_dir });
    let shell = c_string_field(unsafe { (*passwd).pw_shell });

    (username, home, shell)
}

fn c_string_field(ptr: *const libc::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

fn read_cpu_info() -> io::Result<CpuInfo> {
    let content = fs::read_to_string("/proc/cpuinfo")?;
    parse_cpu_info(&content)
}

fn parse_cpu_info(content: &str) -> io::Result<CpuInfo> {
    let mut logical_cpus = 0_u64;
    let mut model_name = None;
    let mut vendor_id = None;
    let mut cpu_mhz = None;

    for line in content.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "processor" => logical_cpus = logical_cpus.saturating_add(1),
            "model name" if model_name.is_none() => model_name = Some(value.to_owned()),
            "vendor_id" if vendor_id.is_none() => vendor_id = Some(value.to_owned()),
            "cpu MHz" if cpu_mhz.is_none() => cpu_mhz = value.parse::<f64>().ok(),
            _ => {}
        }
    }

    if logical_cpus == 0 {
        logical_cpus = std::thread::available_parallelism()
            .map(|count| count.get() as u64)
            .unwrap_or(0);
    }

    if logical_cpus == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no CPUs found in /proc/cpuinfo",
        ));
    }

    Ok(CpuInfo {
        logical_cpus,
        model_name,
        vendor_id,
        cpu_mhz,
    })
}

fn os_info() -> OsInfo {
    let values = fs::read_to_string("/etc/os-release")
        .ok()
        .map(|content| parse_os_release(&content))
        .unwrap_or_default();

    OsInfo {
        pretty_name: values.get("PRETTY_NAME").cloned(),
        id: values.get("ID").cloned(),
        version_id: values.get("VERSION_ID").cloned(),
    }
}

fn read_uptime() -> io::Result<UptimeInfo> {
    let content = fs::read_to_string("/proc/uptime")?;
    let seconds = content
        .split_whitespace()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing uptime seconds"))?
        .parse::<f64>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(UptimeInfo { seconds })
}

fn read_load_average() -> io::Result<LoadAverage> {
    let content = fs::read_to_string("/proc/loadavg")?;
    let mut parts = content.split_whitespace();
    let one_minute = parse_f64(parts.next(), "one minute load")?;
    let five_minutes = parse_f64(parts.next(), "five minute load")?;
    let fifteen_minutes = parse_f64(parts.next(), "fifteen minute load")?;
    Ok(LoadAverage {
        one_minute,
        five_minutes,
        fifteen_minutes,
    })
}

fn read_memory_info() -> io::Result<MemoryInfo> {
    let content = fs::read_to_string("/proc/meminfo")?;
    let mut values = BTreeMap::new();
    for line in content.lines() {
        if let Some((key, rest)) = line.split_once(':') {
            if let Some(kib) = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok())
            {
                values.insert(key.to_owned(), kib.saturating_mul(1024));
            }
        }
    }

    let total_bytes = values.get("MemTotal").copied().unwrap_or(0);
    let available_bytes = values.get("MemAvailable").copied();
    let free_bytes = values.get("MemFree").copied();
    let used_percent = available_bytes
        .and_then(|available| percent(total_bytes.saturating_sub(available), total_bytes));

    Ok(MemoryInfo {
        total_bytes,
        available_bytes,
        free_bytes,
        used_percent,
    })
}

fn parse_os_release(content: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.to_owned(), value.trim_matches('"').to_owned());
        }
    }
    values
}

fn parse_f64(value: Option<&str>, label: &str) -> io::Result<f64> {
    value
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing {label}")))?
        .parse::<f64>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn percent(used: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some((used as f64 / total as f64) * 100.0)
    }
}

fn c_chars_to_string(chars: &[libc::c_char]) -> String {
    unsafe { CStr::from_ptr(chars.as_ptr()) }
        .to_string_lossy()
        .into_owned()
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
    fn parses_os_release_values() {
        let parsed = parse_os_release("PRETTY_NAME=\"Demo Linux\"\nID=demo\n");

        assert_eq!(parsed.get("PRETTY_NAME"), Some(&"Demo Linux".to_owned()));
        assert_eq!(parsed.get("ID"), Some(&"demo".to_owned()));
    }

    #[test]
    fn snapshot_has_required_core_fields() {
        let snapshot = snapshot();

        assert!(!snapshot.hostname.is_empty());
        assert!(!snapshot.kernel.sysname.is_empty());
        assert!(snapshot.memory.total_bytes > 0);
        assert_eq!(snapshot.current_user.effective_uid, unsafe {
            libc::geteuid()
        });
        assert!(snapshot.cpu.logical_cpus > 0);
    }

    #[test]
    fn parses_cpu_info_summary() {
        let cpu = parse_cpu_info(
            "processor : 0\nvendor_id : GenuineIntel\nmodel name : Demo CPU\ncpu MHz : 2400.000\nprocessor : 1\n",
        )
        .expect("cpu info");

        assert_eq!(cpu.logical_cpus, 2);
        assert_eq!(cpu.vendor_id, Some("GenuineIntel".to_owned()));
        assert_eq!(cpu.model_name, Some("Demo CPU".to_owned()));
        assert_eq!(cpu.cpu_mhz, Some(2400.0));
    }
}
