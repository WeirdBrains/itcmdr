use crate::db::DbPool;
use crate::generated::itcmdr::{MetricPoint, Rollup};
use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use sqlx::Row;
use std::collections::BTreeMap;

/// Metric keys returned (one point per key/bucket):
/// - `incidents:<machine>` / `uptime:<machine>` / `repair_failures_bucket:<machine>` per rollup bucket
/// - `incidents_total:<machine>`: incidents in the window (sorted high to low = ranking)
/// - `open_incidents:<machine>`, `mttr:<machine>` (ms), `repair_failures:<machine>:<check>`,
///   `missed_runs:<machine>`
/// - `flaps:<machine>:<check>`: ok<->failed transitions in the window (only when >= setting metrics.flap_min)
/// - `repeat_offender:<machine>:<check>`: incidents opened in the window (only when >= setting metrics.repeat_min)
pub async fn get_metrics(
    pool: &DbPool,
    window_days: i64,
    rollup: Rollup,
) -> Result<Vec<MetricPoint>> {
    let now = Utc::now();
    let window_days = window_days.max(1);
    let start = now - Duration::days(window_days);
    let start_day = start.date_naive().to_string();
    let now_ms = now.timestamp_millis();
    let mut points = Vec::new();

    let rows = sqlx::query(
        "SELECT day_utc, computer, SUM(incidents) AS inc, SUM(runs) AS runs, SUM(failed_repairs) AS fr \
         FROM rollup_daily WHERE day_utc >= ? GROUP BY day_utc, computer ORDER BY day_utc, computer",
    )
    .bind(&start_day)
    .fetch_all(pool)
    .await?;
    let mut daily = Vec::new();
    for r in rows {
        let day: String = r.try_get("day_utc")?;
        if let Ok(d) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
            daily.push(DailyRow {
                day: d,
                computer: r.try_get("computer")?,
                incidents: r.try_get::<Option<i64>, _>("inc")?.unwrap_or(0),
                runs: r.try_get::<Option<i64>, _>("runs")?.unwrap_or(0),
                failed_repairs: r.try_get::<Option<i64>, _>("fr")?.unwrap_or(0),
            });
        }
    }
    points.extend(aggregate_daily(
        &daily,
        rollup,
        start.date_naive(),
        now.date_naive(),
    ));
    points.extend(
        rank_by_incidents(&daily)
            .into_iter()
            .map(|(c, v)| MetricPoint {
                key: format!("incidents_total:{c}"),
                ts: now_ms,
                value: v as f64,
            }),
    );

    let rows = sqlx::query(
        "SELECT computer, COUNT(*) as n FROM incidents WHERE is_open=1 GROUP BY computer",
    )
    .fetch_all(pool)
    .await?;
    for r in rows {
        let c: String = r.try_get("computer")?;
        let n: i64 = r.try_get::<Option<i64>, _>("n")?.unwrap_or(0);
        points.push(MetricPoint {
            key: format!("open_incidents:{c}"),
            ts: now_ms,
            value: n as f64,
        });
    }

    let rows = sqlx::query(
        "SELECT computer, AVG(closed_ts_ms - opened_ts_ms) AS mttr FROM incidents \
         WHERE closed_ts_ms IS NOT NULL AND opened_ts_ms >= ? GROUP BY computer",
    )
    .bind(start.timestamp_millis())
    .fetch_all(pool)
    .await?;
    for r in rows {
        let c: String = r.try_get("computer")?;
        let v: f64 = r.try_get::<Option<f64>, _>("mttr")?.unwrap_or(0.0);
        points.push(MetricPoint {
            key: format!("mttr:{c}"),
            ts: now_ms,
            value: v,
        });
    }

    // Where repairs failed: attempted repairs whose result was "failed".
    let rows = sqlx::query(
        "SELECT computer, check_id, COUNT(*) AS fails FROM checks \
         WHERE attempted_repair=1 AND lower(result)='failed' AND ts_ms >= ? \
         GROUP BY computer, check_id ORDER BY fails DESC",
    )
    .bind(start.timestamp_millis())
    .fetch_all(pool)
    .await?;
    for r in rows {
        let c: String = r.try_get("computer")?;
        let chk: String = r.try_get("check_id")?;
        let v: i64 = r.try_get::<Option<i64>, _>("fails")?.unwrap_or(0);
        points.push(MetricPoint {
            key: format!("repair_failures:{c}:{chk}"),
            ts: now_ms,
            value: v as f64,
        });
    }

    let rows = sqlx::query(
        "SELECT computer, COUNT(DISTINCT strftime('%Y-%m-%d', datetime(ts_ms/1000, 'unixepoch'))) AS days_present \
         FROM runs WHERE ts_ms >= ? GROUP BY computer",
    )
    .bind(start.timestamp_millis())
    .fetch_all(pool)
    .await?;
    for r in rows {
        let c: String = r.try_get("computer")?;
        let present: i64 = r.try_get::<Option<i64>, _>("days_present")?.unwrap_or(0);
        points.push(MetricPoint {
            key: format!("missed_runs:{c}"),
            ts: now_ms,
            value: (window_days - present).max(0) as f64,
        });
    }
    let flap_min = crate::settings::get_i64(pool, None, "metrics.flap_min")
        .await
        .max(1) as usize;
    let repeat_min = crate::settings::get_i64(pool, None, "metrics.repeat_min")
        .await
        .max(1);
    points.extend(flapping(pool, start.timestamp_millis(), now_ms, flap_min).await?);
    let rows = sqlx::query(
        "SELECT computer, check_id, COUNT(*) AS n FROM incidents WHERE opened_ts_ms >= ? \
         GROUP BY computer, check_id HAVING COUNT(*) >= ? ORDER BY n DESC",
    )
    .bind(start.timestamp_millis())
    .bind(repeat_min)
    .fetch_all(pool)
    .await?;
    for r in rows {
        let c: String = r.try_get("computer")?;
        let k: String = r.try_get("check_id")?;
        let n: i64 = r.try_get("n")?;
        points.push(MetricPoint {
            key: format!("repeat_offender:{c}:{k}"),
            ts: now_ms,
            value: n as f64,
        });
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
    pub failed_repairs: i64,
}

