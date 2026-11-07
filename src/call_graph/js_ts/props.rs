//! Bounded imported object-alias proof over the supplied source snapshot.
//! Not a TypeScript program resolver; ambient input blocks this route globally.
use crate::{
    ast::{JsTsReceiverBindingEvidence, ParsedFile},
    languages::Language,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedPropsProof {
    pub defining_file: String,
    pub alias_span: (usize, usize),
    pub class_reference_span: (usize, usize),
    pub class_name: String,
}
pub type ImportedPropsProofs = BTreeMap<(String, usize, usize), ImportedPropsProof>;

fn visit(node: Node<'_>, f: &mut impl FnMut(Node<'_>)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, f);
    }
}

/// One named ESM specifier; reject other imports and local declaration collisions.
pub(crate) fn named_import(parsed: &ParsedFile, local: &str) -> Option<(String, String)> {
    let root = parsed.tree.root_node();
    if root.has_error() {
        return None;
    }
    let mut found = None;
    let mut blocked = false;
    let mut cursor = root.walk();
    for statement in root.named_children(&mut cursor) {
        if statement.kind() == "import_statement" {
            visit(statement, &mut |node| {
                if node.kind() == "import_specifier" {
                    let Some(name) = node.child_by_field_name("name") else {
                        return;
                    };
                    let binding = node.child_by_field_name("alias").unwrap_or(name);
                    if parsed.node_text(&binding) != local {
                        return;
                    }
                    let module = statement
                        .child_by_field_name("source")
                        .map(|n| parsed.node_text(&n).trim_matches(['\'', '"']).to_string());
                    let Some(module) = module else {
                        blocked = true;
                        return;
                    };
                    if name.kind() != "identifier"
                        || found
                            .replace((module, parsed.node_text(&name).to_string()))
                            .is_some()
                    {
                        blocked = true;
                    }
                } else if node.kind() == "identifier"
                    && parsed.node_text(&node) == local
                    && node
                        .parent()
                        .is_some_and(|p| matches!(p.kind(), "import_clause" | "namespace_import"))
                {
                    blocked = true;
                }
            });
        } else {
            // Only module-level declarations collide with the import; nested
            // lexical shadows are fenced by the original type-use AST position.
            let declaration = statement
                .child_by_field_name("declaration")
                .unwrap_or(statement);
            if declaration
                .child_by_field_name("name")
                .is_some_and(|n| parsed.node_text(&n) == local)
            {
                blocked = true;
            }
            if matches!(
                declaration.kind(),
                "lexical_declaration" | "variable_declaration"
            ) {
                let mut cursor = declaration.walk();
                if declaration.named_children(&mut cursor).any(|n| {
                    n.child_by_field_name("name")
                        .is_some_and(|n| parsed.node_text(&n) == local)
                }) {
                    blocked = true;
                }
            }
        }
    }
    if blocked {
        None
    } else {
        found
    }
}

/// Complete-map recomputation, also on direct-subset builds. Cache merge must
/// replace this map, so augmentation addition/removal changes retained callers.
pub(crate) fn collect(files: &BTreeMap<String, ParsedFile>) -> ImportedPropsProofs {
    let mut proofs = BTreeMap::new();
    for parsed in files.values().filter(|p| {
        matches!(
            p.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        )
    }) {
        let mut blocked = parsed.tree.root_node().has_error();
        visit(parsed.tree.root_node(), &mut |n| {
            blocked |= n.kind() == "ambient_declaration";
            // No prototype/reflective mutation analysis in this first slice.
            // Even reads of prototype are fenced (they can escape an alias).
            if matches!(n.kind(), "member_expression" | "subscript_expression") {
                let member = n
                    .child_by_field_name("property")
                    .or_else(|| n.child_by_field_name("index"));
                let name = member.map(|p| parsed.node_text(&p).trim_matches(['\'', '"']));
                blocked |= name == Some("prototype");
                let object = n
                    .child_by_field_name("object")
                    .map(|p| parsed.node_text(&p));
                blocked |= matches!(object, Some("Object" | "Reflect"))
                    && matches!(
                        name,
                        Some(
                            "assign"
                                | "defineProperty"
                                | "defineProperties"
                                | "set"
                                | "deleteProperty"
                                | "setPrototypeOf"
                        )
                    );
            }
        });
        if blocked {
            return proofs;
        }
    }
    let indexed: BTreeSet<_> = files.keys().cloned().collect();
    for (file, parsed) in files
        .iter()
        .filter(|(_, p)| matches!(p.language, Language::TypeScript | Language::Tsx))
    {
        let root = parsed.tree.root_node();
        visit(root, &mut |call| {
            let prove = || {
                if call.kind() != "call_expression" {
                    return None;
                }
                let callee = call.child_by_field_name("function")?;
                if callee.kind() != "member_expression" {
                    return None;
                }
                let receiver = callee.child_by_field_name("object")?;
                let JsTsReceiverBindingEvidence::ImportedProps {
                    type_name,
                    property,
                    ..
                } = parsed.js_ts_receiver_binding_evidence_at_call(&root, Some(receiver))?
                else {
                    return None;
                };
                let (module, member) = named_import(parsed, &type_name)?;
                let candidates =
                    crate::call_graph::js_ts_relative_module_candidates(&module, file, &indexed)?;
                if candidates.len() != 1 {
                    return None;
                }
                let defining_file = &candidates[0];
                let target = files.get(defining_file)?;
                if !matches!(target.language, Language::TypeScript | Language::Tsx) {
                    return None;
                }
                let (class_name, alias_span, class_reference_span) =
                    target.js_ts_exported_props_property(&member, &property)?;
                Some(ImportedPropsProof {
                    defining_file: defining_file.clone(),
                    alias_span,
                    class_reference_span,
                    class_name,
                })
            };
            if let Some(proof) = prove() {
                proofs.insert((file.clone(), call.start_byte(), call.end_byte()), proof);
            }
        });
    }
    proofs
}
