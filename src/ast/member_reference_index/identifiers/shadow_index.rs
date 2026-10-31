//! Per-owner shadow answers for the simple-name `scoped` search (non-JS-family languages).
//!
//! The old loop asked `is_shadowed_at` once per candidate and definition. Each
//! ask descended from the owner to the candidate with `child_with_descendant`,
//! which walks the children of every node on the way (a goyacc `switch` has
//! thousands of cases), and then re-tested the same scope blocks for every
//! definition of the name.
//!
//! The verdict only depends on two things: which scope blocks strictly between
//! the owner and the candidate declare the name (a property of the candidate,
//! not of the definition) and whether the definition's line lies inside each of
//! those blocks. This module computes the first part once per `(owner, name)`
//! and leaves only the cheap line test for each definition.
//!
//! Exactness: for a non-empty candidate range strictly inside a scope block's
//! range, tree-sitter's nesting makes the block a true ancestor (ranges of
//! distinct nodes either nest or are disjoint). Every other candidate (empty
//! range, outside the owner, or sharing a block's exact range) keeps the
//! original `shadow` call, so the decision is never guessed.
use crate::ast::ParsedFile;
use crate::profile::gates::shadow;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use tree_sitter::Node;

/// Owners smaller than this keep the per-candidate walk: the index costs one
/// owner walk and only repays itself when a name has many occurrences.
pub(super) const MIN_OWNER_BYTES: usize = 8 * 1024;

#[cfg(test)]
thread_local! {
    pub(super) static MODE: std::cell::Cell<Mode> = const { std::cell::Cell::new(Mode::Auto) };
    pub(super) static STATS: std::cell::Cell<Stats> = const { std::cell::Cell::new(Stats { indexed: 0, slow: 0, tree_builds: 0 }) };
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Production gate.
    Auto,
    /// Always the original per-candidate walk (the frozen reference).
    Reference,
    /// The index for every owner, whatever its size.
    Force,
}
#[cfg(test)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) struct Stats {
    /// Candidates decided from the index alone.
    pub indexed: usize,
    /// Candidates that fell back to the original `shadow` walk.
    pub slow: usize,
    pub tree_builds: usize,
}

/// Test switch for graph-level differentials: `Some(true)` runs the original
/// per-candidate walk, `Some(false)` the index for every owner, `None` the gate.
#[cfg(test)]
pub(crate) fn set_mode_for_tests(reference: Option<bool>) {
    MODE.with(|m| {
        m.set(match reference {
            Some(true) => Mode::Reference,
            Some(false) => Mode::Force,
            None => Mode::Auto,
        })
    });
}

/// Scope-tree builds executed on the workers of `pool` (the index ran there).
#[cfg(test)]
pub(crate) fn tree_builds_on(pool: &rayon::ThreadPool) -> usize {
    pool.broadcast(|_| STATS.with(|s| s.get().tree_builds))
        .iter()
        .sum()
}

pub(super) fn enabled_for(parsed: &ParsedFile, owner: &Node<'_>) -> bool {
    #[cfg(test)]
    match MODE.with(|m| m.get()) {
        Mode::Reference => return false,
        Mode::Force => return !is_js_family(parsed),
        Mode::Auto => {}
    }
    !is_js_family(parsed) && owner.byte_range().len() >= MIN_OWNER_BYTES
}

fn is_js_family(parsed: &ParsedFile) -> bool {
    use crate::languages::Language;
    matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

struct Scope {
    start: usize,
    end: usize,
    parent: Option<u32>,
    /// 1-indexed rows, inclusive, exactly as `shadowed_in` computes them.
    rows: (usize, usize),
}

/// Every scope block strictly below the owner, in document (pre)order.
struct ScopeTree<'a> {
    scopes: Vec<Scope>,
    /// `(scope index, name)` for every declaration node whose nearest scope-block
    /// ancestor is that scope: exactly the nodes `scope_has_declaration_uncached`
    /// reaches from it (direct children, and descendants not behind a nested
    /// scope block; a declaration that is itself a scope block counts for its
    /// parent scope). Filled in the same walk, so `declares` is a set lookup.
    declared: HashSet<(u32, &'a str)>,
}

impl<'a> ScopeTree<'a> {
    fn build(parsed: &'a ParsedFile, owner: Node<'a>) -> Self {
        #[cfg(test)]
        STATS.with(|s| {
            let mut v = s.get();
            v.tree_builds += 1;
            s.set(v)
        });
        let mut scopes: Vec<Scope> = Vec::new();
        let mut declared: HashSet<(u32, &'a str)> = HashSet::new();
        // stack[d]: innermost scope visible to the children of the node at depth d.
        let mut stack: Vec<Option<u32>> = Vec::new();
        let mut cursor = owner.walk();
        loop {
            let node = cursor.node();
            let mut visible = stack.last().copied().flatten();
            // Attributed before this node can open a scope of its own. The owner
            // sees no scope (`visible` is `None`), so nothing is recorded for it.
            if let Some(scope) = visible {
                if parsed.language.is_declaration_node(node.kind()) {
                    #[cfg(test)]
                    crate::ast::binding_verdict_memo::probe::observe(
                        "declaration",
                        node.id(),
                        1,
                        "",
                    );
                    if let Some(name) = parsed.language.declaration_name(&node) {
                        declared.insert((scope, parsed.node_text(&name)));
                    }
                }
            }
            if node.id() != owner.id() && parsed.language.is_scope_block(node.kind()) {
                scopes.push(Scope {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    parent: visible,
                    rows: (node.start_position().row + 1, node.end_position().row + 1),
                });
                visible = Some(scopes.len() as u32 - 1);
            }
            stack.push(visible);
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                stack.pop();
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return Self { scopes, declared };
                }
            }
        }
    }

    /// Whether `scope` directly declares `name`; the predicate of
    /// `ParsedFile::scope_has_declaration`.
    fn declares(&self, scope: u32, name: &str) -> bool {
        self.declared.contains(&(scope, name))
    }

    /// Innermost scope whose range starts at or before `start` and ends at or
    /// after `end`, then its ancestors via `parent`.
    fn innermost(&self, start: usize, end: usize) -> Option<u32> {
        let after = self.scopes.partition_point(|s| s.start <= start);
        let mut current = after.checked_sub(1).map(|i| i as u32);
        while let Some(index) = current {
            let scope = &self.scopes[index as usize];
            if scope.end >= end {
                return Some(index);
            }
            current = scope.parent;
        }
        None
    }
}

