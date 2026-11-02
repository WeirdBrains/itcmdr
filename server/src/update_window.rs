//! v0.21.0: the site's update window (`updates.window`, `updates.timezone`). Upgrades of the
//! server and the agents (offers and ring advances) only start inside it unless pushed.
//!
//! Format: `[days] HH:MM-HH:MM`, days `daily` (default) or a list like `Mon-Fri` / `Sat,Sun`.
//! A range that ends at or before it starts spans midnight and belongs to the day it starts on:
//! `Fri 23:00-02:00` is Friday 23:00 to Saturday 02:00. Times are wall-clock times in the site
//! zone, so a window keeps its local hours across DST changes (a 01:00-04:00 window is three
//! hours long most nights, two on the spring-forward night and four on the fall-back night).
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Datelike, NaiveTime, Utc, Weekday};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub days: Vec<Weekday>,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

pub fn parse(s: &str) -> Result<Window> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let (days_s, times) = match parts.as_slice() {
        [t] => ("daily", *t),
        [d, t] => (*d, *t),
        _ => bail!(
            "the update window must look like \"daily 01:00-04:00\" or \"Mon-Fri 22:00-02:00\""
        ),
    };
    let (a, b) = times
        .split_once('-')
        .ok_or_else(|| anyhow!("bad hours {times}"))?;
    let t = |x: &str| NaiveTime::parse_from_str(x, "%H:%M").map_err(|_| anyhow!("bad time {x}"));
    let (start, end) = (t(a)?, t(b)?);
    if start == end {
        bail!("the update window must not be empty (start equals end)");
    }
    let mut days = Vec::new();
    if days_s.eq_ignore_ascii_case("daily") {
        days = vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ];
    } else {
        for item in days_s.split(',') {
            let wd = |x: &str| Weekday::from_str(x).map_err(|_| anyhow!("bad day {x}"));
            match item.split_once('-') {
                Some((x, y)) => {
                    let (mut d, last) = (wd(x)?, wd(y)?);
                    loop {
                        if !days.contains(&d) {
                            days.push(d);
                        }
                        if d == last {
                            break;
                        }
                        d = d.succ();
                    }
                }
                None => {
                    let d = wd(item)?;
                    if !days.contains(&d) {
                        days.push(d);
                    }
                }
            }
        }
    }
    Ok(Window { days, start, end })
}

/// Is `(weekday, wall time)` inside the window?
pub fn contains_local(w: &Window, wd: Weekday, t: NaiveTime) -> bool {
    if w.start < w.end {
        w.days.contains(&wd) && t >= w.start && t < w.end
    } else {
        // Spans midnight: the evening part on a listed day, the morning part the day after one.
        (w.days.contains(&wd) && t >= w.start) || (w.days.contains(&wd.pred()) && t < w.end)
    }
}

/// The site zone: `updates.timezone` (IANA name) or, when empty or "local", the host's zone.
pub fn local_parts(tz: &str, at: DateTime<Utc>) -> Result<(Weekday, NaiveTime)> {
    let tz = tz.trim();
    let named = if tz.is_empty() || tz.eq_ignore_ascii_case("local") {
        iana_time_zone::get_timezone().ok()
    } else {
        Some(tz.to_string())
    };
    match named.map(|n| n.parse::<chrono_tz::Tz>()) {
        Some(Ok(z)) => {
            let l = at.with_timezone(&z);
            Ok((l.weekday(), l.time()))
        }
        Some(Err(_)) if !(tz.is_empty() || tz.eq_ignore_ascii_case("local")) => {
            bail!("unknown timezone {tz:?}")
        }
        _ => {
            let l = at.with_timezone(&chrono::Local);
            Ok((l.weekday(), l.time()))
        }
    }
}

pub fn open_at(window: &str, tz: &str, at: DateTime<Utc>) -> Result<bool> {
    let w = parse(window)?;
    let (wd, t) = local_parts(tz, at)?;
    Ok(contains_local(&w, wd, t))
}

/// The site's settings: auto-update on and the window open now. A bad window fails closed.
pub async fn site_open(pool: &crate::db::DbPool, at: DateTime<Utc>) -> bool {
    let w = crate::settings::get_str(pool, None, "updates.window").await;
    let tz = crate::settings::get_str(pool, None, "updates.timezone").await;
    open_at(&w, &tz, at).unwrap_or(false)
}

