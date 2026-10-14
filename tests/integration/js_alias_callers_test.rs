//! Slice L round-1 fold, item 4: the callers index counts a call site whose binding is a
//! followed alias exactly once for its resolved target, whatever the alias is spelled (the
//! site is filed under the alias's spelling, not the target's name). Round 3, finding C:
//! `--format callers` never takes a followed site from its spelling bucket.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::collections::BTreeMap;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

/// The names of the functions `nav callers` lists for `target` in `file`, one per item.
fn caller_names(s: &NavigationSession, target: &str, file: &str) -> Vec<String> {
    let ev = queries::callers(s, Some(target), Some(file), None, 1).unwrap();
    ev.items
        .iter()
        .filter_map(|i| match &i.symbol {
            Some(SymbolRef::Function { name, .. }) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

const M: &str = "export function f() {\n  return 1;\n}\n";

#[test]
fn an_alias_of_an_import_is_a_caller_once() {
    let app = "import { f } from './m';\nexport function viaImportAlias() {\n  const h = f;\n  \
        return h();\n}\n";
    let s = session(&[("m.js", M), ("app.js", app)]);
    assert_eq!(caller_names(&s, "f", "m.js"), ["viaImportAlias"]);
}

#[test]
fn an_alias_of_a_renamed_import_is_a_caller_once() {
    let app =
        "import { f as imp } from './m';\nexport function viaRenamed() {\n  const h = imp;\n  \
        return h();\n}\n";
    let s = session(&[("m.js", M), ("app.js", app)]);
    assert_eq!(caller_names(&s, "f", "m.js"), ["viaRenamed"]);
}

#[test]
fn a_namespace_member_alias_is_a_caller_once() {
    let app = "import * as N from './m';\nexport function viaMember() {\n  const h = N.f;\n  \
        return h();\n}\n";
    let s = session(&[("m.js", M), ("app.js", app)]);
    assert_eq!(caller_names(&s, "f", "m.js"), ["viaMember"]);
}

#[test]
fn destructured_namespace_aliases_are_callers_once_whatever_their_spelling() {
    // The shorthand spells the target's own name (bucket key == target name): still once.
    let shorthand =
        "import * as N from './m';\nexport function viaShorthand() {\n  const { f } = N;\n  \
        return f();\n}\n";
    let renamed =
        "import * as N from './m';\nexport function viaRename() {\n  const { f: p2 } = N;\n  \
        return p2();\n}\n";
    for (label, app, who) in [
        ("shorthand", shorthand, "viaShorthand"),
        ("rename", renamed, "viaRename"),
    ] {
        let s = session(&[("m.js", M), ("app.js", app)]);
        assert_eq!(caller_names(&s, "f", "m.js"), [who], "{label}");
    }
}

#[test]
fn an_alias_spelled_like_its_target_is_counted_once() {
    // `const f = N.f` in an inner scope: the alias and the target share a spelling.
    let app = "import * as N from './m';\nexport function viaSameName() {\n  const f = N.f;\n  \
        return f();\n}\n";
    let s = session(&[("m.js", M), ("app.js", app)]);
    assert_eq!(caller_names(&s, "f", "m.js"), ["viaSameName"]);
}

#[test]
fn an_alias_of_a_same_file_function_is_a_caller_once() {
    let src = "function f() {\n  return 1;\n}\nexport function viaLocal() {\n  const h = f;\n  \
        return h();\n}\n";
    let s = session(&[("app.js", src)]);
    assert_eq!(caller_names(&s, "f", "app.js"), ["viaLocal"]);
}

/// `to_callers_output` (`--format callers`) over `files` for the given unified diff and depth:
/// per touched function `(file, [start, end], [(caller name, depth)])`.
fn format_callers_at(
    files: &[(&str, &str)],
    diff: &str,
    depth: usize,
) -> Vec<(String, [usize; 2], Vec<(String, usize)>)> {
    use prism::cpg::CpgContext;
    use prism::diff::DiffInput;
    let mut parsed = BTreeMap::new();
    for (path, text) in files {
        parsed.insert(
            (*path).to_string(),
            ParsedFile::parse(path, text, Language::from_path(path).unwrap()).unwrap(),
        );
    }
    let ctx = CpgContext::build(&parsed, None);
    let out = prism::output::to_callers_output(&ctx, &DiffInput::parse_unified_diff(diff), depth);
    out.callers
        .iter()
        .map(|e| {
            (
                e.file.clone(),
                e.line_range,
                e.callers
                    .iter()
                    .map(|c| (c.name.clone(), c.depth))
                    .collect(),
            )
        })
        .collect()
}

// ---- round 3, finding C: a followed site is never taken from its spelling bucket ----

const C_LIB: &str = "export function target() {\n  return 1;\n}\n";
const C_DIFF: &str = "--- a/lib.js\n+++ b/lib.js\n@@ -2 +2 @@\n-  return 0;\n+  return 1;\n";

fn names(depth_rows: &[(String, [usize; 2], Vec<(String, usize)>)]) -> Vec<(String, usize)> {
    depth_rows.iter().flat_map(|r| r.2.clone()).collect()
}

#[test]
fn format_callers_does_not_credit_a_followed_site_for_the_function_named_like_its_alias() {
    // `outer`'s `run()` is proven to call `decoy`; it must not be a caller of the function `run`.
    let app = "import * as N from './lib';\nexport function run() {\n  const h = N.target;\n  \
        h();\n}\nfunction decoy() {\n  return 2;\n}\nexport function outer() {\n  \
        const run = decoy;\n  run();\n}\n";
    let files = [("lib.js", C_LIB), ("app.js", app)];
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 1)),
        [("run".to_string(), 1)]
    );
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 2)),
        [("run".to_string(), 1)]
    );
}

