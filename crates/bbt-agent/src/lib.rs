use serde::{Deserialize, Serialize};

pub const AGENT_REPORT_SCHEMA_V1: &str = "bbt.agent.report.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    pub schema: String,
    pub generated_at: String,
    pub subject: Subject,
    pub summary: String,
    pub severity: Severity,
    pub facts: Vec<Fact>,
    pub interpretation: Vec<String>,
    pub safe_next_steps: Vec<NextStep>,
    pub risky_next_steps: Vec<NextStep>,
}

impl AgentReport {
    pub fn new(
        generated_at: impl Into<String>,
        subject: Subject,
        summary: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            schema: AGENT_REPORT_SCHEMA_V1.to_owned(),
            generated_at: generated_at.into(),
            subject,
            summary: summary.into(),
            severity,
            facts: Vec::new(),
            interpretation: Vec::new(),
            safe_next_steps: Vec::new(),
            risky_next_steps: Vec::new(),
        }
    }

    pub fn with_fact(mut self, fact: Fact) -> Self {
        self.facts.push(fact);
        self
    }

    pub fn with_interpretation(mut self, interpretation: impl Into<String>) -> Self {
        self.interpretation.push(interpretation.into());
        self
    }

    pub fn with_safe_next_step(mut self, next_step: NextStep) -> Self {
        self.safe_next_steps.push(next_step);
        self
    }

    pub fn with_risky_next_step(mut self, next_step: NextStep) -> Self {
        self.risky_next_steps.push(next_step);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub domain: String,
    pub command: String,
    pub target: Option<String>,
}

impl Subject {
    pub fn new(
        domain: impl Into<String>,
        command: impl Into<String>,
        target: Option<impl Into<String>>,
    ) -> Self {
        Self {
            domain: domain.into(),
            command: command.into(),
            target: target.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Ok,
    Info,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub key: String,
    pub value: serde_json::Value,
    pub severity: Severity,
    pub evidence: Vec<String>,
}

impl Fact {
    pub fn new(
        key: impl Into<String>,
        value: serde_json::Value,
        severity: Severity,
        evidence: Vec<impl Into<String>>,
    ) -> Self {
        Self {
            key: key.into(),
            value,
            severity,
            evidence: evidence.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextStep {
    pub command: String,
    pub risk: Risk,
    pub purpose: String,
    pub requires_confirmation: bool,
}

impl NextStep {
    pub fn read_only(command: impl Into<String>, purpose: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            risk: Risk::ReadOnly,
            purpose: purpose.into(),
            requires_confirmation: false,
        }
    }

    pub fn risky(command: impl Into<String>, risk: Risk, purpose: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            risk,
            purpose: purpose.into(),
            requires_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    ReadOnly,
    MutatingLow,
    MutatingMedium,
    DestructiveLow,
    DestructiveMedium,
    DestructiveHigh,
    Privileged,
    Networked,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_agent_report_with_stable_schema_and_kebab_case_values() {
        let report = AgentReport::new(
            "2026-05-29T12:00:00Z",
            Subject::new("fs", "inspect", Some("Cargo.toml")),
            "Cargo.toml is a readable regular file.",
            Severity::Ok,
        )
        .with_fact(Fact::new(
            "path_exists",
            json!(true),
            Severity::Info,
            vec!["Cargo.toml exists"],
        ))
        .with_safe_next_step(NextStep::read_only(
            "bbt fs inspect Cargo.toml --json",
            "Fetch complete raw filesystem metadata",
        ));

        let serialized = serde_json::to_value(report).expect("report serializes");

        assert_eq!(serialized["schema"], "bbt.agent.report.v1");
        assert_eq!(serialized["generated_at"], "2026-05-29T12:00:00Z");
        assert_eq!(serialized["subject"]["domain"], "fs");
        assert_eq!(serialized["severity"], "ok");
        assert_eq!(serialized["facts"][0]["severity"], "info");
        assert_eq!(serialized["safe_next_steps"][0]["risk"], "read-only");
    }

    #[test]
    fn risky_next_steps_require_confirmation_by_default() {
        let step = NextStep::risky(
            "sudo bbt fs inspect /var/log/nginx/access.log --agent",
            Risk::Privileged,
            "Inspect path with elevated privileges",
        );

        assert!(step.requires_confirmation);
    }
}
