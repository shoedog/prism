//! PR-C: a JS/TS/TSX formal read only through static member paths (`p.x`) gets a
//! base Def at its identifier plus `Def(p) -> Use(p.<path>)` projection edges.
//! Edges start only at the base Def; a field never reaches a sibling field.

use crate::ast::{ParameterUse, ParsedFile};
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const JS_TS: [Language; 3] = [Language::JavaScript, Language::TypeScript, Language::Tsx];

pub(super) fn file_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript => "m.js",
        Language::TypeScript => "m.ts",
        Language::Tsx => "m.tsx",
        Language::Python => "m.py",
        Language::Go => "m.go",
        Language::C => "m.c",
        _ => unreachable!(),
    }
}

/// `{P}` in a template is the formal; TS/TSX get a type annotation.
pub(super) fn render(language: Language, template: &str, formal: &str) -> String {
    let formal = match language {
        Language::JavaScript => formal.to_string(),
        _ => format!("{formal}: any"),
    };
    template.replace("{P}", &formal)
}

pub(super) fn parse(language: Language, source: &str) -> ParsedFile {
    ParsedFile::parse(file_name(language), source, language).unwrap()
}

pub(super) fn graph(language: Language, source: &str) -> DataFlowGraph {
    let parsed = parse(language, source);
    DataFlowGraph::build(&BTreeMap::from([(file_name(language).to_string(), parsed)]))
}

/// (function, path, line) of every Def whose path base is `base` and which has no fields.
pub(super) fn base_defs(g: &DataFlowGraph, base: &str) -> BTreeSet<(String, usize)> {
    g.defs
        .values()
        .flatten()
        .filter(|d| d.path.base == base && !d.path.has_fields())
        .map(|d| (d.function.clone(), d.line))
        .collect()
}

/// (from path, from line, to path, to line) of every edge ending in a field path of `base`.
pub(super) fn field_edges(
    g: &DataFlowGraph,
    base: &str,
) -> BTreeSet<(String, usize, String, usize)> {
    g.edges
        .iter()
        .filter(|e| e.to.path.base == base && e.to.path.has_fields())
        .map(|e| {
            (
                e.from.path.to_string(),
                e.from.line,
                e.to.path.to_string(),
                e.to.line,
            )
        })
        .collect()
}

pub(super) fn edge(from: &str, fl: usize, to: &str, tl: usize) -> (String, usize, String, usize) {
    (from.into(), fl, to.into(), tl)
}

pub(super) fn for_each_language(template: &str, check: impl Fn(Language, &DataFlowGraph, &str)) {
    for language in JS_TS {
        let source = render(language, template, "p");
        let g = graph(language, &source);
        check(language, &g, &source);
    }
}

#[test]
fn plain_member_only_formal_gets_a_base_def_and_projection_edges() {
    for_each_language(
        "function h({P}) {\n  a(p.x);\n  b(p.y);\n}\n",
        |language, g, _| {
            assert_eq!(
                base_defs(g, "p"),
                BTreeSet::from([("h".to_string(), 1)]),
                "{language:?}"
            );
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 1, "p.x", 2), edge("p", 1, "p.y", 3)]),
                "{language:?}: edges start only at the base Def, never field->field"
            );
        },
    );
    // Negative: a formal that is never read gets no Def.
    for_each_language("function h({P}) {\n  return 1;\n}\n", |language, g, _| {
        assert!(base_defs(g, "p").is_empty(), "{language:?}");
        assert!(field_edges(g, "p").is_empty(), "{language:?}");
    });
}

#[test]
fn arrow_expression_body_and_method_member_only_formals() {
    for_each_language("const h = ({P}) => p.x;\n", |language, g, _| {
        assert_eq!(
            base_defs(g, "p").len(),
            1,
            "{language:?}: arrow expression body"
        );
    });
    for_each_language(
        "class C {\n  m({P}) {\n    return p.x;\n  }\n}\n",
        |language, g, _| {
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 2, "p.x", 3)]),
                "{language:?}: method"
            );
        },
    );
}

#[test]
fn optional_chaining_member_read_is_projected() {
    for_each_language("function h({P}) {\n  a(p?.x);\n}\n", |language, g, _| {
        assert_eq!(
            field_edges(g, "p"),
            BTreeSet::from([edge("p", 1, "p.x", 2)]),
            "{language:?}"
        );
    });
    // Negative: optional chaining does not conjure other fields.
    for_each_language("function h({P}) {\n  a(p?.x);\n}\n", |language, g, _| {
        assert!(
            !field_edges(g, "p").iter().any(|e| e.2 != "p.x"),
            "{language:?}"
        );
    });
}

