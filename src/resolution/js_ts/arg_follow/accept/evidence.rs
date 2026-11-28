//! Syntax, scope and callable evidence for the E1 accept-list.

use super::super::*;
use super::{first, rank, Check, Expr, Proof, Span};
use crate::ast::{js_ts::formal_binding, JsBinding};
use param_slot::{children, name, peel, property_key, Walk};

pub(super) fn ordinary_call(node: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let callee = node.child_by_field_name("function").and_then(peel)?;
    let args = node.child_by_field_name("arguments")?;
    (node.kind() == "call_expression" && args.kind() == "arguments").then_some((callee, args))
}

pub(super) fn static_member<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<(Node<'a>, String)> {
    let object = node
        .child_by_field_name("object")
        .and_then(peel)
        .filter(|node| node.kind() == "identifier")?;
    let key = match node.kind() {
        "member_expression" => node.child_by_field_name("property")?,
        "subscript_expression" => node
            .child_by_field_name("index")
            .and_then(peel)
            .filter(|key| matches!(key.kind(), "string" | "number"))?,
        _ => return None,
    };
    Some((object, property_key(parsed, key)?))
}

pub(super) fn copy_declaration<'a>(read: Node<'a>, observation: Node<'a>) -> Option<Node<'a>> {
    let value = outer(read);
    value.parent().filter(|node| {
        param_slot::is_const(*node)
            && node.child_by_field_name("value") == Some(value)
            && node.end_byte() <= observation.start_byte()
    })
}

