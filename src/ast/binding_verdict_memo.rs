//! Binding sub-results for one immutable parsed-file search session.
//!
//! The guard borrows the exact ParsedFile, so tree/source replacement cannot
//! overlap the cache lifetime. TLS contains only weak registrations, never AST
//! nodes or an owning cache. Rc makes the guard thread confined; rayon file jobs
//! register independent sessions. See the lane DESIGN.md for key completeness.
use super::ParsedFile;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use tree_sitter::Node;

type Binding = Option<(usize, (usize, usize))>;
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum Key {
    Scope(usize, u8),
    At(usize),
    Seam(usize),
    Declaration(usize),
}
#[derive(Clone, Copy)]
enum Value {
    Bool(bool),
    Binding(Binding),
}
#[derive(Default)]
struct Memo {
    names: HashMap<String, HashMap<Key, Value>>,
}
struct Registration {
    file: usize,
    root: usize,
    memo: Weak<RefCell<Memo>>,
}
thread_local! {
    static ACTIVE: RefCell<Vec<Registration>> = const { RefCell::new(Vec::new()) };
}

/// Lives alongside MemberReferenceSearch, including all gates in a DFG file job.
/// Out-of-order drop and overlapping searches/files are supported.
pub(crate) struct Session<'a> {
    _parsed: &'a ParsedFile,
    memo: Rc<RefCell<Memo>>,
}
impl<'a> Session<'a> {
    pub(crate) fn new(parsed: &'a ParsedFile) -> Self {
        let memo = Rc::new(RefCell::new(Memo::default()));
        ACTIVE.with(|slot| {
            slot.borrow_mut().push(Registration {
                file: parsed as *const ParsedFile as usize,
                root: parsed.tree.root_node().id(),
                memo: Rc::downgrade(&memo),
            })
        });
        Self {
            _parsed: parsed,
            memo,
        }
    }
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        let weak = Rc::downgrade(&self.memo);
        ACTIVE.with(|slot| {
            slot.borrow_mut()
                .retain(|entry| !Weak::ptr_eq(&entry.memo, &weak))
        });
    }
}

fn cached(
    p: &ParsedFile,
    name: &str,
    key: impl FnOnce() -> Key,
    evaluate: impl FnOnce() -> Value,
) -> Value {
    #[cfg(test)]
    if BYPASS.with(|b| b.get()) {
        return evaluate();
    }
    let memo = ACTIVE.with(|slot| {
        slot.borrow()
            .iter()
            .rev()
            .find(|entry| {
                entry.file == p as *const ParsedFile as usize
                    && entry.root == p.tree.root_node().id()
            })
            .and_then(|entry| entry.memo.upgrade())
    });
    let Some(memo) = memo else {
        return evaluate();
    };
    let raw_name = name;
    let key = key();
    let hit = memo
        .borrow()
        .names
        .get(raw_name)
        .and_then(|table| table.get(&key))
        .copied();
    if let Some(hit) = hit {
        return hit;
    }
    // No RefCell borrow crosses a recursive binder evaluation.
    let value = evaluate();
    memo.borrow_mut()
        .names
        .entry(raw_name.to_owned())
        .or_default()
        .insert(key, value);
    value
}
fn boolean(value: Value) -> bool {
    match value {
        Value::Bool(v) => v,
        _ => unreachable!("binding memo key/value mismatch"),
    }
}

pub(super) fn cacheable(scope: Node<'_>, boundaries: &[&str]) -> bool {
    boundaries.contains(&scope.kind())
        || matches!(
            scope.kind(),
            "statement_block"
                | "switch_body"
                | "enum_body"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "class_static_block"
                | "for_statement"
                | "for_in_statement"
                | "catch_clause"
        )
}
impl ParsedFile {
    /// These are exactly the byte-dependent predicates read by scope_binds.
    /// Defaults share the parameter environment. Computed method keys set the
    /// generic name bit, which the method binder ignores.
    pub(super) fn binding_verdict_region(
        &self,
        scope: Node<'_>,
        boundaries: &[&str],
        byte: usize,
    ) -> u8 {
        if !boundaries.contains(&scope.kind()) {
            return 0;
        }
        let inside =
            |n: Option<Node<'_>>| n.is_some_and(|n| n.start_byte() <= byte && byte < n.end_byte());
        u8::from(inside(scope.child_by_field_name("body")))
            | u8::from(inside(self.parameter_binding_region(&scope))) << 1
            | u8::from(inside(scope.child_by_field_name("name"))) << 2
            | u8::from(inside(scope.child_by_field_name("return_type"))) << 3
    }

    pub(super) fn js_ts_scope_binds(
        &self,
        scope: Node<'_>,
        boundaries: &[&str],
        name: &str,
        byte: usize,
    ) -> bool {
        debug_assert_eq!(boundaries, self.language.callable_boundary_node_types());
        if !cacheable(scope, boundaries) {
            return self.js_ts_scope_binds_uncached(scope, boundaries, name, byte);
        }
        boolean(cached(
            self,
            name,
            || {
                Key::Scope(
                    scope.id(),
                    self.binding_verdict_region(scope, boundaries, byte),
                )
            },
            || Value::Bool(self.js_ts_scope_binds_uncached(scope, boundaries, name, byte)),
        ))
    }

    pub(super) fn js_ts_binding_scope_at(&self, byte: usize, name: &str) -> Binding {
        match cached(
            self,
            name,
            || Key::At(byte),
            || Value::Binding(self.js_ts_binding_scope_at_uncached(byte, name)),
        ) {
            Value::Binding(v) => v,
            _ => unreachable!("binding memo key/value mismatch"),
        }
    }

    pub(crate) fn js_ts_seam_binding(&self, owner: &Node<'_>, name: &str) -> bool {
        boolean(cached(
            self,
            name,
            || Key::Seam(owner.id()),
            || Value::Bool(self.js_ts_seam_binding_uncached(owner, name)),
        ))
    }

    pub(super) fn scope_has_declaration(&self, scope: Node<'_>, name: &str) -> bool {
        boolean(cached(
            self,
            name,
            || Key::Declaration(scope.id()),
            || Value::Bool(self.scope_has_declaration_uncached(scope, name)),
        ))
    }
}

#[cfg(test)]
thread_local! { static BYPASS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// The original binder, including all recursive helper calls, without any memo.
#[cfg(test)]
pub(crate) fn direct<R>(run: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            BYPASS.with(|b| b.set(self.0));
        }
    }
    let _restore = Restore(BYPASS.with(|b| b.replace(true)));
    run()
}
#[cfg(test)]
#[path = "binding_verdict_probe.rs"]
pub(crate) mod probe;
#[cfg(test)]
mod tests;
