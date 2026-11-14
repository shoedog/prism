//! The value-flow graph of the owner (`js_hold.rs`, alias-twin slice 4c fold 3): nodes are binding identities
//! (`js_hold_resolve.rs`), one `this` per non-arrow callable (class bodies share their class's), `arguments` where
//! mentioned, and one callable node per function (`fn@`), class (`cls@`) and object literal holding a method
//! (`lit@`). Edges, all undirected (a union-find): E1 value flow (assignment, declarator, pattern leaf, default, loop
//! head join the roots of every target with the value names `V` of the value), E2 calls, E3 exceptions, E4
//! callables (captures, returns, `yield`, members), E5 `arguments` and formals. `V(e)` is the table in `val`;
//! an unlisted expression kind contributes every name in it (closed default).
//!
//! E2: a call joins its callee, receiver and arguments when the callee is not a plain identifier (member, `new`,
//! tagged, `super`), when a plain callee is declared in the owner and is not a formal, or when a callable can reach
//! it (a name of the call has an `fn@` in its component). A plain call of a free name or a formal that no callable
//! reaches joins nothing among its arguments (its result is still a value, E1): the one out-of-model call shape.
//! Every callable in the component of a joined call joins its formals, `this` and `arguments`. A catch parameter joins
//! the operands of every call, `new`, tagged template, `await` and `yield` in its `try` block (all operands of
//! callables in the owner when one of them can reach a callable) and every `throw` value of the owner.
use super::js_hold_resolve::{is_callable_kind, is_class_kind, Ident, Resolver};
use super::js_hold_val::{chain_root, leaf_kind};
use crate::ast::ParsedFile;
use crate::js_write_census::walk::{Form, Site};
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) struct Callable {
    pub(super) start: usize,
    pub(super) fn_id: usize,
}

pub(super) struct Op {
    pub(super) start: usize,
    pub(super) names: Vec<usize>,
    pub(super) in_callable: bool,
}

pub(super) struct CatchRec {
    pub(super) range: (usize, usize),
    pub(super) params: Vec<usize>,
    pub(super) generator: Option<usize>,
}

pub(super) struct Flow<'a, 't> {
    pub(super) parsed: &'a ParsedFile,
    pub(super) owner: Node<'t>,
    pub(super) res: Resolver,
    pub(super) ids: BTreeMap<String, usize>,
    pub(super) parent: Vec<usize>,
    pub(super) by_spelling: BTreeMap<String, Vec<usize>>,
    pub(super) formals: BTreeMap<usize, Vec<usize>>,
    pub(super) args_of: BTreeMap<usize, usize>,
    pub(super) callables: Vec<Callable>,
    pub(super) conditional: Vec<Vec<usize>>,
    pub(super) invoked: Vec<usize>,
    pub(super) ops: Vec<Op>,
    pub(super) catches: Vec<CatchRec>,
    pub(super) throws: Vec<Vec<usize>>,
    pub(super) nonname: Vec<Vec<usize>>,
    pub(super) namespaces: Vec<(Vec<usize>, usize, usize)>,
    pub(super) rns_ok: BTreeMap<String, bool>,
}

