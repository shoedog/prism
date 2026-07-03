//! S3: MCP-only Concise-mode item slimming (`PRISM_MCP_CONCISE_SHAPE`).
//!
//! **Grounded fact:** Concise IS the MCP default verbosity (`input::parse_verbosity` returns
//! `Concise` when a tool call omits `verbosity`), so an unconditional slim projection would
//! silently change the default response shape for every existing MCP client. The slim shape is
//! therefore env-gated DEFAULT-OFF (`legacy`), exactly like S2's `StructuredContentMode`
//! (codex MAJOR).
//!
//! This operates on the already-shaped `serde_json::Value` (not the typed `Evidence`/
//! `EvidenceItem` structs) because those output types intentionally have no `Deserialize` impl
//! (never persisted, no cache round-trip). Working on the generic JSON tree also keeps this an
//! MCP-only post-processing step, entirely separate from the shared `Evidence` serde derive that
//! the CLI's byte-identical goldens (`tests/cli/nav_compat_test.rs`) depend on — no shared type is
//! touched.

use super::output::{McpToolResult, Verbosity};
use serde_json::Value;

/// Shape of items in a `Verbosity::Concise` result. `Legacy` (the default) is today's shape,
/// byte-unchanged. `Slim` drops per-item redundancy: symbol byte-offset/ordinal fields, a
/// `location` that duplicates the symbol's own file/line span, and a null `snippet` key.
/// `Verbosity::Detailed` and agent views are never affected by this mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConciseShapeMode {
    #[default]
    Legacy,
    Slim,
}

pub fn resolve_concise_shape_mode() -> ConciseShapeMode {
    resolve_concise_shape_mode_from(std::env::var("PRISM_MCP_CONCISE_SHAPE").ok().as_deref())
}

pub fn resolve_concise_shape_mode_from(value: Option<&str>) -> ConciseShapeMode {
    match value {
        None => ConciseShapeMode::Legacy,
        Some("legacy") => ConciseShapeMode::Legacy,
        Some("slim") => ConciseShapeMode::Slim,
        Some(other) => {
            eprintln!(
                "PRISM_MCP_CONCISE_SHAPE={other:?} is not \"legacy\" or \"slim\"; using \"legacy\""
            );
            ConciseShapeMode::Legacy
        }
    }
}

/// Applies the S3 item-slimming transform to a FINAL (non-agent-view) canonical `McpToolResult`.
/// No-op unless BOTH: `verbosity` is `Concise` AND `mode` is `Slim`.
///
/// Call this ONLY at a true terminal default-path return point
/// (`tools_reasoning::taint_reaches`'s return, `evidence_view::shape_navigation_result`'s early
/// non-agent-view return) — NEVER on an intermediate `McpToolResult` that might still be cloned
/// into an agent-view response (`evidence_view::clone_like`), or the slim shape would leak into
/// agent views' only canonical-Evidence carrier (`structuredContent`), which must stay UNCHANGED
/// always (codex-adjudicated).
pub fn apply_concise_shape(
    mut result: McpToolResult,
    verbosity: Verbosity,
    mode: ConciseShapeMode,
) -> McpToolResult {
    if result.is_error || mode == ConciseShapeMode::Legacy || verbosity != Verbosity::Concise {
        return result;
    }
    let Some(structured) = result.structured.clone() else {
        return result;
    };
    let slim = slim_evidence_value(structured);
    if let Ok(text) = serde_json::to_string_pretty(&slim) {
        result.content_text = text;
    }
    result.structured = Some(slim);
    result
}

fn slim_evidence_value(mut value: Value) -> Value {
    if let Some(items) = value
        .as_object_mut()
        .and_then(|obj| obj.get_mut("items"))
        .and_then(Value::as_array_mut)
    {
        for item in items.iter_mut() {
            slim_item_in_place(item);
        }
    }
    value
}

