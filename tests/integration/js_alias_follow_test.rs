//! Slice L: a JS/TS `const` alias is followed to what it names. Each row builds a small repo,
//! resolves every call site, and pins the target, kind and confidence. The alias is spelled
//! differently from the callable it names, and a same-named decoy (same file or another file)
//! is present whenever the old by-name search could have picked it.
use super::js_alias_follow_fallback_test::{equals_main, records_of};
use super::js_binding_scope_resolution_test::sites;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

pub(super) fn graph_n(files: &[(&str, &str)]) -> CallGraph {
    let mut parsed = BTreeMap::new();
    for (path, src) in files {
        let lang = Language::from_path(path).unwrap();
        parsed.insert(
            (*path).to_string(),
            ParsedFile::parse(path, src, lang).unwrap(),
        );
    }
    CallGraph::build(&parsed)
}

/// The sites of `caller` only, so callee files may hold anything.
fn caller_sites(files: &[(&str, &str)]) -> Vec<String> {
    sites(&graph_n(files))
}

const CALLEE: &str = "export function target() {\n  return 1;\n}\n\
    export function Comp() {\n  return null;\n}\n";
const DECOY: &str =
    "export function f() {\n  return 2;\n}\nexport function C() {\n  return 3;\n}\n";

#[test]
fn identifier_alias_of_a_renamed_import_binds_the_import_not_the_same_named_decoy() {
    for ext in ["js", "ts"] {
        let callee = format!("callee.{ext}");
        let caller = format!("caller.{ext}");
        let decoy = format!("decoy.{ext}");
        let src = "import { target as imp } from './callee';\nexport function run() {\n  \
            const f = imp;\n  return f();\n}\n";
        let got = caller_sites(&[(&callee, CALLEE), (&caller, src), (&decoy, DECOY)]);
        assert_eq!(got, ["L4 f: Exact import_member target@1-3"], ".{ext}");
    }
}

#[test]
fn identifier_alias_beats_a_same_file_function_of_the_aliases_name() {
    let src = "import { target } from './callee';\nfunction f() {\n  return 0;\n}\n\
        export function run() {\n  const f = target;\n  return f();\n}\n";
    let got = caller_sites(&[("callee.js", CALLEE), ("caller.js", src)]);
    assert_eq!(got, ["L7 f: Exact import_member target@1-3"]);
}

#[test]
fn alias_of_a_same_file_function_binds_it_by_span_and_not_the_decoy() {
    let src = "function real() {\n  return 1;\n}\nexport function run() {\n  const f = real;\n  \
        return f();\n}\n";
    let got = caller_sites(&[("decoy.js", DECOY), ("caller.js", src)]);
    assert_eq!(got, ["L6 f: Exact local_def real@1-3"]);
}

