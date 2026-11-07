//! Rule S (#630 slice 3, Go): a pair between two different explicit declarations
//! of one name is dropped, with the storage-alias rows kept. Every row is also
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
fn check(name: &str, src: &str, rows: &[(&str, usize, usize, bool)]) {
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
}

/// Rule S must have removed something here (not rule U or D).
fn scope_counted(name: &str, src: &str) {
    let after = Producer::file_refs(&"t.go".to_string(), &parse(src));
    assert!(
        after.rd_function_stats.dfg_pruned_scope > 0,
        "{name}: dfg_pruned_scope is 0"
    );
}

#[test]
fn case_scoped_var_splits_per_case() {
    // Case 2 declares through an `if` header, so no dominating block write
    // (rule D) already separates it from case 1's `v`.
    let src = "package p\nfunc f(n int) {\n\tswitch n {\n\tcase 1:\n\t\tvar v int\n\t\tv = n\n\t\tuse(v)\n\tcase 2:\n\t\tif v := g(); v > 0 {\n\t\t\tuse(v)\n\t\t}\n\t}\n}\n";
    let rows = [("v", 6, 7, true), ("v", 6, 9, false), ("v", 6, 10, false)];
    check("case", src, &rows);
    scope_counted("case", src);
}

#[test]
fn short_redeclaration_reuses_binding() {
    let src =
        "package p\nfunc f() {\n\ta, err := f1()\n\tuse(err)\n\tb, err := f2()\n\tuse(a, b)\n}\n";
    let parsed = parse(src);
    let after = Producer::file_refs(&"t.go".to_string(), &parsed);
    assert!(has(&after, "err", 3, 4));
    assert_eq!(after.rd_function_stats.dfg_pruned_scope, 0);
}

#[test]
fn inner_block_shadow_after_use_keeps_outer_edge() {
    let src = "package p\nfunc f() {\n\tx := 1\n\t{\n\t\tx := 2\n\t\tuse(x)\n\t}\n\tuse(x)\n}\n";
    check("shadow", src, &[("x", 3, 8, true)]);
}

#[test]
fn zero_value_var_in_new_block_splits() {
    let src = "package p\nfunc f(c bool) {\n\tx := 1\n\tif c {\n\t\tvar x int\n\t\tuse(x)\n\t}\n\tuse(x)\n}\n";
    // The existing shadow walk already drops 3->6 (baseline has no such edge).
    check("varzero", src, &[("x", 3, 8, true)]);
}

#[test]
fn select_case_scopes_split() {
    let src = "package p\nfunc f(ch, ch2 chan int) {\n\tselect {\n\tcase <-ch:\n\t\tm := 1\n\t\tuse(m)\n\tcase <-ch2:\n\t\tm := 2\n\t\tuse(m)\n\t}\n}\n";
    check("select", src, &[("m", 5, 6, true), ("m", 5, 9, false)]);
    let hdr = "package p\nfunc f(ch, ch2 chan int) {\n\tselect {\n\tcase <-ch:\n\t\tm := 1\n\t\tm = 3\n\t\tuse(m)\n\tcase <-ch2:\n\t\tif m := g(); m > 0 {\n\t\t\tuse(m)\n\t\t}\n\t}\n}\n";
    check("selecthdr", hdr, &[("m", 6, 7, true), ("m", 6, 10, false)]);
    scope_counted("selecthdr", hdr);
}

#[test]
fn header_short_var_splits() {
    let src = "package p\nfunc f() {\n\tx := 1\n\tx = 2\n\tif x := g(); x > 0 {\n\t\tuse(x)\n\t}\n\tuse(x)\n}\n";
    check("header", src, &[("x", 4, 6, false), ("x", 4, 8, true)]);
    scope_counted("header", src);
}

