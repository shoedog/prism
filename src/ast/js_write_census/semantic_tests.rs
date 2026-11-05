//! Semantic fixtures for the walker, `ident` and `precedes` (DESIGN.md 18.3, 7.4): pure functions
//! on small sources, no CPG, no reaching definitions, no labels.

use super::harness::{writes, Fx};
use super::identity::{ident, Comp, Ident};
use super::order::{precedes, single_site_operand, Position};
use super::walk::{Form, Refusal};

fn pos_pair(src_line: &str) -> (Fx, Position, Position) {
    let fx = Fx::line(src_line);
    let c = fx.first_census();
    assert!(!c.refused(), "{src_line}: {:?}", c.refusals);
    let w = writes(&c, "a");
    assert_eq!(
        w.len(),
        1,
        "{src_line}: one counting write of a, got {:?}",
        w.iter().map(|s| s.path.render()).collect::<Vec<_>>()
    );
    let wp = Position::of_site(w[0]);
    let reads = fx.reads(&c, "a");
    assert_eq!(reads.len(), 1, "{src_line}: one read of a");
    let rp = reads[0].1.clone();
    (fx, rp, wp)
}

/// seq rows: a read-before-write source proves read -> write only; a write-before-read source
/// proves write -> read only.
fn seq_row(kind: &str, read_first: &str, write_first: &str) {
    let (fx, r, w) = pos_pair(read_first);
    let on = |p: &Position| p.chain.as_ref().unwrap().contains(&kind);
    assert!(
        on(&r) || on(&w),
        "{kind}: chain {:?} / {:?}",
        r.chain,
        w.chain
    );
    assert!(
        precedes(&fx.t, &r, &w),
        "{kind}: read-before-write proven: {read_first}"
    );
    assert!(
        !precedes(&fx.t, &w, &r),
        "{kind}: write is not before the read: {read_first}"
    );
    let (fx, r, w) = pos_pair(write_first);
    assert!(
        !precedes(&fx.t, &r, &w),
        "{kind}: write-before-read is not a read-before-write proof: {write_first}"
    );
    assert!(
        precedes(&fx.t, &w, &r),
        "{kind}: write-before-read proven as write -> read: {write_first}"
    );
}

#[test]
fn seq_rows_prove_source_order() {
    let rows: &[(&str, &str, &str)] = &[
        (
            "try_statement",
            "try { sink(a); } finally { a++; }",
            "try { a++; } finally { sink(a); }",
        ),
        (
            "finally_clause",
            "try { sink(a); } finally { a++; }",
            "try { a++; } finally { sink(a); }",
        ),
        (
            "catch_clause",
            "try { g(); } catch (e) { sink(a); a++; }",
            "try { g(); } catch (e) { a++; sink(a); }",
        ),
        ("if_statement", "if (sink(a)) a++;", "if (a++) sink(a);"),
        (
            "else_clause",
            "if (c) g(); else { sink(a); a++; }",
            "if (c) g(); else { a++; sink(a); }",
        ),
        (
            "ternary_expression",
            "c ? sink(a) : a++;",
            "c ? a++ : sink(a);",
        ),
        ("binary_expression", "sink(a) && a++;", "a++ && sink(a);"),
        ("binary_expression", "sink(a) || a++;", "a++ || sink(a);"),
        ("binary_expression", "sink(a) ?? a++;", "a++ ?? sink(a);"),
        (
            "member_expression",
            "(sink(a), a++, o)?.x;",
            "(a++, sink(a), o)?.x;",
        ),
        (
            "labeled_statement",
            "lbl: { sink(a); a++; }",
            "lbl: { a++; sink(a); }",
        ),
        ("sequence_expression", "sink(a), a++;", "a++, sink(a);"),
        ("call_expression", "a(a++);", "g(a++)(a);"),
        ("new_expression", "new a(a++);", "new (g(a++))(a);"),
        (
            "template_string",
            "`${sink(a)}${a++}`;",
            "`${a++}${sink(a)}`;",
        ),
        (
            "jsx_element",
            "<div x={sink(a)}>{a++}</div>;",
            "<div x={a++}>{sink(a)}</div>;",
        ),
        (
            "object",
            "({ x: sink(a), y: a++ });",
            "({ x: a++, y: sink(a) });",
        ),
        ("array", "[sink(a), a++];", "[a++, sink(a)];"),
        (
            "assignment_expression",
            "[a] = [sink(a)];",
            "([a] = [], sink(a));",
        ),
        (
            "variable_declaration",
            "var b = sink(a), c = a++;",
            "var b = a++, c = sink(a);",
        ),
        (
            "lexical_declaration",
            "let b = sink(a), c = a++;",
            "let b = a++, c = sink(a);",
        ),
    ];
    for (kind, rf, wf) in rows {
        seq_row(kind, rf, wf);
    }
}