impl<'a> Proof<'a, '_> {
    pub(super) fn source_dynamic(&self, file: &'a str, call: Node<'a>, callee: Node<'a>) -> bool {
        let parsed = &self.context.files[file];
        let mut cache = self.context.bindings[file].cache();
        let lookup = callee
            .child_by_field_name("object")
            .and_then(peel)
            .unwrap_or(callee);
        if ancestors(call).any(|node| node.kind() == "with_statement") {
            return true;
        }
        name(parsed, lookup).is_some_and(|spelling| {
            match parsed.js_ts_binding_walk(lookup, &spelling, &mut cache) {
                Walk::Found(scope, None) => self.dynamic_lookup(file, scope, &spelling, &mut cache),
                Walk::Found(_, Some(_)) | Walk::Unchecked(_) => true,
                Walk::Unbound => false,
            }
        })
    }

    fn dynamic_lookup(
        &self,
        file: &'a str,
        scope: Node<'a>,
        spelling: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let parsed = &self.context.files[file];
        self.context.bindings[file]
            .eval_sites
            .get_or_init(|| identity::index_eval_sites(parsed))
            .get(&scope.id())
            .into_iter()
            .flatten()
            .any(|&(call, callee)| {
                matches!(parsed.js_ts_binding_walk(callee, spelling, cache),
                    Walk::Found(found, _) if canonical_scope(found) == canonical_scope(scope))
                    && !self.primitive_eval(parsed, callee, cache)
                    && identity::eval_may_write(parsed, call, callee, spelling, cache)
            })
    }

    fn primitive_eval(
        &self,
        parsed: &'a ParsedFile,
        callee: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        parsed
            .js_ts_site_declaration(callee, "eval", cache)
            .is_some_and(|(scope, decl)| {
                param_slot::is_const(decl)
                    && const_before(parsed, decl, callee)
                    && !parsed.js_ts_scoped_written(scope, "eval", cache)
                    && decl
                        .child_by_field_name("value")
                        .and_then(peel)
                        .is_some_and(primitive)
            })
    }

    pub(super) fn undefined(&self, (file, node): Expr<'a>) -> bool {
        let parsed = &self.context.files[file];
        if node.kind() == "unary_expression" && has_token(node, "void") {
            return node
                .child_by_field_name("argument")
                .and_then(peel)
                .is_some_and(primitive);
        }
        let mut cache = self.context.bindings[file].cache();
        // A sloppy direct eval anywhere in the file that may declare
        // `undefined` can shadow the intrinsic in an enclosing variable
        // environment the lexical walk does not see; a proved non-callable
        // `eval` binding cannot.
        let eval_may_shadow = self.context.bindings[file]
            .eval_sites
            .get_or_init(|| identity::index_eval_sites(parsed))
            .get(&parsed.tree.root_node().id())
            .into_iter()
            .flatten()
            .any(|&(call, callee)| {
                !self.primitive_eval(parsed, callee, &mut cache)
                    && identity::eval_may_write(parsed, call, callee, "undefined", &mut cache)
            });
        !eval_may_shadow
            && name(parsed, node).as_deref() == Some("undefined")
            && matches!(
                parsed.js_ts_binding_walk(node, "undefined", &mut cache),
                Walk::Unbound
            )
            && !ancestors(node).any(|node| node.kind() == "with_statement")
    }

    pub(super) fn closed(&self, expr: Expr<'a>, depth: usize) -> Check<()> {
        let (file, node) = normalize(Some(expr)).ok_or("hidden_literal_key")?;
        match node.kind() {
            _ if primitive(node) || self.undefined((file, node)) => Ok(()),
            "array" | "object" if depth >= 2 => Err("hidden_literal_key"),
            "array" => param_slot::array_entries(node)
                .into_iter()
                .filter_map(|entry| {
                    entry
                        .ok_or("hidden_literal_key")
                        .and_then(|entry| self.closed((file, entry), depth + 1))
                        .err()
                })
                .min_by_key(|reason| rank(reason))
                .map_or(Ok(()), Err),
            "object" => self.object((file, node), depth).map(|_| ()),
            _ => self
                .callable((file, node), 0)
                .map(|_| ())
                .map_err(|reason| {
                    if reason == "callable_collision" {
                        reason
                    } else {
                        "hidden_literal_key"
                    }
                }),
        }
    }

    pub(super) fn object(
        &self,
        (file, node): Expr<'a>,
        depth: usize,
    ) -> Check<(Vec<(String, Node<'a>)>, bool)> {
        let parsed = &self.context.files[file];
        let mut entries = Vec::new();
        let mut null_proto = false;
        let mut status = Ok(());
        for entry in children(node) {
            let data = (|| {
                let (key, value) = match entry.kind() {
                    "pair" => (
                        entry
                            .child_by_field_name("key")
                            .ok_or("hidden_literal_key")?,
                        entry
                            .child_by_field_name("value")
                            .ok_or("hidden_literal_key")?,
                    ),
                    "shorthand_property_identifier" => (entry, entry),
                    "method_definition" if accessor(entry) => return Err("undefined_lookup"),
                    "method_definition" if ordinary_method(entry) => (
                        entry
                            .child_by_field_name("name")
                            .ok_or("hidden_literal_key")?,
                        entry,
                    ),
                    _ => return Err("hidden_literal_key"),
                };
                let spelling = property_key(parsed, key).ok_or("hidden_literal_key")?;
                if entry.kind() == "pair"
                    && spelling == "__proto__"
                    && key.kind() != "computed_property_name"
                {
                    if null_proto || peel(value).is_none_or(|node| node.kind() != "null") {
                        return Err("undefined_lookup");
                    }
                    null_proto = true;
                } else {
                    self.closed((file, value), depth + 1)?;
                    entries.push((spelling, value));
                }
                Ok(())
            })();
            if let Err(reason) = data {
                status = Err(status.err().map_or(reason, |prior| first(prior, reason)));
            }
        }
        status.map(|()| (entries, null_proto))
    }

    pub(super) fn default_dependency(&self, file: &'a str, default: Node<'a>) -> Check<()> {
        let parsed = &self.context.files[file];
        let mut cache = self.context.bindings[file].cache();
        if descendants(default).any(|node| {
            name(parsed, node).is_some()
                && evaluated_in(parsed, default, node)
                && !parsed.js_ts_identifier_is_not_a_read(node)
                && param_slot::parameter(parsed, node, &mut cache)
                    .is_some_and(|p| p.body_read.is_none())
        }) {
            return Err("default_dependency");
        }
        Ok(())
    }

    pub(super) fn dynamic(
        &self,
        file: &'a str,
        scope: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let parsed = &self.context.files[file];
        if ancestors(scope).any(|node| node.kind() == "with_statement")
            || (parsed.node_text(&scope).contains("with")
                && descendants(scope).any(|node| node.kind() == "with_statement"))
        {
            return true;
        }
        self.context.bindings[file]
            .eval_sites
            .get_or_init(|| identity::index_eval_sites(parsed))
            .get(&scope.id())
            .into_iter()
            .flatten()
            .any(|&(_, callee)| !self.primitive_eval(parsed, callee, cache))
    }

    pub(super) fn callable(
        &self,
        expr: Expr<'a>,
        links: usize,
    ) -> Check<(FunctionId, Span, &'static str)> {
        let (file, node) = normalize(Some(expr)).ok_or("opaque_terminal")?;
        let parsed = &self.context.files[file];
        if ancestors(node).any(|node| node.kind() == "with_statement") {
            return Err("dynamic_binding");
        }
        if matches!(
            node.kind(),
            "arrow_function" | "function_expression" | "generator_function"
        ) || (node.kind() == "method_definition" && ordinary_method(node))
        {
            return self.witness(file, node, "F1");
        }
        let mut cache = self.context.bindings[file].cache();
        if let Some((object, member)) =
            static_member(parsed, node).filter(|_| !has_token(node, "optional_chain"))
        {
            let Some((JsBinding::Import, Some(module))) =
                parsed.js_ts_namespace_binding(object, &mut cache)
            else {
                return Err("opaque_terminal");
            };
            let scope = parsed.tree.root_node();
            if self.dynamic_lookup(file, scope, parsed.node_text(&object), &mut cache) {
                return Err("dynamic_binding");
            }
            drop(cache);
            return self.import(file, &module, &member);
        }
        let spelling = name(parsed, node).ok_or("opaque_terminal")?;
        let (scope, decl) = parsed
            .js_ts_site_declaration(node, &spelling, &mut cache)
            .ok_or("stale_terminal")?;
        if self.dynamic_lookup(file, scope, &spelling, &mut cache) {
            return Err("dynamic_binding");
        }
        if parsed.js_ts_scoped_written(scope, &spelling, &mut cache) {
            return Err("stale_terminal");
        }
        match decl.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if !hoisted(parsed, scope, decl) {
                    return Err("stale_terminal");
                }
                self.witness(file, decl, "F2")
            }
            "variable_declarator"
                if links < 8
                    && param_slot::is_const(decl)
                    && const_before(parsed, decl, node)
                    && decl
                        .child_by_field_name("name")
                        .is_some_and(|name| name.kind() == "identifier") =>
            {
                let initializer = decl.child_by_field_name("value").ok_or("stale_terminal")?;
                drop(cache);
                let (id, span, _) = self.callable((file, initializer), links + 1)?;
                Ok((id, span, "F3"))
            }
            "import_statement" => {
                let (module, member) =
                    named_import(parsed, decl, &spelling).ok_or("stale_terminal")?;
                drop(cache);
                self.import(file, &module, &member)
            }
            _ => Err("stale_terminal"),
        }
    }

    fn import(
        &self,
        file: &str,
        module: &str,
        member: &str,
    ) -> Check<(FunctionId, Span, &'static str)> {
        if !module.starts_with("./") && !module.starts_with("../") {
            return Err("stale_terminal");
        }
        let graph = self.context.cg;
        let module_file = graph
            .resolve_js_ts_member_module(module, file)
            .ok_or("stale_terminal")?;
        let export = graph
            .js_ts_resolved_exports
            .get(&module_file)
            .and_then(|exports| exports.get(member))
            .filter(|export| {
                export.file == module_file
                    && export.local_name == member
                    && !export.wrapped
                    && !export.via_unresolved_star
            })
            .ok_or("stale_terminal")?;
        let (file, parsed) = self
            .context
            .files
            .get_key_value(&export.file)
            .ok_or("stale_terminal")?;
        let scope = parsed.tree.root_node();
        let mut exports = children(scope).filter_map(|statement| {
            (statement.kind() == "export_statement" && !has_token(statement, "default"))
                .then(|| statement.child_by_field_name("declaration"))
                .flatten()
                .filter(|node| {
                    hoisted(parsed, scope, *node)
                        && node
                            .child_by_field_name("name")
                            .and_then(|name| param_slot::name(parsed, name))
                            .as_deref()
                            == Some(member)
                })
        });
        let decl = exports
            .next()
            .filter(|_| exports.next().is_none())
            .ok_or("stale_terminal")?;
        let mut cache = self.context.bindings[file.as_str()].cache();
        let (declarations, annex) = parsed.js_ts_scope_lookup(scope, member, &mut cache);
        if self.dynamic_lookup(file, scope, member, &mut cache) {
            return Err("dynamic_binding");
        }
        if annex
            || declarations.as_deref() != Some(&[decl])
            || parsed.js_ts_scoped_written(scope, member, &mut cache)
        {
            return Err("stale_terminal");
        }
        self.witness(file, decl, "F4")
    }

    fn witness(
        &self,
        file: &str,
        node: Node<'a>,
        form: &'static str,
    ) -> Check<(FunctionId, Span, &'static str)> {
        if node.has_error() {
            return Err("stale_terminal");
        }
        let id = crate::anon_owner::identity(&self.context.files[file], &node, file)
            .ok_or("stale_terminal")?;
        if !self.context.unique_owner(&id, node) {
            return Err("callable_collision");
        }
        Ok((id, bytes(node), form))
    }
}

