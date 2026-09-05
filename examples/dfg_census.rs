//! Throwaway census binary for the item-2 spec §12 Q1 measurement pass:
//! size the reaching-definitions caps (`RD_MAX_DEFS`, `RD_MAX_LINES`) with data.
//!
//! Emits one CSV row per function the DFG builds over, plus a per-repo summary
//! JSON with cold (no-cache) build wall times.
//!
//! Usage:
//!   cargo run --release --example dfg_census -- <repo-root> <label> <out-dir>
//!
//! Columns (CSV): repo,lang,file,function,start_line,end_line,n_lines,n_defs,
//! n_defs_raw,n_uses,n_uses_raw,n_cfg_nodes,n_cfg_edges,n_dfg_edges,n_dfg_edges_cfg_ok,
//! n_dfg_edges_span_ok,n_dfg_edges_distinct_stmt,n_dfg_edges_nested_callable,
//! n_dfg_edges_unmapped_use,n_dfg_edges_unmapped_def,n_dfg_edges_same_stmt
//!
//! Column definitions, matching spec §4.1–4.3.
//!
//! `n_defs` is the DefSite count: distinct `(AccessPath, line)` Def occurrences in the
//! function, i.e. distinct `VarLocation` Def identities (`VarLocation::identity_key`,
//! `src/data_flow.rs:29-41`). This is the RD bit-vector width. It includes parameter
//! Defs and alias-resolved twins, exactly as `DataFlowGraph::build` records them.
//!
//! `n_defs_raw` / `n_uses_raw` are the un-deduplicated `Vec<VarLocation>` push counts,
//! which bound the width an occurrence-granular `DefId` identity would need.
//!
//! `n_lines` is `function_end_line - function_start_line + 1` (`node_line_range`).
//!
//! `n_cfg_nodes` is `ParsedFile::statements_in_function(func).len()` — the CFG's line
//! universe, and the `RD_MAX_LINES` quantity in spec §4.2 step 2.
//!
//! `n_cfg_edges` counts CFG edges from `cfg::build_cfg_edges(parsed)` whose `from_line`
//! is one of this function's statement lines. `build_function_cfg` is private, so a
//! per-function CFG is not directly obtainable.
//!
//! `n_dfg_edges_cfg_ok` counts DFG edges RD could label at all: use line in the CFG line
//! universe, def line in it or at the function start (the synthetic ENTRY of §4.2 step 4).
//!
//! `n_dfg_edges_span_ok` applies the v6.3 form of that test: each non-ENTRY endpoint line
//! may fall anywhere from the start through end line of any statement span. Edges with
//! either endpoint byte-contained by a nested callable body are excluded from this
//! numerator and its denominator; `n_dfg_edges_nested_callable` reports that population.
//!
//! `n_dfg_edges_distinct_stmt` is the v6.4 RD-relevant metric. It additionally requires
//! the two endpoints to map to different innermost containing statement start lines, or
//! permits the definition to be the function ENTRY.
//!
//! This binary reads only public API (`prism::ast`, `prism::cfg`, `prism::cpg`,
//! `prism::data_flow`, `prism::repo_loader`) and changes nothing under `src/`.

use prism::ast::ParsedFile;
use prism::cfg;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{DataFlowGraph, VarAccessKind};
use prism::languages::Language;
use prism::repo_loader;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tree_sitter::Node;

fn lang_name(l: Language) -> &'static str {
    match l {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Rust => "rust",
        Language::Lua => "lua",
        Language::Terraform => "terraform",
        Language::Tsx => "tsx",
        Language::Bash => "bash",
    }
}

/// Identity of a function as the DFG keys it: (file, name, function_start_line).
type FnKey = (String, String, usize);

#[derive(Default, Clone)]
struct Counts {
    defs: usize,
    defs_raw: usize,
    uses: usize,
    uses_raw: usize,
    dfg_edges: usize,
    /// Source coordinates of every DFG edge in this function, retained so the
    /// census can measure both line and lexical-body containment.
    edge_endpoints: Vec<EdgeEndpoints>,
}

