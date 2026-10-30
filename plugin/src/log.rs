//! `itcmdr.log/v1` structured JSONL (decision 11, docs/LOG-STANDARD.md): writer helpers and the
//! validator the server uses at ingest.
use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "itcmdr.log/v1";
pub const REQUIRED: [&str; 10] = [
    "schema",
    "ts",
    "level",
    "plugin",
    "plugin_version",
    "run_id",
    "machine",
    "stage",
    "event",
    "message",
];
pub const LEVELS: [&str; 4] = ["debug", "info", "warn", "error"];
pub const STATUSES: [&str; 5] = ["ok", "fail", "repaired", "skipped", "excepted"];
pub const KINDS: [&str; 3] = ["counter", "gauge", "histogram"];

/// One metric in a line's `metrics` list.
#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub kind: String,
    pub labels: Vec<(String, String)>,
}

impl Metric {
    pub fn gauge(name: &str, value: f64, unit: &str) -> Self {
        Self {
            name: name.into(),
            value,
            unit: unit.into(),
            kind: "gauge".into(),
            labels: vec![],
        }
    }
    pub fn counter(name: &str, value: f64) -> Self {
        Self {
            kind: "counter".into(),
            ..Self::gauge(name, value, "")
        }
    }
    pub fn label(mut self, k: &str, v: &str) -> Self {
        self.labels.push((k.into(), v.into()));
        self
    }
    fn to_json(&self) -> Value {
        let labels: Map<String, Value> = self
            .labels
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        json!({"name": self.name, "value": self.value, "unit": self.unit, "kind": self.kind, "labels": labels})
    }
}

/// Writes conforming lines to one file (append). Plugins never format lines themselves.
#[derive(Debug, Clone)]
pub struct Logger {
    pub path: PathBuf,
    pub plugin: String,
    pub plugin_version: String,
    pub run_id: String,
    pub machine: String,
    pub stage: String,
}

impl Logger {
    pub fn for_ctx(plugin: &str, version: &str, ctx: &crate::proto::RunContext) -> Self {
        let path = if ctx.log_path.is_empty() {
            std::env::var(crate::ENV_LOG_PATH).unwrap_or_default()
        } else {
            ctx.log_path.clone()
        };
        Self {
            path: path.into(),
            plugin: plugin.into(),
            plugin_version: version.into(),
            run_id: ctx.run_id.clone(),
            machine: ctx.machine.clone(),
            stage: if ctx.stage.is_empty() {
                "execute".into()
            } else {
                ctx.stage.clone()
            },
        }
    }

    /// Build (but don't write) a line.
    pub fn line(
        &self,
        level: &str,
        event: &str,
        message: &str,
        extra: Map<String, Value>,
    ) -> Value {
        let mut m = Map::new();
        m.insert("schema".into(), SCHEMA.into());
        m.insert(
            "ts".into(),
            chrono::Local::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
                .into(),
        );
        m.insert("level".into(), level.into());
        m.insert("plugin".into(), self.plugin.clone().into());
        m.insert("plugin_version".into(), self.plugin_version.clone().into());
        m.insert("run_id".into(), self.run_id.clone().into());
        m.insert("machine".into(), self.machine.clone().into());
        m.insert("stage".into(), self.stage.clone().into());
        m.insert("event".into(), event.into());
        m.insert("message".into(), message.into());
        m.extend(extra);
        Value::Object(m)
    }

    pub fn write(&self, v: &Value) -> std::io::Result<()> {
        if let Some(p) = self.path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{v}")
    }

    pub fn info(&self, event: &str, message: &str) -> std::io::Result<()> {
        self.write(&self.line("info", event, message, Map::new()))
    }
    pub fn warn(&self, event: &str, message: &str) -> std::io::Result<()> {
        self.write(&self.line("warn", event, message, Map::new()))
    }
    pub fn error(&self, event: &str, message: &str) -> std::io::Result<()> {
        self.write(&self.line("error", event, message, Map::new()))
    }
    /// A check result line (`check_id`, `status`, optional duration).
    pub fn check(
        &self,
        check_id: &str,
        status: &str,
        message: &str,
        duration_ms: Option<i64>,
    ) -> std::io::Result<()> {
        let mut x = Map::new();
        x.insert("check_id".into(), check_id.into());
        x.insert("status".into(), status.into());
        if let Some(d) = duration_ms {
            x.insert("duration_ms".into(), d.into());
        }
        let level = if status == "fail" { "warn" } else { "info" };
        self.write(&self.line(level, "check", message, x))
    }
    /// A line carrying metrics.
    pub fn metrics(&self, message: &str, metrics: &[Metric]) -> std::io::Result<()> {
        let mut x = Map::new();
        x.insert(
            "metrics".into(),
            Value::Array(metrics.iter().map(Metric::to_json).collect()),
        );
        self.write(&self.line("info", "metrics", message, x))
    }
    pub fn metric(&self, m: Metric) -> std::io::Result<()> {
        let msg = format!("{} = {}", m.name, m.value);
        self.metrics(&msg, &[m])
    }
}

/// `<root>/plugin-logs/<plugin>/<yyyy-mm-dd>.jsonl` for a run starting now (local date).
pub fn run_log_path(plugin_logs: &Path, plugin: &str) -> PathBuf {
    plugin_logs
        .join(plugin)
        .join(format!("{}.jsonl", chrono::Local::now().format("%Y-%m-%d")))
}

