//! Python facts for rules U and D (#630 slice 5). Removal-only, lexical.
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Scopes the producer does not split from the owner: a def or use inside one is
/// never proven (comprehensions are scopes; a class body binds its own names).
pub(super) const NESTED_SCOPES: [&str; 5] = [
    "list_comprehension",
    "dictionary_comprehension",
    "set_comprehension",
    "generator_expression",
    "class_definition",
];

/// Builtins that read a function's locals by name at run time (a string passed to `eval`/`exec`,
/// the mapping `locals()`/`vars()`/`globals()` returns, `str.format(**locals())`). Prism sees no
/// read for them, so U could take a line that reads a name this way as write-only.
const HIDDEN_READERS: [&str; 5] = ["eval", "exec", "locals", "vars", "globals"];

/// Names of the builtins namespace: `builtins.eval(..)`, `getattr(builtins, "eval")` and
/// `__builtins__["eval"]` reach a hidden reader through them.
const BUILTIN_NAMESPACES: [&str; 2] = ["builtins", "__builtins__"];

/// Names the file binds, anywhere, to a hidden reader or to the builtins namespace: a plain
/// assignment from one (`ev = eval`, chains followed), any name imported from `builtins`
/// (`from builtins import eval as ev`) and an alias of the module (`import builtins as b`).
/// A reader carried as data (a parameter, an attribute, another module's alias) is out of model.
fn file_hidden_aliases(parsed: &ParsedFile) -> BTreeSet<String> {
    let mut aliases = BTreeSet::new();
    let mut assigns: Vec<(String, String)> = Vec::new();
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "assignment" => {
                if let (Some(l), Some(r)) = (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ) {
                    if l.kind() == "identifier" && r.kind() == "identifier" {
                        assigns.push((
                            parsed.node_text(&l).to_owned(),
                            parsed.node_text(&r).to_owned(),
                        ));
                    }
                }
            }
            "import_from_statement" => {
                let from_builtins = node
                    .child_by_field_name("module_name")
                    .is_some_and(|m| BUILTIN_NAMESPACES.contains(&parsed.node_text(&m)));
                if from_builtins {
                    let mut c = node.walk();
                    for name in node.children_by_field_name("name", &mut c) {
                        let bound = match name.kind() {
                            "aliased_import" => name.child_by_field_name("alias"),
                            _ => Some(name),
                        };
                        if let Some(b) = bound {
                            aliases.insert(parsed.node_text(&b).to_owned());
                        }
                    }
                }
            }
            "aliased_import" => {
                let module = node
                    .child_by_field_name("name")
                    .is_some_and(|m| BUILTIN_NAMESPACES.contains(&parsed.node_text(&m)));
                if module {
                    if let Some(a) = node.child_by_field_name("alias") {
                        aliases.insert(parsed.node_text(&a).to_owned());
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    let reader = |n: &str, aliases: &BTreeSet<String>| {
        HIDDEN_READERS.contains(&n) || BUILTIN_NAMESPACES.contains(&n) || aliases.contains(n)
    };
    loop {
        let before = aliases.len();
        for (l, r) in &assigns {
            if reader(r, &aliases) {
                aliases.insert(l.clone());
            }
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// Whether the owner (nested scopes included) can reach a hidden reader: a bare name that is
/// one of them, the builtins namespace, or a file-level alias of either (`file_hidden_aliases`),
/// or a string literal spelling a reader (`getattr(builtins, "eval")`). The member of an
/// attribute (`model.eval()`) and a keyword argument's name (`f(eval=True)`) are not the builtin.
/// U and D are off for such an owner (#805: precise handling would let them back on).
pub(super) fn has_hidden_reader(parsed: &ParsedFile, owner: Node<'_>) -> bool {
    let aliases = parsed.python_hidden_aliases(file_hidden_aliases);
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier" => {
                let text = parsed.node_text(&node);
                if HIDDEN_READERS.contains(&text)
                    || BUILTIN_NAMESPACES.contains(&text)
                    || aliases.contains(text)
                {
                    let member = node.parent().is_some_and(|p| {
                        (p.kind() == "attribute"
                            && p.child_by_field_name("attribute")
                                .is_some_and(|a| a.id() == node.id()))
                            || (p.kind() == "keyword_argument"
                                && p.child_by_field_name("name")
                                    .is_some_and(|a| a.id() == node.id()))
                    });
                    if !member {
                        return true;
                    }
                }
            }
            "string_content" => {
                if HIDDEN_READERS.contains(&parsed.node_text(&node)) {
                    return true;
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    false
}

/// Single-entry region: control enters a `block` only at its first statement.
pub(super) fn is_region(kind: &str) -> bool {
    kind == "block"
}

/// Names declared `global`/`nonlocal` anywhere in the owner (nested defs included):
/// another callable may write them between a dominating write and a read.
pub(super) fn note_scope_declaration(
    parsed: &ParsedFile,
    node: Node<'_>,
    out: &mut BTreeSet<String>,
) {
    if matches!(node.kind(), "global_statement" | "nonlocal_statement") {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            out.insert(parsed.node_text(&child).to_owned());
        }
    }
}

/// The write statement of an lvalue occurrence, and whether it is a plain `=`.
/// Only a whole `expression_statement` holding one assignment qualifies: a chained
/// inner target, a walrus, loop/`with`/`except` target, import or `x: int` is no write.
/// Tuple/list targets write every element when the statement completes; an abnormal
/// completion leaves the region, so each element is an unconditional write.
pub(super) fn classify_write<'a>(node: Node<'a>, span: (usize, usize)) -> Option<(Node<'a>, bool)> {
    let mut node = node.parent()?;
    loop {
        match node.kind() {
            "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern" => {}
            "assignment" | "augmented_assignment" => {
                let left = node.child_by_field_name("left")?;
                node.child_by_field_name("right")?;
                if !(left.start_byte() <= span.0 && span.1 <= left.end_byte()) {
                    return None;
                }
                let statement = node
                    .parent()
                    .filter(|p| p.kind() == "expression_statement")?;
                return Some((statement, node.kind() == "assignment"));
            }
            _ => return None,
        }
        node = node.parent()?;
    }
}

/// Receiver names whose attribute paths D proves (`self.x`, `cls.x`): fixed storage.
/// Properties, descriptors, `__setattr__`/`__getattr__`/`__getattribute__`, `__slots__`
/// tricks and runtime mutation are out of model, as for Exact.
const RECEIVERS: [&str; 2] = ["self", "cls"];

/// The receiver names (`self`/`cls`) that are parameters of `owner` and are never rebound
/// anywhere in it, nested callables, comprehensions and class bodies included.
pub(super) fn stable_receivers(parsed: &ParsedFile, owner: Node<'_>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Some(params) = owner.child_by_field_name("parameters") else {
        return found;
    };
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        let name = match param.kind() {
            "identifier" => Some(param),
            "default_parameter" | "typed_default_parameter" => param.child_by_field_name("name"),
            "typed_parameter" => param.named_child(0).filter(|n| n.kind() == "identifier"),
            _ => None,
        };
        if let Some(name) = name {
            let text = parsed.node_text(&name);
            if RECEIVERS.contains(&text) {
                found.insert(text.to_owned());
            }
        }
    }
    if !found.is_empty() {
        strip_rebound(parsed, owner, params.id(), &mut found);
    }
    found
}

fn strip_rebound(parsed: &ParsedFile, node: Node<'_>, skip: usize, found: &mut BTreeSet<String>) {
    if node.id() == skip {
        return;
    }
    if node.kind() == "identifier" && is_binding_occurrence(node) {
        found.remove(parsed.node_text(&node));
    }
    if found.is_empty() {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        strip_rebound(parsed, child, skip, found);
    }
}

/// Conservative: any form that can bind or unbind the identifier counts (a read that
/// shares a binding shape, such as a nested parameter default, only costs a keep).
fn is_binding_occurrence(ident: Node<'_>) -> bool {
    let mut up = ident.parent();
    while let Some(a) = up {
        if matches!(
            a.kind(),
            "case_pattern" | "import_statement" | "import_from_statement"
        ) {
            return true;
        }
        up = a.parent();
    }
    let mut node = ident;
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "as_pattern" => return node.kind() == "as_pattern_target",
            "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "list_splat_pattern"
            | "parenthesized_expression"
            | "tuple"
            | "expression_list"
            | "list_splat"
            | "as_pattern_target"
            | "dictionary_splat_pattern" => {}
            "assignment" | "augmented_assignment" => {
                return parent.child_by_field_name("left") == Some(node)
            }
            "for_statement" | "for_in_clause" => {
                return parent.child_by_field_name("left") == Some(node)
            }
            "named_expression" => return parent.child_by_field_name("name") == Some(node),
            "function_definition" | "class_definition" => {
                return parent.child_by_field_name("name") == Some(node)
            }
            "delete_statement"
            | "global_statement"
            | "nonlocal_statement"
            | "type_alias_statement" => return true,
            "except_clause" | "except_group_clause" => {
                return node
                    .prev_sibling()
                    .is_some_and(|s| !s.is_named() && s.kind() == "as")
            }
            "parameters"
            | "lambda_parameters"
            | "default_parameter"
            | "typed_parameter"
            | "typed_default_parameter" => return true,
            _ => return false,
        }
        node = parent;
    }
    false
}
