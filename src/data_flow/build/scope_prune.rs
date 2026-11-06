//! Rule S (#630 slice 3), Go only: drop a (def, use) pair when the two resolve to
//! different explicit declarations of one name. No CFG or RD label is consulted.
//!
//! A different declaration is not different storage (#687): `*x = 7` normalizes
//! to path `x`, and `x := x` copies the pointer. So a pair is dropped only when
//! the def writes the variable itself and no use-line occurrence that resolves
//! elsewhere is a storage access through the name. Anything unproven is kept.
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::cpg::{DefSite, RdFileStats};
use crate::languages::Language;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use tree_sitter::Node;

#[derive(Clone)]
struct Occurrence {
    binding: BindingId,
    /// `*x`, `x[i]`, `x[i:j]`, `x.f`, `x.M()`, `(x)`: an access through the name.
    storage: bool,
}

type LineKey = (String, usize);
type LineCache = RefCell<BTreeMap<LineKey, Rc<Vec<Occurrence>>>>;

pub(super) struct ScopeFacts<'a> {
    parsed: &'a ParsedFile,
    facts: Option<BindingFacts>,
    defs: &'a [DefSite],
    /// `(start byte, path)` of every genuine (not alias-derived) def.
    def_index: BTreeMap<(usize, AccessPath), usize>,
    /// `(name, line)` of every genuine def, for same-line coalescing.
    defs_on_line: BTreeMap<LineKey, Vec<usize>>,
    /// Identifier nodes by `(name, line)`.
    identifiers: BTreeMap<LineKey, Vec<Node<'a>>>,
    lines: LineCache,
    clean_names: RefCell<BTreeMap<String, bool>>,
}

impl<'a> ScopeFacts<'a> {
    pub(super) fn new(parsed: &'a ParsedFile, owner: Node<'a>, defs: &'a [DefSite]) -> Self {
        let enabled = parsed.language == Language::Go && !owner.has_error() && !defs.is_empty();
        let mut this = Self {
            parsed,
            facts: None,
            defs,
            def_index: BTreeMap::new(),
            defs_on_line: BTreeMap::new(),
            identifiers: BTreeMap::new(),
            lines: RefCell::new(BTreeMap::new()),
            clean_names: RefCell::new(BTreeMap::new()),
        };
        if !enabled {
            return this;
        }
        // The producer has no emitted edges yet, so the use-byte table is empty;
        // RD keeps its own instance because its table is built from the edges.
        this.facts = Some(BindingFacts::new(parsed, &owner, defs, std::iter::empty()));
        for (index, def) in defs.iter().enumerate().filter(|(_, d)| !d.alias_derived) {
            this.def_index
                .insert((def.start_byte, def.path.clone()), index);
            this.defs_on_line
                .entry((def.path.base.clone(), def.line))
                .or_default()
                .push(index);
        }
        this.collect(owner);
        this
    }

    fn collect(&mut self, node: Node<'a>) {
        if node.kind() == "identifier" {
            self.identifiers
                .entry((
                    self.parsed.node_text(&node).to_owned(),
                    node.start_position().row + 1,
                ))
                .or_default()
                .push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect(child);
        }
    }

    /// Reads on `line`; declaration-introducing names are writes and are omitted.
    fn occurrences(&self, facts: &BindingFacts, name: &str, line: usize) -> Rc<Vec<Occurrence>> {
        let key = (name.to_owned(), line);
        if let Some(found) = self.lines.borrow().get(&key) {
            return found.clone();
        }
        let list: Vec<Occurrence> = self
            .identifiers
            .get(&key)
            .into_iter()
            .flatten()
            .filter(|node| !is_declaring_name(**node))
            .map(|node| Occurrence {
                binding: facts.binding_at(name, node.start_byte()),
                storage: is_storage_access(*node),
            })
            .collect();
        let list = Rc::new(list);
        self.lines.borrow_mut().insert(key, list.clone());
        list
    }

    fn name_is_clean(&self, facts: &BindingFacts, name: &str) -> bool {
        *self
            .clean_names
            .borrow_mut()
            .entry(name.to_owned())
            .or_insert_with(|| facts.unclassified_binding_lines(name).is_empty())
    }

