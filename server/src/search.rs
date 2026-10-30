//! Log search over agent JSON lines: filters, newest-first ordering, paging.
use crate::generated::itcmdr::SearchLogsRequest;
use serde_json::Value;

pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 1000;

fn field<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// Record timestamp: `ts`, else `started`, else `finished`.
pub fn line_ts(v: &Value) -> Option<i64> {
    ["ts", "started", "finished"]
        .iter()
        .find_map(|k| crate::ingest::parse_ts(field(v, k)))
}

pub struct Page {
    pub lines: Vec<String>,
    pub total: usize,
    pub next_offset: usize,
}

pub fn search(lines: Vec<String>, p: &SearchLogsRequest) -> Page {
    search_with(lines, p, DEFAULT_LIMIT, MAX_LIMIT)
}

/// `default_limit` / `max_limit` come from the `search.*` settings.
pub fn search_with(
    lines: Vec<String>,
    p: &SearchLogsRequest,
    default_limit: usize,
    max_limit: usize,
) -> Page {
    let q = p.query.to_lowercase();
    let eq = |a: Option<&str>, want: &str| {
        want.is_empty() || a.is_some_and(|a| a.eq_ignore_ascii_case(want))
    };
    let mut hits: Vec<(Option<i64>, String)> = lines
        .into_iter()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(&l).ok()?;
            if !eq(field(&v, "computer"), &p.machine)
                || !eq(field(&v, "category"), &p.category)
                || !eq(field(&v, "status"), &p.status)
            {
                return None;
            }
            if !q.is_empty() && !l.to_lowercase().contains(&q) {
                return None;
            }
            let ts = line_ts(&v);
            if p.start_ts > 0 && ts.is_none_or(|t| t < p.start_ts) {
                return None;
            }
            if p.end_ts > 0 && ts.is_none_or(|t| t > p.end_ts) {
                return None;
            }
            Some((ts, l))
        })
        .collect();
    // Newest first; undated lines last; stable for equal timestamps.
    hits.sort_by(|a, b| b.0.unwrap_or(i64::MIN).cmp(&a.0.unwrap_or(i64::MIN)));
    let total = hits.len();
    let limit = if p.limit <= 0 {
        default_limit
    } else {
        (p.limit as usize).min(max_limit)
    };
    let offset = p.offset.max(0) as usize;
    let lines: Vec<String> = hits
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(_, l)| l)
        .collect();
    let end = offset + lines.len();
    Page {
        lines,
        total,
        next_offset: if end < total { end } else { 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(c: &str, cat: &str, st: &str, ts: &str) -> String {
        format!(
            r#"{{"type":"check","computer":"{c}","category":"{cat}","checkId":"X","status":"{st}","ts":"{ts}"}}"#
        )
    }
    fn ms(s: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn filters_sorts_and_pages() {
        let lines = vec![
            line("WS-01", "network", "ok", "2026-10-01T01:00:00-04:00"),
            line("WS-01", "network", "failed", "2026-10-03T01:00:00-04:00"),
            line("WS-02", "services", "failed", "2026-10-02T01:00:00-04:00"),
            line(
                "ws-01",
                "configuration",
                "skipped",
                "2026-10-04T01:00:00-04:00",
            ),
            "not json".into(),
            r#"{"type":"run-start","computer":"WS-01","started":"2026-10-05T01:00:00-04:00"}"#
                .into(),
        ];
        let req = |f: fn(&mut SearchLogsRequest)| {
            let mut r = SearchLogsRequest::default();
            f(&mut r);
            r
        };
        let all = search(lines.clone(), &req(|_| {}));
        assert_eq!(all.total, 5);
        assert!(all.lines[0].contains("run-start"), "newest first");

        assert_eq!(
            search(lines.clone(), &req(|r| r.machine = "WS-01".into())).total,
            4
        );
        assert_eq!(
            search(lines.clone(), &req(|r| r.status = "failed".into())).total,
            2
        );
        assert_eq!(
            search(
                lines.clone(),
                &req(|r| {
                    r.category = "network".into();
                    r.status = "failed".into()
                })
            )
            .total,
            1
        );
        let win = search(
            lines.clone(),
            &req(|r| {
                r.start_ts = ms("2026-10-02T00:00:00-04:00");
                r.end_ts = ms("2026-10-03T23:59:59-04:00");
            }),
        );
        assert_eq!(win.total, 2);

        let p1 = search(lines.clone(), &req(|r| r.limit = 2));
        assert_eq!((p1.lines.len(), p1.total, p1.next_offset), (2, 5, 2));
        let p3 = search(
            lines.clone(),
            &req(|r| {
                r.limit = 2;
                r.offset = 4
            }),
        );
        assert_eq!((p3.lines.len(), p3.next_offset), (1, 0));
        assert!(
            p3.lines[0].contains("2026-10-01"),
            "oldest on the last page"
        );
    }
}
