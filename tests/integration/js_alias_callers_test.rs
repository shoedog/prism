//! Slice L round-1 fold, item 4: the callers index counts a call site whose binding is a
//! followed alias exactly once for its resolved target, whatever the alias is spelled (the
//! site is filed under the alias's spelling, not the target's name).

use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
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
