//! C rules U and D: lexical name resolution (E1/E2): declarator unwrapping, the declaration an
//! identifier belongs to, and the innermost-scope-first resolver. Moved verbatim out of
//! `c_prune.rs` (#820).
use super::c_prune::{tokens, COwner, STORAGE_OR_QUALIFIER};
use super::cpp_prune;
use super::prune_roots::{function_params, operand};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use tree_sitter::Node;

// Test-only work counter: declarator and node visits of lexical resolution and of the entry
// walk. A guard test (`c_prune_perf_tests`) bounds its growth with the owner's size, so a
// resolver that rescans blocks per occurrence fails without a wall-clock assertion.
#[cfg(test)]
thread_local! {
    pub(super) static WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn count_work(n: usize) {
    WORK.with(|w| w.set(w.get() + n));
}

/// One declared name of a scope: `(gate, start, end)`. The name is in scope for an occurrence
/// at or after `gate`; `start..end` is the identifier node.
type ScopeEntry = (usize, usize, usize);
type ScopeIndex = std::collections::HashMap<String, Vec<ScopeEntry>>;

/// Per-owner memo of the declarations of each block scope (#820): a resolution used to rescan
/// every declaration of every enclosing block per occurrence (quadratic in the statements of
/// an owner); now each block is read once and each name is looked up by spelling. The result is
/// the one the scan gave: same entries, same order, same position gate.
#[derive(Default)]
pub(super) struct ResolveCache {
    scopes: std::cell::RefCell<std::collections::HashMap<usize, std::rc::Rc<ScopeIndex>>>,
    entries: std::cell::RefCell<std::collections::HashMap<usize, std::rc::Rc<Vec<usize>>>>,
}

impl ResolveCache {
    fn scope(&self, parsed: &ParsedFile, scope: Node<'_>) -> std::rc::Rc<ScopeIndex> {
        if let Some(hit) = self.scopes.borrow().get(&scope.id()) {
            return hit.clone();
        }
        let mut index = ScopeIndex::new();
        let mut cursor = scope.walk();
        for child in scope.named_children(&mut cursor) {
            if child.kind() == "declaration" {
                index_declaration(parsed, child, &mut index);
            }
        }
        let index = std::rc::Rc::new(index);
        self.scopes.borrow_mut().insert(scope.id(), index.clone());
        index
    }

    /// Starts of the labels and non-direct `case`s under `region`, sorted: control can enter a
    /// window there (what `entry_in_range` finds by walking).
    pub(super) fn entry_in_window(&self, region: Node<'_>, from: usize, to: usize) -> bool {
        let starts = {
            let hit = self.entries.borrow().get(&region.id()).cloned();
            match hit {
                Some(h) => h,
                None => {
                    let mut v = Vec::new();
                    entry_starts(region, &mut v);
                    v.sort_unstable();
                    let h = std::rc::Rc::new(v);
                    self.entries.borrow_mut().insert(region.id(), h.clone());
                    h
                }
            }
        };
        let i = starts.partition_point(|s| *s < from);
        starts.get(i).is_some_and(|s| *s < to)
    }
}

fn entry_starts(node: Node<'_>, out: &mut Vec<usize>) {
    #[cfg(test)]
    count_work(1);
    match node.kind() {
        "labeled_statement" => out.push(node.start_byte()),
        "case_statement" => {
            let direct = node.parent().is_some_and(|b| {
                b.kind() == "compound_statement" && super::c_prune_write::is_switch_body(b)
            });
            if !direct {
                out.push(node.start_byte());
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        entry_starts(child, out);
    }
}

/// `decl_idents` for every name at once: the entries each name would get, gated by position.
fn index_declaration(parsed: &ParsedFile, decl: Node<'_>, index: &mut ScopeIndex) {
    let mut cursor = decl.walk();
    for d in decl.children_by_field_name("declarator", &mut cursor) {
        #[cfg(test)]
        count_work(1);
        let top = if d.kind() == "init_declarator" {
            match d.child_by_field_name("declarator") {
                Some(t) => t,
                None => continue,
            }
        } else {
            d
        };
        let mut add = |gate: usize, id: Node<'_>| {
            index
                .entry(parsed.node_text(&id).to_owned())
                .or_default()
                .push((gate, id.start_byte(), id.end_byte()));
        };
        if structured_binding_in(top).is_some() || declared(top).is_none() {
            let mut found = Vec::new();
            decl_names(top, &mut found);
            for id in found {
                add(top.start_byte(), id);
            }
            continue;
        }
        if let Some(dd) = declared(top) {
            add(dd.ident.start_byte(), dd.ident);
        }
    }
}

/// Every identifier `decl_name` could return for some name under a declarator `top`.
fn decl_names<'a>(top: Node<'a>, out: &mut Vec<Node<'a>>) {
    if let Some(sb) = structured_binding_in(top) {
        let mut cursor = sb.walk();
        for c in sb.named_children(&mut cursor) {
            if c.kind() == "identifier" {
                out.push(c);
            }
        }
    } else if let Some(dd) = declared(top) {
        out.push(dd.ident);
    } else {
        collect_identifiers(top, out);
    }
}

/// A declarator chain's identifier and shape.
pub(super) struct Declared<'a> {
    pub(super) ident: Node<'a>,
    pub(super) pointer: bool,
    /// An array or function declarator, or an unknown wrapper: never eligible.
    pub(super) exotic: bool,
}