/// `unknown` and `loop` rows: a pair with a position under the kind is not proven in either direction.
fn unproven(kind: &str, line: &str) {
    let (fx, r, w) = pos_pair(line);
    // A wrapper kind (`with_statement`, `switch_case`, ...) is not a statement span: a position
    // under it is either mapped through an inner statement (the kind is on its chain) or unmapped
    // (E18); both refuse.
    let in_chain = [&r, &w]
        .iter()
        .any(|p| p.chain.as_ref().is_none_or(|c| c.contains(&kind)));
    assert!(
        in_chain,
        "{kind} is on a chain or the position is unmapped: {line}: {:?} / {:?}",
        r.chain, w.chain
    );
    assert!(
        !precedes(&fx.t, &r, &w),
        "{kind}: read -> write not proven: {line}"
    );
    assert!(
        !precedes(&fx.t, &w, &r),
        "{kind}: write -> read not proven: {line}"
    );
}

#[test]
fn unknown_and_loop_rows_prove_nothing() {
    for (kind, line) in [
        ("array_pattern", "const [b = sink(a)] = (a++, []);"),
        ("assignment_pattern", "const [b = sink(a)] = (a++, []);"),
        ("object_pattern", "const { b = sink(a) } = (a++, {});"),
        (
            "object_assignment_pattern",
            "const { b = sink(a) } = (a++, {});",
        ),
        ("pair_pattern", "({ [sink(a)]: b } = (a++, {}));"),
        ("rest_pattern", "const [...[b = sink(a)]] = (a++, []);"),
        (
            "switch_case",
            "switch (k) { case 1: sink(a); case a++: break; }",
        ),
        (
            "switch_default",
            "switch (999) { default: sink(a); break; case a++: break; }",
        ),
        (
            "switch_body",
            "switch (k) { case 1: a++; case 2: sink(a); }",
        ),
        (
            "class_body",
            "const C = class { static x = sink(a); static y = a++; };",
        ),
        (
            "class_static_block",
            "const C = class { static { sink(a); a++; } };",
        ),
        ("decorator", "@dec(sink(a)) @dec2(a++) class C {}"),
        ("with_statement", "with (o) { sink(a); a++; }"),
        ("for_statement", "for (sink(a); c(); a++) g();"),
        ("for_in_statement", "for (const b of (sink(a), xs)) a++;"),
        ("while_statement", "while (sink(a)) a++;"),
        ("do_statement", "do { sink(a); } while (a++ < 3);"),
    ] {
        unproven(kind, line);
    }
}

#[test]
fn deferred_callable_on_the_chain_proves_nothing() {
    let fx = Fx::line("g(() => { a++; }), sink(a);");
    let c = fx.first_census();
    let w = writes(&c, "a");
    assert_eq!(w.len(), 1);
    assert!(w[0].region.nested);
    let r = &fx.reads(&c, "a")[0].1;
    assert!(w[0].chain.as_ref().unwrap().contains(&"arrow_function"));
    assert!(!precedes(&fx.t, &Position::of_site(w[0]), r));
}

#[test]
fn unmapped_or_cross_span_positions_prove_nothing() {
    let fx = Fx::new("function f(a, b = a++) {\n  sink(a);\n}");
    let c = fx.first_census();
    let w: Vec<_> = c
        .sites
        .iter()
        .filter(|s| s.path.base == "a" && s.form == Form::ReadWrite)
        .collect();
    assert_eq!(w.len(), 1);
    assert!(
        w[0].region.own_params && w[0].chain.is_none(),
        "parameter default write is unmapped"
    );
    let r = fx.reads(&c, "a").remove(0).1;
    assert!(!precedes(&fx.t, &Position::of_site(w[0]), &r));
    // Two statements of the same function map to different spans.
    let fx = Fx::new("function f(a) {\n  sink(a);\n  a++;\n}");
    let c = fx.first_census();
    let r = fx.reads(&c, "a").remove(0).1;
    assert!(!precedes(&fx.t, &r, &Position::of_site(writes(&c, "a")[0])));
}

