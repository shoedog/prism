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
//! n_defs_raw,n_uses,n_uses_raw,n_cfg_nodes,n_cfg_edges,n_dfg_edges,n_dfg_edges_cfg_ok
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
//! per-function CFG is not directly obtainable. Statement collection descends into
//! nested callables, so an outer function's count includes edges lexically inside its
//! nested closures.
//!
//! `n_dfg_edges_cfg_ok` counts DFG edges RD could label at all: use line in the CFG line
//! universe, def line in it or at the function start (the synthetic ENTRY of §4.2 step 4).
//!
//! This binary reads only public API (`prism::ast`, `prism::cfg`, `prism::cpg`,
//! `prism::data_flow`, `prism::repo_loader`) and changes nothing under `src/`.

use prism::ast::ParsedFile;
use prism::cfg;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{DataFlowGraph, VarAccessKind};
use prism::languages::Language;
use prism::repo_loader;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
    /// (def_line, use_line) of every DFG edge in this function, so the census can
    /// count how many edges have both endpoints inside the CFG's line universe.
    edge_lines: Vec<(usize, usize)>,
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
        e.edge_lines.push((edge.from.line, edge.to.line));
    }

    // ---- 4. Walk the AST the same way `DataFlowGraph::build` does, so the CSV
    //         has a row per function the DFG iterates, joined to CFG shape.
    let mut csv = String::new();
    csv.push_str(
        "repo,lang,file,function,start_line,end_line,n_lines,n_defs,n_defs_raw,n_uses,n_uses_raw,n_cfg_nodes,n_cfg_edges,n_dfg_edges,n_dfg_edges_cfg_ok\n",
    );
    let mut n_functions = 0usize;
    let mut n_functions_unnamed = 0usize;
    let mut seen: BTreeSet<FnKey> = BTreeSet::new();

    for (path, parsed) in &files {
        let lang = lang_name(parsed.language);
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
            // Spec §4.2 step 33: an edge is labelable at all only when its use line is
            // in the CFG's line universe and its def line is in that universe or is the
            // synthetic ENTRY at the function start (step 4 — parameter Defs are pinned
            // to the signature line, which `collect_statements` never yields). Every
            // other edge is `NameOnly(CfgIncomplete)` before any cap is consulted.
            let stmt_set: BTreeSet<usize> = stmts.iter().map(|(l, _)| *l).collect();
            let n_dfg_edges_cfg_ok = c
                .edge_lines
                .iter()
                .filter(|(d, u)| stmt_set.contains(u) && (stmt_set.contains(d) || *d == start))
                .count();
            let n_lines = end.saturating_sub(start) + 1;
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
                n_dfg_edges_cfg_ok,
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
