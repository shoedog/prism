//! Lexical binding lookup for Python sink rules (#622 S1): what a receiver or a
//! bare callee is (a collection, a file object, a function defined in this file),
//! taken from the nearest enclosing function or module scope.

use super::callee::Src;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) enum PyBind<'t> {
    Unresolved,
    /// A parameter, with its annotation when it has one.
    Param {
        annotation: Option<Node<'t>>,
    },
    /// `name = value`.
    Assigned(Node<'t>),
    /// `with value as name`.
    WithAs(Node<'t>),
    /// `def name` / `class name`.
    Def,
    Other,
}

pub(super) fn resolve<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> PyBind<'t> {
    let mut cur = from.parent();
    while let Some(scope) = cur {
        match scope.kind() {
            "function_definition" => {
                if let Some(b) = in_params(p, scope, name) {
                    return b;
                }
                if let Some(body) = scope.child_by_field_name("body") {
                    if let Some(b) = scan(p, body, name) {
                        return b;
                    }
                }
            }
            "module" => {
                if let Some(b) = scan(p, scope, name) {
                    return b;
                }
            }
            _ => {}
        }
        cur = scope.parent();
    }
    PyBind::Unresolved
}

/// True when `name` is bound more than `allowed` times in the file. Every binding form counts
/// toward one total: plain, annotated and tuple/list-unpacking assignment targets, `with .. as
/// name`, `for` targets, walrus and augmented assignment. A fact proved from the first binding
/// alone no longer holds once the total passes `allowed`, so the caller treats the name as
/// unknown. Conservative by design: the count is file-wide, by spelling.
pub(super) fn is_rebound(p: Src<'_>, from: Node<'_>, name: &str, allowed: usize) -> bool {
    let mut root = from;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut count = 0usize;
    super::callee::walk(root, &mut |n| {
        let target = match n.kind() {
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                n.child_by_field_name("left")
            }
            "named_expression" => n.child_by_field_name("name"),
            "as_pattern_target" => n.named_child(0),
            _ => None,
        };
        if target.is_some_and(|t| binds_name(p, t, name)) {
            count += 1;
        }
    });
    count > allowed
}

/// True when the assignment target `t` binds `name` (directly or inside an unpacking pattern;
/// attribute and subscript targets bind nothing).
fn binds_name(p: Src<'_>, t: Node<'_>, name: &str) -> bool {
    match t.kind() {
        "identifier" => p.node_text(&t) == name,
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "parenthesized_expression"
        | "tuple"
        | "list" => {
            let mut cursor = t.walk();
            let mut kids = t.named_children(&mut cursor);
            kids.any(|c| binds_name(p, c, name))
        }
        _ => false,
    }
}

fn in_params<'t>(p: Src<'_>, func: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let params = func.child_by_field_name("parameters")?;
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        match param.kind() {
            "identifier" if p.node_text(&param) == name => {
                return Some(PyBind::Param { annotation: None })
            }
            "typed_parameter" => {
                let ident = param.named_child(0).filter(|n| n.kind() == "identifier");
                if ident.is_some_and(|n| p.node_text(&n) == name) {
                    return Some(PyBind::Param {
                        annotation: param.child_by_field_name("type"),
                    });
                }
            }
            "default_parameter" | "typed_default_parameter" => {
                let n = param.child_by_field_name("name");
                if n.is_some_and(|n| p.node_text(&n) == name) {
                    return Some(PyBind::Param {
                        annotation: param.child_by_field_name("type"),
                    });
                }
            }
            _ => {}
        }
    }
    None
}

/// First binding of `name` in source order inside `node`, not entering nested scopes.
fn scan<'t>(p: Src<'_>, node: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_definition" | "class_definition" => {
                if child
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    return Some(PyBind::Def);
                }
                continue;
            }
            "lambda" => continue,
            "assignment" => {
                let left = child.child_by_field_name("left");
                if left.is_some_and(|l| l.kind() == "identifier" && p.node_text(&l) == name) {
                    return Some(match child.child_by_field_name("right") {
                        Some(v) => PyBind::Assigned(v),
                        None => PyBind::Other,
                    });
                }
            }
            "with_item" => {
                if let Some(b) = with_alias(p, child, name) {
                    return Some(b);
                }
            }
            "for_statement" => {
                let left = child.child_by_field_name("left");
                if left.is_some_and(|l| l.kind() == "identifier" && p.node_text(&l) == name) {
                    return Some(PyBind::Other);
                }
            }
            _ => {}
        }
        if let Some(b) = scan(p, child, name) {
            return Some(b);
        }
    }
    None
}

