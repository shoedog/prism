//! One lazy member-candidate index for an immutable parsed-file borrow.
//!
//! Nodes borrow the exact tree, so neither reparsing nor partial-cache rebuilds
//! can reuse this session. Only normalization/traversal is shared; binding and
//! read decisions remain query-specific.
use super::*;
use std::cell::OnceCell;
use std::collections::HashMap;

struct Candidate<'a> {
    node: Node<'a>,
    preorder: usize,
    subtree_end: usize,
}

struct Index<'a> {
    by_path: HashMap<AccessPath, Vec<Candidate<'a>>>,
    root: Node<'a>,
}

impl<'a> Index<'a> {
    fn build(parsed: &'a ParsedFile) -> Self {
        let mut index = Self {
            by_path: HashMap::new(),
            root: parsed.tree.root_node(),
        };
        let mut cursor = index.root.walk();
        let mut frames = Vec::new();
        let mut next = 0;
        loop {
            let node = cursor.node();
            let preorder = next;
            next += 1;
            let slot = if ParsedFile::is_field_access_node(node.kind()) {
                #[cfg(test)]
                count_normalization();
                let path = parsed
                    .bounded_member_path(node)
                    .map(|member| member.path)
                    .unwrap_or_else(|| AccessPath::from_expr(parsed.node_text(&node)));
                let candidates = index.by_path.entry(path.clone()).or_default();
                let slot = candidates.len();
                candidates.push(Candidate {
                    node,
                    preorder,
                    subtree_end: 0,
                });
                Some((path, slot))
            } else {
                None
            };
            frames.push(slot);
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if let Some((path, slot)) = frames.pop().unwrap() {
                    index.by_path.get_mut(&path).unwrap()[slot].subtree_end = next;
                }
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return index;
                }
            }
        }
    }

    fn exact_descendant(&self, root: Node<'a>, c: &Candidate<'a>) -> bool {
        #[cfg(test)]
        EXACT_CHECKS.with(|n| n.set(n.get() + 1));
        // The build and goto_descendant use the same all-child preorder.
        // Unlike Node::parent(), the cursor retains its descent context;
        // ascending it tests exact node identity without restarting at the root.
        let mut cursor = self.root.walk();
        cursor.goto_descendant(c.preorder);
        loop {
            if cursor.node() == root {
                return true;
            }
            if !cursor.goto_parent() {
                return false;
            }
        }
    }

    fn matching<'b>(
        &'b self,
        root: Node<'a>,
        path: &AccessPath,
        def_line: usize,
    ) -> impl Iterator<Item = Node<'a>> + 'b {
        let candidates = self.by_path.get(path).map(Vec::as_slice).unwrap_or(&[]);
        let start = candidates.partition_point(|c| c.node.start_byte() < root.start_byte());
        let mut suppressed_until = 0;
        candidates[start..]
            .iter()
            .take_while(move |c| c.node.start_byte() <= root.end_byte())
            .filter(move |c| {
                #[cfg(test)]
                EXAMINED.with(|n| n.set(n.get() + 1));
                let line = c.node.start_position().row + 1;
                if c.preorder < suppressed_until
                    || c.node.end_byte() > root.end_byte()
                    || line <= def_line
                {
                    return false;
                }
                // Non-empty, contained, unequal ranges imply ancestry because
                // tree-sitter ranges nest. Equal/empty ranges need node identity.
                let interior = c.node.start_byte() < c.node.end_byte()
                    && (c.node.start_byte(), c.node.end_byte())
                        != (root.start_byte(), root.end_byte());
                if interior || self.exact_descendant(root, c) {
                    // The walker returns even when a subsequent gate refuses.
                    suppressed_until = c.subtree_end;
                    return true;
                }
                false
            })
            .map(|c| c.node)
    }
}

pub(crate) struct MemberReferenceSearch<'a> {
    parsed: &'a ParsedFile,
    index: OnceCell<Index<'a>>,
    _bindings: super::binding_verdict_memo::Session<'a>,
}

impl<'a> MemberReferenceSearch<'a> {
    pub(crate) fn new(parsed: &'a ParsedFile) -> Self {
        Self {
            parsed,
            index: OnceCell::new(),
            _bindings: super::binding_verdict_memo::Session::new(parsed),
        }
    }

