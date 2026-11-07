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
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use tree_sitter::Node;

#[derive(Clone)]
struct Occurrence {
    binding: BindingId,
    /// `*x`, `x[i]`, `x[i:j]`, `x.f`, `x.M()`, `(x)`: an access through the name.
    storage: bool,
}

type LineKey = (String, usize);
/// Per use line, the classified reads of each name; a hit allocates nothing.
type LineCache = RefCell<HashMap<usize, Vec<(String, Rc<Vec<Occurrence>>)>>>;
/// Per def `(start, end, line)`, the plain-def verdict of each base name.
type DefCache = RefCell<HashMap<(usize, usize, usize), Vec<(String, Option<(usize, usize)>)>>>;

pub(super) struct ScopeFacts<'a> {
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    enabled: bool,
    /// Names with two or more declaration sites in the owner. A pair can only
    /// be dropped for such a name, so nothing else is ever built for others.
    multi: BTreeSet<&'a str>,
    /// Names whose address is taken anywhere in the owner (`&x`, `&(x)`,
    /// `&x.f`, `&x[i]`; a different declaration may share their storage).
    address_taken: BTreeSet<&'a str>,
    built: OnceCell<Built<'a>>,
    defs: &'a [DefSite],
    lines: LineCache,
    plain_defs: DefCache,
    clean_names: RefCell<HashMap<String, bool>>,
}

/// The RD-sized state, built on the first pair of a multiply declared name.
struct Built<'a> {
    facts: BindingFacts,
    /// `(start byte, path)` of every genuine (not alias-derived) def.
    def_index: BTreeMap<(usize, AccessPath), usize>,
    /// `(name, line)` of every genuine def, for same-line coalescing.
    defs_on_line: BTreeMap<LineKey, Vec<usize>>,
    /// Identifier nodes by `(name, line)`.
    identifiers: BTreeMap<LineKey, Vec<Node<'a>>>,
}

impl<'a> ScopeFacts<'a> {
    pub(super) fn new(parsed: &'a ParsedFile, owner: Node<'a>, defs: &'a [DefSite]) -> Self {
        let enabled = parsed.language == Language::Go && !owner.has_error() && !defs.is_empty();
        let multi = if enabled {
            multiply_declared(parsed, owner)
        } else {
            BTreeSet::new()
        };
        let address_taken = if enabled && !multi.is_empty() {
            address_taken_names(parsed, owner, &multi)
        } else {
            BTreeSet::new()
        };
        Self {
            parsed,
            owner,
            enabled: enabled && !multi.is_empty(),
            multi,
            address_taken,
            built: OnceCell::new(),
            defs,
            lines: RefCell::new(HashMap::new()),
            plain_defs: RefCell::new(HashMap::new()),
            clean_names: RefCell::new(HashMap::new()),
        }
    }

