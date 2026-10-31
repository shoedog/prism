use super::timeline::{Event, Summary};
use super::{NameFacts, Recovery, SpanValue};
use crate::ast::ParsedFile;
use crate::resolution::ReceiverRecovery;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tree_sitter::Node;

fn boundary(p: &ParsedFile, n: Node<'_>) -> bool {
    p.language.function_node_types().contains(&n.kind())
        || matches!(
            n.kind(),
            "block"
                | "expression_case"
                | "type_case"
                | "communication_case"
                | "func_literal"
                | "if_statement"
                | "for_statement"
                | "expression_switch_statement"
                | "type_switch_statement"
                | "select_statement"
        )
}

/// Earliest declaration per scope/name, with precisely the old scope-walk
/// fences. A short declaration stops recursion even when its RHS has bindings.
struct Scopes(HashMap<usize, BTreeMap<String, usize>>);
impl Scopes {
    fn new(p: &ParsedFile, root: Node<'_>) -> Self {
        fn names(p: &ParsedFile, n: Node<'_>, is_root: bool, out: &mut BTreeMap<String, usize>) {
            if !is_root && boundary(p, n) {
                return;
            }
            if n.kind() == "short_var_declaration" {
                if let Some(left) = n.child_by_field_name("left") {
                    for name in p.go_short_decl_names(left) {
                        out.entry(name).or_insert(n.start_byte());
                    }
                    return;
                }
            }
            if n.kind() == "var_spec" {
                let mut c = n.walk();
                for name in n.children_by_field_name("name", &mut c) {
                    out.entry(p.node_text(&name).trim().to_owned())
                        .or_insert(n.start_byte());
                }
            }
            let mut c = n.walk();
            for child in n.children(&mut c) {
                names(p, child, false, out);
            }
        }
        fn walk(p: &ParsedFile, root: Node<'_>, n: Node<'_>, out: &mut Scopes) {
            if boundary(p, n) {
                let mut row = BTreeMap::new();
                let owner = n
                    .parent()
                    .filter(|v| {
                        matches!(
                            v.kind(),
                            "function_declaration" | "method_declaration" | "func_literal"
                        )
                    })
                    .or_else(|| {
                        root.child_by_field_name("body")
                            .filter(|body| body.start_byte() == n.start_byte())
                            .map(|_| root)
                    });
                if let Some(owner) = owner {
                    for (name, _) in parameters(p, owner, false) {
                        row.insert(name, 0);
                    }
                }
                names(p, n, true, &mut row);
                out.0.insert(n.id(), row);
            }
            let mut c = n.walk();
            for child in n.children(&mut c) {
                walk(p, root, child, out);
            }
        }
        let mut out = Self(HashMap::new());
        walk(p, root, root, &mut out);
        out
    }
    fn reuses(&self, p: &ParsedFile, n: Node<'_>) -> BTreeSet<String> {
        let Some(scope) = p.go_enclosing_binding_scope(n) else {
            return BTreeSet::new();
        };
        let Some(left) = n.child_by_field_name("left") else {
            return BTreeSet::new();
        };
        let names = p.go_short_decl_names(left);
        let declared = |name: &str| {
            self.0
                .get(&scope.id())
                .and_then(|s| s.get(name))
                .is_some_and(|start| *start < n.start_byte())
        };
        names
            .iter()
            .filter(|receiver| {
                names.len() >= 2
                    && declared(receiver)
                    && names
                        .iter()
                        .any(|name| name != *receiver && name != "_" && !declared(name))
            })
            .cloned()
            .collect()
    }
}

pub(super) fn parameters(
    p: &ParsedFile,
    owner: Node<'_>,
    only_plain: bool,
) -> Vec<(String, Recovery)> {
    let mut out = Vec::new();
    if let Some(params) = p.find_parameters_node(&owner) {
        let mut c = params.walk();
        for param in params.children(&mut c) {
            if only_plain && param.kind() != "parameter_declaration" {
                continue;
            }
            let Some(ty) = param.child_by_field_name("type") else {
                continue;
            };
            let mut pc = param.walk();
            let mut seen = BTreeSet::new();
            for name in param.children(&mut pc) {
                if name.start_byte() < ty.start_byte()
                    && name.kind() == "identifier"
                    && seen.insert(p.node_text(&name).to_owned())
                {
                    out.push((
                        p.node_text(&name).to_owned(),
                        (p.node_text(&ty).to_owned(), ReceiverRecovery::TypedParam),
                    ));
                }
            }
        }
    }
    out
}