#[test]
fn closure_redeclaration_splits_and_capture_keeps() {
    let src = "package p\nfunc f() {\n\tx := 1\n\tx = 3\n\tg := func() {\n\t\tx := 2\n\t\tuse(x)\n\t}\n\th := func() {\n\t\tuse(x)\n\t}\n\tuse(g, h)\n}\n";
    // 4->7 is already absent in the baseline (shadow walk); the capture must stay.
    check("closure", src, &[("x", 4, 10, true)]);
}

#[test]
fn scalar_control_drops_but_storage_aliases_keep() {
    // Scalar def, plain `use(x)` through the new declaration: drop.
    let scalar =
        "package p\nfunc f() {\n\tx := 1\n\tx = 2\n\tif x := x; true {\n\t\tuse(x)\n\t}\n}\n";
    check("scalar", scalar, &[("x", 4, 6, false)]);
    scope_counted("scalar", scalar);
    // `*x = 7` normalizes to path x; the new x copies the pointer.
    let deref = "package p\nfunc f() {\n\tx := new(int)\n\t*x = 7\n\tif x := x; true {\n\t\tuse(x)\n\t}\n}\n";
    check("deref", deref, &[("x", 4, 6, true)]);
    // `x[0] = 7` is not a def of `x` in the producer at all (no baseline row);
    // the use-side `x[0]` storage access is covered by the `idx` row below.
    // Same-line coalescing: the first span is a plain write, a later one is not.
    let same = "package p\nfunc f() {\n\tx := new(int)\n\tx = new(int); *x = 7\n\tif x := x; true {\n\t\tuse(x)\n\t}\n}\n";
    check("sameline", same, &[("x", 4, 6, true)]);
    let tuple = "package p\nfunc f() {\n\tx := new(int)\n\tx, *x = x, 7\n\tif x := x; true {\n\t\tuse(x)\n\t}\n}\n";
    check("tuple", tuple, &[("x", 4, 6, true)]);
    // Field def: path has fields.
    let field = "package p\nfunc f() {\n\tp := &T{}\n\tp.f = 1\n\tif p := p; true {\n\t\tuse(p.f)\n\t}\n}\n";
    check("field", field, &[("p.f", 4, 6, true)]);
    // Use-side storage access through the new name.
    for (n, body) in [
        ("sel", "use(x.f)"),
        ("idx", "use(x[0])"),
        ("star", "use(*x)"),
        ("method", "x.M()"),
    ] {
        let src = format!(
            "package p\nfunc f() {{\n\tx := 1\n\tx = 2\n\tif x := x; true {{\n\t\t{body}\n\t}}\n}}\n"
        );
        check(n, &src, &[("x", 4, 6, true)]);
    }
}

#[test]
fn two_occurrences_in_different_scopes_keep_if_any_keeps() {
    let src = "package p\nfunc f() {\n\tx := 1\n\tx = 2\n\tuse(x, func() int { x := 2; return x }())\n}\n";
    check("twoocc", src, &[("x", 4, 5, true)]);
}

#[test]
fn edges_after_are_a_subset_of_edges_before() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dfg_prune/go");
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "go") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        let parsed = parse(&src);
        let key = |r: &FileRefs| {
            let mut v: Vec<_> = r
                .edges
                .iter()
                .map(|e| (e.from.clone(), e.to.clone()))
                .collect();
            v.retain(|(f, t)| !(t.path != f.path && t.path.is_prefix_of(&f.path)));
            v.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
            v
        };
        let before = key(&Producer::file_refs_frozen(&"t.go".to_string(), &parsed));
        for e in key(&Producer::file_refs(&"t.go".to_string(), &parsed)) {
            assert!(before.contains(&e), "{path:?}: added edge {e:?}");
        }
    }
}

