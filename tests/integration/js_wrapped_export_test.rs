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