#[test]
fn namespace_member_alias_binds_the_member_through_the_namespace_route() {
    let src = "import * as N from './callee';\nexport function run() {\n  const f = N.target;\n  \
        return f();\n}\n";
    let got = caller_sites(&[
        ("callee.ts", CALLEE),
        ("caller.ts", src),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(got, ["L4 f: Exact import_member target@1-3"]);
}

#[test]
fn flat_pattern_over_a_namespace_binds_shorthand_and_renamed_keys() {
    let src =
        "import * as N from './callee';\nexport function run() {\n  const { target } = N;\n  \
        const { target: f } = N;\n  return [target(), f()];\n}\n";
    let got = caller_sites(&[
        ("callee.ts", CALLEE),
        ("caller.ts", src),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        got,
        [
            "L5 f: Exact import_member target@1-3",
            "L5 target: Exact import_member target@1-3",
        ]
    );
}

#[test]
fn jsx_element_sites_follow_aliases_too() {
    let src = "import * as N from './callee';\nimport { Comp } from './callee';\n\
        export function App() {\n  const C = N.Comp;\n  const D = Comp;\n  \
        return <div><C /><D /></div>;\n}\n";
    let got = caller_sites(&[
        ("callee.jsx", CALLEE),
        ("caller.jsx", src),
        ("decoy.jsx", DECOY),
    ]);
    // (the graph also holds an unrelated derived `Comp` site; the two element sites are pinned)
    for want in [
        "L6 C: Exact import_member Comp@4-6",
        "L6 D: Exact import_member Comp@4-6",
    ] {
        assert!(got.iter().any(|s| s == want), "{want} in {got:?}");
    }
}

#[test]
fn a_wrapped_terminal_binds_from_jsx_only() {
    let src = "import { memo } from 'react';\nconst W = memo(() => null);\nconst A = W;\n\
        export function App() {\n  A();\n  return <A />;\n}\n";
    let got = caller_sites(&[("a.jsx", src)]);
    assert!(
        got.contains(&"L5 A: drop WrappedExportNonJsx".to_string())
            && got.contains(&"L6 A: Exact local_def W@2-2".to_string()),
        "{got:?}"
    );
}

#[test]
fn an_import_alias_with_no_proven_target_keeps_mains_outcome() {
    // `function f` is a same-file decoy of the inner alias `f` (R4 would bind it by name).
    let src = "import { missing } from './callee';\nimport ext from 'some-package';\n\
        function f() {\n  return 0;\n}\n\
        export function run() {\n  const f = missing;\n  const g = ext;\n  return [f(), g()];\n}\n";
    let decoys = "export function f() {\n  return 2;\n}\nexport function g() {\n  return 3;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    // main (same-file decoy `f` binds; next slice)
    equals_main(
        "import local, no candidate",
        got,
        &["L9 f: Exact local_def f@3-5", "L9 g: drop UnknownName"],
    );
}

#[test]
fn a_namespace_member_alias_with_no_proven_target_keeps_mains_outcome() {
    let src = "import * as N from './callee';\nimport * as P from 'some-package';\n\
        function f() {\n  return 0;\n}\n\
        export function run() {\n  const f = N.ghost;\n  const g = P.g;\n  return [f(), g()];\n}\n";
    let decoys = "export function ghost() {\n  return 2;\n}\n\
        export function g() {\n  return 3;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    // main
    equals_main(
        "namespace member, no candidate",
        got,
        &["L9 f: Exact local_def f@3-5", "L9 g: drop UnknownName"],
    );
}

#[test]
fn an_unfollowable_alias_keeps_the_base_outcome() {
    // `let`, an object member and a call result are not alias links: base behavior is
    // unchanged by this slice (the lexical guard drops the by-name hit on the decoys).
    let src = "import { target } from './callee';\nexport function run(o) {\n  let a = target;\n  \
        const b = o.target;\n  const c = make();\n  return [a(), b(), c()];\n}\n";
    let decoys = "export function a() {\n  return 1;\n}\nexport function b() {\n  return 2;\n}\n\
        export function c() {\n  return 3;\n}\nexport function make() {\n  return 4;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    for name in ["a", "b", "c"] {
        let row = format!("L6 {name}: drop UnknownName");
        assert!(got.contains(&row), "{row} in {got:?}");
    }
}

#[test]
fn an_exported_alias_stays_unproven_across_files() {
    // D4 is unchanged: `export const x = f;` is still an unproven local export, so a consumer
    // of `x` gets what it got before this slice.
    let producer = "function f() {\n  return 1;\n}\nexport const x = f;\n";
    let consumer = "import { x } from './producer';\nexport function run() {\n  return x();\n}\n";
    let got = caller_sites(&[("producer.js", producer), ("consumer.js", consumer)]);
    assert_eq!(got, ["L3 x: drop UnknownName"]);
}

#[test]
fn an_alias_of_a_namespace_import_local_is_not_a_member_import() {
    // `X` is a program-scope import, but a namespace, not a member: no candidate, and the
    // same-file decoy `a` must not bind.
    let src = "import * as X from './callee';\nfunction a() {\n  return 0;\n}\n\
        export function run() {\n  const a = X;\n  return a();\n}\n";
    let got = caller_sites(&[("callee.js", CALLEE), ("caller.js", src)]);
    // main: the same-file `a` decoy (unchanged; refusing it is the next slice)
    equals_main(
        "alias of a namespace import local",
        got,
        &["L7 a: Exact local_def a@2-4"],
    );
}

#[test]
fn an_alias_of_a_tsconfig_paths_import_binds_through_the_paths_route() {
    use super::js_paths_common::{config, fixture, graph, outcome, write};
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib": ["lib/real"]})),
        );
        write(
            d.path(),
            &format!("app.{ext}"),
            "import { real as picked } from '@lib';\nexport function run() {\n  \
             const viaAlias = picked;\n  viaAlias();\n}\n",
        );
        let row = outcome(&graph(d.path()), &format!("app.{ext}"), "viaAlias");
        let targets = row["resolved_targets"].as_array().unwrap();
        assert_eq!(targets.len(), 1, "{row}");
        assert_eq!(targets[0]["function_id"]["file"], format!("lib/real.{ext}"));
        assert_eq!(targets[0]["confidence"], "exact", "{row}");
        assert_eq!(targets[0]["kind"], "import_member", "{row}");
    }
}

