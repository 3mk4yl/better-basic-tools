//! systemd service domain for Better Basic Tools.

use bbt_agent::{AgentReport, Fact, NextStep, Severity, Subject};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInspection {
    pub name: String,
    pub exists: bool,
    pub load_state: Option<String>,
    pub active_state: Option<String>,
    pub sub_state: Option<String>,
    pub unit_file_state: Option<String>,
    pub description: Option<String>,
    pub main_pid: Option<u32>,
    pub fragment_path: Option<String>,
    pub exec_main_status: Option<i32>,
    pub warnings: Vec<String>,
}

pub fn inspect(name: &str) -> ServiceInspection {
    inspect_with_systemctl("systemctl", name)
}

fn inspect_with_systemctl(systemctl: &str, name: &str) -> ServiceInspection {
    let mut inspection = ServiceInspection {
        name: name.to_owned(),
        exists: false,
        load_state: None,
        active_state: None,
        sub_state: None,
        unit_file_state: None,
        description: None,
        main_pid: None,
        fragment_path: None,
        exec_main_status: None,
        warnings: Vec::new(),
    };
    if name.starts_with('-') {
        inspection.warnings.push(format!(
            "unit name {} was rejected: names starting with '-' would be read as systemctl options",
            escape_human(name)
        ));
        return inspection;
    }
    let output = Command::new(systemctl)
        .args([
            "show",
            name,
            "--no-pager",
            "--property=Id,LoadState,ActiveState,SubState,UnitFileState,Description,MainPID,FragmentPath,ExecMainStatus",
        ])
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            inspection
                .warnings
                .push(format!("systemctl unavailable: {error}"));
            return inspection;
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && !stderr.trim().is_empty() {
        inspection.warnings.push(stderr.trim().to_owned());
    }
    let props = parse_properties(&stdout);
    inspection.load_state = props.get("LoadState").filter(|v| !v.is_empty()).cloned();
    inspection.active_state = props.get("ActiveState").filter(|v| !v.is_empty()).cloned();
    inspection.sub_state = props.get("SubState").filter(|v| !v.is_empty()).cloned();
    inspection.unit_file_state = props
        .get("UnitFileState")
        .filter(|v| !v.is_empty())
        .cloned();
    inspection.description = props.get("Description").filter(|v| !v.is_empty()).cloned();
    inspection.main_pid = props
        .get("MainPID")
        .and_then(|v| v.parse().ok())
        .filter(|pid| *pid != 0);
    inspection.fragment_path = props.get("FragmentPath").filter(|v| !v.is_empty()).cloned();
    inspection.exec_main_status = props.get("ExecMainStatus").and_then(|v| v.parse().ok());
    inspection.exists = inspection
        .load_state
        .as_deref()
        .is_some_and(|state| state != "not-found")
        || inspection.fragment_path.is_some();
    if !inspection.exists && inspection.warnings.is_empty() {
        inspection.warnings.push(format!(
            "service {} is not loaded or not found",
            inspection.name
        ));
    }
    inspection
}

pub fn agent_report(
    inspection: &ServiceInspection,
    generated_at: impl Into<String>,
) -> AgentReport {
    let service_state_problem = inspection.exists
        && inspection
            .active_state
            .as_deref()
            .is_some_and(|state| state != "active");
    let severity = if !inspection.exists || service_state_problem {
        Severity::Warning
    } else if inspection.active_state.as_deref() == Some("active") {
        Severity::Ok
    } else {
        Severity::Info
    };
    let summary = if inspection.exists {
        format!(
            "Service {} is {} / {}.",
            inspection.name,
            inspection.active_state.as_deref().unwrap_or("unknown"),
            inspection.sub_state.as_deref().unwrap_or("unknown")
        )
    } else {
        format!("Service {} is not loaded or not found.", inspection.name)
    };
    let mut report = AgentReport::new(
        generated_at,
        Subject::new("service", "inspect", Some(inspection.name.clone())),
        summary,
        severity,
    )
    .with_fact(Fact::new(
        "service_exists",
        serde_json::json!(inspection.exists),
        Severity::Info,
        vec!["queried systemctl show LoadState/FragmentPath"],
    ))
    .with_fact(Fact::new(
        "active_state",
        serde_json::json!(inspection.active_state),
        Severity::Info,
        vec!["queried systemctl show ActiveState"],
    ))
    .with_fact(Fact::new(
        "main_pid",
        serde_json::json!(inspection.main_pid),
        Severity::Info,
        vec!["queried systemctl show MainPID"],
    ))
    .with_fact(Fact::new(
        "service_state_problem",
        serde_json::json!(service_state_problem),
        if service_state_problem {
            Severity::Warning
        } else {
            Severity::Info
        },
        vec!["derived from systemctl ActiveState and service existence"],
    ));
    if !inspection.warnings.is_empty() {
        report = report.with_interpretation(format!(
            "{} service inspection warnings occurred.",
            inspection.warnings.len()
        ));
    }
    report
        .with_safe_next_step(NextStep::read_only(
            format!(
                "bbt --json service inspect {}",
                shell_quote(&inspection.name)
            ),
            "Fetch complete structured service state",
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt --agent net listeners",
            "Inspect local listening sockets related to services",
        ))
}

fn parse_properties(input: &str) -> BTreeMap<String, String> {
    input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub fn escape_human(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_systemctl_properties() {
        let props = parse_properties("LoadState=loaded\nActiveState=active\nMainPID=42\n");
        assert_eq!(props["LoadState"], "loaded");
        assert_eq!(props["MainPID"], "42");
    }

    #[test]
    fn option_like_unit_name_is_rejected_before_any_systemctl_invocation() {
        let inspection = inspect_with_systemctl("/bbt/definitely/missing/systemctl", "-version");

        assert_eq!(inspection.name, "-version");
        assert!(!inspection.exists);
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.contains("rejected")));
        assert!(
            !inspection
                .warnings
                .iter()
                .any(|warning| warning.contains("systemctl unavailable")),
            "an option-like name must be rejected before systemctl is invoked"
        );
    }

    #[test]
    fn unavailable_systemctl_returns_structured_warning() {
        let inspection = inspect_with_systemctl("/bbt/definitely/missing/systemctl", "ssh.service");

        assert_eq!(inspection.name, "ssh.service");
        assert!(!inspection.exists);
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.contains("systemctl unavailable")));
    }

    #[test]
    fn service_agent_report_flags_inactive_service_state() {
        let inspection = ServiceInspection {
            name: "demo.service".to_owned(),
            exists: true,
            load_state: Some("loaded".to_owned()),
            active_state: Some("inactive".to_owned()),
            sub_state: Some("dead".to_owned()),
            unit_file_state: Some("enabled".to_owned()),
            description: Some("demo".to_owned()),
            main_pid: None,
            fragment_path: Some("/etc/systemd/system/demo.service".to_owned()),
            exec_main_status: Some(1),
            warnings: Vec::new(),
        };

        let report = agent_report(&inspection, "2026-01-01T00:00:00Z");
        let value = serde_json::to_value(report).expect("report serializes");
        let facts = value["facts"].as_array().expect("facts");

        assert_eq!(value["severity"], "warning");
        assert!(facts
            .iter()
            .any(|fact| fact["key"] == "service_state_problem"));
    }
}
