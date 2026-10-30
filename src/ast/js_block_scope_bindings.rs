//! Finite-set form of the child-scanning scope predicates. The immutable
//! session keys scope identity and boundary contents; callable-body status is
//! derived from those inputs. Byte-region admission remains in the caller.
use super::*;
use js_scope_bindings::{block_statement, declaration, pattern};
use std::collections::HashSet;

pub(super) fn binds(p: &ParsedFile, scope: Node<'_>, boundaries: &[&str], name: &str) -> bool {
    binding_verdict_memo::scope_binds(p, scope, boundaries, name, || collect(p, scope, boundaries))
}

fn collect(p: &ParsedFile, scope: Node<'_>, boundaries: &[&str]) -> HashSet<String> {
    #[cfg(test)]
    binding_verdict_memo::probe::observe("scope_names", scope.id(), 0, "");
    let mut names = HashSet::new();
    if boundaries.contains(&scope.kind()) {
        if let Some(params) = p
            .find_parameters_node(&scope)
            .or_else(|| scope.child_by_field_name("parameter"))
        {
            pattern(p, params, &mut names);
        }
        return names;
    }
    match scope.kind() {
        "statement_block" | "switch_body" => {
            let callable_body = scope.parent().is_some_and(|parent| {
                boundaries.contains(&parent.kind())
                    && parent.child_by_field_name("body").map(|b| b.id()) == Some(scope.id())
            });
            let mut cursor = scope.walk();
            for statement in scope.named_children(&mut cursor) {
                let mut unlabelled = statement;
                while unlabelled.kind() == "labeled_statement" {
                    let Some(body) = unlabelled.child_by_field_name("body") else {
                        break;
                    };
                    unlabelled = body;
                }
                if callable_body
                    && matches!(
                        unlabelled.kind(),
                        "function_declaration" | "generator_function_declaration"
                    )
                {
                    continue;
                }
                block_statement(p, statement, &mut names);
            }
        }
        "for_statement" => {
            if let Some(init) = scope
                .child_by_field_name("initializer")
                .filter(|init| init.kind() == "lexical_declaration")
            {
                declaration(p, init, &mut names);
            }
        }
        "for_in_statement" => {
            if scope
                .child_by_field_name("kind")
                .is_some_and(|k| matches!(p.node_text(&k), "let" | "const"))
            {
                if let Some(left) = scope.child_by_field_name("left") {
                    pattern(p, left, &mut names);
                }
            }
        }
        "catch_clause" => {
            if let Some(parameter) = scope.child_by_field_name("parameter") {
                pattern(p, parameter, &mut names);
            }
        }
        "enum_body" => enum_names(p, scope, &mut names),
        _ => {}
    }
    names
}

fn enum_members(p: &ParsedFile, body: Node<'_>, names: &mut HashSet<String>) {
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        let key = if member.kind() == "enum_assignment" {
            member.child_by_field_name("name")
        } else {
            Some(member)
        };
        if let Some(key) = key {
            let decoded = match key.kind() {
                "string" => p.js_ts_module_export_name(key),
                "identifier" | "property_identifier" | "type_identifier" => {
                    ParsedFile::js_ts_decoded_identifier(p.node_text(&key))
                }
                _ => None,
            };
            if let Some(decoded) = decoded {
                names.insert(decoded);
            }
        }
    }
}

fn enum_names(p: &ParsedFile, scope: Node<'_>, names: &mut HashSet<String>) {
    enum_members(p, scope, names);
    let Some(declaration) = scope.parent() else {
        return;
    };
    let enum_name = declaration.child_by_field_name("name");
    let Some(parent) = declaration.parent() else {
        return;
    };
    let mut cursor = parent.walk();
    for sibling in parent.named_children(&mut cursor) {
        if sibling.kind() == "enum_declaration"
            && sibling.child_by_field_name("name").is_some_and(|n| {
                enum_name.is_some_and(|own| {
                    ParsedFile::js_ts_decoded_identifier(p.node_text(&n))
                        == ParsedFile::js_ts_decoded_identifier(p.node_text(&own))
                })
            })
        {
            if let Some(body) = sibling.child_by_field_name("body") {
                enum_members(p, body, names);
            }
        }
    }
}