fn with_alias<'t>(p: Src<'_>, item: Node<'t>, name: &str) -> Option<PyBind<'t>> {
    let value = item.child_by_field_name("value")?;
    if value.kind() != "as_pattern" {
        return None;
    }
    let mut cursor = value.walk();
    let aliased = value
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "as_pattern_target")
        .any(|t| {
            t.named_child(0)
                .is_some_and(|i| i.kind() == "identifier" && p.node_text(&i) == name)
        });
    if aliased {
        Some(PyBind::WithAs(value.named_child(0)?))
    } else {
        None
    }
}

// ---- Binding-site census and single-binding resolution (#636 design A) ----------------------

/// One way a spelling is bound. Every Python binding form produces one.
pub(super) enum Bound<'t> {
    /// `import m [as a]` (member empty) or `from m import member [as a]`.
    Import(String, String),
    /// `name = value`: one plain identifier target of a non-chained assignment.
    Assign(Node<'t>),
    /// A parameter of any kind, with its annotation when it has one.
    Param(Option<Node<'t>>),
    /// A `for` / comprehension / `with .. as` / `except .. as` target, with the iterated or
    /// entered expression.
    Loop(Option<Node<'t>>),
    /// A `def` or `class` name (the defining node).
    Def(Node<'t>),
    /// Unpacking, augmented assignment, walrus, `del`, a match capture, a type alias.
    Opaque,
    /// Named by a `global` or `nonlocal` statement.
    Declared,
}

type Found<'t> = Vec<(String, Bound<'t>)>;

/// Identifiers bound by an assignment-like target (attribute and subscript targets bind
/// nothing); `plain` is true for a lone identifier.
fn target_names<'t>(p: Src<'_>, t: Node<'t>, plain: bool, out: &mut Vec<(String, bool)>) {
    match t.kind() {
        "identifier" => out.push((p.node_text(&t).to_string(), plain)),
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "parenthesized_expression"
        | "tuple"
        | "list"
        | "typed_parameter"
        | "expression_list" => {
            let mut c = t.walk();
            for k in t.named_children(&mut c) {
                target_names(p, k, false, out);
            }
        }
        _ => {}
    }
}

fn names_of<'t>(p: Src<'_>, t: Node<'t>, plain: bool) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    target_names(p, t, plain, &mut out);
    out
}

fn dotted<'a>(p: Src<'a>, n: Node<'_>) -> &'a str {
    p.node_text(&n)
}

