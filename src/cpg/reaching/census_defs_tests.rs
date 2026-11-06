//! #660 S1b-1: the census completion, its two seams, availability parity and the trace.
use super::*;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::test_support::label_snapshot;
use crate::js_write_census::matrix_tests::{fixtures, lang_of};

fn parse(language: Language, src: &str) -> ParsedFile {
    let ext = match language {
        Language::TypeScript => "ts",
        Language::Tsx => "tsx",
        _ => "js",
    };
    ParsedFile::parse(&format!("a.{ext}"), src, language).unwrap()
}

/// The producer's recorded defs of `owner`, read back from the DataFlow graph the CPG built for the
/// file: every distinct edge source of the owner (path, line, start byte).
fn recorded(parsed: &ParsedFile, owner: &Node<'_>) -> (Vec<DefSite>, Vec<FlowEdge>) {
    let name = parsed.dfg_owner_name(owner).unwrap_or_default();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(parsed.path.clone(), parsed.clone())]));
    let edges: Vec<FlowEdge> = label_snapshot(&cpg.dfg)
        .into_keys()
        .filter(|(from, _)| from.function == name)
        .map(|(from, to)| FlowEdge { from, to })
        .collect();
    let mut seen = BTreeSet::new();
    let defs = edges
        .iter()
        .filter(|e| seen.insert((e.from.path.clone(), e.from.line, e.from.start_byte)))
        .enumerate()
        .map(|(i, e)| DefSite {
            id: DefId(i as u32),
            path: e.from.path.clone(),
            line: e.from.line,
            start_byte: e.from.start_byte,
            alias_derived: false,
        })
        .collect();
    (defs, edges)
}

fn availability(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
) -> (Option<RdUnavailable>, Option<RdUnavailable>) {
    let cfg = cfg::build_cfg_edges_with_arms(parsed);
    let index = FileCfgEdgeIndex::new(&cfg);
    let pre = unavailable_before_edges(parsed, owner, defs, &index);
    let solver = match reaching_definitions_with_exact(parsed, owner, defs, &[], &[], &index) {
        RdOutcome::Available(_) => None,
        RdOutcome::Unavailable(reason) => Some(reason),
    };
    (pre, solver)
}

#[test]
fn preflight_and_solver_agree_on_recorded_defs_for_every_matrix_fixture() {
    let mut owners = 0;
    for (name, _slice, src) in fixtures() {
        let parsed = parse(lang_of(&name), &src.replace("\\n", "\n"));
        for owner in parsed.all_functions() {
            let (defs, _) = recorded(&parsed, &owner);
            let (pre, solver) = availability(&parsed, &owner, &defs);
            assert_eq!(pre, solver, "{name}");
            owners += 1;
        }
    }
    assert!(owners > 150, "{owners}");
}

fn def(id: u32, path: &str, line: usize, byte: usize) -> DefSite {
    DefSite {
        id: DefId(id),
        path: AccessPath::from_expr(path),
        line,
        start_byte: byte,
        alias_derived: false,
    }
}

#[test]
fn the_census_never_changes_availability() {
    // `delete o.x` adds a def of base `o` to a one-node owner, and 2,050 updates overflow the def cap:
    // preflight and solver both run on recorded defs, so both stay available as on main.
    let one_node = "const f = (o) => (delete o.x, use(o));\n";
    let mut capped = String::from("function f(x) {\n  x = 0;\n");
    for _ in 0..(RD_MAX_DEFS + 2) {
        capped.push_str("  x++;\n");
    }
    capped.push_str("  sink(x);\n}\n");
    for src in [one_node.to_string(), capped] {
        let parsed = parse(Language::JavaScript, &src);
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let (defs, _) = recorded(&parsed, &owner);
        assert!(!defs.is_empty());
        assert_eq!(availability(&parsed, &owner, &defs), (None, None));
        let (completed, facts) = census_defs::complete(&parsed, &owner, defs.clone());
        assert!(completed.len() > defs.len(), "the completed list is longer");
        assert_eq!(facts.n_recorded, defs.len());
    }
}

