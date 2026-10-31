//! itcmdr shared types (v0.15.0): the service plugin error contract.
//!
//! A plugin error has a **criticality** (critical, cautious, informational), the plugin that
//! raised it, a message and a backtrace. It travels as `PluginError` in `proto/plugin.proto`
//! (`StageResponse.error`) and is written by the server as an itcmdr.log/v1 line with
//! `event: "plugin-error"` (docs/LOG-STANDARD.md). v0.16.0 turns each one into an alert.
//!
//! ```
//! use itcmdr_common::{ErrorCriticality, ServicePluginError};
//!
//! #[derive(Debug, ServicePluginError)]
//! #[service_plugin(name = "disk-free")]
//! pub struct DiskError {
//!     error: String,
//!     criticality: ErrorCriticality,
//!     backtrace: String,
//!     service_plugin: String,
//!     drive: String, // extra fields go into the record's context
//! }
//!
//! let e = DiskError::new("C: is 97% full", ErrorCriticality::Critical);
//! let r = e.to_record();
//! assert_eq!(r.service_plugin, "disk-free");
//! assert_eq!(r.criticality.log_level(), "error");
//! assert!(!r.backtrace.is_empty());
//! ```
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub use itcmdr_common_derive::ServicePluginError;

/// Largest backtrace kept in a record, log line or proto message (16 KiB).
pub const MAX_BACKTRACE: usize = 16 * 1024;
/// Largest error message kept (4 KiB).
pub const MAX_ERROR: usize = 4 * 1024;

/// How bad a plugin error is. Ordered: `Critical < Cautious < Informational`
/// (sort ascending to get the worst first).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCriticality {
    /// Something is broken and needs a person now (pages in v0.16.0).
    Critical,
    /// Something is wrong or degraded; look at it soon. The default for uncaught errors.
    #[default]
    Cautious,
    /// Worth knowing, nothing to do now.
    Informational,
}

impl ErrorCriticality {
    pub const ALL: [ErrorCriticality; 3] = [Self::Critical, Self::Cautious, Self::Informational];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Cautious => "cautious",
            Self::Informational => "informational",
        }
    }

    /// itcmdr.log/v1 `level` for a plugin-error line: critical → error, cautious → warn,
    /// informational → info.
    pub fn log_level(self) -> &'static str {
        match self {
            Self::Critical => "error",
            Self::Cautious => "warn",
            Self::Informational => "info",
        }
    }

    /// Case-insensitive; also accepts `info` and `warn`/`warning`/`caution`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "critical" | "crit" => Some(Self::Critical),
            "cautious" | "caution" | "warn" | "warning" => Some(Self::Cautious),
            "informational" | "info" => Some(Self::Informational),
            _ => None,
        }
    }
}

impl fmt::Display for ErrorCriticality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ErrorCriticality {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::parse(s).ok_or_else(|| {
            format!("unknown criticality {s:?} (use critical, cautious or informational)")
        })
    }
}

/// The wire/log shape of a plugin error (`PluginError` in plugin.proto, `plugin-error` lines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginErrorRecord {
    pub service_plugin: String,
    /// Error type, for dedupe (plugin + kind + machine). The struct name for derived errors,
    /// the exception type for PowerShell.
    pub kind: String,
    pub criticality: ErrorCriticality,
    pub error: String,
    pub backtrace: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
}

impl PluginErrorRecord {
    pub fn new(service_plugin: &str, criticality: ErrorCriticality, error: &str) -> Self {
        Self {
            service_plugin: service_plugin.into(),
            kind: String::new(),
            criticality,
            error: truncate(error, MAX_ERROR),
            backtrace: String::new(),
            context: BTreeMap::new(),
        }
    }

    /// The same record with message and backtrace cut to their limits.
    pub fn bounded(mut self) -> Self {
        self.error = truncate(&self.error, MAX_ERROR);
        self.backtrace = truncate(&self.backtrace, MAX_BACKTRACE);
        self
    }

