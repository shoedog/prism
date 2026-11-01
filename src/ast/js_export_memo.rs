//! Export-extraction-local memoization of the existing refusal predicates.
//!
//! A guard borrows one immutable ParsedFile. TLS holds weak registrations only;
//! caches contain owned names and node identities, never AST nodes. This avoids
//! stale facts when the public source/tree fields are replaced between calls.
//! Rayon file jobs have independent sessions, and nesting/unwinding is supported.
use super::ParsedFile;
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use tree_sitter::Node;

mod inventory;
#[cfg(test)]
mod oracle;
#[cfg(test)]
mod tests;
mod walks;

#[derive(Default)]
struct Memo {
    inventory: OnceCell<inventory::Inventory>,
    lexical: RefCell<HashMap<(usize, String), bool>>,
    written: RefCell<HashMap<(String, bool), bool>>,
    replaced: RefCell<HashMap<usize, bool>>,
}
struct Registration {
    file: usize,
    root: usize,
    memo: Weak<Memo>,
}
thread_local! {
    static ACTIVE: RefCell<Vec<Registration>> = const { RefCell::new(Vec::new()) };
}
pub(super) struct Session<'a> {
    _parsed: &'a ParsedFile,
    memo: Rc<Memo>,
}
impl<'a> Session<'a> {
    pub(super) fn new(p: &'a ParsedFile) -> Self {
        let memo = Rc::new(Memo::default());
        ACTIVE.with(|slot| {
            slot.borrow_mut().push(Registration {
                file: p as *const _ as usize,
                root: p.tree.root_node().id(),
                memo: Rc::downgrade(&memo),
            })
        });
        Self { _parsed: p, memo }
    }
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        let weak = Rc::downgrade(&self.memo);
        ACTIVE.with(|slot| slot.borrow_mut().retain(|r| !Weak::ptr_eq(&r.memo, &weak)));
    }
}
fn active(p: &ParsedFile) -> Option<Rc<Memo>> {
    ACTIVE.with(|slot| {
        slot.borrow()
            .iter()
            .rev()
            .find(|r| r.file == p as *const _ as usize && r.root == p.tree.root_node().id())
            .and_then(|r| r.memo.upgrade())
    })
}

impl ParsedFile {
    pub(in crate::ast) fn js_ts_unshadowed_ambient(&self, identifier: Node<'_>) -> bool {
        #[cfg(test)]
        if tests::reference() {
            tests::count(|s| s.references += 1);
            return self.js_ts_unshadowed_ambient_reference(identifier);
        }
        !self.js_ts_export_lexically_bound(identifier)
            && !self.js_ts_module_value_written(self.node_text(&identifier))
    }

    fn js_ts_export_lexically_bound(&self, identifier: Node<'_>) -> bool {
        let evaluate = || {
            #[cfg(test)]
            tests::count(|s| s.lexical += 1);
            self.js_ts_receiver_lexically_bound_at_call(&self.tree.root_node(), Some(identifier))
        };
        let Some(memo) = active(self) else {
            return evaluate();
        };
        if identifier.into_raw().tree != self.tree.root_node().into_raw().tree {
            return evaluate();
        }
        let inventory = memo.inventory.get_or_init(|| {
            #[cfg(test)]
            tests::count(|s| s.inventories += 1);
            inventory::Inventory::new(self)
        });
        let name = self.node_text(&identifier);
        let Some(&scope) = inventory.scopes.get(&identifier.id()) else {
            return evaluate();
        };
        if !inventory.recovery && !inventory.names.contains(name) {
            return false;
        }
        // The old predicate uses only enclosing function membership, lexical
        // scope membership, class-expression/namespace membership, and spelling.
        // It never compares declaration/call byte order. Every such boundary is
        // recorded in Inventory, so this key preserves all receiver-dependent inputs.
        let key = (scope, name.to_owned());
        if let Some(&answer) = memo.lexical.borrow().get(&key) {
            return answer;
        }
        let answer = evaluate();
        memo.lexical.borrow_mut().insert(key, answer);
        answer
    }

    /// Replacing module.exports detaches exports throughout the whole file.
    pub(in crate::ast) fn js_ts_module_exports_replaced(&self, node: Node<'_>) -> bool {
        #[cfg(test)]
        if tests::reference() {
            tests::count(|s| s.references += 1);
            return self.js_ts_module_exports_replaced_reference(node);
        }
        let evaluate = || {
            #[cfg(test)]
            tests::count(|s| s.replacements += 1);
            self.js_ts_module_exports_replaced_uncached(node)
        };
        let Some(memo) = active(self) else {
            return evaluate();
        };
        if node.into_raw().tree != self.tree.root_node().into_raw().tree {
            return evaluate();
        }
        if let Some(&answer) = memo.replaced.borrow().get(&node.id()) {
            return answer;
        }
        let answer = evaluate();
        memo.replaced.borrow_mut().insert(node.id(), answer);
        answer
    }

    /// Module writes preserve the original source-self-binding policy.
    pub(crate) fn js_ts_module_value_written(&self, name: &str) -> bool {
        self.js_ts_module_written(name, false)
    }

    pub(crate) fn js_ts_module_written(&self, name: &str, skip_using: bool) -> bool {
        #[cfg(test)]
        if tests::reference() {
            tests::count(|s| s.references += 1);
            return self.js_ts_module_written_reference(name, skip_using);
        }
        let evaluate = || {
            #[cfg(test)]
            tests::count(|s| s.writes += 1);
            self.js_ts_module_written_uncached(name, skip_using)
        };
        let Some(memo) = active(self) else {
            return evaluate();
        };
        let key = (name.to_owned(), skip_using);
        if let Some(&answer) = memo.written.borrow().get(&key) {
            return answer;
        }
        let answer = evaluate();
        memo.written.borrow_mut().insert(key, answer);
        answer
    }
}
