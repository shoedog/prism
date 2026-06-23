use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome};
use std::collections::BTreeMap;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(p, s)| {
            let lang = Language::from_path(p).expect("known extension");
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, lang).expect("parse"),
            )
        })
        .collect()
}

fn resolve_self_call<'a>(
    cg: &'a CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> ResolutionOutcome<'a> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|v| v.iter().find(|f| f.file == caller_file))
        .expect("caller fn");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == callee))
        .expect("call site");
    cg.resolve_call_site_full(site)
}

// -----------------------------------------------------------------------
// 1. Depth-1 Exact: self.m() in Child resolves to Base.m
// -----------------------------------------------------------------------
#[test]
fn inherited_self_depth1_exact() {
    let src = "\
class Base:
    def m(self):
        return 1

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert_eq!(
        out.resolved.len(),
        1,
        "should resolve to exactly one target"
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
    assert_eq!(out.resolved[0].target.name, "m");
    // Verify the target is in the Base class, not the Child class
    assert!(
        out.resolved[0].target.start_line <= 3,
        "target should be Base.m"
    );
}

// -----------------------------------------------------------------------
// 2. Grandparent NOT resolved: only depth-1
// -----------------------------------------------------------------------
#[test]
fn inherited_self_grandparent_drops() {
    let src = "\
class A:
    def m(self):
        return 1

class B(A):
    pass

class C(B):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    // B has no method m, so the inherited lookup should NOT recurse to A.
    // This should drop as UnknownName.
    assert!(
        out.resolved.is_empty(),
        "grandparent method must NOT resolve (depth-1 only)"
    );
}

// -----------------------------------------------------------------------
// 3. Member-shadow safe: base has m, child's base redefines m as attribute
// -----------------------------------------------------------------------
#[test]
fn inherited_self_member_shadow_drops() {
    // Child inherits from Shadow which has `m` as a plain attribute (not a method).
    // The method lookup should find nothing in Shadow's methods index.
    let src = "\
class Base:
    def m(self):
        return 1

class Shadow(Base):
    m = 0

class Child(Shadow):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    // Shadow has no method `m` (only an attribute), so inherited_direct_base
    // should find nothing via the methods index.
    assert!(
        out.resolved.is_empty(),
        "shadow attribute should not be resolved as a method"
    );
}

// -----------------------------------------------------------------------
// 4. MI drop: class C(B, D) -> 2 bases -> drop
// -----------------------------------------------------------------------
#[test]
fn inherited_self_mi_drops() {
    let src = "\
class B:
    def m(self):
        return 1

class D:
    def m(self):
        return 2

class C(B, D):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(out.resolved.is_empty(), "multiple inheritance must drop");
}

// -----------------------------------------------------------------------
// 5. Non-simple base preserved as Barrier: class C(make_base(), A)
// -----------------------------------------------------------------------
#[test]
fn inherited_self_nonsimple_base_drops() {
    let src = "\
class A:
    def m(self):
        return 1

def make_base():
    return A

class C(make_base(), A):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    // 2 base slots -> MI drop regardless
    assert!(out.resolved.is_empty(), "non-simple base + MI must drop");
}

// -----------------------------------------------------------------------
// 6. Subscript base: class C(Base[int]) -> Barrier -> drop
// -----------------------------------------------------------------------
#[test]
fn inherited_self_subscript_base_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

class C(Base[int]):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    // Subscript is a non-simple base -> Barrier. Single slot, but Barrier -> drop.
    assert!(
        out.resolved.is_empty(),
        "subscript base must be Barrier and drop"
    );
}

// -----------------------------------------------------------------------
// 7. Import rebind: same-file class Base + from ext import Base -> Barrier
// -----------------------------------------------------------------------
#[test]
fn inherited_self_import_rebind_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

from ext import Base

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    // `Base` has 2 top-level bindings (class + import) -> occurrence-dirty -> Barrier.
    assert!(
        out.resolved.is_empty(),
        "import rebind must poison to Barrier"
    );
}

// -----------------------------------------------------------------------
// 8. Wildcard-poison: from x import * -> Barrier
// -----------------------------------------------------------------------
#[test]
fn inherited_self_wildcard_import_drops() {
    let src = "\
from ext import *

class Base:
    def m(self):
        return 1

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "wildcard import must poison all base names to Barrier"
    );
}

// -----------------------------------------------------------------------
// 9. Ambiguous base: two class Base -> Barrier
// -----------------------------------------------------------------------
#[test]
fn inherited_self_ambiguous_base_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

class Base:
    def m(self):
        return 2

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "ambiguous same-name classes must drop"
    );
}

// -----------------------------------------------------------------------
// 10. Nested class excluded (module-scope guard)
// -----------------------------------------------------------------------
#[test]
fn inherited_self_nested_class_excluded() {
    let src = "\
class Base:
    def m(self):
        return 1

def factory():
    class Local(Base):
        def run(self):
            return self.m()
    return Local()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    // The nested class Local should NOT have a class_bases entry.
    // However, `run` calling `self.m()` should still not resolve via inheritance
    // because Local is function-scoped (not in class_bases).
    let caller = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "svc.py"));
    if let Some(caller) = caller {
        if let Some(sites) = cg.calls.get(caller) {
            if let Some(site) = sites.iter().find(|s| s.callee_name == "m") {
                let out = cg.resolve_call_site_full(site);
                assert!(
                    out.resolved.is_empty(),
                    "nested class must not use inherited-self hook"
                );
            }
        }
    }
}

