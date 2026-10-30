//! Anonymous import authority: real byte-binder facts, independent of shadow enumeration.
use super::*;
use crate::resolution::ResolutionConfidence::Exact;

const REVIEW_CASES: &[(&str, &str)] = &[

        ("module_catch", "import { f } from './lib'; try { throw (() => 0); } catch (f) { sink(() => f()); }"),
        ("module_catch_destructure", "import { f } from './lib'; try { throw (() => 0); } catch ({f}) { sink(() => f()); }"),
        ("module_block", "import { f } from './lib'; { let f = unknown; sink(() => f()); }"),
        ("module_for", "import { f } from './lib'; for (const f of values) { sink(() => f()); }"),
        ("module_switch", "import { f } from './lib'; switch (value) { case 0: let f=unknown; sink(() => f()); }"),
        ("class_expression", "import { f } from './lib'; const C=class f { field=wrap(() => f()); };"),
        ("named_control", "import { f } from './lib'; { let f=unknown; const cb = () => f(); }"),
        ("enclosing_for", "import { f } from './lib'; function outer(){ for (const f of values) { sink(() => f()); } }"),
        ("enclosing_catch", "import { f } from './lib'; function outer(){ try { throw (() => 0); } catch (f) { sink(() => f()); } }"),
        ("normal_enclosing", "import { f } from './lib'; function outer(){ let f=unknown; sink(() => f()); }"),
        ("normal_own", "import { f } from './lib'; sink((f) => f());"),
        ("own_destructure_object", "import { f } from './lib'; sink(({f}) => f());"),
        ("own_destructure_array", "import { f } from './lib'; sink(([f]) => f());"),
        ("own_default", "import { f } from './lib'; sink((f=unknown) => f());"),
        ("own_rest", "import { f } from './lib'; sink((...f) => f());"),
        ("ancestor_destructure", "import { f } from './lib'; function outer({f}){ sink(() => f()); }"),
        ("ancestor_default", "import { f } from './lib'; function outer(f=unknown){ sink(() => f()); }"),
        ("ancestor_rest", "import { f } from './lib'; function outer(...f){ sink(() => f()); }"),
        ("own_var_hoist", "import { f } from './lib'; sink(() => { f(); if(cond){ var f=unknown; } });"),
        ("ancestor_var_hoist", "import { f } from './lib'; function outer(){ sink(() => f()); if(cond){var f=unknown;} }"),
        ("own_declaration", "import { f } from './lib'; sink(() => { f(); function f(){} });"),
        ("own_generator_declaration", "import { f } from './lib'; sink(() => { f(); function* f(){} });"),
        ("module_classic_for", "import { f } from './lib'; for(let f=unknown; cond;) { sink(() => f()); }"),
        ("own_escaped_parameter", "import { f } from './lib'; sink((\\u0066) => f());"),
        ("ancestor_escaped_parameter", "import { f } from './lib'; function outer(\\u0066){ sink(() => f()); }"),
        ("clean", "import { f } from './lib'; sink(() => f());"),
];

const LOOP_CASES: &[(&str, &str)] = &[
    (
        "own_for_of",
        "import { f } from './lib'; sink(() => { for (const f of values) f(); });",
    ),
    (
        "own_for_in",
        "import { f } from './lib'; sink(() => { for (let f in values) f(); });",
    ),
    (
        "callback_for_var",
        "import { f } from './lib'; values.forEach(x => { for (var f of x) f(); });",
    ),
    (
        "enclosing_for_in",
        "import { f } from './lib'; function outer(){ for (let f in values) sink(() => f()); }",
    ),
    (
        "iife_for_of",
        "import { f } from './lib'; for (const f of values) { (function(){ f(); })(); }",
    ),
    (
        "iife_for_in",
        "import { f } from './lib'; for (let f in values) { (() => f())(); }",
    ),
    (
        "aliased_for_of",
        "import { original as f } from './lib'; for (const f of values) sink(() => f());",
    ),
    (
        "aliased_for_in",
        "import { original as f } from './lib'; for (let f in values) sink(() => f());",
    ),
    (
        "destructured_for",
        "import { f } from './lib'; for (const {f} of values) sink(() => f());",
    ),
];