#[test]
fn a_member_alias_is_proved_per_export_even_when_the_module_namespace_is_not() {
    // `export const renamed = target;` makes the callee's whole-namespace proof incomplete. The
    // alias resolves the way `import { target }` does (per-export facts), so `N.target` still
    // binds Exact import_member and `N.renamed` (no proven callable) drops.
    let callee = format!("{CALLEE}export const renamed = target;\n");
    let src = "import * as N from './callee';\nexport function run() {\n  const f = N.target;\n  \
        const g = N.renamed;\n  return [f(), g()];\n}\n";
    let got = caller_sites(&[
        ("callee.js", &callee),
        ("caller.js", src),
        ("decoy.js", DECOY),
    ]);
    assert_eq!(
        got,
        [
            "L5 f: Exact import_member target@1-3",
            "L5 g: drop UnknownName"
        ]
    );
}

/// Every record at the caller's last line, as `callee origin: target kind confidence` (or
/// `drop`), sorted: the `Source` record and any `IndirectResolution` record the textual
/// function-pointer path (`call_graph.rs` Level 1) adds at the same site.
fn records_at(files: &[(&str, &str)], caller_file: &str, line: u64) -> Vec<String> {
    let cg = graph_n(files);
    let mut out = Vec::new();
    for row in prism::navigation::queries::call_site_dump(&cg) {
        if row["caller"]["file"] != caller_file || row["source_span"]["line"] != line {
            continue;
        }
        let shown = match row["resolved_targets"].as_array().filter(|t| !t.is_empty()) {
            Some(targets) => targets
                .iter()
                .map(|t| {
                    format!(
                        "{}:{}@{}-{} {} {}",
                        t["function_id"]["file"].as_str().unwrap(),
                        t["function_id"]["name"].as_str().unwrap(),
                        t["function_id"]["start_line"],
                        t["function_id"]["end_line"],
                        t["kind"].as_str().unwrap(),
                        t["confidence"].as_str().unwrap()
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
            None => format!("drop {}", row["drop"].as_str().unwrap_or("?")),
        };
        out.push(format!(
            "{} {}: {shown}",
            row["callee_text"].as_str().unwrap(),
            row["origin"].as_str().unwrap()
        ));
    }
    out.sort();
    out
}

const F_CALLEE: (&str, &str) = ("callee.js", "export function f(x) {\n  return x;\n}\n");

#[test]
fn textual_fptr_path_case1_alias_of_an_import_under_its_own_name() {
    let src = "import { f } from './callee';\nexport function run() {\n  const h = f;\n  \
        return h(1);\n}\n";
    // Main: the `Source` record for `h` drops UnknownName; the textual path's record binds `f`.
    // Now the binder follows the alias, so the textual path stands down: one `Source` record.
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 4),
        ["h Source: callee.js:f@1-3 import_member exact"]
    );
}

#[test]
fn textual_fptr_path_case2_alias_of_a_same_file_function() {
    let src = "function f() {\n  return 0;\n}\nexport function run() {\n  const h = f;\n  \
        return h(1);\n}\n";
    // Main: `Source` drops UnknownName; the textual record binds `f`. Now the binder follows
    // the alias and the textual path stands down: one `Source` record.
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 6),
        ["h Source: caller.js:f@1-3 local_def exact"]
    );
}

#[test]
fn textual_fptr_path_still_fires_where_the_binder_does_not_follow() {
    // Control: `let` is not an alias link, so the binder does not follow `h` and the textual
    // path behaves exactly as on the base: `Source` drops, the `IndirectResolution` record binds.
    let src = "import { f } from './callee';\nexport function run() {\n  let h = f;\n  \
        return h(1);\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 4),
        [
            "f IndirectResolution: callee.js:f@1-3 import_member exact",
            "h Source: drop UnknownName",
        ]
    );
    let direct = "function h() {\n  return 0;\n}\nexport function run() {\n  return h(1);\n}\n";
    assert_eq!(
        records_at(&[("caller.js", direct)], "caller.js", 5),
        ["h Source: caller.js:h@1-3 local_def exact"]
    );
}

