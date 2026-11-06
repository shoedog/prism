//! Removal-only Go proofs. No CFG or RD label is used to discard a pair.
use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::cpg::RdFileStats;
use crate::languages::Language;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

struct Write<'a> {
    span: (usize, usize),
    statement: Node<'a>,
    plain: bool,
}
#[derive(Clone)]
struct Window {
    write_start: usize,
    write_end: usize,
    child_end: usize,
}
impl Window {
    fn contains(&self, start: usize, end: usize) -> bool {
        // Keep every lvalue of the dominating statement, including parallel writes.
        (self.write_start <= start && end <= self.write_end)
            || (self.write_end < start && start < self.child_end)
    }
}
#[derive(Clone, Default)]
struct UseProof {
    write_only: bool,
    // One window per occurrence; a missing proof makes the whole line uncertain.
    windows: Option<Vec<Window>>,
}

pub(super) struct LexicalFacts<'a> {
    parsed: &'a ParsedFile,
    enabled: bool,
    writes: BTreeMap<AccessPath, Vec<Write<'a>>>,
    occurrences: BTreeMap<(String, usize), Vec<Node<'a>>>,
    real_reads: BTreeSet<(AccessPath, usize)>,
    nested: Vec<(usize, usize)>,
    cache: RefCell<BTreeMap<(AccessPath, usize), UseProof>>,
    /// #662: def-independent header spans per (base, line); `None` = not provable.
    header_cache: RefCell<BTreeMap<(String, usize), Option<Vec<(usize, usize)>>>>,
}
impl<'a> LexicalFacts<'a> {
    pub(super) fn new(
        parsed: &'a ParsedFile,
        owner: Node<'a>,
        lvalues: &[PathSpan],
        rvalues: &[PathSpan],
    ) -> Self {
        let mut facts = Self {
            parsed,
            enabled: parsed.language == Language::Go && !owner.has_error(),
            writes: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            real_reads: BTreeSet::new(),
            nested: Vec::new(),
            cache: RefCell::new(BTreeMap::new()),
            header_cache: RefCell::new(BTreeMap::new()),
        };
        if !facts.enabled {
            return facts;
        }
        facts.collect(
            owner,
            owner.id(),
            &parsed.language.callable_boundary_node_types(),
        );
        facts
            .real_reads
            .extend(rvalues.iter().map(|s| (s.path.clone(), s.line)));
        for span in lvalues {
            if let Some(write) = facts.classify_write(span) {
                facts
                    .writes
                    .entry(span.path.clone())
                    .or_default()
                    .push(write);
            }
        }
        for writes in facts.writes.values_mut() {
            writes.sort_by_key(|w| w.statement.start_byte());
        }
        facts
    }

    fn collect(&mut self, node: Node<'a>, owner_id: usize, boundaries: &[&str]) {
        if node.id() != owner_id && boundaries.contains(&node.kind()) {
            self.nested.push((node.start_byte(), node.end_byte()));
        }
        if self.parsed.language.is_identifier_node(node.kind()) {
            self.occurrences
                .entry((
                    self.parsed.node_text(&node).to_owned(),
                    node.start_position().row + 1,
                ))
                .or_default()
                .push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect(child, owner_id, boundaries);
        }
    }

    fn nested_at(&self, byte: usize) -> bool {
        self.nested
            .iter()
            .any(|(start, end)| *start <= byte && byte < *end)
    }

