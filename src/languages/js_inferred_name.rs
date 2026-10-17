//! One classification for inferred function identity and negative lexical-binding facts.
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JsInferredNameSource {
    Declarator,
    Pair,
    WrappedDeclarator,
    MemberAssign,
    Field,
    IdentAssign,
}

pub(crate) struct JsInferredName<'a> {
    pub source: JsInferredNameSource,
    pub name: Node<'a>,
    owner: Node<'a>,
}

impl JsInferredName<'_> {
    pub(crate) fn is_property(&self, source: &str) -> bool {
        match self.source {
            JsInferredNameSource::Pair | JsInferredNameSource::Field => true,
            JsInferredNameSource::MemberAssign => !self.is_declarator_value(source),
            JsInferredNameSource::Declarator
            | JsInferredNameSource::WrappedDeclarator
            | JsInferredNameSource::IdentAssign => false,
        }
    }

    /// `var f = (obj.f = function() {})` holds this very function as lexical f.
    /// Only parentheses are transparent: calls, sequences and other wrappers
    /// need their own value proof and cannot receive this exemption.
    fn is_declarator_value(&self, source: &str) -> bool {
        let Some(mut value) = self.owner.parent() else {
            return false;
        };
        while let Some(parent) = value.parent() {
            if parent.kind() == "parenthesized_expression" {
                value = parent;
                continue;
            }
            return parent.kind() == "variable_declarator"
                && parent.child_by_field_name("value") == Some(value)
                && parent.child_by_field_name("name").is_some_and(|name| {
                    name.kind() == "identifier"
                        && source.get(name.byte_range()) == source.get(self.name.byte_range())
                });
        }
        false
    }
}

/// Mirrors the five existing function_name patterns without requiring source bytes.
/// Explicit function-expression names are lexical and never use inference.
pub(crate) fn js_inferred_name_source(node: Node<'_>) -> Option<JsInferredName<'_>> {
    use JsInferredNameSource::*;
    if !matches!(node.kind(), "arrow_function" | "function_expression")
        || node.child_by_field_name("name").is_some()
    {
        return None;
    }
    let parent = node.parent()?;
    let (source, name) = match parent.kind() {
        "variable_declarator" => (Declarator, parent.child_by_field_name("name")?),
        "pair" => (Pair, parent.child_by_field_name("key")?),
        "arguments" => {
            let call = parent.parent()?;
            let declarator = call.parent()?;
            if call.kind() != "call_expression" || declarator.kind() != "variable_declarator" {
                return None;
            }
            (WrappedDeclarator, declarator.child_by_field_name("name")?)
        }
        "public_field_definition" | "field_definition" => (
            Field,
            parent
                .child_by_field_name("property")
                .or_else(|| parent.child_by_field_name("name"))?,
        ),
        "assignment_expression" => {
            let left = parent.child_by_field_name("left")?;
            match left.kind() {
                "member_expression" => (MemberAssign, left.child_by_field_name("property")?),
                "identifier" => (IdentAssign, left),
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(JsInferredName {
        source,
        name,
        owner: node,
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::ParsedFile;
    use crate::call_graph::CallGraph;
    use crate::languages::Language;
    use crate::resolution::{ResolutionConfidence, ResolutionKind};
    use std::collections::BTreeMap;

    #[test]
    fn declarator_value_facts_control_bare_name_admission() {
        for ext in ["js", "ts", "tsx"] {
            for (source, property) in [
                (
                    "var f = (host.f = function() {}); function run(){ f(); }",
                    false,
                ),
                (
                    "var different = (host.f = function() {}); function run(){ f(); }",
                    true,
                ),
                (
                    "var f = wrap(host.f = function() {}); function run(){ f(); }",
                    true,
                ),
                (
                    "var f = (host.f = function() {}, other); function run(){ f(); }",
                    true,
                ),
            ] {
                let file = format!("a.{ext}");
                let parsed =
                    ParsedFile::parse(&file, source, Language::from_extension(ext).unwrap())
                        .unwrap();
                assert!(!parsed.tree.root_node().has_error());
                let owner = crate::anon_owner::candidate_nodes(&parsed, true)
                    .into_iter()
                    .find(|node| node.kind() == "function_expression")
                    .unwrap();
                let id = crate::anon_owner::identity(&parsed, &owner, &file).unwrap();
                let graph = CallGraph::build(&BTreeMap::from([(file, parsed)]));
                assert!(graph.functions.get("f").unwrap().contains(&id));
                let site = graph
                    .calls
                    .values()
                    .flatten()
                    .find(|s| s.callee_name == "f")
                    .unwrap();
                let full = graph.resolve_call_site_full(site);
                assert_eq!(
                    graph.js_anonymous.is_property_method(&id),
                    property,
                    "{ext}: {source}; full resolution: {full:?}"
                );
                let fallback = graph.bare_name_exact(vec![&id], ResolutionKind::LocalDef);
                assert_eq!(
                    fallback
                        .resolved
                        .iter()
                        .any(|r| r.confidence == ResolutionConfidence::Exact),
                    !property,
                    "{ext}: {source}"
                );
            }
        }
    }
}
