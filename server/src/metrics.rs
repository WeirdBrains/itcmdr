use crate::db::DbPool;
use crate::generated::itcmdr::{MetricPoint, Rollup};
use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use std::collections::BTreeMap;
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

    let mut daily = Vec::new();
    for r in rows {
        let day: String = r.try_get("day_utc")?;
        let comp: String = r.try_get("computer")?;
        let inc: i64 = r.try_get::<Option<i64>, _>("inc")?.unwrap_or(0);
        let runs: i64 = r.try_get::<Option<i64>, _>("runs")?.unwrap_or(0);
        if let Ok(d) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
            daily.push(DailyRow { day: d, computer: comp, incidents: inc, runs });
        }
    }
    points.extend(aggregate_daily(&daily, rollup, start.date_naive(), now.date_naive()));

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
        let v: f64 = r.try_get::<Option<f64>, _>("mttr")?.unwrap_or(0.0);
        points.push(MetricPoint { key: format!("mttr:{}", comp), ts: now.timestamp_millis(), value: v });
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



/// One row of the per-day, per-machine rollup table.
#[derive(Debug, Clone)]
pub struct DailyRow {
    pub day: NaiveDate,
    pub computer: String,
    pub incidents: i64,
    pub runs: i64,
}

/// First day of the bucket that `day` falls into for the given rollup level.
/// Weekly buckets start on Monday (ISO week).
pub fn bucket_start(day: NaiveDate, rollup: Rollup) -> NaiveDate {
    match rollup {
        Rollup::Daily => day,
        Rollup::Weekly => day - Duration::days(day.weekday().num_days_from_monday() as i64),
        Rollup::Monthly => NaiveDate::from_ymd_opt(day.year(), day.month(), 1).unwrap_or(day),
        Rollup::Quarterly => {
            let m = ((day.month() - 1) / 3) * 3 + 1;
            NaiveDate::from_ymd_opt(day.year(), m, 1).unwrap_or(day)
        }
        Rollup::Yearly => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
    }
}

/// First day of the bucket after the one starting at `start`.
fn next_bucket(start: NaiveDate, rollup: Rollup) -> NaiveDate {
    let add_months = |months: u32| {
        let total = start.year() * 12 + start.month0() as i32 + months as i32;
        NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1).unwrap_or(start)
    };
    match rollup {
        Rollup::Daily => start + Duration::days(1),
        Rollup::Weekly => start + Duration::days(7),
        Rollup::Monthly => add_months(1),
        Rollup::Quarterly => add_months(3),
        Rollup::Yearly => add_months(12),
    }
}

/// Group daily rows into buckets per machine. Emits `incidents:<machine>` (sum over the
/// bucket) and `uptime:<machine>` (fraction of days in the bucket, clipped to the
/// window [window_start, today], that had at least one run).
pub fn aggregate_daily(rows: &[DailyRow], rollup: Rollup, window_start: NaiveDate, today: NaiveDate) -> Vec<MetricPoint> {
    let mut buckets: BTreeMap<(NaiveDate, String), (i64, i64)> = BTreeMap::new();
    for r in rows {
        let e = buckets.entry((bucket_start(r.day, rollup), r.computer.clone())).or_insert((0, 0));
        e.0 += r.incidents;
        if r.runs > 0 {
            e.1 += 1;
        }
    }
    let mut out = Vec::with_capacity(buckets.len() * 2);
    for ((b, comp), (inc, days_with_runs)) in buckets {
        let first = b.max(window_start);
        let last = (next_bucket(b, rollup) - Duration::days(1)).min(today);
        let days = ((last - first).num_days() + 1).max(1);
        let ts = b.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp_millis()).unwrap_or_default();
        out.push(MetricPoint { key: format!("incidents:{}", comp), ts, value: inc as f64 });
        out.push(MetricPoint { key: format!("uptime:{}", comp), ts, value: (days_with_runs as f64 / days as f64).min(1.0) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn bucket_starts() {
        // 2026-10-29 is a Thursday
        assert_eq!(bucket_start(d(2026, 10, 29), Rollup::Daily), d(2026, 10, 29));
        assert_eq!(bucket_start(d(2026, 10, 29), Rollup::Weekly), d(2026, 10, 26));
        assert_eq!(bucket_start(d(2026, 10, 29), Rollup::Monthly), d(2026, 10, 1));
        assert_eq!(bucket_start(d(2026, 10, 29), Rollup::Quarterly), d(2026, 10, 1));
        assert_eq!(bucket_start(d(2026, 10, 29), Rollup::Yearly), d(2026, 1, 1));
        assert_eq!(next_bucket(d(2026, 12, 1), Rollup::Monthly), d(2027, 1, 1));
    }

    #[test]
    fn aggregates_by_rollup() {
        let row = |day, inc, runs| DailyRow { day, computer: "WS-01".into(), incidents: inc, runs };
        let rows = vec![
            row(d(2026, 9, 30), 1, 1),
            row(d(2026, 10, 26), 2, 1),
            row(d(2026, 10, 27), 3, 0),
            row(d(2026, 10, 28), 0, 1),
        ];
        let start = d(2026, 1, 1);
        let today = d(2026, 10, 28);
        let get = |pts: &[MetricPoint], key: &str| pts.iter().filter(|p| p.key == key).map(|p| p.value).collect::<Vec<_>>();

        let weekly = aggregate_daily(&rows, Rollup::Weekly, start, today);
        assert_eq!(get(&weekly, "incidents:WS-01"), vec![1.0, 5.0]);
        // Week of 10-26: 3 elapsed days (26..28), 2 with runs
        assert!((get(&weekly, "uptime:WS-01")[1] - 2.0 / 3.0).abs() < 1e-9);

        let monthly = aggregate_daily(&rows, Rollup::Monthly, start, today);
        assert_eq!(get(&monthly, "incidents:WS-01"), vec![1.0, 5.0]);

        let yearly = aggregate_daily(&rows, Rollup::Yearly, start, today);
        assert_eq!(get(&yearly, "incidents:WS-01"), vec![6.0]);
        assert_eq!(yearly[0].ts, d(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    }
}