// -----------------------------------------------------------------------
// 11. JS extends analogue
// -----------------------------------------------------------------------
#[test]
fn inherited_self_js_extends() {
    let src = "\
class Base {
    m() { return 1; }
}
class Child extends Base {
    run() { return this.m(); }
}
";
    let cg = CallGraph::build(&files(&[("svc.js", src)]));
    let out = resolve_self_call(&cg, "svc.js", "run", "m");
    assert_eq!(out.resolved.len(), 1, "JS extends should resolve depth-1");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
    assert_eq!(out.resolved[0].target.name, "m");
}

// -----------------------------------------------------------------------
// 12. TS extends analogue
// -----------------------------------------------------------------------
#[test]
fn inherited_self_ts_extends() {
    let src = "\
class Base {
    m(): number { return 1; }
}
class Child extends Base {
    run(): number { return this.m(); }
}
";
    let cg = CallGraph::build(&files(&[("svc.ts", src)]));
    let out = resolve_self_call(&cg, "svc.ts", "run", "m");
    assert_eq!(out.resolved.len(), 1, "TS extends should resolve depth-1");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}

// -----------------------------------------------------------------------
// 13. Same-class method still takes priority over inherited
// -----------------------------------------------------------------------
#[test]
fn inherited_self_same_class_takes_priority() {
    let src = "\
class Base:
    def m(self):
        return 1

class Child(Base):
    def m(self):
        return 2
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    // Should resolve to Child.m, not Base.m
    assert_eq!(out.resolved[0].target.name, "m");
    assert!(
        out.resolved[0].target.start_line > 4,
        "should be Child.m, not Base.m"
    );
}

// -----------------------------------------------------------------------
// 14. Assignment rebind poisons
// -----------------------------------------------------------------------
#[test]
fn inherited_self_assignment_rebind_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

Base = something_else()

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "top-level assignment rebinding Base must poison to Barrier"
    );
}

// -----------------------------------------------------------------------
// 15. Attribute base (module.Base) -> Barrier
// -----------------------------------------------------------------------
#[test]
fn inherited_self_attribute_base_drops() {
    let src = "\
import module

class Child(module.Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    // `module.Base` is an attribute access, non-simple -> Barrier.
    // The call has no target for `m`.
    let caller = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "svc.py"));
    if let Some(caller) = caller {
        if let Some(sites) = cg.calls.get(caller) {
            if let Some(site) = sites.iter().find(|s| s.callee_name == "m") {
                let out = cg.resolve_call_site_full(site);
                assert!(out.resolved.is_empty(), "attribute base must be Barrier");
            }
        }
    }
}

// -----------------------------------------------------------------------
// 16. Rust/Go inert: inherited self should not fire for non Py/JS/TS
// -----------------------------------------------------------------------
#[test]
fn inherited_self_rust_inert() {
    // Rust doesn't use method_class_span, so the hook should be inert.
    let src = "\
struct Base;
impl Base {
    fn m(&self) -> i32 { 1 }
}
struct Child;
impl Child {
    fn run(&self) -> i32 { self.m() }
}
";
    let cg = CallGraph::build(&files(&[("svc.rs", src)]));
    // class_bases should be empty for Rust files
    assert!(
        cg.class_bases.is_empty(),
        "Rust files should have no class_bases entries"
    );
}

// -----------------------------------------------------------------------
// 17. If-block rebind poisons (top-level if is still module scope in Python)
// -----------------------------------------------------------------------
#[test]
fn inherited_self_if_block_rebind_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

if True:
    Base = other

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "if-block assignment rebinding must poison"
    );
}

// -----------------------------------------------------------------------
// 18. Ambiguous caller class span drops inherited lookup
// -----------------------------------------------------------------------
#[test]
fn inherited_self_ambiguous_caller_span_drops() {
    // Two classes on the same line in JS triggers method_class_span_ambiguous
    // for methods within them.  inherited_direct_base must bail.
    let src = "\
class Base { m() { return 1; } }
class Child extends Base { run() { this.m(); } } class Dup extends Base { run() { this.m(); } }
";
    let cg = CallGraph::build(&files(&[("svc.js", src)]));
    // If the caller's FunctionId is in method_class_span_ambiguous the inherited
    // lookup should return None (no resolution).
    let dup_run = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "svc.js" && f.start_line == 2));
    if let Some(caller) = dup_run {
        if let Some(sites) = cg.calls.get(caller) {
            if let Some(site) = sites.iter().find(|s| s.callee_name == "m") {
                let out = cg.resolve_call_site_full(site);
                // With the ambiguous guard the inherited hook bails; resolution
                // should fall through to the normal owner_lookup path or be empty.
                // The key property: it must NOT produce a single Exact via the
                // inherited path (which would be unsound).
                for r in &out.resolved {
                    assert_ne!(
                        r.confidence,
                        ResolutionConfidence::Exact,
                        "ambiguous caller span must not produce Exact via inheritance"
                    );
                }
            }
        }
    }
}

// -----------------------------------------------------------------------
// 19. Wildcard import inside compound suite (if-block) poisons
// -----------------------------------------------------------------------
#[test]
fn inherited_self_wildcard_import_in_compound_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

if True:
    from ext import *

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "wildcard import inside if-block must poison to Barrier"
    );
}

// -----------------------------------------------------------------------
// 20. for-loop header binder at module scope poisons base name
// -----------------------------------------------------------------------
#[test]
fn inherited_self_for_header_rebind_drops() {
    let src = "\
class Base:
    def m(self):
        return 1

for Base in [1, 2, 3]:
    pass

class Child(Base):
    def run(self):
        return self.m()
";
    let cg = CallGraph::build(&files(&[("svc.py", src)]));
    let out = resolve_self_call(&cg, "svc.py", "run", "m");
    assert!(
        out.resolved.is_empty(),
        "for-loop header rebinding must poison base to Barrier"
    );
}
