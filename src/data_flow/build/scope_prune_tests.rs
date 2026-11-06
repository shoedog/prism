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
    let index = "package p\nfunc f() {\n\tx := make([]int, 1)\n\tx[0] = 7\n\tif x := x; true {\n\t\tuse(x)\n\t}\n}\n";
    check("index", index, &[("x", 4, 6, true)]);
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
