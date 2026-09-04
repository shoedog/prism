use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ReceiverRecovery, ResolutionConfidence, ResolutionKind};
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

fn site(cg: &CallGraph, caller: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("missing {caller}->{callee}"))
        .clone()
}

#[test]
fn test_python_typed_param_constructor_and_annotation_hit() {
    let cg = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\nclass Other:\n    def m(self):\n        pass\ndef typed(x: Foo):\n    x.m()\ndef made():\n    x = Foo()\n    x.m()\ndef annotated():\n    x: Foo\n    x.m()\n",
    )]);
    for caller in ["typed", "made", "annotated"] {
        let s = site(&cg, caller, "m");
        assert_eq!(s.receiver_type.as_deref(), Some("Foo"), "{caller}");
        let r = cg.resolve_call_site(&s);
        assert_eq!(r.len(), 1, "{caller}");
        assert_eq!(r[0].target.start_line, 2, "{caller}");
        assert_eq!(r[0].confidence, ResolutionConfidence::Exact, "{caller}");
        if caller == "typed" {
            assert_eq!(s.receiver_recovery, Some(ReceiverRecovery::TypedParam));
            assert_eq!(r[0].kind, ResolutionKind::TypedParam);
        } else {
            assert_eq!(
                s.receiver_recovery,
                Some(ReceiverRecovery::ConstructorLocal)
            );
            assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);
        }
    }
}

#[test]
fn test_python_imported_typed_receiver_resolves_exact_direct_method() {
    let cg = graph(&[
        (
            "pkg/models.py",
            "class Client:\n    def send(self):\n        pass\n",
        ),
        (
            "app.py",
            "from pkg.models import Client as ImportedClient\ndef typed(client: ImportedClient):\n    client.send()\ndef made():\n    client = ImportedClient()\n    client.send()\n",
        ),
    ]);

    for (caller, recovery) in [
        ("typed", ReceiverRecovery::TypedParam),
        ("made", ReceiverRecovery::ConstructorLocal),
    ] {
        let s = site(&cg, caller, "send");
        assert_eq!(
            s.receiver_type.as_deref(),
            Some("ImportedClient"),
            "{caller}"
        );
        assert_eq!(s.receiver_recovery, Some(recovery), "{caller}");
        let resolved = cg.resolve_call_site(&s);
        assert_eq!(resolved.len(), 1, "{caller}: {resolved:?}");
        assert_eq!(resolved[0].target.file, "pkg/models.py", "{caller}");
        assert_eq!(resolved[0].target.start_line, 2, "{caller}");
        assert_eq!(
            resolved[0].confidence,
            ResolutionConfidence::Exact,
            "{caller}"
        );
        assert_eq!(
            resolved[0].kind,
            if caller == "typed" {
                ResolutionKind::TypedParam
            } else {
                ResolutionKind::ConstructorLocal
            },
            "{caller}"
        );
    }
}

#[test]
fn test_python_imported_typed_receiver_requires_unique_module_and_clean_class() {
    let ambiguous_module = graph(&[
        (
            "a/models.py",
            "class Client:\n    def send(self):\n        pass\n",
        ),
        ("b/models.py", "class Other:\n    pass\n"),
        (
            "app.py",
            "from models import Client\ndef run(client: Client):\n    client.send()\n",
        ),
    ]);
    let s = site(&ambiguous_module, "run", "send");
    assert!(ambiguous_module.resolve_call_site(&s).iter().all(|callee| {
        callee.confidence != ResolutionConfidence::Exact
            || callee.kind != ResolutionKind::TypedParam
    }));

    let external_collision = graph(&[
        (
            "decoy.py",
            "class Client:\n    def send(self):\n        pass\n",
        ),
        (
            "app.py",
            "from external import Client\ndef run(client: Client):\n    client.send()\n",
        ),
    ]);
    let s = site(&external_collision, "run", "send");
    assert!(external_collision
        .resolve_call_site(&s)
        .iter()
        .all(|callee| {
            callee.confidence != ResolutionConfidence::Exact
                || callee.kind != ResolutionKind::TypedParam
        }));
}

