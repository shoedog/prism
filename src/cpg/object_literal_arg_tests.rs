//! #594 variant matrix: an object-literal call argument binds field-selectively
//! to the callee's plain-identifier formal.
//!
//! The new edges are the only cross-function DataFlow edges that end at a
//! `Use` node (plain argument edges end at the formal's `Def`), so every case
//! asserts the exact set of cross-function `Use`-targeted edges.
//! Node spelling: `function:path@line`.

use super::build::CodePropertyGraph;
use super::flow_confidence::{FlowConfidence, FlowDoubt};
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

const LANGS: [(Language, &str); 3] = [
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];

fn build(language: Language, files: &[(&str, &str)], ext: &str) -> CodePropertyGraph {
    let mut parsed = BTreeMap::new();
    for (stem, source) in files {
        let name = format!("{stem}.{ext}");
        let file = ParsedFile::parse(&name, source, language).unwrap();
        assert_eq!(file.parse_error_count, 0, "{name}: {source}");
        parsed.insert(name, file);
    }
    CodePropertyGraph::build(&parsed)
}

fn spell(cpg: &CodePropertyGraph, idx: petgraph::graph::NodeIndex) -> Option<(String, bool)> {
    match cpg.node(idx) {
        CpgNode::Variable {
            path,
            function,
            line,
            access,
            ..
        } => Some((
            format!("{function}:{path}@{line}"),
            *access == VarAccess::Use,
        )),
        _ => None,
    }
}

/// Cross-function DataFlow edges that end at a Use node, with their label.
fn literal_edges(cpg: &CodePropertyGraph) -> BTreeMap<String, FlowConfidence> {
    let mut out = BTreeMap::new();
    for edge in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(label) = cpg.graph[edge] else {
            continue;
        };
        let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
        let (CpgNode::Variable { function: ff, .. }, CpgNode::Variable { function: tf, .. }) =
            (cpg.node(from), cpg.node(to))
        else {
            continue;
        };
        let (Some((f, _)), Some((t, to_use))) = (spell(cpg, from), spell(cpg, to)) else {
            continue;
        };
        if ff != tf && to_use {
            out.insert(format!("{f} -> {t}"), label);
        }
    }
    out
}

fn edge_set(cpg: &CodePropertyGraph) -> BTreeSet<String> {
    literal_edges(cpg).into_keys().collect()
}

/// All three JS-family languages must produce exactly `expected`.
fn check_files(name: &str, files: &[(&str, &str)], expected: &[&str]) {
    let want: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    for (language, ext) in LANGS {
        let cpg = build(language, files, ext);
        let got = edge_set(&cpg);
        assert_eq!(got, want, "{name} [{language:?}]");
    }
}

fn check(name: &str, source: &str, expected: &[&str]) {
    check_files(name, &[("m", source)], expected);
}

// ---------------------------------------------------------------------------
// Positive shapes
// ---------------------------------------------------------------------------

#[test]
fn member_only_formal_binds_key_to_its_own_reads_only() {
    let src = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b) {\n  f({ a: a, b: b });\n}\n";
    check("keyed", src, &["h:a@6 -> f:p.a@2", "h:b@6 -> f:p.b@3"]);
}

#[test]
fn shorthand_properties_bind_like_keyed_ones() {
    let src =
        "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b) {\n  f({ a, b });\n}\n";
    check("shorthand", src, &["h:a@6 -> f:p.a@2", "h:b@6 -> f:p.b@3"]);
}

#[test]
fn mixed_formal_bare_reads_take_every_key_member_reads_only_their_own() {
    // `p` is read bare (line 2) and through members (lines 3, 4). Member reads of a
    // mixed formal are base `p` Uses: the bound key is decided from the source chain.
    let src = "function f(p) {\n  sinkP(p);\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b) {\n  f({ a, b });\n}\n";
    check(
        "mixed",
        src,
        &[
            "h:a@7 -> f:p@2",
            "h:a@7 -> f:p@3",
            "h:b@7 -> f:p@2",
            "h:b@7 -> f:p@4",
        ],
    );
}

#[test]
fn deeper_reads_and_prefix_reads_of_a_key_are_bound() {
    let src = "function f(p) {\n  sink(p.a.x);\n  sink2(p.a);\n  sink3(p.b);\n}\nfunction h(a, b) {\n  f({ a, b });\n}\n";
    check(
        "deeper",
        src,
        &[
            "h:a@7 -> f:p.a.x@2",
            "h:a@7 -> f:p.a@2",
            "h:a@7 -> f:p.a@3",
            "h:b@7 -> f:p.b@4",
        ],
    );
}

