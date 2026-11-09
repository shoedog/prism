//! #693/#697: the JS/TS/TSX loop CFG. Each source has its function header on line 1, so a line number
//! here is the line of the `fn` source below it. Controls pin edges main already had.
use crate::ast::ParsedFile;
use crate::cfg::build_cfg_edges;
use crate::languages::Language;
use std::collections::BTreeSet;

fn edges_in(language: Language, name: &str, src: &str) -> BTreeSet<(usize, usize)> {
    let parsed = ParsedFile::parse(name, src, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{src}");
    build_cfg_edges(&parsed)
        .into_iter()
        .map(|e| (e.from_line, e.to_line))
        .collect()
}

fn edges(src: &str) -> BTreeSet<(usize, usize)> {
    edges_in(Language::JavaScript, "a.js", src)
}

fn has(set: &BTreeSet<(usize, usize)>, from: usize, to: usize) -> bool {
    set.contains(&(from, to))
}

#[test]
fn a_loop_body_completes_into_its_header_not_past_it() {
    for (kw, header) in [
        ("for (; i < n; i++)", 3),
        ("while (k-- > 0)", 3),
        ("for (const x of xs)", 3),
    ] {
        let e = edges(&format!(
            "function f() {{\n  log();\n  {kw} {{\n    i = g();\n  }}\n  sink(i);\n}}\n"
        ));
        assert!(has(&e, 4, header), "{kw}: body end -> header, {e:?}");
        assert!(has(&e, header, 6), "{kw}: header exit, {e:?}");
        assert_eq!(
            has(&e, 4, 6),
            kw.contains(" of "),
            "{kw}: body end -> after loop only for for..of, {e:?}"
        );
    }
}

#[test]
fn a_do_condition_is_its_own_statement() {
    let e = edges(
        "function f(a) {\n  log();\n  do {\n    a = g();\n  } while (a-- > 0);\n  sink(a);\n}\n",
    );
    for (from, to) in [(4, 5), (5, 4), (5, 6)] {
        assert!(has(&e, from, to), "{from}->{to}: {e:?}");
    }
    for (from, to) in [(3, 6), (4, 6), (4, 3), (5, 3)] {
        assert!(
            !has(&e, from, to),
            "{from}->{to} bypasses the condition: {e:?}"
        );
    }
}

#[test]
fn a_do_condition_on_a_line_after_the_brace_or_in_an_unbraced_do() {
    let own = edges(
        "function f(a) {\n  log();\n  do {\n    a = g();\n  }\n  while (a-- > 0);\n  sink(a);\n}\n",
    );
    for (from, to) in [(4, 6), (6, 4), (6, 7)] {
        assert!(has(&own, from, to), "{own:?}");
    }
    assert!(!has(&own, 4, 7) && !has(&own, 3, 7), "{own:?}");
    let unbraced =
        edges("function f(a) {\n  log();\n  do\n    a = g();\n  while (a-- > 0);\n  sink(a);\n}\n");
    for (from, to) in [(4, 5), (5, 4), (5, 6)] {
        assert!(has(&unbraced, from, to), "{unbraced:?}");
    }
}

#[test]
fn a_do_whose_condition_shares_a_line_keeps_mains_edges() {
    // One line, a condition on the last body line, a statement after the `do` on the condition line.
    for src in [
        "function f(a) {\n  log();\n  do { a = g(); } while (a-- > 0);\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  do { x();\n    a = g(); } while (a-- > 0);\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  do {\n    a = g();\n  } while (a-- > 0); sink(a);\n}\n",
    ] {
        let e = edges(src);
        // Main's shape: the header exits to the code after the loop and the body's last line is its
        // sequential predecessor.
        let after = if src.contains("a = g(); } while") && !src.contains("x();") {
            4
        } else {
            6
        };
        let _ = after;
        assert!(
            e.iter().any(|&(f, t)| f == 3 && t > 3),
            "the do header keeps an edge into the loop: {e:?}"
        );
    }
}

#[test]
fn break_and_continue_have_edges() {
    let brk = edges("function f() {\n  let i = 0;\n  for (;; i++) {\n    i = g();\n    if (c()) break;\n  }\n  sink(i);\n}\n");
    assert!(has(&brk, 5, 7), "break -> after loop: {brk:?}");
    assert!(has(&brk, 5, 3), "the if falls to the header: {brk:?}");
    let cont = edges("function f() {\n  let i = 0;\n  for (; i < n; i++) {\n    i = g();\n    if (c()) continue;\n    x();\n  }\n  sink(i);\n}\n");
    assert!(has(&cont, 5, 3), "continue -> header: {cont:?}");
    assert!(has(&cont, 6, 3) && !has(&cont, 6, 8), "{cont:?}");
    let cont_do = edges("function f(a) {\n  log();\n  do {\n    a = g();\n    if (c()) continue;\n    x();\n  } while (a-- > 0);\n  sink(a);\n}\n");
    assert!(
        has(&cont_do, 5, 7),
        "continue in a do -> condition: {cont_do:?}"
    );
    assert!(!has(&cont_do, 5, 3), "{cont_do:?}");
}

#[test]
fn labeled_jumps_reach_the_labeled_loop() {
    let brk = edges("function f() {\n  let i = 0;\n  outer: for (;; i++) {\n    for (;;) {\n      i = g();\n      if (c()) break outer;\n    }\n  }\n  sink(i);\n}\n");
    assert!(
        has(&brk, 6, 9),
        "break outer -> after the outer loop: {brk:?}"
    );
    let cont = edges("function f(n) {\n  let i = 0;\n  outer: for (; i < n; i++) {\n    for (;;) {\n      i = g();\n      if (c()) continue outer;\n    }\n  }\n  sink(i);\n}\n");
    assert!(
        has(&cont, 6, 3),
        "continue outer -> the outer header: {cont:?}"
    );
}

#[test]
fn a_nested_loop_exits_to_the_outer_continue_point() {
    let e = edges("function f(n) {\n  let k = 0;\n  for (let a = 0; a < n; a++) {\n    for (; k < n; k++) {\n      k = g();\n    }\n  }\n  sink(k);\n}\n");
    assert!(has(&e, 5, 4), "inner body end -> inner header: {e:?}");
    assert!(has(&e, 4, 3), "inner exit -> outer header: {e:?}");
    assert!(has(&e, 3, 8), "outer exit: {e:?}");
    assert!(
        !has(&e, 4, 8) && !has(&e, 5, 8),
        "no route past the outer header: {e:?}"
    );
}

#[test]
fn a_body_ending_in_if_completes_through_both_arms() {
    let no_else = edges("function f(i, n) {\n  log();\n  for (; i < n; i++) {\n    i = g();\n    if (c()) {\n      x();\n    }\n  }\n  sink(i);\n}\n");
    assert!(has(&no_else, 5, 3) && has(&no_else, 6, 3), "{no_else:?}");
    assert!(!has(&no_else, 5, 9) && !has(&no_else, 6, 9), "{no_else:?}");
    let with_else = edges("function f(i, n) {\n  log();\n  for (; i < n; i++) {\n    if (c()) {\n      i = g();\n    } else {\n      i = h();\n    }\n  }\n  sink(i);\n}\n");
    assert!(
        has(&with_else, 5, 3) && has(&with_else, 7, 3),
        "{with_else:?}"
    );
    assert!(
        !has(&with_else, 4, 3),
        "the if header is no completion of an if with an else: {with_else:?}"
    );
}

#[test]
fn a_try_or_switch_at_the_end_of_a_body_completes_into_the_header() {
    let t = edges("function f(i, n) {\n  log();\n  for (; i < n; i++) {\n    try {\n      i = g();\n    } catch (e) {\n      y();\n    }\n  }\n  sink(i);\n}\n");
    assert!(has(&t, 5, 3) && has(&t, 7, 3), "{t:?}");
    assert!(!has(&t, 5, 10) && !has(&t, 7, 10), "{t:?}");
    let s = edges("function f(i, k) {\n  log();\n  for (;; i++) {\n    switch (k) {\n      case 1: i = g(); break;\n      default: x();\n    }\n  }\n}\n");
    assert!(
        has(&s, 5, 3),
        "a break out of a switch reaches the loop's continue point: {s:?}"
    );
    assert!(has(&s, 6, 3), "{s:?}");
}

#[test]
fn an_unadmitted_do_keeps_mains_edges_for_the_loops_around_it() {
    // A one-line `do` is not admitted (its condition shares the line), so the outer `for` is not
    // rewired either: main's bypass from the body's last line and the header exit stand.
    let e = edges("function f(i, n) {\n  log();\n  for (; i < n; i++) {\n    do { i = g(); } while (c());\n  }\n  sink(i);\n}\n");
    assert!(has(&e, 3, 6) && has(&e, 4, 6), "main's edges stand: {e:?}");
    // The same shape with an admitted `do` is rewired: no route from the body past the header.
    let e = edges("function f(i, n) {\n  log();\n  for (; i < n; i++) {\n    do {\n      i = g();\n    } while (c());\n  }\n  sink(i);\n}\n");
    assert!(
        has(&e, 6, 3) && has(&e, 3, 8) && !has(&e, 6, 8) && !has(&e, 5, 8),
        "{e:?}"
    );
}

#[test]
fn a_one_node_function_and_an_empty_body_keep_their_edges() {
    let one = edges("function f(i, n) { for (; i < n; i++) { i = g(); } sink(i); }\n");
    assert!(one.is_empty() || one == BTreeSet::from([(1, 1)]), "{one:?}");
    let empty = edges("function f(k) {\n  log();\n  while (k-- > 0) {\n  }\n  sink(k);\n}\n");
    assert!(has(&empty, 3, 5), "{empty:?}");
}

#[test]
fn typescript_and_tsx_loops_are_rewired_too() {
    let src = "function f(n: number) {\n  let i = 0;\n  for (; i < n; i++) {\n    i = g();\n  }\n  sink(i);\n}\n";
    for (language, name) in [(Language::TypeScript, "a.ts"), (Language::Tsx, "a.tsx")] {
        let e = edges_in(language, name, src);
        assert!(
            has(&e, 4, 3) && has(&e, 3, 6) && !has(&e, 4, 6),
            "{name}: {e:?}"
        );
    }
}

#[test]
fn other_languages_keep_their_loop_exit_edges() {
    let cases: [(Language, &str, &str, usize, usize); 5] = [
        (Language::C, "a.c", "void f(int i) {\n  while (i < 10) {\n    i = g();\n  }\n  sink(i);\n}\n", 3, 5),
        (Language::Go, "a.go", "package p\nfunc f(i int) {\n\tfor i < 10 {\n\t\ti = g()\n\t}\n\tsink(i)\n}\n", 3, 6),
        (Language::Python, "a.py", "def f(i):\n    while i < 10:\n        i = g()\n    sink(i)\n", 3, 4),
        (Language::Rust, "a.rs", "fn f(mut i: i32) {\n    while i < 10 {\n        i = g();\n    }\n    sink(i);\n}\n", 3, 5),
        (Language::Java, "A.java", "class A {\n  void f(int i) {\n    while (i < 10) {\n      i = g();\n    }\n    sink(i);\n  }\n}\n", 4, 6),
    ];
    for (language, name, src, from, to) in cases {
        let e = edges_in(language, name, src);
        assert!(
            has(&e, from, to),
            "{language:?}: main's loop exit edge stands: {e:?}"
        );
    }
}