#[test]
fn textual_fptr_path_case3_shadowed_right_hand_name_is_still_a_false_exact() {
    // `f` at `const h = f` is the parameter of `outer`, not the import. The binder-based follow
    // correctly refuses (the `Source` record drops). The textual function-pointer path
    // (`call_graph.rs` Level 1, `resolve_fptr_assignment`) reads source text, is scope-blind,
    // and still emits an `IndirectResolution` record with a false Exact import_member edge.
    // Pinned as CURRENT behavior, unchanged by slice L; making that path binding-aware for
    // JS/TS is the next slice.
    let src =
        "import { f } from './callee';\nexport function outer(f) {\n  function inner() {\n    \
        const h = f;\n    return h(1);\n  }\n  return inner();\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 5),
        [
            "f IndirectResolution: callee.js:f@1-3 import_member exact",
            "h Source: drop UnknownName",
        ]
    );
}

// ---- namespace-member terminal: the named-export proof, never a stem match ----

const TRAP_APP: &str = "import * as N from './x';\nexport function outer() {\n  const h = N.f;\n  \
    return function inner() {\n    return h(1);\n  };\n}\n";
const TRAP_SAME_FN: &str = "import * as N from './x';\nexport function outer() {\n  \
    const h = N.f;\n  return h(1);\n}\n";

#[test]
fn a_member_missing_from_the_module_never_binds_an_unrelated_file_of_the_same_stem() {
    // `./x` lacks `f`; `other/x.ts` has one. A stem match would bind it Exact; the export
    // proof finds no `f` in `x.ts` and the site drops.
    for (label, src, line) in [("nested", TRAP_APP, 5), ("same function", TRAP_SAME_FN, 4)] {
        let got = caller_sites(&[
            ("app.ts", src),
            ("x.ts", "export function other() {}\n"),
            ("other/x.ts", "export function f() {}\n"),
        ]);
        let row = format!("L{line} h: drop UnknownName");
        assert!(got.contains(&row), "{label}: {row} in {got:?}");
        assert!(!got.iter().any(|s| s.contains("Exact")), "{label}: {got:?}");
    }
}

#[test]
fn a_member_of_a_module_that_has_an_unproven_export_is_still_proved_per_export() {
    let x = "export function f() {}\nexport const renamed = f;\n";
    let app = "import * as N from './x';\nexport function run() {\n  const a = N.f;\n  \
        const b = N.renamed;\n  return [a(), b()];\n}\n";
    let got = caller_sites(&[
        ("app.ts", app),
        ("x.ts", x),
        ("other/x.ts", "export function f() {}\n"),
    ]);
    assert_eq!(
        got,
        ["L5 a: Exact import_member f@1-1", "L5 b: drop UnknownName"]
    );
}

#[test]
fn a_member_through_a_barrel_matches_its_named_import_twin() {
    let imp = "export function f() {}\n";
    for barrel in ["export { f } from './impl';\n", "export * from './impl';\n"] {
        let alias = "import * as N from './barrel';\nexport function run() {\n  const h = N.f;\n  \
            return h(1);\n}\n";
        let twin =
            "import { f } from './barrel';\nexport function run() {\n  // pad\n  return f(1);\n}\n";
        let files = |app| vec![("app.ts", app), ("barrel.ts", barrel), ("impl.ts", imp)];
        let norm = |rows: Vec<String>| {
            rows.iter()
                .filter(|r| r.starts_with("L4 "))
                .map(|r| r.split_once(": ").unwrap().1.to_string())
                .collect::<Vec<_>>()
        };
        let got_alias = norm(caller_sites(&files(alias)));
        let got_twin = norm(caller_sites(&files(twin)));
        assert_eq!(got_alias, got_twin, "{barrel}");
        assert!(!got_alias.is_empty(), "{barrel}");
        assert_eq!(got_alias[0], "Exact import_member f@1-1", "{barrel}");
    }
}

#[test]
fn a_member_that_is_not_a_function_drops() {
    let x = "export class K {}\nexport const c = 1;\nexport function f() {}\n";
    let app = "import * as N from './x';\nexport function run() {\n  const a = N.K;\n  \
        const b = N.c;\n  return [a(), b()];\n}\n";
    let got = caller_sites(&[("app.ts", app), ("x.ts", x)]);
    assert_eq!(got, ["L5 a: drop UnknownName", "L5 b: drop UnknownName"]);
}

