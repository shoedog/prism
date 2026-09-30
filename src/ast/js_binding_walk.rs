//! S1b-3 (SPEC §3.1a): the site walk, driven by the evaluation-context table `E(kind, field)`.
//! At every step from a child to its parent the walk looks up the parent's kind and the
//! child's field. Kinds outside Σ′ are structural. Every field of a Σ′ kind has a rule; a
//! field without one is an unproven position and keeps base behavior (owner OQ1 = K).
use super::js_binding::{is_class, JsBindingCache};
use super::js_binding_site::is_scope;
use super::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// How the child at a Σ′ kind's field is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pos {
    /// In the environment the kind opens (look the name up there if it is a scope).
    Inside,
    /// J1: in the parameter environment, already looked up; skip the function's own.
    Param,
    /// In the enclosing environment: skip this node's (J3, the `with` object).
    Outside,
    /// The `with` body: an object environment that may supply any name.
    WithBody,
    /// A decorator expression: J2 by holder.
    Decorator,
    /// A namespace or enum body: on leaving it unbound, apply the leave predicate.
    Leave,
    /// Never a site position (a binding name or type space); unproven if reached.
    Unlisted,
}

/// The walk's answer: the scope that binds the name (`explicit` for a `with` body or a
/// dotted-namespace segment, otherwise the scope's own index), no binding, or an unproven
/// position (base behavior, counted by reason).
pub(crate) enum Walk<'a> {
    Found(Node<'a>, Option<Node<'a>>),
    Unbound,
    Unchecked(&'static str),
}

/// The evaluation-context table (SPEC §3.1a). `probes/grammar_closure.py` derives every
/// `(kind, field)` of every Σ′ kind from the pinned grammars and checks this table against it;
/// "children" names the unfielded named children.
pub(crate) const E_TABLE: &[(&str, &str, Pos)] = &[
    ("program", "children", Pos::Inside),
    ("function_declaration", "body", Pos::Inside),
    ("function_declaration", "parameters", Pos::Param),
    ("function_declaration", "name", Pos::Unlisted),
    ("function_declaration", "return_type", Pos::Unlisted),
    ("function_declaration", "type_parameters", Pos::Unlisted),
    ("generator_function_declaration", "body", Pos::Inside),
    ("generator_function_declaration", "parameters", Pos::Param),
    ("generator_function_declaration", "name", Pos::Unlisted),
    (
        "generator_function_declaration",
        "return_type",
        Pos::Unlisted,
    ),
    (
        "generator_function_declaration",
        "type_parameters",
        Pos::Unlisted,
    ),
    ("function_expression", "body", Pos::Inside),
    ("function_expression", "parameters", Pos::Param),
    ("function_expression", "name", Pos::Unlisted),
    ("function_expression", "return_type", Pos::Unlisted),
    ("function_expression", "type_parameters", Pos::Unlisted),
    ("generator_function", "body", Pos::Inside),
    ("generator_function", "parameters", Pos::Param),
    ("generator_function", "name", Pos::Unlisted),
    ("generator_function", "return_type", Pos::Unlisted),
    ("generator_function", "type_parameters", Pos::Unlisted),
    ("arrow_function", "body", Pos::Inside),
    ("arrow_function", "parameters", Pos::Param),
    ("arrow_function", "parameter", Pos::Param),
    ("arrow_function", "return_type", Pos::Unlisted),
    ("arrow_function", "type_parameters", Pos::Unlisted),
    ("method_definition", "body", Pos::Inside),
    ("method_definition", "parameters", Pos::Param),
    ("method_definition", "name", Pos::Outside),
    ("method_definition", "decorator", Pos::Outside),
    ("method_definition", "children", Pos::Unlisted),
    ("method_definition", "return_type", Pos::Unlisted),
    ("method_definition", "type_parameters", Pos::Unlisted),
    ("formal_parameters", "children", Pos::Inside),
    ("statement_block", "children", Pos::Inside),
    ("class_static_block", "body", Pos::Inside),
    ("for_statement", "initializer", Pos::Inside),
    ("for_statement", "condition", Pos::Inside),
    ("for_statement", "increment", Pos::Inside),
    ("for_statement", "body", Pos::Inside),
    ("for_in_statement", "left", Pos::Inside),
    ("for_in_statement", "right", Pos::Inside),
    ("for_in_statement", "value", Pos::Inside),
    ("for_in_statement", "body", Pos::Inside),
    ("catch_clause", "parameter", Pos::Inside),
    ("catch_clause", "body", Pos::Inside),
    ("catch_clause", "type", Pos::Unlisted),
    ("switch_body", "children", Pos::Inside),
    ("class_declaration", "body", Pos::Inside),
    ("class_declaration", "children", Pos::Inside),
    ("class_declaration", "decorator", Pos::Outside),
    ("class_declaration", "name", Pos::Unlisted),
    ("class_declaration", "type_parameters", Pos::Unlisted),
    ("class", "body", Pos::Inside),
    ("class", "children", Pos::Inside),
    ("class", "decorator", Pos::Outside),
    ("class", "name", Pos::Unlisted),
    ("class", "type_parameters", Pos::Unlisted),
    ("abstract_class_declaration", "body", Pos::Inside),
    ("abstract_class_declaration", "children", Pos::Inside),
    ("abstract_class_declaration", "decorator", Pos::Outside),
    ("abstract_class_declaration", "name", Pos::Unlisted),
    (
        "abstract_class_declaration",
        "type_parameters",
        Pos::Unlisted,
    ),
    ("with_statement", "object", Pos::Outside),
    ("with_statement", "body", Pos::WithBody),
    ("internal_module", "body", Pos::Leave),
    ("internal_module", "name", Pos::Unlisted),
    ("module", "body", Pos::Leave),
    ("module", "name", Pos::Unlisted),
    ("enum_declaration", "body", Pos::Leave),
    ("enum_declaration", "name", Pos::Unlisted),
    ("enum_body", "children", Pos::Inside),
    ("enum_body", "name", Pos::Inside),
    ("decorator", "children", Pos::Decorator),
];