    /// The declaration a def binds, only when the def writes the variable itself.
    fn plain_def_binding(
        &self,
        facts: &BindingFacts,
        path: &AccessPath,
        start: usize,
        end: usize,
        line: usize,
    ) -> Option<usize> {
        if !path.fields.is_empty() {
            return None;
        }
        let index = *self.def_index.get(&(start, path.clone()))?;
        let BindingId::Declaration(declaration) = facts.binding_of_def(index) else {
            return None;
        };
        let node = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(start, end)?;
        if node.kind() != "identifier"
            || node.start_byte() != start
            || node.end_byte() != end
            || !is_plain_write_site(node)
        {
            return None;
        }
        // Same-line defs of one path are coalesced to the first span: each must
        // bind this declaration, and no occurrence on the line may be a storage
        // access (`x, *x = x, 7`, `x = new(int); *x = 7`).
        let coalesced = self.defs_on_line.get(&(path.base.clone(), line))?;
        if coalesced.iter().any(|i| {
            self.defs[*i].path == *path && facts.binding_of_def(*i) != facts.binding_of_def(index)
        }) {
            return None;
        }
        if self
            .occurrences(facts, &path.base, line)
            .iter()
            .any(|o| o.storage)
            || self
                .identifiers
                .get(&(path.base.clone(), line))
                .into_iter()
                .flatten()
                .any(|n| is_storage_access(*n))
        {
            return None;
        }
        Some(*declaration)
    }

    /// False (drop the pair) only when every read of the name on `use_line`
    /// resolves to a different explicit declaration without a storage access.
    pub(super) fn keep_pair(
        &self,
        path: &AccessPath,
        def_start: usize,
        def_end: usize,
        def_line: usize,
        use_line: usize,
        stats: &mut RdFileStats,
    ) -> bool {
        let Some(facts) = &self.facts else {
            return true;
        };
        if !self.name_is_clean(facts, &path.base) {
            return true;
        }
        let Some(declaration) = self.plain_def_binding(facts, path, def_start, def_end, def_line)
        else {
            return true;
        };
        let reads = self.occurrences(facts, &path.base, use_line);
        let drop = !reads.is_empty()
            && reads.iter().all(|o| {
                !o.storage
                    && matches!(&o.binding, BindingId::Declaration(other) if *other != declaration)
            });
        if drop {
            stats.dfg_pruned_scope += 1;
        }
        !drop
    }
}

/// A name introduced by `:=`, `var`/`const` or a parameter list is a write of
/// that declaration, never a read.
fn is_declaring_name(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let in_field = |owner: Node<'_>, field: &str| {
        owner
            .child_by_field_name(field)
            .is_some_and(|f| f.start_byte() <= node.start_byte() && node.end_byte() <= f.end_byte())
    };
    match parent.kind() {
        "var_spec" | "const_spec" => in_field(parent, "name"),
        "parameter_declaration" | "variadic_parameter_declaration" => in_field(parent, "name"),
        "expression_list" => parent
            .parent()
            .is_some_and(|g| g.kind() == "short_var_declaration" && in_field(g, "left")),
        _ => false,
    }
}

fn is_storage_access(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let is_operand = |field: &str| {
        parent
            .child_by_field_name(field)
            .is_some_and(|f| f.id() == node.id())
    };
    match parent.kind() {
        "unary_expression" => {
            is_operand("operand")
                && parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "*")
        }
        "index_expression" | "slice_expression" | "selector_expression" => is_operand("operand"),
        "parenthesized_expression" => true,
        _ => false,
    }
}

/// The identifier is the written variable itself, in a statement position that
/// is not an operand of `*`, indexing, slicing or selection.
fn is_plain_write_site(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let within = |owner: Node<'_>, field: &str| {
        owner
            .child_by_field_name(field)
            .is_some_and(|f| f.start_byte() <= node.start_byte() && node.end_byte() <= f.end_byte())
    };
    match parent.kind() {
        "var_spec" | "const_spec" | "parameter_declaration" | "variadic_parameter_declaration" => {
            within(parent, "name")
        }
        "inc_statement" | "dec_statement" => true,
        "type_switch_statement" => within(parent, "alias"),
        "expression_list" => parent.parent().is_some_and(|g| {
            matches!(
                g.kind(),
                "assignment_statement"
                    | "short_var_declaration"
                    | "range_clause"
                    | "receive_statement"
            ) && within(g, "left")
        }),
        _ => false,
    }
}
