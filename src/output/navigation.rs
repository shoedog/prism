use crate::navigation::types::{Evidence, Location, QueryError, SymbolSpans};

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

pub fn render_symbol_spans(result: &SymbolSpans, format: &str) -> String {
    fn push_location(output: &mut String, label: &str, location: Option<&Location>) {
        match location {
            Some(location) => output.push_str(&format!(
                "  {label}: {}:{}-{} bytes {}..{}\n",
                location.file,
                location.start_line,
                location.end_line,
                location.start_byte,
                location.end_byte
            )),
            None => output.push_str(&format!("  {label}: unavailable\n")),
        }
    }

    if format == "json" {
        return serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".into());
    }

    let mut output = format!("{}\n", result.query);
    push_location(&mut output, "symbol_span", Some(&result.symbol_span));
    push_location(&mut output, "name_span", result.name_span.as_ref());
    push_location(&mut output, "body_span", result.body_span.as_ref());
    output.push_str(&format!(
        "  insert_before: {}:{} byte {}\n",
        result.insert_before.file, result.insert_before.line, result.insert_before.byte
    ));
    output.push_str(&format!(
        "  insert_after: {}:{} byte {}\n",
        result.insert_after.file, result.insert_after.line, result.insert_after.byte
    ));
    match &result.indentation.symbol {
        Some(value) => output.push_str(&format!("  indentation.symbol: {value:?}\n")),
        None => output.push_str("  indentation.symbol: unavailable\n"),
    }
    match &result.indentation.body {
        Some(value) => output.push_str(&format!("  indentation.body: {value:?}\n")),
        None => output.push_str("  indentation.body: unavailable\n"),
    }
    for (field, reason) in &result.unavailable {
        output.push_str(&format!("  unavailable {field}: {reason}\n"));
    }
    for warning in &result.warnings {
        output.push_str(&format!("  ! {:?}: {}\n", warning.kind, warning.message));
    }
    output
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
                            // `call_site_line` is a SOURCE-side line; do NOT pair it with
                            // `it.location.file` (target-side for module_deps) — that would
                            // name the wrong file (holistic-review MAJOR). The target file is
                            // already shown in the item header line above.
                            s.push_str(&format!(
                                "    calls {} @ call-site line {}\n",
                                callee, call_site_line
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
            if let Some(graph) = &ev.graph {
                for (i, n) in graph.nodes.iter().enumerate() {
                    s.push_str(&format!(
                        "  [{i}] {}:{}\n",
                        n.location.file, n.location.start_line
                    ));
                }
                for e in &graph.edges {
                    s.push_str(&format!("  {} --{}--> {}\n", e.from, e.kind, e.to));
                }
            }
            s
        }
    }
}