/// The lazy gate admits every name the binding inventory can declare twice.
#[test]
fn multiply_declared_prescan_is_a_superset_of_the_inventory() {
    let src = "package p\n\
func f(a int, b int, c any) {\n\
\tvar once int\n\
\tfor i := range a {\n\
\t\t_ = i\n\
\t}\n\
\tswitch v := c.(type) {\n\
\tcase int:\n\
\t\t_ = v\n\
\tcase string:\n\
\t\t_ = v\n\
\t}\n\
\tif x := b; x > 0 {\n\
\t\tx := 1\n\
\t\t_ = x\n\
\t}\n\
\t_ = once\n\
}\n";
    let parsed = parse(src);
    let owner = parsed
        .tree
        .root_node()
        .named_child(1)
        .expect("function_declaration");
    let names: Vec<&str> = super::scope_prune::multiply_declared(&parsed, owner)
        .into_iter()
        .collect();
    assert_eq!(names, vec!["v", "x"]);
}

#[test]
fn multiline_indirect_write_keeps() {
    // The `*`/newline/`x` lvalue is recorded at the line holding `*`; the first
    // plain `x` on that line is the coalesced span. The path's other lvalue
    // sits on the next line, so the line scan alone cannot see it.
    let multiline = "package p\nfunc f() {\n\tx := new(int)\n\tx = new(int); *\n\tx = 7\n\tif x := x; true {\n\t\tg(x)\n\t}\n}\nfunc g(p *int) { println(*p) }\n";
    check("multiline", multiline, &[("x", 4, 7, true)]);
    // Control: the same shape on one line was already kept.
    let one = "package p\nfunc f() {\n\tx := new(int)\n\tx = new(int); *x = 7\n\tif x := x; true {\n\t\tg(x)\n\t}\n}\nfunc g(p *int) { println(*p) }\n";
    check("oneline", one, &[("x", 4, 6, true)]);
}

#[test]
fn address_taken_names_keep() {
    // `x := &x` shares storage across the two bindings.
    let shared = "package p\ntype P *int\nfunc f() {\n\tx := 0\n\tx = 7\n\tif x := &x; true {\n\t\tprintln(*P(x))\n\t}\n}\n";
    check("addr", shared, &[("x", 5, 7, true)]);
    let paren = "package p\ntype P *int\nfunc f() {\n\tx := 0\n\tx = 7\n\tif x := &(x); true {\n\t\tprintln(*P(x))\n\t}\n}\n";
    check("addrparen", paren, &[("x", 5, 7, true)]);
    for (n, take) in [("field", "q := &x.f"), ("index", "q := &x[0]")] {
        let src = format!(
            "package p\nfunc f() {{\n\tx := 0\n\tx = 7\n\t{take}\n\tif x := g(); true {{\n\t\tuse(x)\n\t}}\n\tuse(q)\n}}\n"
        );
        check(n, &src, &[("x", 4, 7, true)]);
    }
    // Slicing an array shares its storage like `&x`.
    let sl = "package p\nfunc f() {\n\tx := [1]int{}\n\ts := x[:]\n\tx = [1]int{7}\n\tif x := s; true {\n\t\tg(x)\n\t}\n}\n";
    check("slice", sl, &[("x", 5, 7, true)]);
    // Control: another name's address does not stop the drop.
    let other = "package p\nfunc f() {\n\tz := 1\n\tq := &z\n\tx := 0\n\tx = 7\n\tif x := x; true {\n\t\tuse(x, q)\n\t}\n}\n";
    check("otheraddr", other, &[("x", 6, 8, false)]);
    scope_counted("otheraddr", other);
}

