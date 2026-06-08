use crate::navigation::types::{EgoGraph, Evidence, QueryError};

pub fn render_err(e: &QueryError, format: &str) -> (String, i32) {
    let rendered = match format {
        "json" => serde_json::to_string_pretty(&serde_json::json!({ "error": e }))
            .unwrap_or_else(|_| "{}".into()),
        _ => format!("error: {}", error_text(e)),
    };
    (rendered, 3)
}

fn error_text(e: &QueryError) -> String {
    match e {
        QueryError::AmbiguousSymbol { candidates } => {
            format!("ambiguous symbol ({} candidates)", candidates.len())
        }
        QueryError::SymbolNotFound { seed } => format!("symbol not found: {seed}"),
        QueryError::LocationOutOfRange { file, line } => {
            format!("location out of range: {file}:{line}")
        }
        QueryError::UnsupportedFile { file } => format!("unsupported file: {file}"),
        QueryError::UnknownEdge { edge } => format!("unknown edge: {edge}"),
    }
}

pub fn render_ego(g: &EgoGraph, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(g).unwrap_or_else(|_| "{}".into()),
        _ => {
            let mut s = format!("{}\n", g.query);
            for (i, n) in g.nodes.iter().enumerate() {
                s.push_str(&format!(
                    "  [{i}] {}:{}\n",
                    n.location.file, n.location.start_line
                ));
            }
            for e in &g.edges {
                s.push_str(&format!("  {} --{}--> {}\n", e.from, e.kind, e.to));
            }
            s
        }
    }
}

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
                for reason in &it.why {
                    match reason {
                        crate::navigation::types::Reason::Calls {
                            callee,
                            call_site_line,
                            qualifier,
                        } => {
                            let callee = qualifier
                                .as_ref()
                                .map(|q| format!("{q}.{callee}"))
                                .unwrap_or_else(|| callee.clone());
                            s.push_str(&format!(
                                "    calls {} @ {}:{}\n",
                                callee, it.location.file, call_site_line
                            ));
                        }
                        crate::navigation::types::Reason::CalledBy {
                            caller,
                            call_site_line,
                        } => {
                            s.push_str(&format!(
                                "    called by {} @ line {}\n",
                                caller, call_site_line
                            ));
                        }
                        crate::navigation::types::Reason::EnclosingFunction { function } => {
                            s.push_str(&format!("    in {:?}\n", function));
                        }
                        other => {
                            s.push_str(&format!("    {:?}\n", other));
                        }
                    }
                }
            }
            for w in &ev.warnings {
                s.push_str(&format!("  ! {:?}: {}\n", w.kind, w.message));
            }
            s
        }
    }
}