pub(super) fn normalize(expr: Option<Expr<'_>>) -> Option<Expr<'_>> {
    let (file, node) = expr?;
    peel(node)
        .filter(|node| !node.has_error())
        .map(|node| (file, node))
}

fn primitive(node: Node<'_>) -> bool {
    matches!(node.kind(), "null" | "true" | "false" | "string" | "number")
}

pub(super) fn has_token(node: Node<'_>, kind: &str) -> bool {
    node.children(&mut node.walk())
        .any(|child| child.kind() == kind)
}

fn accessor(node: Node<'_>) -> bool {
    has_token(node, "get") || has_token(node, "set")
}

fn ordinary_method(node: Node<'_>) -> bool {
    node.parent()
        .is_some_and(|parent| parent.kind() == "object")
        && !accessor(node)
        && !has_token(node, "async")
        && !has_token(node, "*")
}

pub(super) fn ordinary_declaration(node: Node<'_>) -> bool {
    node.kind() == "function_declaration" && !has_token(node, "async")
}

pub(super) fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(Some(node), Node::parent)
}

pub(super) fn descendants(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut pending = vec![node];
    std::iter::from_fn(move || {
        let node = pending.pop()?;
        pending.extend(children(node).rev());
        Some(node)
    })
}

pub(super) fn outer(node: Node<'_>) -> Node<'_> {
    ancestors(node)
        .skip(1)
        .take_while(|parent| peel(*parent) == Some(node))
        .last()
        .unwrap_or(node)
}

pub(super) fn canonical_scope(node: Node<'_>) -> Node<'_> {
    if node.kind() == "formal_parameters" {
        node.parent().unwrap_or(node)
    } else {
        node
    }
}

pub(super) fn formal_list(owner: Node<'_>) -> Check<Vec<(Node<'_>, Option<Node<'_>>)>> {
    let list = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"))
        .filter(|list| !list.has_error())
        .ok_or("path_step")?;
    let entries = if list.kind() == "formal_parameters" {
        children(list).collect()
    } else {
        vec![list]
    };
    let mut result = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        let (binding, default) = formal_binding(entry).ok_or("path_step")?;
        if matches!(entry.kind(), "required_parameter" | "optional_parameter")
            && children(entry).any(|part| {
                part != binding && Some(part) != default && part.kind() != "type_annotation"
            })
        {
            return Err("path_step");
        }
        if binding.kind() == "this" {
            if index != 0 || default.is_some() || entry.child_by_field_name("type").is_none() {
                return Err("path_step");
            }
        } else {
            result.push((binding, default));
        }
    }
    Ok(result)
}

pub(super) fn pattern<'a>(
    parsed: &ParsedFile,
    binding: Node<'a>,
    depth: usize,
    path: &mut Vec<(String, Option<Node<'a>>)>,
) -> Check<Node<'a>> {
    if name(parsed, binding).is_some() {
        return Ok(binding);
    }
    if binding.kind() != "object_pattern" || depth == 2 {
        return Err("path_step");
    }
    let mut entries = children(binding);
    let entry = entries
        .next()
        .filter(|_| entries.next().is_none())
        .ok_or("path_step")?;
    let (key, mut leaf) = match entry.kind() {
        "pair_pattern" => (
            entry.child_by_field_name("key").ok_or("path_step")?,
            entry.child_by_field_name("value").ok_or("path_step")?,
        ),
        "object_assignment_pattern" => {
            (entry.child_by_field_name("left").ok_or("path_step")?, entry)
        }
        "shorthand_property_identifier_pattern" => (entry, entry),
        _ => return Err("path_step"),
    };
    let mut default = None;
    if matches!(
        leaf.kind(),
        "assignment_pattern" | "object_assignment_pattern"
    ) {
        default = Some(leaf.child_by_field_name("right").ok_or("path_step")?);
        leaf = leaf.child_by_field_name("left").ok_or("path_step")?;
    }
    path.push((property_key(parsed, key).ok_or("path_step")?, default));
    pattern(parsed, leaf, depth + 1, path)
}