#[test]
fn the_same_value_under_two_keys_binds_each_occurrence_to_its_own_key() {
    let src = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(x) {\n  f({ a: x, b: x });\n}\n";
    // One Use of `x` per occurrence: each is bound to its own key only.
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let spans: Vec<(String, usize, usize)> = cpg
            .graph
            .edge_indices()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(_) = cpg.graph[edge] else {
                    return None;
                };
                let (from, to) = cpg.graph.edge_endpoints(edge)?;
                let (spell_from, _) = spell(&cpg, from)?;
                let (spell_to, to_use) = spell(&cpg, to)?;
                if !spell_from.starts_with("h:x@6") || !to_use || spell_to.starts_with("h:") {
                    return None;
                }
                let CpgNode::Variable { start_byte, .. } = cpg.node(from) else {
                    return None;
                };
                Some((spell_to, *start_byte, 0))
            })
            .collect();
        assert_eq!(spans.len(), 2, "{language:?}: {spans:?}");
        let by_target: BTreeMap<_, _> = spans.iter().map(|s| (s.0.clone(), s.1)).collect();
        assert_eq!(by_target.len(), 2, "{language:?}");
        assert!(
            by_target["f:p.a@2"] < by_target["f:p.b@3"],
            "{language:?}: first `x` feeds a, second feeds b: {by_target:?}"
        );
    }
}

#[test]
fn member_path_value_binds_to_the_key() {
    // As for a plain argument, both the member path and its base object feed the read: the
    // base carries whole-object taint. Neither reaches the sibling read.
    let src = "function f(opts) {\n  exec(opts.cmd);\n  log(opts.other);\n}\nfunction h(req) {\n  f({ cmd: req.body.cmd });\n}\n";
    check(
        "member-path",
        src,
        &[
            "h:req.body.cmd@6 -> f:opts.cmd@2",
            "h:req@6 -> f:opts.cmd@2",
        ],
    );
}

#[test]
fn this_member_value_binds() {
    let src = "function f(p) {\n  sink(p.a);\n  sink2(p.b);\n}\nclass K {\n  m() {\n    f({ a: this.x });\n  }\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        assert!(
            edges.contains("m:this.x@7 -> f:p.a@2"),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges.iter().all(|e| e.ends_with("f:p.a@2")),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn literal_in_a_later_argument_slot_binds_that_slots_formal() {
    let src = "function f(first, p) {\n  sinkA(p.a);\n  sinkB(first);\n}\nfunction h(a, b) {\n  f(b, { a });\n}\n";
    check("slot-1", src, &["h:a@6 -> f:p.a@2"]);
}

#[test]
fn parenthesised_and_arrow_formals_bind() {
    let paren =
        "const f = (p) => {\n  sinkA(p.a);\n  sinkB(p.b);\n};\nfunction h(a) {\n  f({ a });\n}\n";
    check("paren-arrow", paren, &["h:a@6 -> f:p.a@2"]);
    let method = "class K {\n  f(p) {\n    sinkA(p.a);\n    sinkB(p.b);\n  }\n  g(a) {\n    this.f({ a });\n  }\n}\n";
    check("method-self-call", method, &["g:a@7 -> f:p.a@3"]);
}

#[test]
fn parentheses_around_the_literal_are_transparent() {
    let src =
        "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a) {\n  f(({ a }));\n}\n";
    check("parenthesised-literal", src, &["h:a@6 -> f:p.a@2"]);
}

#[test]
fn type_assertions_around_the_literal_are_transparent_in_ts() {
    let src = "function f(p: any) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a: any) {\n  f({ a } as any);\n  f({ a } as const);\n}\n";
    for (language, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        assert_eq!(
            edges,
            BTreeSet::from([
                "h:a@6 -> f:p.a@2".to_string(),
                "h:a@7 -> f:p.a@2".to_string()
            ]),
            "{language:?}"
        );
    }
}

#[test]
fn cross_file_import_binds() {
    let callee = "export function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\n";
    let caller = "import { f } from \"./lib\";\nexport function h(a, b) {\n  f({ a });\n}\n";
    check_files(
        "cross-file",
        &[("lib", callee), ("main", caller)],
        &["h:a@3 -> f:p.a@2"],
    );
}

// ---------------------------------------------------------------------------
// Nested literals
// ---------------------------------------------------------------------------

#[test]
fn nested_literal_binds_its_leaf_path_never_the_container_key_alone() {
    // `{ a: { b: x } }`: x is `p.a.b`. A read of `p.a.c` or `p.d` must not get x, and
    // `p.a` is never "bound to x" as if x were the whole value.
    let src = "function f(p) {\n  sink(p.a.b);\n  sink2(p.a.c);\n  sink3(p.d);\n}\nfunction h(x) {\n  f({ a: { b: x } });\n}\n";
    check("nested", src, &["h:x@7 -> f:p.a.b@2"]);
}

#[test]
fn nested_literal_container_read_sees_the_leaf() {
    // Reading the container `p.a` can read `p.a.b`, so the leaf flows into it.
    let src = "function f(p) {\n  sink(p.a);\n  sink3(p.d);\n}\nfunction h(x) {\n  f({ a: { b: x } });\n}\n";
    check("nested-container", src, &["h:x@6 -> f:p.a@2"]);
}

#[test]
fn nested_literal_deeper_than_two_levels() {
    let src = "function f(p) {\n  sink(p.a.b.c);\n  sink2(p.a.b.d);\n}\nfunction h(x) {\n  f({ a: { b: { c: x } } });\n}\n";
    check("nested-3", src, &["h:x@6 -> f:p.a.b.c@2"]);
}

// ---------------------------------------------------------------------------
// Negatives and key-structure decisions
// ---------------------------------------------------------------------------

#[test]
fn computed_key_refuses_itself_and_every_key_before_it() {
    // `[k]` may equal any key at run time, so keys written before it may be overridden
    // (refused); keys after it are definitive.
    let before = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b, k, v) {\n  f({ a, [k]: v, b });\n}\n";
    check("computed-before", before, &["h:b@6 -> f:p.b@3"]);
    let after = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b, k, v) {\n  f({ [k]: v, a, b });\n}\n";
    check(
        "computed-after",
        after,
        &["h:a@6 -> f:p.a@2", "h:b@6 -> f:p.b@3"],
    );
    let only = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(k, v) {\n  f({ [k]: v });\n}\n";
    check("computed-only", only, &[]);
}

