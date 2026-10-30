//! Exception (allowlist) rules. One JSON shape, camelCase, shared by the database,
//! the export file agents read (`exceptions.json` in the maintenanceRuns share) and
//! the agent's own `site.json` `exceptions` array:
//! `{"id","scope":{"machines":[],"role"},"checkId","checkPattern","action","reason","expires"}`
use crate::db::DbPool;
use crate::generated::itcmdr::{ExceptionRule, ExceptionScope};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const ACTIONS: [&str; 3] = ["ignore", "downgrade:warn", "downgrade:info"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeDoc {
    #[serde(default)]
    pub machines: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDoc {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub scope: ScopeDoc,
    #[serde(default, alias = "check_id", skip_serializing_if = "String::is_empty")]
    pub check_id: String,
    #[serde(default, alias = "check_pattern", skip_serializing_if = "String::is_empty")]
    pub check_pattern: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub expires: Option<String>,
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

impl ExceptionDoc {
    pub fn from_proto(r: &ExceptionRule) -> Self {
        let scope = r.scope.clone().unwrap_or_default();
        Self {
            id: r.id.trim().to_string(),
            scope: ScopeDoc {
                machines: scope.machines.iter().filter_map(|m| non_empty(m)).collect(),
                role: non_empty(&scope.role),
            },
            check_id: r.check_id.trim().to_string(),
            check_pattern: r.check_pattern.trim().to_string(),
            action: r.action.trim().to_string(),
            reason: r.reason.trim().to_string(),
            expires: non_empty(&r.expires),
        }
    }

    pub fn to_proto(&self, active: bool) -> ExceptionRule {
        ExceptionRule {
            id: self.id.clone(),
            scope: Some(ExceptionScope { machines: self.scope.machines.clone(), role: self.scope.role.clone().unwrap_or_default() }),
            check_id: self.check_id.clone(),
            check_pattern: self.check_pattern.clone(),
            action: self.action.clone(),
            reason: self.reason.clone(),
            expires: self.expires.clone().unwrap_or_default(),
            active,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.check_id.is_empty() && self.check_pattern.is_empty() {
            return Err("a rule needs a check id or a check pattern".into());
        }
        if !ACTIONS.contains(&self.action.as_str()) {
            return Err(format!("action must be one of {}", ACTIONS.join(", ")));
        }
        if self.reason.is_empty() {
            return Err("a reason is required".into());
        }
        if let Some(e) = &self.expires {
            let ok = chrono::NaiveDate::parse_from_str(e, "%Y-%m-%d").is_ok() || chrono::DateTime::parse_from_rfc3339(e).is_ok();
            if !ok {
                return Err("expires must be YYYY-MM-DD or an RFC 3339 timestamp".into());
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub version: u32,
    pub generated: String,
    pub rules: Vec<ExceptionDoc>,
}

pub async fn load(pool: &DbPool, include_inactive: bool) -> anyhow::Result<Vec<(ExceptionDoc, bool)>> {
    Ok(crate::db::list_exceptions(pool, include_inactive)
        .await?
        .into_iter()
        .filter_map(|(json, active)| serde_json::from_str::<ExceptionDoc>(&json).ok().map(|d| (d, active)))
        .collect())
}

/// Write the active rules to `path` atomically (temp file + rename).
pub async fn export(pool: &DbPool, path: &Path) -> anyhow::Result<usize> {
    let rules: Vec<ExceptionDoc> = load(pool, false).await?.into_iter().map(|(d, _)| d).collect();
    let n = rules.len();
    let file = ExportFile { version: 1, generated: chrono::Utc::now().to_rfc3339(), rules };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&file)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_legacy_snake_case_and_writes_camel_case() {
        let legacy = r#"{"id":"a","scope":{"machines":["WS-01"],"role":null},"check_id":"NET-IP-CORRECT","check_pattern":"","action":"ignore","reason":"r","expires":"","active":true}"#;
        let d: ExceptionDoc = serde_json::from_str(legacy).unwrap();
        assert_eq!(d.check_id, "NET-IP-CORRECT");
        let out = serde_json::to_string(&d).unwrap();
        assert!(out.contains("\"checkId\":\"NET-IP-CORRECT\""));
        assert!(!out.contains("check_id"));
    }

    #[test]
    fn validates() {
        let mut d = ExceptionDoc { check_id: "X".into(), action: "ignore".into(), reason: "r".into(), ..Default::default() };
        assert!(d.validate().is_ok());
        d.action = "delete".into();
        assert!(d.validate().is_err());
        d.action = "downgrade:warn".into();
        d.expires = Some("next week".into());
        assert!(d.validate().is_err());
        d.expires = Some("2099-12-31".into());
        assert!(d.validate().is_ok());
        d.check_id.clear();
        assert!(d.validate().is_err());
    }
}
