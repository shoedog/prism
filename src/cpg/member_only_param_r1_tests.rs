//! PR #390 round-1 fold: write census, escaped identifiers, exact-span Uses, `with`
//! bodies, erased type positions, declaration-line reads, Bare classification of reads
//! inside a member object, deeper-write prefix kills, and the FullFlow composition leak.

use super::member_only_param_tests::{
    base_defs, edge, field_edges, for_each_language, graph, parse, render, JS_TS,
};
use crate::ast::ParameterUse;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

fn has_edge_to(g: &DataFlowGraph, path: &str, line: usize) -> bool {
    field_edges(g, "p")
        .iter()
        .any(|e| e.0 == "p" && e.2 == path && e.3 == line)
}

fn slice_lines(
    language: Language,
    source: &str,
    file: &str,
    diff_lines: &[usize],
    algorithm: SlicingAlgorithm,
) -> BTreeSet<usize> {
    let files = BTreeMap::from([(
        file.to_string(),
        crate::ast::ParsedFile::parse(file, source, language).unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: file.into(),
            modify_type: ModifyType::Modified,
            diff_lines: diff_lines.iter().copied().collect(),
        }],
    };
    let result = crate::algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(algorithm),
        None,
    )
    .unwrap();
    result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.get(file).into_iter().flatten())
        .map(|(line, _)| *line)
        .collect()
}

// 1. FullFlow sibling leak -------------------------------------------------------------

const CARD: &str = "export function Card(props: CardProps) {\n  const title = props.title.trim().toUpperCase();\n  const html = props.rawHtml;\n  return (\n    <div dangerouslySetInnerHTML={{ __html: html }}>\n      {title}\n    </div>\n  );\n}\n";

#[test]
fn fullflow_diff_on_a_field_read_never_pulls_in_a_sibling_field() {
    // Main (no parameter Def at all) gives exactly [1,2,4,6,9] for a diff on line 2.
    let lines = slice_lines(
        Language::Tsx,
        CARD,
        "c.tsx",
        &[2],
        SlicingAlgorithm::FullFlow,
    );
    assert_eq!(lines, BTreeSet::from([1, 2, 4, 6, 9]), "{lines:?}");

    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  const a = p.x;\n  sinkB(p.y);\n  const html = p.z;\n  sinkC(html);\n}\n",
            "p",
        );
        let file = super::member_only_param_tests::file_name(language);
        let lines = slice_lines(language, &src, file, &[2], SlicingAlgorithm::FullFlow);
        assert!(
            !lines.contains(&3) && !lines.contains(&4) && !lines.contains(&5),
            "{language:?}: sibling fields leaked into {lines:?}"
        );
    }
}

#[test]
fn fullflow_diff_on_the_parameter_line_still_fans_out() {
    // Accepted cost: a seed on the whole parameter reaches every field read.
    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  a(p.x);\n  b(p.y);\n}\n",
            "p",
        );
        let file = super::member_only_param_tests::file_name(language);
        let lines = slice_lines(language, &src, file, &[1], SlicingAlgorithm::FullFlow);
        let left = slice_lines(language, &src, file, &[1], SlicingAlgorithm::LeftFlow);
        assert!(
            (lines.contains(&2) && lines.contains(&3)) || (left.contains(&2) && left.contains(&3)),
            "{language:?}: full {lines:?} left {left:?}"
        );
    }
}

#[test]
fn leftflow_condition_variable_slice_is_pinned() {
    // LeftFlow's condition walk stops at the projection-only base Def and keeps the
    // pre-#390 name fallback (every `p` line); pin that it does not shrink or grow.
    // Sibling-derived locals: member_only_param_leftflow_tests.
    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  if (p.x) {\n    a();\n  }\n  b(p.y);\n}\n",
            "p",
        );
        let file = super::member_only_param_tests::file_name(language);
        let lines = slice_lines(language, &src, file, &[2], SlicingAlgorithm::LeftFlow);
        assert!(
            lines.contains(&2) && lines.contains(&5),
            "{language:?}: {lines:?}"
        );
    }
}

// 2. Write census ----------------------------------------------------------------------