impl<'a, 't> Flow<'a, 't> {
    pub(super) fn new(parsed: &'a ParsedFile, owner: Node<'t>, sites: &[Site]) -> Self {
        let res = Resolver::new(parsed, owner, sites);
        let mut flow = Self {
            parsed,
            owner,
            res,
            ids: BTreeMap::new(),
            parent: Vec::new(),
            by_spelling: BTreeMap::new(),
            formals: BTreeMap::new(),
            args_of: BTreeMap::new(),
            callables: Vec::new(),
            conditional: Vec::new(),
            invoked: Vec::new(),
            ops: Vec::new(),
            catches: Vec::new(),
            throws: Vec::new(),
            nonname: Vec::new(),
            namespaces: Vec::new(),
            rns_ok: BTreeMap::new(),
        };
        let root = parsed.tree.root_node();
        for s in sites
            .iter()
            .filter(|s| s.form == Form::Declare && s.decl == "param")
        {
            let Some(n) = root.descendant_for_byte_range(s.target_start, s.target_end) else {
                continue;
            };
            let mut cur = n.parent();
            while let Some(p) = cur {
                if is_callable_kind(p.kind()) {
                    let id = flow.name_id(&s.path.base, s.target_start);
                    flow.formals.entry(p.start_byte()).or_default().push(id);
                    break;
                }
                cur = p.parent();
            }
        }
        let mut stack = vec![owner];
        while let Some(n) = stack.pop() {
            flow.visit(n);
            let mut c = n.walk();
            stack.extend(n.children(&mut c));
        }
        for (names, start, end) in std::mem::take(&mut flow.namespaces) {
            for s in sites.iter().filter(|s| {
                s.form == Form::Declare && s.target_start >= start && s.target_start < end
            }) {
                let id = flow.name_id(&s.path.base, s.target_start);
                flow.union_all(&names);
                flow.union_all(&[names.first().copied().unwrap_or(id), id]);
            }
        }
        flow.fixpoint();
        flow
    }

    // ---- union-find ----

    pub(super) fn intern(&mut self, key: &str, spelling: &str) -> usize {
        if let Some(&i) = self.ids.get(key) {
            return i;
        }
        let i = self.parent.len();
        self.parent.push(i);
        self.ids.insert(key.to_string(), i);
        if !spelling.is_empty() {
            self.by_spelling
                .entry(spelling.to_string())
                .or_default()
                .push(i);
        }
        i
    }

    pub(super) fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    pub(super) fn union(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        self.parent[b] = a;
        true
    }

    pub(super) fn union_all(&mut self, ids: &[usize]) -> bool {
        let mut changed = false;
        for w in ids.windows(2) {
            changed |= self.union(w[0], w[1]);
        }
        changed
    }

    pub(super) fn name_id(&mut self, base: &str, at: usize) -> usize {
        let key = self.res.key(base, at);
        self.intern(&key.key, base)
    }

    pub(super) fn ident_of(&mut self, node: Node<'_>) -> (usize, Ident) {
        let ident = self.res.ident(self.parsed, node, self.owner);
        let spelling = match node.kind() {
            "this" | "super" => "this".to_string(),
            _ => self.parsed.node_text(&node).trim().to_string(),
        };
        let spelling = if ident.key.starts_with("args@") {
            "arguments".to_string()
        } else {
            spelling
        };
        (self.intern(&ident.key, &spelling), ident)
    }

    pub(super) fn len(&self) -> usize {
        self.parent.len()
    }

    /// The identities of each spelling (the alias target is a spelling here: every identity of it counts).
    pub(super) fn spellings(&self) -> BTreeMap<String, Vec<usize>> {
        self.by_spelling.clone()
    }

    // ---- edges ----

