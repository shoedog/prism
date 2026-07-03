//! P7: Python `@property`/`@cached_property` access nav surfacing (S3),
//! exercised end-to-end through `nav_callers`/`nav_callees`. Mirrors
//! `func_value_test.rs`'s style for the P5 Go func-value callback feature.

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

// NOTE: an incremental-rebuild parity test (confirming
// `build_incremental_with_scope_graph_inputs` explicitly clears + re-applies
// the whole-program S1/S2 state via `apply_python_property_accesses`, the way
// it already does for Go func-value callbacks) lives alongside the existing
// `incremental_from_previous_*` tests in `src/navigation/mod.rs`'s own
// `#[cfg(test)]` module for the same reason `func_value_test.rs` documents:
// `NavigationIndex::build_incremental_from_previous` is `pub(crate)`.

#[test]
fn self_attr_property_access_surfaces_as_nameonly_caller() {
    let s = session(&[(
        "resp.py",
        "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
    )]);
    let ev = queries::callers(&s, Some("text"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "dump"))
        .expect("dump() surfaces as a caller of text via the property access");
    assert!(
        (hit.score - 0.6).abs() < 1e-6,
        "property_access is NameOnly -> score 0.6, got {}",
        hit.score
    );
    assert!(hit.why.iter().any(|r| matches!(r,
        prism::navigation::types::Reason::Resolution { kind } if kind == "property_access"
    )));
}

#[test]
fn property_access_surfaces_symmetrically_in_callees() {
    let s = session(&[(
        "resp.py",
        "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
    )]);
    let ev = queries::callees(&s, Some("dump"), None, None, 1).unwrap();
    assert!(
        ev.items
            .iter()
            .any(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "text")),
        "callees(dump) should include text via the property access"
    );
}

#[test]
fn unknown_receiver_single_owner_property_access_surfaces() {
    let s = session(&[(
        "resp.py",
        "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    return r.text\n",
    )]);
    let ev = queries::callers(&s, Some("text"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f"))
        .expect("f() surfaces as a caller of text via the unknown-receiver access");
    assert!(
        (hit.score - 0.6).abs() < 1e-6,
        "property_access is NameOnly -> score 0.6, got {}",
        hit.score
    );
}

#[test]
fn property_access_fanout_over_cap_is_not_attributed_to_any_single_getter() {
    let s = session(&[(
        "resp.py",
        "class A:\n    @property\n    def text(self):\n        return 1\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n\n\n\
class C:\n    @property\n    def text(self):\n        return 3\n\n\n\
class D:\n    @property\n    def text(self):\n        return 4\n\n\n\
def f(r):\n    return r.text\n",
    )]);
    // 4 same-named `text` getters in one file are ambiguous by symbol+file;
    // disambiguate with a location seed on A's getter (line 2).
    let ev = queries::callers(&s, None, None, Some("resp.py:2"), 1).unwrap();
    assert!(
        !ev.items
            .iter()
            .any(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f")),
        ">3 distinct classes defining the property must not attribute the access to any one getter"
    );
}

#[test]
fn store_target_is_not_a_caller_of_the_getter() {
    let s = session(&[(
        "resp.py",
        "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text = \"v\"\n    r.text += \"v\"\n",
    )]);
    let ev = queries::callers(&s, Some("text"), None, None, 1).unwrap();
    assert!(
        !ev.items
            .iter()
            .any(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f")),
        "a plain/augmented assignment STORE target must never surface as a property_access caller"
    );
}
