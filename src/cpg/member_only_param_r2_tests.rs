//! PR #390 round-2 fold: FullFlow through any route into a member-only formal, the write
//! census over a whole receiver chain, `with` refusal, and linear-cost fixtures.

use super::member_only_param_tests::{base_defs, field_edges, graph, render, JS_TS};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

// 1. FullFlow through any route ---------------------------------------------------------

#[test]
fn bounded_backward_walk_stops_at_a_projection_only_def_and_keeps_context() {
    use crate::cpg::CodePropertyGraph;
    let src = "function run(q) {\n h(q);\n}\nfunction h(p) {\n  const a = p.x;\n  const b = p.y;\n  sink(b);\n}\n";
    let files = BTreeMap::from([(
        "m.js".to_string(),
        crate::ast::ParsedFile::parse("m.js", src, Language::JavaScript).unwrap(),
    )]);
    let cpg = CodePropertyGraph::build(&files);
    let use_px = cpg
        .nodes_at("m.js", 5)
        .into_iter()
        .filter_map(|idx| cpg.to_var_location(idx))
        .find(|l| l.path.to_string() == "p.x" && l.kind == crate::data_flow::VarAccessKind::Use)
        .expect("Use(p.x)");
    let (all, seeds) = cpg.dfg_backward_reachable_bounded(&use_px);
    assert_eq!(all, cpg.dfg_backward_reachable(&use_px));
    assert!(all.iter().any(|l| l.line == 4), "the formal Def is reached");
    assert!(
        all.iter().any(|l| l.line == 2),
        "upstream argument context stays in the backward set: {all:?}"
    );
    assert!(
        seeds.is_empty(),
        "no forward seed at or beyond the Def: {seeds:?}"
    );
    // A local reached before any projection-only Def does seed.
    let use_b = cpg
        .nodes_at("m.js", 7)
        .into_iter()
        .filter_map(|idx| cpg.to_var_location(idx))
        .find(|l| l.path.to_string() == "b" && l.kind == crate::data_flow::VarAccessKind::Use)
        .expect("Use(b)");
    let (_, seeds) = cpg.dfg_backward_reachable_bounded(&use_b);
    assert!(seeds.iter().any(|l| l.path.to_string() == "b"), "{seeds:?}");
    assert!(
        seeds.iter().all(|l| l.line >= 5),
        "nothing upstream seeds: {seeds:?}"
    );
}

// 2. Write census over the whole chain --------------------------------------------------

fn edge_paths(language: Language, src: &str) -> BTreeSet<String> {
    let g = graph(language, src);
    field_edges(&g, "p")
        .into_iter()
        .filter(|e| e.0 == "p")
        .map(|e| e.2)
        .collect()
}

#[test]
fn a_write_through_a_wrapper_inside_the_chain_kills_the_whole_path() {
    let js = [
        "(p.x).y = 0;",
        "((p.x)).y = 0;",
        "delete (p.x).y;",
        "(p.x).y++;",
    ];
    let ts = [
        "p.x!.y = 0;",
        "(p.x as any).y = 0;",
        "(p.x satisfies any).y = 0;",
        "delete p.x!.y;",
        "(p.x!).y = 0;",
        "p.x!.y!.w = 0;",
    ];
    let tsx_ok_ts_only = ["(<any>p.x).y = 0;"];
    let mut cases: Vec<(Language, &str)> = Vec::new();
    for w in js {
        cases.extend(JS_TS.map(|l| (l, w)));
    }
    for w in ts {
        cases.extend([Language::TypeScript, Language::Tsx].map(|l| (l, w)));
    }
    for w in tsx_ok_ts_only {
        cases.push((Language::TypeScript, w));
    }
    for (language, write) in cases {
        let src =
            format!("function h(p: any) {{\n  {write}\n  a(p.x);\n  b(p.x.y);\n  c(p.z);\n}}\n");
        let src = if language == Language::JavaScript {
            src.replace("p: any", "p")
        } else {
            src
        };
        let paths = edge_paths(language, &src);
        assert!(
            !paths.contains("p.x") && !paths.contains("p.x.y"),
            "{language:?} `{write}`: stale projection {paths:?}"
        );
        assert!(
            paths.contains("p.z"),
            "{language:?} `{write}`: sibling lost {paths:?}"
        );
    }
}