fn slim_item_in_place(item: &mut Value) {
    let Some(item_obj) = item.as_object_mut() else {
        return;
    };

    let symbol_span = item_obj
        .get_mut("symbol")
        .and_then(|symbol| symbol.as_object_mut())
        .and_then(symbol_variant_mut)
        .and_then(|inner| {
            let span = symbol_span_from_fields(inner);
            inner.remove("start_byte");
            inner.remove("end_byte");
            inner.remove("ordinal");
            span
        });

    if let Some(span) = symbol_span {
        let drop_location = item_obj
            .get("location")
            .is_some_and(|location| location_matches_span(location, &span));
        if drop_location {
            item_obj.remove("location");
        }
    }

    if item_obj.get("snippet").is_some_and(Value::is_null) {
        item_obj.remove("snippet");
    }
}

/// `symbol` is externally tagged (`{"Function": {...}}` / `{"Statement": {...}}` /
/// `{"Variable": {...}}` — exactly one entry, mirroring `SymbolRef`'s serde derive); returns the
/// inner fields object so the caller can strip byte fields + ordinal in place.
fn symbol_variant_mut(
    symbol: &mut serde_json::Map<String, Value>,
) -> Option<&mut serde_json::Map<String, Value>> {
    symbol.values_mut().next().and_then(Value::as_object_mut)
}

/// (file, start_line, end_line, start_byte, end_byte) — the full pre-strip span used to detect
/// whether `EvidenceItem.location` merely duplicates its own `symbol`.
type SymbolSpan = (String, u64, u64, u64, u64);

fn symbol_span_from_fields(inner: &serde_json::Map<String, Value>) -> Option<SymbolSpan> {
    let file = inner.get("file")?.as_str()?.to_string();
    let start_byte = inner.get("start_byte")?.as_u64()?;
    let end_byte = inner.get("end_byte")?.as_u64()?;
    // `Function` has start_line/end_line; `Statement`/`Variable` have a single `line`.
    let (start_line, end_line) =
        if let (Some(s), Some(e)) = (inner.get("start_line"), inner.get("end_line")) {
            (s.as_u64()?, e.as_u64()?)
        } else {
            let line = inner.get("line")?.as_u64()?;
            (line, line)
        };
    Some((file, start_line, end_line, start_byte, end_byte))
}

