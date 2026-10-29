use crate::db::DbPool;
use crate::generated::itcmdr::{MetricPoint, Rollup};
use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc, Duration};
use sqlx::Row;

pub async fn get_metrics(pool: &DbPool, window_days: i64, rollup: Rollup) -> Result<Vec<MetricPoint>> {
    let now = Utc::now();
    let start = now - Duration::days(window_days.max(1));
    let start_day = start.date_naive().to_string();
    let mut points = Vec::new();

    // Incidents per day per machine from rollup_daily
    let rows = sqlx::query(
        r#"
        SELECT day_utc, computer, SUM(incidents) AS inc, SUM(runs) AS runs
        FROM rollup_daily
        WHERE day_utc >= ?
        GROUP BY day_utc, computer
        ORDER BY day_utc ASC, computer ASC
        "#,
    )
    .bind(&start_day)
    .fetch_all(pool)
    .await?;

    for r in rows {
        let day: String = r.try_get("day_utc")?;
        let comp: String = r.try_get("computer")?;
        let inc: i64 = r.try_get::<Option<i64>, _>("inc")?.unwrap_or(0);
        let runs: i64 = r.try_get::<Option<i64>, _>("runs")?.unwrap_or(0);
        let ts = NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or_else(|| now.timestamp_millis());
        points.push(MetricPoint { key: format!("incidents:{}", comp), ts, value: inc as f64 });
        points.push(MetricPoint { key: format!("uptime:{}", comp), ts, value: if runs > 0 { 1.0 } else { 0.0 } });
    }

    // Open incidents count "now"
    let rows2 = sqlx::query("SELECT computer, COUNT(*) as open_count FROM incidents WHERE is_open=1 GROUP BY computer")
        .fetch_all(pool).await?;
    for r in rows2 {
        let comp: String = r.try_get("computer")?;
        let c: i64 = r.try_get::<Option<i64>, _>("open_count")?.unwrap_or(0);
        points.push(MetricPoint { key: format!("open_incidents:{}", comp), ts: now.timestamp_millis(), value: c as f64 });
    }

    // MTTR per machine (ms) over window
    let rows3 = sqlx::query(
        r#"
        SELECT computer, AVG(closed_ts_ms - opened_ts_ms) AS mttr
        FROM incidents
        WHERE closed_ts_ms IS NOT NULL AND opened_ts_ms >= ?
        GROUP BY computer
        "#,
    )
    .bind(start.timestamp_millis())
    .fetch_all(pool)
    .await?;
    for r in rows3 {
        let comp: String = r.try_get("computer")?;
        let v: i64 = r.try_get::<Option<i64>, _>("mttr")?.unwrap_or(0);
        points.push(MetricPoint { key: format!("mttr:{}", comp), ts: now.timestamp_millis(), value: v as f64 });
    }

    // Repair failures by check
    let rows4 = sqlx::query(
        r#"
        SELECT computer, check_id, COUNT(*) AS fails
        FROM checks
        WHERE attempted_repair=1 AND (lower(result)='failed' OR lower(status)='failed') AND ts_ms >= ?
        GROUP BY computer, check_id
        ORDER BY fails DESC
        "#,
    )
    .bind(start.timestamp_millis())
    .fetch_all(pool)
    .await?;
    for r in rows4 {
        let comp: String = r.try_get("computer")?;
        let chk: String = r.try_get("check_id")?;
        let v: i64 = r.try_get::<Option<i64>, _>("fails")?.unwrap_or(0);
        points.push(MetricPoint { key: format!("repair_failures:{}:{}", comp, chk), ts: now.timestamp_millis(), value: v as f64 });
    }

    // Missed runs per machine over window: window_days - distinct days with runs
    let rows5 = sqlx::query(
        r#"
        SELECT computer, COUNT(DISTINCT strftime('%Y-%m-%d', datetime(ts_ms/1000, 'unixepoch'))) AS days_present
        FROM runs
        WHERE ts_ms >= ?
        GROUP BY computer
        "#,
    ).bind(start.timestamp_millis()).fetch_all(pool).await?;
    for r in rows5 {
        let comp: String = r.try_get("computer")?;
        let days_present: i64 = r.try_get::<Option<i64>, _>("days_present")?.unwrap_or(0);
        let missed = (window_days - days_present).max(0);
        points.push(MetricPoint { key: format!("missed_runs:{}", comp), ts: now.timestamp_millis(), value: missed as f64 });
    }

    Ok(points)
}