    pub(super) fn visit(&mut self, node: Node<'t>) {
        if !node.is_named() {
            return;
        }
        let kind = node.kind();
        if leaf_kind(kind) {
            self.reference(node);
            return;
        }
        match kind {
            "assignment_expression" | "augmented_assignment_expression" => {
                let mut all = Vec::new();
                if let Some(l) = node.child_by_field_name("left") {
                    all.extend(self.roots(l));
                }
                if let Some(r) = node.child_by_field_name("right") {
                    all.extend(self.val(r));
                }
                self.union_all(&all);
            }
            "variable_declarator" => {
                let mut all = Vec::new();
                if let Some(n) = node.child_by_field_name("name") {
                    all.extend(self.roots(n));
                }
                if let Some(v) = node.child_by_field_name("value") {
                    all.extend(self.val(v));
                }
                self.union_all(&all);
            }
            "for_in_statement" => {
                let mut all = Vec::new();
                if let Some(l) = node.child_by_field_name("left") {
                    all.extend(self.roots(l));
                }
                if let Some(r) = node.child_by_field_name("right") {
                    all.extend(self.val(r));
                }
                self.union_all(&all);
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                let mut all = Vec::new();
                if let Some(l) = node.child_by_field_name("left") {
                    all.extend(self.roots(l));
                }
                if let Some(r) = node.child_by_field_name("right") {
                    all.extend(self.val(r));
                }
                self.union_all(&all);
            }
            "required_parameter" | "optional_parameter" => {
                let mut all = Vec::new();
                if let Some(p) = node.child_by_field_name("pattern") {
                    all.extend(self.roots(p));
                }
                if let Some(v) = node.child_by_field_name("value") {
                    all.extend(self.val(v));
                }
                let mut c = node.walk();
                let property = node
                    .named_children(&mut c)
                    .any(|n| matches!(n.kind(), "accessibility_modifier" | "override_modifier"));
                if property {
                    if let Some(class) = ancestor_class(node) {
                        all.push(self.intern(&format!("this@{}", class.start_byte()), "this"));
                    }
                }
                self.union_all(&all);
            }
            "call_expression" | "new_expression" => self.visit_call(node),
            "await_expression" => {
                let names = self.val_all(node);
                let in_callable = self.in_callable(node);
                self.ops.push(Op {
                    start: node.start_byte(),
                    names,
                    in_callable,
                });
            }
            "yield_expression" => {
                let names = self.val(node);
                let in_callable = self.in_callable(node);
                self.ops.push(Op {
                    start: node.start_byte(),
                    names,
                    in_callable,
                });
            }
            "throw_statement" => {
                let names = self.val_all(node);
                self.throws.push(names);
            }
            "return_statement" if self.enclosing_callable(node).id() != self.owner.id() => {
                let f = self.enclosing_callable(node);
                let mut all = vec![self.fn_node(f.start_byte())];
                all.extend(self.val_all(node));
                self.union_all(&all);
            }
            "catch_clause" => self.visit_catch(node),
            "object" => {
                let lit = self.intern(&format!("lit@{}", node.start_byte()), "");
                for m in self.object_methods(node) {
                    let this = self.intern(&format!("this@{}", m.start_byte()), "this");
                    self.union(lit, this);
                }
            }
            "internal_module" | "module" | "enum_declaration" => {
                if let Some(n) = node.child_by_field_name("name") {
                    let names = self.roots(n);
                    self.namespaces
                        .push((names, node.start_byte(), node.end_byte()));
                }
            }
            "member_expression" | "subscript_expression" => {
                if super::js_hold::is_write_target(node) {
                    let r = chain_root(node);
                    if r.kind() == "super" {
                        let id = self.ident_of(r).0;
                        self.nonname.push(vec![id]);
                    } else if !leaf_kind(r.kind()) {
                        let names = self.val(r);
                        self.nonname.push(names);
                    }
                }
            }
            k if is_callable_kind(k) => self.visit_callable(node),
            k if is_class_kind(k) => self.visit_class(node),
            _ => {}
        }
    }

    pub(super) fn visit_call(&mut self, node: Node<'t>) {
        let (names, opaque) = self.call_names(node);
        let in_callable = self.in_callable(node);
        self.ops.push(Op {
            start: node.start_byte(),
            names: names.clone(),
            in_callable,
        });
        if opaque {
            self.conditional.push(names);
        } else {
            self.union_all(&names);
            if let Some(&f) = names.first() {
                self.invoked.push(f);
            }
        }
    }

    pub(super) fn visit_catch(&mut self, node: Node<'t>) {
        let params = node
            .child_by_field_name("parameter")
            .map(|p| self.roots(p))
            .unwrap_or_default();
        let range = node
            .parent()
            .and_then(|t| t.child_by_field_name("body"))
            .map(|b| (b.start_byte(), b.end_byte()))
            .unwrap_or((0, 0));
        let f = self.enclosing_callable(node);
        let generator = (matches!(
            f.kind(),
            "generator_function" | "generator_function_declaration"
        ) || (f.kind() == "method_definition" && has_star(f)))
        .then(|| self.fn_node(f.start_byte()));
        self.catches.push(CatchRec {
            range,
            params,
            generator,
        });
    }

