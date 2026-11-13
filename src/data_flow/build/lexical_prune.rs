//! Removal-only Go, Python, Rust, JS/TS and C proofs. No CFG or RD label is used to discard a pair.
use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::cpg::RdFileStats;
use crate::languages::Language;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

struct Write<'a> {
    span: (usize, usize),
    statement: Node<'a>,
    plain: bool,
    /// A split `var`/`const` spec (`scope_split`): never a dominating write (rule D).
    split_spec: bool,
}
impl Write<'_> {
    /// A range/receive `:=` target, a type-switch alias or a formal/named result: only ever
    /// declares its name, so the header line it sits on holds no read of it, wherever it is
    /// nested (#694 slices A and B).
    fn is_header_target(&self) -> bool {
        is_declaring_kind(self.statement.kind())
    }
}
#[derive(Clone)]
struct Window {
    write_start: usize,
    write_end: usize,
    child_end: usize,
    /// C: byte start of the declaration the occurrence and the write both resolve to.
    decl: usize,
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
    real_reads: BTreeMap<(AccessPath, usize), Vec<(usize, usize)>>,
    nested: Vec<(usize, usize)>,
    /// Python names declared `global`/`nonlocal` in the owner: D keeps them.
    scope_declared: BTreeSet<String>,
    /// Python `self`/`cls` parameters never rebound in the owner: their attribute paths
    /// are fixed storage for D (see `python_prune::stable_receivers`).
    stable_receivers: BTreeSet<String>,
    /// Rust names mutably borrowed anywhere in the owner: D keeps them
    /// (`rust_prune::mutably_borrowed`).
    rust_mut_borrowed: BTreeSet<String>,
    /// Rust names that appear as identifier tokens of a macro invocation the census refuses
    /// (it can emit a `macro_rules!`): neither U nor D touches them in this owner
    /// (`rust_prune::scan_macros`).
    rust_refused: BTreeSet<String>,
    /// C owner facts (`c_prune`), present when the owner passes G1-G5.
    c: Option<super::c_prune::COwner>,
    /// C++ `template_declaration` owner: its own inner `function_definition` (F6).
    template_inner: Option<usize>,
    cache: RefCell<BTreeMap<(AccessPath, usize), UseProof>>,
    /// C/C++ D: the variable-write certificate per raw def span (`certified_candidate`
    /// resolves the declaration, linear in the block, so it is computed once per span).
    c_cert: RefCell<BTreeMap<(usize, usize, AccessPath), Option<usize>>>,
}
impl<'a> LexicalFacts<'a> {
    pub(super) fn new(
        parsed: &'a ParsedFile,
        owner: Node<'a>,
        lvalues: &[PathSpan],
        rvalues: &[PathSpan],
        census: &RustMacroCensus,
        c_census: &CMacroCensus,
    ) -> Self {
        // No lvalue span leaves the write table empty: U cannot prove a write-only line and D has
        // no dominating write (formals still get defs), so the owner switch is not consulted.
        let js_enabled = parsed.language.is_js_family()
            && !lvalues.is_empty()
            && super::js_prune::owner_enabled(parsed, owner);
        // Rust: an owner that defines a `macro_rules!` is off (a local macro reads a local by
        // definition-site hygiene while the line-anchored use sits at the invocation); so is
        // one whose macro token trees hold `macro_rules` or build names (`paste!`).
        let rust_macros = (parsed.language == Language::Rust
            && !owner.has_error()
            && owner.kind() == "function_item"
            && !super::rust_prune::has_local_macro(owner))
        .then(|| super::rust_prune::scan_macros(parsed, owner, census))
        .filter(|scan| !scan.owner_off);
        let c_owner = (matches!(parsed.language, Language::C | Language::Cpp)
            && !(parsed.language == Language::Cpp && super::mute6d::mute("cpp_off")))
        .then(|| {
            let scanned = super::c_prune::scan(parsed, owner, c_census);
            super::c_prune::trace(parsed, owner, &scanned);
            scanned.ok()
        })
        .flatten();
        let mut facts = Self {
            parsed,
            enabled: match parsed.language {
                Language::Go => !owner.has_error(),
                Language::Python => {
                    !owner.has_error()
                        && owner.kind() == "function_definition"
                        && !super::python_prune::has_hidden_reader(parsed, owner)
                }
                Language::Rust => rust_macros.is_some(),
                Language::C | Language::Cpp => c_owner.is_some(),
                l if l.is_js_family() => js_enabled,
                _ => false,
            },
            writes: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            real_reads: BTreeMap::new(),
            nested: Vec::new(),
            scope_declared: BTreeSet::new(),
            stable_receivers: BTreeSet::new(),
            rust_mut_borrowed: BTreeSet::new(),
            rust_refused: BTreeSet::new(),
            c: c_owner,
            template_inner: None,
            cache: RefCell::new(BTreeMap::new()),
            c_cert: RefCell::new(BTreeMap::new()),
        };
        if !facts.enabled {
            return facts;
        }
        if parsed.language == Language::Python {
            facts.stable_receivers = super::python_prune::stable_receivers(parsed, owner);
        }
        if let Some(scan) = rust_macros {
            facts.rust_mut_borrowed = super::rust_prune::mutably_borrowed(parsed, owner);
            facts.rust_refused = scan.refused;
            // An implicit format capture is a read of the name on every line of its
            // statement: it blocks U and is an occurrence D must dominate. It adds no Use.
            for (name, literal) in scan.captures {
                for line in super::rust_prune::statement_lines(literal) {
                    facts
                        .occurrences
                        .entry((name.clone(), line))
                        .or_default()
                        .push(literal);
                }
            }
        }
        if parsed.language.is_js_family() {
            facts.stable_receivers = super::js_prune::stable_bases(parsed, owner);
        }
        let mut boundaries = parsed.language.callable_boundary_node_types();
        if parsed.language.is_js_family() {
            boundaries.extend(super::js_prune::EXTRA_BOUNDARIES);
        }
        if parsed.language == Language::Python {
            boundaries.extend(super::python_prune::NESTED_SCOPES);
        }
        if parsed.language == Language::Rust {
            boundaries.extend(super::rust_prune::NESTED_ITEMS);
        }
        // F6: a `template_declaration` owner's own inner function is not a boundary for it.
        if parsed.language == Language::Cpp && owner.kind() == "template_declaration" {
            let mut cursor = owner.walk();
            facts.template_inner = owner
                .named_children(&mut cursor)
                .find(|c| c.kind() == "function_definition")
                .map(|c| c.id());
        }
        facts.collect(owner, owner.id(), &boundaries);
        for span in rvalues {
            facts
                .real_reads
                .entry((span.path.clone(), span.line))
                .or_default()
                .push((span.start_byte, span.end_byte));
        }
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
        if node.id() != owner_id
            && Some(node.id()) != self.template_inner
            && boundaries.contains(&node.kind())
        {
            self.nested.push((node.start_byte(), node.end_byte()));
        }
        if self.parsed.language == Language::Python {
            super::python_prune::note_scope_declaration(
                self.parsed,
                node,
                &mut self.scope_declared,
            );
        }
        if self.parsed.language.is_identifier_node(node.kind())
            || (node.kind() == "this" && self.parsed.language.is_js_family())
        {
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

    fn pure_path(&self, node: Node<'_>) -> Option<AccessPath> {
        match self.parsed.language {
            Language::Rust => super::rust_prune::pure_path(self.parsed, node),
            Language::C | Language::Cpp => super::c_prune::pure_path(self.parsed, node),
            _ => pure_path(self.parsed, node),
        }
    }

    fn nested_at(&self, byte: usize) -> bool {
        self.nested
            .iter()
            .any(|(start, end)| *start <= byte && byte < *end)
    }

    fn classify_write(&self, span: &PathSpan) -> Option<Write<'a>> {
        let node = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(span.start_byte, span.end_byte)?;
        // Normalized index/dereference/truncated paths do not prove identical storage.
        if node.start_byte() != span.start_byte
            || node.end_byte() != span.end_byte
            || self.pure_path(node).as_ref() != Some(&span.path)
        {
            return None;
        }
        let write = self.classify_write_in(node, span)?;
        // A nested callable's write proves nothing, except a header target (see `Write`).
        (!self.nested_at(span.start_byte) || write.is_header_target()).then_some(write)
    }

    fn classify_write_in(&self, mut node: Node<'a>, span: &PathSpan) -> Option<Write<'a>> {
        if self.parsed.language == Language::Python {
            let (statement, plain) =
                super::python_prune::classify_write(node, (span.start_byte, span.end_byte))?;
            return Some(Write {
                span: (span.start_byte, span.end_byte),
                statement,
                plain,
                split_spec: false,
            });
        }
        if self.parsed.language == Language::Rust {
            let (statement, plain) =
                super::rust_prune::classify_write(node, (span.start_byte, span.end_byte))?;
            return Some(Write {
                span: (span.start_byte, span.end_byte),
                statement,
                plain,
                split_spec: false,
            });
        }
        if matches!(self.parsed.language, Language::C | Language::Cpp) {
            let c = self.c.as_ref()?;
            let (statement, plain) = super::c_prune::classify_write(self.parsed, node, c)?;
            return Some(Write {
                span: (span.start_byte, span.end_byte),
                statement,
                plain,
                split_spec: false,
            });
        }
        if self.parsed.language.is_js_family() {
            let (statement, plain) = super::js_prune::classify_write(self.parsed, node)?;
            return Some(Write {
                span: (span.start_byte, span.end_byte),
                statement,
                plain,
                split_spec: false,
            });
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
                        split_spec: false,
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
                    let split_spec = !super::scope_split::split_declared(node).is_empty();
                    let mut statement = node;
                    while statement.kind() != "var_declaration" {
                        statement = statement.parent()?;
                    }
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement,
                        plain: true,
                        split_spec,
                    });
                }
                // #694 slice A: range/receive `:=` targets and type-switch aliases are plain
                // writes of their own declaration, so a header line that only declares the
                // name is write-only (rule U). They are never dominating writes (rule D).
                "range_clause" | "receive_statement" | "type_switch_statement" => {
                    let (field, declares) = if node.kind() == "type_switch_statement" {
                        ("alias", true)
                    } else {
                        let mut cursor = node.walk();
                        let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
                        ("left", declares)
                    };
                    let target = node.child_by_field_name(field)?;
                    if !declares
                        || !(target.start_byte() <= span.start_byte
                            && span.end_byte <= target.end_byte())
                    {
                        return None;
                    }
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement: node,
                        plain: true,
                        split_spec: false,
                    });
                }
                // #694 slice B: a function-literal formal or a named result is a plain write of
                // its own declaration (its entry value); like a header target it only declares.
                "parameter_declaration" | "variadic_parameter_declaration" => {
                    let mut cursor = node.walk();
                    let named = node.children_by_field_name("name", &mut cursor).any(|n| {
                        n.start_byte() == span.start_byte && n.end_byte() == span.end_byte
                    });
                    if !named {
                        return None;
                    }
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement: node,
                        plain: true,
                        split_spec: false,
                    });
                }
                "inc_statement" | "dec_statement" => {
                    return Some(Write {
                        span: (span.start_byte, span.end_byte),
                        statement: node,
                        plain: false,
                        split_spec: false,
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
        if !self.enabled || self.rust_refused.contains(rust_ident(&path.base)) {
            return UseProof::default();
        }
        let key = (path.clone(), line);
        if let Some(proof) = self.cache.borrow().get(&key) {
            return proof.clone();
        }
        let occurrences = self.occurrences.get(&(path.base.clone(), line));
        let mut proof = UseProof::default();
        if let Some(occurrences) = occurrences.filter(|o| !o.is_empty()) {
            // A read the rvalue collector records at a header target's own identifier (it
            // does inside nested callables) is no read.
            let writes = self.writes.get(path);
            let own_target = |(start, end): &(usize, usize)| {
                writes.is_some_and(|ws| {
                    ws.iter()
                        .any(|w| w.is_header_target() && w.span == (*start, *end))
                })
            };
            proof.write_only = self
                .real_reads
                .get(&key)
                .is_none_or(|r| r.iter().all(own_target))
                && occurrences.iter().all(|node| {
                    self.writes.get(path).is_some_and(|writes| {
                        writes.iter().any(|w| {
                            w.plain
                                && w.span.0 <= node.start_byte()
                                && node.end_byte() <= w.span.1
                                && (!self.nested_at(node.start_byte()) || w.is_header_target())
                        })
                    })
                });
            proof.windows = occurrences
                .iter()
                .map(|node| self.window(path, *node))
                .collect();
            if let Some(c) = &self.c {
                // C: E1-E3 for every occurrence, no field path, and G6 for D.
                let bound = path.fields.is_empty()
                    && occurrences
                        .iter()
                        .all(|node| c.binding(self.parsed, *node).is_some());
                if !bound && super::c_prune::trace_enabled() {
                    let why = if path.fields.is_empty() {
                        occurrences
                            .iter()
                            .find_map(|n| c.refusal(self.parsed, *n))
                            .unwrap_or_else(|| "unknown".into())
                    } else {
                        "field_path".into()
                    };
                    eprintln!(
                        "c-prune-use\t{}\t{line}\t{}\t{why}",
                        self.parsed.path, path.base
                    );
                }
                if !bound {
                    proof = UseProof::default();
                } else if !c.d_ok {
                    proof.windows = None;
                }
            }
        }
        self.cache.borrow_mut().insert(key, proof.clone());
        proof
    }

    fn window(&self, path: &AccessPath, occurrence: Node<'a>) -> Option<Window> {
        if self.nested_at(occurrence.start_byte()) {
            return None;
        }
        if self.scope_declared.contains(&path.base) {
            return None;
        }
        let own = self.writes.get(path)?;
        // A stable receiver's attribute path is fixed storage: a write of the path or of
        // any prefix of it is a write of it. Every other path compares its own writes only.
        let mut prefixed: Vec<&Write<'a>> = own.iter().collect();
        // JS compares identical paths only (brief: no prefix kills).
        if self.stable_receivers.contains(&path.base)
            && !path.fields.is_empty()
            && !self.parsed.language.is_js_family()
        {
            for len in 1..path.fields.len() {
                let mut prefix = path.clone();
                prefix.fields.truncate(len);
                prefixed.extend(self.writes.get(&prefix).into_iter().flatten());
            }
        }
        let writes = &prefixed;
        let mut child = occurrence;
        while let Some(region) = crate::languages::go::statements::parent(child) {
            if self.is_region(region) {
                if let Some(write) = writes
                    .iter()
                    .filter(|write| {
                        !is_declaring_kind(write.statement.kind())
                            && !write.split_spec
                            // C: a compound assignment reads the old value and never dominates.
                            && (self.c.is_none() || write.plain)
                            && (!self.parsed.language.is_js_family()
                                || matches!(
                                    write.statement.kind(),
                                    "expression_statement"
                                        | "lexical_declaration"
                                        | "variable_declaration"
                                ))
                            && write.statement.end_byte() <= child.start_byte()
                            && crate::languages::go::statements::parent(write.statement)
                                .is_some_and(|p| p.id() == region.id())
                    })
                    .max_by_key(|w| w.statement.start_byte())
                {
                    let mut broken = self.parsed.language == Language::Go
                        && crate::languages::go::statements::children(region, Language::Go).any(
                            |n| {
                                n.kind() == "labeled_statement"
                                    && write.statement.end_byte() <= n.start_byte()
                                    && n.start_byte() < child.end_byte()
                            },
                        );
                    let mut decl = 0;
                    if let Some(c) = &self.c {
                        // C: a label or nested `case` in [W.start, child_end) can enter the
                        // window; W must write the occurrence's own declaration (E1); E4.
                        match self.c_window_decl(c, occurrence, write) {
                            Some(d)
                                if !super::c_prune::entry_in_range(
                                    region,
                                    write.statement.start_byte(),
                                    child.end_byte(),
                                ) =>
                            {
                                decl = d
                            }
                            _ => broken = true,
                        }
                    }
                    if !broken {
                        return Some(Window {
                            write_start: write.statement.start_byte(),
                            write_end: write.statement.end_byte(),
                            child_end: child.end_byte(),
                            decl,
                        });
                    }
                }
            }
            child = region;
        }
        None
    }

    /// C: the declaration the occurrence resolves to when the write targets the same one and
    /// E4 holds; `None` keeps the window closed.
    fn c_window_decl(
        &self,
        c: &super::c_prune::COwner,
        occurrence: Node<'a>,
        write: &Write<'a>,
    ) -> Option<usize> {
        let target = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(write.span.0, write.span.1)?;
        let target = super::c_prune::write_target_identifier(target)?;
        let want = c.binding(self.parsed, occurrence)?;
        let got = c.binding(self.parsed, target)?;
        (want == got && c.subobject_free(self.parsed, want)).then_some(want)
    }

    fn is_region(&self, node: Node<'_>) -> bool {
        match self.parsed.language {
            Language::Python => super::python_prune::is_region(node.kind()),
            Language::Rust => super::rust_prune::is_region(node.kind()),
            Language::C | Language::Cpp => super::c_prune::is_region(node),
            l if l.is_js_family() => super::js_prune::is_region(node),
            _ => is_region(node.kind()),
        }
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
            if self.c.is_some() && super::c_prune::trace_enabled() {
                eprintln!(
                    "c-prune-drop\tU\t{}\t{line}\t{}",
                    self.parsed.path, path.base
                );
            }
            stats.dfg_pruned_write_only += 1;
        }
        drop
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
        // Python: D proves plain names and the attribute paths of a stable `self`/`cls`
        // parameter; any other base (a rebound name, `o.x`) may alias elsewhere.
        if (self.parsed.language == Language::Python || self.parsed.language.is_js_family())
            && !path.fields.is_empty()
            && !self.stable_receivers.contains(&path.base)
        {
            return true;
        }
        // Rust: D proves identifier paths only (a field path's storage can be re-pointed
        // behind its spelling), never a mutably borrowed name, and never a name passed to a
        // macro the census refuses.
        if self.parsed.language == Language::Rust
            && (!path.fields.is_empty()
                || self.rust_mut_borrowed.contains(rust_ident(&path.base))
                || self.rust_refused.contains(rust_ident(&path.base)))
        {
            return true;
        }
        // JS: an intermediate object is a reference the owner can swap and restore, so only a
        // single-field path `base.f` is fixed storage.
        if self.parsed.language.is_js_family() && path.fields.len() > 1 {
            return true;
        }
        // C: D proves identifier paths only; the candidate must be a variable write of the
        // very declaration the windows resolved to (F1: a dereference store is no def of
        // the pointer variable).
        let mut c_decl = None;
        if self.c.is_some() {
            if !path.fields.is_empty() {
                return true;
            }
            // Only a use with dominating windows can drop; ask that cached question before
            // the certificate (an escaped name has none, whatever the pair).
            if self.proof(path, line).windows.is_none() {
                return true;
            }
            let cert = *self
                .c_cert
                .borrow_mut()
                .entry((start, end, path.clone()))
                .or_insert_with(|| {
                    super::c_prune::certified_candidate(self.parsed, start, end, path)
                });
            match cert {
                Some(d) => c_decl = Some(d),
                None => return true,
            }
        }
        let drop = self.proof(path, line).windows.is_some_and(|windows| {
            windows.iter().all(|window| {
                !window.contains(start, end) && c_decl.is_none_or(|d| d == window.decl)
            })
        });
        if drop {
            if self.c.is_some() && super::c_prune::trace_enabled() {
                eprintln!(
                    "c-prune-drop\tD\t{}\t{line}\t{}",
                    self.parsed.path, path.base
                );
            }
            stats.dfg_pruned_dominated += 1;
        }
        !drop
    }
}

/// Statement kinds whose write only declares a name and is never a dominating write (rule D).
fn is_declaring_kind(kind: &str) -> bool {
    matches!(
        kind,
        "range_clause"
            | "receive_statement"
            | "type_switch_statement"
            | "parameter_declaration"
            | "variadic_parameter_declaration"
    )
}

fn is_region(kind: &str) -> bool {
    matches!(
        kind,
        "block" | "expression_case" | "type_case" | "communication_case" | "default_case"
    )
}
fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    if parsed.language.is_js_family() {
        return super::js_prune::pure_path(parsed, node);
    }
    match node.kind() {
        "identifier" => Some(AccessPath::simple(parsed.node_text(&node))),
        "attribute" => {
            let mut path = pure_path(parsed, node.child_by_field_name("object")?)?;
            let field = node.child_by_field_name("attribute")?;
            if field.kind() != "identifier" {
                return None;
            }
            path.fields.push(parsed.node_text(&field).to_owned());
            Some(path)
        }
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

/// A Rust identifier without the `r#` raw prefix (`r#type` and `type` name one variable; the
/// macro scan stores the stripped spelling).
fn rust_ident(base: &str) -> &str {
    base.strip_prefix("r#").unwrap_or(base)
}
