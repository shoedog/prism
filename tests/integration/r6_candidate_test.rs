//! P3: unknown-receiver, multi-owner R6 residue emits a capped, labeled
//! NameOnly candidate edge (`R6MultiOwnerCandidate`) for Python/JS/TS/Tsx
//! instead of the silent drop, gated to `method_ids.len() <= 3`. Rust/Go keep
//! the precision-floor drop unconditionally (fixture-pinned at
//! `eval/fixtures/rust/r6_multi_owner_drop`).

use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn build(sources: &[(&str, &str, Language)]) -> CallGraph {
    let mut files = BTreeMap::new();
    for (path, src, lang) in sources {
        files.insert(
            path.to_string(),
            prism::ast::ParsedFile::parse(path, src, *lang).unwrap(),
        );
    }
    CallGraph::build(&files)
}

fn site_in(cg: &CallGraph, caller_name: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller_name)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("no site {caller_name}->{callee}"))
        .clone()
}

#[test]
fn python_two_owner_untyped_receiver_yields_candidate_hit() {
    use Language::Python;
    let cg = build(&[
        (
            "a.py",
            "class A:\n    def handle(self):\n        pass\n",
            Python,
        ),
        (
            "b.py",
            "class B:\n    def handle(self):\n        pass\n",
            Python,
        ),
        ("m.py", "def run(x):\n    x.handle()\n", Python),
    ]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.drop.is_none(), "must not drop: {out:?}");
    assert_eq!(
        out.resolved.len(),
        2,
        "both A.handle/B.handle kept: {out:?}"
    );
    assert!(out
        .resolved
        .iter()
        .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::R6MultiOwnerCandidate));
}

#[test]
fn python_four_owner_untyped_receiver_over_cap_still_drops() {
    use Language::Python;
    let cg = build(&[
        (
            "a.py",
            "class A:\n    def handle(self):\n        pass\n",
            Python,
        ),
        (
            "b.py",
            "class B:\n    def handle(self):\n        pass\n",
            Python,
        ),
        (
            "c.py",
            "class C:\n    def handle(self):\n        pass\n",
            Python,
        ),
        (
            "d.py",
            "class D:\n    def handle(self):\n        pass\n",
            Python,
        ),
        ("m.py", "def run(x):\n    x.handle()\n", Python),
    ]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.is_empty(), "over the fanout cap: {out:?}");
    assert_eq!(out.drop, Some(DropReason::MultiOwnerCollision));
}

#[test]
fn rust_two_owner_untyped_receiver_still_drops_guard() {
    // Guard: the language gate must not leak to Rust. Mirrors
    // eval/fixtures/rust/r6_multi_owner_drop.
    use Language::Rust;
    let cg = build(&[
        ("a.rs", "impl A {\n    fn handle(&self) {}\n}\n", Rust),
        ("b.rs", "impl B {\n    fn handle(&self) {}\n}\n", Rust),
        (
            "m.rs",
            "fn run() {\n    let x = mystery();\n    x.handle();\n}\n",
            Rust,
        ),
    ]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.is_empty(), "Rust must still drop: {out:?}");
    assert_eq!(out.drop, Some(DropReason::MultiOwnerCollision));
}

#[test]
fn go_two_owner_untyped_receiver_still_drops_guard() {
    // Go is not in the P3 language gate either — same residue policy as Rust.
    // `mystery()` is an unresolvable call (no constructor-guess convention),
    // so `x` stays untyped and the site falls straight through to the R6
    // residue, mirroring the Rust guard's `mystery()` receiver.
    use Language::Go;
    let cg = build(&[(
        "main.go",
        "package main\ntype A struct{}\nfunc (a A) Handle() {}\ntype B struct{}\nfunc (b B) Handle() {}\nfunc run() {\n\tx := mystery()\n\tx.Handle()\n}\n",
        Go,
    )]);
    let site = site_in(&cg, "run", "Handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.resolved.is_empty(), "Go must still drop: {out:?}");
    assert_eq!(out.drop, Some(DropReason::MultiOwnerCollision));
}

#[test]
fn javascript_two_owner_untyped_receiver_yields_candidate_hit() {
    use Language::JavaScript;
    let cg = build(&[(
        "svc.js",
        "class A { handle() {} }\nclass B { handle() {} }\nfunction run(x) { x.handle(); }\n",
        JavaScript,
    )]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.drop.is_none(), "must not drop: {out:?}");
    assert_eq!(out.resolved.len(), 2, "{out:?}");
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::R6MultiOwnerCandidate
            && c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn typescript_two_owner_untyped_receiver_yields_candidate_hit() {
    use Language::TypeScript;
    let cg = build(&[(
        "svc.ts",
        "class A { handle() {} }\nclass B { handle() {} }\nfunction run(x) { x.handle(); }\n",
        TypeScript,
    )]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.drop.is_none(), "must not drop: {out:?}");
    assert_eq!(out.resolved.len(), 2, "{out:?}");
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::R6MultiOwnerCandidate
            && c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn tsx_two_owner_untyped_receiver_yields_candidate_hit() {
    use Language::Tsx;
    let cg = build(&[(
        "svc.tsx",
        "class A { handle() {} }\nclass B { handle() {} }\nfunction run(x) { x.handle(); }\n",
        Tsx,
    )]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.drop.is_none(), "must not drop: {out:?}");
    assert_eq!(out.resolved.len(), 2, "{out:?}");
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::R6MultiOwnerCandidate
            && c.confidence == ResolutionConfidence::NameOnly));
}

#[test]
fn r6_multi_owner_candidate_kind_as_str() {
    assert_eq!(
        ResolutionKind::R6MultiOwnerCandidate.as_str(),
        "r6_multi_owner_candidate"
    );
}

#[test]
fn javascript_three_owner_untyped_receiver_at_cap_yields_candidate_hit() {
    // Boundary: method_ids.len() == 3 is AT the cap (<=3), so still a hit,
    // not a drop.
    use Language::JavaScript;
    let cg = build(&[(
        "svc.js",
        "class A { handle() {} }\nclass B { handle() {} }\nclass C { handle() {} }\nfunction run(x) { x.handle(); }\n",
        JavaScript,
    )]);
    let site = site_in(&cg, "run", "handle");
    let out = cg.resolve_call_site_full(&site);
    assert!(out.drop.is_none(), "at-cap must still hit: {out:?}");
    assert_eq!(out.resolved.len(), 3, "{out:?}");
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::R6MultiOwnerCandidate));
}