#[derive(Clone, Copy, Debug)]
struct EdgeEndpoints {
    def_line: usize,
    def_start_byte: usize,
    def_end_byte: usize,
    use_line: usize,
    use_start_byte: usize,
    use_end_byte: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EdgeAdmissibility {
    cfg_ok: usize,
    span_ok: usize,
    distinct_stmt: usize,
    nested_callable: usize,
    /// Mutually exclusive residual classes after nested-callable exclusion.
    unmapped_use: usize,
    unmapped_def: usize,
    same_stmt: usize,
}

#[derive(Clone, Copy, Debug)]
struct ByteRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug)]
struct StatementLineSpan {
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
}

impl ByteRange {
    fn contains(self, start: usize, end: usize) -> bool {
        self.start <= start && end <= self.end
    }
}

fn edge_admissibility(
    parsed: &ParsedFile,
    func_node: Node<'_>,
    function_start: usize,
    stmt_lines: &BTreeSet<usize>,
    source_line_starts: &[usize],
    edges: &[EdgeEndpoints],
) -> EdgeAdmissibility {
    let statement_spans: Vec<StatementLineSpan> = parsed
        .statement_spans_in_function(&func_node)
        .into_iter()
        .map(|span| {
            let last_byte = span.end_byte.saturating_sub(1).max(span.start_byte);
            StatementLineSpan {
                start_line: span.line,
                end_line: line_at_byte(source_line_starts, last_byte),
                start_byte: span.start_byte,
                end_byte: span.end_byte,
            }
        })
        .collect();
    let nested_callable_bodies = nested_callable_body_ranges(parsed, func_node);
    let mut result = EdgeAdmissibility::default();

    for edge in edges {
        if stmt_lines.contains(&edge.use_line)
            && (stmt_lines.contains(&edge.def_line) || edge.def_line == function_start)
        {
            result.cfg_ok += 1;
        }

        let touches_nested_callable = nested_callable_bodies.iter().any(|body| {
            body.contains(edge.def_start_byte, edge.def_end_byte)
                || body.contains(edge.use_start_byte, edge.use_end_byte)
        });
        if touches_nested_callable {
            result.nested_callable += 1;
            continue;
        }

        let use_stmt = innermost_statement_start(edge.use_line, &statement_spans);
        let def_is_entry = edge.def_line == function_start;
        let def_stmt = (!def_is_entry)
            .then(|| innermost_statement_start(edge.def_line, &statement_spans))
            .flatten();

        let Some(use_stmt) = use_stmt else {
            result.unmapped_use += 1;
            continue;
        };
        if !def_is_entry && def_stmt.is_none() {
            result.unmapped_def += 1;
            continue;
        }

        result.span_ok += 1;
        if def_is_entry || def_stmt != Some(use_stmt) {
            result.distinct_stmt += 1;
        } else {
            result.same_stmt += 1;
        }
    }

    debug_assert_eq!(result.span_ok, result.distinct_stmt + result.same_stmt);
    debug_assert_eq!(
        edges.len(),
        result.nested_callable
            + result.unmapped_use
            + result.unmapped_def
            + result.same_stmt
            + result.distinct_stmt
    );

    result
}

fn source_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        source
            .as_bytes()
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b'\n').then_some(index + 1)),
    );
    starts
}

fn line_at_byte(line_starts: &[usize], byte: usize) -> usize {
    line_starts.partition_point(|start| *start <= byte).max(1)
}

fn innermost_statement_start(line: usize, spans: &[StatementLineSpan]) -> Option<usize> {
    spans
        .iter()
        .filter(|span| span.start_line <= line && line <= span.end_line)
        .min_by_key(|span| {
            (
                span.end_byte.saturating_sub(span.start_byte),
                Reverse(span.start_byte),
                Reverse(span.start_line),
            )
        })
        .map(|span| span.start_line)
}

fn nested_callable_body_ranges(parsed: &ParsedFile, func_node: Node<'_>) -> Vec<ByteRange> {
    fn collect(language: Language, node: Node<'_>, out: &mut Vec<ByteRange>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if is_nested_callable_kind(language, child.kind()) {
                let body = callable_body_node(language, child).unwrap_or(child);
                out.push(ByteRange {
                    start: body.start_byte(),
                    end: body.end_byte(),
                });
                continue;
            }
            collect(language, child, out);
        }
    }

    let mut out = Vec::new();
    let body = callable_body_node(parsed.language, func_node).unwrap_or(func_node);
    collect(parsed.language, body, &mut out);
    out
}