#[test]
fn format_callers_credits_a_genuine_transitive_caller_through_a_followed_site() {
    let app = "import * as N from './lib';\nexport function run() {\n  const h = N.target;\n  \
        h();\n}\nexport function top() {\n  const g = run;\n  g();\n}\n";
    let files = [("lib.js", C_LIB), ("app.js", app)];
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 2)),
        [("run".to_string(), 1), ("top".to_string(), 2)]
    );
}

#[test]
fn format_callers_direct_twin_keeps_mains_walk_for_predicate_false_sites() {
    // `N.target()` and `run()` are direct calls (no alias): both are predicate-false sites and
    // the name-keyed walk is main's, depth 1 and depth 2 alike.
    let app = "import * as N from './lib';\nexport function run() {\n  N.target();\n}\n\
        export function top() {\n  run();\n}\n";
    let files = [("lib.js", C_LIB), ("app.js", app)];
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 1)),
        [("run".to_string(), 1)]
    );
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 2)),
        [("run".to_string(), 1), ("top".to_string(), 2)]
    );
}

#[test]
fn format_callers_direct_twin_is_not_credited_through_a_proven_other_target() {
    // Direct call `N.target()` in place of the alias: `run` is found by the name-keyed walk. On
    // main the walk then also lists `outer` at depth 2, because `outer`'s site is spelled `run`;
    // that site is proven to call `decoy` (a followed alias), so by the rule it is never taken
    // from the `run` bucket and the inherited over-collection is gone for this site.
    let app = "import * as N from './lib';\nexport function run() {\n  N.target();\n}\n\
        function decoy() {\n  return 2;\n}\nexport function outer() {\n  const run = decoy;\n  \
        run();\n}\n";
    let files = [("lib.js", C_LIB), ("app.js", app)];
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 2)),
        [("run".to_string(), 1)]
    );
}

#[test]
fn format_callers_still_walks_an_unproven_alias_from_its_bucket() {
    // `const run = missing; run()` is NOT proven (a non-const binding keeps main's path), so it
    // stays in the `run` bucket and is credited by the name-keyed walk exactly as on main.
    let app = "import * as N from './lib';\nexport function run() {\n  N.target();\n}\n\
        export function outer(missing) {\n  let run = missing;\n  run();\n}\n";
    let files = [("lib.js", C_LIB), ("app.js", app)];
    assert_eq!(
        names(&format_callers_at(&files, C_DIFF, 2)),
        [("run".to_string(), 1), ("outer".to_string(), 2)]
    );
}
