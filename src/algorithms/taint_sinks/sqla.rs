//! Closed allow-list grader for Python SQL weak proofs (#636 design A).
//!
//! A SQL sink is weak only when every node of its statement, clause chain or value arguments is
//! proven by the tables in `sqla_tables`; anything else stays strong. Library identity: a name is
//! SQLAlchemy/peewee/Django only when every binding of that spelling in the file is an import of
//! it. Local names resolve to exactly one binding, at most [`MAX_HOPS`] hops deep (D3). A value
//! position binds the Python object it receives (D1): a construct arriving from outside the
//! function (a parameter, a cross-module return, an attribute set elsewhere) is out of model;
//! everything the function or file builds itself is checked.

use super::callee::{args, callee_of, literal_string, Src};
use super::py_scope::{local_binding, Bound, Local};
use super::python::Facts;
use super::sqla_tables::{self as t, lib_path, Entry, Kw, Lib};
use std::cell::Cell;
use tree_sitter::Node;

const MAX_HOPS: u8 = 8;
const FUEL: u32 = 4000;

/// An `execute`-style argument is a weak statement.
pub(super) fn stmt_weak(p: Src<'_>, f: &Facts, q: Node<'_>) -> bool {
    Proof::new(p, f).stmt(q)
}

/// The argument of `query(..)`: a statement, or an entity/table proven by a never-rebound
/// annotated parameter or a single proven binding (`session.query(model_cls)`).
pub(super) fn query_weak(p: Src<'_>, f: &Facts, q: Node<'_>) -> bool {
    let pr = Proof::new(p, f);
    pr.stmt(q)
        || (q.kind() == "identifier"
            && matches!(
                pr.local(q, pr.text(q)),
                Local::One(Bound::Assign(_) | Bound::Param(Some(_)))
            )
            && pr.root_name(q))
}

/// A `where`/`filter`/`filter_by` call is weak: its own arguments and the whole method chain
/// around it (maximal chain) prove.
pub(super) fn clause_weak(p: Src<'_>, f: &Facts, call: Node<'_>) -> bool {
    let pr = Proof::new(p, f);
    let Some(func) = call.child_by_field_name("function") else {
        return false;
    };
    let (Some(obj), Some(m)) = (
        func.child_by_field_name("object"),
        func.child_by_field_name("attribute"),
    ) else {
        return false;
    };
    let Some(en) = t::find(t::CHAIN, pr.text(m)) else {
        return false;
    };
    pr.args_ok(call, en) && pr.outer_chain(call) && pr.inner_chain(obj)
}

/// The line belt: a callee on the line is a raw-SQL constructor (resolved through imports) or a
/// belt-named method, called with a non-literal argument. Strength only, never a finding.
pub(super) fn line_has_raw(p: Src<'_>, f: &Facts, id: Node<'_>) -> bool {
    let Some(c) = callee_of(id) else {
        return false;
    };
    let name = p.node_text(&id);
    let bare_import = if c.receiver.is_none() {
        f.names.get(name)
    } else {
        None
    };
    // A bare callee imported from another library is that library's function (D1).
    if bare_import.is_some_and(|(m, x)| lib_path(m, x).is_none()) {
        return false;
    }
    let orig = bare_import.map_or(name, |(_, o)| o.as_str());
    orig != "raw"
        && t::BELT.contains(&orig)
        && args(c.call).iter().any(|a| {
            let v = if a.kind() == "keyword_argument" {
                a.child_by_field_name("value").unwrap_or(*a)
            } else {
                *a
            };
            !plain_literal(p, v)
        })
}

pub(super) fn plain_literal(p: Src<'_>, n: Node<'_>) -> bool {
    let n = peel(n);
    match n.kind() {
        "string" => literal_string(p, n).is_some(),
        "concatenated_string" => {
            let mut c = n.walk();
            let all = n.named_children(&mut c).all(|k| plain_literal(p, k));
            all
        }
        "integer" | "float" | "true" | "false" | "none" => true,
        "unary_operator" => n
            .child_by_field_name("argument")
            .is_some_and(|a| plain_literal(p, a)),
        _ => false,
    }
}