#[test]
fn declarations_with_a_def_drop_the_outer_pair() {
    // #694 slice A: the producer now emits a def for a type-switch alias and a range/receive
    // `:=` target, so the value reaches the body through it and S drops the outer pair
    // (the old categorical keep, when no def existed, is lifted).
    let ts = "package p\nfunc f(c any) {\n\tv := c\n\tswitch v := v.(type) {\n\tcase int:\n\t\tuse(v)\n\t}\n}\n";
    check(
        "typeswitch",
        ts,
        &[("v", 3, 4, true), ("v", 3, 6, false), ("v", 4, 6, true)],
    );
    scope_counted("typeswitch", ts);
    let rg =
        "package p\nfunc f(c []int) {\n\tv := c\n\tfor _, v := range v {\n\t\tuse(v)\n\t}\n}\n";
    check("range", rg, &[("v", 3, 4, true), ("v", 4, 5, true)]);
    let paren = "package p\nfunc f() {\n\tv := any(7)\n\tc := (v)\n\tswitch v := c.(type) {\n\tcase int:\n\t\tprintln(v)\n\t}\n}\n";
    check(
        "typeswitch-paren-copy",
        paren,
        &[("v", 3, 7, false), ("v", 5, 7, true)],
    );
    let rc = "package p\nfunc f(c chan int) {\n\tv := c\n\tselect {\n\tcase v := <-v:\n\t\tuse(v)\n\t}\n}\n";
    check("receive", rc, &[("v", 3, 5, true), ("v", 5, 6, true)]);
}

#[test]
fn declarations_still_without_a_def_keep_the_outer_pair() {
    // Function-literal formals and named results keep the categorical keep (#694 slice B).
    let lit = "package p\nfunc f() {\n\tv := 7\n\tfunc(v int) {\n\t\tprintln(v)\n\t}(v)\n}\n";
    check("literal-formal", lit, &[("v", 3, 5, true)]);
    let res = "package p\nfunc f() (v int) {\n\tdefer func() { println(v) }()\n\t{\n\t\tv := 7\n\t\tc := (v)\n\t\treturn c\n\t}\n}\n";
    check("named-result", res, &[("v", 5, 3, true)]);
    let var2 =
        "package p\nfunc f() {\n\tv := 7\n\t{\n\t\tvar _, v = 0, v\n\t\tprintln(v)\n\t}\n}\n";
    check(
        "var-second-name",
        var2,
        &[("v", 3, 6, false), ("v", 5, 6, true)],
    );
}

#[test]
fn named_result_reused_by_short_var_keeps() {
    let src = "package p\nfunc f() (err error) {\n\tdefer func() {\n\t\tuse(err)\n\t}()\n\tx, err := g()\n\tuse(x)\n\treturn\n}\n";
    check("namedresult", src, &[("err", 6, 4, true)]);
    let after = Producer::file_refs(&"t.go".to_string(), &parse(src));
    assert_eq!(after.rd_function_stats.dfg_pruned_scope, 0);
    let two = "package p\nfunc f() {\n\ta, err := f1()\n\tb, err := f2()\n\tuse(a, b, err)\n}\n";
    check("tworeuse", two, &[("err", 4, 5, true)]);
    let after = Producer::file_refs(&"t.go".to_string(), &parse(two));
    assert_eq!(after.rd_function_stats.dfg_pruned_scope, 0);
}

/// Over every Go fixture function, each name the inventory declares twice is in
/// the pre-scan (superset), so the lazy gate never hides a prunable name.
#[test]
fn multiply_declared_prescan_covers_inventory_on_all_go_fixtures() {
    use crate::cpg::binding_facts::BindingFacts;
    fn go_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                go_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "go") {
                out.push(path);
            }
        }
    }
    fn owners<'a>(node: tree_sitter::Node<'a>, out: &mut Vec<tree_sitter::Node<'a>>) {
        if matches!(node.kind(), "function_declaration" | "method_declaration") {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            owners(child, out);
        }
    }
    let mut files = Vec::new();
    go_files(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        &mut files,
    );
    files.sort();
    let (mut checked, mut multi_names) = (0usize, 0usize);
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let Ok(parsed) = ParsedFile::parse("t.go", &src, Language::Go) else {
            continue;
        };
        let mut found = Vec::new();
        owners(parsed.tree.root_node(), &mut found);
        for owner in found {
            let facts = BindingFacts::new(&parsed, &owner, &[], std::iter::empty());
            let scan = super::scope_prune::multiply_declared(&parsed, owner);
            for (name, count) in facts.declaration_counts() {
                checked += 1;
                if count >= 2 {
                    multi_names += 1;
                    assert!(
                        scan.contains(name.as_str()),
                        "{path:?}: `{name}` ({count}) missing from pre-scan"
                    );
                }
            }
        }
    }
    assert!(checked > 100 && multi_names > 0, "{checked} {multi_names}");
}