#[test]
fn other_languages_are_returned_unchanged() {
    let cases = [
        (
            Language::Python,
            "def f(a):\n    log()\n    a = 1\n    sink(a)\n",
        ),
        (
            Language::Go,
            "package p\nfunc f(a int) {\n\tlog()\n\ta++\n\tsink(a)\n}\n",
        ),
        (
            Language::Rust,
            "fn f(mut a: i32) {\n    log();\n    a += 1;\n    sink(a);\n}\n",
        ),
        (
            Language::C,
            "void f(int a) {\n  log();\n  a++;\n  sink(a);\n}\n",
        ),
    ];
    for (language, src) in cases {
        let parsed = ParsedFile::parse("a.x", src, language).unwrap();
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let defs = vec![def(0, "a", 2, 7), def(1, "a", 4, 30)];
        let (out, facts) = census_defs::complete(&parsed, &owner, defs.clone());
        assert_eq!(out, defs, "{language:?}");
        assert!(facts.is_inert(), "{language:?}");
        assert_eq!(facts.n_recorded, 2);
    }
}

#[test]
fn only_bases_the_owner_defines_get_a_kill_only_def() {
    let src = "function f(a) {\n  log();\n  z++;\n  sink(a);\n}\n";
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let defs = vec![def(0, "a", 1, 11)];
    let (out, facts) = census_defs::complete(&parsed, &owner, defs.clone());
    assert_eq!(out, defs, "z is not a def base of this owner (E16)");
    assert!(facts.is_inert());
    // Once the owner defines z, the same write is a kill-only def at its statement line.
    let defs = vec![def(0, "a", 1, 11), def(1, "z", 2, 22)];
    let (out, _) = census_defs::complete(&parsed, &owner, defs);
    assert_eq!(out.len(), 3);
    assert_eq!((out[2].path.to_string(), out[2].line), ("z".to_string(), 3));
}

#[test]
fn coverage_is_the_stored_path_at_the_same_start_byte() {
    // `o!.x = 2` is recorded under another spelling on main (G3): not covered, a kill-only `o.x` is added.
    let wrapped = "function f(o) {\n  o.x = 1;\n  o!.x = 2;\n  sink(o.x);\n}\n";
    let ordinary = "function f(o) {\n  o.x = 1;\n  o.x = 2;\n  sink(o.x);\n}\n";
    let paren = "function f(o) {\n  o.x = 1;\n  (o).x = 2;\n  sink(o.x);\n}\n";
    for (src, language, added) in [
        (wrapped, Language::TypeScript, 1),
        (ordinary, Language::TypeScript, 0),
        (paren, Language::TypeScript, 0),
    ] {
        let parsed = parse(language, src);
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let (defs, _) = recorded(&parsed, &owner);
        let (out, _) = census_defs::complete(&parsed, &owner, defs.clone());
        assert_eq!(out.len() - defs.len(), added, "{src}: {out:?}");
        if added == 1 {
            let k = out.last().unwrap();
            assert_eq!((k.path.to_string(), k.line), ("o.x".to_string(), 3));
        }
    }
}

// ---- E1: kill-only defs never collapse; the header def keeps its edge into the body -----------------

fn labels(src: &str, language: Language, edge: &str) -> Vec<String> {
    crate::cpg::census_label_tests::labels_of(language, src, edge)
}

#[test]
fn a_for_header_increment_does_not_collapse_the_header_def() {
    let src = "function f(n) {\n  for (let i = 0; i < n; i++) {\n    use(i);\n  }\n}\n";
    assert_eq!(labels(src, Language::JavaScript, "i@2->i@3"), ["exact"]);
}