/// Unwrap a declarator through pointer and parenthesized declarators to its identifier.
pub(super) fn declared<'a>(mut node: Node<'a>) -> Option<Declared<'a>> {
    let (mut pointer, mut exotic) = (false, false);
    loop {
        match node.kind() {
            "identifier" => {
                return Some(Declared {
                    ident: node,
                    pointer,
                    exotic,
                })
            }
            "pointer_declarator" => {
                pointer = true;
                node = node.child_by_field_name("declarator")?;
            }
            "parenthesized_declarator" => {
                node = operand(node)?;
            }
            "array_declarator" | "function_declarator" => {
                exotic = true;
                node = node.child_by_field_name("declarator")?;
            }
            "attributed_declarator" => {
                exotic = true;
                node = operand(node)?;
            }
            // C++: a reference never names its own storage.
            "reference_declarator" => {
                exotic = true;
                node = operand(node)?;
            }
            _ => return None,
        }
    }
}

/// `pure_path` for C: an identifier, or a pointer/parenthesized declarator around one.
pub(super) fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    let d = declared(node)?;
    if d.exotic || (d.ident.id() != node.id() && !is_plain_declarator_chain(node)) {
        return None;
    }
    Some(AccessPath::simple(parsed.node_text(&d.ident)))
}

pub(super) fn is_plain_declarator_chain(mut node: Node<'_>) -> bool {
    loop {
        match node.kind() {
            "identifier" => return true,
            "pointer_declarator" => match node.child_by_field_name("declarator") {
                Some(n) => node = n,
                None => return false,
            },
            "parenthesized_declarator" => match operand(node) {
                Some(n) => node = n,
                None => return false,
            },
            _ => return false,
        }
    }
}

/// The declaration (or parameter) a declarator identifier sits in, and the declarator
/// subtree directly under the declaration or `init_declarator`.
pub(super) fn declaration_of<'a>(ident: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    let mut top = ident;
    loop {
        let parent = top.parent()?;
        match parent.kind() {
            "pointer_declarator" | "parenthesized_declarator" => top = parent,
            "array_declarator" | "function_declarator" | "attributed_declarator" => top = parent,
            "init_declarator" => {
                if parent.child_by_field_name("declarator")?.id() != top.id() {
                    return None;
                }
                let decl = parent.parent()?;
                return (decl.kind() == "declaration").then_some((decl, top));
            }
            "declaration" | "parameter_declaration" => {
                return Some((parent, top));
            }
            _ => return None,
        }
    }
}