#[test]
fn nested_path_with_a_prefix_read_projects_each_outermost_path() {
    for_each_language(
        "function h({P}) {\n  a(p.a.b);\n  c(p.a);\n}\n",
        |language, g, _| {
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([
                    edge("p", 1, "p.a.b", 2),
                    edge("p", 1, "p.a", 2),
                    edge("p", 1, "p.a", 3)
                ]),
                "{language:?}: the `p.a` read also matches as a prefix inside `p.a.b`"
            );
        },
    );
    // Only the outermost path of a read is collected: with no `p.a` read, nothing projects `p.a`.
    for_each_language("function h({P}) {\n  a(p.a.b);\n}\n", |language, g, _| {
        assert_eq!(
            field_edges(g, "p"),
            BTreeSet::from([edge("p", 1, "p.a.b", 2)]),
            "{language:?}"
        );
    });
}

#[test]
fn path_write_suppresses_that_path_but_not_its_siblings() {
    for_each_language(
        "function h({P}) {\n  p.x = 1;\n  a(p.x);\n  b(p.y);\n}\n",
        |language, g, _| {
            let edges = field_edges(g, "p");
            assert!(
                !edges.iter().any(|e| e.0 == "p" && e.1 == 1 && e.2 == "p.x"),
                "{language:?}: the written path is not projected from the parameter: {edges:?}"
            );
            assert!(
                edges.contains(&edge("p", 1, "p.y", 4)),
                "{language:?}: {edges:?}"
            );
        },
    );
    // A write to a deeper path suppresses that path and its prefixes (the deeper write may
    // change what the prefix holds), but not an unrelated sibling.
    for_each_language(
        "function h({P}) {\n  p.a.b = 1;\n  c(p.a.b);\n  d(p.a);\n  e(p.c);\n}\n",
        |language, g, _| {
            let edges = field_edges(g, "p");
            assert!(
                !edges
                    .iter()
                    .any(|e| e.0 == "p" && e.1 == 1 && e.2 == "p.a.b"),
                "{language:?}: {edges:?}"
            );
            assert!(
                !edges.iter().any(|e| e.0 == "p" && e.2 == "p.a"),
                "{language:?}: {edges:?}"
            );
            assert!(
                edges.contains(&edge("p", 1, "p.c", 5)),
                "{language:?}: {edges:?}"
            );
        },
    );
    // A write to a prefix suppresses the longer path (flow-insensitive, narrower).
    for_each_language(
        "function h({P}) {\n  a(p.a.b);\n  p.a = other;\n}\n",
        |language, g, _| {
            assert!(
                !field_edges(g, "p")
                    .iter()
                    .any(|e| e.0 == "p" && e.1 == 1 && e.2 == "p.a.b"),
                "{language:?}"
            );
        },
    );
}

#[test]
fn write_to_the_parameter_itself_projects_nothing() {
    for_each_language(
        "function h({P}) {\n  p = other();\n  a(p.x);\n}\n",
        |language, g, _| {
            assert!(
                !field_edges(g, "p").iter().any(|e| e.0 == "p" && e.1 == 1),
                "{language:?}: {:?}",
                field_edges(g, "p")
            );
        },
    );
}

#[test]
fn computed_member_and_bare_uses_keep_the_old_behaviour() {
    // `p[k]` counts as a bare reference: the old contract (a Def, no field projection).
    for_each_language(
        "function h({P}, k) {\n  a(p[k]);\n  b(p.x);\n}\n",
        |language, g, _| {
            assert!(
                base_defs(g, "p").contains(&("h".to_string(), 1)),
                "{language:?}"
            );
            assert!(
                field_edges(g, "p").is_empty(),
                "{language:?}: {:?}",
                field_edges(g, "p")
            );
        },
    );
    // A bare use plus a member use: old behaviour, no projection edges.
    for_each_language(
        "function h({P}) {\n  a(p);\n  b(p.x);\n}\n",
        |language, g, _| {
            assert!(
                base_defs(g, "p").contains(&("h".to_string(), 1)),
                "{language:?}"
            );
            assert!(field_edges(g, "p").is_empty(), "{language:?}");
        },
    );
}