#[test]
fn every_member_write_form_suppresses_its_path_but_not_a_sibling() {
    let writes = [
        "p.x++;",
        "p.x--;",
        "++p.x;",
        "delete p.x;",
        "delete p?.x;",
        "[p.x] = [0];",
        "[...p.x] = arr;",
        "[[p.x]] = arr;",
        "[{ a: p.x }] = arr;",
        "({ a: [p.x] } = o);",
        "({ ...p.x } = o);",
        "for (p.x of arr) {}",
        "for (p.x in o) {}",
        "const f = () => { p.x++; };",
        "const f = () => { delete p.x; };",
        "const f = () => { [p.x] = arr; };",
        "const f = () => { for (p.x of arr) {} };",
    ];
    for write in writes {
        let template =
            format!("function h({{P}}, arr, o) {{\n  {write}\n  a(p.x);\n  b(p.y);\n}}\n");
        for language in JS_TS {
            let src = render(language, &template, "p");
            let g = graph(language, &src);
            assert!(
                !field_edges(&g, "p")
                    .iter()
                    .any(|e| e.0 == "p" && e.2 == "p.x"),
                "{language:?} `{write}`: written path must not be projected: {:?}",
                field_edges(&g, "p")
            );
            assert!(
                has_edge_to(&g, "p.y", 4),
                "{language:?} `{write}`: sibling field keeps its edge: {:?}",
                field_edges(&g, "p")
            );
        }
    }
}

#[test]
fn writes_to_another_base_or_a_longer_sibling_do_not_suppress() {
    for write in [
        "q.x++;",
        "delete q.x;",
        "[q.x] = arr;",
        "p.xy++;",
        "for (q.x of arr) {}",
    ] {
        let template = format!("function h({{P}}, q, arr) {{\n  {write}\n  a(p.x);\n}}\n");
        for language in JS_TS {
            let src = render(language, &template, "p");
            let g = graph(language, &src);
            assert!(has_edge_to(&g, "p.x", 3), "{language:?} `{write}`");
        }
    }
}

// 3. Escaped identifiers fail closed ---------------------------------------------------

#[test]
fn a_callable_with_an_escaped_identifier_gets_no_member_only_def() {
    for source in [
        // sol W2: block-local `p` shadows the formal for the first read only.
        "function handler(p) {\n { let \\u0070 = o; sink(p.x); } sink(p.x);\n}\n",
        // sol W1 inverse: `p` is the nested formal, the write targets it.
        "function handler(p) {\n register((\\u0070) => { p.x = 0; });\n sink(p.x);\n sink(p.y);\n}\n",
        // an unrelated escaped name anywhere in the callable
        "function handler(p) {\n var \\u0061 = 1;\n sink(p.x);\n}\n",
    ] {
        for language in JS_TS {
            let g = graph(language, source);
            assert!(base_defs(&g, "p").is_empty(), "{language:?}: {source}");
            assert!(field_edges(&g, "p").is_empty(), "{language:?}: {source}");
            let parsed = parse(language, source);
            let f = parsed.all_functions()[0];
            assert_eq!(
                parsed.classify_parameter_use(&f, "p"),
                ParameterUse::Unused,
                "{language:?}: {source}"
            );
        }
    }
    // Control: the same shapes without an escape still project.
    for language in JS_TS {
        let g = graph(
            language,
            "function handler(p) {\n register((q) => { q.x = 0; });\n sink(p.x);\n sink(p.y);\n}\n",
        );
        assert!(
            has_edge_to(&g, "p.x", 3) && has_edge_to(&g, "p.y", 4),
            "{language:?}"
        );
    }
}

// 4. Exact-span Uses -------------------------------------------------------------------

#[test]
fn named_owner_projection_use_sits_at_the_exact_admitted_span() {
    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  sink(p.x, p.x);\n  sink(p.y);\n}\n",
            "p",
        );
        let g = graph(language, &src);
        let want = src.find("p.x").unwrap();
        let to: Vec<_> = g
            .edges
            .iter()
            .filter(|e| e.from.path.to_string() == "p" && e.to.path.to_string() == "p.x")
            .map(|e| (e.to.start_byte, e.to.end_byte, e.to.line))
            .collect();
        assert_eq!(to, vec![(want, want + 3, 2)], "{language:?}: {to:?}");
    }
}

// 5. `with` bodies ----------------------------------------------------------------------

#[test]
fn with_body_reads_are_never_projected_for_named_owners() {
    // Round 2: a callable containing any `with` is never member-only, so the outside
    // read `p.y` is no longer projected either (documented narrowing, known-gaps 6).
    for language in JS_TS {
        let src =
            "function handler(p) {\n with ({[\"p\"]: {x: 0}}) {\n sink(p.x);\n }\n sink(p.y);\n}\n";
        let g = graph(language, src);
        assert!(
            field_edges(&g, "p").iter().all(|e| e.0 != "p"),
            "{language:?}: {:?}",
            field_edges(&g, "p")
        );
        assert!(base_defs(&g, "p").is_empty(), "{language:?}");
    }
}

