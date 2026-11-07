use serde::Serialize;
use crate::engine::ProbeSample;
use crate::stats::SessionSummary;

pub struct JsonReporter;

impl JsonReporter {
    /// Format final session summary as pretty-printed JSON.
    pub fn to_pretty_json(summary: &SessionSummary) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(summary)
    }

    /// Serialize any item to a single compact NDJSON line.
    pub fn to_ndjson_line<T: Serialize>(item: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string(item)
    }

    /// Format entire session telemetry as streaming newline-delimited JSON (NDJSON).
    /// Each sample is output as a probe record, followed by the final session summary record.
    pub fn to_ndjson_stream(
        samples: &[ProbeSample],
        summary: &SessionSummary,
    ) -> Result<String, serde_json::Error> {
        let mut out = String::with_capacity(samples.len() * 128 + 256);
        for s in samples {
            let line = serde_json::to_string(s)?;
            out.push_str(&line);
            out.push('\n');
        }
        let summary_line = serde_json::to_string(summary)?;
        out.push_str(&summary_line);
        out.push('\n');
        Ok(out)
    }
}