#[test]
fn computed_method_is_also_a_wildcard_writer() {
    let src = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b, k) {\n  f({ a, [k]() {}, b });\n}\n";
    check("computed-method", src, &["h:b@6 -> f:p.b@3"]);
}

#[test]
fn spread_refuses_keys_before_it_and_keeps_keys_after_it() {
    let after = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(o, a, b) {\n  f({ ...o, a, b });\n}\n";
    check(
        "spread-first",
        after,
        &["h:a@6 -> f:p.a@2", "h:b@6 -> f:p.b@3"],
    );
    let before = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(o, a, b) {\n  f({ a, ...o, b });\n}\n";
    check("spread-middle", before, &["h:b@6 -> f:p.b@3"]);
    let last = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(o, a, b) {\n  f({ a, b, ...o });\n}\n";
    check("spread-last", last, &[]);
}

#[test]
fn spread_source_is_not_bound_to_the_formal() {
    // `...o` carries unknown keys: documented gap, no edge from `o` anywhere.
    let src =
        "function f(p) {\n  sinkA(p.a);\n  sinkP(p);\n}\nfunction h(o) {\n  f({ ...o });\n}\n";
    check("spread-source", src, &[]);
}

#[test]
fn duplicate_keys_bind_only_the_last_value() {
    let src = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(x, y) {\n  f({ a: x, a: y });\n}\n";
    check("duplicate", src, &["h:y@5 -> f:p.a@2"]);
}

#[test]
fn accessors_and_methods_are_refused_and_last_property_wins() {
    let getter = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(b) {\n  f({ get a() { return 1; }, b });\n}\n";
    check("getter", getter, &["h:b@6 -> f:p.b@3"]);
    let method = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(x) {\n  f({ a: x, a() {} });\n}\n";
    check("method-last", method, &[]);
    let data_last = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(x) {\n  f({ get a() { return 1; }, a: x });\n}\n";
    check("data-after-getter", data_last, &["h:x@5 -> f:p.a@2"]);
}

#[test]
fn proto_key_is_never_an_own_property() {
    let src = "function f(p) {\n  sinkA(p.__proto__);\n  sinkB(p.b);\n}\nfunction h(x, b) {\n  f({ __proto__: x, b });\n}\n";
    check("proto", src, &["h:b@6 -> f:p.b@3"]);
}

