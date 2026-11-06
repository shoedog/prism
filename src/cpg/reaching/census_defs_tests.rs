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
fn availability_agrees_at_both_seams_on_every_matrix_fixture() {
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

#[test]
fn completed_lists_are_equal_at_both_seams_cold_and_warm() {
    for (name, _slice, src) in fixtures() {
        let src = src.replace("\\n", "\n");
        let language = lang_of(&name);
        let parsed = parse(language, &src);
        for owner in parsed.all_functions() {
            let (defs, _) = recorded(&parsed, &owner);
            let first = census_defs::complete(&parsed, &owner, defs.clone());
            let second = census_defs::complete(&parsed, &owner, defs.clone());
            let cold = parse(language, &src);
            let cold_owner = cold
                .all_functions()
                .into_iter()
                .find(|o| o.start_byte() == owner.start_byte() && o.end_byte() == owner.end_byte())
                .unwrap();
            let fresh = census_defs::complete(&cold, &cold_owner, defs.clone());
            assert_eq!(first.0, second.0, "{name}: seam 1 vs seam 2");
            assert_eq!(first.0, fresh.0, "{name}: warm memo vs cold walk");
            assert_eq!(first.1.n_recorded, defs.len());
        }
    }
}

/// A recorded def of `base` at `byte` on `line`, for tests that need no producer.
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
fn census_only_availability_flip_is_the_same_at_both_seams() {
    // One node: `delete o.x` is a census write of `o`, which the producer records no def for. The
    // completed list holds two defs of `o`, so the one-node distinct-bases belt refuses the owner.
    let src = "const f = (o) => (delete o.x, use(o));\n";
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let defs = vec![def(0, "o", 1, src.find("(o)").unwrap() + 1)];
    let (completed, facts) = census_defs::complete(&parsed, &owner, defs.clone());
    assert_eq!(completed.len(), 2, "{completed:?}");
    assert_eq!(facts.n_recorded, 1);
    assert_eq!(
        availability(&parsed, &owner, &defs),
        (
            Some(RdUnavailable::NoCfgEdges),
            Some(RdUnavailable::NoCfgEdges)
        )
    );
    // The same owner without the write is available at both seams.
    let plain = "const f = (o) => (touch(o.x), use(o));\n";
    let parsed = parse(Language::JavaScript, plain);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let defs = vec![def(0, "o", 1, plain.find("(o)").unwrap() + 1)];
    assert_eq!(availability(&parsed, &owner, &defs), (None, None));
}

#[test]
fn census_defs_count_toward_the_def_cap_at_both_seams() {
    let mut body = String::from("function f(o) {\n");
    for i in 0..(RD_MAX_DEFS + 2) {
        body.push_str(&format!("  delete o.k{i};\n"));
    }
    body.push_str("  use(o);\n}\n");
    let parsed = parse(Language::JavaScript, &body);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let defs = vec![def(0, "o", 1, body.find("(o)").unwrap() + 1)];
    let (pre, solver) = availability(&parsed, &owner, &defs);
    assert!(
        matches!(pre, Some(RdUnavailable::DefinitionsCapExceeded { .. })),
        "{pre:?}"
    );
    assert_eq!(pre, solver);
    // Recorded defs alone are under the cap: only the census pushes the owner over it.
    assert_eq!(defs.len(), 1);
}

/// One-node availability over `defs` as the census-free rule states it (the control).
fn census_off_availability(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
) -> Option<RdUnavailable> {
    let bases: Vec<&str> = defs.iter().map(|d| d.path.base.as_str()).collect();
    (!crate::cfg::js_single::single_node_cfg_is_complete(parsed, owner, 1, &bases))
        .then_some(RdUnavailable::NoCfgEdges)
}

#[test]
fn label_only_refusals_never_move_availability() {
    // `delete o.x` alone flips a one-node owner (above). Under an escape, a parse error or a refused
    // target kind the census is refused, adds no def, and availability is exactly the census-free rule.
    let flips = "const f = (o) => (delete o.x, g(o, b));\n";
    for (src, language, refused) in [
        (flips, Language::JavaScript, false),
        (
            "const f = (o) => (delete o.x, g(o, \\u0062));\n",
            Language::JavaScript,
            true,
        ),
        (
            "const f = (o) => (delete o.x, g(o, @));\n",
            Language::JavaScript,
            true,
        ),
        (
            "const f = (o) => (delete o.x, (using a = 1));\n",
            Language::TypeScript,
            true,
        ),
    ] {
        let parsed = parse(language, src);
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let defs = vec![def(0, "o", 1, src.find("(o)").unwrap() + 1)];
        let (completed, facts) = census_defs::complete(&parsed, &owner, defs.clone());
        let off = census_off_availability(&parsed, &owner, &defs);
        assert_eq!(off, None, "{src}: the census-free rule admits the owner");
        let (pre, solver) = availability(&parsed, &owner, &defs);
        assert_eq!(pre, solver, "{src}");
        if refused {
            assert_eq!(completed, defs, "{src}: a refusal adds no kill-only def");
            assert!(!facts.is_inert(), "{src}: the refusal is recorded");
            assert_eq!(
                pre, off,
                "{src}: availability untouched by a label-only refusal"
            );
        } else {
            assert_eq!(completed.len(), 2, "{src}");
            assert_eq!(
                pre,
                Some(RdUnavailable::NoCfgEdges),
                "{src}: the census-only flip"
            );
        }
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

#[test]
fn a_changed_recorded_list_recomputes_the_memoised_completion() {
    let src = "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n";
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let bare = vec![def(0, "a", 1, 11)];
    let (out, _) = census_defs::complete(&parsed, &owner, bare.clone());
    assert_eq!(out.len(), 2);
    // A recorded def at the site's own start byte covers it: the stale kill-only def must be gone.
    let at = src.find("a++").unwrap();
    let covered = vec![def(0, "a", 1, 11), def(1, "a", 3, at)];
    let (out, _) = census_defs::complete(&parsed, &owner, covered.clone());
    assert_eq!(out, covered);
    let (again, _) = census_defs::complete(&parsed, &owner, bare);
    assert_eq!(again.len(), 2);
}

#[test]
fn the_walk_runs_once_per_owner_across_both_seams() {
    let src = "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n";
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let (defs, _) = recorded(&parsed, &owner);
    let before = crate::js_write_census::completion::walks();
    let cfg = cfg::build_cfg_edges_with_arms(&parsed);
    let index = FileCfgEdgeIndex::new(&cfg);
    let _ = unavailable_before_edges(&parsed, &owner, &defs, &index);
    let _ = reaching_definitions_with_exact(&parsed, &owner, &defs, &[], &[], &index);
    assert_eq!(crate::js_write_census::completion::walks() - before, 1);
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
        let traced_outcome = traced(&parsed, &owner, &defs, &edges, &[], &index, &mut out);
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