fn location_matches_span(location: &Value, span: &SymbolSpan) -> bool {
    let Some(loc) = location.as_object() else {
        return false;
    };
    let (file, start_line, end_line, start_byte, end_byte) = span;
    loc.get("file").and_then(Value::as_str) == Some(file.as_str())
        && loc.get("start_line").and_then(Value::as_u64) == Some(*start_line)
        && loc.get("end_line").and_then(Value::as_u64) == Some(*end_line)
        && loc.get("start_byte").and_then(Value::as_u64) == Some(*start_byte)
        && loc.get("end_byte").and_then(Value::as_u64) == Some(*end_byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::output::{shape_result, StructuredContentMode, Verbosity as OutVerbosity};
    use crate::navigation::types::*;
    use serde_json::json;

    #[test]
    fn resolve_concise_shape_mode_branches() {
        assert_eq!(
            resolve_concise_shape_mode_from(None),
            ConciseShapeMode::Legacy
        );
        assert_eq!(
            resolve_concise_shape_mode_from(Some("legacy")),
            ConciseShapeMode::Legacy
        );
        assert_eq!(
            resolve_concise_shape_mode_from(Some("slim")),
            ConciseShapeMode::Slim
        );
        assert_eq!(
            resolve_concise_shape_mode_from(Some("bogus")),
            ConciseShapeMode::Legacy
        ); // warn + default
    }

    fn item_with_matching_location(n: usize) -> EvidenceItem {
        EvidenceItem {
            symbol: Some(SymbolRef::Function {
                file: "a.rs".into(),
                name: format!("f{n}"),
                start_line: n,
                end_line: n,
                start_byte: n * 10,
                end_byte: n * 10 + 5,
                ordinal: 0,
            }),
            location: Location {
                file: "a.rs".into(),
                start_line: n,
                end_line: n,
                start_byte: n * 10,
                end_byte: n * 10 + 5,
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

    fn evidence_with(items: Vec<EvidenceItem>) -> Evidence {
        Evidence {
            query: "callees:x@a.rs".into(),
            items,
            truncated: false,
            warnings: vec![],
            graph: None,
            reasoning: None,
        }
    }

    fn result_for(items: Vec<EvidenceItem>, verbosity: OutVerbosity) -> McpToolResult {
        let n = items.len();
        shape_result(
            evidence_with(items),
            n,
            false,
            verbosity,
            100_000,
            StructuredContentMode::Always,
        )
    }

    #[test]
    fn legacy_mode_is_a_no_op_regardless_of_verbosity() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let before = concise.content_text.clone();
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Legacy);
        assert_eq!(after.content_text, before);
    }

    #[test]
    fn detailed_verbosity_is_a_no_op_even_under_slim_mode() {
        let detailed = result_for(vec![item_with_matching_location(1)], OutVerbosity::Detailed);
        let before = detailed.content_text.clone();
        let after = apply_concise_shape(detailed, OutVerbosity::Detailed, ConciseShapeMode::Slim);
        assert_eq!(after.content_text, before);
    }

    #[test]
    fn slim_drops_symbol_byte_fields_and_ordinal() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        let symbol = &v["items"][0]["symbol"]["Function"];
        assert_eq!(symbol["name"], "f1");
        assert_eq!(symbol["file"], "a.rs");
        assert_eq!(symbol["start_line"], 1);
        assert_eq!(symbol["end_line"], 1);
        assert!(symbol.get("start_byte").is_none());
        assert!(symbol.get("end_byte").is_none());
        assert!(symbol.get("ordinal").is_none());
    }

    #[test]
    fn slim_drops_location_when_it_duplicates_the_symbol_span() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        assert!(
            v["items"][0].get("location").is_none(),
            "{}",
            after.content_text
        );
    }

    #[test]
    fn slim_keeps_location_when_symbol_is_absent() {
        let item = EvidenceItem {
            symbol: None,
            location: Location {
                file: "util.py".into(),
                start_line: 1,
                end_line: 2,
                start_byte: 0,
                end_byte: 10,
            },
            score: 1.0,
            source: Source::HeuristicImport,
            fallback: false,
            why: vec![],
            snippet: None,
        };
        let concise = result_for(vec![item], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        assert_eq!(v["items"][0]["location"]["file"], "util.py");
    }

    #[test]
    fn slim_keeps_location_when_it_differs_from_the_symbol_span() {
        let mut item = item_with_matching_location(1);
        // Force a mismatch: the call-site location differs from the symbol's own definition span.
        item.location = Location {
            file: "a.rs".into(),
            start_line: 42,
            end_line: 42,
            start_byte: 999,
            end_byte: 1005,
        };
        let concise = result_for(vec![item], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        assert_eq!(v["items"][0]["location"]["start_line"], 42);
    }

    #[test]
    fn slim_omits_null_snippet_key_instead_of_serializing_null() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        assert!(!v["items"][0].as_object().unwrap().contains_key("snippet"));
    }

    #[test]
    fn slim_keeps_score_source_fallback_and_emptied_why() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let v: Value = serde_json::from_str(&after.content_text).unwrap();
        let item = &v["items"][0];
        assert_eq!(item["score"], 1.0);
        assert_eq!(item["source"], "PrismCpg");
        assert_eq!(item["fallback"], false);
        // Concise's existing `why.clear()` semantics (build_result_with_options) are untouched by
        // the slim projection — `why` stays present and empty, not removed.
        assert_eq!(item["why"], json!([]));
    }

    #[test]
    fn slim_structured_field_matches_content_text_projection() {
        let concise = result_for(vec![item_with_matching_location(1)], OutVerbosity::Concise);
        let after = apply_concise_shape(concise, OutVerbosity::Concise, ConciseShapeMode::Slim);
        let from_text: Value = serde_json::from_str(&after.content_text).unwrap();
        assert_eq!(after.structured.as_ref().unwrap(), &from_text);
    }

    #[test]
    fn slim_is_a_no_op_on_an_error_result() {
        let mut meta = serde_json::Map::new();
        meta.insert("prism/schema_version".into(), json!("0.2"));
        let error_result = McpToolResult {
            content_text: "boom".into(),
            structured: None,
            is_error: true,
            meta,
        };
        let after =
            apply_concise_shape(error_result, OutVerbosity::Concise, ConciseShapeMode::Slim);
        assert_eq!(after.content_text, "boom");
        assert!(after.structured.is_none());
    }
}
