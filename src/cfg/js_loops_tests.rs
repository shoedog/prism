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

/// The edges of the same source with `js_loops` and the JS `do` condition admission off: main's.
fn main_edges(src: &str) -> BTreeSet<(usize, usize)> {
    crate::cfg::js_loops::with_main_loop_cfg(|| edges(src))
}

fn universe(src: &str) -> Vec<usize> {
    let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    let owner = parsed.all_functions().into_iter().next().unwrap();
    parsed
        .statements_in_function(&owner)
        .into_iter()
        .map(|(line, _)| line)
        .collect()
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
    // One line, a condition on the last body line, a statement after the `do` on the condition line:
    // not admitted, so the whole CFG is main's (compared against a build under `with_main_loop_cfg`).
    for src in [
        "function f(a) {\n  log();\n  do { a = g(); } while (a-- > 0);\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  do { x();\n    a = g(); } while (a-- > 0);\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  do {\n    a = g();\n  } while (a-- > 0); sink(a);\n}\n",
    ] {
        assert_eq!(edges(src), main_edges(src), "{src}");
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

// ---- Round 1 of PR #785 ------------------------------------------------------------------------------

/// F1: a jump that leaves a `try` body or `catch` whose `finally` does not enclose its target would
/// skip the `finally`'s writes with a direct edge (a wrong `exact`): the tree keeps main's edges.
#[test]
fn a_jump_through_a_finally_keeps_mains_edges_for_the_tree() {
    for (what, src) in [
        ("break", "function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    try {\n      x = 1;\n      break;\n    } finally {\n      x = 2;\n    }\n  }\n  return x;\n}\n"),
        ("continue", "function f(n, x) {\n  x = 0;\n  while (n-- > 0) {\n    try {\n      x = 1;\n      continue;\n    } finally {\n      x = 2;\n    }\n  }\n  return x;\n}\n"),
        ("labeled break from an inner loop", "function f(xs, x) {\n  x = 0;\n  outer: for (const a of xs) {\n    for (const b of a) {\n      try {\n        x = 1;\n        break outer;\n      } finally {\n        x = 2;\n      }\n    }\n  }\n  return x;\n}\n"),
        ("continue from a catch", "function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    try {\n      x = 1;\n    } catch (e) {\n      x = 3;\n      continue;\n    } finally {\n      x = 2;\n    }\n  }\n  return x;\n}\n"),
        ("switch break", "function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    switch (a) {\n      case 1:\n        try {\n          x = 1;\n          break;\n        } finally {\n          x = 2;\n        }\n    }\n    use(x);\n  }\n  return x;\n}\n"),
        ("labeled block break", "function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    blk: {\n      try {\n        x = 1;\n        break blk;\n      } finally {\n        x = 2;\n      }\n    }\n    use(x);\n  }\n  return x;\n}\n"),
    ] {
        assert_eq!(edges(src), main_edges(src), "{what}");
    }
}

#[test]
fn a_finally_that_encloses_the_target_or_holds_the_jump_does_not_block_rewiring() {
    // The `finally` surrounds the whole loop: `after(loop)` enters it, the break edge is real.
    let around = edges("function f(xs, x) {\n  x = 0;\n  try {\n    for (const a of xs) {\n      x = 1;\n      break;\n    }\n  } finally {\n    x = 2;\n  }\n  return x;\n}\n");
    assert!(has(&around, 6, 9), "break -> the finally: {around:?}");
    // A jump written in the `finally` block itself is not leaving it through a skipped `finally`.
    let inside = edges("function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    try {\n      x = 1;\n    } finally {\n      x = 2;\n      break;\n    }\n  }\n  return x;\n}\n");
    assert!(
        has(&inside, 8, 11),
        "break in a finally -> after the loop: {inside:?}"
    );
    // No `finally`: nothing to skip.
    let catch_only = edges("function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    try {\n      x = 1;\n      break;\n    } catch (e) {\n      x = 3;\n    }\n  }\n  return x;\n}\n");
    assert!(has(&catch_only, 6, 11), "{catch_only:?}");
}

/// F1 (Opus p1): a `try` whose body and `catch` never complete normally completes nowhere, so its
/// `finally` is not a route back to the loop head.
#[test]
fn a_try_with_no_normal_completion_completes_nowhere() {
    let e = edges("function f(xs, x) {\n  x = 0;\n  for (const a of xs) {\n    try {\n      return a;\n    } catch (e) {\n      throw e;\n    } finally {\n      x = 2;\n    }\n  }\n  return x;\n}\n");
    assert!(
        !has(&e, 9, 3),
        "the finally is not a loop-back route: {e:?}"
    );
    assert!(has(&e, 3, 12), "the header exit stands: {e:?}");
}

/// F2: a `do` condition is a CFG statement only when its loop tree is rewired.
#[test]
fn an_unrewirable_do_keeps_main_s_universe_and_edges() {
    let body_unknown = "function f(c) {\n  let x = 0;\n  do {\n    use(x);\n    debugger;\n  } while ((x = next()));\n  return x;\n}\n";
    assert!(
        !universe(body_unknown).contains(&6),
        "no condition statement"
    );
    assert_eq!(edges(body_unknown), main_edges(body_unknown));
    let around_one_line = "function f(c) {\n  let x = 0;\n  do {\n    do { x = next(); } while (c);\n    use(x);\n  } while ((x = next()));\n  return x;\n}\n";
    assert!(
        !universe(around_one_line).contains(&6),
        "no condition statement"
    );
    assert_eq!(edges(around_one_line), main_edges(around_one_line));
}

#[test]
fn a_do_with_an_empty_else_statement_is_rewired() {
    // `else ;` completes at the `if`; the loop is rewired and its condition is a statement.
    let src = "function f(c) {\n  let x = 0;\n  do {\n    use(x);\n    if (c) { log(); } else ;\n  } while ((x = next()));\n  return x;\n}\n";
    assert!(universe(src).contains(&6));
    let e = edges(src);
    for (from, to) in [(5, 6), (6, 4), (6, 7)] {
        assert!(has(&e, from, to), "{from}->{to}: {e:?}");
    }
    assert!(!has(&e, 3, 7), "{e:?}");
}

/// F3: the initializer runs once but shares the header node.
#[test]
fn a_for_whose_initializer_writes_a_name_the_body_writes_keeps_mains_edges() {
    for (what, src) in [
        ("assignment", "function f(more) {\n  var i;\n  for (i = 0; more();) {\n    i = 1;\n  }\n  sink(i);\n}\n"),
        ("sequence", "function f(a) {\n  var i, count;\n  for (i = 0, count = 0; i < a.length; i++) {\n    if (a[i]) {\n      count = count + 1;\n    }\n  }\n  return count;\n}\n"),
        ("let declaration", "function f(n, g) {\n  for (let m = 0; m < n;) {\n    m = g(m);\n  }\n}\n"),
        ("var declaration", "function f(n, g) {\n  for (var m = 0; m < n;) {\n    m = g(m);\n  }\n  sink(m);\n}\n"),
        ("multi-line header", "function f(more) {\n  var i;\n  for (\n    i = 0;\n    more();\n  ) {\n    i = 1;\n  }\n  sink(i);\n}\n"),
        ("destructuring initializer", "function f(p, more) {\n  var a;\n  for ([a] = p; more();) {\n    a = 1;\n  }\n  sink(a);\n}\n"),
    ] {
        assert_eq!(edges(src), main_edges(src), "{what}");
    }
}

#[test]
fn a_for_initializer_the_update_or_the_body_leaves_alone_does_not_block_rewiring() {
    // The update writes the name: the kill at the header is real.
    let update = edges("function f(n) {\n  var i;\n  for (i = 0; i < n; i++) {\n    i = g();\n  }\n  sink(i);\n}\n");
    assert!(has(&update, 4, 3) && !has(&update, 4, 6), "{update:?}");
    // The body never writes the initializer's name.
    let other = edges("function f(more) {\n  var k, z;\n  for (k = 0; more();) {\n    z = g();\n  }\n  sink(z);\n}\n");
    assert!(has(&other, 4, 3) && !has(&other, 4, 6), "{other:?}");
}

/// F4: a `continue` of a `for`..`in`/`of`/`await` loop also reaches the exhaustion successor.
#[test]
fn a_continue_of_an_iteration_loop_also_reaches_the_code_after_it() {
    for (what, src, cont, head, after) in [
        ("for..of", "function f(xs) {\n  let x;\n  for (x of xs) {\n    x = g(x);\n    continue;\n  }\n  return x;\n}\n", 5, 3, 7),
        ("member head", "function f(o, xs) {\n  o.p = 0;\n  for (o.p in xs) {\n    o.p = g(o.p);\n    continue;\n  }\n  return o.p;\n}\n", 5, 3, 7),
        ("destructuring head", "function f(xs) {\n  let a, b;\n  a = 1;\n  for ([a, b] of xs) {\n    a = 2;\n    continue;\n  }\n  return a;\n}\n", 6, 4, 8),
        ("for await", "async function f(xs) {\n  let x = 0;\n  for await (x of xs) {\n    x = g(x);\n    continue;\n  }\n  return x;\n}\n", 5, 3, 7),
        ("labeled continue from an inner loop", "function f(xs, x) {\n  x = 0;\n  outer: for (x of xs) {\n    for (const b of x) {\n      x = 1;\n      continue outer;\n    }\n  }\n  return x;\n}\n", 6, 3, 9),
    ] {
        let e = edges(src);
        assert!(has(&e, cont, head), "{what}: continue -> head: {e:?}");
        assert!(has(&e, cont, after), "{what}: continue -> after the loop: {e:?}");
    }
    // A `for` or `while` has no exhaustion successor of its own: its header exit is the head's.
    let plain = edges("function f(n) {\n  let i = 0;\n  for (; i < n; i++) {\n    i = g();\n    continue;\n  }\n  return i;\n}\n");
    assert!(has(&plain, 5, 3) && !has(&plain, 5, 7), "{plain:?}");
}

/// F5: an empty arm completes where it was entered.
#[test]
fn an_empty_arm_completes_where_it_was_entered() {
    for (what, src, completing) in [
        ("empty else", "function f(xs, c) {\n  let y = 0;\n  for (const a of xs) {\n    use(y);\n    y = a;\n    if (c) {\n      y = 1;\n    } else {\n    }\n  }\n  return y;\n}\n", [6, 7].as_slice()),
        ("empty then", "function f(xs, c) {\n  let y = 0;\n  for (const a of xs) {\n    use(y);\n    y = a;\n    if (c) {\n    } else {\n      y = 1;\n    }\n  }\n  return y;\n}\n", &[6, 8]),
        ("empty finally", "function f(xs) {\n  let x = 0;\n  for (const a of xs) {\n    use(x);\n    try {\n      x = a;\n    } finally {\n    }\n  }\n  return x;\n}\n", &[6]),
        ("empty catch", "function f(xs) {\n  let y = 0;\n  for (const a of xs) {\n    use(y);\n    y = a;\n    try {\n      y = parse(a);\n    } catch (e) {}\n  }\n  return y;\n}\n", &[6, 7]),
        ("comment-only catch", "function f(xs) {\n  let y = 0;\n  for (const a of xs) {\n    use(y);\n    y = a;\n    try {\n      y = parse(a);\n    } catch (e) {\n      // ignore\n    }\n  }\n  return y;\n}\n", &[6, 7]),
    ] {
        let e = edges(src);
        for &from in completing {
            assert!(has(&e, from, 3), "{what}: {from} completes into the head: {e:?}");
        }
    }
}

/// F6: only the edges the loop's builders own are dropped: an `if` that starts on the loop's first line
/// keeps its branch edge to the `else`.
#[test]
fn an_enclosing_if_keeps_its_branch_edge() {
    let e = edges("function f(c, x) {\n  let y = 0;\n  if (c) while (x--) {\n    y = 1;\n  }\n  else {\n    use(y);\n  }\n  return y;\n}\n");
    assert!(has(&e, 3, 7), "if -> else: {e:?}");
    assert!(has(&e, 3, 9), "loop exit -> after the if: {e:?}");
    assert!(has(&e, 4, 3), "{e:?}");
}

/// S1: a loop tree is rewired all or nothing, so an unsupported statement outside an inner loop keeps
/// main's edges for the inner loop's `continue outer` too; a supported tree routes it.
#[test]
fn a_loop_tree_is_rewired_all_or_nothing() {
    let unsupported = "function f(xs) {\n  let x = 0;\n  outer: for (const a of xs) {\n    for (const b of a) {\n      x = 1;\n      continue outer;\n    }\n    debugger;\n  }\n  return x;\n}\n";
    assert_eq!(edges(unsupported), main_edges(unsupported));
    let supported = edges("function f(xs) {\n  let x = 0;\n  outer: for (const a of xs) {\n    for (const b of a) {\n      x = 1;\n      continue outer;\n    }\n    x = 2;\n  }\n  return x;\n}\n");
    assert!(
        has(&supported, 6, 3) && has(&supported, 6, 10),
        "{supported:?}"
    );
}

/// S2: counters can move by construction. A one-line owner whose loop holds a `continue` gains its
/// first CFG edge (a self-loop), which flips RD availability from no-CFG to available; a two-line `do`
/// gains its condition as a node.
#[test]
fn the_edge_and_node_counts_of_small_owners_can_move() {
    let one_line =
        "function p15(xs) { let y = 0; for (const x of xs) { y = g(x, y); continue; } }\n";
    assert!(main_edges(one_line).is_empty());
    assert_eq!(edges(one_line), BTreeSet::from([(1, 1)]));
    let two_line_do = "function p11(c) {\n  do x = c.next();\n  while (x);\n}\n";
    assert_eq!(universe(two_line_do), [2, 3]);
    crate::cfg::js_loops::with_main_loop_cfg(|| assert_eq!(universe(two_line_do), [2]));
    // The body starts on the `do` line: main's back edge 2 -> 2 would skip the condition, so it is gone.
    assert_eq!(edges(two_line_do), BTreeSet::from([(2, 3), (3, 2)]));
}
