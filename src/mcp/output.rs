use crate::navigation::types::{Evidence, GraphEdge, GraphPayload, Warning, WarningKind};
use crate::reasoning::types::SourceWarningKey;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The result-size cap is measured in **serialized UTF-8 wire bytes** (what the shaper compares via
/// `String::len()`), not Unicode scalar values. For the common ASCII JSON case bytes == chars; the
/// `_CHARS`/`Chars` names below (and the `PRISM_MCP_MAX_RESULT_CHARS` env var / `maxResultSizeChars`
/// meta key) are kept for experimental-v1 wire-contract stability. A separate pre-serialization
/// character limit is a documented follow-up (holistic re-review MINOR).
pub const MAX_RESULT_CHARS: usize = 80_000;
pub const MAX_RESULT_CHARS_FLOOR: usize = 12_000;
// S2: navigation symbols/locations carry byte ranges (additive).
pub const SCHEMA_VERSION: &str = "0.2";

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

    if let Some(candidate) =
        fit_reasoning_source_cap(&ev, n, total, max_results_clipped, verbosity, cap, budget)
    {
        return candidate;
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

fn fit_reasoning_source_cap(
    ev: &Evidence,
    retained: usize,
    total: usize,
    max_results_clipped: bool,
    verbosity: Verbosity,
    cap: usize,
    budget: usize,
) -> Option<McpToolResult> {
    let max_sources = max_sources_per_sink(ev)?;
    if max_sources > 1 {
        let mut lo = 1usize;
        let mut hi = max_sources - 1;
        let mut best = None;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            let candidate = build_result_with_options(
                ev,
                retained,
                total,
                max_results_clipped,
                verbosity,
                cap,
                Some(mid),
                false,
            );
            if candidate.serialized_len() <= budget {
                best = Some(mid);
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
        if let Some(limit) = best {
            return Some(build_result_with_options(
                ev,
                retained,
                total,
                max_results_clipped,
                verbosity,
                cap,
                Some(limit),
                false,
            ));
        }
    }

    let verdict_only = build_result_with_options(
        ev,
        retained,
        total,
        max_results_clipped,
        verbosity,
        cap,
        Some(0),
        true,
    );
    (verdict_only.serialized_len() <= budget).then_some(verdict_only)
}

fn max_sources_per_sink(ev: &Evidence) -> Option<usize> {
    ev.reasoning
        .as_ref()
        .filter(|reasoning| !reasoning.per_sink.is_empty())
        .map(|reasoning| {
            reasoning
                .per_sink
                .iter()
                .map(|sink| sink.sources.len())
                .max()
                .unwrap_or(0)
        })
}

fn retained_count(ev: &Evidence) -> usize {
    if let Some(reasoning) = &ev.reasoning {
        if !reasoning.per_sink.is_empty() {
            return reasoning.per_sink.len();
        }
    }
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
    build_result_with_options(
        ev,
        retained,
        total,
        max_results_clipped,
        verbosity,
        cap,
        None,
        false,
    )
}

fn build_result_with_options(
    ev: &Evidence,
    retained: usize,
    total: usize,
    max_results_clipped: bool,
    verbosity: Verbosity,
    cap: usize,
    source_retained: Option<usize>,
    force_compact_reasoning: bool,
) -> McpToolResult {
    let original_n = retained_count(ev);
    let mut shaped = ev.clone();
    shaped.items.truncate(retained);
    if let Verbosity::Concise = verbosity {
        for item in &mut shaped.items {
            item.why.clear();
        }
    }

    let mut source_clipped = false;
    if shaped
        .reasoning
        .as_ref()
        .is_some_and(|reasoning| !reasoning.per_sink.is_empty())
    {
        if let Some(reasoning) = &mut shaped.reasoning {
            reasoning.truncate_sinks(retained);
            let source_limit =
                source_retained.or_else(|| (retained < original_n).then_some(retained.max(1)));
            if let Some(source_limit) = source_limit {
                source_clipped = reasoning
                    .per_sink
                    .iter()
                    .any(|sink| source_limit < sink.sources.len());
                reasoning.truncate_sources(source_limit);
            }
            if matches!(verbosity, Verbosity::Concise) || force_compact_reasoning || source_clipped
            {
                reasoning.compact_non_verdict_detail();
            }
        }
        if source_clipped {
            retain_visible_source_warnings(&mut shaped);
        }
        if matches!(verbosity, Verbosity::Concise) || force_compact_reasoning || source_clipped {
            shaped.graph = None;
        } else {
            prune_graph_to_reasoning(&mut shaped);
        }
    } else if let Some(graph) = &mut shaped.graph {
        let kept = retained.min(graph.nodes.len());
        graph.nodes.truncate(kept);
        graph.edges = graph
            .edges
            .iter()
            .filter_map(|edge| retain_edge(edge, kept))
            .collect();
        if let Some(reasoning) = &mut shaped.reasoning {
            reasoning.repair_after_clip(kept);
        }
    }

    let adapter_clipped =
        max_results_clipped || retained < original_n || source_clipped || force_compact_reasoning;
    // Compose with the query's own truncation rather than clobbering it (round-6 MAJOR): a future nav
    // query may set truncated/ResultTruncated itself; don't reset it to false or drop its warning.
    shaped.truncated = ev.truncated || adapter_clipped;
    if adapter_clipped {
        // The adapter's own clip is authoritative (carries retained/total/detail state); replace
        // earlier adapter ResultTruncated warnings so max_results and byte-cap passes cannot emit
        // conflicting "showing N of M" counts.
        shaped
            .warnings
            .retain(|warning| warning.kind != WarningKind::ResultTruncated);
        shaped
            .warnings
            .push(result_truncated_warning(retained, total, source_clipped));
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

fn retain_visible_source_warnings(shaped: &mut Evidence) {
    let visible_source_keys = visible_source_warning_keys(shaped);
    shaped.warnings.retain(|warning| match &warning.kind {
        WarningKind::Reasoning(crate::reasoning::types::ReasoningWarning::Cleansed {
            source_function,
        }) => warning
            .location
            .as_ref()
            .map(|location| SourceWarningKey::from_warning(source_function, location))
            .is_some_and(|key| visible_source_keys.contains(&key)),
        _ => true,
    });
}

fn visible_source_warning_keys(shaped: &Evidence) -> BTreeSet<SourceWarningKey> {
    let mut keys = BTreeSet::new();
    let Some(reasoning) = &shaped.reasoning else {
        return keys;
    };
    for sink in &reasoning.per_sink {
        for source in &sink.sources {
            if let Some(key) = SourceWarningKey::from_symbol(&source.source) {
                keys.insert(key);
            }
        }
    }
    keys
}

fn prune_graph_to_reasoning(shaped: &mut Evidence) {
    let Some(graph) = shaped.graph.take() else {
        return;
    };
    let Some(reasoning) = &mut shaped.reasoning else {
        shaped.graph = Some(graph);
        return;
    };

    let mut sinks = BTreeSet::new();
    for sink in &reasoning.per_sink {
        for source in &sink.sources {
            if let Some(node) = source.graph_node {
                sinks.insert(node);
            }
        }
    }
    if sinks.is_empty() {
        shaped.graph = None;
        return;
    }

    let mut keep = sinks.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for edge in &graph.edges {
            if keep.contains(&edge.to) && keep.insert(edge.from) {
                changed = true;
            }
        }
    }

    let mut remap = BTreeMap::new();
    let mut nodes = Vec::new();
    for (old, node) in graph.nodes.into_iter().enumerate() {
        if keep.contains(&old) {
            remap.insert(old, nodes.len());
            nodes.push(node);
        }
    }
    let edges = graph
        .edges
        .into_iter()
        .filter_map(|edge| {
            let from = *remap.get(&edge.from)?;
            let to = *remap.get(&edge.to)?;
            Some(GraphEdge {
                from,
                to,
                kind: edge.kind,
            })
        })
        .collect();

    for sink in &mut reasoning.per_sink {
        for source in &mut sink.sources {
            if let Some(node) = source.graph_node {
                source.graph_node = remap.get(&node).copied();
            }
        }
    }
    reasoning.repair_after_clip(nodes.len());
    shaped.graph = Some(GraphPayload { nodes, edges });
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

fn result_truncated_warning(retained: usize, total: usize, source_clipped: bool) -> Warning {
    let detail = if source_clipped {
        "; per-sink source detail was also truncated to fit the result cap"
    } else {
        ""
    };
    Warning {
        kind: WarningKind::ResultTruncated,
        message: format!(
            "showing {retained} of {total}{detail}; raise max_results or narrow - e.g. lower depth/hops or a more specific seed"
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
                start_byte: 0,
                end_byte: 0,
                ordinal: 0,
            }),
            location: Location {
                file: "a.rs".into(),
                start_line: n,
                end_line: n,
                start_byte: 0,
                end_byte: 0,
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
            reasoning: None,
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
                            start_byte: 0,
                            end_byte: 0,
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
            reasoning: None,
        }
    }

    fn reasoning_graph() -> Evidence {
        Evidence {
            query: "taint_reaches".into(),
            items: vec![],
            truncated: false,
            warnings: vec![],
            graph: Some(GraphPayload {
                nodes: (0..4)
                    .map(|i| GraphNode {
                        symbol: None,
                        location: Location {
                            file: "a.py".into(),
                            start_line: i + 1,
                            end_line: i + 1,
                            start_byte: i,
                            end_byte: i + 1,
                        },
                    })
                    .collect(),
                edges: vec![
                    GraphEdge {
                        from: 0,
                        to: 1,
                        kind: "DataFlow".into(),
                    },
                    GraphEdge {
                        from: 2,
                        to: 3,
                        kind: "DataFlow".into(),
                    },
                ],
            }),
            reasoning: Some(ReasoningSummary {
                reachability: Some(Reachability::Reached),
                per_sink: vec![
                    SinkResult {
                        sink: sym("sink_a"),
                        reachability: Reachability::Reached,
                        sources: vec![SinkSourceResult {
                            source: sym("source_a"),
                            reachability: Reachability::Reached,
                            graph_node: Some(1),
                            sanitizers_present_in_source_fn: vec!["html".into()],
                        }],
                        sources_omitted: 0,
                    },
                    SinkResult {
                        sink: sym("sink_b"),
                        reachability: Reachability::Reached,
                        sources: vec![SinkSourceResult {
                            source: sym("source_b"),
                            reachability: Reachability::Reached,
                            graph_node: Some(3),
                            sanitizers_present_in_source_fn: vec![],
                        }],
                        sources_omitted: 0,
                    },
                ],
                source_count: 2,
                frontier_count: 4,
                sinks_omitted: 0,
            }),
        }
    }

    fn large_reasoning_graph(sinks: usize) -> Evidence {
        Evidence {
            query: "taint_reaches".into(),
            items: vec![],
            truncated: false,
            warnings: vec![],
            graph: Some(GraphPayload {
                nodes: (0..sinks * 2)
                    .map(|i| GraphNode {
                        symbol: None,
                        location: Location {
                            file: "a.py".into(),
                            start_line: i + 1,
                            end_line: i + 1,
                            start_byte: i,
                            end_byte: i + 1,
                        },
                    })
                    .collect(),
                edges: (0..sinks)
                    .map(|i| GraphEdge {
                        from: i * 2,
                        to: i * 2 + 1,
                        kind: "DataFlow".into(),
                    })
                    .collect(),
            }),
            reasoning: Some(ReasoningSummary {
                reachability: Some(Reachability::Reached),
                per_sink: (0..sinks)
                    .map(|i| SinkResult {
                        sink: sym(&format!("sink_{i}")),
                        reachability: Reachability::Reached,
                        sources: vec![SinkSourceResult {
                            source: sym(&format!("source_{i}")),
                            reachability: Reachability::Reached,
                            graph_node: Some(i * 2 + 1),
                            sanitizers_present_in_source_fn: vec![format!("sanitizer_{i}")],
                        }],
                        sources_omitted: 0,
                    })
                    .collect(),
                source_count: sinks,
                frontier_count: sinks * 2,
                sinks_omitted: 0,
            }),
        }
    }

    fn one_sink_many_sources(sources: usize) -> Evidence {
        Evidence {
            query: "taint_reaches".into(),
            items: vec![],
            truncated: false,
            warnings: vec![],
            graph: Some(GraphPayload {
                nodes: (0..sources + 1)
                    .map(|i| GraphNode {
                        symbol: None,
                        location: Location {
                            file: "a.py".into(),
                            start_line: i + 1,
                            end_line: i + 1,
                            start_byte: i,
                            end_byte: i + 1,
                        },
                    })
                    .collect(),
                edges: (0..sources)
                    .map(|i| GraphEdge {
                        from: i,
                        to: sources,
                        kind: "DataFlow".into(),
                    })
                    .collect(),
            }),
            reasoning: Some(ReasoningSummary {
                reachability: Some(Reachability::Reached),
                per_sink: vec![SinkResult {
                    sink: sym("sink"),
                    reachability: Reachability::Reached,
                    sources: (0..sources)
                        .map(|i| SinkSourceResult {
                            source: sym(&format!("source_{i}")),
                            reachability: Reachability::Reached,
                            graph_node: Some(sources),
                            sanitizers_present_in_source_fn: vec![format!(
                                "sanitizer_{i}_with_extra_context"
                            )],
                        })
                        .collect(),
                    sources_omitted: 0,
                }],
                source_count: sources,
                frontier_count: sources + 1,
                sinks_omitted: 0,
            }),
        }
    }

    fn one_sink_many_sources_same_function_with_warnings(sources: usize) -> Evidence {
        let mut ev = one_sink_many_sources(sources);
        let reasoning = ev.reasoning.as_mut().unwrap();
        for (i, source) in reasoning.per_sink[0].sources.iter_mut().enumerate() {
            source.source = sym_at("a.py", "shared_source_fn", i + 1, i * 10, i * 10 + 1);
            source.sanitizers_present_in_source_fn = vec![format!("sanitizer_{i}")];
            ev.warnings.push(cleansed_warning(
                "a.py",
                "shared_source_fn",
                i + 1,
                i * 10,
                i * 10 + 1,
                &format!("sanitizer_{i}"),
            ));
        }
        ev
    }

    fn one_sink_many_cross_file_same_function_name_with_warnings(sources: usize) -> Evidence {
        let mut ev = one_sink_many_sources(sources);
        let reasoning = ev.reasoning.as_mut().unwrap();
        for (i, source) in reasoning.per_sink[0].sources.iter_mut().enumerate() {
            let file = if i % 2 == 0 { "a.py" } else { "b.py" };
            source.source = sym_at(file, "shared_source_fn", i + 1, i * 10, i * 10 + 1);
            source.sanitizers_present_in_source_fn = vec![format!("sanitizer_{i}")];
            ev.warnings.push(cleansed_warning(
                file,
                "shared_source_fn",
                i + 1,
                i * 10,
                i * 10 + 1,
                &format!("sanitizer_{i}"),
            ));
        }
        ev
    }

    fn real_taint_reaches_many_sanitized_sources(sources: usize) -> Evidence {
        let mut src = String::from("import html\n\ndef f():\n");
        for i in 0..sources {
            src.push_str(&format!("    v{i} = input()\n"));
        }
        src.push_str("    safe = html.escape(v0)\n");
        src.push_str("    sink(v0)\n");

        let session = crate::mcp::tools::test_support::session(&[("app.py", &src)]);
        let source_specs = (0..sources)
            .map(|i| crate::reasoning::seeds::SeedSpec::Loc {
                file: "app.py".into(),
                line: 4 + i,
            })
            .collect::<Vec<_>>();
        let sink_specs = [crate::reasoning::seeds::SeedSpec::Loc {
            file: "app.py".into(),
            line: 4 + sources + 1,
        }];

        crate::reasoning::taint_reaches::taint_reaches(&session, &source_specs, Some(&sink_specs))
            .expect("taint_reaches evidence")
    }

    fn sym(name: &str) -> SymbolRef {
        sym_in("f", name)
    }

    fn sym_in(function: &str, name: &str) -> SymbolRef {
        SymbolRef::Variable {
            file: "a.py".into(),
            function: function.into(),
            line: 1,
            path: name.into(),
            access: "use".into(),
            start_byte: 0,
            end_byte: 0,
            ordinal: 0,
        }
    }

    fn sym_at(file: &str, function: &str, line: usize, start: usize, end: usize) -> SymbolRef {
        SymbolRef::Variable {
            file: file.into(),
            function: function.into(),
            line,
            path: format!("source_{line}"),
            access: "use".into(),
            start_byte: start,
            end_byte: end,
            ordinal: 0,
        }
    }

    fn cleansed_warning(
        file: &str,
        source_function: &str,
        line: usize,
        start: usize,
        end: usize,
        sanitizer: &str,
    ) -> Warning {
        Warning {
            kind: WarningKind::Reasoning(crate::reasoning::types::ReasoningWarning::Cleansed {
                source_function: source_function.into(),
            }),
            message: format!("{source_function} contains sanitizer categories: {sanitizer}"),
            location: Some(Location {
                file: file.into(),
                start_line: line,
                end_line: line,
                start_byte: start,
                end_byte: end,
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
        assert_eq!(r.meta["prism/schema_version"], "0.2");
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
    fn reasoning_sink_clip_prunes_witness_graph_and_preserves_sink_warning() {
        let mut ev = reasoning_graph();
        ev.reasoning.as_mut().unwrap().truncate_sinks(1);
        ev.truncated = true;
        ev.warnings.push(Warning {
            kind: WarningKind::ResultTruncated,
            message: "showing 1 of 2 sinks".into(),
            location: None,
        });

        let r = shape_result(ev, 2, false, Verbosity::Detailed, 100_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["reasoning"]["per_sink"].as_array().unwrap().len(), 1);
        assert_eq!(v["reasoning"]["sinks_omitted"], 1);
        assert_eq!(v["graph"]["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(v["reasoning"]["per_sink"][0]["sources"][0]["graph_node"], 1);
        assert_eq!(
            v["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|warning| warning["kind"] == "ResultTruncated")
                .count(),
            1
        );
    }

    #[test]
    fn reasoning_byte_cap_truncates_sinks_and_prunes_witness_graph() {
        let r = shape_result(
            large_reasoning_graph(60),
            60,
            false,
            Verbosity::Detailed,
            8_000,
        );
        assert!(!r.is_error && r.serialized_len() <= 8_000);

        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        let retained = v["reasoning"]["per_sink"].as_array().unwrap().len();
        let graph_nodes = v["graph"]["nodes"].as_array().unwrap().len();
        assert!(retained < 60);
        assert_eq!(v["truncated"], true);
        assert!(v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["kind"] == "ResultTruncated"));
        assert!(v["reasoning"]["per_sink"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|sink| sink["sources"].as_array().unwrap())
            .filter_map(|source| source["graph_node"].as_u64())
            .all(|node| (node as usize) < graph_nodes));
    }

    #[test]
    fn reasoning_byte_cap_truncates_one_sink_sources_before_terminal_error() {
        let r = shape_result(
            one_sink_many_sources(80),
            1,
            false,
            Verbosity::Detailed,
            8_000,
        );
        assert!(!r.is_error && r.serialized_len() <= 8_000);

        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(v["truncated"], true);
        assert!(v.get("graph").is_none());
        assert_eq!(v["reasoning"]["per_sink"].as_array().unwrap().len(), 1);
        assert!(
            v["reasoning"]["per_sink"][0]["sources"]
                .as_array()
                .unwrap()
                .len()
                < 80
        );
        assert!(v["reasoning"]["per_sink"][0]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|source| source["graph_node"].is_null()));
        assert!(
            v["reasoning"]["per_sink"][0]["sources_omitted"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["kind"] == "ResultTruncated"
                && warning["message"]
                    .as_str()
                    .unwrap()
                    .contains("per-sink source detail")));
    }

    fn assert_source_clip_warnings_match_retained_sources(ev: Evidence) {
        let r = shape_result(ev, 1, false, Verbosity::Detailed, 8_000);
        assert!(!r.is_error && r.serialized_len() <= 8_000);

        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        let visible_sources = v["reasoning"]["per_sink"][0]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| {
                let source = &source["source"]["Variable"];
                (
                    source["file"].as_str().unwrap().to_string(),
                    source["function"].as_str().unwrap().to_string(),
                    source["line"].as_u64().unwrap(),
                    source["start_byte"].as_u64().unwrap(),
                    source["end_byte"].as_u64().unwrap(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            v["reasoning"]["per_sink"][0]["sources_omitted"]
                .as_u64()
                .unwrap()
                > 0
        );

        let mut cleansed_warning_count = 0;
        for warning in v["warnings"].as_array().unwrap() {
            let Some(source_function) =
                warning["kind"]["Reasoning"]["Cleansed"]["source_function"].as_str()
            else {
                continue;
            };
            let location = &warning["location"];
            let warning_key = (
                location["file"].as_str().unwrap().to_string(),
                source_function.to_string(),
                location["start_line"].as_u64().unwrap(),
                location["start_byte"].as_u64().unwrap(),
                location["end_byte"].as_u64().unwrap(),
            );
            cleansed_warning_count += 1;
            assert!(
                visible_sources.contains(&warning_key),
                "warning leaked omitted source identity {warning_key:?}"
            );
        }
        assert!(
            cleansed_warning_count > 0,
            "retained source-specific warnings should remain visible"
        );
    }

    #[test]
    fn reasoning_source_clip_drops_same_function_warnings_for_omitted_sources() {
        assert_source_clip_warnings_match_retained_sources(
            one_sink_many_sources_same_function_with_warnings(80),
        );
    }

    #[test]
    fn reasoning_source_clip_drops_cross_file_same_function_warnings_for_omitted_sources() {
        assert_source_clip_warnings_match_retained_sources(
            one_sink_many_cross_file_same_function_name_with_warnings(80),
        );
    }

    #[test]
    fn reasoning_source_clip_filters_real_cleansed_warnings_by_retained_source_identity() {
        assert_source_clip_warnings_match_retained_sources(
            real_taint_reaches_many_sanitized_sources(80),
        );
    }

    #[test]
    fn reasoning_adapter_truncation_warning_is_replaced_not_duplicated() {
        let mut ev = large_reasoning_graph(60);
        ev.reasoning.as_mut().unwrap().truncate_sinks(10);
        ev.truncated = true;
        ev.warnings.push(Warning {
            kind: WarningKind::ResultTruncated,
            message: "showing 10 of 60 sinks".into(),
            location: None,
        });

        let r = shape_result(ev, 60, true, Verbosity::Detailed, 8_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert_eq!(
            v["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|warning| warning["kind"] == "ResultTruncated")
                .count(),
            1
        );
    }

    #[test]
    fn concise_reasoning_compacts_non_verdict_detail() {
        let r = shape_result(reasoning_graph(), 2, false, Verbosity::Concise, 100_000);
        let v: serde_json::Value = serde_json::from_str(&r.content_text).unwrap();
        assert!(v.get("graph").is_none());
        assert_eq!(
            v["reasoning"]["per_sink"][0]["sources"][0]["graph_node"],
            serde_json::Value::Null
        );
        assert!(
            v["reasoning"]["per_sink"][0]["sources"][0]["sanitizers_present_in_source_fn"]
                .as_array()
                .unwrap()
                .is_empty()
        );
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
        assert_eq!(resolve_cap_from(Some("100")), 80_000); // < FLOOR -> default
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