#[test]
fn a_for_increment_kills_an_outer_def_on_every_successor() {
    // The accepted over-kill class (`kill:for-increment`, DESIGN 7.3), pinned so a change shows.
    let src = "function f(n) {\n  let i = 0;\n  for (; i < n; i++) {\n    use(i);\n  }\n}\n";
    assert_eq!(
        labels(src, Language::JavaScript, "i@2->i@4"),
        ["nameonly/killed"]
    );
}

// ---- The trace --------------------------------------------------------------------------------------

fn trace_lines(language: Language, src: &str) -> Vec<String> {
    let parsed = parse(language, src);
    let mut out = Vec::new();
    for owner in parsed.all_functions() {
        let (defs, edges) = recorded(&parsed, &owner);
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        let index = FileCfgEdgeIndex::new(&cfg);
        let traced_outcome =
            census_pass::traced(&parsed, &owner, &defs, &edges, &[], &index, &mut out);
        let plain = reaching_definitions_with_exact(&parsed, &owner, &defs, &edges, &[], &index);
        assert_eq!(traced_outcome, plain, "the trace never changes the outcome");
    }
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

fn mechanisms(language: Language, src: &str) -> Vec<String> {
    trace_lines(language, src)
        .iter()
        .map(|l| l.split('\t').nth(1).unwrap().to_string())
        .collect()
}

#[test]
fn each_census_mechanism_is_named() {
    let js = Language::JavaScript;
    for (language, src, want) in [
        (
            js,
            "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n",
            "kill:update",
        ),
        (
            js,
            "function f(o) {\n  o.x = 1;\n  delete o.x;\n  sink(o.x);\n}\n",
            "kill:delete",
        ),
        (
            js,
            "function f(a) {\n  log();\n  [a = 1] = src();\n  sink(a);\n}\n",
            "kill:pattern",
        ),
        (
            js,
            "function f(a) {\n  log();\n  for ((a) of xs) {}\n  sink(a);\n}\n",
            "kill:for-head",
        ),
        (
            js,
            "function f(n) {\n  let i = 0;\n  for (; i < n; i++) {\n    use(i);\n  }\n}\n",
            "kill:for-increment",
        ),
        (
            js,
            "function f(a) {\n  log();\n  \\u0061++;\n  sink(a);\n}\n",
            "owner-refuse:escape",
        ),
        (
            js,
            "function f(a) {\n  log();\n  a++ +;\n  sink(a);\n}\n",
            "owner-refuse:parse",
        ),
        (
            Language::TypeScript,
            "function f(a) {\n  log();\n  using a = 1;\n  sink(a);\n}\n",
            "owner-refuse:target:using",
        ),
        (
            js,
            "function f(a) {\n  log();\n  class C {\n    static { a++; }\n  }\n  sink(a);\n}\n",
            "site-unmapped",
        ),
    ] {
        let got = mechanisms(language, src);
        assert!(!got.is_empty(), "{want}: no trace for {src}");
        assert!(got.iter().all(|m| m == want), "{want}: {got:?} for {src}");
    }
}

#[test]
fn the_trace_is_silent_where_the_census_decides_nothing() {
    for src in [
        "function f(a) {\n  log();\n  a = 2;\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  z++;\n  sink(a);\n}\n",
        "function f(o) {\n  log();\n  o.x++;\n  sink(o.y);\n}\n",
    ] {
        assert!(trace_lines(Language::JavaScript, src).is_empty(), "{src}");
    }
}

#[test]
fn the_trace_line_has_the_s1a_shape() {
    let lines = trace_lines(
        Language::JavaScript,
        "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n",
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    let f: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(&f[..4], ["rd-census", "kill:update", "a.js", "f"]);
    assert!(f[4].starts_with("a@1:11"), "{lines:?}");
    assert!(f[5].starts_with("a@4:"), "{lines:?}");
}

#[test]
fn comments_in_target_positions_do_not_refuse_the_owner() {
    use crate::js_write_census::{tables::Tables, walk::census_with};
    for body in [
        "const { x, // note\n    y } = src();",
        "[a, /* c */ b] = pair();",
        "for (/* h */ a of xs) {}",
        "for (const /* c */ [k, v] of xs) {}",
        "({ a /* c */ = 1 } = o);",
        "[a = /* c */ 1] = o;",
    ] {
        let src = format!("function f(a) {{\n  log();\n  {body}\n  sink(a);\n}}\n");
        let parsed = parse(Language::JavaScript, &src);
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let c = census_with(&parsed, Tables::shared(), owner, false);
        assert!(c.refusals.is_empty(), "{body}: {:?}", c.refusals);
    }
}

// ---- Round 1 -----------------------------------------------------------------------------------------

fn not_exact(language: Language, src: &str, edge: &str) {
    let got = labels(src, language, edge);
    assert!(
        !got.is_empty() && got.iter().all(|l| l != "exact"),
        "{edge}: {got:?} for {src}"
    );
}

fn exact(language: Language, src: &str, edge: &str) {
    assert_eq!(labels(src, language, edge), ["exact"], "{edge}: {src}");
}

#[test]
fn a_write_lost_behind_another_statement_on_its_line_kills_at_that_line() {
    // W1: `a++` shares line 4 with `log();`, so no retained span holds it; the enclosing `if` (line 2)
    // runs earlier than the recorded `let a` on line 3 and must not take the kill.
    let src =
        "function f(c) {\n  if (c) {\n    let a = 1;\n    log(); a++;\n    sink(a);\n  }\n}\n";
    for language in [Language::JavaScript, Language::TypeScript] {
        not_exact(language, src, "a@3->a@5");
    }
    // An eager class position under the same retained ancestor refuses the base instead.
    let class = "function f(c) {\n  if (c) {\n    let a = 1;\n    class C { static { a++; } }\n    sink(a);\n  }\n}\n";
    not_exact(Language::JavaScript, class, "a@3->a@5");
}

#[test]
fn header_sites_keep_their_header_node() {
    // Case values, `for` heads and the one-line block stay mapped as before (fb03, fb11, fb12).
    let sw = "function f(a, k) {\n  log();\n  switch (k) {\n    case a++: break;\n    default: break;\n  }\n  sink(a);\n}\n";
    not_exact(Language::JavaScript, sw, "a@1->a@7");
}

#[test]
fn eager_decorator_writes_keep_the_owners_this() {
    // W4: parameter decorators, JS method decorators and computed keys run in the enclosing scope.
    let ts = "function f() {\n  this.x = 1;\n  class C { constructor(@dec(this.x++) p: unknown) {} }\n  sink(this.x);\n}\n";
    not_exact(Language::TypeScript, ts, "this.x@2->this.x@4");
    let js =
        "function f() {\n  this.x = 1;\n  class C { @dec(this.x++) m() {} }\n  sink(this.x);\n}\n";
    not_exact(Language::JavaScript, js, "this.x@2->this.x@4");
}

#[test]
fn this_in_class_bodies_stays_the_classes() {
    // The rule the census shares with the producer: a static block or field value rebinds `this`.
    for member in [
        "static { this.x++; }",
        "static y = (this.x++);",
        "static { this.x = 2; }",
        "m() { this.x++; }",
    ] {
        let src = format!(
            "function f() {{\n  this.x = 1;\n  class C {{ {member} }}\n  sink(this.x);\n}}\n"
        );
        exact(Language::JavaScript, &src, "this.x@2->this.x@4");
    }
}

#[test]
fn kills_follow_the_binding() {
    // A distinct inner binding's update leaves the outer edge exact; an unresolved (flat) binding kills.
    let inner = "function f(a) {\n  log();\n  {\n    let a = 0;\n    a++;\n    use(a);\n  }\n  sink(a);\n}\n";
    exact(Language::JavaScript, inner, "a@1->a@8");
    let flat = "function f() {\n  g = 1;\n  log();\n  g++;\n  use(g);\n}\n";
    not_exact(Language::JavaScript, flat, "g@2->g@5");
}

#[test]
fn own_parameter_writes_and_declarations_are_not_kill_only_defs() {
    // S1d's classes (dead formals, hoisting): main's label stands in S1b-1.
    let own = "function f(a, b = a++) {\n  log();\n  use(a);\n}\n";
    exact(Language::JavaScript, own, "a@1->a@3");
    let decl = "function f(a) {\n  log();\n  function a() {}\n  sink(a);\n}\n";
    exact(Language::JavaScript, decl, "a@1->a@4");
}

#[test]
fn affects_is_prefix_with_a_wildcard_on_either_side() {
    // The JS producer records no subscript def, so the def-side `[]` is pinned on the relation itself.
    use super::def_buckets::affects;
    let p = |s: &str| AccessPath::from_expr(s);
    for (w, q, want) in [
        ("o", "o.x", true),
        ("o.x", "o.x", true),
        ("o.x", "o", false),
        ("o.x", "o.y", false),
        ("o.[]", "o.x", true),
        ("o.x", "o.[]", true),
        ("o.a.x", "o.a.[]", true),
        ("o.x", "p.x", false),
    ] {
        let (w, q) = (
            if w.contains("[]") {
                AccessPath::with_fields("o", w.split('.').skip(1).map(String::from).collect())
            } else {
                p(w)
            },
            if q.contains("[]") {
                AccessPath::with_fields(
                    q.split('.').next().unwrap(),
                    q.split('.').skip(1).map(String::from).collect(),
                )
            } else {
                p(q)
            },
        );
        assert_eq!(affects(&w, &q), want, "{w} affects {q}");
    }
}

#[test]
fn an_alias_twin_does_not_cover_a_census_site() {
    let src = "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n";
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let at = src.find("a++").unwrap();
    let mut twin = def(1, "a", 3, at);
    twin.alias_derived = true;
    let defs = vec![def(0, "a", 1, 11), twin];
    let (out, _) = census_defs::complete(&parsed, &owner, defs);
    assert_eq!(
        out.len(),
        3,
        "a twin never kills, so it cannot cover: {out:?}"
    );
}

#[test]
fn availability_refusals_are_traced_and_leave_the_outcome_alone() {
    let mut capped = String::from("function f(x) {\n  x = 0;\n");
    for _ in 0..(RD_MAX_DEFS + 2) {
        capped.push_str("  x++;\n");
    }
    capped.push_str("  sink(x);\n}\n");
    for (src, want) in [
        (
            "const f = (o) => (delete o.x, use(o));\n".to_string(),
            "owner-refuse:one-node",
        ),
        (capped, "owner-refuse:def-cap"),
    ] {
        let got = mechanisms(Language::JavaScript, &src);
        assert!(!got.is_empty(), "{want}: no trace");
        assert!(got.iter().all(|m| m == want), "{want}: {got:?}");
    }
}

#[test]
fn a_grammar_gap_inside_the_owner_refuses_it_by_the_parse_precondition() {
    // The pinned TypeScript grammar does not parse a tuple element labeled with a predefined type name (`symbol:`) as a generic
    // argument (`new Map<number, [symbol: Symbol, ..]>()`); the ERROR node refuses the whole owner, as DESIGN 4.4 intends (TypeScript `services/exportInfoMap.ts`).
    let src = "function f(a) {\n  log();\n  a++;\n  const m = new Map<number, [symbol: Symbol, moduleSymbol: Symbol]>();\n  sink(a);\n}\n";
    let got = mechanisms(Language::TypeScript, src);
    assert!(
        !got.is_empty() && got.iter().all(|m| m == "owner-refuse:parse"),
        "{got:?}"
    );
}