pub(super) fn evaluated_in(parsed: &ParsedFile, root: Node<'_>, node: Node<'_>) -> bool {
    if node == root {
        return true;
    }
    let mut child = node;
    for parent in ancestors(node).skip(1) {
        if parsed
            .language
            .callable_boundary_node_types()
            .contains(&parent.kind())
            && !(parent.kind() == "method_definition"
                && parent.child_by_field_name("name") == Some(child))
        {
            return false;
        }
        if parent == root {
            return true;
        }
        child = parent;
    }
    false
}

fn hoisted(parsed: &ParsedFile, scope: Node<'_>, declaration: Node<'_>) -> bool {
    if !matches!(
        declaration.kind(),
        "function_declaration" | "generator_function_declaration"
    ) {
        return false;
    }
    let Some(parent) = declaration.parent() else {
        return false;
    };
    let list = if parent.kind() == "export_statement" {
        parent.parent()
    } else {
        Some(parent)
    };
    if scope.kind() == "program" {
        return parsed.js_ts_is_module() && list == Some(scope);
    }
    (list == Some(scope) && scope.kind() == "statement_block")
        || scope.child_by_field_name("body") == list
}

fn const_before(parsed: &ParsedFile, decl: Node<'_>, read: Node<'_>) -> bool {
    let Some(statement) = decl.parent() else {
        return false;
    };
    let statement = statement
        .parent()
        .filter(|node| node.kind() == "export_statement")
        .unwrap_or(statement);
    let Some(list) = statement
        .parent()
        .filter(|node| matches!(node.kind(), "program" | "statement_block"))
    else {
        return false;
    };
    identity::initialized_at(parsed, decl, read)
        && ancestors(read)
            .take_while(|node| *node != list)
            .all(|node| {
                !matches!(
                    node.kind(),
                    "statement_block"
                        | "if_statement"
                        | "switch_statement"
                        | "for_statement"
                        | "for_in_statement"
                        | "while_statement"
                        | "do_statement"
                        | "try_statement"
                        | "with_statement"
                        | "class_body"
                )
            })
}

fn named_import(parsed: &ParsedFile, stmt: Node<'_>, local: &str) -> Option<(String, String)> {
    if has_token(stmt, "type") {
        return None;
    }
    let mut specifiers = descendants(stmt)
        .filter(|node| node.kind() == "import_specifier" && !has_token(*node, "type"))
        .filter_map(|spec| {
            let original = spec.child_by_field_name("name")?;
            let binding = spec.child_by_field_name("alias").unwrap_or(original);
            (name(parsed, binding).as_deref() == Some(local))
                .then(|| property_key(parsed, original))
                .flatten()
        });
    let member = specifiers.next().filter(|_| specifiers.next().is_none())?;
    Some((
        property_key(parsed, stmt.child_by_field_name("source")?)?,
        member,
    ))
}