#[test]
fn test_python_imported_typed_receiver_excludes_local_import_and_inherited_method() {
    let local_import = graph(&[
        (
            "pkg/models.py",
            "class Client:\n    def send(self):\n        pass\n",
        ),
        (
            "app.py",
            "def run():\n    from pkg.models import Client\n    client: Client\n    client.send()\n",
        ),
    ]);
    let s = site(&local_import, "run", "send");
    assert_eq!(s.receiver_type, None);
    assert!(s.receiver_materialized);

    let inherited_only = graph(&[
        (
            "pkg/models.py",
            "class Base:\n    def send(self):\n        pass\nclass Client(Base):\n    pass\n",
        ),
        (
            "app.py",
            "from pkg.models import Client\ndef run(client: Client):\n    client.send()\n",
        ),
    ]);
    let s = site(&inherited_only, "run", "send");
    assert!(inherited_only.resolve_call_site(&s).iter().all(|callee| {
        callee.confidence != ResolutionConfidence::Exact
            || callee.kind != ResolutionKind::TypedParam
    }));
}

#[test]
fn test_python_shadow_import_wildcard_and_singleton_external_skip() {
    let shadow = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x = other()\n    x.m()\n",
    )]);
    assert_eq!(site(&shadow, "run", "m").receiver_type, None);

    let imported = graph(&[(
        "svc.py",
        "from ext import Foo\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
    )]);
    let s = site(&imported, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(imported.resolve_call_site(&s).iter().all(
        |c| c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
    ));

    for (path, src) in [
        (
            "before.py",
            "from ext import *\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        ),
        (
            "after.py",
            "from ext import *\ndef run(x: Foo):\n    x.m()\nclass Foo:\n    def m(self):\n        pass\n",
        ),
    ] {
        let cg = graph(&[(path, src)]);
        let s = site(&cg, "run", "m");
        assert_eq!(s.receiver_type, None, "{path}");
        assert!(cg.resolve_call_site(&s).iter().all(|c| {
            c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
        }));
    }
}

#[test]
fn test_python_r3b_collision_and_local_miss_fallthrough() {
    let collision = graph(&[(
        "svc.py",
        "class x:\n    def m(self):\n        pass\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
    )]);
    let s = site(&collision, "run", "m");
    let r = collision.resolve_call_site(&s);
    assert_eq!(s.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.start_line, 5);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);

    let miss = graph(&[(
        "miss.py",
        "class Foo:\n    pass\nclass Other:\n    def missing(self):\n        pass\ndef annotated(x: Foo):\n    x.missing()\ndef plain(x):\n    x.missing()\n",
    )]);
    let annotated = site(&miss, "annotated", "missing");
    let plain = site(&miss, "plain", "missing");
    let annotated_out = miss.resolve_call_site_full(&annotated);
    let plain_out = miss.resolve_call_site_full(&plain);
    assert_eq!(annotated.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(annotated_out.resolved, plain_out.resolved);
    assert_eq!(annotated_out.drop, plain_out.drop);
    assert_ne!(annotated_out.drop, Some(DropReason::ExternalReceiver));
}

#[test]
fn test_python_poisoned_materialized_param_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "from ext import Bar\nclass Foo:\n    def m(self):\n        pass\ndef run(Foo: Bar):\n    Foo.m()\n",
    )]);
    let s = site(&cg, "run", "m");
    let out = cg.resolve_call_site_full(&s);

    assert_eq!(s.receiver_type, None);
    assert!(s.receiver_materialized);
    assert!(out.resolved.iter().all(|c| {
        c.kind != ResolutionKind::QualifierOwner
            || c.confidence != ResolutionConfidence::Exact
            || c.target.start_line != 2
    }));
}

#[test]
fn test_python_import_shadowing_materialized_param_suppresses_r3() {
    let cg = graph(&[
        ("api.py", "def m():\n    pass\n"),
        (
            "svc.py",
            "import api\nclass Foo:\n    def m(self):\n        pass\ndef run(api: Foo):\n    api.m()\n",
        ),
    ]);
    let s = site(&cg, "run", "m");
    let out = cg.resolve_call_site(&s);

    assert_eq!(s.receiver_type.as_deref(), Some("Foo"));
    assert!(s.receiver_materialized);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
    assert_ne!(out[0].target.file, "api.py");
}