pub async fn auto_on(pool: &crate::db::DbPool) -> bool {
    crate::settings::get_bool(pool, None, "updates.auto").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn parse_forms() {
        assert_eq!(parse("01:00-04:00").unwrap().days.len(), 7);
        assert_eq!(parse("Mon-Fri 22:00-02:00").unwrap().days.len(), 5);
        assert_eq!(
            parse("Sat,Sun 01:00-04:00").unwrap().days,
            vec![Weekday::Sat, Weekday::Sun]
        );
        assert_eq!(parse("Fri-Mon 01:00-02:00").unwrap().days.len(), 4);
        assert!(parse("01:00-01:00").is_err());
        assert!(parse("Funday 01:00-02:00").is_err());
        assert!(parse("1-4").is_err());
    }

    #[test]
    fn same_day_window() {
        let w = parse("daily 01:00-04:00").unwrap();
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert!(contains_local(&w, Weekday::Tue, t(1, 0)));
        assert!(contains_local(&w, Weekday::Tue, t(3, 59)));
        assert!(!contains_local(&w, Weekday::Tue, t(4, 0)));
        assert!(!contains_local(&w, Weekday::Tue, t(0, 59)));
    }

    #[test]
    fn midnight_spanning_window_belongs_to_its_start_day() {
        let w = parse("Fri 23:00-02:00").unwrap();
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        assert!(contains_local(&w, Weekday::Fri, t(23, 30)));
        assert!(contains_local(&w, Weekday::Sat, t(1, 59)));
        assert!(!contains_local(&w, Weekday::Sat, t(2, 0)));
        assert!(
            !contains_local(&w, Weekday::Sat, t(23, 30)),
            "Saturday evening is not listed"
        );
        assert!(
            !contains_local(&w, Weekday::Fri, t(1, 0)),
            "Friday morning belongs to Thursday"
        );
        let w = parse("Sun 22:00-03:00").unwrap();
        assert!(
            contains_local(&w, Weekday::Mon, t(2, 0)),
            "Sunday's window runs into Monday"
        );
    }

    #[test]
    fn dst_keeps_local_hours() {
        let ny = "America/New_York";
        let w = "daily 01:00-04:00";
        // Spring forward 2026-03-08: 02:00 EST jumps to 03:00 EDT.
        assert!(open_at(w, ny, at("2026-03-08T06:30:00Z")).unwrap()); // 01:30 EST
        assert!(open_at(w, ny, at("2026-03-08T07:30:00Z")).unwrap()); // 03:30 EDT
        assert!(!open_at(w, ny, at("2026-03-08T08:30:00Z")).unwrap()); // 04:30 EDT
                                                                       // Fall back 2026-11-01: 01:00-02:00 happens twice; both are inside.
        assert!(open_at(w, ny, at("2026-11-01T05:30:00Z")).unwrap()); // 01:30 EDT
        assert!(open_at(w, ny, at("2026-11-01T06:30:00Z")).unwrap()); // 01:30 EST
        assert!(open_at(w, ny, at("2026-11-01T08:30:00Z")).unwrap()); // 03:30 EST
        assert!(!open_at(w, ny, at("2026-11-01T09:30:00Z")).unwrap()); // 04:30 EST
                                                                       // The same UTC instant is 04:30 in summer but 03:30 in winter at 08:30Z.
        assert!(!open_at(w, ny, at("2026-07-01T08:30:00Z")).unwrap());
        assert!(open_at(w, ny, at("2026-12-01T08:30:00Z")).unwrap());
        // A midnight-spanning window across the fall-back night.
        let w = "Sat 23:00-02:00";
        assert!(open_at(w, ny, at("2026-11-01T06:59:00Z")).unwrap()); // Sun 01:59 EST
        assert!(!open_at(w, ny, at("2026-11-01T07:00:00Z")).unwrap()); // Sun 02:00 EST
        assert!(open_at(
            "daily 01:00-04:00",
            "Europe/Berlin",
            Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap()
        )
        .unwrap()); // 03:30 CEST
        assert!(open_at("01:00-04:00", "No/Such_Zone", at("2026-01-01T00:00:00Z")).is_err());
    }
}