/// Machines ordered by incident count (highest first, then name).
pub fn rank_by_incidents(rows: &[DailyRow]) -> Vec<(String, i64)> {
    let mut m: BTreeMap<String, i64> = BTreeMap::new();
    for r in rows {
        *m.entry(r.computer.clone()).or_default() += r.incidents;
    }
    let mut v: Vec<(String, i64)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Defaults of the `metrics.flap_min` / `metrics.repeat_min` settings.
pub const FLAP_MIN: usize = 3;
pub const REPEAT_MIN: i64 = 3;

/// Number of ok<->failed transitions in a time-ordered status sequence (true = failed).
pub fn count_flaps(failed: &[bool]) -> usize {
    failed.windows(2).filter(|w| w[0] != w[1]).count()
}

async fn flapping(
    pool: &DbPool,
    start_ms: i64,
    now_ms: i64,
    flap_min: usize,
) -> Result<Vec<MetricPoint>> {
    let rows = sqlx::query(
        "SELECT computer, check_id, status FROM checks WHERE ts_ms >= ? AND status IN ('ok','failed') \
         ORDER BY computer, check_id, ts_ms",
    )
    .bind(start_ms)
    .fetch_all(pool)
    .await?;
    let mut seq: BTreeMap<(String, String), Vec<bool>> = BTreeMap::new();
    for r in rows {
        let status: String = r.try_get("status")?;
        seq.entry((r.try_get("computer")?, r.try_get("check_id")?))
            .or_default()
            .push(status == "failed");
    }
    Ok(seq
        .into_iter()
        .filter_map(|((c, k), v)| {
            let n = count_flaps(&v);
            (n >= flap_min).then(|| MetricPoint {
                key: format!("flaps:{c}:{k}"),
                ts: now_ms,
                value: n as f64,
            })
        })
        .collect())
}

fn monday(year: i32, week: u32) -> NaiveDate {
    NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).expect("valid ISO week")
}

