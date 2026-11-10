//! Rule O (#630 slice 4, Go): a def textually after the use, with no loop around
//! both, is dropped for a local declaration of the owner. Every row is also
//! checked against the frozen reference so a vacuous row cannot pass.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

fn parse(src: &str) -> ParsedFile {
    ParsedFile::parse("t.go", src, Language::Go).unwrap()
}

fn has(refs: &FileRefs, path: &str, def_line: usize, use_line: usize) -> bool {
    refs.edges.iter().any(|e| {
        e.from.path.to_string() == path && e.from.line == def_line && e.to.line == use_line
    })
}

/// (path, def line, use line, keep). Lines are 1-indexed in `src`.
fn check(name: &str, src: &str, rows: &[(&str, usize, usize, bool)]) -> usize {
    let parsed = parse(src);
    let before = Producer::file_refs_frozen(&"t.go".to_string(), &parsed);
    let after = Producer::file_refs(&"t.go".to_string(), &parsed);
    let mut failures = Vec::new();
    for (path, d, u, keep) in rows {
        if !has(&before, path, *d, *u) {
            failures.push(format!("{name}: inadmissible baseline row {path} {d}->{u}"));
        }
        if has(&after, path, *d, *u) != *keep {
            failures.push(format!("{name}: {path} {d}->{u} expected keep={keep}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    after.rd_function_stats.dfg_pruned_order
}

fn dropped(name: &str, src: &str, rows: &[(&str, usize, usize)]) {
    let rows: Vec<_> = rows.iter().map(|(p, d, u)| (*p, *d, *u, false)).collect();
    assert!(check(name, src, &rows) > 0, "{name}: dfg_pruned_order is 0");
}

fn kept(name: &str, src: &str, rows: &[(&str, usize, usize)]) {
    let rows: Vec<_> = rows.iter().map(|(p, d, u)| (*p, *d, *u, true)).collect();
    assert_eq!(check(name, src, &rows), 0, "{name}: order counted");
}

fn head(params: &str, body: &str) -> String {
    format!("package p\nfunc f({params}) {{\n{body}}}\n")
}

#[test]
fn def_after_use_no_loop() {
    // The parameter is the only def before the use, so rule D has no d* to
    // separate the later write: only rule O can drop 4->3.
    let src = head("x int", "\tuse(x)\n\tx = 2\n\tuse2()\n");
    let rows = [("x", 4, 3, false), ("x", 2, 3, true)];
    assert!(check("plain", &src, &rows) > 0);
}

#[test]
fn def_after_use_conditional_branches() {
    let src = head(
        "c bool, x int",
        "\tif c {\n\t\tuse(x)\n\t} else {\n\t\tx = 2\n\t}\n",
    );
    dropped("branch", &src, &[("x", 6, 4)]);
}

#[test]
fn def_after_use_in_loops_kept() {
    kept(
        "for",
        &head("x int", "\tfor {\n\t\tuse(x)\n\t\tx = 2\n\t}\n"),
        &[("x", 5, 4)],
    );
    kept(
        "range",
        &head(
            "x int, s []int",
            "\tfor range s {\n\t\tuse(x)\n\t\tx = 2\n\t}\n",
        ),
        &[("x", 5, 4)],
    );
    kept(
        "threeclause",
        &head(
            "x int",
            "\tfor i := 0; i < 3; i++ {\n\t\tuse(x)\n\t\tx = 2\n\t}\n",
        ),
        &[("x", 5, 4)],
    );
    kept(
        "header",
        &head("x int", "\tfor i := 0; i < x; i++ {\n\t\tx = 2\n\t}\n"),
        &[("x", 4, 3)],
    );
    // The use precedes a loop that holds the def: no loop holds both.
    dropped(
        "after",
        &head("x int", "\tuse(x)\n\tfor {\n\t\tx = 2\n\t\tbreak\n\t}\n"),
        &[("x", 5, 3)],
    );
}

#[test]
fn goto_and_label_owner_kept() {
    kept(
        "goto",
        &head("x int", "L:\n\tuse(x)\n\tx = 2\n\tgoto L\n"),
        &[("x", 5, 4)],
    );
    kept(
        "label_after",
        &head("x int", "\tuse(x)\n\tx = 2\nL:\n\tuse2()\n"),
        &[("x", 4, 3)],
    );
}

#[test]
fn non_local_names_kept() {
    kept(
        "package_var",
        "package p\nvar g int\nfunc f() {\n\tuse(g)\n\tg = 1\n}\n",
        &[("g", 5, 4)],
    );
    kept(
        "captured",
        &head("x int", "\tfunc() {\n\t\tuse(x)\n\t\tx = 2\n\t}()\n"),
        &[("x", 5, 4)],
    );
}

#[test]
fn storage_kept() {
    kept(
        "address",
        &head("x int", "\tp := &x\n\tuse(x)\n\tx = 2\n\tuse(p)\n"),
        &[("x", 5, 4)],
    );
    kept(
        "slice",
        &head(
            "x [3]int",
            "\ts := x[:]\n\tuse(x)\n\tx = [3]int{}\n\tuse(s)\n",
        ),
        &[("x", 5, 4)],
    );
    // A field def (`p.f = 2`) has no reference edge to a use of `p.f` at all, so
    // there is no row to pin; `keep_pair` keeps every path with fields anyway.
    kept(
        "deref",
        &head("x *int", "\tuse(x)\n\t*x = 2\n"),
        &[("x", 4, 3)],
    );
}

#[test]
fn nested_callables_kept() {
    kept(
        "def_in_closure",
        &head(
            "x int",
            "\tuse(x)\n\tg := func() {\n\t\tx = 2\n\t}\n\tg()\n",
        ),
        &[("x", 5, 3)],
    );
    kept(
        "use_in_closure",
        &head(
            "x int",
            "\tg := func() {\n\t\tuse(x)\n\t}\n\tx = 2\n\tg()\n",
        ),
        &[("x", 6, 4)],
    );
}

#[test]
fn named_result_written_after_use_is_dropped_only_for_direct_reads() {
    // #694 slice B: a named result has a producer def and is an ordinary local for O. The later
    // write cannot reach the earlier direct read; a read in a deferred literal is nested, so O
    // keeps it (the write reaches it at exit).
    dropped(
        "result",
        "package p\nfunc f() (r int) {\n\tuse(r)\n\tr = 1\n\treturn\n}\n",
        &[("r", 4, 3)],
    );
    kept(
        "result-deferred",
        "package p\nfunc f() (r int) {\n\tdefer func() {\n\t\tuse(r)\n\t}()\n\tr = 1\n\treturn\n}\n",
        &[("r", 6, 4)],
    );
}

#[test]
fn guard_rows_that_fail_without_their_guard() {
    // Outermost loop: the def's innermost loop does not contain the use, but an
    // enclosing loop contains both (backward flow through the outer loop).
    let outer = head(
        "x int",
        "\tfor {\n\t\tuse(x)\n\t\tfor {\n\t\t\tx = 2\n\t\t\tbreak\n\t\t}\n\t}\n",
    );
    kept("outermost-loop", &outer, &[("x", 6, 4)]);
    // Every occurrence on the use line: one is inside a deferred closure (runs at
    // exit, after the later write).
    let mixed = head("x int", "\tdefer func() { use(x) }(); use(x)\n\tx = 2\n");
    kept("mixed-use-line", &mixed, &[("x", 4, 3)]);
    // Use-line storage access: a deferred pointer-receiver call reads `b` at exit.
    let recv = "package p\ntype T struct{ n int }\nfunc (t *T) Close() { println(t.n) }\nfunc f(b T) {\n\tdefer b.Close()\n\tb = T{n: 1}\n}\n";
    kept("receiver-storage", recv, &[("b", 6, 5)]);
}

#[test]
fn order_prune_cache_and_incremental_custody() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{self, CacheResult};
    use std::collections::{BTreeMap, BTreeSet};
    let active =
        |n: &str| format!("package p\nfunc {n}(x int) {{\n\tuse(x)\n\tx = 2\n\tuse2()\n}}\n");
    let mut sources = BTreeMap::from([
        ("a.go".to_owned(), active("a")),
        ("b.go".to_owned(), active("b")),
    ]);
    let parse = |sources: &BTreeMap<String, String>| {
        sources
            .iter()
            .map(|(f, s)| (f.clone(), ParsedFile::parse(f, s, Language::Go).unwrap()))
            .collect()
    };
    let cold = CodePropertyGraph::build(&parse(&sources));
    assert!(cold.dfg.rd_function_stats()["a.go"].dfg_pruned_order > 0);
    assert!(cold.dfg.rd_function_stats()["b.go"].dfg_pruned_order > 0);
    assert!(cold.dfg_label_stats.dfg_pruned_order > 0);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(cold.dfg_label_stats, hit.dfg_label_stats);
    assert_eq!(cold.dfg.rd_function_stats(), hit.dfg.rd_function_stats());
    sources.insert(
        "a.go".into(),
        "package p\nfunc a(x int) int {\nreturn x\n}\n".into(),
    );
    let edited = parse(&sources);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["a.go".into()]));
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited);
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    let stats = incremental.dfg.rd_function_stats();
    assert_eq!(
        stats["a.go"].dfg_pruned_order, 0,
        "edited file's contribution is gone"
    );
    assert!(
        stats["b.go"].dfg_pruned_order > 0,
        "untouched file keeps its contribution"
    );
}