#[test]
fn equal_moment_writes_order_the_larger_construct_later() {
    let fx = Fx::line("a = b = 1;");
    let c = fx.first_census();
    let a = Position::of_site(writes(&c, "a")[0]);
    let b = Position::of_site(writes(&c, "b")[0]);
    assert_eq!(a.time, b.time);
    assert!(precedes(&fx.t, &b, &a), "the inner assignment writes first");
    assert!(!precedes(&fx.t, &a, &b));
    let same = Position::of_site(writes(&c, "a")[0]);
    assert!(!precedes(&fx.t, &a, &same), "equal constructs refuse");
}

fn counting<'c>(
    c: &'c super::walk::Census,
    base: &str,
    span: (usize, usize),
) -> Vec<&'c super::walk::Site> {
    c.sites
        .iter()
        .filter(|s| s.path.base == base && s.span == Some(span) && s.form != Form::Declare)
        .collect()
}

#[test]
fn single_site_exception_needs_exactly_one_counting_write() {
    // c13: while (--k): the only counting write of the node is the update, and the use is its operand.
    let fx = Fx::new("function f(n) {\n  let k = n;\n  while (--k) {\n    sink(k);\n  }\n}");
    let c = fx.first_census();
    let upd = writes(&c, "k")
        .into_iter()
        .find(|s| s.form == Form::ReadWrite)
        .unwrap();
    let node = counting(&c, "k", upd.span.unwrap());
    assert_eq!(node.len(), 1);
    assert!(single_site_operand(&node, upd.target_start));
    assert!(
        !single_site_operand(&node, upd.target_start + 1),
        "another read is not the operand"
    );
    // A second counting write in the node withdraws the exception.
    let fx = Fx::new("function f(n) {\n  let k = n;\n  while (--k) { k = 1; }\n}");
    let c = fx.first_census();
    let upd = writes(&c, "k")
        .into_iter()
        .find(|s| s.form == Form::ReadWrite)
        .unwrap();
    let node = counting(&c, "k", upd.span.unwrap());
    assert_eq!(node.len(), 2);
    assert!(!single_site_operand(&node, upd.target_start));
    // o21: delete o?.x with the operand as the read.
    let fx = Fx::new("function f(o) {\n  o.x = 1;\n  delete o?.x;\n  sink(o.x);\n}");
    let c = fx.first_census();
    let del = c.sites.iter().find(|s| s.form == Form::Delete).unwrap();
    assert_eq!(
        del.path.render(),
        "o.x",
        "optional chain normalises to the plain path"
    );
    let node = counting(&c, "o.x".split('.').next().unwrap(), del.span.unwrap());
    assert!(single_site_operand(&node, del.target_start));
    let fx = Fx::new("function f(o) {\n  o.x = 1;\n  delete o?.x, o.x = 2;\n  sink(o.x);\n}");
    let c = fx.first_census();
    let del = c.sites.iter().find(|s| s.form == Form::Delete).unwrap();
    let node = counting(&c, "o", del.span.unwrap());
    assert_eq!(node.len(), 2);
    assert!(!single_site_operand(&node, del.target_start));
    // A plain assignment is not an operand read.
    let fx = Fx::line("a = 1;");
    let c = fx.first_census();
    let w = writes(&c, "a");
    assert!(!single_site_operand(&w, w[0].target_start));
}

/// Withdrawn exceptions and the astra/Fable examples: no direction of the pair is proven.
fn nothing_proven(src: &str, base: &str, i: usize, j: usize) {
    let fx = Fx::new(src);
    let c = fx.first_census();
    let w = writes(&c, base);
    assert!(w.len() > i.max(j), "{src}: {} writes", w.len());
    let (x, y) = (Position::of_site(w[i]), Position::of_site(w[j]));
    assert!(
        !precedes(&fx.t, &x, &y) && !precedes(&fx.t, &y, &x),
        "{src}"
    );
}