fn e_rule(kind: &str, field: &str) -> Option<Pos> {
    let mut known = false;
    for (k, f, p) in E_TABLE {
        if *k == kind {
            known = true;
            if *f == field {
                return Some(*p);
            }
        }
    }
    known.then_some(Pos::Unlisted)
}

/// The field through which `child` hangs off `parent` ("children" when unfielded).
fn field_of(parent: Node<'_>, child: Node<'_>) -> &'static str {
    let mut cursor = parent.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.node().id() == child.id() {
                return cursor.field_name().unwrap_or("children");
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    "children"
}

impl ParsedFile {
    /// The nearest environment enclosing `site` that declares `name` (SPEC §3.1a).
    pub(crate) fn js_ts_binding_walk<'a>(
        &'a self,
        site: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> Walk<'a> {
        let mut child = site;
        let mut current = site.parent();
        while let Some(node) = current {
            let kind = node.kind();
            let field = field_of(node, child);
            let mut next = node;
            match e_rule(kind, field) {
                None | Some(Pos::Param) | Some(Pos::Outside) => {}
                Some(Pos::Unlisted) => return Walk::Unchecked("unproven_position"),
                Some(Pos::WithBody) => return Walk::Found(node, Some(node)),
                Some(Pos::Decorator) => {
                    let Some(holder) = node.parent() else {
                        return Walk::Unchecked("unproven_position");
                    };
                    if holder.kind() == "class" && holder.child_by_field_name("name").is_some() {
                        return Walk::Unchecked("decorated_class_expression");
                    }
                    if is_class(holder.kind()) {
                        next = holder; // J2: a class decorator is evaluated outside the class.
                    } else {
                        // A member or parameter decorator is evaluated in the class scope (T8).
                        let mut up = Some(holder);
                        while let Some(n) = up.filter(|n| n.kind() != "class_body") {
                            up = n.parent();
                        }
                        let Some(body) = up else {
                            return Walk::Unchecked("unproven_position");
                        };
                        child = body;
                        current = body.parent();
                        continue;
                    }
                }
                Some(Pos::Leave) => {
                    // Dotted `namespace A.B.C`: inside the body, B and C denote namespaces.
                    let text = node.child_by_field_name("name");
                    let text = text.map_or("", |n| self.node_text(&n));
                    if kind == "internal_module"
                        && text.split('.').skip(1).any(|s| s.trim() == name)
                    {
                        return Walk::Found(node, Some(node));
                    }
                    if self.js_ts_merge_leave(node, cache) {
                        return Walk::Unchecked("namespace_leave");
                    }
                }
                Some(Pos::Inside) => {
                    // Fold F (gpt-6.1-sol r1 W-S; Opus r1 S1 independently found the same dead
                    // code): for every Σ′ kind, `Pos::Inside` is reached only through a field
                    // this table maps to `Inside`, and a function-like kind's only `Inside`
                    // field is `body` (its other fields are `Param`/`Unlisted`), so a guard
                    // gating this arm by "own field is body" was always true and never fired
                    // for any other value — dead code, confirmed behavior-preserving by two
                    // independent reviewers (1,247/1,247 library tests unchanged) and dropped.
                    // This also directly restores C-M15 (`method_definition`'s computed `name`
                    // field, Outside vs Inside) as a classify-level killable mutant: with the
                    // guard gone, flipping that row to Inside makes the walk look the computed
                    // key up inside the method's own environment, killed by
                    // `fold_writes_before_function_environments_are_may_call`'s J2 decorator
                    // row and the new C-M15 row below.
                    if is_scope(node) || kind == "enum_body" {
                        let (decls, annex_b) = self.js_ts_scope_lookup(node, name, cache);
                        if decls.is_some() || annex_b {
                            return Walk::Found(node, None);
                        }
                    }
                }
            }
            child = next;
            current = next.parent();
        }
        Walk::Unbound
    }

    /// Leaving a namespace or enum body unbound (SPEC §3.1a leave predicate): unproven when
    /// another namespace, module or enum in the file shares the first name segment (a merge
    /// partner), or the file is a script (cross-file merging).
    fn js_ts_merge_leave<'a>(&'a self, node: Node<'a>, cache: &mut JsBindingCache<'a>) -> bool {
        let first = |n: Node<'_>| {
            n.child_by_field_name("name").map(|x| {
                let text = self.node_text(&x);
                text.split('.').next().unwrap_or("").trim().to_string()
            })
        };
        let partners = cache.partners.get_or_insert_with(|| {
            let mut out = BTreeMap::<String, usize>::new();
            let mut stack = vec![self.tree.root_node()];
            while let Some(n) = stack.pop() {
                if matches!(n.kind(), "internal_module" | "module" | "enum_declaration") {
                    if let Some(f) = first(n) {
                        *out.entry(f).or_default() += 1;
                    }
                }
                let mut cursor = n.walk();
                stack.extend(n.named_children(&mut cursor));
            }
            out
        });
        let shared = first(node).is_some_and(|f| partners.get(&f).copied().unwrap_or(0) >= 2);
        shared || !self.js_ts_is_module()
    }
}

#[cfg(test)]
#[path = "js_binding_walk_tests.rs"]
mod tests;