const S_ACTIVE: &str = "func f() {\nx := 1\nx = 2\nif x := g(); x > 0 {\nuse(x)\n}\nuse(x)\n";

#[test]
fn scope_prune_cache_and_incremental_custody() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{self, CacheResult};
    use std::collections::{BTreeMap, BTreeSet};
    let active = |n: &str| {
        format!(
            "package p\n{}}}\n",
            S_ACTIVE.replace("func f", &format!("func {n}"))
        )
    };
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
    assert!(cold.dfg.rd_function_stats()["a.go"].dfg_pruned_scope > 0);
    assert!(cold.dfg_label_stats.dfg_pruned_scope > 0);
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
    assert_eq!(
        incremental.dfg.rd_function_stats()["a.go"].dfg_pruned_scope,
        0
    );
    assert!(incremental.dfg.rd_function_stats()["b.go"].dfg_pruned_scope > 0);
}

#[test]
fn scope_prune_cap_counts_survivors_and_replays() {
    let source = format!("package p\n{S_ACTIVE}{}}}\n", "g()\n".repeat(4100));
    let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
    let file = parsed.path.clone();
    let before = Producer::file_refs_frozen_with_cap(&file, &parsed, None);
    let single = Producer::file_refs_with_cap(&file, &parsed, None);
    let s = &single.rd_function_stats;
    assert_eq!(s.functions_over_cap, 1);
    assert!(s.dfg_pruned_scope > 0, "{s:?}");
    let count = single.edges.len();
    assert!(count > 0);
    assert_eq!(
        before.edges.len() - count,
        s.dfg_pruned_write_only + s.dfg_pruned_scope + s.dfg_pruned_dominated + s.dfg_pruned_order
    );
    for cap in [count, count + 1] {
        let replay = Producer::file_refs_with_cap(&file, &parsed, Some(cap));
        assert_eq!(replay.edges, single.edges);
        assert_eq!(replay.labels, single.labels);
        assert_eq!(replay.rd_function_stats, single.rd_function_stats);
    }
    let capped = Producer::file_refs_with_cap(&file, &parsed, Some(count - 1));
    assert!(capped.edges.is_empty());
    assert_eq!(
        capped.rd_function_stats.dfg_pruned_scope,
        s.dfg_pruned_scope
    );
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
    assert_eq!(capped.rd_function_stats.fallback_edges_capped, count);
}

#[test]
fn split_var_spec_keeps_the_outer_pair_and_one_line_drops_it() {
    // (A label before the use switches rule D off, so only S can drop.) The multiline spec's
    // right-hand side reads the outer `a`/`b` on another line than the names: those outer defs
    // are the only path into the new ones, so the pair stays (#694a fold 1).
    let split = "package p\nfunc f(n int) {\n\ta, b := 1, 2\n\tswitch n {\n\tcase 2:\n\t\tvar a, b = b,\n\t\t\ta\n\tL:\n\t\tuse(a, b)\n\t}\n}\n";
    check("split-var", split, &[("a", 3, 9, true), ("b", 3, 9, true)]);
    let one = "package p\nfunc f(n int) {\n\ta, b := 1, 2\n\tswitch n {\n\tcase 2:\n\t\tvar a, b = b, a\n\tL:\n\t\tuse(a, b)\n\t}\n}\n";
    check("one-var", one, &[("a", 3, 8, false), ("b", 3, 8, false)]);
    scope_counted("one-var", one);
}