#[test]
fn test_python_nested_class_body_assignment_does_not_recover_outer_receiver() {
    let cg = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\ndef run():\n    class C:\n        x = Foo()\n    x.m()\n",
    )]);
    let s = site(&cg, "run", "m");
    let r = cg.resolve_call_site(&s);

    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert!(r.iter().all(|c| c.kind != ResolutionKind::ConstructorLocal));
}

#[test]
fn test_python_same_line_assignment_after_call_does_not_recover_receiver() {
    let cg = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\ndef run():\n    x.m(); x = Foo()\n",
    )]);
    let s = site(&cg, "run", "m");
    let r = cg.resolve_call_site(&s);

    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert!(r.iter().all(|c| c.kind != ResolutionKind::ConstructorLocal));
}

// ─── Slice 2c: binding-PRESENCE suppression ─────────────────────────────

/// When a `for` loop binds a name that collides with a class name,
/// the local binding should suppress R3b owner-key resolution.
#[test]
fn test_python_for_loop_binding_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run(items):
    for Foo in items:
        Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "for-loop binding should materialize"
    );
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by for-loop binding"
    );
}

/// When a `with ... as` clause binds a name that collides with an import,
/// the local binding should suppress R3 import-qualifier resolution.
#[test]
fn test_python_with_as_binding_suppresses_r3_import() {
    let cg = graph(&[
        ("api.py", "def m():\n    pass\n"),
        (
            "svc.py",
            "\
import api
def run():
    with open('f') as api:
        api.m()
",
        ),
    ]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "with-as binding should materialize"
    );
    let out = cg.resolve_call_site(&s);
    assert!(
        out.iter().all(|c| c.target.file != "api.py"),
        "R3 import resolution should be suppressed by with-as binding"
    );
}

/// When an `except ... as` clause binds a name that collides with a class,
/// the local binding should suppress R3b owner-key resolution.
#[test]
fn test_python_except_as_binding_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run():
    try:
        pass
    except ValueError as Foo:
        Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "except-as binding should materialize"
    );
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by except-as binding"
    );
}

/// When a walrus (`:=`) expression binds a name that collides with a class,
/// the local binding should suppress R3b owner-key resolution.
#[test]
fn test_python_walrus_binding_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run():
    if (Foo := compute()):
        Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(s.receiver_materialized, "walrus binding should materialize");
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by walrus binding"
    );
}

/// Python 3 comprehensions have their own scope — a `for` target inside a
/// comprehension does NOT rebind the name in the enclosing function scope.
/// `[x for Foo in items]; Foo.m()` should still resolve `Foo` as a class.
#[test]
fn test_python_comprehension_scope_does_not_suppress_enclosing() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run(items):
    _ = [x for Foo in items]
    Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    // The comprehension binding is scoped — it should NOT materialize in run().
    assert!(
        !s.receiver_materialized,
        "comprehension binding should NOT materialize in enclosing scope"
    );
    let out = cg.resolve_call_site(&s);
    assert!(
        out.iter()
            .any(|c| c.confidence == ResolutionConfidence::Exact),
        "Foo.m() should still resolve after comprehension (comprehension has own scope)"
    );
}

/// When a `for` loop binds a name that collides with an import,
/// the local binding should suppress R3 import-qualifier resolution.
#[test]
fn test_python_for_loop_binding_suppresses_r3_import() {
    let cg = graph(&[
        ("api.py", "def m():\n    pass\n"),
        (
            "svc.py",
            "\
import api
def run(items):
    for api in items:
        api.m()
",
        ),
    ]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "for-loop binding should materialize"
    );
    let out = cg.resolve_call_site(&s);
    assert!(
        out.iter().all(|c| c.target.file != "api.py"),
        "R3 import resolution should be suppressed by for-loop binding"
    );
}