pub(super) fn peel(mut n: Node<'_>) -> Node<'_> {
    while n.kind() == "parenthesized_expression" {
        match n.named_child(0) {
            Some(c) => n = c,
            None => break,
        }
    }
    n
}

pub(super) struct Proof<'a> {
    pub(super) p: Src<'a>,
    pub(super) f: &'a Facts,
    hops: Cell<u8>,
    pub(super) defs: Cell<u8>,
    fuel: Cell<u32>,
}

pub(super) type LibRef = (Lib, Vec<String>);

impl<'a> Proof<'a> {
    pub(super) fn new(p: Src<'a>, f: &'a Facts) -> Self {
        Self {
            p,
            f,
            hops: Cell::new(0),
            defs: Cell::new(0),
            fuel: Cell::new(FUEL),
        }
    }

    pub(super) fn text(&self, n: Node<'_>) -> &'a str {
        self.p.node_text(&n)
    }

    pub(super) fn tick(&self) -> bool {
        let left = self.fuel.get();
        self.fuel.set(left.saturating_sub(1));
        left > 0
    }

    /// Resolve one name to its initializer: at most [`MAX_HOPS`] deep, so a cycle refuses.
    pub(super) fn hop(&self, run: impl FnOnce() -> bool) -> bool {
        let depth = self.hops.get();
        if depth >= MAX_HOPS {
            return false;
        }
        self.hops.set(depth + 1);
        let ok = run();
        self.hops.set(depth);
        ok
    }

    pub(super) fn local<'t>(&self, at: Node<'t>, name: &str) -> Local<'t> {
        local_binding(self.p, &self.f.sites, at, name)
    }

    // ---- identity --------------------------------------------------------------------------

    pub(super) fn lib_of(&self, f: Node<'_>) -> Option<LibRef> {
        let mut segs: Vec<&str> = Vec::new();
        let mut cur = f;
        while cur.kind() == "attribute" {
            segs.push(self.text(cur.child_by_field_name("attribute")?));
            cur = cur.child_by_field_name("object")?;
        }
        if cur.kind() != "identifier" {
            return None;
        }
        let (m, x) = self.f.sites.trusted_import(self.text(cur))?;
        let (lib, mut path) = lib_path(m, x)?;
        path.extend(segs.into_iter().rev().map(str::to_string));
        Some((lib, path))
    }

    pub(super) fn is_func(path: &[String]) -> bool {
        path.len() >= 2 && matches!(path[path.len() - 2].as_str(), "func" | "fn")
    }

    /// A name no enclosing scope could rebind: another module's object or a model class.
    pub(super) fn schema_name(&self, name: &str) -> bool {
        self.f
            .sites
            .is_schema_name(name, &|(m, x)| lib_path(m, x).is_some())
    }

    // ---- argument classes ------------------------------------------------------------------

    pub(super) fn by_class(&self, c: char, n: Node<'_>) -> bool {
        match c {
            'C' => self.clause(n),
            'V' | 'M' => self.value(n),
            'F' => self.from_(n),
            'S' => self.stmt(n),
            'L' => plain_literal(self.p, n),
            'T' => self.type_(n),
            'O' => self.loader(n),
            'X' => plain_literal(self.p, n) || self.type_(n),
            _ => false,
        }
    }

    pub(super) fn kw_class(kw: Kw, name: &str) -> Option<char> {
        let hit = kw.iter().find(|(k, _)| *k == name);
        hit.or_else(|| kw.iter().find(|(k, _)| *k == "*"))
            .map(|(_, c)| *c)
    }

    pub(super) fn args_ok(&self, call: Node<'_>, en: &Entry) -> bool {
        self.args_by(call, en.pos, en.kw)
    }

    pub(super) fn args_by(&self, call: Node<'_>, pos: &str, kw: Kw) -> bool {
        if call
            .child_by_field_name("arguments")
            .is_some_and(|a| a.kind() != "argument_list")
        {
            return false;
        }
        let classes: Vec<char> = pos.split(' ').filter_map(|s| s.chars().next()).collect();
        let mut i = 0;
        for a in args(call) {
            let ok = match a.kind() {
                "keyword_argument" => {
                    let name = a.child_by_field_name("name").map_or("", |n| self.text(n));
                    match (Self::kw_class(kw, name), a.child_by_field_name("value")) {
                        (Some(c), Some(v)) => self.by_class(c, v),
                        _ => false,
                    }
                }
                "dictionary_splat" => {
                    Self::kw_class(kw, "*") == Some('V')
                        && a.named_child(0).is_some_and(|v| self.value(v))
                }
                _ => {
                    let c = classes.get(i).or(classes.last()).copied().unwrap_or('-');
                    i += 1;
                    if a.kind() == "list_splat" {
                        c == 'V' && a.named_child(0).is_some_and(|v| self.value(v))
                    } else {
                        self.by_class(c, a)
                    }
                }
            };
            if !ok {
                return false;
            }
        }
        true
    }

    // ---- statements and from-objects -------------------------------------------------------

    pub(super) fn annotation_in(&self, ann: Node<'_>, allowed: &[&str]) -> bool {
        let text = self.text(ann).trim_matches(|c| c == '"' || c == '\'');
        let base = text.split('[').next().unwrap_or(text);
        let last = base.rsplit('.').next().unwrap_or(base).trim();
        allowed.contains(&last)
            || (allowed == t::ROOT_ANNOTATIONS && last.eq_ignore_ascii_case("type"))
    }

    pub(super) fn stmt(&self, n: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let n = peel(n);
        match n.kind() {
            "string" | "concatenated_string" => plain_literal(self.p, n),
            "identifier" => match self.local(n, self.text(n)) {
                Local::One(Bound::Assign(v)) => self.hop(|| self.stmt(v)),
                Local::One(Bound::Param(Some(a))) => self.annotation_in(a, t::STMT_ANNOTATIONS),
                _ => false,
            },
            "attribute" => {
                // A Django manager: `Model.objects`.
                n.child_by_field_name("attribute")
                    .is_some_and(|a| self.text(a) == "objects")
                    && n.child_by_field_name("object")
                        .is_some_and(|o| self.from_(o))
            }
            "call" => self.stmt_call(n),
            _ => false,
        }
    }

    pub(super) fn text_literal(&self, call: Node<'_>) -> bool {
        let a = args(call);
        a.len() == 1 && a[0].kind() != "keyword_argument" && plain_literal(self.p, a[0])
    }

    pub(super) fn stmt_call(&self, call: Node<'_>) -> bool {
        let Some(f) = call.child_by_field_name("function") else {
            return false;
        };
        if let Some((lib, path)) = self.lib_of(f) {
            let Some(last) = path.last().filter(|_| lib == Lib::Sqla) else {
                return false;
            };
            if last == "text" {
                return self.text_literal(call);
            }
            return t::find(t::ROOTS, last).is_some_and(|en| self.args_ok(call, en));
        }
        let (Some(obj), Some(m)) = (
            f.child_by_field_name("object"),
            f.child_by_field_name("attribute"),
        ) else {
            return false;
        };
        let m = self.text(m);
        if m == "query" {
            return self.args_by(call, "C", &[]);
        }
        if let Some(en) = t::find(t::CHAIN, m) {
            if self.stmt(obj) {
                return self.args_ok(call, en);
            }
        }
        // `<model or table>.select(..)`; a bare module name (`import x as sa`) is no schema root.
        let module = obj.kind() == "identifier" && self.f.sites.is_module(self.text(obj));
        t::find(t::ROOT_METHODS, m)
            .is_some_and(|en| !module && self.from_(obj) && self.args_ok(call, en))
    }

    /// A schema root, a FROM object: a model or table, a join target, a proven subquery.
    pub(super) fn from_(&self, n: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let n = peel(n);
        match n.kind() {
            "identifier" => self.root_name(n),
            "attribute" | "subscript" => self.colref(n),
            "call" => self.expr_call(n),
            _ => false,
        }
    }

    pub(super) fn root_name(&self, n: Node<'_>) -> bool {
        let name = self.text(n);
        match self.local(n, name) {
            Local::One(Bound::Assign(v)) => self.hop(|| self.from_(v)),
            // D2: a never-rebound parameter annotated as a table or statement.
            Local::One(Bound::Param(Some(a))) => self.annotation_in(a, t::ROOT_ANNOTATIONS),
            Local::Unbound | Local::One(Bound::Import(..) | Bound::Def(_)) => {
                self.schema_name(name)
            }
            _ => false,
        }
    }

    /// `R.a.b`, `R.c["lit"]`, or a column chain whose base is an allowed construct.
    pub(super) fn colref(&self, n: Node<'_>) -> bool {
        let mut cur = n;
        loop {
            match cur.kind() {
                "attribute" => cur = cur.child_by_field_name("object").unwrap_or(cur),
                "subscript" => {
                    let mut c = cur.walk();
                    let lits = cur
                        .children_by_field_name("subscript", &mut c)
                        .all(|s| plain_literal(self.p, s));
                    match (lits, cur.child_by_field_name("value")) {
                        (true, Some(v)) => cur = v,
                        _ => return false,
                    }
                }
                "parenthesized_expression" => cur = peel(cur),
                _ => break,
            }
            if matches!(cur.kind(), "attribute" | "subscript") {
                continue;
            }
            break;
        }
        match cur.kind() {
            "identifier" => !matches!(self.text(cur), "self" | "cls") && self.root_name(cur),
            "call" => self.expr_call(cur),
            _ => false,
        }
    }

    // ---- clauses ---------------------------------------------------------------------------

    pub(super) fn clause(&self, n: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let n = peel(n);
        let child = |f: &str| n.child_by_field_name(f);
        match n.kind() {
            "string" | "concatenated_string" | "integer" | "float" | "true" | "false" | "none" => {
                plain_literal(self.p, n)
            }
            "attribute" | "subscript" => self.colref(n),
            "identifier" => {
                let name = self.text(n);
                match self.local(n, name) {
                    Local::One(Bound::Assign(v)) => self.hop(|| self.clause(v)),
                    Local::One(Bound::Param(Some(a))) => self.annotation_in(a, t::ROOT_ANNOTATIONS),
                    Local::Unbound | Local::One(Bound::Import(..) | Bound::Def(_)) => {
                        self.schema_name(name)
                    }
                    _ => false,
                }
            }
            "comparison_operator" => {
                let mut c = n.walk();
                let ops: Vec<Node<'_>> = n.named_children(&mut c).collect();
                // At least one side is a proven non-literal clause; the others are C or V.
                let anchor = ops
                    .iter()
                    .position(|o| !plain_literal(self.p, *o) && self.clause(*o));
                anchor.is_some_and(|a| {
                    ops.iter()
                        .enumerate()
                        .all(|(i, o)| i == a || self.value(*o))
                })
            }
            "boolean_operator" => {
                child("left").is_some_and(|l| self.clause(l))
                    && child("right").is_some_and(|r| self.clause(r))
            }
            "not_operator" => child("argument").is_some_and(|a| self.clause(a)),
            "unary_operator" => child("argument").is_some_and(|a| self.clause(a)),
            "binary_operator" => {
                let (Some(l), Some(r)) = (child("left"), child("right")) else {
                    return false;
                };
                if matches!(child("operator").map(|o| self.text(o)), Some("&" | "|")) {
                    return self.clause(l) && self.clause(r);
                }
                // Arithmetic: one side is a proven non-literal clause, the other C or V.
                let side = |a: Node<'_>, b: Node<'_>| {
                    !plain_literal(self.p, a) && self.clause(a) && self.value(b)
                };
                side(l, r) || side(r, l)
            }
            "conditional_expression" => {
                n.named_child_count() == 3
                    && n.named_child(0).is_some_and(|b| self.clause(b))
                    && n.named_child(2).is_some_and(|e| self.clause(e))
            }
            "call" => self.expr_call(n),
            _ => false,
        }
    }

    pub(super) fn expr_call(&self, call: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let Some(f) = call.child_by_field_name("function") else {
            return false;
        };
        if let Some((lib, path)) = self.lib_of(f) {
            return self.lib_call(call, lib, &path);
        }
        if f.kind() != "attribute" {
            return false;
        }
        let (Some(obj), Some(m)) = (
            f.child_by_field_name("object"),
            f.child_by_field_name("attribute"),
        ) else {
            return false;
        };
        let m = self.text(m);
        if t::find(t::CHAIN, m).is_some() && self.stmt_call(call) {
            return true;
        }
        t::find(t::COLUMN_METHODS, m).is_some_and(|en| self.clause(obj) && self.args_ok(call, en))
    }

    pub(super) fn lib_call(&self, call: Node<'_>, lib: Lib, path: &[String]) -> bool {
        let Some(last) = path.last() else {
            return false;
        };
        if Self::is_func(path) {
            return self.args_by(call, "M", &[]);
        }
        if lib == Lib::Peewee {
            return false;
        }
        match last.as_str() {
            "text" => self.text_literal(call),
            "case" => self.case_args(call),
            "cast" | "type_coerce" => self.cast_args(call),
            n => {
                if let Some(en) = t::find(t::ROOTS, n) {
                    return self.args_ok(call, en);
                }
                t::find(t::CONSTRUCTS, n).is_some_and(|en| self.args_ok(call, en))
            }
        }
    }

    pub(super) fn case_args(&self, call: Node<'_>) -> bool {
        let whens = |w: Node<'_>| {
            w.kind() == "tuple"
                && w.named_child_count() == 2
                && w.named_child(0).is_some_and(|c| self.clause(c))
                && w.named_child(1).is_some_and(|v| self.value(v))
        };
        args(call).into_iter().all(|a| match a.kind() {
            "keyword_argument" => {
                let v = a.child_by_field_name("value");
                match a.child_by_field_name("name").map(|n| self.text(n)) {
                    Some("else_") => v.is_some_and(|v| self.value(v)),
                    Some("value") => v.is_some_and(|v| self.clause(v)),
                    _ => false,
                }
            }
            "dictionary" => self.value(a),
            "list" => {
                let mut c = a.walk();
                let all = a.named_children(&mut c).all(whens);
                all
            }
            _ => whens(a),
        })
    }

    pub(super) fn cast_args(&self, call: Node<'_>) -> bool {
        let list = args(call);
        let mut pos = 0;
        list.iter().all(|a| {
            if a.kind() == "keyword_argument" {
                return a
                    .child_by_field_name("name")
                    .is_some_and(|n| self.text(n) == "type_")
                    && a.child_by_field_name("value")
                        .is_some_and(|v| self.type_(v));
            }
            pos += 1;
            match pos {
                1 => self.value(*a),
                2 => self.type_(*a),
                _ => false,
            }
        })
    }

    /// A type: a SQLAlchemy or model name, or a call of one with literal or type arguments.
    pub(super) fn type_(&self, n: Node<'_>) -> bool {
        let n = peel(n);
        match n.kind() {
            "identifier" | "attribute" => self.lib_of(n).is_some() || self.colref(n),
            "call" => {
                n.child_by_field_name("function")
                    .is_some_and(|f| self.type_(f))
                    && args(n).iter().all(|a| {
                        a.kind() != "keyword_argument"
                            && (plain_literal(self.p, *a) || self.type_(*a))
                    })
            }
            _ => false,
        }
    }

    /// `options(..)`: loader calls over column references or literals.
    pub(super) fn loader(&self, n: Node<'_>) -> bool {
        let n = peel(n);
        let Some(f) = n
            .child_by_field_name("function")
            .filter(|_| n.kind() == "call")
        else {
            return false;
        };
        let (name, inner) = match self.lib_of(f) {
            Some((_, path)) => (path.last().cloned(), None),
            None if f.kind() == "attribute" => (
                f.child_by_field_name("attribute")
                    .map(|a| self.text(a).to_string()),
                f.child_by_field_name("object"),
            ),
            None => (None, None),
        };
        name.is_some_and(|nm| t::LOADERS.contains(&nm.as_str()))
            && inner.is_none_or(|i| self.loader(i))
            && args(n).iter().all(|a| {
                let v = if a.kind() == "keyword_argument" {
                    a.child_by_field_name("value").unwrap_or(*a)
                } else {
                    *a
                };
                plain_literal(self.p, v) || self.colref(v)
            })
    }
}