#[test]
fn a_package_namespace_member_alias_keeps_mains_outcome() {
    let app = "import * as N from 'pkg';\nexport function run() {\n  const h = N.f;\n  \
        return h(1);\n}\n";
    let got = caller_sites(&[
        ("app.ts", app),
        ("pkg.ts", "export function f() {}\n"),
        ("pkg/index.ts", "export function f() {}\n"),
    ]);
    equals_main("package namespace member", got, &["L4 h: drop UnknownName"]);
}

#[test]
fn a_tsconfig_paths_namespace_member_alias_keeps_mains_outcome() {
    // The same file also imports `real` by name from `@lib`, so the paths module table knows
    // the specifier: only the relative-module requirement keeps the namespace alias out.
    use super::js_paths_common::{config, fixture, graph, outcome, write};
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib": ["lib/real"]})),
        );
        write(
            d.path(),
            &format!("app.{ext}"),
            "import * as N from '@lib';\nimport { real as picked } from '@lib';\n\
             export function run() {\n  const viaNs = N.real;\n  viaNs();\n  picked();\n}\n",
        );
        let cg = graph(d.path());
        equals_main(
            &format!("paths namespace member .{ext}"),
            records_of(&cg, &format!("app.{ext}")),
            &[
                "L5 viaNs Source: drop UnknownName",
                &format!("L6 picked Source: lib/real.{ext}:real@1-1 import_member exact"),
            ],
        );
    }
}

#[test]
fn an_unresolvable_relative_module_alias_keeps_mains_outcome() {
    let app = "import * as N from './missing';\nexport function run() {\n  const h = N.target;\n  \
        return h(1);\n}\n";
    let got = caller_sites(&[
        ("app.ts", app),
        ("callee.ts", CALLEE),
        ("other/missing.ts", CALLEE),
    ]);
    equals_main(
        "unresolvable relative module",
        got,
        &["L4 h: drop UnknownName"],
    );
}

#[test]
fn a_js_suffixed_relative_module_resolves_to_the_ts_file_like_a_named_import() {
    // `./callee.js` finds the same-stem `callee.ts` (relative-candidate fallback on main).
    let app =
        "import * as N from './callee.js';\nexport function run() {\n  const h = N.target;\n  \
        return h(1);\n}\n";
    let got = caller_sites(&[("app.ts", app), ("callee.ts", CALLEE)]);
    assert_eq!(got, ["L4 h: Exact import_member target@1-3"]);
}

#[test]
fn one_followed_alias_call_is_one_edge_for_every_consumer() {
    // `const h = f; h(1)`: the `Source` record resolves to `f`, and the textual function-pointer
    // path stands down for a followed alias, so consumers see exactly one edge (the base had
    // one, from the textual record alone).
    use prism::cpg::{CpgContext, CpgEdge, CpgNode};
    let import_case = "import { f } from './callee';\nexport function run() {\n  const h = f;\n  \
        return h(1);\n}\n";
    let same_file = "function f() {\n  return 0;\n}\nexport function run() {\n  const h = f;\n  \
        return h(1);\n}\n";
    for (label, inputs) in [
        ("import", vec![F_CALLEE, ("caller.js", import_case)]),
        ("same file", vec![("caller.js", same_file)]),
    ] {
        let mut files = BTreeMap::new();
        for (path, text) in inputs {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, text, Language::JavaScript).unwrap(),
            );
        }
        let ctx = CpgContext::build(&files, None);
        let g = &ctx.cpg.graph;
        let to_f = g
            .edge_indices()
            .filter(|&e| matches!(g[e], CpgEdge::Call(_)))
            .filter(|&e| {
                let (from, to) = g.edge_endpoints(e).unwrap();
                matches!(g.node_weight(from), Some(CpgNode::Function { name, .. }) if name == "run")
                    && matches!(g.node_weight(to), Some(CpgNode::Function { name, .. }) if name == "f")
            })
            .count();
        let f = ctx.cpg.call_graph.functions["f"][0].clone();
        let rows = ctx.cpg.call_graph.resolved_caller_edges(&f).len();
        assert_eq!(
            (to_f, rows),
            (1, 1),
            "{label}: (CPG Call edges run -> f, resolved_caller_edges rows)"
        );
    }
}

#[test]
fn a_wrapped_member_binds_from_jsx_only() {
    let x = "import { memo } from 'react';\nexport const W = memo(() => null);\n";
    let app = "import * as N from './x';\nconst A = N.W;\nexport function App() {\n  A();\n  \
        return <A />;\n}\n";
    let got = caller_sites(&[("app.jsx", app), ("x.jsx", x)]);
    // The export is a wrapped component: the JSX site binds it through the alias route (main
    // drops it); the non-JSX call is not a proof and continues as main does (UnknownName).
    assert_eq!(
        got,
        ["L4 A: drop UnknownName", "L5 A: Exact import_member W@2-2"]
    );
}