/// 4-week block number (1..=13) for an ISO week; week 53 folds into block 13.
pub fn block_of_week(week: u32) -> u32 {
    (week.min(52) - 1) / 4 + 1
}

/// First day of the bucket containing `day`. All buckets are ISO-week aligned:
/// weekly = ISO week (Mon), block4 = Monday of ISO week 4N-3, yearly = Monday of ISO week 1.
pub fn bucket_start(day: NaiveDate, rollup: Rollup) -> NaiveDate {
    let iw = day.iso_week();
    match rollup {
        Rollup::Daily => day,
        Rollup::Weekly => day - Duration::days(day.weekday().num_days_from_monday() as i64),
        Rollup::Block4 => monday(iw.year(), 4 * block_of_week(iw.week()) - 3),
        Rollup::Yearly => monday(iw.year(), 1),
    }
}

/// First day of the bucket after the one starting at `start`.
pub fn next_bucket(start: NaiveDate, rollup: Rollup) -> NaiveDate {
    let iw = start.iso_week();
    match rollup {
        Rollup::Daily => start + Duration::days(1),
        Rollup::Weekly => start + Duration::days(7),
        Rollup::Block4 => {
            let b = block_of_week(iw.week());
            if b == 13 {
                monday(iw.year() + 1, 1)
            } else {
                monday(iw.year(), 4 * b + 1)
            }
        }
        Rollup::Yearly => monday(iw.year() + 1, 1),
    }
}