/// E2 on one declarator identifier: automatic, unqualified, no macro type, not an array or
/// function. `Some(is_pointer)` when eligible.
pub(super) fn eligible_declared(
    parsed: &ParsedFile,
    owner: &COwner,
    d: &Declared<'_>,
) -> Option<bool> {
    if d.exotic || owner.macro_names.contains(parsed.node_text(&d.ident)) {
        return None;
    }
    let (decl, top) = declaration_of(d.ident)?;
    // Specifier tokens (everything before the first declarator) must hold no storage class,
    // qualifier or in-repo macro name.
    let first_decl_start = {
        let mut cursor = decl.walk();
        let found = decl
            .children_by_field_name("declarator", &mut cursor)
            .map(|c| c.start_byte())
            .min();
        found.unwrap_or_else(|| top.start_byte())
    };
    let spec = &parsed.source[decl.start_byte()..first_decl_start.max(decl.start_byte())];
    if tokens(spec).any(|t| STORAGE_OR_QUALIFIER.contains(&t) || owner.macro_names.contains(t)) {
        return None;
    }
    // Qualifiers inside the declarator (`int *volatile p`).
    if tokens(parsed.node_text(&top)).any(|t| STORAGE_OR_QUALIFIER.contains(&t)) {
        return None;
    }
    // C++ owners: the scalar gate (a class, typedef, template, `decltype` or
    // non-literal `auto` type, or a reference, may hide operator and constructor effects).
    if owner.cpp && !cpp_prune::scalar_gate(parsed, decl, top, d.pointer) {
        return None;
    }
    Some(d.pointer)
}

/// The binding an occurrence resolves to: the declarator identifier node, found lexically,
/// innermost scope first. `None`: unresolved, ambiguous or not an owner-local declaration.
pub(super) fn resolve<'a>(
    parsed: &ParsedFile,
    cache: &ResolveCache,
    occ: Node<'a>,
) -> Option<Node<'a>> {
    // C++: the name of a qualified identifier (`N::x`) is another scope's entity.
    if parsed.language == crate::languages::Language::Cpp && cpp_prune::is_qualified_name(occ) {
        return None;
    }
    let name = parsed.node_text(&occ);
    let mut child = occ;
    loop {
        let parent = child.parent()?;
        let mut found: Vec<Node<'a>> = Vec::new();
        match parent.kind() {
            "compound_statement" => {
                cached_scope_decls(parsed, cache, parent, name, occ, &mut found);
                if parent
                    .parent()
                    .is_some_and(|p| p.kind() == "switch_statement")
                {
                    let mut cursor = parent.walk();
                    for case in parent.named_children(&mut cursor) {
                        if case.kind() == "case_statement" {
                            cached_scope_decls(parsed, cache, case, name, occ, &mut found);
                        }
                    }
                }
            }
            "for_statement" => {
                if let Some(init) = parent.child_by_field_name("initializer") {
                    if init.kind() == "declaration" {
                        decl_idents(parsed, init, name, occ, &mut found);
                    }
                }
            }
            // C++ headers: `if (int v = f(); v)`, `while (T* p = ..)`, `switch (int s = ..)`.
            "if_statement" | "while_statement" | "switch_statement" => {
                cpp_prune::condition_decls(parent, &mut found, &|d, f| {
                    decl_idents(parsed, d, name, occ, f)
                });
            }
            "for_range_loop" => {
                if let Some(init) = parent.child_by_field_name("initializer") {
                    let mut cursor = init.walk();
                    for d in init.named_children(&mut cursor) {
                        if d.kind() == "declaration" {
                            decl_idents(parsed, d, name, occ, &mut found);
                        }
                    }
                }
                // The loop variable is in scope in the body only.
                if let (Some(var), Some(body)) = (
                    parent.child_by_field_name("declarator"),
                    parent.child_by_field_name("body"),
                ) {
                    if occ.start_byte() >= body.start_byte() {
                        decl_name(parsed, var, name, &mut found);
                    }
                }
            }
            "catch_clause" => {
                if let Some(params) = parent.child_by_field_name("parameters") {
                    let mut cursor = params.walk();
                    for p in params.named_children(&mut cursor) {
                        if p.kind() == "parameter_declaration" {
                            decl_idents(parsed, p, name, occ, &mut found);
                        }
                    }
                }
            }
            "function_definition" => {
                let mut cursor = parent.walk();
                // K&R parameter declarations: not modelled.
                if parent
                    .named_children(&mut cursor)
                    .any(|c| c.kind() == "declaration")
                {
                    return None;
                }
                if let Some(params) = parent
                    .child_by_field_name("declarator")
                    .and_then(function_params)
                {
                    let mut cursor = params.walk();
                    for p in params.named_children(&mut cursor) {
                        if p.kind() == "parameter_declaration" {
                            decl_idents(parsed, p, name, occ, &mut found);
                        }
                    }
                }
                // The nearest function is the end of every scope walk.
                return (found.len() == 1).then(|| found[0]);
            }
            _ => {}
        }
        match found.len() {
            0 => {}
            1 => return Some(found[0]),
            _ => return None,
        }
        child = parent;
    }
}

