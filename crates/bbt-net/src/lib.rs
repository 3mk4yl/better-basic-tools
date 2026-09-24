//! Network domain for Better Basic Tools.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::net::Ipv6Addr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerReport {
    pub listeners: Vec<Listener>,
    pub listener_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listener {
    pub protocol: String,
    pub address: String,
    pub port: u16,
    pub inode: Option<u64>,
    pub pids: Vec<u32>,
    pub processes: Vec<String>,
}

pub fn listeners() -> ListenerReport {
    let mut warnings = Vec::new();
    let owners = socket_owners(&mut warnings);
    let mut listeners = Vec::new();
    for (path, protocol) in [
        ("/proc/net/tcp", "tcp"),
        ("/proc/net/tcp6", "tcp6"),
        ("/proc/net/udp", "udp"),
        ("/proc/net/udp6", "udp6"),
    ] {
        match parse_proc_net(path, protocol, &owners) {
            Ok(mut values) => listeners.append(&mut values),
            Err(error) => warnings.push(format!("{path} unavailable: {error}")),
        }
    }
    listeners
        .sort_by(|a, b| (&a.protocol, a.port, &a.address).cmp(&(&b.protocol, b.port, &b.address)));
    let listener_count = listeners.len();
    ListenerReport {
        listeners,
        listener_count,
        warnings,
    }
}

pub fn agent_report(report: &ListenerReport, generated_at: impl Into<String>) -> AgentReport {
    let severity = if report.warnings.is_empty() {
        Severity::Ok
    } else {
        Severity::Info
    };
    let externally_bound_count = report
        .listeners
        .iter()
        .filter(|listener| is_externally_bound(&listener.address))
        .count();
    let privileged_listener_count = report
        .listeners
        .iter()
        .filter(|listener| listener.port < 1024)
        .count();
    let unattributed_listener_count = report
        .listeners
        .iter()
        .filter(|listener| listener.pids.is_empty())
        .count();

    let mut agent = AgentReport::new(
        generated_at,
        Subject::new("net", "listeners", Some("local")),
        format!(
            "Observed {} local listening sockets.",
            report.listener_count
        ),
        severity,
    )
    .with_fact(Fact::new(
        "listener_count",
        serde_json::json!(report.listener_count),
        Severity::Info,
        vec!["parsed /proc/net/{tcp,tcp6,udp,udp6} entries in listener state"],
    ))
    .with_fact(Fact::new(
        "socket_owner_mapping_available",
        serde_json::json!(report
            .listeners
            .iter()
            .any(|listener| !listener.pids.is_empty())),
        Severity::Info,
        vec!["scanned visible /proc/<pid>/fd symlinks for socket inodes"],
    ))
    .with_fact(Fact::new(
        "externally_bound_listener_count",
        serde_json::json!(externally_bound_count),
        if externally_bound_count > 0 {
            Severity::Warning
        } else {
            Severity::Info
        },
        vec!["counted listeners bound to wildcard addresses 0.0.0.0 or ::"],
    ))
    .with_fact(Fact::new(
        "privileged_listener_count",
        serde_json::json!(privileged_listener_count),
        if privileged_listener_count > 0 {
            Severity::Warning
        } else {
            Severity::Info
        },
        vec!["counted listeners with local port below 1024"],
    ))
    .with_fact(Fact::new(
        "unattributed_listener_count",
        serde_json::json!(unattributed_listener_count),
        Severity::Info,
        vec!["counted listeners without visible process attribution"],
    ));
    if !report.warnings.is_empty() {
        agent = agent.with_interpretation(format!(
            "{} listener collection warnings occurred; permissions or kernel support may limit process attribution.",
            report.warnings.len()
        ));
    }
    agent
        .with_safe_next_step(NextStep::read_only(
            "bbt --json net listeners",
            "Fetch complete structured listener inventory",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent proc list",
            "Correlate listeners with the local process inventory",
        ))
}

fn is_externally_bound(address: &str) -> bool {
    matches!(address, "0.0.0.0" | "::")
}