fn callable_body_node(language: Language, node: Node<'_>) -> Option<Node<'_>> {
    let callable = if language == Language::Python && node.kind() == "decorated_definition" {
        let mut cursor = node.walk();
        let function = node
            .children(&mut cursor)
            .find(|child| child.kind() == "function_definition")
            .unwrap_or(node);
        function
    } else if language == Language::Cpp && node.kind() == "template_declaration" {
        let mut cursor = node.walk();
        let function = node
            .children(&mut cursor)
            .find(|child| child.kind() == "function_definition")
            .unwrap_or(node);
        function
    } else {
        node
    };

    callable
        .child_by_field_name("body")
        .or_else(|| callable.child_by_field_name("consequence"))
}

fn is_nested_callable_kind(language: Language, kind: &str) -> bool {
    match language {
        Language::Python => matches!(
            kind,
            "function_definition" | "decorated_definition" | "lambda"
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => matches!(
            kind,
            "function_declaration"
                | "method_definition"
                | "arrow_function"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
                | "generator_function_expression"
        ),
        Language::Go => matches!(
            kind,
            "function_declaration" | "method_declaration" | "func_literal"
        ),
        Language::Rust => matches!(
            kind,
            "function_item" | "closure_expression" | "async_block" | "gen_block"
        ),
        Language::Java => matches!(
            kind,
            "method_declaration" | "constructor_declaration" | "lambda_expression"
        ),
        Language::Cpp => matches!(
            kind,
            "function_definition" | "template_declaration" | "lambda_expression"
        ),
        Language::C => kind == "function_definition",
        Language::Lua => matches!(kind, "function_declaration" | "function_definition"),
        Language::Bash => kind == "function_definition",
        Language::Terraform => false,
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: dfg_census <repo-root> <label> <out-dir>");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[1]);
    let label = args[2].clone();
    let out_dir = PathBuf::from(&args[3]);
    std::fs::create_dir_all(&out_dir)?;

    // The whole census runs on prism's large-stack build pool: `ParsedFile::parse`
    // and the CPG build recurse over the AST and overflow a default rayon worker
    // stack on deeply nested files (`src/build_pool.rs`).
    prism::api::with_build_pool(|| run(&root, &label, &out_dir))
}

fn run(root: &Path, label: &str, out_dir: &Path) -> anyhow::Result<()> {
    let t_all = Instant::now();

    // ---- 1. Cold parse of the whole repo (no cache exists for this path at all;
    //         `repo_loader::load_repo` reads and parses from disk every time).
    let t0 = Instant::now();
    let loaded = repo_loader::load_repo(root)?;
    let parse_secs = t0.elapsed().as_secs_f64();
    let files: BTreeMap<String, ParsedFile> = loaded.files;
    let n_files = files.len();
    let n_skipped = loaded.skipped.len();

    // ---- 2. Cold DFG build (this is the pass the RD work would live inside).
    let t1 = Instant::now();
    let dfg = DataFlowGraph::build(&files);
    let dfg_secs = t1.elapsed().as_secs_f64();

    // ---- 3. Fold the DFG into per-function counts.
    let mut counts: BTreeMap<FnKey, Counts> = BTreeMap::new();
    for ((file, func, start, _path), locs) in &dfg.defs {
        let e = counts
            .entry((file.clone(), func.clone(), *start))
            .or_default();
        e.defs_raw += locs.len();
        // All locs under this key share (file, function, start, path, Def), so
        // distinct VarLocation identity within the Vec == distinct `line`.
        let distinct: BTreeSet<usize> = locs.iter().map(|l| l.line).collect();
        e.defs += distinct.len();
    }
    for ((file, func, start, _path), locs) in &dfg.uses {
        let e = counts
            .entry((file.clone(), func.clone(), *start))
            .or_default();
        e.uses_raw += locs.len();
        let distinct: BTreeSet<usize> = locs.iter().map(|l| l.line).collect();
        e.uses += distinct.len();
    }
    for edge in &dfg.edges {
        debug_assert_eq!(edge.from.kind, VarAccessKind::Def);
        let e = counts
            .entry((
                edge.from.file.clone(),
                edge.from.function.clone(),
                edge.from.function_start_line,
            ))
            .or_default();
        e.dfg_edges += 1;
        e.edge_endpoints.push(EdgeEndpoints {
            def_line: edge.from.line,
            def_start_byte: edge.from.start_byte,
            def_end_byte: edge.from.end_byte,
            use_line: edge.to.line,
            use_start_byte: edge.to.start_byte,
            use_end_byte: edge.to.end_byte,
        });
    }

    // ---- 4. Walk the AST the same way `DataFlowGraph::build` does, so the CSV
    //         has a row per function the DFG iterates, joined to CFG shape.
    let mut csv = String::new();
    csv.push_str(
        "repo,lang,file,function,start_line,end_line,n_lines,n_defs,n_defs_raw,n_uses,n_uses_raw,n_cfg_nodes,n_cfg_edges,n_dfg_edges,n_dfg_edges_cfg_ok,n_dfg_edges_span_ok,n_dfg_edges_distinct_stmt,n_dfg_edges_nested_callable,n_dfg_edges_unmapped_use,n_dfg_edges_unmapped_def,n_dfg_edges_same_stmt\n",
    );
    let mut n_functions = 0usize;
    let mut n_functions_unnamed = 0usize;
    let mut seen: BTreeSet<FnKey> = BTreeSet::new();

    for (path, parsed) in &files {
        let lang = lang_name(parsed.language);
        let file_line_starts = source_line_starts(&parsed.source);
        // One CFG build per file (this is exactly what CPG Step 8 does).
        let file_edges = cfg::build_cfg_edges(parsed);
        let mut from_lines: BTreeMap<usize, usize> = BTreeMap::new();
        for e in &file_edges {
            *from_lines.entry(e.from_line).or_insert(0) += 1;
        }

        for func_node in parsed.all_functions() {
            let func_name = match parsed.language.function_name(&func_node) {
                Some(n) => parsed.node_text(&n).to_string(),
                None => {
                    // `DataFlowGraph::build` skips these (`continue`), so they carry
                    // no Defs/Uses; counted but not emitted.
                    n_functions_unnamed += 1;
                    continue;
                }
            };
            let (start, end) = parsed.node_line_range(&func_node);
            let key: FnKey = (path.clone(), func_name.clone(), start);
            // Two AST nodes can collapse onto one DFG key (same name, same start
            // line). Emit one row per DFG key so the census matches the DFG.
            if !seen.insert(key.clone()) {
                continue;
            }
            n_functions += 1;

            let stmts = parsed.statements_in_function(&func_node);
            // DFG_CENSUS_DUMP_STMTS=<function name> prints the CFG line universe for
            // that function, so an `n_cfg_nodes` value can be audited against source.
            if std::env::var("DFG_CENSUS_DUMP_STMTS").as_deref() == Ok(func_name.as_str()) {
                eprintln!("STMTS {path}:{start}-{end} {func_name}: {stmts:?}");
            }
            let n_cfg_nodes = stmts.len();
            let n_cfg_edges: usize = stmts
                .iter()
                .map(|(line, _)| from_lines.get(line).copied().unwrap_or(0))
                .sum();

            let c = counts.get(&key).cloned().unwrap_or_default();
            // The raw start-line metric is retained for comparison. The v6.3 gate uses
            // statement-span containment and excludes edges that touch nested callable
            // bodies; those have unmodeled execution timing and are reported separately.
            let stmt_set: BTreeSet<usize> = stmts.iter().map(|(l, _)| *l).collect();
            let admissibility = edge_admissibility(
                parsed,
                func_node,
                start,
                &stmt_set,
                &file_line_starts,
                &c.edge_endpoints,
            );
            let n_lines = end.saturating_sub(start) + 1;
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                csv_escape(label),
                lang,
                csv_escape(path),
                csv_escape(&func_name),
                start,
                end,
                n_lines,
                c.defs,
                c.defs_raw,
                c.uses,
                c.uses_raw,
                n_cfg_nodes,
                n_cfg_edges,
                c.dfg_edges,
                admissibility.cfg_ok,
                admissibility.span_ok,
                admissibility.distinct_stmt,
                admissibility.nested_callable,
                admissibility.unmapped_use,
                admissibility.unmapped_def,
                admissibility.same_stmt,
            );
        }
    }

    let csv_path = out_dir.join(format!("{label}.csv"));
    let mut f = std::fs::File::create(&csv_path)?;
    f.write_all(csv.as_bytes())?;

    // Any DFG function key with no matching AST row means the join is lossy —
    // report it rather than silently under-counting.
    let unjoined = counts.keys().filter(|k| !seen.contains(*k)).count();

    let json_path = out_dir.join(format!("{label}.summary.json"));
    let write_summary = |cpg: Option<(f64, usize, usize)>| -> anyhow::Result<()> {
        let summary = serde_json::json!({
            "repo": label,
            "root": root.display().to_string(),
            "files_parsed": n_files,
            "files_skipped": n_skipped,
            "functions_named": n_functions,
            "functions_unnamed_skipped_by_dfg": n_functions_unnamed,
            "dfg_function_keys_without_ast_row": unjoined,
            "dfg_edges_total": dfg.edges.len(),
            "cpg_nodes": cpg.map(|c| c.1),
            "cpg_edges": cpg.map(|c| c.2),
            "parse_load_secs": parse_secs,
            "dfg_build_secs": dfg_secs,
            "cpg_build_secs": cpg.map(|c| c.0),
            "census_total_secs": t_all.elapsed().as_secs_f64(),
            "threads": rayon::current_num_threads(),
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&summary)? + "\n")?;
        Ok(())
    };

    // The census data is now durable: a later CPG-build timeout cannot discard it.
    write_summary(None)?;
    eprintln!("{label}: {n_files} files, {n_functions} functions, parse {parse_secs:.2}s, dfg {dfg_secs:.2}s");

    // ---- 5. Cold full CPG build (no cache dir is passed anywhere, so this is an
    //         unconditional cold build; it rebuilds its own DFG internally). Set
    //         DFG_CENSUS_SKIP_CPG=1 to omit it on repos where it blows the budget.
    if std::env::var_os("DFG_CENSUS_SKIP_CPG").is_none() {
        let t2 = Instant::now();
        let cpg = CodePropertyGraph::build(&files);
        let cpg_secs = t2.elapsed().as_secs_f64();
        write_summary(Some((
            cpg_secs,
            cpg.graph.node_count(),
            cpg.graph.edge_count(),
        )))?;
        eprintln!("{label}: cpg {cpg_secs:.2}s");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_admissibility(
        path: &str,
        source: &str,
        language: Language,
        function_name: &str,
        edge_path_base: Option<&str>,
    ) -> (EdgeAdmissibility, usize) {
        let parsed = ParsedFile::parse(path, source, language).expect("fixture must parse");
        let func_node = parsed
            .all_functions()
            .into_iter()
            .find(|node| {
                parsed
                    .language
                    .function_name(node)
                    .is_some_and(|name| parsed.node_text(&name) == function_name)
            })
            .expect("fixture function must exist");
        let (start, _) = parsed.node_line_range(&func_node);
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed.clone());
        let dfg = DataFlowGraph::build(&files);
        let edges: Vec<EdgeEndpoints> = dfg
            .edges
            .iter()
            .filter(|edge| {
                edge.from.file == path
                    && edge.from.function == function_name
                    && edge.from.function_start_line == start
                    && edge_path_base.is_none_or(|base| edge.from.path.base == base)
            })
            .map(|edge| EdgeEndpoints {
                def_line: edge.from.line,
                def_start_byte: edge.from.start_byte,
                def_end_byte: edge.from.end_byte,
                use_line: edge.to.line,
                use_start_byte: edge.to.start_byte,
                use_end_byte: edge.to.end_byte,
            })
            .collect();
        let stmt_lines = parsed
            .statements_in_function(&func_node)
            .into_iter()
            .map(|(line, _)| line)
            .collect();
        let line_starts = source_line_starts(&parsed.source);

        (
            edge_admissibility(&parsed, func_node, start, &stmt_lines, &line_starts, &edges),
            edges.len(),
        )
    }

    #[test]
    fn continuation_line_edge_is_span_admissible_but_not_cfg_start_admissible() {
        let source = "def f(x):\n    consume(\n        x\n    )\n";
        let (admissibility, n_edges) =
            fixture_admissibility("continuation.py", source, Language::Python, "f", None);

        assert!(n_edges > 0, "fixture must build at least one DFG edge");
        assert_eq!(admissibility.cfg_ok, 0);
        assert_eq!(admissibility.span_ok, n_edges);
        assert_eq!(admissibility.distinct_stmt, n_edges);
        assert_eq!(admissibility.same_stmt, 0);
        assert_eq!(admissibility.nested_callable, 0);
    }

    #[test]
    fn rust_if_body_edge_maps_to_distinct_inner_statements() {
        let source =
            "fn f(x: i32) {\n    if x > 0 {\n        let y = x;\n        sink(y);\n    }\n}\n";
        let (admissibility, n_edges) =
            fixture_admissibility("distinct.rs", source, Language::Rust, "f", Some("y"));

        assert!(n_edges > 0, "fixture must build a y def-to-use edge");
        assert_eq!(admissibility.span_ok, n_edges);
        assert_eq!(admissibility.distinct_stmt, n_edges);
        assert_eq!(admissibility.same_stmt, 0);
        assert_eq!(admissibility.nested_callable, 0);
    }

    #[test]
    fn same_statement_def_to_use_is_never_distinct() {
        let source = "fn f() {\n    let y =\n        y;\n}\n";
        let (admissibility, n_edges) =
            fixture_admissibility("same-statement.rs", source, Language::Rust, "f", Some("y"));

        assert!(n_edges > 0, "fixture must build a y def-to-use edge");
        assert_eq!(admissibility.span_ok, n_edges);
        assert_eq!(admissibility.distinct_stmt, 0);
        assert_eq!(admissibility.same_stmt, n_edges);
        assert_eq!(admissibility.nested_callable, 0);
    }

    #[test]
    fn with_header_use_and_definition_are_unmapped_residuals() {
        let source = "def f(y):\n    with (x := y):\n        consume(x)\n";
        let (use_admissibility, n_y_edges) =
            fixture_admissibility("with-header.py", source, Language::Python, "f", Some("y"));
        let (def_admissibility, n_x_edges) =
            fixture_admissibility("with-header.py", source, Language::Python, "f", Some("x"));

        assert!(n_y_edges > 0, "fixture must use y on the with header");
        assert_eq!(use_admissibility.unmapped_use, n_y_edges);
        assert!(n_x_edges > 0, "fixture must define x on the with header");
        assert_eq!(def_admissibility.unmapped_def, n_x_edges);
    }

    #[test]
    fn multiline_parameter_definition_is_normalized_to_entry() {
        let source = "fn f(\n    x: i32,\n) {\n    sink(x);\n}\n";
        let (admissibility, n_edges) = fixture_admissibility(
            "multiline-signature.rs",
            source,
            Language::Rust,
            "f",
            Some("x"),
        );

        assert!(n_edges > 0, "fixture must build an x def-to-use edge");
        assert_eq!(admissibility.cfg_ok, n_edges);
        assert_eq!(admissibility.distinct_stmt, n_edges);
        assert_eq!(admissibility.unmapped_def, 0);
    }

    #[test]
    fn lambda_body_edge_is_counted_as_nested_callable() {
        let source = "def f(x):\n    apply = lambda y: y + x\n    return apply(x)\n";
        let (admissibility, n_edges) =
            fixture_admissibility("lambda.py", source, Language::Python, "f", None);

        assert!(n_edges > 0, "fixture must build at least one DFG edge");
        assert!(
            admissibility.nested_callable > 0,
            "an edge touching the lambda body must be excluded"
        );
        assert!(admissibility.nested_callable < n_edges);
    }

    #[test]
    fn same_line_arrow_does_not_exclude_unrelated_outer_endpoint() {
        let source = "function f(x, z) {\n  invoke(y => y + x); use(z);\n}\n";
        let (admissibility, n_edges) =
            fixture_admissibility("same-line.js", source, Language::JavaScript, "f", None);

        assert!(
            n_edges > 1,
            "fixture must contain nested and outer DFG edges"
        );
        assert!(admissibility.nested_callable > 0);
        assert!(admissibility.nested_callable < n_edges);
    }
}