    fn classify_write(&self, span: &PathSpan) -> Option<Write<'a>> {
        let mut node = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(span.start_byte, span.end_byte)?;
        // Normalized index/dereference/truncated paths do not prove identical storage.
        if node.start_byte() != span.start_byte
            || node.end_byte() != span.end_byte
            || pure_path(self.parsed, node).as_ref() != Some(&span.path)
            || self.nested_at(span.start_byte)
        {
            return None;
        }
        node = node.parent()?;
        loop {
            match node.kind() {
                "assignment_statement" | "short_var_declaration" => {
                    let left = node.child_by_field_name("left")?;
                    if !(left.start_byte() <= span.start_byte && span.end_byte <= left.end_byte()) {
                        return None;
                    }
                    let plain = node.kind() == "short_var_declaration"
                        || node
                            .child_by_field_name("operator")
                            .is_some_and(|op| self.parsed.node_text(&op) == "=");
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement: node,
                        plain,
                    });
                }
                "var_spec" => {
                    // Only declaration names, never an occurrence in its initializer/type.
                    let mut cursor = node.walk();
                    if !node
                        .children_by_field_name("name", &mut cursor)
                        .any(|n| n.start_byte() == span.start_byte && n.end_byte() == span.end_byte)
                    {
                        return None;
                    }
                    let mut statement = node;
                    while statement.kind() != "var_declaration" {
                        statement = statement.parent()?;
                    }
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement,
                        plain: true,
                    });
                }
                "inc_statement" | "dec_statement" => {
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement: node,
                        plain: false,
                    })
                }
                // Do not cross a larger statement/expression to discover an unrelated write.
                "expression_list" => {}
                _ => return None,
            }
            node = node.parent()?;
        }
    }

    fn proof(&self, path: &AccessPath, line: usize) -> UseProof {
        if !self.enabled {
            return UseProof::default();
        }
        let key = (path.clone(), line);
        if let Some(proof) = self.cache.borrow().get(&key) {
            return proof.clone();
        }
        let occurrences = self.occurrences.get(&(path.base.clone(), line));
        let mut proof = UseProof::default();
        if let Some(occurrences) = occurrences.filter(|o| !o.is_empty()) {
            proof.write_only = !self.real_reads.contains(&key)
                && occurrences.iter().all(|node| {
                    !self.nested_at(node.start_byte())
                        && self.writes.get(path).is_some_and(|writes| {
                            writes.iter().any(|w| {
                                w.plain
                                    && w.span.0 <= node.start_byte()
                                    && node.end_byte() <= w.span.1
                            })
                        })
                });
            proof.windows = occurrences
                .iter()
                .map(|node| self.window(path, *node))
                .collect();
        }
        self.cache.borrow_mut().insert(key, proof.clone());
        proof
    }

    fn window(&self, path: &AccessPath, occurrence: Node<'a>) -> Option<Window> {
        if self.nested_at(occurrence.start_byte()) {
            return None;
        }
        let writes = self.writes.get(path)?;
        let mut child = occurrence;
        while let Some(region) = crate::go_statements::parent(child) {
            if is_region(region.kind()) {
                if let Some(write) = writes.iter().rev().find(|write| {
                    write.statement.end_byte() <= child.start_byte()
                        && crate::go_statements::parent(write.statement)
                            .is_some_and(|p| p.id() == region.id())
                }) {
                    let broken = crate::go_statements::children(region, Language::Go).any(|n| {
                        n.kind() == "labeled_statement"
                            && write.statement.end_byte() <= n.start_byte()
                            && n.start_byte() < child.end_byte()
                    });
                    if !broken {
                        return Some(Window {
                            write_start: write.statement.start_byte(),
                            write_end: write.statement.end_byte(),
                            child_end: child.end_byte(),
                        });
                    }
                }
            }
            child = region;
        }
        None
    }

    /// U runs before get_use, so a proven write-only anchor is never minted.
    pub(super) fn drop_write_only(
        &self,
        path: &AccessPath,
        line: usize,
        stats: &mut RdFileStats,
    ) -> bool {
        let drop = self.proof(path, line).write_only;
        if drop {
            stats.dfg_pruned_write_only += 1;
        }
        drop
    }

    /// #662: every occurrence of the name on the use line is bound to the declaration
    /// of an `if`/`switch`/`for` header, and the def lies outside all those headers.
    /// Plain variables only: a field path or dereference may alias through a copy.
    fn header_scoped(&self, path: &AccessPath, start: usize, line: usize) -> bool {
        if !path.fields.is_empty() {
            return false;
        }
        let key = (path.base.clone(), line);
        if !self.header_cache.borrow().contains_key(&key) {
            let spans = self.header_spans(&key);
            self.header_cache.borrow_mut().insert(key.clone(), spans);
        }
        let cache = self.header_cache.borrow();
        cache
            .get(&key)
            .and_then(|s| s.as_ref())
            .is_some_and(|spans| {
                spans
                    .iter()
                    .all(|(from, to)| !(*from <= start && start < *to))
            })
    }

    fn header_spans(&self, key: &(String, usize)) -> Option<Vec<(usize, usize)>> {
        let mut spans = Vec::new();
        for node in self.occurrences.get(key)? {
            match super::lexical_header::classify(self.parsed, *node) {
                super::lexical_header::Occurrence::Declaration => {}
                super::lexical_header::Occurrence::Bound(from, to) => spans.push((from, to)),
                super::lexical_header::Occurrence::Other => return None,
            }
        }
        (!spans.is_empty()).then_some(spans)
    }

    /// D runs after get_use; edge-less real use locations remain in the graph.
    pub(super) fn keep_def(
        &self,
        path: &AccessPath,
        start: usize,
        end: usize,
        line: usize,
        stats: &mut RdFileStats,
    ) -> bool {
        if !self.enabled || self.nested_at(start) {
            return true;
        }
        if self.header_scoped(path, start, line) {
            stats.dfg_pruned_scope += 1;
            return false;
        }
        let drop = self
            .proof(path, line)
            .windows
            .is_some_and(|windows| windows.iter().all(|window| !window.contains(start, end)));
        if drop {
            stats.dfg_pruned_dominated += 1;
        }
        !drop
    }
}

fn is_region(kind: &str) -> bool {
    matches!(
        kind,
        "block" | "expression_case" | "type_case" | "communication_case" | "default_case"
    )
}
fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    match node.kind() {
        "identifier" => Some(AccessPath::simple(parsed.node_text(&node))),
        "selector_expression" => {
            let mut path = pure_path(parsed, node.child_by_field_name("operand")?)?;
            let field = node.child_by_field_name("field")?;
            if field.kind() != "field_identifier" {
                return None;
            }
            path.fields.push(parsed.node_text(&field).to_owned());
            Some(path)
        }
        _ => None,
    }
}