fn parse_proc_net(
    path: &str,
    protocol: &str,
    owners: &BTreeMap<u64, Vec<ProcessOwner>>,
) -> io::Result<Vec<Listener>> {
    let content = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }
        let state = fields[3];
        // TCP LISTEN is 0A. UDP sockets with state 07 are included as listening-ish unconnected sockets.
        if protocol.starts_with("tcp") && state != "0A" {
            continue;
        }
        if protocol.starts_with("udp") && state != "07" {
            continue;
        }
        let Some((address, port)) = parse_addr_port(fields[1], protocol.ends_with('6')) else {
            continue;
        };
        let inode = fields[9].parse::<u64>().ok();
        let mut pids = BTreeSet::new();
        let mut processes = BTreeSet::new();
        if let Some(inode) = inode {
            if let Some(values) = owners.get(&inode) {
                for owner in values {
                    pids.insert(owner.pid);
                    if let Some(name) = &owner.name {
                        processes.insert(name.clone());
                    }
                }
            }
        }
        out.push(Listener {
            protocol: protocol.to_owned(),
            address,
            port,
            inode,
            pids: pids.into_iter().collect(),
            processes: processes.into_iter().collect(),
        });
    }
    Ok(out)
}

fn parse_addr_port(value: &str, ipv6: bool) -> Option<(String, u16)> {
    let (addr_hex, port_hex) = value.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let address = if ipv6 {
        parse_ipv6_proc_address(addr_hex)?
    } else {
        let raw = u32::from_str_radix(addr_hex, 16).ok()?;
        let bytes = raw.to_le_bytes();
        format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    };
    Some((address, port))
}

fn parse_ipv6_proc_address(addr_hex: &str) -> Option<String> {
    if addr_hex.len() != 32 {
        return None;
    }

    let mut bytes = [0_u8; 16];
    for (chunk_index, chunk) in addr_hex.as_bytes().chunks_exact(8).enumerate() {
        let chunk = std::str::from_utf8(chunk).ok()?;
        let value = u32::from_str_radix(chunk, 16).ok()?;
        bytes[chunk_index * 4..chunk_index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    Some(Ipv6Addr::from(bytes).to_string())
}

#[derive(Debug, Clone)]
struct ProcessOwner {
    pid: u32,
    name: Option<String>,
}

fn socket_owners(warnings: &mut Vec<String>) -> BTreeMap<u64, Vec<ProcessOwner>> {
    let mut owners: BTreeMap<u64, Vec<ProcessOwner>> = BTreeMap::new();
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("/proc scan unavailable: {error}"));
            return owners;
        }
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let name = read_process_name(pid);
        let fd_dir = entry.path().join("fd");
        let Ok(fds) = fs::read_dir(&fd_dir) else {
            continue;
        };
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            let text = target.to_string_lossy();
            let Some(inode_text) = text
                .strip_prefix("socket:[")
                .and_then(|v| v.strip_suffix(']'))
            else {
                continue;
            };
            if let Ok(inode) = inode_text.parse::<u64>() {
                owners.entry(inode).or_default().push(ProcessOwner {
                    pid,
                    name: name.clone(),
                });
            }
        }
    }
    owners
}

fn read_process_name(pid: u32) -> Option<String> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("Name:\t").map(str::to_owned))
}

pub fn escape_human(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_proc_net_address() {
        assert_eq!(
            parse_addr_port("0100007F:1F90", false),
            Some(("127.0.0.1".to_owned(), 8080))
        );
    }

    #[test]
    fn parses_ipv6_proc_net_loopback_address() {
        assert_eq!(
            parse_addr_port("00000000000000000000000001000000:1F90", true),
            Some(("::1".to_owned(), 8080))
        );
    }

    #[test]
    fn net_agent_report_includes_external_listener_triage() {
        let report = ListenerReport {
            listeners: vec![Listener {
                protocol: "tcp".to_owned(),
                address: "0.0.0.0".to_owned(),
                port: 22,
                inode: Some(42),
                pids: vec![123],
                processes: vec!["sshd".to_owned()],
            }],
            listener_count: 1,
            warnings: Vec::new(),
        };

        let agent = agent_report(&report, "2026-01-01T00:00:00Z");
        let value = serde_json::to_value(agent).expect("agent report serializes");
        let facts = value["facts"].as_array().expect("facts");

        assert!(facts
            .iter()
            .any(|fact| fact["key"] == "externally_bound_listener_count"));
        assert!(facts
            .iter()
            .any(|fact| fact["key"] == "privileged_listener_count"));
    }
}
