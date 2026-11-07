pub mod json_export;
pub mod terminal;

use serde::{Deserialize, Serialize};

/// Supported output formatting modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
    Ndjson,
}
