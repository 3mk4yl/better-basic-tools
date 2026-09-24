pub mod filesystems;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelope<T> {
    pub schema: String,
    pub generated_at: String,
    pub command: CommandContext,
    pub data: T,
}

impl<T> SchemaEnvelope<T> {
    pub fn new(schema: impl Into<String>, command: CommandContext, data: T) -> Self {
        Self {
            schema: schema.into(),
            generated_at: current_timestamp(),
            command,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContext {
    pub argv: Vec<String>,
    pub argv_bytes_hex: Vec<String>,
    pub cwd: Option<String>,
    pub effective_uid: Option<u32>,
}

impl CommandContext {
    pub fn new(
        argv: Vec<String>,
        argv_bytes_hex: Vec<String>,
        cwd: Option<String>,
        effective_uid: Option<u32>,
    ) -> Self {
        Self {
            argv,
            argv_bytes_hex,
            cwd,
            effective_uid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
    Yaml,
    Agent,
}

pub fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