    fn build(&self) -> Built<'a> {
        // The producer has no emitted edges yet, so the use-byte table is empty;
        // RD keeps its own instance because its table is built from the edges.
        let facts = BindingFacts::new(self.parsed, &self.owner, self.defs, std::iter::empty());
        let mut built = Built {
            facts,
            def_index: BTreeMap::new(),
            defs_on_line: BTreeMap::new(),
            identifiers: BTreeMap::new(),
        };
        for (index, def) in self
            .defs
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.alias_derived)
        {
            built
                .def_index
                .insert((def.start_byte, def.path.clone()), index);
            built
                .defs_on_line
                .entry((def.path.base.clone(), def.line))
                .or_default()
                .push(index);
        }
        self.collect(self.owner, &mut built);
        built
    }

    fn collect(&self, node: Node<'a>, built: &mut Built<'a>) {
        if node.kind() == "identifier" {
            let text = self.parsed.node_text(&node);
            if self.multi.contains(text) {
                built
                    .identifiers
                    .entry((text.to_owned(), node.start_position().row + 1))
                    .or_default()
                    .push(node);
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect(child, built);
        }
    }

    /// Reads on `line`; declaration-introducing names are writes and are omitted.
    fn occurrences(&self, built: &Built<'a>, name: &str, line: usize) -> Rc<Vec<Occurrence>> {
        if let Some((_, found)) = self
            .lines
            .borrow()
            .get(&line)
            .and_then(|names| names.iter().find(|(n, _)| n == name))
        {
            return found.clone();
        }
        let facts = &built.facts;
        let list: Vec<Occurrence> = built
            .identifiers
            .get(&(name.to_owned(), line))
            .into_iter()
            .flatten()
            .filter(|node| !is_declaring_name(**node))
            .map(|node| Occurrence {
                binding: facts.binding_at(name, node.start_byte()),
                storage: is_storage_access(*node),
            })
            .collect();
        let list = Rc::new(list);
        self.lines
            .borrow_mut()
            .entry(line)
            .or_default()
            .push((name.to_owned(), list.clone()));
        list
    }

    fn name_is_clean(&self, facts: &BindingFacts, name: &str) -> bool {
        if let Some(known) = self.clean_names.borrow().get(name) {
            return *known;
        }
        let clean = facts.unclassified_binding_lines(name).is_empty();
        self.clean_names.borrow_mut().insert(name.to_owned(), clean);
        clean
    }

    /// The declaration a def binds and the def's index, only when the def writes the
    /// variable itself.
    fn plain_def_binding(
        &self,
        built: &Built<'a>,
        path: &AccessPath,
        start: usize,
        end: usize,
        line: usize,
    ) -> Option<(usize, usize)> {
        if !path.fields.is_empty() {
            return None;
        }
        let facts = &built.facts;
        let index = *built.def_index.get(&(start, path.clone()))?;
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
        let coalesced = built.defs_on_line.get(&(path.base.clone(), line))?;
        if coalesced.iter().any(|i| {
            self.defs[*i].path == *path && facts.binding_of_def(*i) != facts.binding_of_def(index)
        }) {
            return None;
        }
        // Every lvalue of the path attributed to this line is classified by its
        // own span, wherever its identifier sits: `*` and the name may be on
        // different lines (`x = new(int); *` / `x = 7`), and only the first
        // span is the coalesced endpoint.
        if coalesced
            .iter()
            .filter(|i| self.defs[**i].path == *path)
            .any(|i| !self.def_is_plain_variable_write(self.defs[*i].start_byte))
        {
            return None;
        }
        if self
            .occurrences(built, &path.base, line)
            .iter()
            .any(|o| o.storage)
            || built
                .identifiers
                .get(&(path.base.clone(), line))
                .into_iter()
                .flatten()
                .any(|n| is_storage_access(*n))
        {
            return None;
        }
        Some((*declaration, index))
    }

    /// The def starts at an identifier that is the written variable itself.
    fn def_is_plain_variable_write(&self, start: usize) -> bool {
        self.parsed
            .tree
            .root_node()
            .descendant_for_byte_range(start, start)
            .is_some_and(|node| {
                node.kind() == "identifier"
                    && node.start_byte() == start
                    && is_plain_write_site(node)
            })
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
        if !self.enabled
            || !self.multi.contains(path.base.as_str())
            || self.address_taken.contains(path.base.as_str())
        {
            return true;
        }
        let built = self.built.get_or_init(|| self.build());
        if !self.name_is_clean(&built.facts, &path.base) {
            return true;
        }
        if !path.fields.is_empty() {
            return true;
        }
        let key = (def_start, def_end, def_line);
        let cached = self
            .plain_defs
            .borrow()
            .get(&key)
            .and_then(|names| names.iter().find(|(n, _)| *n == path.base))
            .map(|(_, verdict)| *verdict);
        let verdict = cached.unwrap_or_else(|| {
            let verdict = self.plain_def_binding(built, path, def_start, def_end, def_line);
            self.plain_defs
                .borrow_mut()
                .entry(key)
                .or_default()
                .push((path.base.clone(), verdict));
            verdict
        });
        let Some((_, def_index)) = verdict else {
            return true;
        };
        let reads = self.occurrences(built, &path.base, use_line);
        let drop = !reads.is_empty()
            && reads.iter().all(|o| {
                !o.storage
                    && matches!(&o.binding, BindingId::Declaration(other) if !built.facts.def_reaches_declaration(def_index, *other))
            });
        if drop {
            stats.dfg_pruned_scope += 1;
        }
        !drop
    }
}

/// A name introduced by `:=`, `var`/`const` or a parameter list is a write of
/// that declaration, never a read.
pub(super) fn is_declaring_name(node: Node<'_>) -> bool {
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
        "expression_list" => parent.parent().is_some_and(|g| match g.kind() {
            "short_var_declaration" => in_field(g, "left"),
            "range_clause" | "receive_statement" => has_define_token(g) && in_field(g, "left"),
            "type_switch_statement" => in_field(g, "alias"),
            _ => false,
        }),
        _ => false,
    }
}

pub(super) fn is_storage_access(node: Node<'_>) -> bool {
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
pub(super) fn is_plain_write_site(node: Node<'_>) -> bool {
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
        "expression_list" => parent.parent().is_some_and(|g| {
            matches!(
                g.kind(),
                "assignment_statement"
                    | "short_var_declaration"
                    | "range_clause"
                    | "receive_statement"
            ) && within(g, "left")
                || g.kind() == "type_switch_statement" && within(g, "alias")
        }),
        _ => false,
    }
}