// 6. Erased type positions -------------------------------------------------------------

#[test]
fn erased_member_type_queries_are_not_reads() {
    for language in [Language::TypeScript, Language::Tsx] {
        for body in [
            "type T = typeof p.x;",
            "let v: typeof p.f = mk();",
            "interface T { x: typeof p.x }",
        ] {
            let src = format!("function handler(p: any) {{\n {body}\n}}\n");
            let parsed = parse(language, &src);
            let f = parsed.all_functions()[0];
            assert_ne!(
                parsed.classify_parameter_use(&f, "p"),
                ParameterUse::MemberOnly,
                "{language:?} `{body}`"
            );
            let g = graph(language, &src);
            assert!(base_defs(&g, "p").is_empty(), "{language:?} `{body}`");
            assert!(field_edges(&g, "p").is_empty(), "{language:?} `{body}`");

            // Mixed with an evaluated read: only the evaluated read is projected.
            let src = format!("function handler(p: any) {{\n {body}\n sink(p.y);\n}}\n");
            let g = graph(language, &src);
            assert_eq!(
                field_edges(&g, "p"),
                BTreeSet::from([edge("p", 1, "p.y", 3)]),
                "{language:?} `{body}`"
            );
        }
        // Executable `typeof p.x` stays a read.
        let src = "function handler(p: any) {\n sink(typeof p.x);\n}\n";
        let g = graph(language, src);
        assert!(has_edge_to(&g, "p.x", 2), "{language:?}");
    }
    let g = graph(
        Language::JavaScript,
        "function handler(p) {\n sink(typeof p.x);\n}\n",
    );
    assert!(has_edge_to(&g, "p.x", 2));
}

// 7. First-line and default reads ------------------------------------------------------

#[test]
fn declaration_line_body_reads_are_projected_and_defaults_are_not() {
    for_each_language("function h({P}) { sink(p.x); }\n", |language, g, _| {
        assert_eq!(
            field_edges(g, "p"),
            BTreeSet::from([edge("p", 1, "p.x", 1)]),
            "{language:?}"
        );
    });
    for_each_language("const h = ({P}) => p.x;\n", |language, g, _| {
        assert_eq!(
            field_edges(g, "p"),
            BTreeSet::from([edge("p", 1, "p.x", 1)]),
            "{language:?}"
        );
    });
    for_each_language(
        "function h({P}) { sink(p.x);\n  sink(p.y);\n}\n",
        |language, g, _| {
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 1, "p.x", 1), edge("p", 1, "p.y", 2)]),
                "{language:?}"
            );
        },
    );
    // Anonymous callback owner on one line.
    for_each_language("app.get('/', ({P}) => p.x);\n", |language, g, _| {
        assert_eq!(
            field_edges(g, "p"),
            BTreeSet::from([edge("p", 1, "p.x", 1)]),
            "{language:?}"
        );
    });
    // A default expression is excluded by byte span even when the same path is read in the
    // body, and regardless of layout (documented: a default reading another formal).
    for_each_language(
        "function h({P},\n  q = p.x) {\n  sink(p.x);\n  sink(p.y);\n}\n",
        |language, g, _| {
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 1, "p.x", 3), edge("p", 1, "p.y", 4)]),
                "{language:?}"
            );
        },
    );
    for_each_language(
        "function h({P},\n  q = p.x) {\n  sink(p.y);\n}\n",
        |language, g, _| {
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 1, "p.y", 3)]),
                "{language:?}"
            );
        },
    );
}

// 8. Bare read inside a member object --------------------------------------------------

#[test]
fn a_bare_read_inside_a_member_object_makes_the_formal_bare() {
    for (source, label) in [
        ("function h(p) {\n  return wrap(p).x + p.y;\n}\n", "call"),
        ("function h(p) {\n  return [p][0].x + p.y;\n}\n", "array"),
    ] {
        for language in JS_TS {
            let parsed = parse(language, source);
            let f = parsed.all_functions()[0];
            assert_eq!(
                parsed.classify_parameter_use(&f, "p"),
                ParameterUse::Bare,
                "{language:?} {label}"
            );
            let g = graph(language, source);
            assert!(field_edges(&g, "p").is_empty(), "{language:?} {label}");
        }
    }
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "function h(p: any) {\n  return (p as any).i + p.y;\n}\n",
            "function h(p: any) {\n  return p!.j + p.y;\n}\n",
        ] {
            let parsed = parse(language, source);
            let f = parsed.all_functions()[0];
            assert_eq!(
                parsed.classify_parameter_use(&f, "p"),
                ParameterUse::Bare,
                "{language:?} {source}"
            );
        }
    }
    // Control: a pure member chain stays member-only.
    for language in JS_TS {
        let parsed = parse(language, "function h(p) {\n  return p.a.b + p.y;\n}\n");
        let f = parsed.all_functions()[0];
        assert_eq!(
            parsed.classify_parameter_use(&f, "p"),
            ParameterUse::MemberOnly,
            "{language:?}"
        );
    }
}