/// The bindings the node `n` itself introduces (not those of its descendants).
fn bindings_of<'t>(p: Src<'_>, n: Node<'t>) -> Found<'t> {
    let mut out: Found<'t> = Vec::new();
    fn targets<'t>(
        p: Src<'_>,
        out: &mut Found<'t>,
        t: Option<Node<'t>>,
        mk: &dyn Fn() -> Bound<'t>,
    ) {
        if let Some(t) = t {
            for (name, _) in names_of(p, t, true) {
                out.push((name, mk()));
            }
        }
    }
    match n.kind() {
        "assignment" => {
            let left = n.child_by_field_name("left");
            let right = n.child_by_field_name("right");
            let is_plain = left.is_some_and(|l| l.kind() == "identifier");
            match (left, right) {
                (Some(l), Some(r)) if is_plain && r.kind() != "assignment" => {
                    out.push((p.node_text(&l).to_string(), Bound::Assign(r)));
                }
                (l, _) => targets(p, &mut out, l, &|| Bound::Opaque),
            }
        }
        "augmented_assignment" => targets(p, &mut out, n.child_by_field_name("left"), &|| {
            Bound::Opaque
        }),
        "for_statement" | "for_in_clause" => {
            let it = n.child_by_field_name("right");
            targets(p, &mut out, n.child_by_field_name("left"), &|| {
                Bound::Loop(it)
            });
        }
        "named_expression" => targets(p, &mut out, n.child_by_field_name("name"), &|| {
            Bound::Opaque
        }),
        "as_pattern" => {
            let it = n.named_child(0);
            let mut c = n.walk();
            let aliases: Vec<Node<'t>> = n
                .named_children(&mut c)
                .filter(|k| k.kind() == "as_pattern_target")
                .collect();
            for a in aliases {
                targets(p, &mut out, a.named_child(0), &|| Bound::Loop(it));
            }
        }
        "function_definition" | "class_definition" => {
            if let Some(name) = n.child_by_field_name("name") {
                out.push((p.node_text(&name).to_string(), Bound::Def(n)));
            }
        }
        "parameters" | "lambda_parameters" => {
            let mut c = n.walk();
            for param in n.named_children(&mut c) {
                let (id, ann) = match param.kind() {
                    "typed_parameter" => (param.named_child(0), param.child_by_field_name("type")),
                    "default_parameter" => (param.child_by_field_name("name"), None),
                    "typed_default_parameter" => (
                        param.child_by_field_name("name"),
                        param.child_by_field_name("type"),
                    ),
                    _ => (Some(param), None),
                };
                if let Some(id) = id {
                    for (name, _) in names_of(p, id, true) {
                        out.push((name, Bound::Param(ann)));
                    }
                }
            }
        }
        "global_statement" | "nonlocal_statement" | "delete_statement" | "type_alias_statement" => {
            let declared = n.kind().ends_with("_statement") && n.kind() != "delete_statement";
            let mut c = n.walk();
            for k in n.named_children(&mut c) {
                for (name, _) in names_of(p, k, true) {
                    out.push((
                        name,
                        if declared {
                            Bound::Declared
                        } else {
                            Bound::Opaque
                        },
                    ));
                }
            }
        }
        "case_clause" => {
            let mut c = n.walk();
            for k in n
                .named_children(&mut c)
                .filter(|k| k.kind() == "case_pattern")
            {
                super::callee::walk(k, &mut |x| {
                    if x.kind() == "identifier" {
                        out.push((p.node_text(&x).to_string(), Bound::Opaque));
                    }
                });
            }
        }
        "import_statement" | "import_from_statement" => import_bindings(p, n, &mut out),
        _ => {}
    }
    out
}

fn import_bindings<'t>(p: Src<'_>, n: Node<'t>, out: &mut Found<'t>) {
    let from = n.kind() == "import_from_statement";
    let module = if from {
        n.child_by_field_name("module_name")
            .map(|m| dotted(p, m))
            .unwrap_or("")
    } else {
        ""
    };
    let mut c = n.walk();
    for name in n.children_by_field_name("name", &mut c) {
        let (orig, alias) = if name.kind() == "aliased_import" {
            let o = name.child_by_field_name("name").map(|x| dotted(p, x));
            let a = name.child_by_field_name("alias").map(|x| dotted(p, x));
            (o.unwrap_or(""), a)
        } else {
            (dotted(p, name), None)
        };
        if from {
            let local = alias.unwrap_or(orig);
            out.push((
                local.to_string(),
                Bound::Import(module.to_string(), orig.to_string()),
            ));
        } else if let Some(a) = alias {
            out.push((
                a.to_string(),
                Bound::Import(orig.to_string(), String::new()),
            ));
        } else {
            let first = orig.split('.').next().unwrap_or(orig);
            out.push((
                first.to_string(),
                Bound::Import(first.to_string(), String::new()),
            ));
        }
    }
}

/// File-wide count of the binding sites of one spelling.
#[derive(Default)]
pub(super) struct Site {
    pub total: usize,
    pub imports: Vec<(String, String)>,
    pub classes: usize,
    /// Named by a `global` or `nonlocal` statement.
    pub declared: bool,
}

/// Binding-site census: every spelling's binding sites over every tree-sitter-python binding
/// form. The count is by spelling and conservative; `exec` and `globals()[..]` writes are out of
/// model (the `Exact` doctrine).
#[derive(Default)]
pub(super) struct Sites(BTreeMap<String, Site>);