struct Guarded {
    row: usize,
    guards: std::ops::Range<usize>,
}

/// Decisions for one name in one owner, independent of the definition line.
struct NameShadow<'a> {
    /// Rows of candidates no declaring scope block can shadow; sorted, unique.
    plain_rows: Vec<usize>,
    guarded: Vec<Guarded>,
    /// Row ranges of the declaring ancestor blocks, shared by `guarded`.
    guard_rows: Vec<(usize, usize)>,
    /// Candidates the index cannot decide exactly.
    slow: Vec<(usize, Node<'a>)>,
}

struct OwnerShadow<'a> {
    key: (usize, usize, usize),
    tree: ScopeTree<'a>,
    names: HashMap<String, NameShadow<'a>>,
}

#[derive(Default)]
pub(super) struct ShadowIndex<'a> {
    owner: RefCell<Option<OwnerShadow<'a>>>,
}

impl<'a> ShadowIndex<'a> {
    /// Lines of the candidates that are not shadowed for `def_line`, equal to
    /// the lines the per-candidate `shadow` loop keeps.
    pub(super) fn lines<const COUNT: bool>(
        &self,
        parsed: &'a ParsedFile,
        owner: &Node<'a>,
        name: &str,
        def_line: usize,
        candidates: impl FnOnce() -> Vec<Node<'a>>,
    ) -> BTreeSet<usize> {
        let key = (owner.id(), owner.start_byte(), owner.end_byte());
        let mut slot = self.owner.borrow_mut();
        if slot.as_ref().is_none_or(|cached| cached.key != key) {
            *slot = Some(OwnerShadow {
                key,
                tree: ScopeTree::build(parsed, *owner),
                names: HashMap::new(),
            });
        }
        let cached = slot.as_mut().unwrap();
        if !cached.names.contains_key(name) {
            let built = NameShadow::build(owner, &cached.tree, name, candidates());
            cached.names.insert(name.to_owned(), built);
        }
        let shadow_for = &cached.names[name];

        let mut rows = shadow_for.plain_rows.clone();
        let mut extra = false;
        for guarded in &shadow_for.guarded {
            let shadowed = shadow_for.guard_rows[guarded.guards.clone()]
                .iter()
                .any(|&(lo, hi)| !(def_line >= lo && def_line <= hi));
            #[cfg(test)]
            bump(|s| s.indexed += 1);
            if !shadowed {
                rows.push(guarded.row);
                extra = true;
            }
        }
        for (row, node) in &shadow_for.slow {
            #[cfg(test)]
            bump(|s| s.slow += 1);
            if !shadow::<COUNT>(parsed, node, owner, name, def_line) {
                rows.push(*row);
                extra = true;
            }
        }
        if extra {
            rows.sort_unstable();
            rows.dedup();
        }
        rows.into_iter().collect()
    }
}

#[cfg(test)]
fn bump(update: impl FnOnce(&mut Stats)) {
    STATS.with(|s| {
        let mut v = s.get();
        update(&mut v);
        s.set(v)
    });
}

impl<'a> NameShadow<'a> {
    fn build(
        owner: &Node<'a>,
        tree: &ScopeTree<'a>,
        name: &str,
        candidates: Vec<Node<'a>>,
    ) -> Self {
        let (owner_start, owner_end) = (owner.start_byte(), owner.end_byte());
        let mut plain = Vec::new();
        let mut guarded = Vec::new();
        let mut guard_rows = Vec::new();
        let mut slow = Vec::new();
        'candidates: for node in candidates {
            let (start, end) = (node.start_byte(), node.end_byte());
            let row = node.start_position().row + 1;
            let inside_owner = start < end
                && owner_start <= start
                && end <= owner_end
                && (start, end) != (owner_start, owner_end);
            if !inside_owner {
                slow.push((row, node));
                continue;
            }
            let first_guard = guard_rows.len();
            let mut current = tree.innermost(start, end);
            while let Some(index) = current {
                let scope = &tree.scopes[index as usize];
                if (scope.start, scope.end) == (start, end) {
                    // Same range as the candidate: ancestry is not implied.
                    guard_rows.truncate(first_guard);
                    slow.push((row, node));
                    continue 'candidates;
                }
                if tree.declares(index, name) {
                    guard_rows.push(scope.rows);
                }
                current = scope.parent;
            }
            if guard_rows.len() == first_guard {
                plain.push(row);
            } else {
                guarded.push(Guarded {
                    row,
                    guards: first_guard..guard_rows.len(),
                });
            }
        }
        plain.sort_unstable();
        plain.dedup();
        Self {
            plain_rows: plain,
            guarded,
            guard_rows,
            slow,
        }
    }
}

#[cfg(test)]
mod tests;