fn authority_files(source: &str, lang: Language) -> BTreeMap<String, ParsedFile> {
    let fs = BTreeMap::from([
        (
            "a.ts".into(),
            ParsedFile::parse("a.ts", source, lang).unwrap(),
        ),
        (
            "lib.ts".into(),
            ParsedFile::parse(
                "lib.ts",
                "export function f(){} export function original(){} export function g(){}",
                lang,
            )
            .unwrap(),
        ),
    ]);
    assert!(!fs["a.ts"].tree.root_node().has_error(), "{source}");
    fs
}

#[test]
fn real_binder_blocks_all_reviewed_shadow_shapes() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for &(label, source) in REVIEW_CASES.iter().chain(LOOP_CASES) {
            if [
                "clean",
                "named_control",
                "own_declaration",
                "own_generator_declaration",
            ]
            .contains(&label)
            {
                continue;
            }
            let fs = authority_files(source, lang);
            let p = &fs["a.ts"];
            for mut cg in [
                CallGraph::build(&fs),
                CallGraph::build_skeleton(&fs),
                CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect()),
            ] {
                let mut sites = 0;
                for node in candidate_nodes(p, true)
                    .into_iter()
                    .filter(|n| n.kind() != "program" && p.language.function_name(n).is_none())
                {
                    let range = p.node_line_range(&node);
                    if !p
                        .function_calls_with_qualifier_and_spans_on_lines(
                            &node,
                            &(range.0..=range.1).collect(),
                            &BTreeSet::new(),
                        )
                        .0
                        .iter()
                        .any(|s| s.callee_name == "f")
                    {
                        continue;
                    }
                    let site = inject(&mut cg, p, &node, "f");
                    // The binder must carry authority even if the flattened shadow set is absent.
                    cg.js_ts_function_locals.remove(&site.caller);
                    assert!(
                        !matches!(&site.local_binding, JsLocalBinding::Unproven(r) if r == "import"),
                        "binder missed {lang:?}/{label}"
                    );
                    let out = cg.resolve_call_site_full(&site);
                    if out.resolved.iter().any(|r| r.confidence == Exact) {
                        failures.push(format!(
                            "{lang:?}/{label}: {:?} -> {out:?}",
                            site.local_binding
                        ));
                    }
                    checked += 1;
                    sites += 1;
                }
                assert!(
                    sites > 0,
                    "fixture failed to select an anonymous f call: {label}"
                );
            }
        }
    }
    assert_eq!(checked, 279);
    assert!(
        failures.is_empty(),
        "{} false Exacts:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn unproven_and_unchecked_facts_never_reach_same_name_exact() {
    let fs = authority_files(
        "import { f } from './lib'; sink(() => f(), () => g());",
        Language::TypeScript,
    );
    let p = &fs["a.ts"];
    let nodes: Vec<_> = candidate_nodes(p, true)
        .into_iter()
        .filter(|n| n.kind() != "program" && p.language.function_name(n).is_none())
        .collect();
    let mut cg = CallGraph::build(&fs);
    let mut site = inject(&mut cg, p, &nodes[0], "f");
    cg.js_ts_function_locals.remove(&site.caller);
    for fact in [
        JsLocalBinding::Unchecked,
        JsLocalBinding::Position("unknown".into()),
        JsLocalBinding::MayCall("alias".into()),
        JsLocalBinding::Unproven("not_callable".into()),
        JsLocalBinding::Unproven("unbound".into()),
        JsLocalBinding::Unproven("escaped_identifier".into()),
        JsLocalBinding::AliasImport {
            local: "missing".into(),
        },
    ] {
        site.local_binding = fact;
        assert!(
            cg.resolve_call_site_full(&site)
                .resolved
                .iter()
                .all(|r| r.confidence != Exact),
            "{:?}",
            site.local_binding
        );
    }
    // A repo-global fallback and then a same-file fallback must also refuse Unchecked.
    let mut site = inject(&mut cg, p, &nodes[1], "g");
    site.local_binding = JsLocalBinding::Unchecked;
    for file in ["lib.ts", "a.ts"] {
        cg.functions.insert(
            "g".into(),
            vec![FunctionId {
                file: file.into(),
                name: "g".into(),
                start_line: 1,
                end_line: 1,
            }],
        );
        assert!(cg
            .resolve_call_site_full(&site)
            .resolved
            .iter()
            .all(|r| r.confidence != Exact));
    }
}