impl Sites {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut sites = Sites::default();
        super::callee::walk(root, &mut |n| {
            for (name, b) in bindings_of(p, n) {
                let s = sites.0.entry(name).or_default();
                s.total += 1;
                match b {
                    Bound::Import(m, x) => s.imports.push((m, x)),
                    Bound::Def(d) if d.kind() == "class_definition" => s.classes += 1,
                    Bound::Declared => s.declared = true,
                    _ => {}
                }
            }
        });
        sites
    }

    /// Some binding of `name` is `import module` (a module object, never a model class).
    pub(super) fn is_module(&self, name: &str) -> bool {
        self.0
            .get(name)
            .is_some_and(|s| s.imports.iter().any(|(_, member)| member.is_empty()))
    }

    /// The `(module, member)` every binding of `name` imports, when they are all the same import.
    pub(super) fn trusted_import(&self, name: &str) -> Option<&(String, String)> {
        let s = self.0.get(name)?;
        let first = s.imports.first()?;
        (s.total == s.imports.len() && s.imports.iter().all(|i| i == first)).then_some(first)
    }

    /// Only imports of other modules, class definitions, or nothing at all (a star import)
    /// bind the spelling: a module-level object that cannot carry request data.
    pub(super) fn is_schema_name(
        &self,
        name: &str,
        lib: &dyn Fn(&(String, String)) -> bool,
    ) -> bool {
        self.0.get(name).is_none_or(|s| {
            !s.declared && s.total == s.imports.len() + s.classes && !s.imports.iter().any(lib)
        })
    }
}

/// What a name refers to at a use, found lexically.
pub(super) enum Local<'t> {
    /// Bound in no enclosing scope (a builtin, a star import, or a global from elsewhere).
    Unbound,
    /// Bound more than once in the scope that binds it.
    Multi,
    One(Bound<'t>),
}

fn is_scope(kind: &str) -> bool {
    matches!(kind, "function_definition" | "lambda" | "class_definition")
}

/// Bindings of `name` among the direct code of `scope`: nested function, lambda and class
/// bodies are other scopes; comprehensions and blocks are counted here (conservative).
fn scan_scope<'t>(p: Src<'_>, scope: Node<'t>, name: &str) -> Vec<Bound<'t>> {
    let mut found: Vec<Bound<'t>> = Vec::new();
    let mut take = |n: Node<'t>| {
        for (nm, b) in bindings_of(p, n) {
            if nm == name {
                found.push(b);
            }
        }
    };
    if matches!(scope.kind(), "function_definition" | "lambda") {
        if let Some(params) = scope.child_by_field_name("parameters") {
            take(params);
        }
    }
    let mut stack: Vec<Node<'t>> = Vec::new();
    let mut c = scope.walk();
    let body = scope.child_by_field_name("body");
    match body {
        Some(b) if is_scope(scope.kind()) => stack.push(b),
        _ => stack.extend(scope.named_children(&mut c)),
    }
    while let Some(n) = stack.pop() {
        take(n);
        if is_scope(n.kind()) {
            continue;
        }
        let mut k = n.walk();
        stack.extend(n.named_children(&mut k));
    }
    found
}

/// Resolve `name` at `from`: the nearest enclosing scope that binds it, which must bind it
/// exactly once. Class bodies are visible only to code directly in them.
pub(super) fn local_binding<'t>(
    p: Src<'_>,
    sites: &Sites,
    from: Node<'t>,
    name: &str,
) -> Local<'t> {
    if sites.0.get(name).is_some_and(|s| s.declared) {
        return Local::Multi;
    }
    let mut crossed = false;
    let mut cur = from.parent();
    while let Some(s) = cur {
        let scoped = match s.kind() {
            "function_definition" | "lambda" => Some(true),
            "class_definition" => Some(false),
            "module"
            | "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => Some(false),
            _ => None,
        };
        if let Some(is_fn) = scoped {
            if s.kind() != "class_definition" || !crossed {
                let mut found = scan_scope(p, s, name);
                if found.len() > 1 {
                    return Local::Multi;
                }
                if let Some(b) = found.pop() {
                    return Local::One(b);
                }
            }
            crossed |= is_fn;
        }
        cur = s.parent();
    }
    Local::Unbound
}