#[test]
fn wrapped_chain_writes_do_not_kill_unrelated_paths_and_wrapped_reads_project() {
    for language in JS_TS {
        // A write to a sibling chain leaves `p.x` alone.
        let src = render(
            language,
            "function h({P}) {\n  (p.z).y = 0;\n  a(p.x);\n  b(p.z);\n}\n",
            "p",
        );
        let paths = edge_paths(language, &src);
        assert!(paths.contains("p.x"), "{language:?}: {paths:?}");
        assert!(!paths.contains("p.z"), "{language:?}: {paths:?}");
        // A wrapper in the receiver chain of a READ resolves to the full path.
        let src = render(
            language,
            "function h({P}) {\n  sink((p.x).y);\n  sink(p.z);\n}\n",
            "p",
        );
        let g = graph(language, &src);
        let edges = field_edges(&g, "p");
        assert!(
            edges.iter().any(|e| e.2 == "p.x.y" && e.3 == 2),
            "{language:?}: {edges:?}"
        );
        assert!(
            !edges.iter().any(|e| e.2 == "p.x"),
            "{language:?}: the inner member is not its own outermost read: {edges:?}"
        );
    }
    for language in [Language::TypeScript, Language::Tsx] {
        let src = "function h(p: any) {\n  sink(p.x!.y);\n  sink((p.a as any).b);\n}\n";
        let edges = field_edges(&graph(language, src), "p");
        assert!(
            edges.iter().any(|e| e.2 == "p.x.y" && e.3 == 2),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges.iter().any(|e| e.2 == "p.a.b" && e.3 == 3),
            "{language:?}: {edges:?}"
        );
    }
}

// 3. `with` ----------------------------------------------------------------------------

#[test]
fn a_callable_containing_with_is_never_member_only() {
    for source in [
        // sol W2.B: a literal with-object binds `p`; the write must not kill the formal.
        "function h(p) {\n with ({[\"p\"]: {x: 0}}) { p.x = 1; }\n sink(p.x);\n sink(p.y);\n}\n",
        // with inside a nested arrow
        "function h(p) {\n const f = (o) => { with (o) { g(); } };\n sink(p.x);\n}\n",
        // with after every read
        "function h(p) {\n sink(p.x);\n with (o) { g(); }\n}\n",
    ] {
        for language in JS_TS {
            let g = graph(language, source);
            assert!(base_defs(&g, "p").is_empty(), "{language:?}: {source}");
            assert!(
                field_edges(&g, "p").iter().all(|e| e.0 != "p"),
                "{language:?}: {source}"
            );
            let parsed = super::member_only_param_tests::parse(language, source);
            let f = parsed.all_functions()[0];
            assert_eq!(
                parsed.classify_parameter_use(&f, "p"),
                crate::ast::ParameterUse::Unused,
                "{language:?}: {source}"
            );
        }
    }
    // Controls: no `with` anywhere projects; a `with` in a sibling callable does not
    // taint this one; a bare formal stays Bare.
    for language in JS_TS {
        let g = graph(language, "function h(p) {\n sink(p.x);\n sink(p.y);\n}\nfunction k(o) {\n with (o) { g(); }\n}\n");
        let paths = edge_paths(language, "function h(p) {\n sink(p.x);\n sink(p.y);\n}\nfunction k(o) {\n with (o) { g(); }\n}\n");
        assert!(
            paths.contains("p.x") && paths.contains("p.y"),
            "{language:?}: {paths:?}"
        );
        assert_eq!(base_defs(&g, "p").len(), 1);
        let parsed = super::member_only_param_tests::parse(
            language,
            "function h(p) {\n with (o) { use(p); }\n}\n",
        );
        let f = parsed.all_functions()[0];
        assert_eq!(
            parsed.classify_parameter_use(&f, "p"),
            crate::ast::ParameterUse::Bare
        );
    }
}

#[test]
fn a_member_only_callable_nested_in_a_with_body_still_refuses_reads_under_it() {
    // The per-read `with` refusal is reachable (a callable whose own formal is read while
    // the callable itself sits inside a with body), so it stays.
    for language in JS_TS {
        let src = "function outer(o) {\n with (o) {\n  const f = (p) => p.x;\n }\n}\n";
        let paths = edge_paths(language, src);
        assert!(paths.is_empty(), "{language:?}: {paths:?}");
    }
}

// 4. Cost -------------------------------------------------------------------------------

#[test]
fn member_facts_work_is_bounded_by_scopes_not_occurrences() {
    // Cost witness: every scope verdict is computed once per (scope, region), so the
    // number of verdicts is a small constant here while the occurrences grow 8000-fold
    // (an unmemoised walk computes one per occurrence). Deterministic, unlike a timer.
    for with_default in [false, true] {
        let mut verdicts = Vec::new();
        for n in [10usize, 1_000, 8_000] {
            let mut src = format!(
                "function h(p{}) {{\n",
                if with_default { ", q = 1" } else { "" }
            );
            for i in 0..n {
                src.push_str(&format!("  use(p.f{i});\n"));
            }
            src.push_str("}\n");
            let parsed = super::member_only_param_tests::parse(Language::JavaScript, &src);
            let f = parsed.all_functions()[0];
            let (_, def_byte, _) = parsed.function_parameter_occurrences(&f)[0].clone();
            let facts = parsed.formal_member_facts(&f, "p", def_byte);
            assert_eq!(facts.outermost.len(), n);
            verdicts.push(facts.scope_verdicts);
        }
        assert!(
            verdicts.iter().all(|v| (1..=8).contains(v)) && verdicts[1] == verdicts[2],
            "default={with_default}: verdicts {verdicts:?}"
        );
    }
}