/// Existing typed-param recovery still works when no competing binding forms.
/// Regression guard: the return-type change must not break recovery.
#[test]
fn test_python_typed_param_still_recovers_after_return_type_change() {
    let cg = graph(&[(
        "svc.py",
        "\
class Svc:
    def m(self):
        pass
def run(x: Svc):
    x.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type.as_deref(), Some("Svc"));
    assert!(s.receiver_materialized);
    let r = cg.resolve_call_site(&s);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);
}

// ─── Slice 2c review-fix: comprehension scope + params + augmented + as_pattern ─

/// Untyped parameter `def run(Foo):` shadows the class `Foo` — `Foo.m()` should
/// be suppressed (the local param binding poisons the receiver).
#[test]
fn test_python_untyped_param_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run(Foo):
    Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "bare param binding should materialize"
    );
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by untyped param binding"
    );
}

/// Augmented assignment `Foo += x` rebinds the name — `Foo.m()` should be
/// suppressed.
#[test]
fn test_python_augmented_assignment_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run():
    Foo += 1
    Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "augmented assignment should materialize"
    );
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by augmented assignment"
    );
}

/// Destructuring `with cm() as (Foo, other):` binds `Foo` via a tuple pattern
/// in the as_pattern alias — `Foo.m()` should be suppressed.
#[test]
fn test_python_as_pattern_destructuring_suppresses_r3b_owner() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run():
    with open('f') as (Foo, other):
        Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(
        s.receiver_materialized,
        "as_pattern destructuring should materialize"
    );
    let out = cg.resolve_call_site_full(&s);
    assert!(
        out.resolved.iter().all(|c| {
            c.kind != ResolutionKind::QualifierOwner || c.confidence != ResolutionConfidence::Exact
        }),
        "R3b should be suppressed by as_pattern destructuring"
    );
}

// ─── Slice 2c round-2 review fixes ─────────────────────────────────────

/// `obj.Foo = x` is an attribute access, NOT a binding of `Foo`.
/// `Foo.method()` should still resolve via R3b owner-key.
#[test]
fn test_attribute_assignment_no_suppress() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def method(self):
        pass
def run():
    obj.Foo = x
    Foo.method()
",
    )]);
    let s = site(&cg, "run", "method");
    // The attribute assignment `obj.Foo = x` should NOT count as a binding of `Foo`.
    assert!(
        !s.receiver_materialized || s.receiver_type.is_some(),
        "attribute access should not suppress Foo"
    );
    let out = cg.resolve_call_site(&s);
    assert!(
        out.iter()
            .any(|c| c.confidence == ResolutionConfidence::Exact),
        "Foo.method() should still resolve after attribute assignment"
    );
}

/// `def run(*Foo):` — splat parameter should count as a binding and suppress.
#[test]
fn test_splat_param_suppresses() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def method(self):
        pass
def run(*Foo):
    Foo.method()
",
    )]);
    let s = site(&cg, "run", "method");
    assert!(
        s.receiver_materialized,
        "splat param should materialize as binding"
    );
    assert_eq!(s.receiver_type, None, "splat param has no recoverable type");
}

/// `def run(): class Foo: pass; Foo.method()` — nested class name should
/// count as a binding in the enclosing scope and suppress R3b.
#[test]
fn test_nested_class_def_suppresses() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def method(self):
        pass
def run():
    class Foo:
        pass
    Foo.method()
",
    )]);
    let s = site(&cg, "run", "method");
    assert!(
        s.receiver_materialized,
        "nested class def should materialize as binding"
    );
}

/// `def run(): def Foo(): pass; Foo.m()` — nested function name should
/// count as a binding in the enclosing scope and suppress R3b.
#[test]
fn test_nested_func_def_suppresses() {
    let cg = graph(&[(
        "svc.py",
        "\
class Foo:
    def m(self):
        pass
def run():
    def Foo():
        pass
    Foo.m()
",
    )]);
    let s = site(&cg, "run", "m");
    assert!(
        s.receiver_materialized,
        "nested function def should materialize as binding"
    );
}