#[test]
fn nested_shadow_of_the_formal_never_receives_projection_edges() {
    // The only reads of `p` are of an inner binding named `p`. The classifier is syntactic, so the
    // outer formal still gets a Def (disclosed in js-ts-known-gaps.md section 6), but the fenced
    // reference walk proves the inner reads bind to the inner `p`: no edge leaves the outer Def.
    for_each_language(
        "function h({P}) {\n  const inner = (p) => p.x;\n  return inner;\n}\n",
        |language, g, _| {
            let edges = field_edges(g, "p");
            assert!(
                !edges.iter().any(|e| e.0 == "p" && e.1 == 1),
                "{language:?}: outer Def must not project into the shadowing callable: {edges:?}"
            );
            assert!(
                g.edges
                    .iter()
                    .all(|e| !(e.from.function == "h" && e.from.path.base == "p")),
                "{language:?}: no edge at all leaves the outer formal"
            );
        },
    );
    // A block-scoped shadow of the formal inside the same callable.
    for_each_language(
        "function h({P}) {\n  {\n    let p = other();\n    a(p.x);\n  }\n}\n",
        |language, g, _| {
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.from.function == "h" && e.from.line == 1 && e.from.path.base == "p"),
                "{language:?}: {:?}",
                g.edges
            );
        },
    );
}

#[test]
fn parse_recovered_callable_gets_no_member_only_def() {
    for language in JS_TS {
        let source = render(
            language,
            "function h({P}) {\n  a(p.x);\n  let = ;\n}\n",
            "p",
        );
        let parsed = parse(language, &source);
        assert!(
            parsed.parse_error_count > 0,
            "{language:?}: fixture must be recovered"
        );
        let function = parsed
            .all_functions()
            .into_iter()
            .find(|f| parsed.language.function_name(f).is_some())
            .unwrap();
        assert_eq!(
            parsed.classify_parameter_use(&function, "p"),
            ParameterUse::Unused,
            "{language:?}"
        );
        let g = graph(language, &source);
        assert!(base_defs(&g, "p").is_empty(), "{language:?}");
        assert!(field_edges(&g, "p").is_empty(), "{language:?}");
    }
}

#[test]
fn anonymous_callback_owner_gets_the_base_def_and_edges() {
    for_each_language(
        "app.get('/', ({P}) => {\n  a(p.body);\n});\n",
        |language, g, _| {
            let defs = base_defs(g, "p");
            assert_eq!(defs.len(), 1, "{language:?}: {defs:?}");
            assert!(
                defs.iter().all(|(owner, _)| owner.starts_with("<cb@")),
                "{language:?}: synthetic owner expected: {defs:?}"
            );
            assert_eq!(
                field_edges(g, "p"),
                BTreeSet::from([edge("p", 1, "p.body", 2)]),
                "{language:?}"
            );
        },
    );
}

#[test]
fn classifier_distinguishes_bare_member_only_and_unused() {
    for language in JS_TS {
        let classify = |template: &str| {
            let source = render(language, template, "p");
            let parsed = parse(language, &source);
            let function = parsed.all_functions()[0];
            parsed.classify_parameter_use(&function, "p")
        };
        assert_eq!(classify("function h({P}) { a(p); }"), ParameterUse::Bare);
        assert_eq!(
            classify("function h({P}) { a(p.x); }"),
            ParameterUse::MemberOnly
        );
        assert_eq!(
            classify("function h({P}) { a(p?.x.y); }"),
            ParameterUse::MemberOnly
        );
        assert_eq!(
            classify("function h({P}) { a(p.x, p); }"),
            ParameterUse::Bare
        );
        assert_eq!(
            classify("function h({P}) { return 1; }"),
            ParameterUse::Unused
        );
        // Member reads of some other identifier do not make `p` member-only.
        assert_eq!(
            classify("function h({P}) { a(q.x); }"),
            ParameterUse::Unused
        );
    }
}

#[test]
fn non_js_languages_keep_field_only_formals_without_a_def() {
    for (language, source) in [
        (Language::Python, "def h(p):\n    a(p.x)\n    b(p.y)\n"),
        (
            Language::Go,
            "package m\nfunc h(p T) {\n\ta(p.x)\n\tb(p.y)\n}\n",
        ),
        (
            Language::C,
            "void h(struct S *p) {\n  a(p->x);\n  b(p->y);\n}\n",
        ),
    ] {
        let parsed = parse(language, source);
        for function in parsed.all_functions() {
            assert_ne!(
                parsed.classify_parameter_use(&function, "p"),
                ParameterUse::MemberOnly,
                "{language:?}"
            );
        }
        let g = graph(language, source);
        assert!(
            base_defs(&g, "p").is_empty(),
            "{language:?}: {:?}",
            base_defs(&g, "p")
        );
    }
}