#[test]
fn import_local_alias_and_sibling_controls_keep_authority() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let fs = authority_files("import { f } from './lib'; sink((f) => f(), () => f(), () => { function f(){} f(); }, () => { const alias=f; alias(); }, () => { function local(){} const alias=local; alias(); });", lang);
        let p = &fs["a.ts"];
        let nodes: Vec<_> = candidate_nodes(p, true)
            .into_iter()
            .filter(|n| n.kind() != "program" && p.language.function_name(n).is_none())
            .collect();
        let mut cg = CallGraph::build(&fs);
        for (i, node) in nodes.iter().enumerate() {
            let site = inject(&mut cg, p, node, if i < 3 { "f" } else { "alias" });
            let out = cg.resolve_call_site_full(&site);
            if i == 0 {
                assert!(out.resolved.is_empty());
                continue;
            }
            assert_eq!(out.resolved.len(), 1, "{i}: {out:?}");
            assert_eq!(out.resolved[0].confidence, Exact);
            assert_eq!(
                out.resolved[0].target.file,
                if [1, 3].contains(&i) {
                    "lib.ts"
                } else {
                    "a.ts"
                }
            );
        }
    }
}

#[test]
fn named_guards_preserve_own_locals_and_legacy_claims() {
    let fs = files("function outer(f){ function named(){ f(); } }");
    let p = &fs["a.ts"];
    let node = p
        .all_functions()
        .into_iter()
        .find(|n| {
            p.language
                .function_name(n)
                .is_some_and(|name| p.node_text(&name) == "named")
        })
        .unwrap();
    assert!(!shadow_locals(p, &node).contains("f"));
    // Named owners retain their legacy inventory even outside strict-scope coordinates.
    assert!(may_mint_call(p, &node, 0, 1, &BTreeSet::from([(0, 1)])));
}

#[test]
fn nonrelative_imports_require_the_same_real_binder_proof() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let fs = authority_files(
            "import { f } from 'lib'; sink((f) => f(), () => f());",
            lang,
        );
        let p = &fs["a.ts"];
        let nodes: Vec<_> = candidate_nodes(p, true)
            .into_iter()
            .filter(|n| n.kind() != "program" && p.language.function_name(n).is_none())
            .collect();
        let mut cg = CallGraph::build(&fs);
        cg.js_ts_path_modules.insert(
            ("a.ts".into(), "lib".into()),
            ("lib.ts".into(), "project".into()),
        );
        cg.js_ts_path_exports
            .insert("project".into(), cg.js_ts_resolved_exports.clone());
        for (i, node) in nodes.iter().enumerate() {
            let site = inject(&mut cg, p, node, "f");
            cg.js_ts_function_locals.remove(&site.caller);
            let out = cg.resolve_call_site_full(&site);
            if i == 0 {
                assert!(out.resolved.is_empty(), "{out:?}");
            } else {
                assert_eq!(out.resolved.len(), 1, "{out:?}");
                assert_eq!(out.resolved[0].confidence, Exact);
                assert_eq!(out.resolved[0].target.file, "lib.ts");
            }
        }
    }
}
