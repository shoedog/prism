use crate::navigation::types::{Evidence, GraphEdge, Warning, WarningKind};
use serde_json::{Map, Value};

/// The result-size cap is measured in **serialized UTF-8 wire bytes** (what the shaper compares via
/// `String::len()`), not Unicode scalar values. For the common ASCII JSON case bytes == chars; the
/// `_CHARS`/`Chars` names below (and the `PRISM_MCP_MAX_RESULT_CHARS` env var / `maxResultSizeChars`
/// meta key) are kept for experimental-v1 wire-contract stability. A separate pre-serialization
/// character limit is a documented follow-up (holistic re-review MINOR).
pub const MAX_RESULT_CHARS: usize = 80_000;
pub const MAX_RESULT_CHARS_FLOOR: usize = 4_000;
pub const SCHEMA_VERSION: &str = "0.1";

/// Byte ceiling for echoing a user-controlled string into an error-path result.
pub(crate) const MAX_ECHO_BYTES: usize = 256;

/// Byte-bound a user-controlled string before it is interpolated into an error-path
/// `McpToolResult`. Error results bypass `shape_result` (codex MAJOR: the cap/budget machinery only
/// shapes the success path), so a hostile huge argument — an unknown tool `name`, a bad-arguments
/// `message`, or a user-provided symbol/path inside a `QueryError` — would otherwise flow to the wire
/// uncapped and blow past `PRISM_MCP_MAX_RESULT_CHARS`. Truncates on a UTF-8 char boundary (never
/// splits a multi-byte scalar, never panics) and appends `…` when it had to cut.
pub(crate) fn clamp_user_text(s: &str) -> String {
    if s.len() <= MAX_ECHO_BYTES {
        return s.to_string();
    }
    // Largest char boundary <= MAX_ECHO_BYTES so we never slice mid-scalar.
    let mut end = 0;
    for (idx, _) in s.char_indices() {
        if idx > MAX_ECHO_BYTES {
            break;
        }
        end = idx;
    }
    let mut out = s[..end].to_string();
    out.push('…');
    out
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Verbosity {
    #[default]
    Concise,
    Detailed,
}

pub struct McpToolResult {
    pub content_text: String,
    pub structured: Option<serde_json::Value>,
    pub is_error: bool,
    pub meta: serde_json::Map<String, serde_json::Value>,
}

impl McpToolResult {
    pub fn to_call_tool_result_value(&self) -> serde_json::Value {
        let mut value = Map::new();
        value.insert(
            "content".into(),
            serde_json::json!([{ "type": "text", "text": self.content_text }]),
        );
        if let Some(structured) = &self.structured {
            value.insert("structuredContent".into(), structured.clone());
        }
        value.insert("isError".into(), Value::Bool(self.is_error));
        value.insert("_meta".into(), Value::Object(self.meta.clone()));
        Value::Object(value)
    }

    pub fn serialized_len(&self) -> usize {
        self.to_call_tool_result_value().to_string().len()
    }
}

pub fn resolve_cap() -> usize {
    resolve_cap_from(std::env::var("PRISM_MCP_MAX_RESULT_CHARS").ok().as_deref())
}

pub fn resolve_cap_from(value: Option<&str>) -> usize {
    let Some(value) = value else {
        return MAX_RESULT_CHARS;
    };
    match value.parse::<usize>() {
        Ok(cap) if cap >= MAX_RESULT_CHARS_FLOOR => cap,
        Ok(cap) => {
            eprintln!(
                "PRISM_MCP_MAX_RESULT_CHARS={cap} is below the floor of {MAX_RESULT_CHARS_FLOOR}; using {MAX_RESULT_CHARS}"
            );
            MAX_RESULT_CHARS
        }
        Err(err) => {
            eprintln!(
                "could not parse PRISM_MCP_MAX_RESULT_CHARS={value:?}: {err}; using {MAX_RESULT_CHARS}"
            );
            MAX_RESULT_CHARS
        }
    }
}

pub fn shape_result(
    ev: Evidence,
    total: usize,
    max_results_clipped: bool,
    verbosity: Verbosity,
    cap: usize,
) -> McpToolResult {
    // The transport wraps `to_call_tool_result_value()` in a JSON-RPC success envelope
    // (`{"jsonrpc","id","result":…}`) before writing, so the wire bytes exceed the McpToolResult's
    // own length. The transport owns the envelope reserve (it builds the envelope) and exposes the net
    // payload budget, so the *wire* response stays under `cap` (holistic re-review MAJOR). `cap` is the
    // wire ceiling, reported verbatim in `_meta`. Production caps are floored at construction
    // (`resolve_cap_from` >= `MAX_RESULT_CHARS_FLOOR`); a sub-floor `cap` (only from tests) degrades
    // safely to the terminal `is_error` path rather than misbehaving. Enforcing the floor via a cap
    // newtype is a documented follow-up (holistic re-review MINOR).
    let budget = super::transport::payload_budget(cap);
    let n = retained_count(&ev);
    let full = build_result(&ev, n, total, max_results_clipped, verbosity, cap);
    if full.serialized_len() <= budget {
        return full;
    }

    if n >= 1 {
        let mut lo = 1usize;
        let mut hi = n - 1;
        let mut best = None;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            let candidate = build_result(&ev, mid, total, max_results_clipped, verbosity, cap);
            if candidate.serialized_len() <= budget {
                best = Some(mid);
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        if let Some(count) = best {
            return build_result(&ev, count, total, max_results_clipped, verbosity, cap);
        }
    }

    terminal_over_cap_result()
}

fn retained_count(ev: &Evidence) -> usize {
    ev.graph
        .as_ref()
        .map(|graph| graph.nodes.len())
        .unwrap_or_else(|| ev.items.len())
}

fn build_result(
    ev: &Evidence,
    retained: usize,
    total: usize,
    max_results_clipped: bool,
    verbosity: Verbosity,
    cap: usize,
) -> McpToolResult {
    let original_n = retained_count(ev);
    let mut shaped = ev.clone();
    shaped.items.truncate(retained);
    if let Verbosity::Concise = verbosity {
        for item in &mut shaped.items {
            item.why.clear();
        }
    }

    if let Some(graph) = &mut shaped.graph {
        let kept = retained.min(graph.nodes.len());
        graph.nodes.truncate(kept);
        graph.edges = graph
            .edges
            .iter()
            .filter_map(|edge| retain_edge(edge, kept))
            .collect();
    }

    let adapter_clipped = max_results_clipped || retained < original_n;
    // Compose with the query's own truncation rather than clobbering it (round-6 MAJOR): a future nav
    // query may set truncated/ResultTruncated itself; don't reset it to false or drop its warning.
    shaped.truncated = ev.truncated || adapter_clipped;
    if adapter_clipped {
        // The adapter's own clip is authoritative (carries retained/total); replace any prior
        // ResultTruncated.
        shaped
            .warnings
            .retain(|warning| warning.kind != WarningKind::ResultTruncated);
        shaped
            .warnings
            .push(result_truncated_warning(retained, total));
    }
    // else: leave the query's own warnings (including any ResultTruncated) intact.

    let content_text = crate::output::navigation::render(&shaped, "json");
    let structured = Some(serde_json::to_value(&shaped).unwrap_or_else(|_| serde_json::json!({})));
    McpToolResult {
        content_text,
        structured,
        is_error: false,
        meta: result_meta(cap),
    }
}

/// Retains edges whose endpoints survive a PREFIX clip (`nodes.truncate(kept)` keeps indices
/// `0..kept`), so original indices stay valid and need no remapping. If node selection ever becomes
/// non-contiguous, edges must be index-remapped instead.
fn retain_edge(edge: &GraphEdge, kept: usize) -> Option<GraphEdge> {
    if edge.from < kept && edge.to < kept {
        Some(GraphEdge {
            from: edge.from,
            to: edge.to,
            kind: edge.kind.clone(),
        })
    } else {
        None
    }
}

fn result_truncated_warning(retained: usize, total: usize) -> Warning {
    Warning {
        kind: WarningKind::ResultTruncated,
        message: format!(
            "showing {retained} of {total}; raise max_results or narrow - e.g. lower depth/hops or a more specific seed"
        ),
        location: None,
    }
}

fn result_meta(cap: usize) -> Map<String, Value> {
    let mut meta = Map::new();
    meta.insert(
        "prism/schema_version".into(),
        Value::String(SCHEMA_VERSION.into()),
    );
    meta.insert(
        "anthropic/maxResultSizeChars".into(),
        Value::Number(serde_json::Number::from(cap)),
    );
    meta
}

fn terminal_over_cap_result() -> McpToolResult {
    let mut meta = Map::new();
    meta.insert(
        "prism/schema_version".into(),
        Value::String(SCHEMA_VERSION.into()),
    );
    McpToolResult {
        content_text: "result exceeds size cap even at 1 item; narrow the query - e.g. lower depth/hops or a more specific seed".into(),
        structured: None,
        is_error: true,
        meta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::types::*;

    fn item(n: usize) -> EvidenceItem {
        EvidenceItem {
            symbol: Some(SymbolRef::Function {
                file: "a.rs".into(),
                name: format!("f{n}"),
                start_line: n,
                end_line: n,
                ordinal: 0,
            }),
            location: Location {
                file: "a.rs".into(),
                start_line: n,
                end_line: n,
            },
            score: 1.0,
            source: Source::PrismCpg,
            fallback: false,
            why: vec![Reason::Calls {
                callee: format!("g{n}"),
                call_site_line: n,
                qualifier: None,
            }],
            snippet: None,
        }
    }

    fn flat(n: usize) -> Evidence {
        Evidence {
            query: "callees:x@a.rs".into(),
            items: (0..n).map(item).collect(),
            truncated: false,
            warnings: vec![],
            graph: None,
        }
    }

    fn graph(nodes: usize) -> Evidence {
        Evidence {
            query: "repo-map".into(),
            items: vec![],
            truncated: false,
            warnings: vec![],
            graph: Some(GraphPayload {
                nodes: (0..nodes)
                    .map(|i| GraphNode {
                        symbol: None,
                        location: Location {
                            file: format!("f{i}.rs"),
                            start_line: 1,
                            end_line: 1,
                        },
                    })
                    .collect(),
                edges: (0..nodes.saturating_sub(1))
                    .map(|i| GraphEdge {
                        from: i,
                        to: i + 1,
                        kind: "ModuleDep".into(),
                    })
                    .collect(),
            }),
        }
    }

    #[test]
    fn full_under_cap_untruncated() {
        let r = shape_result(flat(2), 2, false, Verbosity::Detailed, 100_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["truncated"], false);
        assert!(!v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "ResultTruncated"));
        assert_eq!(r.meta["prism/schema_version"], "0.1");
        assert!(r.meta.contains_key("anthropic/maxResultSizeChars"));
    } // M12 positive _meta

    #[test]
    fn phase1_max_results_clip_keeps_warning() {
        // M10 composed phase-1 truncation
        let r = shape_result(flat(50), 500, true, Verbosity::Detailed, 100_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["truncated"], true);
        assert!(v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "ResultTruncated"));
    }

    #[test]
    fn query_own_truncation_survives_when_adapter_does_not_clip() {
        // Round-6 MAJOR: a future nav query may set `truncated:true` and emit its own ResultTruncated
        // warning. When the ADAPTER does not clip (retained == original_n, max_results_clipped=false),
        // build_result must COMPOSE — not clobber `truncated` back to false nor strip the query's
        // warning — so an agent never treats a partial result as exhaustive.
        let mut ev = flat(2);
        ev.truncated = true;
        ev.warnings.push(Warning {
            kind: WarningKind::ResultTruncated,
            message: "query-side: showing 2 of 999".into(),
            location: None,
        });
        let r = shape_result(ev, 2, false, Verbosity::Detailed, 100_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["truncated"], true, "query's own truncation must survive");
        assert!(
            v["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["kind"] == "ResultTruncated"),
            "query's own ResultTruncated warning must survive"
        );
    }

    #[test]
    fn over_cap_truncates_under_cap() {
        // cap large enough to truncate (not hit the terminal path) once the §6.3 envelope reserve
        // is applied; flat(200) (~80KB) still vastly exceeds it, so it truncates.
        let r = shape_result(flat(200), 200, false, Verbosity::Detailed, 8_000);
        assert!(!r.is_error && r.serialized_len() <= 8_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["truncated"], true);
    }

    #[test]
    fn terminal_over_cap_iserror_under_floor() {
        let r = shape_result(flat(200), 200, false, Verbosity::Detailed, 300);
        assert!(r.is_error && r.structured.is_none() && r.serialized_len() < 4_000);
    }

    #[test]
    fn graph_clip_edges_in_bounds() {
        let r = shape_result(graph(50), 50, false, Verbosity::Detailed, 6_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        let n = v["graph"]["nodes"].as_array().unwrap().len();
        assert!(v["graph"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| (e["from"].as_u64().unwrap() as usize) < n
                && (e["to"].as_u64().unwrap() as usize) < n));
        assert_eq!(v["truncated"], true);
    }

    #[test]
    fn concise_nulls_why() {
        let c: serde_json::Value = serde_json::from_str(
            &shape_result(flat(1), 1, false, Verbosity::Concise, 100_000).content_text,
        )
        .unwrap();
        assert!(c["items"][0]["why"].as_array().unwrap().is_empty());
    }

    #[test]
    fn detailed_content_is_render_byte_parity() {
        // M8 §13 golden
        let ev = flat(2);
        let r = shape_result(ev.clone(), 2, false, Verbosity::Detailed, 100_000);
        assert_eq!(
            r.content_text,
            crate::output::navigation::render(&ev, "json")
        );
    }

    #[test]
    fn resolve_cap_branches() {
        // M7 §6.5 env parse
        assert_eq!(resolve_cap_from(None), 80_000);
        assert_eq!(resolve_cap_from(Some("bad")), 80_000); // warn + default
        assert_eq!(resolve_cap_from(Some("100")), 80_000); // < FLOOR(4000) -> default
        assert_eq!(resolve_cap_from(Some("50000")), 50_000);
    }

    #[test]
    fn clamp_user_text_bounds_and_never_splits_chars() {
        // Short ASCII passes through untouched.
        assert_eq!(clamp_user_text("ok"), "ok");

        // Long ASCII is truncated under the bound with a marker.
        let long = "x".repeat(5000);
        let clamped = clamp_user_text(&long);
        assert!(clamped.len() <= MAX_ECHO_BYTES + '…'.len_utf8());
        assert!(clamped.ends_with('…'));

        // A multi-byte string that would otherwise split a scalar at the boundary must not panic and
        // must stay bounded — '€' is 3 bytes, so the boundary lands mid-scalar without char-aware cut.
        let multibyte = "€".repeat(5000);
        let clamped = clamp_user_text(&multibyte);
        assert!(clamped.len() <= MAX_ECHO_BYTES + '…'.len_utf8());
        assert!(clamped.ends_with('…'));
        // Round-trips as valid UTF-8 (no truncated scalar).
        assert!(std::str::from_utf8(clamped.as_bytes()).is_ok());
    }
}