/// Validate one line against itcmdr.log/v1. Returns the parsed object.
pub fn validate(line: &str) -> Result<Map<String, Value>, String> {
    let v: Value = serde_json::from_str(line.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("not JSON: {e}"))?;
    let Value::Object(m) = v else {
        return Err("not a JSON object".into());
    };
    if m.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("schema must be \"{SCHEMA}\""));
    }
    for k in REQUIRED {
        match m.get(k) {
            Some(Value::String(_)) => {}
            Some(_) => return Err(format!("{k} must be a string")),
            None => return Err(format!("missing {k}")),
        }
    }
    let s = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or_default();
    chrono::DateTime::parse_from_rfc3339(s("ts"))
        .map_err(|_| "ts must be RFC 3339 with an offset".to_string())?;
    if !LEVELS.contains(&s("level")) {
        return Err(format!("level must be one of {LEVELS:?}"));
    }
    if let Some(st) = m.get("status") {
        if !st.as_str().is_some_and(|x| STATUSES.contains(&x)) {
            return Err(format!("status must be one of {STATUSES:?}"));
        }
    }
    if let Some(d) = m.get("duration_ms") {
        if !d.is_i64() && !d.is_u64() {
            return Err("duration_ms must be an integer".into());
        }
    }
    if let Some(c) = m.get("check_id") {
        if !c.is_string() {
            return Err("check_id must be a string".into());
        }
    }
    if let Some(ms) = m.get("metrics") {
        let Value::Array(list) = ms else {
            return Err("metrics must be a list".into());
        };
        for (i, x) in list.iter().enumerate() {
            let name_ok = x
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| !n.is_empty());
            let val_ok = x.get("value").is_some_and(Value::is_number);
            let kind_ok = x
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|k| KINDS.contains(&k));
            let labels_ok = match x.get("labels") {
                None => true,
                Some(Value::Object(l)) => l.values().all(Value::is_string),
                _ => false,
            };
            if !(name_ok && val_ok && kind_ok && labels_ok) {
                return Err(format!(
                    "metrics[{i}] needs name, numeric value, kind {KINDS:?} and string labels"
                ));
            }
        }
    }
    Ok(m)
}

/// Metrics of a validated line.
pub fn metrics_of(m: &Map<String, Value>) -> Vec<Metric> {
    let Some(Value::Array(list)) = m.get("metrics") else {
        return vec![];
    };
    list.iter()
        .map(|x| Metric {
            name: x["name"].as_str().unwrap_or_default().into(),
            value: x["value"].as_f64().unwrap_or_default(),
            unit: x
                .get("unit")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            kind: x["kind"].as_str().unwrap_or_default().into(),
            labels: x
                .get("labels")
                .and_then(Value::as_object)
                .map(|l| {
                    l.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().into()))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logger(p: &Path) -> Logger {
        Logger {
            path: p.into(),
            plugin: "xray-watch".into(),
            plugin_version: "1.0.0".into(),
            run_id: "r1".into(),
            machine: "SERVER01".into(),
            stage: "execute".into(),
        }
    }

    #[test]
    fn helpers_write_valid_lines() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x/2026-10-30.jsonl");
        let l = logger(&p);
        l.info("start", "hello").unwrap();
        l.check("NET-DNS", "ok", "fine", Some(12)).unwrap();
        l.metric(Metric::gauge("disk_free", 12.5, "GiB").label("drive", "C:"))
            .unwrap();
        l.metrics(
            "two",
            &[Metric::counter("runs", 1.0), Metric::gauge("t", 2.0, "s")],
        )
        .unwrap();
        l.error("boom", "bad").unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 5);
        for line in &lines {
            validate(line).unwrap_or_else(|e| panic!("{e}: {line}"));
        }
        let m = metrics_of(&validate(lines[2]).unwrap());
        assert_eq!(
            m,
            vec![Metric::gauge("disk_free", 12.5, "GiB").label("drive", "C:")]
        );
        assert_eq!(metrics_of(&validate(lines[3]).unwrap()).len(), 2);
    }

    #[test]
    fn bad_lines_are_rejected() {
        let d = tempfile::tempdir().unwrap();
        let good = logger(&d.path().join("a")).line("info", "e", "m", Map::new());
        let mut cases: Vec<(String, &str)> = vec![
            ("not json".into(), "not JSON"),
            ("[1]".into(), "object"),
            (r#"{"type":"run-start"}"#.into(), "schema"),
        ];
        for k in ["ts", "run_id", "message"] {
            let mut v = good.clone();
            v.as_object_mut().unwrap().remove(k);
            cases.push((v.to_string(), "missing"));
        }
        let with = |k: &str, x: Value| {
            let mut v = good.clone();
            v[k] = x;
            v.to_string()
        };
        cases.push((with("ts", "yesterday".into()), "RFC 3339"));
        cases.push((with("ts", "2026-10-30T10:00:00".into()), "RFC 3339"));
        cases.push((with("level", "loud".into()), "level"));
        cases.push((with("status", "great".into()), "status"));
        cases.push((with("duration_ms", 1.5.into()), "duration_ms"));
        cases.push((with("metrics", json!({"a": 1})), "list"));
        cases.push((
            with(
                "metrics",
                json!([{"name": "x", "value": "1", "kind": "gauge"}]),
            ),
            "metrics[0]",
        ));
        cases.push((
            with("metrics", json!([{"name": "x", "value": 1, "kind": "sum"}])),
            "metrics[0]",
        ));
        cases.push((
            with(
                "metrics",
                json!([{"name": "x", "value": 1, "kind": "gauge", "labels": {"a": 1}}]),
            ),
            "metrics[0]",
        ));
        cases.push((with("plugin", 5.into()), "string"));
        for (line, want) in cases {
            let e = validate(&line).expect_err(&line);
            assert!(e.contains(want), "{line}: {e}");
        }
        validate(&good.to_string()).unwrap();
    }
}