#[test]
fn non_data_values_bind_nothing() {
    // Same carrier rule as plain arguments: only identifiers and member paths.
    let src = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n  sinkC(p.c);\n  sinkD(p.d);\n  sinkE(p.e);\n}\nfunction h(x, o) {\n  f({ a: 1, b: g(x), c: x + 1, d: [x], e: o.m(x) });\n}\n";
    check("non-data", src, &[]);
}

#[test]
fn key_mismatch_with_the_callee_reads_binds_nothing() {
    let src = "function f(p) {\n  sinkA(p.other);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("mismatch", src, &[]);
}

#[test]
fn spread_argument_before_the_literal_shifts_the_slot_and_refuses() {
    // The literal sits in slot 1 of the argument list but may land anywhere at run time.
    let src = "function f(first, p) {\n  sinkA(p.a);\n  sinkF(first);\n}\nfunction h(xs, a) {\n  f(...xs, { a });\n}\n";
    check("spread-arg", src, &[]);
    let control = "function f(first, p) {\n  sinkA(p.a);\n  sinkF(first);\n}\nfunction h(xs, a) {\n  f(xs, { a });\n}\n";
    check("no-spread-arg", control, &["h:a@6 -> f:p.a@2"]);
}

#[test]
fn extra_literal_beyond_the_formals_binds_nothing() {
    let src = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(a) {\n  f(a, { a });\n}\n";
    check("extra", src, &[]);
}

#[test]
fn destructured_rest_and_bare_arrow_formals_are_out_of_scope() {
    let destructured = "function f({ a }) {\n  sinkA(a);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("destructured", destructured, &[]);
    let rest = "function f(...p) {\n  sinkA(p[0]);\n  sinkB(p.a);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("rest", rest, &[]);
    let bare =
        "const f = p => {\n  sinkA(p.a);\n  sinkB(p.b);\n};\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("bare-arrow", bare, &[]);
}

#[test]
fn unused_formal_has_no_target() {
    let src = "function f(p) {\n  return 1;\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("unused", src, &[]);
}

#[test]
fn callee_writes_and_reassignments_are_inherited_from_the_definition_edges() {
    // A member write to `p.a` suppresses the `p.a` projection: nothing is bound to it.
    let write = "function f(p) {\n  p.a = 1;\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a, b) {\n  f({ a, b });\n}\n";
    check("member-write", write, &["h:b@7 -> f:p.b@4"]);
    // A bare reassignment kills the formal before the read. The definition edge to that
    // read stays in the graph labelled `Killed`; the shortcut inherits the label, exactly
    // as the plain-argument path `Use -> Def(p) -> Use(p)` would carry it.
    let reassign = "function f(p) {\n  sinkP(p);\n  p = clean();\n  sinkA(p.a);\n}\nfunction h(a) {\n  f({ a });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", reassign)], ext);
        let edges = literal_edges(&cpg);
        assert_eq!(
            edges.get("h:a@7 -> f:p@2"),
            Some(&FlowConfidence::Exact),
            "{language:?}: bare read before the reassignment: {edges:?}"
        );
        assert!(
            matches!(
                edges.get("h:a@7 -> f:p@4"),
                Some(FlowConfidence::NameOnly(FlowDoubt::Killed { .. }))
            ),
            "{language:?}: the reassigned read is fed only as a Killed edge: {edges:?}"
        );
    }
}