fn read_vs_write(src: &str, base: &str, wi: usize) {
    let fx = Fx::new(src);
    let c = fx.first_census();
    let w = Position::of_site(writes(&c, base)[wi]);
    let reads = fx.reads(&c, base);
    assert!(!reads.is_empty(), "{src}");
    for (_, r) in reads {
        assert!(!precedes(&fx.t, &r, &w), "{src}: read -> write not proven");
    }
}

#[test]
fn withdrawn_exceptions_stay_refused() {
    // c03: a for-header def with an increment (declarator, then i++).
    nothing_proven("function f(xs) {\n  log();\n  for (let i = 0; i < xs.length; i++) {\n    sink(xs[i], i);\n  }\n}", "i", 0, 1);
    // a19: a condition write.
    nothing_proven(
        "function f(n) {\n  log();\n  for (let i = 0; (i = n) > 0; i++)\n    g(i);\n}",
        "i",
        0,
        1,
    );
    // fb01: a body def on the line, then the increment.
    nothing_proven(
        "function f(a) {\n  log();\n  for (; c(); a++) a = g();\n  sink(a);\n}",
        "a",
        0,
        1,
    );
    // a20: an initializer inside a do-while whose condition writes the same name (other lines map to the do).
    nothing_proven("function f(a) {\n  log();\n  do { for (a = 10; false;)\n    noop();\n  } while ((a++, false));\n  sink(a);\n}", "a", 0, 1);
    // a04, a06, fb09: body defs against a condition or other write.
    nothing_proven("function f(a) {\n  log();\n  for (let i = 0; (a++, i < 1); i++) { a = 10; }\n  sink(a);\n}", "a", 0, 1);
    nothing_proven("function f(a) {\n  log();\n  while (true) { a++; if (y()) break; a = g(); }\n  sink(a);\n}", "a", 0, 1);
    nothing_proven(
        "function f(a) {\n  log();\n  do { a = g(); } while (a++ < 3);\n  sink(a);\n}",
        "a",
        0,
        1,
    );
}

#[test]
fn astra_and_fable_use_side_examples_stay_refused() {
    read_vs_write("function f(a) {\n  log();\n  switch (999) { default: sink(a); break; case a++: break; }\n}", "a", 0); // a01
    read_vs_write("function f(a, k) {\n  log();\n  switch (k) { case 1: g(); default: sink(a); break; case a++: break; }\n}", "a", 0); // a02
    read_vs_write(
        "function f(a, k) {\n  log();\n  switch (k) { case 1: a++; case 2: sink(a); }\n}",
        "a",
        0,
    ); // a03
    read_vs_write(
        "function f(a) {\n  let k = 0;\n  do { if (k++) sink(a); a++; } while (k < 2);\n}",
        "a",
        0,
    ); // a05
    read_vs_write(
        "function f(a) {\n  log();\n  for (; c(); ) { if (y()) continue; sink(a); a++; }\n}",
        "a",
        0,
    ); // a07
    read_vs_write("function f(a, k) {\n  log();\n  switch (k) { default: sink(a); break; case (a++, 2): g(); }\n}", "a", 0); // fb02
    read_vs_write(
        "function f(a, xs) {\n  log();\n  for (a of xs) sink(a);\n}",
        "a",
        0,
    ); // fb07
    read_vs_write(
        "function f(a) {\n  log();\n  const [b = sink(a)] = (a++, []);\n}",
        "a",
        0,
    ); // o01
}

#[test]
fn proven_controls_stay_proven() {
    // d02, d03, o08, o11, o13: reads that precede the only write in a seq node.
    for line in [
        "sink(a), a++;",
        "a = g(a);",
        "[b] = [sink(a), (a = 2)];",
        "[a] = [sink(a)];",
        "try { sink(a); } finally { a++; }",
    ] {
        let (fx, r, w) = pos_pair(line);
        assert!(precedes(&fx.t, &r, &w), "{line}");
    }
}

