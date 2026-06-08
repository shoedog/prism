use crate::navigation::types::Evidence;

pub fn render(ev: &Evidence, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(ev).unwrap_or_else(|_| "{}".into()),
        _ => {
            // text
            let mut s = format!("{}\n", ev.query);
            for it in &ev.items {
                s.push_str(&format!(
                    "  {}:{}-{}  score={:.2}  {:?}\n",
                    it.location.file,
                    it.location.start_line,
                    it.location.end_line,
                    it.score,
                    it.source
                ));
            }
            for w in &ev.warnings {
                s.push_str(&format!("  ! {:?}: {}\n", w.kind, w.message));
            }
            s
        }
    }
}
