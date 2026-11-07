//! #694 slice B: producer defs for function-literal formals and named results, and rule S/O
//! behaviour for them. Each row asserts reachability over the emitted edges: `reach` follows
//! `from -> to` edges by `(name, line)` node, so a row says whether a source def's value can
//! arrive at the final use through edges prism has, not only whether one pair is present.
//! Fixtures are static Go text; nothing is executed. Lines are 1-indexed in the fixture.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeSet, VecDeque};

fn refs(src: &str) -> FileRefs {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "fixture must parse");
    Producer::file_refs(&"t.go".to_string(), &parsed)
}

type Node = (String, usize);

fn reach(r: &FileRefs, from: (&str, usize), to: (&str, usize)) -> bool {
    let start: Node = (from.0.to_owned(), from.1);
    let goal: Node = (to.0.to_owned(), to.1);
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for e in &r.edges {
            if (e.from.path.to_string(), e.from.line) == node {
                let next = (e.to.path.to_string(), e.to.line);
                if next == goal {
                    return true;
                }
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
    }
    false
}

fn direct(r: &FileRefs, name: &str, d: usize, u: usize) -> bool {
    r.edges
        .iter()
        .any(|e| e.from.path.to_string() == name && e.from.line == d && e.to.line == u)
}

/// `(name, def line, use line, want reach)`.
fn check(label: &str, src: &str, rows: &[(&str, usize, usize, bool)]) {
    let r = refs(src);
    let bad: Vec<String> = rows
        .iter()
        .filter(|(n, d, u, want)| reach(&r, (n, *d), (n, *u)) != *want)
        .map(|(n, d, u, want)| format!("{label}: {n} {d}->{u} expected reach={want}"))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn has_def(src: &str, name: &str, line: usize) -> bool {
    refs(src)
        .defs
        .values()
        .flatten()
        .any(|d| d.path.to_string() == name && d.line == line)
}

// ---- producer defs ---------------------------------------------------------------------

#[test]
fn literal_formals_and_named_results_get_defs() {
    let lit = "package p\nfunc f(y int) {\n\tfunc(v int, w ...int) {\n\t\tuse(v, w)\n\t}(y)\n}\n";
    assert!(has_def(lit, "v", 3) && has_def(lit, "w", 3));
    let res = "package p\nfunc f() (r int, err error) {\n\tuse(r, err)\n\treturn\n}\n";
    assert!(has_def(res, "r", 2) && has_def(res, "err", 2));
    let lit_res =
        "package p\nfunc f() {\n\tg := func() (r int) {\n\t\tuse(r)\n\t\treturn 1\n\t}\n\tg()\n}\n";
    assert!(has_def(lit_res, "r", 3));
}

#[test]
fn unnamed_and_blank_results_and_formals_get_no_def() {
    let src = "package p\nfunc f() (int, error) {\n\tfunc(_ int, string) {}(1, \"a\")\n\treturn 0, nil\n}\n";
    let r = refs(src);
    assert!(r.defs.values().flatten().all(|d| d.path.to_string() != "_"));
}

#[test]
fn formal_defs_mint_no_alias_twin() {
    // `p := q` aliases p to q flow-insensitively; a literal formal named p is a new variable.
    let src = "package p\nfunc f(q int) {\n\tp := q\n\tuse(p)\n\tfunc(p int) {\n\t\tuse(p)\n\t}(1)\n\tuse(q)\n}\n";
    check("no-twin", src, &[("p", 5, 6, true), ("q", 5, 8, false)]);
}

// ---- immediately invoked literals ------------------------------------------------------

#[test]
fn iife_with_unrelated_argument_drops_the_outer_pair() {
    let src =
        "package p\nfunc f(x int, y int) {\n\tv := x\n\tfunc(v int) {\n\t\tuse(v)\n\t}(y)\n}\n";
    check(
        "iife-diff",
        src,
        &[
            ("v", 3, 5, false), // outer v cannot reach the body: the formal comes from y
            ("v", 4, 5, true),  // the formal's own def reaches the body
            ("y", 2, 6, true),  // argument read, as on main
        ],
    );
}

#[test]
fn iife_reading_the_name_in_its_argument_keeps_the_real_path() {
    let src = "package p\nfunc f(x int) {\n\tv := x\n\tfunc(v int) {\n\t\tuse(v)\n\t}(v)\n}\n";
    check(
        "iife-same",
        src,
        &[("v", 3, 5, true), ("v", 3, 6, true), ("v", 4, 5, true)],
    );
}

#[test]
fn go_and_defer_literals() {
    let go =
        "package p\nfunc f(x int, c int) {\n\tv := x\n\tgo func(v int) {\n\t\tuse(v)\n\t}(c)\n}\n";
    check("go-diff", go, &[("v", 3, 5, false), ("v", 4, 5, true)]);
    let go_same =
        "package p\nfunc f(x int) {\n\tv := x\n\tgo func(v int) {\n\t\tuse(v)\n\t}(v)\n}\n";
    check("go-same", go_same, &[("v", 3, 5, true), ("v", 4, 5, true)]);
    let defer =
        "package p\nfunc f(err error) {\n\tdefer func(e error) {\n\t\tuse(e)\n\t}(err)\n}\n";
    check("defer", defer, &[("e", 3, 4, true), ("err", 2, 5, true)]);
}

// ---- literals whose caller is not visible ----------------------------------------------

#[test]
fn callback_literal_keeps_the_outer_pair_for_its_formals() {
    // The callee supplies the formals; prism connects nothing into them, so main's behaviour
    // (the same-name outer def reaches the body) stays. The formal def reaches only its own body.
    let src = "package p\nfunc f(s []int) {\n\ti := 1\n\tsort.Slice(s, func(i, j int) bool {\n\t\treturn s[i] < s[j]\n\t})\n\tuse(i)\n}\n";
    check(
        "callback",
        src,
        &[
            ("i", 3, 5, true),  // kept, documented limit
            ("i", 3, 7, true),  // outer def still reaches the outer use
            ("i", 4, 5, true),  // formal def -> body
            ("i", 4, 7, false), // formal def never reaches the outer use after the literal
        ],
    );
}

#[test]
fn assigned_literal_keeps_the_outer_pair_for_its_formals() {
    let src = "package p\nfunc f(v int) {\n\tg := func(v int) {\n\t\tuse(v)\n\t}\n\tg(v)\n}\n";
    check(
        "assigned",
        src,
        &[("v", 2, 4, true), ("v", 3, 4, true), ("v", 2, 6, true)],
    );
}

// ---- named results ---------------------------------------------------------------------

#[test]
fn named_result_def_reaches_direct_uses() {
    let src =
        "package p\nfunc f() (r int, err error) {\n\tuse(r)\n\tr = 2\n\tuse(r)\n\treturn\n}\n";
    check(
        "named-direct",
        src,
        &[("r", 2, 3, true), ("r", 4, 5, true), ("r", 2, 5, true)],
    );
}

#[test]
fn deferred_literal_reads_a_named_result() {
    let src = "package p\nfunc f(c bool) (r int) {\n\tdefer func() {\n\t\tuse(r)\n\t}()\n\tr = 1\n\treturn 2\n}\n";
    check(
        "deferred-read",
        src,
        &[("r", 6, 4, true), ("r", 2, 4, true)],
    );
}

#[test]
fn return_value_reaches_a_deferred_read_of_the_result_by_name() {
    // `return err` assigns the result `err`, which the deferred literal reads: the inner def's
    // only path to the read is the name coincidence, so S keeps it (a use of a named result
    // inside a nested callable keeps).
    let src = "package p\nfunc f(c bool) (err error) {\n\tdefer func() {\n\t\tuse(err)\n\t}()\n\tif c {\n\t\terr := g()\n\t\treturn err\n\t}\n\treturn nil\n}\n";
    check("return-assigns", src, &[("err", 7, 4, true)]);
}

#[test]
fn nested_literal_result_shadows_the_owner_result() {
    let src = "package p\nfunc f() (r int) {\n\tg := func() (r int) {\n\t\tuse(r)\n\t\treturn 5\n\t}\n\tr = g()\n\tuse(r)\n\treturn\n}\n";
    check(
        "shadow-result",
        src,
        &[
            ("r", 3, 4, true),  // the literal's own result def reaches its body
            ("r", 7, 4, false), // the owner's later write is another variable
            ("r", 2, 4, false), // so is the owner's result
            ("r", 3, 8, false), // the literal's result never reaches the owner's use
            ("r", 7, 8, true),
            ("r", 2, 8, true),
        ],
    );
}

#[test]
fn literal_formal_shadows_a_named_result() {
    let src = "package p\nfunc f() (v int) {\n\tfunc(v int) {\n\t\tuse(v)\n\t}(3)\n\tuse(v)\n\treturn\n}\n";
    check(
        "formal-shadows-result",
        src,
        &[
            ("v", 3, 4, true),
            ("v", 2, 4, false),
            ("v", 2, 6, true),
            ("v", 3, 6, false),
        ],
    );
}

#[test]
fn two_literals_with_one_formal_name_do_not_connect() {
    let src = "package p\nfunc f(a int) {\n\tfunc(v int) {\n\t\tuse(v)\n\t}(a)\n\tfunc(v int) {\n\t\tuse(v)\n\t}(a)\n}\n";
    check(
        "two-literals",
        src,
        &[
            ("v", 3, 4, true),
            ("v", 6, 7, true),
            ("v", 3, 7, false),
            ("v", 6, 4, false),
        ],
    );
}

#[test]
fn a_callback_never_receives_a_sibling_literals_formal() {
    // The creation point of the second literal is outside the first's scope: even though the
    // second literal's caller is not visible, the first literal's formal def cannot reach it.
    let src = "package p\nfunc f(a int) {\n\tfunc(v int) {\n\t\tuse(v)\n\t}(a)\n\treg(func(v int) {\n\t\tuse(v)\n\t})\n}\n";
    check(
        "sibling-callback",
        src,
        &[("v", 3, 7, false), ("v", 6, 7, true)],
    );
}

// ---- rule O ----------------------------------------------------------------------------

#[test]
fn rule_o_treats_a_named_result_like_a_local() {
    // The later write cannot reach the earlier direct use; it still reaches the deferred read.
    let src = "package p\nfunc f() (r int) {\n\tuse(r)\n\tr = 1\n\tdefer func() {\n\t\tuse(r)\n\t}()\n\treturn\n}\n";
    check(
        "o-result",
        src,
        &[("r", 4, 3, false), ("r", 4, 6, true), ("r", 2, 3, true)],
    );
    // A deferred literal created before the write sees it at exit: both are kept.
    let early = "package p\nfunc f() (r int) {\n\tdefer func() {\n\t\tuse(r)\n\t}()\n\tr = 1\n\treturn\n}\n";
    check("o-result-early", early, &[("r", 6, 4, true)]);
}

#[test]
fn direct_pairs_are_unchanged() {
    let src = "package p\nfunc f(x int) {\n\tv := x\n\tuse(v)\n\tfunc() {\n\t\tuse(v)\n\t}()\n}\n";
    check("capture", src, &[("v", 3, 4, true), ("v", 3, 6, true)]);
    assert!(direct(&refs(src), "v", 3, 4));
}