    /// Fields of the itcmdr.log/v1 `plugin-error` line (besides the common envelope fields):
    /// `criticality`, `error`, `backtrace`, `kind` and, when present, `context`.
    pub fn log_fields(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("criticality".into(), self.criticality.as_str().into());
        m.insert("error".into(), truncate(&self.error, MAX_ERROR).into());
        m.insert(
            "backtrace".into(),
            truncate(&self.backtrace, MAX_BACKTRACE).into(),
        );
        if !self.kind.is_empty() {
            m.insert("kind".into(), self.kind.clone().into());
        }
        if !self.context.is_empty() {
            m.insert(
                "context".into(),
                serde_json::to_value(&self.context).unwrap_or_default(),
            );
        }
        m
    }
}

impl fmt::Display for PluginErrorRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.criticality, self.error)
    }
}

/// An error a service plugin raises. Derive it with `#[derive(ServicePluginError)]`.
pub trait ServicePluginError: std::error::Error + Send + Sync + 'static {
    fn criticality(&self) -> ErrorCriticality;
    fn service_plugin(&self) -> &str;
    fn backtrace_text(&self) -> &str;
    /// Error type for dedupe; the derive uses the struct name.
    fn kind(&self) -> &str {
        ""
    }
    /// Extra fields; the derive fills it from every field besides the four required ones.
    fn context(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn to_record(&self) -> PluginErrorRecord {
        PluginErrorRecord {
            service_plugin: self.service_plugin().to_string(),
            kind: self.kind().to_string(),
            criticality: self.criticality(),
            error: self.to_string(),
            backtrace: self.backtrace_text().to_string(),
            context: self.context(),
        }
        .bounded()
    }
}

/// A backtrace of the caller, always captured (does not depend on `RUST_BACKTRACE`), cut to
/// [`MAX_BACKTRACE`].
pub fn capture_backtrace() -> String {
    truncate(
        &std::backtrace::Backtrace::force_capture().to_string(),
        MAX_BACKTRACE,
    )
}

/// Cut `s` to at most `max` bytes on a char boundary, marking the cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    const MARK: &str = "\n…[truncated]";
    let mut end = max.saturating_sub(MARK.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{MARK}", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criticality_names_levels_and_order() {
        assert_eq!(
            serde_json::to_string(&ErrorCriticality::Informational).unwrap(),
            "\"informational\""
        );
        assert_eq!(
            ErrorCriticality::parse("WARN"),
            Some(ErrorCriticality::Cautious)
        );
        assert_eq!(ErrorCriticality::parse("nope"), None);
        assert!("x".parse::<ErrorCriticality>().is_err());
        let lv: Vec<_> = ErrorCriticality::ALL
            .iter()
            .map(|c| c.log_level())
            .collect();
        assert_eq!(lv, ["error", "warn", "info"]);
        assert!(ErrorCriticality::Critical < ErrorCriticality::Informational);
        assert_eq!(ErrorCriticality::default(), ErrorCriticality::Cautious);
    }

    #[test]
    fn truncate_keeps_char_boundaries() {
        let s = "é".repeat(20_000);
        let t = truncate(&s, MAX_BACKTRACE);
        assert!(t.len() <= MAX_BACKTRACE);
        assert!(t.ends_with("[truncated]"));
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn record_log_fields() {
        let mut r = PluginErrorRecord::new("p", ErrorCriticality::Critical, "boom");
        r.backtrace = "x".repeat(MAX_BACKTRACE * 2);
        r.context.insert("drive".into(), "C:".into());
        let f = r.log_fields();
        assert_eq!(f["criticality"], "critical");
        assert!(f["backtrace"].as_str().unwrap().len() <= MAX_BACKTRACE);
        assert_eq!(f["context"]["drive"], "C:");
        assert!(!f.contains_key("kind"));
        let back: PluginErrorRecord = serde_json::from_value(serde_json::json!({
            "service_plugin": "p", "kind": "", "criticality": "cautious", "error": "e", "backtrace": ""
        }))
        .unwrap();
        assert!(back.context.is_empty());
    }
}
