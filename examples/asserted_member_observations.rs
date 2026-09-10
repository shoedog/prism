//! Bounded stdin-only observations; no normalization or runtime authority.
use anyhow::{ensure, Result};
use prism::{ast::ParsedFile, data_flow::DataFlowGraph, languages::Language};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

const MAX_INPUT: usize = 2 * 1024 * 1024;
const MAX_SOURCE: usize = 16 * 1024;
const MAX_CASES: usize = 64;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Dialect {
    TypeScript,
    Tsx,
    JavaScript,
}

impl Dialect {
    fn native(self) -> (Language, &'static str) {
        match self {
            Self::TypeScript => (Language::TypeScript, "fixture.ts"),
            Self::Tsx => (Language::Tsx, "fixture.tsx"),
            Self::JavaScript => (Language::JavaScript, "fixture.js"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    language: Dialect,
    source: String,
    lines: Vec<usize>,
}

fn flatten(graph: DataFlowGraph) -> Value {
    let labels = graph
        .labels
        .iter()
        .map(|((from, to), confidence)| json!({"from":from,"to":to,"confidence":confidence}))
        .collect::<Vec<_>>();
    json!({"defs":graph.defs.values().flatten().collect::<Vec<_>>(),
        "uses":graph.uses.values().flatten().collect::<Vec<_>>(), "edges":graph.edges,"labels":labels})
}

fn observe(case: Case) -> Result<Value> {
    let (language, path) = case.language.native();
    let parsed = ParsedFile::parse(path, &case.source, language)?;
    let calls = parsed.call_names_on_lines(&case.lines);
    let lines = case.lines.into_iter().collect::<BTreeSet<_>>();
    let root = parsed.tree.root_node();
    let names = parsed.rvalue_identifiers_on_lines(&root, &lines);
    let paths = parsed.rvalue_identifier_paths_on_lines(&root, &lines);
    let spans = parsed
        .rvalue_identifier_spans_on_lines(&root, &lines)
        .into_iter()
        .map(|span| {
            json!({"path":span.path,"line":span.line,"start_byte":span.start_byte,
            "end_byte":span.end_byte,
            "semantic_value_use":parsed.is_semantic_value_use(span.start_byte, span.end_byte)})
        })
        .collect::<Vec<_>>();
    let returns =
        parsed
            .all_functions()
            .into_iter()
            .flat_map(|function| {
                parsed.return_value_nodes(&function).into_iter().map(|ret| {
                    let values = ret.values.into_iter().map(|value| json!({
                "slot":value.slot,"line":value.line,"text":value.text,"kind":value.kind,
                "start_byte":value.start_byte,"end_byte":value.end_byte
            })).collect::<Vec<_>>();
                    json!({"line":ret.line,"value_text":ret.value_text,"value_kind":ret.value_kind,
                "is_conditional":ret.is_conditional,"start_byte":ret.start_byte,
                "end_byte":ret.end_byte,"values":values})
                })
            })
            .collect::<Vec<_>>();
    let parse_errors = parsed.parse_error_count;
    let tree = root.to_sexp();
    let files = BTreeMap::from([(path.to_string(), parsed)]);
    let full = flatten(DataFlowGraph::build(&files));
    let subset = flatten(DataFlowGraph::build_subset(
        &files,
        &BTreeSet::from([path.to_string()]),
    ));
    Ok(
        json!({"id":case.id,"language":case.language,"source":case.source,
        "parse_errors":parse_errors,"tree":tree,"calls":calls,"names":names,"paths":paths,
        "spans":spans,"returns":returns,"full":full,"subset":subset}),
    )
}

fn observations(input: impl Read) -> Result<Vec<Value>> {
    let mut bytes = Vec::new();
    input.take((MAX_INPUT + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_INPUT, "input byte limit");
    let cases: Vec<Case> = serde_json::from_slice(&bytes)?;
    ensure!(
        !cases.is_empty() && cases.len() <= MAX_CASES,
        "case count limit"
    );
    let mut ids = BTreeSet::new();
    for case in &cases {
        ensure!(ids.insert(&case.id), "duplicate case ID");
        ensure!(case.source.len() <= MAX_SOURCE, "source byte limit");
        // Source rows match newline-delimited editor/AST rows, including EOF's
        // empty row after a trailing newline. Empty text has one empty row.
        let last_line = case.source.split('\n').count();
        ensure!(
            case.lines
                .iter()
                .all(|line| *line > 0 && *line <= last_line),
            "line out of source range"
        );
    }
    prism::build_pool::install(|| cases.into_iter().map(observe).collect())
}

fn main() -> Result<()> {
    ensure!(
        std::env::args_os().len() == 1,
        "expected JSON on stdin, no arguments"
    );
    let output = serde_json::to_string(&observations(std::io::stdin().lock())?)?;
    println!("{output}"); // Nothing is emitted until the entire batch succeeds.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str, language: &str, source: &str, lines: Value) -> Value {
        json!({"id":id,"language":language,"source":source,"lines":lines})
    }

    fn run(value: Value) -> Result<Vec<Value>> {
        observations(value.to_string().as_bytes())
    }

    #[test]
    fn rejects_unknown_missing_duplicate_fields_and_wrong_shapes() {
        for input in [
            "{}",
            "null",
            "[{}]",
            r#"[{"id":"x","language":"python","source":"","lines":[]}]"#,
            r#"[{"id":"x","language":"tsx","source":"","lines":[],"extra":true}]"#,
            r#"[{"id":"x","id":"y","language":"tsx","source":"","lines":[]}]"#,
            r#"[{"id":"x","language":"tsx","source":"","lines":[-1]}]"#,
        ] {
            assert!(observations(input.as_bytes()).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_empty_duplicate_and_oversized_batches() {
        assert!(run(json!([])).is_err());
        let row = case("x", "typescript", "", json!([]));
        assert!(run(json!([row, row])).is_err());
        let rows = (0..=MAX_CASES)
            .map(|i| case(&i.to_string(), "tsx", "", json!([])))
            .collect::<Vec<_>>();
        assert!(run(json!(rows)).is_err());
        assert_eq!(run(json!(&rows[..MAX_CASES])).unwrap().len(), MAX_CASES);
    }

    #[test]
    fn rejects_out_of_range_lines_but_preserves_empty_last_row() {
        for lines in [json!([0]), json!([3])] {
            assert!(run(json!([case("x", "typescript", "\n", lines)])).is_err());
        }
        assert!(run(json!([case("x", "typescript", "\n", json!([1, 2]))])).is_ok());
        assert!(run(json!([case("x", "typescript", "", json!([1]))])).is_ok());
    }

    #[test]
    fn enforces_input_and_source_byte_limits() {
        assert!(observations(vec![b' '; MAX_INPUT + 1].as_slice()).is_err());
        let source = format!("//{}", "x".repeat(MAX_SOURCE - 2));
        assert!(run(json!([case("x", "tsx", &source, json!([1]))])).is_ok());
        assert!(run(json!([case("x", "tsx", &(source + "x"), json!([1]))])).is_err());
    }

    #[test]
    fn preserves_input_order_dialects_and_flat_graph_shapes() {
        let rows = [
            case("z", "javascript", "", json!([])),
            case("a", "tsx", "", json!([])),
            case("m", "typescript", "", json!([])),
        ];
        let out = run(json!(rows)).unwrap();
        for (row, observed) in rows.iter().zip(out) {
            assert_eq!(observed["id"], row["id"]);
            assert_eq!(observed["language"], row["language"]);
            assert!(observed["calls"].is_object());
            for kind in ["full", "subset"] {
                for field in ["defs", "uses", "edges", "labels"] {
                    assert!(observed[kind][field].is_array());
                }
            }
        }
    }

    #[test]
    fn preserves_non_ascii_source_and_raw_return_byte_slots() {
        let source = "function f(é: string) {\n return é;\n}";
        let out = run(json!([case("unicode", "typescript", source, json!([2]))])).unwrap();
        let row = &out[0];
        assert_eq!(row["source"], source);
        let value = &row["returns"][0]["values"][0];
        let start = value["start_byte"].as_u64().unwrap() as usize;
        let end = value["end_byte"].as_u64().unwrap() as usize;
        assert_eq!(&source[start..end], "é");
        assert_eq!(value["text"], "é");
        assert_eq!(value["slot"], 0);
        for span in row["spans"].as_array().unwrap() {
            let start = span["start_byte"].as_u64().unwrap() as usize;
            let end = span["end_byte"].as_u64().unwrap() as usize;
            assert!(source.get(start..end).is_some());
            assert!(span["semantic_value_use"].is_boolean());
        }
    }

    #[test]
    fn records_parse_errors_without_repairing_source() {
        let source = "function broken( {";
        let out = run(json!([case("broken", "javascript", source, json!([1]))])).unwrap();
        assert_eq!(out[0]["source"], source);
        assert!(out[0]["parse_errors"].as_u64().unwrap() > 0);
        assert!(out[0]["tree"].is_string());
    }
}
