//! Bounded evidence about a spread operand's iteration at this use.

use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use std::ops::Range;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Yield {
    Empty,
    Nonempty,
    Unknown,
}

pub(super) fn at_spread(
    parsed: &ParsedFile,
    arguments: &[(String, Range<usize>)],
    span: &Range<usize>,
    operand: Node<'_>,
) -> Yield {
    let proof = || {
        let spread = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(span.start, span.end)?;
        let call = spread
            .parent()
            .filter(|node| node.kind() == "arguments")?
            .parent()?;
        let block_statement = call_statement(parsed, call)?;
        let block = block_statement.parent()?;
        let mut budget = 96;
        let program = parsed.tree.root_node();
        if block != program {
            let owner = block.parent()?;
            if !parsed
                .language
                .callable_boundary_node_types()
                .contains(&owner.kind())
                || owner.child_by_field_name("body") != Some(block)
            {
                return None;
            }
            // A visible module initializer can replace a standard iterator
            // before the local allocation, even if it never sees the value.
            literal_prefix(parsed, program, None, &mut budget)?;
        }
        let values = literal_prefix(parsed, block, Some(block_statement), &mut budget)?;
        for (_, earlier) in arguments
            .iter()
            .take_while(|(_, arg)| arg.start < span.start)
        {
            let node = parsed
                .tree
                .root_node()
                .descendant_for_byte_range(earlier.start, earlier.end)?;
            if !inert_expression(parsed, node, &mut budget) {
                return None;
            }
        }
        Some(value_yield(parsed, operand, &values))
    };
    proof().unwrap_or(Yield::Unknown)
}

fn call_statement<'a>(parsed: &ParsedFile, call: Node<'a>) -> Option<Node<'a>> {
    let callee = call
        .child_by_field_name("function")
        .or_else(|| call.child_by_field_name("constructor"))?;
    // A call result, computed receiver or computed key may execute user code
    // before argument evaluation starts.
    match callee.kind() {
        "identifier" => {}
        "member_expression"
            if callee
                .child_by_field_name("object")
                .is_some_and(|obj| matches!(obj.kind(), "identifier" | "this")) => {}
        _ => return None,
    }
    let mut statement = call.parent()?;
    if statement.kind() == "variable_declarator"
        && statement.child_by_field_name("value") == Some(call)
        && statement
            .child_by_field_name("name")
            .is_some_and(|name| name.kind() == "identifier")
    {
        let list = statement.parent()?;
        let mut cursor = list.walk();
        if list
            .named_children(&mut cursor)
            .find(|node| !node.is_extra())?
            != statement
        {
            return None;
        }
        statement = list;
    } else if matches!(
        statement.kind(),
        "return_statement" | "expression_statement"
    ) {
        let mut cursor = statement.walk();
        if statement
            .named_children(&mut cursor)
            .find(|node| !node.is_extra())?
            != call
        {
            return None;
        }
    } else {
        return None;
    }
    let block = statement.parent()?;
    (matches!(block.kind(), "program" | "statement_block")
        && !block.has_error()
        && !parsed.node_text(&callee).contains('\\'))
    .then_some(statement)
}

/// Follow local literal initializers and copies along a single statement list.
/// A call, write, escape or branch on the prefix can invalidate contents or
/// iteration, so the proof stops at the first expression it cannot account for.
fn literal_prefix<'a>(
    parsed: &'a ParsedFile,
    block: Node<'a>,
    stop: Option<Node<'a>>,
    budget: &mut usize,
) -> Option<BTreeMap<&'a str, Yield>> {
    let mut cursor = block.walk();
    let mut values = BTreeMap::new();
    for node in block
        .named_children(&mut cursor)
        .filter(|node| !node.is_extra())
        .take_while(|node| Some(*node) != stop)
    {
        spend(budget)?;
        let node = if node.kind() == "export_statement" {
            node.child_by_field_name("declaration")?
        } else {
            node
        };
        match node.kind() {
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = node.walk();
                for decl in node.named_children(&mut cursor).filter(|c| !c.is_extra()) {
                    if decl.kind() != "variable_declarator" {
                        return None;
                    }
                    let name = decl
                        .child_by_field_name("name")
                        .filter(|n| n.kind() == "identifier")?;
                    let name = parsed.node_text(&name);
                    if name.contains('\\') || values.contains_key(name) {
                        return None;
                    }
                    let knowledge = if let Some(value) = decl.child_by_field_name("value") {
                        if !inert_expression(parsed, value, budget) {
                            return None;
                        }
                        value_yield(parsed, value, &values)
                    } else {
                        Yield::Unknown
                    };
                    values.insert(name, knowledge);
                }
            }
            "function_declaration"
            | "generator_function_declaration"
            | "import_statement"
            | "type_alias_declaration"
            | "interface_declaration"
            | "empty_statement" => {}
            "class_declaration" if inert_class(node) => {}
            "expression_statement" => {
                let mut cursor = node.walk();
                let value = node.named_children(&mut cursor).find(|c| !c.is_extra())?;
                if !inert_expression(parsed, value, budget) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(values)
}

fn inert_class(class: Node<'_>) -> bool {
    let mut cursor = class.walk();
    let inert = class
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra())
        .all(|child| match child.kind() {
            "identifier" | "type_identifier" | "type_parameters" => true,
            "class_body" => {
                let mut cursor = child.walk();
                let inert = child
                    .named_children(&mut cursor)
                    .filter(|member| !member.is_extra())
                    .all(|member| {
                        member.kind() == "method_definition"
                            && member
                                .child_by_field_name("name")
                                .is_some_and(|name| name.kind() != "computed_property_name")
                            && member.child_by_field_name("decorator").is_none()
                    });
                inert
            }
            _ => false,
        });
    inert
}

