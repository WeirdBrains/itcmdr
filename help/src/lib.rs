//! In-app help for the CLIs: the guides in docs/ (topic = file name in lower case, README = `index`).
include!(concat!(env!("OUT_DIR"), "/help_topics.rs"));

/// The markdown of a topic (case-insensitive; `.md` and a `docs/` prefix are accepted).
pub fn get(topic: &str) -> Option<&'static str> {
    let t = topic.trim().to_lowercase();
    let t = t.trim_start_matches("docs/").trim_end_matches(".md");
    let t = if t == "readme" { "index" } else { t };
    TOPICS.iter().find(|(id, _)| *id == t).map(|(_, md)| *md)
}

/// The first heading of a topic, for listings.
pub fn title(md: &str) -> &str {
    md.lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("")
        .trim()
}

/// Topics whose title or text contains `q` (case-insensitive), title matches first.
pub fn search(q: &str) -> Vec<&'static str> {
    let q = q.to_lowercase();
    let mut hits: Vec<(u8, &str)> = TOPICS
        .iter()
        .filter_map(|(id, md)| {
            if title(md).to_lowercase().contains(&q) || id.contains(&q) {
                Some((0, *id))
            } else if md.to_lowercase().contains(&q) {
                Some((1, *id))
            } else {
                None
            }
        })
        .collect();
    hits.sort();
    hits.into_iter().map(|(_, id)| id).collect()
}

/// `<prog> help [topic]`: list the topics, print one, or search. Returns the text to print and
/// whether the topic was found.
pub fn render(prog: &str, topic: Option<&str>) -> (String, bool) {
    match topic {
        None => {
            let mut s =
                format!("{prog} help <topic>  (the guides in docs/; `--help` lists commands)\n\n");
            for (id, md) in TOPICS {
                s.push_str(&format!("  {id:<20} {}\n", title(md)));
            }
            (s, true)
        }
        Some(t) => match get(t) {
            Some(md) => (md.to_string(), true),
            None => {
                let hits = search(t);
                if hits.is_empty() {
                    (
                        format!(
                            "no help topic or text matches {t:?}; `{prog} help` lists topics\n"
                        ),
                        false,
                    )
                } else {
                    (
                        format!(
                            "no topic {t:?}; topics mentioning it: {}\n",
                            hits.join(", ")
                        ),
                        false,
                    )
                }
            }
        },
    }
}
