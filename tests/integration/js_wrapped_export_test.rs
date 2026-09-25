//! S1 (wrapped-export SPEC §7): span-verified React `forwardRef`/`memo` named exports on
//! the R4c `import_member` route. Every expectation names the target file, name and span.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

pub(super) const APP: &str =
    "import { Island } from './lib';\nexport function App() {\n  return <Island/>;\n}\n";
pub(super) const WRAP: &str = "import { forwardRef } from 'react';\nexport const Island = \
    forwardRef((props, ref) => {\n  return <div ref={ref}/>;\n});\n";

pub(super) fn graph(files: &[(&str, &str)]) -> CallGraph {
    let parsed: BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(path, src)| {
            let lang = Language::from_path(path).unwrap();
            (
                path.to_string(),
                ParsedFile::parse(path, src, lang).unwrap(),
            )
        })
        .collect();
    CallGraph::build(&parsed)
}

/// Every call site in `app.*`, as `L<line> <callee>: <outcome>`. Target files are shown
/// by stem, so one expectation serves the `.jsx` and `.tsx` runs.
pub(super) fn app_sites(cg: &CallGraph) -> Vec<String> {
    let mut out = Vec::new();
    for site in cg.calls.values().flatten() {
        if !site.caller.file.starts_with("app.") {
            continue;
        }
        let outcome = cg.resolve_call_site_full(site);
        let shown = match outcome.drop {
            Some(reason) => format!("drop {reason:?}"),
            None => outcome
                .resolved
                .iter()
                .map(|r| {
                    let t = r.target;
                    let stem = t.file.split('.').next().unwrap();
                    let kind = r.kind.as_str();
                    format!(
                        "{:?} {kind} {stem}:{}@{}-{}",
                        r.confidence, t.name, t.start_line, t.end_line
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        };
        out.push(format!("L{} {}: {shown}", site.line, site.callee_name));
    }
    out.sort();
    out
}

/// `lib.<ext>` + `app.<ext>`; the app-file outcomes.
pub(super) fn run(lib: &str, app: &str, ext: &str) -> Vec<String> {
    app_sites(&graph(&[
        (&format!("lib.{ext}"), lib),
        (&format!("app.{ext}"), app),
    ]))
}

/// Table runner: each row is `(lib, app, expected app-site outcomes)`.
pub(super) fn check(rows: &[(&str, &str, &[&str])], exts: &[&str]) {
    for (i, (lib, app, expected)) in rows.iter().enumerate() {
        for ext in exts {
            assert_eq!(run(lib, app, ext), *expected, "row {i} .{ext}");
        }
    }
}

const ISLAND_2_4: &[&str] = &["L3 Island: Exact import_member lib:Island@2-4"];

#[test]
fn t_p1_named_forward_ref_binds_the_arrow_span() {
    check(&[(WRAP, APP, ISLAND_2_4)], &["jsx", "tsx"]);
    let generic = WRAP.replace("forwardRef((", "forwardRef<HTMLDivElement, any>((");
    check(&[(&generic, APP, ISLAND_2_4)], &["tsx"]);
}

#[test]
fn t_p7_nested_same_name_decoy_is_not_a_target() {
    // C11: the nested `function Island` at 3-3 must never be bound.
    let lib = "import { forwardRef } from 'react';\nfunction helper() {\n  function Island() \
        { return 1; }\n  return Island();\n}\nexport const Island = forwardRef((props, ref) => \
        {\n  return <div ref={ref}/>;\n});\n";
    check(
        &[(lib, APP, &["L3 Island: Exact import_member lib:Island@6-8"])],
        &["jsx", "tsx"],
    );
}

#[test]
fn t_j1_direct_call_of_a_wrapped_export_drops_non_jsx() {
    let app = "import { Island } from './lib';\nexport function App2() {\n  return \
        Island({}, null);\n}\n";
    let cg = graph(&[("lib.tsx", WRAP), ("app.tsx", app)]);
    assert_eq!(app_sites(&cg), ["L3 Island: drop WrappedExportNonJsx"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["dropped_wrapped_export_non_jsx"], 1);
}

#[test]
fn mb1_default_member_write_is_out_of_model_and_stays_exact() {
    // SPEC §3.2: runtime mutation of the React object is out of model (r1 W1, C28).
    let lib = "import React from 'react';\nReact.forwardRef = function fake() {\n  return \
        function Replacement() { return null; };\n};\nexport const Island = React.forwardRef(\
        (props, ref) => {\n  return <div ref={ref}/>;\n});\n";
    check(
        &[(lib, APP, &["L3 Island: Exact import_member lib:Island@5-7"])],
        &["jsx", "tsx"],
    );
}

#[test]
fn t_p2_t_p3_t_p5_t_p6_t_p10_admitted_callee_forms() {
    let arrow = "((props, ref) => {\n  return <div ref={ref}/>;\n});\n";
    let lib = |import: &str, callee: &str| {
        format!("import {import} from 'react';\nexport const Island = {callee}{arrow}")
    };
    let named_fe = "import { forwardRef } from 'react';\nexport const Island = forwardRef(\
        function IslandImpl(props, ref) {\n  return null;\n});\n";
    let rows = [
        (lib("React", "React.forwardRef"), ISLAND_2_4[0]), // T-P2 (C26)
        (lib("* as React", "React.forwardRef"), ISLAND_2_4[0]), // T-P2 (C27)
        (lib("{ memo }", "memo"), ISLAND_2_4[0]),          // T-P3
        (lib("React", "React.memo"), ISLAND_2_4[0]),       // T-P3
        (lib("{ forwardRef as fr }", "fr"), ISLAND_2_4[0]), // T-P5
        (
            named_fe.to_string(),
            "L3 Island: Exact import_member lib:IslandImpl@2-4",
        ), // T-P6
        (WRAP.replace("ef((", "ef(/* c */ ("), ISLAND_2_4[0]), // T-P10
    ];
    for (lib, want) in &rows {
        check(&[(lib.as_str(), APP, &[*want])], &["jsx", "tsx"]);
    }
}

#[test]
fn t_p4_comparator_on_another_line_binds_the_first_argument() {
    // The comparator arrow is also named `Island` (Pattern 3), at 4-4: never a target.
    let lib = "import { memo } from 'react';\nexport const Island = memo((props) => {\n  \
        return <div/>;\n}, (a, b) => true);\n";
    check(
        &[(lib, APP, &["L3 Island: Exact import_member lib:Island@2-4"])],
        &["jsx", "tsx"],
    );
}

#[test]
fn t_p8_t_p9_chains_and_multi_declarators() {
    let app = APP.replace("./lib", "./index");
    for (index, mid) in [
        (
            "export { Island } from './mid';\n",
            "export * from './lib';\n",
        ),
        (
            "export * from './mid';\n",
            "export { Island } from './lib';\n",
        ),
    ] {
        let files = [
            ("lib.tsx", WRAP),
            ("mid.ts", mid),
            ("index.ts", index),
            ("app.tsx", app.as_str()),
        ];
        assert_eq!(app_sites(&graph(&files)), ISLAND_2_4);
    }
    let c17 = "import { forwardRef, memo } from 'react';\nexport const A = forwardRef((props, \
        ref) => {\n  return <div ref={ref}/>;\n}), B = memo((props) => {\n  return <span/>;\n});\n";
    let app_ab = "import { A, B } from './lib';\nexport function App() {\n  return <A/>;\n}\n\
        export function App2() {\n  return <B/>;\n}\n";
    let want = [
        "L3 A: Exact import_member lib:A@2-4",
        "L6 B: Exact import_member lib:B@4-6",
    ];
    check(&[(c17, app_ab, &want)], &["jsx", "tsx"]);
}

#[test]
fn t_p11_benign_react_uses_stay_exact() {
    // C38: reads, types and a member write on the export itself are not callee writes.
    let lib = "import React from 'react';\ntype P = React.ComponentProps<'div'>;\nconst x: \
        typeof React | null = null;\nexport const Island = React.forwardRef<HTMLDivElement, P>(\
        (props, ref) => {\n  const [s] = React.useState(0);\n  return <React.Fragment><div \
        ref={ref}>{s}</div></React.Fragment>;\n});\nIsland.displayName = 'Island';\n";
    check(
        &[(lib, APP, &["L3 Island: Exact import_member lib:Island@4-7"])],
        &["tsx"],
    );
}

#[test]
fn t_j2_t_j4_jsx_gate_scope() {
    // T-J2 (C32): `new X()` is not a call site; the JSX row alone binds.
    let app = "import { Island } from './lib';\nexport function App() {\n  return new (Island as \
        any)();\n}\nexport function App2() {\n  return <Island/>;\n}\n";
    check(
        &[(
            WRAP,
            app,
            &["L6 Island: Exact import_member lib:Island@2-4"],
        )],
        &["tsx"],
    );
    // T-J4: a plain `Local` export called directly is untouched (base-green control).
    let plain = "export const A = (props) => null;\n";
    let app_a = "import { A } from './lib';\nexport function App() {\n  return A({});\n}\n";
    check(
        &[(plain, app_a, &["L3 A: Exact import_member lib:A@1-1"])],
        &["jsx", "tsx"],
    );
}

#[test]
fn mb2_mb3_react_object_mutation_is_out_of_model() {
    // SPEC §3.2: runtime mutation or re-acquisition of the React object is out of model.
    let fake = "\nfunction fake() {\n  return function Replacement() { return null; };\n}\n\n";
    let mb2 = format!(
        "import {{ forwardRef }} from 'react';\n{fake}require('react').forwardRef = fake;\n\
         export const Island = forwardRef((props, ref) => <div ref={{ref}} />);\n"
    ); // r2 W1 (C53)
    let mb3 = format!(
        "import React from 'react';\n{fake}React.__defineGetter__('forwardRef', () => fake);\n\
         export const Island = React.forwardRef((props, ref) => <div ref={{ref}} />);\n"
    ); // r2 W3 (C54)
    let want = ["L3 Island: Exact import_member lib:Island@8-8"];
    check(&[(&mb2, APP, &want), (&mb3, APP, &want)], &["jsx", "tsx"]);
}
