use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn graph(srcs: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<_, _> = srcs
        .iter()
        .map(|(path, src)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, Language::Python).expect("parse python"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn site(cg: &CallGraph, file: &str, caller: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.file == file && fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("missing {file}:{caller}->{callee}"))
        .clone()
}

fn assert_exact_target(
    cg: &CallGraph,
    site: &CallSite,
    file: &str,
    line: usize,
    kind: ResolutionKind,
) {
    let out = cg.resolve_call_site_full(site);
    assert_eq!(out.resolved.len(), 1, "{out:?}");
    let callee = &out.resolved[0];
    assert_eq!(callee.target.file, file);
    assert_eq!(callee.target.start_line, line);
    assert_eq!(callee.confidence, ResolutionConfidence::Exact);
    assert_eq!(callee.kind, kind);
}

fn assert_no_exact_target(cg: &CallGraph, site: &CallSite, file: &str, line: usize) {
    let out = cg.resolve_call_site_full(site);
    assert!(
        out.resolved.iter().all(|callee| {
            !(callee.target.file == file
                && callee.target.start_line == line
                && callee.confidence == ResolutionConfidence::Exact)
        }),
        "unexpected exact target {file}:{line}: {out:?}"
    );
}

#[test]
fn typed_receiver_inherits_direct_base_with_collision_decoy() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_eq!(call.receiver_type.as_deref(), Some("Child"));
    assert_exact_target(&cg, &call, "svc.py", 2, ResolutionKind::TypedParam);
}

#[test]
fn constructor_local_inherits_direct_base_with_collision_decoy() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run():\n    c = Child()\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_eq!(call.receiver_type.as_deref(), Some("Child"));
    assert_exact_target(&cg, &call, "svc.py", 2, ResolutionKind::ConstructorLocal);
}

#[test]
fn local_child_override_wins_before_inherited_base() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_exact_target(&cg, &call, "svc.py", 6, ResolutionKind::TypedParam);
}

#[test]
fn cross_file_child_owner_does_not_preempt_same_file_base() {
    let cg = graph(&[
        (
            "a.py",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        ),
        ("b.py", "class Child:\n    def go(self):\n        pass\n"),
    ]);
    let call = site(&cg, "a.py", "run", "go");
    assert_exact_target(&cg, &call, "a.py", 2, ResolutionKind::TypedParam);
}

#[test]
fn ambiguous_child_method_blocks_inherited_base_fallback() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    def go(self):\n        pass\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_no_exact_target(&cg, &call, "svc.py", 2);
}

#[test]
fn ambiguous_base_method_blocks_inherited_exact() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_no_exact_target(&cg, &call, "svc.py", 2);
}

#[test]
fn untyped_receiver_collision_remains_non_exact() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    def go(self):\n        pass\n\ndef run(c):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_eq!(call.receiver_type, None);
    assert_no_exact_target(&cg, &call, "svc.py", 6);
}

#[test]
fn dirty_child_identity_does_not_inherit_exact() {
    for (name, src) in [
        (
            "duplicate",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\nclass Child:\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "imported",
            "from ext import Child\nclass Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "assigned",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\nChild = ext.Child\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "list_destructuring",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n[Child] = values\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "function",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\ndef Child():\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "type_alias",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\ntype Child = Other\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "compound",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\nif cond:\n    Child = ext.Child\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "with_alias",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\nwith ctx() as Child:\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "delete",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\ndel Child\n\ndef run(c: Child):\n    c.go()\n",
        ),
        (
            "delete_multi",
            "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\nOther = object()\ndel Child, Other\n\ndef run(c: Child):\n    c.go()\n",
        ),
    ] {
        let cg = graph(&[("svc.py", src)]);
        let call = site(&cg, "svc.py", "run", "go");
        assert_no_exact_target(&cg, &call, "svc.py", 2);
        assert!(
            cg.clean_class_spans_is_empty_for("svc.py", "Child"),
            "{name} should not be clean"
        );
    }
}

#[test]
fn dirty_base_identity_does_not_inherit_exact() {
    for src in [
        "from ext import Base\nclass Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\nBase = ext.Base\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\n[Base] = values\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\ndef Base():\n    pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\nclass Base:\n    pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\nif cond:\n    Base = ext.Base\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\nwith ctx() as Base:\n    pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\ntype Base = Other\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\ndel Base\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
        "class Base:\n    def go(self):\n        pass\nOther = object()\ndel Base, Other\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
    ] {
        let cg = graph(&[("svc.py", src)]);
        let call = site(&cg, "svc.py", "run", "go");
        assert_no_exact_target(&cg, &call, "svc.py", 2);
    }
}

#[test]
fn module_scope_match_barrier_blocks_inherited_exact() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nmatch value:\n    case _:\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_no_exact_target(&cg, &call, "svc.py", 2);
}

#[test]
fn nested_module_scope_match_barrier_blocks_inherited_exact() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nif cond:\n    match value:\n        case Child:\n            pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_no_exact_target(&cg, &call, "svc.py", 2);
    assert!(cg.clean_class_spans_is_empty_for("svc.py", "Child"));
}

#[test]
fn multiple_inheritance_does_not_inherit_exact() {
    let cg = graph(&[(
        "svc.py",
        "class Base:\n    def go(self):\n        pass\n\nclass Other:\n    pass\n\nclass Child(Base, Other):\n    pass\n\ndef run(c: Child):\n    c.go()\n",
    )]);
    let call = site(&cg, "svc.py", "run", "go");
    assert_no_exact_target(&cg, &call, "svc.py", 2);
}

trait CleanClassProbe {
    fn clean_class_spans_is_empty_for(&self, file: &str, owner: &str) -> bool;
}

impl CleanClassProbe for CallGraph {
    fn clean_class_spans_is_empty_for(&self, file: &str, owner: &str) -> bool {
        !self
            .clean_class_spans
            .contains_key(&(file.to_string(), owner.to_string()))
    }
}