    /// A reference: the identity joins every callable or class that captures it.
    pub(super) fn reference(&mut self, node: Node<'t>) {
        let (id, ident) = self.ident_of(node);
        if ident.key.starts_with("args@") {
            let s = ident.home;
            self.args_of.insert(s, id);
            let f = self.fn_node(s);
            self.union(id, f);
            let formals = self.formals.get(&s).cloned().unwrap_or_default();
            for fid in formals {
                self.union(id, fid);
            }
        }
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.id() == self.owner.id() {
                break;
            }
            let k = p.kind();
            let captures = !(p.start_byte() <= ident.home && ident.home < p.end_byte());
            if captures && (is_callable_kind(k) || is_class_kind(k)) {
                let cap = if is_callable_kind(k) {
                    self.fn_node(p.start_byte())
                } else {
                    self.intern(&format!("cls@{}", p.start_byte()), "")
                };
                self.union(id, cap);
            }
            cur = p.parent();
        }
    }

    pub(super) fn visit_callable(&mut self, node: Node<'t>) {
        let start = node.start_byte();
        let f = self.fn_node(start);
        if node.id() != self.owner.id() {
            self.callables.push(Callable { start, fn_id: f });
        }
        if node.kind() != "arrow_function" {
            let this = self.intern(&format!("this@{start}"), "this");
            self.union(f, this);
        }
        if let Some(name) = node
            .child_by_field_name("name")
            .filter(|n| n.kind() == "identifier")
        {
            let text = self.parsed.node_text(&name).trim().to_string();
            let at = if node.kind() == "function_declaration"
                || node.kind() == "generator_function_declaration"
            {
                name.start_byte()
            } else if let Some(b) = node.child_by_field_name("body") {
                b.start_byte()
            } else {
                name.start_byte()
            };
            let id = self.name_id(&text, at);
            self.union(f, id);
        }
        if node.kind() == "arrow_function" && node.id() != self.owner.id() {
            if let Some(b) = node.child_by_field_name("body") {
                if b.kind() != "statement_block" {
                    let mut all = vec![f];
                    all.extend(self.val(b));
                    self.union_all(&all);
                }
            }
        }
        if node.kind() == "method_definition" {
            if let Some(class) = ancestor_class(node) {
                let cls = self.intern(&format!("cls@{}", class.start_byte()), "");
                let this = self.intern(&format!("this@{start}"), "this");
                self.union_all(&[cls, f, this]);
            }
        }
    }

    pub(super) fn visit_class(&mut self, node: Node<'t>) {
        let cls = self.intern(&format!("cls@{}", node.start_byte()), "");
        let this = self.intern(&format!("this@{}", node.start_byte()), "this");
        self.union(cls, this);
        if let Some(name) = node.child_by_field_name("name") {
            let text = self.parsed.node_text(&name).trim().to_string();
            for at in [name.start_byte(), node.start_byte().saturating_sub(1)] {
                let id = self.name_id(&text, at);
                self.union(cls, id);
            }
        }
    }

    /// The write-target roots with no plain name root, as the node sets their expressions can evaluate to.
    pub(super) fn nonname(&self) -> &[Vec<usize>] {
        &self.nonname
    }

    /// The identity of a census write root: `base` at the target's byte.
    pub(super) fn root_id(&mut self, base: &str, at: usize) -> usize {
        let root = self.parsed.tree.root_node();
        match root.descendant_for_byte_range(at, at + 1) {
            Some(n) if base == "this" || base == "arguments" => {
                let mut cur = Some(n);
                while let Some(c) = cur {
                    if leaf_kind(c.kind()) && self.parsed.node_text(&c).trim() == base {
                        return self.ident_of(c).0;
                    }
                    cur = c.parent();
                }
                self.name_id(base, at)
            }
            _ => self.name_id(base, at),
        }
    }
}

pub(super) fn has_star(method: Node<'_>) -> bool {
    let mut c = method.walk();
    let found = method.children(&mut c).any(|ch| ch.kind() == "*");
    found
}

pub(super) fn ancestor_class(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if is_class_kind(p.kind()) {
            return Some(p);
        }
        cur = p.parent();
    }
    None
}
