//! Rule O (#630 slice 4), Go only: drop a (def, use) pair when the def starts on a
//! later line than the use, both are direct code of the owner, no loop of the
//! owner contains both, and the owner has no `goto` or label. Backward flow needs
//! a cycle, and the only cycles are loops and backward gotos.
//!
//! Soundness also needs the storage not to outlive the activation: the name must
//! resolve, at the def and at every occurrence on the use line, to the same
//! explicit declaration of the owner (never a package-level or captured name, a
//! flat fallback or a name with an unclassified binding construct), the def must
//! write the variable itself, and the address of the name must never be taken.
//! Anything unproven keeps the pair.
use super::scope_prune::{
    address_taken_names, is_declaring_name, is_plain_write_site, is_storage_access,
};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::cpg::{DefSite, RdFileStats};
use crate::languages::Language;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tree_sitter::Node;

type Span = (usize, usize);

struct Occurrence {
    binding: BindingId,
    start: usize,
    storage: bool,
    nested: bool,
}

struct Built<'a> {
    facts: BindingFacts,
    def_index: BTreeMap<(usize, AccessPath), usize>,
    defs_on_line: BTreeMap<(String, usize), Vec<usize>>,
    loops: Vec<Span>,
    nested: Vec<Span>,
    address_taken: BTreeSet<&'a str>,
}

pub(super) struct OrderFacts<'a> {
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    enabled: bool,
    defs: &'a [DefSite],
    /// `None` when the owner holds a `goto` or a label: O is off for it.
    built: OnceCell<Option<Built<'a>>>,
    clean: RefCell<HashMap<String, bool>>,
    def_verdicts: RefCell<HashMap<(usize, usize, usize, String), Option<usize>>>,
    def_loops: RefCell<HashMap<usize, Option<Span>>>,
    def_nested: RefCell<HashMap<usize, bool>>,
    lines: RefCell<HashMap<(String, usize), std::rc::Rc<Vec<Occurrence>>>>,
}

impl<'a> OrderFacts<'a> {
    pub(super) fn new(parsed: &'a ParsedFile, owner: Node<'a>, defs: &'a [DefSite]) -> Self {
        Self {
            parsed,
            owner,
            enabled: parsed.language == Language::Go && !owner.has_error() && !defs.is_empty(),
            defs,
            built: OnceCell::new(),
            clean: RefCell::new(HashMap::new()),
            def_verdicts: RefCell::new(HashMap::new()),
            def_loops: RefCell::new(HashMap::new()),
            def_nested: RefCell::new(HashMap::new()),
            lines: RefCell::new(HashMap::new()),
        }
    }