/// Group daily rows into buckets per machine. Emits `incidents:<m>` and
/// `repair_failures_bucket:<m>` (sums) and `uptime:<m>` (share of days in the bucket,
/// clipped to [window_start, today], that had at least one run).
pub fn aggregate_daily(
    rows: &[DailyRow],
    rollup: Rollup,
    window_start: NaiveDate,
    today: NaiveDate,
) -> Vec<MetricPoint> {
    let mut buckets: BTreeMap<(NaiveDate, String), (i64, i64, i64)> = BTreeMap::new();
    for r in rows {
        let e = buckets
            .entry((bucket_start(r.day, rollup), r.computer.clone()))
            .or_insert((0, 0, 0));
        e.0 += r.incidents;
        if r.runs > 0 {
            e.1 += 1;
        }
        e.2 += r.failed_repairs;
    }
    let mut out = Vec::with_capacity(buckets.len() * 3);
    for ((b, comp), (inc, days_with_runs, fr)) in buckets {
        let first = b.max(window_start);
        let last = (next_bucket(b, rollup) - Duration::days(1)).min(today);
        let days = ((last - first).num_days() + 1).max(1);
        let ts = b
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or_default();
        out.push(MetricPoint {
            key: format!("incidents:{comp}"),
            ts,
            value: inc as f64,
        });
        out.push(MetricPoint {
            key: format!("uptime:{comp}"),
            ts,
            value: (days_with_runs as f64 / days as f64).min(1.0),
        });
        out.push(MetricPoint {
            key: format!("repair_failures_bucket:{comp}"),
            ts,
            value: fr as f64,
        });
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
    fn iso_aligned_buckets() {
        // 2026-10-29 is a Thursday in ISO week 44 -> block 11 (weeks 41..44)
        assert_eq!(
            bucket_start(d(2026, 10, 29), Rollup::Daily),
            d(2026, 10, 29)
        );
        assert_eq!(
            bucket_start(d(2026, 10, 29), Rollup::Weekly),
            d(2026, 10, 26)
        );
        assert_eq!(
            bucket_start(d(2026, 10, 29), Rollup::Block4),
            d(2026, 10, 5)
        );
        assert_eq!(next_bucket(d(2026, 10, 5), Rollup::Block4), d(2026, 11, 2));
        // ISO year 2026 starts Mon 2025-12-29 and has 53 weeks
        assert_eq!(
            bucket_start(d(2026, 10, 29), Rollup::Yearly),
            d(2025, 12, 29)
        );
        assert_eq!(next_bucket(d(2025, 12, 29), Rollup::Yearly), d(2027, 1, 4));
    }

    #[test]
    fn week_53_folds_into_block_13() {
        assert_eq!(block_of_week(1), 1);
        assert_eq!(block_of_week(4), 1);
        assert_eq!(block_of_week(5), 2);
        assert_eq!(block_of_week(52), 13);
        assert_eq!(block_of_week(53), 13);
        // 2027-01-01 is in ISO 2026-W53 -> block 13 of 2026 (starts W49 = 2026-11-30)
        assert_eq!(bucket_start(d(2027, 1, 1), Rollup::Block4), d(2026, 11, 30));
        // block 13 of a 53-week year is 5 weeks long
        assert_eq!(next_bucket(d(2026, 11, 30), Rollup::Block4), d(2027, 1, 4));
        // ...and 4 weeks in a 52-week year (ISO 2027 has 52 weeks)
        assert_eq!(
            next_bucket(
                bucket_start(d(2027, 12, 20), Rollup::Block4),
                Rollup::Block4
            ),
            d(2028, 1, 3)
        );
    }

    #[test]
    fn aggregates_by_rollup() {
        let row = |day, inc, runs| DailyRow {
            day,
            computer: "WS-01".into(),
            incidents: inc,
            runs,
            failed_repairs: 0,
        };
        let rows = vec![
            row(d(2026, 9, 30), 1, 1),
            row(d(2026, 10, 26), 2, 1),
            row(d(2026, 10, 27), 3, 0),
            row(d(2026, 10, 28), 0, 1),
        ];
        let start = d(2026, 1, 1);
        let today = d(2026, 10, 28);
        let get = |pts: &[MetricPoint], key: &str| {
            pts.iter()
                .filter(|p| p.key == key)
                .map(|p| p.value)
                .collect::<Vec<_>>()
        };

        let weekly = aggregate_daily(&rows, Rollup::Weekly, start, today);
        assert_eq!(get(&weekly, "incidents:WS-01"), vec![1.0, 5.0]);
        assert!((get(&weekly, "uptime:WS-01")[1] - 2.0 / 3.0).abs() < 1e-9);

        // 09-30 is W40 (block 10), 10-26..28 is W44 (block 11)
        let blocks = aggregate_daily(&rows, Rollup::Block4, start, today);
        assert_eq!(get(&blocks, "incidents:WS-01"), vec![1.0, 5.0]);

        let yearly = aggregate_daily(&rows, Rollup::Yearly, start, today);
        assert_eq!(get(&yearly, "incidents:WS-01"), vec![6.0]);
        assert_eq!(
            yearly[0].ts,
            d(2025, 12, 29)
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis()
        );
    }

    #[test]
    fn counts_flaps() {
        assert_eq!(count_flaps(&[]), 0);
        assert_eq!(count_flaps(&[true, true, true]), 0);
        assert_eq!(count_flaps(&[false, true, false, true]), 3);
        assert_eq!(count_flaps(&[false, false, true, true]), 1);
    }

    #[test]
    fn ranks_machines() {
        let row = |c: &str, inc| DailyRow {
            day: d(2026, 10, 1),
            computer: c.into(),
            incidents: inc,
            runs: 1,
            failed_repairs: 0,
        };
        let r = rank_by_incidents(&[
            row("WS-02", 1),
            row("WS-01", 4),
            row("WS-03", 1),
            row("WS-01", 1),
        ]);
        assert_eq!(
            r,
            vec![
                ("WS-01".into(), 5),
                ("WS-02".into(), 1),
                ("WS-03".into(), 1)
            ]
        );
    }
}