    fn matching(
        &self,
        root: Node<'a>,
        path: &AccessPath,
        def_line: usize,
    ) -> impl Iterator<Item = Node<'a>> + '_ {
        self.index
            .get_or_init(|| Index::build(self.parsed))
            .matching(root, path, def_line)
    }

    fn scoped<const COUNT: bool>(
        &self,
        root: &Node<'a>,
        path: &AccessPath,
        def_line: usize,
    ) -> BTreeSet<usize> {
        if path.is_simple() {
            return crate::profile::gates::scoped_refs::<COUNT>(
                self.parsed,
                root,
                &path.base,
                def_line,
            );
        }
        self.matching(*root, path, def_line)
            .map(|node| node.start_position().row + 1)
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn find_path_reference_spans_fenced_in(
        &self,
        owner: &Node<'a>,
        path: &AccessPath,
        def_line: usize,
        def_byte: usize,
        within: Option<(usize, usize)>,
        reads_only: bool,
    ) -> BTreeMap<usize, (usize, usize)> {
        self.find_profiled::<false>(owner, path, def_line, def_byte, within, reads_only)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn find_profiled<const COUNT: bool>(
        &self,
        owner: &Node<'a>,
        path: &AccessPath,
        def_line: usize,
        def_byte: usize,
        within: Option<(usize, usize)>,
        reads_only: bool,
    ) -> BTreeMap<usize, (usize, usize)> {
        let parsed = self.parsed;
        #[cfg(test)]
        if USE_ORACLE.with(|flag| flag.get()) {
            return parsed.legacy_path_reference_spans_fenced_in(
                owner, path, def_line, def_byte, within, reads_only,
            );
        }
        if !matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || (!reads_only
            && parsed.language.function_name(owner).is_some()
            && parsed.js_ts_seam_binding(owner, &path.base))
        {
            return self
                .scoped::<COUNT>(owner, path, def_line)
                .into_iter()
                .map(|line| {
                    (
                        line,
                        (parsed.line_start_byte(line), parsed.line_start_byte(line)),
                    )
                })
                .collect();
        }
        let root = within
            .and_then(|(s, e)| parsed.tree.root_node().descendant_for_byte_range(s, e))
            .filter(|n| within == Some((n.start_byte(), n.end_byte())))
            .unwrap_or(*owner);
        let mut lines = BTreeMap::new();
        if path.is_simple() {
            parsed.collect_fenced_simple_refs::<COUNT>(
                root, owner, &path.base, def_line, def_byte, reads_only, &mut lines,
            );
        } else {
            for node in self.matching(root, path, def_line) {
                let refused_read = reads_only
                    && ((path.base == "this" && parsed.js_ts_this_rebound_between(&node, owner))
                        || parsed.js_ts_identifier_is_not_a_read(node)
                        || parsed.js_ts_reference_in_with_body(node));
                if !refused_read
                    && !crate::profile::gates::fence::<COUNT>(
                        parsed, &node, owner, &path.base, def_byte,
                    )
                {
                    lines
                        .entry(node.start_position().row + 1)
                        .or_insert((node.start_byte(), node.end_byte()));
                }
            }
        }
        lines
    }
}

impl ParsedFile {
    /// First admitted occurrence per line, preserving the legacy traversal gates.
    pub(crate) fn find_path_reference_spans_fenced_in(
        &self,
        owner: &Node<'_>,
        path: &AccessPath,
        def_line: usize,
        def_byte: usize,
        within: Option<(usize, usize)>,
        reads_only: bool,
    ) -> BTreeMap<usize, (usize, usize)> {
        MemberReferenceSearch::new(self).find_path_reference_spans_fenced_in(
            owner, path, def_line, def_byte, within, reads_only,
        )
    }

    /// Member paths retain the strict after-definition line cutoff. Simple
    /// paths retain the existing variable-reference query without this cutoff.
    pub fn find_path_references_scoped(
        &self,
        owner: &Node<'_>,
        path: &AccessPath,
        def_line: usize,
    ) -> BTreeSet<usize> {
        MemberReferenceSearch::new(self).scoped::<false>(owner, path, def_line)
    }
}

#[cfg(test)]
thread_local! { static NORMALIZATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(super) fn count_normalization() {
    NORMALIZATIONS.with(|n| n.set(n.get() + 1));
}
#[cfg(test)]
thread_local! { static USE_ORACLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
mod tests;

#[cfg(test)]
mod oracle;

#[cfg(test)]
thread_local! {
    static EXAMINED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXACT_CHECKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
mod regressions;