// 9. Prefix read after a deeper write --------------------------------------------------

#[test]
fn a_deeper_write_also_suppresses_the_prefix_path() {
    for_each_language(
        "function h({P}) {\n  p.x.y = 1;\n  use(p.x);\n  use(p.x.y);\n  use(p.z);\n  use(p.xy);\n}\n",
        |language, g, _| {
            let edges = field_edges(g, "p");
            assert!(
                !edges
                    .iter()
                    .any(|e| e.0 == "p" && (e.2 == "p.x" || e.2 == "p.x.y")),
                "{language:?}: {edges:?}"
            );
            assert!(has_edge_to(g, "p.z", 5), "{language:?}: {edges:?}");
            assert!(has_edge_to(g, "p.xy", 6), "{language:?}: {edges:?}");
        },
    );
}

// Mutation-lane obligations: write-target shapes, facts-level binding proofs, predicate --

#[test]
fn ts_wrapped_member_write_targets_and_read_shaped_patterns() {
    for language in [Language::TypeScript, Language::Tsx] {
        for write in [
            "p.x! = 1;",
            "(p.x) = 1;",
            "(p.x as any) = 1;",
            "[p.x = 5] = arr;",
        ] {
            let src =
                format!("function h(p: any, arr: any) {{\n  {write}\n  a(p.x);\n  b(p.y);\n}}\n");
            let g = graph(language, &src);
            assert!(
                !field_edges(&g, "p")
                    .iter()
                    .any(|e| e.0 == "p" && e.2 == "p.x"),
                "{language:?} `{write}`: {:?}",
                field_edges(&g, "p")
            );
            assert!(has_edge_to(&g, "p.y", 4), "{language:?} `{write}`");
        }
    }
    // Default values and computed keys inside a pattern are reads, not write targets.
    for read in [
        "[q = p.x] = arr;",
        "({ [p.x]: q } = o);",
        "({ a: q = p.x } = o);",
    ] {
        let template = format!("function h({{P}}, arr, o, q) {{\n  {read}\n  b(p.y);\n}}\n");
        for language in JS_TS {
            let src = render(language, &template, "p");
            let g = graph(language, &src);
            assert!(
                has_edge_to(&g, "p.x", 2),
                "{language:?} `{read}`: {:?}",
                field_edges(&g, "p")
            );
        }
    }
    // A spread read is not a write.
    for_each_language(
        "function h({P}, f) {\n  f(...p.x);\n  b(p.y);\n}\n",
        |language, g, _| {
            assert!(has_edge_to(g, "p.x", 2), "{language:?}");
        },
    );
}

fn facts_of(
    language: Language,
    source: &str,
) -> (crate::ast::FormalMemberFacts, Vec<(usize, usize)>) {
    let parsed = parse(language, source);
    let f = parsed.all_functions()[0];
    let (_, def_byte, _) = parsed.function_parameter_occurrences(&f)[0].clone();
    let facts = parsed.formal_member_facts(&f, "p", def_byte);
    let spans = facts
        .reads
        .values()
        .flat_map(|by_line| by_line.values().copied())
        .collect();
    (facts, spans)
}