fn binding_names(p: &ParsedFile, n: Node<'_>) -> BTreeSet<String> {
    fn walk(p: &ParsedFile, n: Node<'_>, out: &mut BTreeSet<String>) {
        if p.language.is_identifier_node(n.kind()) {
            out.insert(p.node_text(&n).to_owned());
            return;
        }
        if matches!(
            n.kind(),
            "attribute" | "subscript" | "member_expression" | "subscript_expression"
        ) {
            return;
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            walk(p, child, out);
        }
    }
    let mut out = BTreeSet::new();
    if let Some(name) = p.simple_binding_text(&n) {
        out.insert(name);
    }
    walk(p, n, &mut out);
    out
}

fn call_rhs(p: &ParsedFile, n: Node<'_>) -> Result<String, ()> {
    let right = n.child_by_field_name("right").ok_or(())?;
    let mut c = right.walk();
    let rhs: Vec<_> = right.named_children(&mut c).collect();
    if rhs.len() != 1 || rhs[0].kind() != "call_expression" {
        return Err(());
    }
    let func = rhs[0].child_by_field_name("function").ok_or(())?;
    if !matches!(func.kind(), "identifier" | "selector_expression") {
        return Err(());
    }
    Ok(p.node_text(&func).trim().to_owned())
}

struct Collector<'p> {
    p: &'p ParsedFile,
    scopes: Scopes,
    names: BTreeMap<String, NameFacts>,
}
impl Collector<'_> {
    fn event(
        &mut self,
        name: String,
        n: Node<'_>,
        span: (usize, usize),
        value: Option<Recovery>,
        flags: (bool, bool, bool),
    ) {
        let (lexical, var, reuse) = flags;
        self.names.entry(name).or_default().events.push(Event {
            lo: span.0.max(n.start_byte() + 1),
            hi: span.1,
            value: Summary::binding(value, lexical),
            var,
            reuse,
        });
    }
    fn walk(
        &mut self,
        n: Node<'_>,
        root: bool,
        mut span: (usize, usize),
        blocked_rhs: &mut Vec<String>,
    ) {
        let p = self.p;
        if !root && p.language.function_node_types().contains(&n.kind()) {
            return;
        }
        if !root && n.kind() == "func_literal" {
            span = intersect(span, (n.start_byte(), n.end_byte()));
            for (name, ty) in parameters(p, n, true) {
                self.event(name, n, span, Some(ty), (true, false, false));
            }
        }
        let scoped = || {
            p.go_enclosing_binding_scope(n)
                .map(|s| intersect(span, (s.start_byte(), s.end_byte())))
                .unwrap_or((0, 0))
        };
        let mut rhs_block = None;
        match n.kind() {
            "short_var_declaration" => {
                let reuses = self.scopes.reuses(p, n);
                let lhs = n.child_by_field_name("left");
                if let Some(left) = lhs {
                    let names = p.go_short_decl_names(left);
                    let first = names.first();
                    for name in &reuses {
                        self.names
                            .entry(name.clone())
                            .or_default()
                            .reuses
                            .push(SpanValue {
                                lo: scoped().0.max(n.start_byte() + 1),
                                hi: scoped().1,
                                value: if first == Some(name) {
                                    call_rhs(p, n)
                                } else {
                                    Err(())
                                },
                            });
                    }
                    // RHS relocation prunes a receiver-first statement's subtree
                    // even if the statement itself has no qualifying call RHS.
                    let mut c = left.walk();
                    let first_rhs_name = left
                        .named_children(&mut c)
                        .next()
                        .map(|v| p.node_text(&v).trim().to_owned());
                    if let Some(name) =
                        first_rhs_name.filter(|_| n.child_by_field_name("right").is_some())
                    {
                        if !blocked_rhs.contains(&name) {
                            if let Ok(value) = call_rhs(p, n) {
                                self.names
                                    .entry(name.clone())
                                    .or_default()
                                    .rhs
                                    .push(SpanValue {
                                        lo: span.0.max(n.start_byte() + 1),
                                        hi: span.1,
                                        value,
                                    });
                            }
                        }
                        rhs_block = Some(name);
                    }
                }
                if let Some(left) = lhs.or_else(|| n.child_by_field_name("name")) {
                    let simple = p.simple_binding_text(&left);
                    let value = n
                        .child_by_field_name("right")
                        .or_else(|| n.child_by_field_name("value"))
                        .and_then(|r| {
                            p.constructor_type(&r)
                                .or_else(|| p.first_constructor_type_child(&r))
                        })
                        .map(|ty| (ty, ReceiverRecovery::ConstructorLocal));
                    for name in binding_names(p, left) {
                        let ty = if simple.as_ref() == Some(&name) {
                            value.clone()
                        } else {
                            None
                        };
                        let reuse = reuses.contains(&name);
                        self.event(name, n, scoped(), ty, (true, false, reuse));
                    }
                }
            }
            "var_spec" => {
                let mut c = n.walk();
                let names: Vec<_> = n.children_by_field_name("name", &mut c).collect();
                let value = if let Some(ty) = n.child_by_field_name("type") {
                    Some(p.node_text(&ty).to_owned())
                } else if names.len() == 1 {
                    n.child_by_field_name("value").and_then(|v| {
                        p.constructor_type(&v)
                            .or_else(|| p.first_constructor_type_child(&v))
                    })
                } else {
                    None
                }
                .map(|ty| (ty, ReceiverRecovery::VarDecl));
                let names: BTreeSet<_> = names
                    .iter()
                    .filter_map(|v| p.simple_binding_text(v))
                    .collect();
                for name in names {
                    self.event(name, n, scoped(), value.clone(), (true, true, false));
                }
            }
            "assignment_statement" => {
                if let Some(name) = n
                    .child_by_field_name("left")
                    .or_else(|| n.child_by_field_name("left_operand"))
                    .and_then(|v| p.simple_binding_text(&v))
                {
                    self.event(name, n, span, None, (false, false, false));
                }
            }
            "type_switch_statement" => {
                if let Some(alias) = n.child_by_field_name("alias") {
                    let mut c = n.walk();
                    for case in n.named_children(&mut c).filter(|v| v.kind() == "type_case") {
                        for name in binding_names(p, alias) {
                            self.event(
                                name,
                                n,
                                intersect(span, (case.start_byte(), case.end_byte())),
                                None,
                                (true, false, false),
                            );
                        }
                    }
                }
            }
            "range_clause" if p.node_text(&n).contains(":=") => {
                if let (Some(left), Some(body)) = (
                    n.child_by_field_name("left"),
                    n.parent()
                        .filter(|v| v.kind() == "for_statement")
                        .and_then(|v| v.child_by_field_name("body")),
                ) {
                    for name in binding_names(p, left) {
                        self.event(
                            name,
                            n,
                            intersect(span, (body.start_byte(), body.end_byte())),
                            None,
                            (true, false, false),
                        );
                    }
                }
            }
            _ => {}
        }
        if let Some(name) = rhs_block.as_ref() {
            blocked_rhs.push(name.clone());
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            self.walk(child, false, span, blocked_rhs);
        }
        if rhs_block.is_some() {
            blocked_rhs.pop();
        }
    }
}

fn intersect(a: (usize, usize), b: (usize, usize)) -> (usize, usize) {
    (a.0.max(b.0), a.1.min(b.1))
}

pub(super) fn collect(p: &ParsedFile, root: Node<'_>) -> BTreeMap<String, NameFacts> {
    let mut collector = Collector {
        p,
        scopes: Scopes::new(p, root),
        names: BTreeMap::new(),
    };
    for (name, ty) in parameters(p, root, true) {
        let seed = &mut collector.names.entry(name).or_default().seed;
        *seed = seed.join(&Summary::binding(Some(ty), false));
    }
    for row in collector.names.values_mut() {
        row.seed = row.seed.clone().parameters();
    }
    collector.walk(root, true, (0, usize::MAX), &mut Vec::new());
    collector.names
}