#[test]
fn site_forms_and_constructs() {
    let fx = Fx::line("a += 1;");
    let c = fx.first_census();
    let w = writes(&c, "a")[0];
    assert_eq!(
        (w.form, w.construct_kind),
        (Form::ReadWrite, Some("augmented_assignment_expression"))
    );
    let fx = Fx::line("var b;");
    let b = fx
        .first_census()
        .sites
        .into_iter()
        .find(|s| s.path.base == "b")
        .unwrap();
    assert!(
        b.form == Form::Declare && b.decl == "declarator" && !b.initialized,
        "var b; stores no value"
    );
    let fx = Fx::line("for (a of xs) g();");
    let h = writes(&fx.first_census(), "a")
        .into_iter()
        .next()
        .unwrap()
        .clone();
    assert!(h.for_head && h.construct_kind.is_none());
    let fx = Fx::line("with (o) { a = 1; }");
    assert!(writes(&fx.first_census(), "a")[0].region.with_body);
    let fx = Fx::line("g(function (z = (a = 1)) {});");
    let w = writes(&fx.first_census(), "a")[0].clone();
    assert!(w.region.nested && w.region.nested_params && !w.region.own_params);
    let fx = Fx::line("const C = class { x = (a = 2); static y = (a = 3); };");
    let c = fx.first_census();
    let flags: Vec<bool> = writes(&c, "a")
        .iter()
        .map(|s| s.region.instance_field)
        .collect();
    assert_eq!(
        flags,
        vec![true, false],
        "instance field initializers are deferred, static ones eager"
    );
    let fx = Fx::new("function f(a = (a = 1)) {}");
    let c = fx.first_census();
    let w = c
        .sites
        .iter()
        .find(|s| s.form == Form::Write && s.path.base == "a")
        .unwrap();
    assert!(
        w.region.own_params,
        "a default-value write in the owner's own parameter list"
    );
}

fn target_ident(src: &str, lang_ts: bool) -> Ident {
    let _ = lang_ts;
    let fx = Fx::new(src);
    let o = super::harness::owners(&fx.parsed, &fx.t)[0];
    let mut stack = vec![o];
    while let Some(n) = stack.pop() {
        if n.kind() == "assignment_expression"
            || n.kind() == "update_expression"
            || n.kind() == "unary_expression"
        {
            let slot = if n.kind() == "assignment_expression" {
                "left"
            } else {
                "argument"
            };
            let t = n.child_by_field_name(slot).unwrap();
            return ident(&fx.t, fx.parsed.source.as_bytes(), t, true);
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c).collect::<Vec<_>>());
    }
    panic!("no target in {src}");
}

fn path(base: &str, comps: &[Comp]) -> Ident {
    Ident::Path(super::identity::Path {
        base: base.to_string(),
        comps: comps.to_vec(),
    })
}

fn name(s: &str) -> Comp {
    Comp::Name(s.to_string())
}

#[test]
fn ident_on_every_target_shape() {
    let w = |s: &str| target_ident(&format!("function f(o, a, k) {{ {s} }}"), true);
    assert_eq!(w("a = 1;"), path("a", &[]));
    assert_eq!(w("o.x.y = 1;"), path("o", &[name("x"), name("y")]));
    assert_eq!(w("o[k] = 1;"), path("o", &[Comp::Index]));
    assert_eq!(
        w("o['x'] = 1;"),
        path("o", &[Comp::Index]),
        "any index is []"
    );
    assert_eq!(
        w("o.x[k].y = 1;"),
        path("o", &[name("x"), Comp::Index, name("y")])
    );
    assert_eq!(w("(a) = 1;"), path("a", &[]));
    assert_eq!(w("((o).x) = 1;"), path("o", &[name("x")]));
    assert_eq!(w("(a as any) = 1;"), path("a", &[]));
    assert_eq!(w("a! = 1;"), path("a", &[]));
    assert_eq!(w("(<any>a) = 1;"), path("a", &[]));
    assert_eq!(w("(a satisfies any) = 1;"), path("a", &[]));
    assert_eq!(
        w("(a as Foo) = 1;"),
        path("a", &[]),
        "a named type is not the operand"
    );
    assert_eq!(w("o!.x = 1;"), path("o", &[name("x")]));
    assert_eq!(
        w("o?.x++;"),
        path("o", &[name("x")]),
        "optional chain is ignored for identity"
    );
    assert_eq!(w("delete o?.x;"), path("o", &[name("x")]));
    assert_eq!(w("this.x = 1;"), path("this", &[name("x")]));
    assert_eq!(w("o.#p = 1;"), path("o", &[name("#p")]));
    // Non-reference roots write no owner binding.
    assert_eq!(w("f().x = 1;"), Ident::NoWrite);
    assert_eq!(w("new C().x = 1;"), Ident::NoWrite);
    assert_eq!(w("'s'.x = 1;"), Ident::NoWrite);
    assert_eq!(w("(await g()).x = 1;"), Ident::NoWrite);
    assert_eq!(w("(c ? o : a).x = 1;"), Ident::NoWrite);
    // REFUSE kinds in a write position refuse; an unclassified wrapper shape refuses too.
    assert_eq!(w("(f()) = 1;"), Ident::Refuse("call_expression".into()));
    assert_eq!(w("(a + k) = 1;"), Ident::Refuse("binary_expression".into()));
    let s = target_ident("class C extends B { m() { super.x = 1; } }", false);
    assert_eq!(s, path("this", &[name("x")]), "super.x writes the receiver");
}