    fn build(&self) -> Option<Built<'a>> {
        let boundary = self.parsed.language.callable_boundary_node_types();
        let mut loops = Vec::new();
        let mut nested = Vec::new();
        let mut names = BTreeSet::new();
        let mut jumps = false;
        let mut stack = vec![self.owner];
        while let Some(node) = stack.pop() {
            match node.kind() {
                "labeled_statement" | "goto_statement" => jumps = true,
                "for_statement" => loops.push((node.start_byte(), node.end_byte())),
                "identifier" => {
                    names.insert(self.parsed.node_text(&node));
                }
                kind if node.id() != self.owner.id() && boundary.contains(&kind) => {
                    nested.push((node.start_byte(), node.end_byte()));
                }
                _ => {}
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        if jumps {
            return None;
        }
        let facts = BindingFacts::new(self.parsed, &self.owner, self.defs, std::iter::empty());
        let mut built = Built {
            facts,
            def_index: BTreeMap::new(),
            defs_on_line: BTreeMap::new(),
            loops,
            nested,
            address_taken: address_taken_names(self.parsed, self.owner, &names),
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
        Some(built)
    }

    fn name_is_clean(&self, facts: &BindingFacts, name: &str) -> bool {
        if let Some(known) = self.clean.borrow().get(name) {
            return *known;
        }
        let clean = facts.unclassified_binding_lines(name).is_empty();
        self.clean.borrow_mut().insert(name.to_owned(), clean);
        clean
    }

    fn identifiers_on_line(&self, name: &str, line: usize, out: &mut Vec<Node<'a>>) {
        let lo = self.parsed.line_start_byte(line);
        let hi = self.parsed.line_start_byte(line + 1);
        let mut stack = vec![self.owner];
        while let Some(node) = stack.pop() {
            if node.end_byte() <= lo || node.start_byte() >= hi {
                continue;
            }
            if node.kind() == "identifier" {
                if node.start_byte() >= lo && self.parsed.node_text(&node) == name {
                    out.push(node);
                }
                continue;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
    }

    /// Reads of `name` on `line` (declaring names are writes and are omitted).
    fn occurrences(
        &self,
        built: &Built<'a>,
        name: &str,
        line: usize,
    ) -> std::rc::Rc<Vec<Occurrence>> {
        if let Some(found) = self.lines.borrow().get(&(name.to_owned(), line)) {
            return found.clone();
        }
        let mut nodes = Vec::new();
        self.identifiers_on_line(name, line, &mut nodes);
        let list: Vec<Occurrence> = nodes
            .into_iter()
            .filter(|n| !is_declaring_name(*n))
            .map(|n| Occurrence {
                binding: built.facts.binding_at(name, n.start_byte()),
                start: n.start_byte(),
                storage: is_storage_access(n),
                nested: built
                    .nested
                    .iter()
                    .any(|(s, e)| *s <= n.start_byte() && n.end_byte() <= *e),
            })
            .collect();
        let list = std::rc::Rc::new(list);
        self.lines
            .borrow_mut()
            .insert((name.to_owned(), line), list.clone());
        list
    }

    /// The declaration the def binds, only when it writes the variable itself.
    fn plain_def_binding(
        &self,
        built: &Built<'a>,
        path: &AccessPath,
        start: usize,
        end: usize,
        line: usize,
    ) -> Option<usize> {
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
        let coalesced = built.defs_on_line.get(&(path.base.clone(), line))?;
        for i in coalesced.iter().filter(|i| self.defs[**i].path == *path) {
            let site = self.defs[*i].start_byte;
            let plain = self
                .parsed
                .tree
                .root_node()
                .descendant_for_byte_range(site, site)
                .is_some_and(|n| {
                    n.kind() == "identifier" && n.start_byte() == site && is_plain_write_site(n)
                });
            if !plain || facts.binding_of_def(*i) != facts.binding_of_def(index) {
                return None;
            }
        }
        let mut on_def_line = Vec::new();
        self.identifiers_on_line(&path.base, line, &mut on_def_line);
        if on_def_line.into_iter().any(is_storage_access) {
            return None;
        }
        Some(*declaration)
    }

    /// False (drop the pair) only when the def is a direct plain write after the
    /// use line and every read on the use line is a direct read of the same
    /// local declaration that no loop of the owner shares with the def.
    pub(super) fn keep_pair(
        &self,
        path: &AccessPath,
        def_start: usize,
        def_end: usize,
        def_line: usize,
        use_line: usize,
        stats: &mut RdFileStats,
    ) -> bool {
        if !self.enabled || !path.fields.is_empty() || def_line <= use_line {
            return true;
        }
        let Some(built) = self.built.get_or_init(|| self.build()).as_ref() else {
            return true;
        };
        if built.address_taken.contains(path.base.as_str())
            || !self.name_is_clean(&built.facts, &path.base)
        {
            return true;
        }
        let key = (def_start, def_end, def_line, path.base.clone());
        let cached = self.def_verdicts.borrow().get(&key).copied();
        let verdict = cached.unwrap_or_else(|| {
            let v = self.plain_def_binding(built, path, def_start, def_end, def_line);
            self.def_verdicts.borrow_mut().insert(key, v);
            v
        });
        let Some(declaration) = verdict else {
            return true;
        };
        let def_nested = *self
            .def_nested
            .borrow_mut()
            .entry(def_start)
            .or_insert_with(|| {
                built
                    .nested
                    .iter()
                    .any(|(s, e)| *s <= def_start && def_end <= *e)
            });
        if def_nested {
            return true;
        }
        let outer_loop = *self
            .def_loops
            .borrow_mut()
            .entry(def_start)
            .or_insert_with(|| {
                // Loops containing the def form a chain; the outermost holds any
                // occurrence that some loop of the chain holds.
                built
                    .loops
                    .iter()
                    .filter(|(s, e)| *s <= def_start && def_end <= *e)
                    .min_by_key(|(s, _)| *s)
                    .copied()
            });
        let reads = self.occurrences(built, &path.base, use_line);
        let drop = !reads.is_empty()
            && reads.iter().all(|o| {
                !o.storage
                    && !o.nested
                    && o.binding == BindingId::Declaration(declaration)
                    && outer_loop.is_none_or(|(s, e)| !(s <= o.start && o.start < e))
            });
        if drop {
            stats.dfg_pruned_order += 1;
        }
        !drop
    }
}