#[test]
fn full_and_subset_builds_agree_on_member_only_edges() {
    for language in JS_TS {
        let source = render(
            language,
            "function h({P}) {\n  a(p.x);\n  b(p.y);\n}\n",
            "p",
        );
        let name = file_name(language).to_string();
        let files = BTreeMap::from([(name.clone(), parse(language, &source))]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from([name]));
        assert_eq!(
            serde_json::to_value(&full.edges).unwrap(),
            serde_json::to_value(&subset.edges).unwrap(),
            "{language:?}"
        );
        assert_eq!(full.labels, subset.labels, "{language:?}");
        assert_eq!(field_edges(&full, "p").len(), 2, "{language:?}");
    }
}

#[test]
fn signature_line_member_reads_are_not_projection_targets() {
    for language in JS_TS {
        let source = render(language, "function h({P}, q = p.y) {\n  a(p.x);\n}\n", "p");
        let g = graph(language, &source);
        let edges = field_edges(&g, "p");
        assert!(
            edges.contains(&edge("p", 1, "p.x", 2)),
            "{language:?}: {edges:?}"
        );
        assert!(
            !edges.iter().any(|e| e.2 == "p.y"),
            "{language:?}: a default expression on the signature line is not a body read: {edges:?}"
        );
    }
}

#[test]
fn seam_binding_formal_is_never_member_only() {
    // A body-level re-declaration of a formal that has default expressions re-binds it at the
    // parameter/body seam. In every shape tried the re-declared name is itself a bare
    // identifier (Bare), so the classifier's explicit seam check is defence in depth that no
    // fixture reaches; this pins the observable contract (never MemberOnly).
    for language in JS_TS {
        let source = render(
            language,
            "function h({P} = 1) {\n  class p {}\n  a(p.x);\n}\n",
            "p",
        );
        let parsed = parse(language, &source);
        let function = parsed.all_functions()[0];
        assert_ne!(
            parsed.classify_parameter_use(&function, "p"),
            ParameterUse::MemberOnly,
            "{language:?}"
        );
    }
}

#[test]
fn writes_to_other_bases_and_nested_callables_are_scoped_correctly() {
    // A write to another object never suppresses the formal's projection.
    for_each_language(
        "function h({P}, q) {\n  q.x = 1;\n  a(p.x);\n}\n",
        |language, g, _| {
            assert!(
                field_edges(g, "p").contains(&edge("p", 1, "p.x", 3)),
                "{language:?}: {:?}",
                field_edges(g, "p")
            );
        },
    );
    // A write to the same path inside a nested callable suppresses it (flow-insensitive).
    for_each_language(
        "function h({P}) {\n  const f = () => {\n    p.x = 1;\n  };\n  a(p.x);\n}\n",
        |language, g, _| {
            assert!(
                !field_edges(g, "p")
                    .iter()
                    .any(|e| e.0 == "p" && e.1 == 1 && e.2 == "p.x"),
                "{language:?}: {:?}",
                field_edges(g, "p")
            );
        },
    );
    // Same inside an anonymous callback owner, where nested-scope writes are tracked apart.
    for_each_language(
        "app.get('/', ({P}) => {\n  const f = () => {\n    p.x = 1;\n  };\n  a(p.x);\n});\n",
        |language, g, _| {
            assert!(
                !field_edges(g, "p")
                    .iter()
                    .any(|e| e.0 == "p" && e.1 == 1 && e.2 == "p.x"),
                "{language:?}: {:?}",
                field_edges(g, "p")
            );
        },
    );
}

#[test]
fn missing_token_recovery_also_refuses_the_def() {
    // `a(p.x;` parses with a MISSING `)` (not an ERROR node).
    for language in JS_TS {
        let source = render(language, "function h({P}) {\n  a(p.x;\n}\n", "p");
        let parsed = parse(language, &source);
        assert!(parsed.parse_error_count > 0, "{language:?}");
        let g = graph(language, &source);
        assert!(
            base_defs(&g, "p").is_empty(),
            "{language:?}: {:?}",
            base_defs(&g, "p")
        );
    }
}

#[test]
fn write_only_member_formal_gets_a_def_with_no_outgoing_edges() {
    // Disclosed in js-ts-known-gaps.md section 6: the classifier counts a member write as a
    // member access, so the formal is registered, but the written path is suppressed.
    for_each_language("function h({P}) {\n  p.x = 1;\n}\n", |language, g, _| {
        assert_eq!(
            base_defs(g, "p"),
            BTreeSet::from([("h".to_string(), 1)]),
            "{language:?}"
        );
        assert!(
            !g.edges
                .iter()
                .any(|e| e.from.path.base == "p" && e.from.line == 1),
            "{language:?}: {:?}",
            g.edges
        );
    });
}