#[test]
fn a_nested_function_of_the_exports_name_does_not_make_the_member_ambiguous() {
    let x = "export function f() {}\nfunction g() {\n  function f() {}\n  return f;\n}\n";
    let app = "import * as N from './x';\nexport function run() {\n  const h = N.f;\n  \
        return h(1);\n}\n";
    let got = caller_sites(&[("app.ts", app), ("x.ts", x)]);
    assert_eq!(got, ["L4 h: Exact import_member f@1-1"]);
}

#[test]
fn format_callers_lists_a_followed_alias_caller_once() {
    // `--format callers` (`to_callers_output`) walks the name-keyed `callers` index; a followed
    // alias is filed under its own spelling, so the walk must also see it. On main the textual
    // function-pointer record, filed under `f`, supplied this caller.
    use prism::cpg::CpgContext;
    use prism::diff::DiffInput;
    let m = "export function f() {\n  return 1;\n}\n";
    let cases = [
        (
            "import",
            "import { f } from './m';\nexport function viaImportAlias() {\n  const h = f;\n  \
                return h();\n}\n",
            "viaImportAlias",
        ),
        (
            "namespace member",
            "import * as N from './m';\nexport function viaMember() {\n  const h = N.f;\n  \
                return h();\n}\n",
            "viaMember",
        ),
    ];
    for (label, app, who) in cases {
        let mut files = BTreeMap::new();
        for (path, text) in [("m.js", m), ("app.js", app)] {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, text, Language::JavaScript).unwrap(),
            );
        }
        let ctx = CpgContext::build(&files, None);
        let diff = DiffInput::parse_unified_diff(
            "--- a/m.js\n+++ b/m.js\n@@ -1,3 +1,3 @@\n export function f() {\n-  return 0;\n+  return 1;\n }\n",
        );
        let out = prism::output::to_callers_output(&ctx, &diff, 1);
        let names: Vec<_> = out
            .callers
            .iter()
            .flat_map(|e| e.callers.iter().map(|c| c.name.clone()))
            .collect();
        assert_eq!(names, [who], "{label}: {out:?}");
    }
}

#[test]
fn call_stats_counts_followed_aliases_by_terminal_kind_and_outcome() {
    let producer = "function w() {\n  return 1;\n}\nexport { w };\nexport const x = w;\n";
    let app = "import { w } from './producer';\nimport { x } from './producer';\n\
        import * as N from './producer';\nfunction local() {\n  return 0;\n}\n\
        export function run() {\n  const a = w;\n  const b = x;\n  const c = N.w;\n  \
        const d = N.x;\n  const e = local;\n  return [a(), b(), c(), d(), e()];\n}\n";
    let cg = graph_n(&[("producer.ts", producer), ("app.ts", app)]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(
        stats["js_alias_follow"],
        serde_json::json!({
            "callable_proven": 1,
            "import_kept_main": 1,
            "import_proven": 1,
            "namespace_member_kept_main": 1,
            "namespace_member_proven": 1,
        })
    );
}

#[test]
fn a_member_binds_the_outer_function_by_its_end_line_too() {
    // `callee.ts` holds an outer `f` (lines 1-3) with a nested `f` starting on the same line 1;
    // only the outer one is the export, so the identity must compare the end line as well.
    let callee = "export function f() { function f() {}\n  return 1;\n}\n";
    let app = "import * as N from './callee';\nexport function run() {\n  const h = N.f;\n  \
        return h();\n}\n";
    let got = caller_sites(&[("app.ts", app), ("callee.ts", callee)]);
    assert_eq!(got, ["L4 h: Exact import_member f@1-3"]);
}

#[test]
fn the_textual_path_stands_down_for_a_namespace_member_alias() {
    // `const f = N.f; f()`: the textual function-pointer path would read `f = N.f`.
    let app = "import * as N from './callee';\nexport function run() {\n  const f = N.f;\n  \
        return f(1);\n}\n";
    let callee = "export function f(x) {\n  return x;\n}\n";
    let got = records_at(&[("callee.ts", callee), ("app.ts", app)], "app.ts", 4);
    assert_eq!(got, ["f Source: callee.ts:f@1-3 import_member exact"]);
}