fn cached_scope_decls<'a>(
    parsed: &ParsedFile,
    cache: &ResolveCache,
    scope: Node<'a>,
    name: &str,
    occ: Node<'a>,
    out: &mut Vec<Node<'a>>,
) {
    let index = cache.scope(parsed, scope);
    for (gate, start, end) in index.get(name).into_iter().flatten() {
        if *gate <= occ.start_byte() {
            if let Some(n) = scope.descendant_for_byte_range(*start, *end) {
                out.push(n);
            }
        }
    }
}

/// Declarator identifiers of `decl` spelled `name` whose scope has begun at `occ` (the
/// declarator itself ends before the occurrence, or is the occurrence).
fn decl_idents<'a>(
    parsed: &ParsedFile,
    decl: Node<'a>,
    name: &str,
    occ: Node<'a>,
    out: &mut Vec<Node<'a>>,
) {
    let mut cursor = decl.walk();
    for d in decl.children_by_field_name("declarator", &mut cursor) {
        #[cfg(test)]
        count_work(1);
        let top = if d.kind() == "init_declarator" {
            match d.child_by_field_name("declarator") {
                Some(t) => t,
                None => continue,
            }
        } else {
            d
        };
        // C++: a structured binding (also under a reference or other wrapper) binds every name
        // into the aggregate: ineligible (see `declaration_of`). A declarator shape this
        // resolver cannot unwrap fails closed the same way (see `decl_name`).
        if structured_binding_in(top).is_some() || declared(top).is_none() {
            if top.start_byte() <= occ.start_byte() {
                decl_name(parsed, top, name, out);
            }
            continue;
        }
        if let Some(dd) = declared(top) {
            if parsed.node_text(&dd.ident) == name && dd.ident.start_byte() <= occ.start_byte() {
                out.push(dd.ident);
            }
        }
    }
}

/// The `structured_binding_declarator` a declarator chain wraps, through reference, pointer,
/// parenthesized, array and attributed declarators.
fn structured_binding_in(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        match node.kind() {
            "structured_binding_declarator" => return Some(node),
            "reference_declarator" | "parenthesized_declarator" | "attributed_declarator" => {
                node = operand(node)?;
            }
            "pointer_declarator" | "array_declarator" => {
                node = node.child_by_field_name("declarator")?;
            }
            _ => return None,
        }
    }
}

/// Declarator identifiers spelled `name` under a declarator without a declaration around it
/// (a range-for variable, a structured binding): the binding exists and is never eligible.
/// A shape that is neither a structured binding nor a chain `declared` can unwrap binds every
/// identifier of that spelling in it (fail closed: resolution never continues outward).
fn decl_name<'a>(parsed: &ParsedFile, top: Node<'a>, name: &str, out: &mut Vec<Node<'a>>) {
    if let Some(sb) = structured_binding_in(top) {
        let mut cursor = sb.walk();
        for c in sb.named_children(&mut cursor) {
            if c.kind() == "identifier" && parsed.node_text(&c) == name {
                out.push(c);
            }
        }
    } else if let Some(dd) = declared(top) {
        if parsed.node_text(&dd.ident) == name {
            out.push(dd.ident);
        }
    } else {
        let mut ids = Vec::new();
        collect_identifiers(top, &mut ids);
        out.extend(ids.into_iter().filter(|i| parsed.node_text(i) == name));
    }
}

fn collect_identifiers<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        collect_identifiers(c, out);
    }
}

/// Re-derive the declarator chain around a declarator identifier.
pub(super) fn declared_of_ident(ident: Node<'_>) -> Option<Declared<'_>> {
    let (_, top) = declaration_of(ident)?;
    let d = declared(top)?;
    (d.ident.id() == ident.id()).then_some(d)
}