#[test]
fn affects_and_eq_relations() {
    let p = |s: &str| match w_path(s) {
        Ident::Path(p) => p,
        other => panic!("{other:?}"),
    };
    let (o, ox, oxy, oidx, ax) = (p("o"), p("o.x"), p("o.x.y"), p("o[k]"), p("a.x"));
    assert!(o.affects(&ox) && ox.affects(&oxy) && ox.affects(&ox));
    assert!(
        !ox.affects(&o),
        "a read of a path above the write is not affected"
    );
    assert!(
        oidx.affects(&ox) && ox.affects(&oidx),
        "[] matches any component on either side"
    );
    assert!(!o.affects(&ax));
    assert!(
        ox.eq_path(&p("o.x")) && !ox.eq_path(&oidx),
        "eq compares raw components"
    );
}

fn w_path(s: &str) -> Ident {
    target_ident(&format!("function f(o, a, k) {{ {s} = 1; }}"), true)
}

fn line_source(fixture: &str) -> String {
    super::matrix_tests::source_of(fixture)
}

#[test]
fn owner_preconditions_refuse_whole_owner() {
    let refusal = |name: &str| {
        let fx = Fx::new(&line_source(name));
        fx.first_census().refusals
    };
    // Escaped NAME tokens anywhere in the owner (writes, reads, members, labels, hoisted names).
    for n in [
        "w01_escaped_update_multi",
        "w02_escaped_braced_assign_multi",
        "w03_escaped_update_one_node",
        "w04_escaped_braced_one_node",
        "w05_escaped_unrelated_control",
        "w06_escaped_read_multi",
        "w07_escaped_member_update",
        "w08_escaped_intermediate_component",
        "w09_escaped_braced_member",
        "w10_escaped_other_object_cost",
        "w11_escaped_property_read",
        "w12_escaped_hoisted_name",
        "w13_escaped_label_cost",
    ] {
        assert_eq!(refusal(n), vec![Refusal::Escape], "{n}");
        assert!(
            Fx::new(&line_source(n)).first_census().sites.is_empty(),
            "{n}: refused as a whole"
        );
    }
    // o26: `using` parses, so the owner is not refused and `a++` is a site.
    let fx = Fx::new(&line_source("o26_using_declaration_update"));
    let c = fx.first_census();
    assert!(
        c.refusals.is_empty()
            && c.sites
                .iter()
                .any(|s| s.path.base == "a" && s.form == Form::ReadWrite)
    );
    // Controls: no escape and no parse error.
    for n in ["m01_update_post", "c01_plain_control", "t26_tsx_update"] {
        assert!(refusal(n).is_empty(), "{n}");
    }
    // A broken owner in an otherwise fine file.
    let fx = Fx::new("function f(a) { a = ; }");
    assert_eq!(fx.first_census().refusals, vec![Refusal::Parse]);
}

#[test]
fn refuse_kind_in_write_position_refuses_owner() {
    let fx = Fx::new("function f(a) {\n  (a + 1) = 2;\n}");
    let c = fx.first_census();
    assert_eq!(
        c.refusals,
        vec![Refusal::Target("binary_expression".into())]
    );
}