#[test]
fn facts_use_the_exact_unshadowed_occurrence_on_a_shared_line() {
    for language in JS_TS {
        // A block-local `p` shadows the first read only; both reads share line 2.
        let src = "function h(p) {\n  { let p = o; sink(p.x); } sink(p.x);\n}\n";
        let (facts, spans) = facts_of(language, src);
        let outer = src.rfind("p.x").unwrap();
        assert_eq!(spans, vec![(outer, outer + 3)], "{language:?}");
        assert_eq!(
            facts.reads[&crate::access_path::AccessPath::from_expr("p.x")].len(),
            1
        );
        // Moved-line control: the same outer read on its own line.
        let src = "function h(p) {\n  { let p = o; sink(p.x); }\n  sink(p.x);\n}\n";
        let (_, spans) = facts_of(language, src);
        let outer = src.rfind("p.x").unwrap();
        assert_eq!(spans, vec![(outer, outer + 3)], "{language:?}");
        // Two unshadowed reads on one line keep the first.
        let src = "function h(p) {\n  sink(p.x, p.x);\n}\n";
        let (_, spans) = facts_of(language, src);
        let first = src.find("p.x").unwrap();
        assert_eq!(spans, vec![(first, first + 3)], "{language:?}");
    }
}

#[test]
fn facts_count_a_write_only_when_it_binds_the_formal() {
    use crate::access_path::AccessPath;
    let px = AccessPath::from_expr("p.x");
    for language in JS_TS {
        // The write targets a nested callable's own `p`: no kill.
        let (facts, _) = facts_of(
            language,
            "function h(p) {\n  const f = (p) => { p.x = 0; };\n  sink(p.x);\n}\n",
        );
        assert!(!facts.is_killed(&px), "{language:?}");
        // Control: a captured write to the same formal kills.
        let (facts, _) = facts_of(
            language,
            "function h(p) {\n  const f = () => { p.x = 0; };\n  sink(p.x);\n}\n",
        );
        assert!(facts.is_killed(&px), "{language:?}");
        // A write below the path kills its prefixes; a longer sibling name does not.
        let (facts, _) = facts_of(
            language,
            "function h(p) {\n  p.x.y = 0;\n  sink(p.xy);\n}\n",
        );
        assert!(facts.is_killed(&px));
        assert!(!facts.is_killed(&AccessPath::from_expr("p.xy")));
        assert!(!facts.is_killed(&AccessPath::from_expr("p.z")));
    }
}

#[test]
fn refused_formals_in_synthetic_owners_are_never_projected() {
    // `eval` and `arguments` make the formal's occurrences unprovable in a callback pass.
    for language in JS_TS {
        for body in ["eval(s);", "sink(arguments[0]);"] {
            let src = format!("app.get('/', (p) => {{\n  {body}\n  sink(p.x);\n}});\n");
            let g = graph(language, &src);
            assert!(
                !field_edges(&g, "p").iter().any(|e| e.0 == "p"),
                "{language:?} `{body}`: {:?}",
                field_edges(&g, "p")
            );
        }
        let src = "app.get('/', (p) => {\n  sink(p.x);\n});\n";
        let g = graph(language, src);
        assert!(has_edge_to(&g, "p.x", 2), "{language:?}");
    }
}

#[test]
fn projection_only_def_predicate_separates_member_only_bare_and_local_defs() {
    use crate::cpg::CodePropertyGraph;
    let defs = |source: &str| {
        let files = BTreeMap::from([(
            "m.js".to_string(),
            crate::ast::ParsedFile::parse("m.js", source, Language::JavaScript).unwrap(),
        )]);
        let cpg = CodePropertyGraph::build(&files);
        let verdicts: BTreeMap<String, bool> = cpg
            .dfg
            .defs
            .values()
            .flatten()
            .filter(|d| !d.path.has_fields())
            .map(|d| (d.path.base.clone(), cpg.dfg_is_projection_only_def(d)))
            .collect();
        verdicts
    };
    let v =
        defs("function h(p, q) {\n  const l = 1;\n  use(l);\n  use(q);\n  return p.x + p.y;\n}\n");
    assert_eq!(v.get("p"), Some(&true), "{v:?}");
    assert_eq!(v.get("q"), Some(&false), "{v:?}");
    assert_eq!(v.get("l"), Some(&false), "{v:?}");
}

#[test]
fn non_js_member_object_reads_are_not_descended() {
    // Only JS/TS descend into a member object for bare reads; other languages keep the old
    // classification (no Def for a formal that is only reached through a member).
    for (language, file, source) in [
        (
            Language::Python,
            "m.py",
            "def h(p):\n    return wrap(p).x\n",
        ),
        (
            Language::Go,
            "m.go",
            "package m\nfunc h(p T) int {\n\treturn wrap(p).x\n}\n",
        ),
    ] {
        let parsed = crate::ast::ParsedFile::parse(file, source, language).unwrap();
        let f = parsed.all_functions()[0];
        assert_eq!(
            parsed.classify_parameter_use(&f, "p"),
            ParameterUse::Unused,
            "{language:?}"
        );
    }
}