/// Names among `multi` that rule S must keep: those whose address is taken in
/// `owner` (the operand of a unary `&`, through parentheses, selectors, indexing
/// and slicing, is rooted at the identifier), and the names of declarations the
/// producer emits no def for (function-literal formals, named results).
/// By name, so it covers the def's and the use's binding.
pub(super) fn address_taken_names<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    multi: &BTreeSet<&'a str>,
) -> BTreeSet<&'a str> {
    fn root<'n>(mut node: Node<'n>) -> Node<'n> {
        loop {
            let next = match node.kind() {
                "parenthesized_expression" => node.named_child(0),
                "selector_expression" | "index_expression" | "slice_expression" => {
                    node.child_by_field_name("operand")
                }
                _ => None,
            };
            match next {
                Some(n) => node = n,
                None => return node,
            }
        }
    }
    fn walk<'a>(
        parsed: &'a ParsedFile,
        node: Node<'a>,
        multi: &BTreeSet<&'a str>,
        out: &mut BTreeSet<&'a str>,
    ) {
        // Slicing an array (`x[:]`) shares its storage as `&x` does.
        if node.kind() == "slice_expression" {
            if let Some(operand) = node.child_by_field_name("operand") {
                let base = root(operand);
                if base.kind() == "identifier" {
                    if let Some(found) = multi.get(parsed.node_text(&base)) {
                        out.insert(*found);
                    }
                }
            }
        }
        if node.kind() == "unary_expression"
            && node
                .child_by_field_name("operator")
                .is_some_and(|op| op.kind() == "&")
        {
            if let Some(operand) = node.child_by_field_name("operand") {
                let base = root(operand);
                if base.kind() == "identifier" {
                    let name = parsed.node_text(&base);
                    if let Some(found) = multi.get(name) {
                        out.insert(*found);
                    }
                }
            }
        }
        // Declarations the producer still emits no def for (#694 slice B): function-literal
        // formals and named results of any function (a `return` assigns them with
        // no def). Range/receive `:=` targets, type-switch aliases and every
        // `var`/`const` name now have producer defs, so S may drop the outer pair.
        let mut keep = |id: Node<'a>| {
            if id.kind() == "identifier" {
                if let Some(found) = multi.get(parsed.node_text(&id)) {
                    out.insert(*found);
                }
            }
        };
        match node.kind() {
            "func_literal" | "function_declaration" | "method_declaration" => {
                let mut lists = Vec::new();
                if node.kind() == "func_literal" {
                    lists.extend(node.child_by_field_name("parameters"));
                }
                lists.extend(
                    node.child_by_field_name("result")
                        .filter(|r| r.kind() == "parameter_list"),
                );
                for list in lists {
                    let mut cursor = list.walk();
                    for param in list.named_children(&mut cursor) {
                        if matches!(
                            param.kind(),
                            "parameter_declaration" | "variadic_parameter_declaration"
                        ) {
                            let mut names = param.walk();
                            for name in param.children_by_field_name("name", &mut names) {
                                keep(name);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(parsed, child, multi, out);
        }
    }
    let mut out = BTreeSet::new();
    walk(parsed, owner, multi, &mut out);
    out
}

fn has_define_token(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|c| c.kind() == ":=");
    found
}

/// Names with at least two declaration sites in `owner`. This is a syntactic
/// superset of the sites `go_bindings::Inventory` turns into declarations
/// (`name` fields, `:=`/range/receive left sides, one per type-switch clause,
/// receiver type arguments): a name outside it has at most one declaration,
/// so rule S can never see a different one.
pub(super) fn multiply_declared<'a>(parsed: &'a ParsedFile, owner: Node<'a>) -> BTreeSet<&'a str> {
    fn bump<'a>(
        parsed: &'a ParsedFile,
        node: Node<'a>,
        by: usize,
        seen: &mut BTreeMap<&'a str, usize>,
    ) {
        if matches!(node.kind(), "identifier" | "type_identifier") {
            *seen.entry(parsed.node_text(&node)).or_insert(0) += by;
        }
    }
    fn walk<'a>(
        parsed: &'a ParsedFile,
        node: Node<'a>,
        in_receiver: bool,
        seen: &mut BTreeMap<&'a str, usize>,
    ) {
        if in_receiver {
            bump(parsed, node, 1, seen);
        }
        let kind = node.kind();
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    let field = cursor.field_name();
                    match field {
                        Some("name") => bump(parsed, child, 1, seen),
                        Some("left")
                            if matches!(
                                kind,
                                "short_var_declaration" | "receive_statement" | "range_clause"
                            ) =>
                        {
                            bump(parsed, child, 1, seen);
                            if child.kind() == "expression_list" {
                                let mut inner = child.walk();
                                for name in child.named_children(&mut inner) {
                                    bump(parsed, name, 1, seen);
                                }
                            }
                        }
                        Some("alias") if kind == "type_switch_statement" => {
                            let mut inner = node.walk();
                            let clauses = node
                                .named_children(&mut inner)
                                .filter(|c| matches!(c.kind(), "type_case" | "default_case"))
                                .count();
                            let by = clauses.max(1);
                            bump(parsed, child, by, seen);
                            if child.kind() == "expression_list" {
                                let mut inner = child.walk();
                                for name in child.named_children(&mut inner) {
                                    bump(parsed, name, by, seen);
                                }
                            }
                        }
                        _ => {}
                    }
                    walk(
                        parsed,
                        child,
                        in_receiver || field == Some("receiver"),
                        seen,
                    );
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    let mut seen = BTreeMap::new();
    walk(parsed, owner, false, &mut seen);
    seen.into_iter()
        .filter_map(|(name, count)| (count >= 2).then_some(name))
        .collect()
}