fn value_yield(parsed: &ParsedFile, value: Node<'_>, values: &BTreeMap<&str, Yield>) -> Yield {
    let Some(value) = parsed.js_ts_unwrap_value(value) else {
        return Yield::Unknown;
    };
    match value.kind() {
        "array" => match super::super::spread_args::dense_array_length(value) {
            Some(0) => Yield::Empty,
            Some(_) => Yield::Nonempty,
            None => Yield::Unknown,
        },
        "identifier" => values
            .get(parsed.node_text(&value))
            .copied()
            .unwrap_or(Yield::Unknown),
        "string" | "template_string" => literal_string_yield(parsed, value),
        _ => Yield::Unknown,
    }
}

fn literal_string_yield(parsed: &ParsedFile, literal: Node<'_>) -> Yield {
    let mut cursor = literal.walk();
    if literal
        .named_children(&mut cursor)
        .any(|node| node.kind() == "template_substitution")
    {
        return Yield::Unknown;
    }
    let text = parsed.node_text(&literal);
    let Some(contents) = text.get(1..text.len().saturating_sub(1)) else {
        return Yield::Unknown;
    };
    // Escaped line continuations contribute no code point. Every other
    // string character or escape contributes at least one.
    let mut rest = contents;
    while !rest.is_empty() {
        let Some(continuation) = rest.strip_prefix('\\').and_then(|escaped| {
            ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"]
                .into_iter()
                .find_map(|ending| escaped.strip_prefix(ending))
        }) else {
            return Yield::Nonempty;
        };
        rest = continuation;
    }
    Yield::Empty
}

fn inert_expression(parsed: &ParsedFile, value: Node<'_>, budget: &mut usize) -> bool {
    if spend(budget).is_none() {
        return false;
    }
    let Some(value) = parsed.js_ts_unwrap_value(value) else {
        return false;
    };
    match value.kind() {
        "number" | "true" | "false" | "null" | "string" | "this" => true,
        "identifier" => !parsed.node_text(&value).contains('\\'),
        "template_string" => literal_string_yield(parsed, value) != Yield::Unknown,
        "array" => {
            let mut cursor = value.walk();
            let inert = value
                .named_children(&mut cursor)
                .filter(|child| !child.is_extra())
                .all(|child| inert_expression(parsed, child, budget));
            inert
        }
        "spread_element" => {
            // A preceding dense literal contributes its A length via standard iteration.
            parsed
                .spread_argument_operand(&value.byte_range())
                .is_some_and(|array| {
                    super::super::spread_args::dense_array_length(array).is_some()
                        && inert_expression(parsed, array, budget)
                })
        }
        _ => false,
    }
}

fn spend(budget: &mut usize) -> Option<()> {
    *budget = budget.checked_sub(1)?;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    fn classify(source: &str, language: Language) -> Yield {
        let parsed = ParsedFile::parse("fixture", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{source}");
        let start = source.rfind("take(").unwrap();
        let end = source.rfind(')').unwrap() + 1;
        let args = parsed.call_argument_texts_and_spans_of_call(start, end, "take");
        let (_, span) = args.last().unwrap();
        let operand = parsed.spread_argument_operand(span).unwrap();
        at_spread(&parsed, &args, span, operand)
    }

    #[test]
    fn rest_yield_direct_and_wrapped_literal_pairs() {
        for (operand, yield_) in [
            ("[]", Yield::Empty),
            ("[1]", Yield::Nonempty),
            ("''", Yield::Empty),
            ("'a'", Yield::Nonempty),
            ("``", Yield::Empty),
            ("`a`", Yield::Nonempty),
            ("'\\\n'", Yield::Empty),
            ("(([] as const)!)", Yield::Empty),
            ("(([1] as const)!)", Yield::Nonempty),
            ("(('' as string)!)", Yield::Empty),
            ("((`a` as string)!)", Yield::Nonempty),
        ] {
            let source = format!("take(...{operand});");
            assert_eq!(classify(&source, Language::TypeScript), yield_, "{source}");
        }
    }

    #[test]
    fn rest_yield_requires_intact_module_and_argument_prefixes() {
        for source in [
            "Array.prototype[Symbol.iterator] = function* () {};\nfunction main() { const xs = [1]; take(...xs); }",
            "function main() { const xs = [1]; take(escape(xs), ...xs); }",
            "function main() { const xs = [1]; if (ready) { take(...xs); } }",
        ] {
            assert_eq!(classify(source, Language::JavaScript), Yield::Unknown, "{source}");
        }
        assert_eq!(
            classify(
                "function main() { const xs = [1]; take(0, ...xs); }",
                Language::JavaScript
            ),
            Yield::Nonempty
        );
    }
}