#[test]
fn default_formal_matches_the_plain_argument_decision() {
    // An object-literal argument is never `undefined`, so a default never replaces it; the
    // literal binds exactly when a plain argument binds this slot.
    for default in ["{}", "null"] {
        let plain = format!(
            "function f(p = {default}) {{\n  sinkA(p.a);\n  sinkP(p);\n}}\nfunction h(a) {{\n  f(a);\n}}\n"
        );
        let lit = format!(
            "function f(p = {default}) {{\n  sinkA(p.a);\n  sinkP(p);\n}}\nfunction h(a) {{\n  f({{ a }});\n}}\n"
        );
        for (language, ext) in LANGS {
            let plain_cpg = build(language, &[("m", &plain)], ext);
            let plain_bound = plain_cpg.graph.edge_indices().any(|edge| {
                let CpgEdge::DataFlow(_) = plain_cpg.graph[edge] else {
                    return false;
                };
                let (a, b) = plain_cpg.graph.edge_endpoints(edge).unwrap();
                matches!((spell(&plain_cpg, a), spell(&plain_cpg, b)),
                    (Some((f, _)), Some((t, _))) if f.starts_with("h:a@") && t.starts_with("f:p@1"))
            });
            let lit_cpg = build(language, &[("m", &lit)], ext);
            let edges = edge_set(&lit_cpg);
            assert_eq!(
                !edges.is_empty(),
                plain_bound,
                "{language:?} default={default}: {edges:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Exact-singleton gate
// ---------------------------------------------------------------------------

#[test]
fn nameonly_singleton_call_gets_no_literal_edges_while_plain_arguments_do() {
    let plain = "class K {\n  f(p) {\n    sinkA(p.a);\n  }\n}\nfunction h(o, a) {\n  o.f(a);\n}\n";
    let lit =
        "class K {\n  f(p) {\n    sinkA(p.a);\n  }\n}\nfunction h(o, a) {\n  o.f({ a });\n}\n";
    for (language, ext) in LANGS {
        // Control: the call does bind plain arguments (so it resolves, as NameOnly).
        let cpg = build(language, &[("m", plain)], ext);
        let plain_edges = cpg
            .graph
            .edge_indices()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(label) = cpg.graph[edge] else {
                    return None;
                };
                let (a, b) = cpg.graph.edge_endpoints(edge)?;
                let (f, _) = spell(&cpg, a)?;
                let (t, _) = spell(&cpg, b)?;
                (f.starts_with("h:a@") && t.starts_with("f:p@")).then_some(label)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            plain_edges,
            vec![FlowConfidence::NameOnly(FlowDoubt::CallNameOnly)],
            "{language:?}: control must be a NameOnly singleton"
        );
        let cpg = build(language, &[("m", lit)], ext);
        assert_eq!(edge_set(&cpg), BTreeSet::new(), "{language:?}");
    }
}

#[test]
fn multi_target_call_gets_no_literal_edges() {
    let src = "class A {\n  f(p) {\n    sinkA(p.a);\n  }\n}\nclass B {\n  f(p) {\n    sinkA(p.a);\n  }\n}\nfunction h(o, a) {\n  o.f({ a });\n}\n";
    check("multi-owner", src, &[]);
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

#[test]
fn label_is_exact_when_call_and_definition_edge_are_exact() {
    let src = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a) {\n  f({ a });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = literal_edges(&cpg);
        assert_eq!(
            edges.get("h:a@6 -> f:p.a@2"),
            Some(&FlowConfidence::Exact),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn label_composes_with_the_definition_edge_doubt() {
    // One-statement callees have no CFG: the Def->Use edge is NameOnly(CfgIncomplete),
    // and the shortcut over it must not be stronger than the two hops it replaces.
    let src = "function f(p) { sinkA(p.a); }\nfunction h(a) {\n  f({ a });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = literal_edges(&cpg);
        assert_eq!(
            edges.get("h:a@3 -> f:p.a@1"),
            Some(&FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
            "{language:?}: {edges:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Reachability: the issue's example and its sibling exclusion
// ---------------------------------------------------------------------------

fn reach(cpg: &CodePropertyGraph, seed: &str) -> BTreeSet<String> {
    let loc = cpg
        .graph
        .node_indices()
        .find_map(|i| {
            let (s, is_use) = spell(cpg, i)?;
            if !is_use || s != seed {
                return None;
            }
            let CpgNode::Variable {
                path,
                file,
                function,
                function_start_line,
                line,
                start_byte,
                end_byte,
                ..
            } = cpg.node(i)
            else {
                return None;
            };
            Some(crate::data_flow::VarLocation {
                file: file.clone(),
                function: function.clone(),
                function_start_line: *function_start_line,
                line: *line,
                path: path.clone(),
                start_byte: *start_byte,
                end_byte: *end_byte,
                kind: crate::data_flow::VarAccessKind::Use,
            })
        })
        .unwrap_or_else(|| panic!("seed {seed} not found"));
    cpg.dfg_forward_reachable(&loc)
        .into_iter()
        .map(|l| format!("{}:{}@{}", l.function, l.path, l.line))
        .collect()
}

#[test]
fn issue_example_taint_reaches_the_sink_read_and_not_the_sibling() {
    let src = "function run(opts) {\n  exec(opts.cmd);\n  log(opts.note);\n}\nfunction handler(req) {\n  run({ cmd: req.body.cmd, note: req.body.note });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let reached = reach(&cpg, "handler:req.body.cmd@6");
        assert!(
            reached.contains("run:opts.cmd@2"),
            "{language:?}: {reached:?}"
        );
        assert!(
            !reached.contains("run:opts.note@3"),
            "{language:?}: sibling key must stay clean: {reached:?}"
        );
        let reached = reach(&cpg, "handler:req.body.note@6");
        assert!(
            reached.contains("run:opts.note@3"),
            "{language:?}: {reached:?}"
        );
        assert!(
            !reached.contains("run:opts.cmd@2"),
            "{language:?}: {reached:?}"
        );
    }
}

#[test]
fn flow_continues_through_the_bound_read_into_callee_locals() {
    let src = "function run(opts) {\n  const c = opts.cmd;\n  exec(c);\n  const n = opts.note;\n  log(n);\n}\nfunction handler(req) {\n  run({ cmd: req.body.cmd, note: 1 });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let reached = reach(&cpg, "handler:req.body.cmd@8");
        assert!(reached.contains("run:c@3"), "{language:?}: {reached:?}");
        assert!(!reached.contains("run:n@5"), "{language:?}: {reached:?}");
    }
}

// ---------------------------------------------------------------------------
// Real-site-derived shapes (excalidraw and TypeScript compiler sources),
// each reduced to the smallest fixture that keeps the call/callee shape.
// ---------------------------------------------------------------------------

#[test]
fn real_shape_options_object_with_bare_and_member_reads() {
    // newElement({ type: element.type, x: element.x + o.x, ... }) with `opts.type` read
    // and `opts` passed on whole.
    let src = "function newElement(opts) {\n  const t = opts.type;\n  return base(t, opts);\n}\nfunction flow(element, offsets) {\n  const next = newElement({\n    type: element.type,\n    x: element.x + offsets.x,\n  });\n  return next;\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        // `type` feeds the bare read; `x` is a binary expression and binds nothing.
        assert!(
            edges.contains("flow:element.type@7 -> newElement:opts@3"),
            "{language:?}: {edges:?}"
        );
        assert!(
            !edges.iter().any(|e| e.starts_with("flow:offsets")),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn real_shape_shorthand_pair_with_whole_object_use() {
    // useStable({ app, onCancel, onChange }): the callee passes the whole object on.
    let src = "function useStable(value) {\n  const ref = make(value);\n  assign(ref.current, value);\n  return ref.current;\n}\nfunction comp(app, onCancel) {\n  const stable = useStable({\n    app,\n    onCancel,\n  });\n  return stable;\n}\n";
    check(
        "use-stable",
        src,
        &[
            "comp:app@8 -> useStable:value@2",
            "comp:app@8 -> useStable:value@3",
            "comp:onCancel@9 -> useStable:value@2",
            "comp:onCancel@9 -> useStable:value@3",
        ],
    );
}

#[test]
fn real_shape_methods_arrays_and_shorthand_in_a_registration_literal() {
    // registerCodeFix({ errorCodes, fixIds: [fixId], getCodeActions(context) {...} })
    let src = "function registerCodeFix(reg) {\n  for (const e of reg.errorCodes) {\n    note(e);\n  }\n  if (reg.fixIds) {\n    note2(reg.fixIds);\n  }\n}\nfunction mod(errorCodes, fixId) {\n  registerCodeFix({\n    errorCodes,\n    fixIds: [fixId],\n    getCodeActions(context) {\n      return context;\n    },\n  });\n}\n";
    check(
        "register-code-fix",
        src,
        &["mod:errorCodes@11 -> registerCodeFix:reg.errorCodes@2"],
    );
}

#[test]
fn real_shape_spread_then_override_key_in_a_later_slot() {
    // performCompilation(sys, cb, rd, { ...commandLine, options: commandLineOptions })
    let src = "function performCompilation(sys, cb, rd, config) {\n  use(config.options);\n  use2(config.fileNames);\n}\nfunction run(sys, cb, rd, commandLine, commandLineOptions) {\n  performCompilation(sys, cb, rd, { ...commandLine, options: commandLineOptions });\n}\n";
    check(
        "perform-compilation",
        src,
        &["run:commandLineOptions@6 -> performCompilation:config.options@2"],
    );
}

#[test]
fn real_shape_whole_object_assigned_in_a_single_statement_callee() {
    // setIdentifierAutoGenerate(node, { flags, id, prefix, suffix }): one-line callee,
    // formal read bare on the right-hand side.
    let src = "function setIdentifierAutoGenerate(node, autoGenerate) {\n  getOrCreateEmitNode(node).autoGenerate = autoGenerate;\n  return node;\n}\nfunction create(node, flags, nextId, prefix, suffix) {\n  setIdentifierAutoGenerate(node, {\n    flags,\n    id: nextId,\n    prefix,\n    suffix,\n  });\n}\n";
    check(
        "set-auto-generate",
        src,
        &[
            "create:flags@7 -> setIdentifierAutoGenerate:autoGenerate@2",
            "create:nextId@8 -> setIdentifierAutoGenerate:autoGenerate@2",
            "create:prefix@9 -> setIdentifierAutoGenerate:autoGenerate@2",
            "create:suffix@10 -> setIdentifierAutoGenerate:autoGenerate@2",
        ],
    );
}

#[test]
fn real_shape_member_constant_and_shorthand_index() {
    // processProjectReferenceFile(fileName, { kind: FileIncludeKind.Source, index })
    let src = "function processProjectReferenceFile(fileName, reason) {\n  return processSourceFile(fileName, false, undefined, reason);\n}\nfunction visit(fileName, index) {\n  processProjectReferenceFile(fileName, {\n    kind: FileIncludeKind.SourceFromProjectReference,\n    index,\n  });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        assert!(
            edges.contains("visit:index@7 -> processProjectReferenceFile:reason@2"),
            "{language:?}: {edges:?}"
        );
        // `fileName` is a plain argument (Def-targeted), never a literal edge.
        assert!(
            !edges.iter().any(|e| e.contains("visit:fileName")),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn real_shape_key_the_callee_never_reads() {
    // getNamedImportSpecifierComparer({ organizeImportsTypeOrder: typeOrder }, comparer)
    // where the callee reads a different key of its first formal.
    let src = "function getComparer(preferences, comparer) {\n  const c = comparer ?? make(!!preferences.organizeImportsIgnoreCase);\n  return c;\n}\nfunction organize(typeOrder, cmp) {\n  return getComparer({ organizeImportsTypeOrder: typeOrder }, cmp);\n}\n";
    check("comparer", src, &[]);
}

#[test]
fn real_shape_logical_or_value_and_array_value_bind_nothing() {
    // restoreElementWithProperties(element, { fontSize, textAlign: element.textAlign || D })
    // and updateElbowArrowPoints(a, m, { points: [e.points[0], e.points[1]] }).
    let src = "function restore(element, props) {\n  const s = props.fontSize;\n  const t = props.textAlign;\n  const p = props.points;\n  return build(s, t, p);\n}\nfunction caller(element, fontSize, e) {\n  restore(element, {\n    fontSize,\n    textAlign: element.textAlign || DEFAULT,\n    points: [e.points[0], e.points[1]],\n  });\n}\n";
    check(
        "restore",
        src,
        &["caller:fontSize@9 -> restore:props.fontSize@2"],
    );
}

#[test]
fn real_shape_rest_formal_method_call_binds_nothing() {
    // this.updateEditorAtom(atom, { swapPreviewOnAlt: true, onSelect: (c, ev) => {} })
    let src = "class App {\n  updateEditorAtom(atom, ...args) {\n    return store.set(atom, ...args);\n  }\n  open(type, atom) {\n    this.updateEditorAtom(atom, {\n      swapPreviewOnAlt: true,\n      colorPickerType: type === \"stroke\" ? \"s\" : \"b\",\n    });\n  }\n}\n";
    check("rest-method", src, &[]);
}

#[test]
fn real_shape_literal_constants_only() {
    // debugDrawLine(segments, { color: "red", permanent: true })
    let src = "function debugDrawLine(segment, opts) {\n  const c = opts.color;\n  draw(segment, c);\n}\nfunction f(seg) {\n  debugDrawLine(seg, { color: \"red\", permanent: true });\n}\n";
    check("draw-line", src, &[]);
}

#[test]
fn real_shape_callback_value_and_data_value_together() {
    // useHandleLibrary({ excalidrawAPI, getInitialLibraryItems })
    let src = "function useHandleLibrary(opts) {\n  const api = opts.excalidrawAPI;\n  const get = opts.getInitialLibraryItems;\n  run(api, get);\n}\nfunction App(excalidrawAPI, loader) {\n  useHandleLibrary({ excalidrawAPI, getInitialLibraryItems: loader });\n}\n";
    check(
        "use-handle-library",
        src,
        &[
            "App:excalidrawAPI@7 -> useHandleLibrary:opts.excalidrawAPI@2",
            "App:loader@7 -> useHandleLibrary:opts.getInitialLibraryItems@3",
        ],
    );
}

#[test]
fn real_shape_tsx_component_helper_with_options_second_argument() {
    // newElementWith(element, { startArrowhead: s.startArrowhead, endArrowhead: s.endArrowhead })
    let src = "function newElementWith(element, updates, force = false) {\n  const a = updates.startArrowhead;\n  const b = updates.endArrowhead;\n  return merge(element, a, b);\n}\nfunction apply(element, s) {\n  return newElementWith(element, {\n    startArrowhead: s.startArrowhead,\n    endArrowhead: s.endArrowhead,\n  });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        assert!(
            edges.contains("apply:s.startArrowhead@8 -> newElementWith:updates.startArrowhead@2"),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges.contains("apply:s.endArrowhead@9 -> newElementWith:updates.endArrowhead@3"),
            "{language:?}: {edges:?}"
        );
        assert!(
            !edges.iter().any(|e| e
                .contains("startArrowhead@8 -> newElementWith:updates.endArrowhead")
                || e.contains("endArrowhead@9 -> newElementWith:updates.startArrowhead")),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn escaped_or_non_ascii_keys_may_alias_any_key_and_act_as_wildcards() {
    // "\u0061" is the key `a`: the later property overwrites `a: x`. Keys after it are fine.
    let escaped = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(x, y, b) {\n  f({ a: x, \"\\u0061\": y, b });\n}\n";
    check("escaped-key", escaped, &["h:b@6 -> f:p.b@3"]);
    // A non-ASCII spelling can alias an identifier key through normalisation or escapes.
    let non_ascii = "function f(p) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(x, y, b) {\n  f({ a: x, \"\u{e9}\": y, b });\n}\n";
    check("non-ascii-key", non_ascii, &["h:b@6 -> f:p.b@3"]);
    // A definite key no member read can name is not a wildcard.
    let odd = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(x, y) {\n  f({ a: x, \"a-b\": y, 1: y });\n}\n";
    check("unnameable-key", odd, &["h:x@5 -> f:p.a@2"]);
}

#[test]
fn member_write_targets_in_a_mixed_formal_are_not_fed() {
    let src = "function f(p) {\n  sinkP(p);\n  p.a = clean();\n  p.b++;\n  delete p.c;\n}\nfunction h(a, b, c) {\n  f({ a, b, c });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        let edges = edge_set(&cpg);
        for line in [3, 4, 5] {
            assert!(
                !edges.iter().any(|e| e.ends_with(&format!("f:p@{line}"))),
                "{language:?}: write line {line} fed: {edges:?}"
            );
        }
    }
}

#[test]
fn exact_multi_target_pool_gets_no_literal_edges_while_plain_arguments_bind_both() {
    // Both assignments resolve Exact (`LocalDef`): a two-member Exact pool, not a singleton.
    let body = |call: &str| {
        format!(
            "let f;\nif (c) {{\n  f = function (p) {{ sinkA(p.a); }};\n}} else {{\n  f = function (p) {{ sinkB(p.a); }};\n}}\nfunction h(a) {{\n  {call}\n}}\n"
        )
    };
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", &body("f(a);"))], ext);
        let plain_targets: BTreeSet<usize> = cpg
            .graph
            .edge_indices()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(_) = cpg.graph[edge] else {
                    return None;
                };
                let (a, b) = cpg.graph.edge_endpoints(edge)?;
                let (from, _) = spell(&cpg, a)?;
                let CpgNode::Variable {
                    function_start_line,
                    access: VarAccess::Def,
                    ..
                } = cpg.node(b)
                else {
                    return None;
                };
                from.starts_with("h:a@").then_some(*function_start_line)
            })
            .collect();
        assert_eq!(plain_targets.len(), 2, "{language:?}: control pool");
        let cpg = build(language, &[("m", &body("f({ a });"))], ext);
        assert_eq!(edge_set(&cpg), BTreeSet::new(), "{language:?}");
    }
}

#[test]
fn exact_singleton_through_a_constructor_local_receiver_binds() {
    let src = "class A {\n  f(p) {\n    sinkA(p.a);\n  }\n}\nfunction h(a) {\n  const b = new A();\n  b.f({ a });\n}\n";
    check("constructor-local", src, &["h:a@8 -> f:p.a@3"]);
}
