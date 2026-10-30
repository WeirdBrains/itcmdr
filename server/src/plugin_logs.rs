//! Ingest of plugin logs (decision 11): every line under `<data>/plugin-logs/**.jsonl` is validated
//! against itcmdr.log/v1. Malformed lines are counted per file (never silently dropped); `metrics`
//! values are stored generically in `plugin_metrics` and served by `GetPluginMetrics`.
use crate::db::DbPool;
use anyhow::Result;
use sqlx::Row;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files: usize,
    pub lines: usize,
    pub rejects: usize,
    pub metrics: usize,
}

/// Read new lines of every plugin log (files are append-only; progress is kept per file).
pub async fn ingest(pool: &DbPool, root: &Path) -> Result<Stats> {
    let mut st = Stats::default();
    if !root.is_dir() {
        return Ok(st);
    }
    for e in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = e.path();
        if !e.file_type().is_file() || p.extension().is_none_or(|x| x != "jsonl") {
            continue;
        }
        let key = p.display().to_string();
        let done: i64 =
            sqlx::query_scalar("SELECT lines_done FROM plugin_log_files WHERE path = ?")
                .bind(&key)
                .fetch_optional(pool)
                .await?
                .unwrap_or(0);
        let text = String::from_utf8_lossy(&std::fs::read(p)?).to_string();
        // Only complete lines (a writer may be mid-line).
        let complete = match text.rfind('\n') {
            Some(i) => &text[..=i],
            None => "",
        };
        let lines: Vec<&str> = complete.lines().collect();
        if lines.len() as i64 <= done {
            continue;
        }
        st.files += 1;
        let mut rejects = 0i64;
        let mut tx = pool.begin().await?;
        for (i, line) in lines.iter().enumerate().skip(done as usize) {
            if line.trim().is_empty() {
                continue;
            }
            st.lines += 1;
            match itcmdr_plugin::log::validate(line) {
                Err(reason) => {
                    rejects += 1;
                    if rejects <= 20 {
                        tracing::warn!(file=%key, line = i + 1, %reason, "itcmdr.log/v1 reject");
                    }
                }
                Ok(m) => {
                    let ts =
                        chrono::DateTime::parse_from_rfc3339(m["ts"].as_str().unwrap_or_default())
                            .map(|t| t.timestamp_millis())
                            .unwrap_or_default();
                    let s = |k: &str| {
                        m.get(k)
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string()
                    };
                    for mt in itcmdr_plugin::log::metrics_of(&m) {
                        let mut labels = mt.labels.clone();
                        labels.sort();
                        let lj = serde_json::Value::Object(
                            labels
                                .into_iter()
                                .map(|(k, v)| (k, serde_json::Value::String(v)))
                                .collect(),
                        )
                        .to_string();
                        sqlx::query(
                            "INSERT INTO plugin_metrics(plugin, name, labels, ts_ms, value, unit, kind, run_id, machine) VALUES (?,?,?,?,?,?,?,?,?)",
                        )
                        .bind(s("plugin"))
                        .bind(&mt.name)
                        .bind(lj)
                        .bind(ts)
                        .bind(mt.value)
                        .bind(&mt.unit)
                        .bind(&mt.kind)
                        .bind(s("run_id"))
                        .bind(s("machine"))
                        .execute(&mut *tx)
                        .await?;
                        st.metrics += 1;
                    }
                }
            }
        }
        st.rejects += rejects as usize;
        sqlx::query(
            "INSERT INTO plugin_log_files(path, lines_done, rejects) VALUES (?,?,?) \
             ON CONFLICT(path) DO UPDATE SET lines_done = excluded.lines_done, rejects = plugin_log_files.rejects + excluded.rejects",
        )
        .bind(&key)
        .bind(lines.len() as i64)
        .bind(rejects)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    }
    Ok(st)
}

/// Files with rejected lines: (path, reject count).
pub async fn rejects(pool: &DbPool) -> Result<Vec<(String, i64)>> {
    Ok(
        sqlx::query_as(
            "SELECT path, rejects FROM plugin_log_files WHERE rejects > 0 ORDER BY path",
        )
        .fetch_all(pool)
        .await?,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub name: String,
    pub labels: String,
    pub ts_ms: i64,
    pub value: f64,
}

/// Metric points: optional name filter, label filter (all must match), window, daily rollup
/// (counters summed, gauges/histograms averaged per local day).
pub async fn query(
    pool: &DbPool,
    name: &str,
    labels: &[(String, String)],
    window_days: i64,
    daily: bool,
) -> Result<Vec<Point>> {
    let start = chrono::Utc::now().timestamp_millis() - window_days.clamp(1, 3650) * 86_400_000;
    let rows = sqlx::query(
        "SELECT name, labels, ts_ms, value, kind FROM plugin_metrics WHERE ts_ms >= ? AND (? = '' OR name = ?) ORDER BY ts_ms",
    )
    .bind(start)
    .bind(name)
    .bind(name)
    .fetch_all(pool)
    .await?;
    let mut pts = Vec::new();
    for r in rows {
        let l: String = r.try_get("labels")?;
        let lv: serde_json::Value = serde_json::from_str(&l).unwrap_or_default();
        if !labels
            .iter()
            .all(|(k, v)| lv.get(k).and_then(|x| x.as_str()) == Some(v.as_str()))
        {
            continue;
        }
        pts.push((
            r.try_get::<String, _>("name")?,
            l,
            r.try_get::<i64, _>("ts_ms")?,
            r.try_get::<f64, _>("value")?,
            r.try_get::<String, _>("kind")?,
        ));
    }
    if !daily {
        return Ok(pts
            .into_iter()
            .map(|(name, labels, ts_ms, value, _)| Point {
                name,
                labels,
                ts_ms,
                value,
            })
            .collect());
    }
    use std::collections::BTreeMap;
    let mut g: BTreeMap<(String, String, i64), (f64, usize, bool)> = BTreeMap::new();
    for (n, l, ts, v, k) in pts {
        let day = chrono::DateTime::from_timestamp_millis(ts)
            .map(|d| d.with_timezone(&chrono::Local).date_naive())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|d| d.and_local_timezone(chrono::Local).earliest())
            .map(|d| d.timestamp_millis())
            .unwrap_or(ts);
        let e = g.entry((n, l, day)).or_insert((0.0, 0, k == "counter"));
        e.0 += v;
        e.1 += 1;
    }
    Ok(g.into_iter()
        .map(|((name, labels, ts_ms), (sum, n, counter))| Point {
            name,
            labels,
            ts_ms,
            value: if counter { sum } else { sum / n as f64 },
        })
        .collect())
}
