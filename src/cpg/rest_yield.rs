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
            let mut enclosing = Some(owner);
            while let Some(node) = enclosing {
                if parsed
                    .language
                    .callable_boundary_node_types()
                    .contains(&node.kind())
                {
                    if !inert_parameters(parsed, node, &mut budget, true) {
                        return None;
                    }
                    // The owner's own block is checked up to the call below; an
                    // enclosing callable can invoke the owner from any of its
                    // statements, so its whole body must be inert.
                    if node != owner {
                        let body = node.child_by_field_name("body")?;
                        if body.kind() == "statement_block" {
                            literal_prefix(parsed, body, None, &mut budget)?;
                        } else if !inert_expression(parsed, body, &mut budget) {
                            return None;
                        }
                    }
                }
                enclosing = node.parent();
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
            "class_declaration" if inert_class(parsed, node, budget) => {}
            "return_statement" => {
                let mut cursor = node.walk();
                let value = node.named_children(&mut cursor).find(|c| !c.is_extra());
                if value.is_some_and(|value| !inert_expression(parsed, value, budget)) {
                    return None;
                }
            }
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

fn inert_class(parsed: &ParsedFile, class: Node<'_>, budget: &mut usize) -> bool {
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
                            && inert_parameters(parsed, member, budget, false)
                    });
                inert
            }
            _ => false,
        });
    inert
}

// Parameter defaults may run before the call body; decorators run at class definition.
fn inert_parameters(
    parsed: &ParsedFile,
    callable: Node<'_>,
    budget: &mut usize,
    defaults: bool,
) -> bool {
    // A computed method name is evaluated when the enclosing class is defined.
    if defaults
        && callable.kind() == "method_definition"
        && callable
            .child_by_field_name("name")
            .is_some_and(|name| name.kind() == "computed_property_name")
        && !inert_computed_name(parsed, callable.child_by_field_name("name"), budget)
    {
        return false;
    }
    let Some(params) = parsed.find_parameters_node(&callable) else {
        return true;
    };
    let mut stack = vec![params];
    while let Some(node) = stack.pop() {
        if spend(budget).is_none() {
            return false;
        }
        if node.kind() == "decorator" {
            return false;
        }
        // A computed key in a destructuring pattern runs at call time, before the body.
        if defaults && node.kind() == "computed_property_name" {
            if !inert_computed_name(parsed, Some(node), budget) {
                return false;
            }
            continue;
        }
        let initializer = match node.kind() {
            "assignment_pattern" | "object_assignment_pattern" => node.child_by_field_name("right"),
            "required_parameter" | "optional_parameter" => node.child_by_field_name("value"),
            _ => None,
        };
        if defaults && initializer.is_some_and(|value| !inert_expression(parsed, value, budget)) {
            return false;
        }
        let annotation = node.child_by_field_name("type");
        let mut cursor = node.walk();
        stack.extend(
            node.named_children(&mut cursor)
                .filter(|child| Some(*child) != initializer && Some(*child) != annotation),
        );
    }
    true
}

fn inert_computed_name(parsed: &ParsedFile, name: Option<Node<'_>>, budget: &mut usize) -> bool {
    let Some(name) = name else {
        return false;
    };
    let mut cursor = name.walk();
    let mut inner = name.named_children(&mut cursor).filter(|c| !c.is_extra());
    match (inner.next(), inner.next()) {
        (Some(expr), None) => inert_expression(parsed, expr, budget),
        _ => false,
    }
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

    /// Classify the spread at the last argument of `call`, written verbatim in `source`.
    fn classify_call(source: &str, call: &str, language: Language) -> Yield {
        let parsed = ParsedFile::parse("fixture", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{source}");
        let start = source.rfind(call).unwrap();
        let end = start + call.len();
        let args = parsed.call_argument_texts_and_spans_of_call(start, end, "take");
        let (_, span) = args.last().unwrap();
        let operand = parsed.spread_argument_operand(span).unwrap();
        at_spread(&parsed, &args, span, operand)
    }

    fn assert_pairs(call: &str, pairs: &[(&str, Yield)]) {
        for (body, want) in pairs {
            let source = format!("function main() {{ {body} {call}; }}");
            assert_eq!(
                classify_call(&source, call, Language::JavaScript),
                *want,
                "{source}"
            );
        }
    }

    #[test]
    fn rest_yield_call_statement_forms() {
        // Positive control first, then each refusal of the statement shape.
        for (source, call, want) in [
            (
                "function main() { const xs = [1]; const ys = take(...xs); }",
                "take(...xs)",
                Yield::Nonempty,
            ),
            (
                "function main() { const xs = [1]; return this.take(...xs); }",
                "this.take(...xs)",
                Yield::Nonempty,
            ),
            // The call is not the first declarator of its list.
            (
                "function main() { const xs = [1]; const a = 1, ys = take(...xs); }",
                "take(...xs)",
                Yield::Unknown,
            ),
            // A pattern declarator is not a plain call statement.
            (
                "function main() { const xs = [1]; const [a] = take(...xs); }",
                "take(...xs)",
                Yield::Unknown,
            ),
            // An unsupported statement directly inside the accepted block.
            (
                "function main() { const xs = [1]; throw take(...xs); }",
                "take(...xs)",
                Yield::Unknown,
            ),
            // A call result or computed receiver can run user code first.
            (
                "function main() { const xs = [1]; get().take(...xs); }",
                "get().take(...xs)",
                Yield::Unknown,
            ),
            (
                "function main() { const xs = [1]; a.b.take(...xs); }",
                "a.b.take(...xs)",
                Yield::Unknown,
            ),
        ] {
            assert_eq!(
                classify_call(source, call, Language::JavaScript),
                want,
                "{source}"
            );
        }
    }

    #[test]
    fn rest_yield_literal_prefix_declarations() {
        assert_pairs(
            "take(...xs)",
            &[
                ("let ys; const xs = [1];", Yield::Nonempty),
                ("let xs;", Yield::Unknown),
                ("var xs = [1]; var xs = [2];", Yield::Unknown),
                ("var xs = [1];", Yield::Nonempty),
                ("const [a] = [1]; const xs = [1];", Yield::Unknown),
                ("const xs = [...[1]];", Yield::Unknown),
                ("const xs = [1, , 2];", Yield::Unknown),
            ],
        );
    }

    #[test]
    fn rest_yield_literal_prefix_statements() {
        assert_pairs(
            "take(...xs)",
            &[
                ("const xs = [1]; return 0;", Yield::Nonempty),
                ("const xs = [1]; return;", Yield::Nonempty),
                ("const xs = [1]; return escape();", Yield::Unknown),
                ("const xs = [1]; xs;", Yield::Nonempty),
                ("const xs = [1]; escape(xs);", Yield::Unknown),
                ("const xs = [1]; class A { m() {} }", Yield::Nonempty),
                ("const xs = [1]; class A extends B {}", Yield::Unknown),
                ("const xs = [1]; class A { static m() {} }", Yield::Nonempty),
                ("const xs = [1]; class A { f = 1; }", Yield::Unknown),
            ],
        );
    }

    #[test]
    fn rest_yield_earlier_spread_arguments() {
        assert_eq!(
            classify_call(
                "function main() { const xs = [1]; take(...[1], ...xs); }",
                "take(...[1], ...xs)",
                Language::JavaScript
            ),
            Yield::Nonempty
        );
        for call in ["take(...[escape()], ...xs)", "take(...ys, ...xs)"] {
            let source = format!("function main() {{ const xs = [1]; {call}; }}");
            assert_eq!(
                classify_call(&source, call, Language::JavaScript),
                Yield::Unknown,
                "{source}"
            );
        }
    }

    #[test]
    fn rest_yield_template_substitution_operand_is_unknown() {
        for (operand, want) in [("`${a}`", Yield::Unknown), ("`a`", Yield::Nonempty)] {
            let source = format!("take(...{operand});");
            assert_eq!(classify(&source, Language::JavaScript), want, "{source}");
        }
    }

    #[test]
    fn rest_yield_budget_bounds_parameters_and_values() {
        let params = |n: usize| {
            (0..n)
                .map(|i| format!("p{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        for (n, want) in [(3, true), (100, false)] {
            let source = format!("function main({}) {{}}", params(n));
            let parsed = ParsedFile::parse("fixture", &source, Language::JavaScript).unwrap();
            let function = parsed.all_functions()[0];
            assert_eq!(
                inert_parameters(&parsed, function, &mut 96, true),
                want,
                "{n}"
            );
        }
        for (n, want) in [(10, Yield::Nonempty), (120, Yield::Unknown)] {
            let zeros = vec!["0"; n].join(", ");
            let source =
                format!("function main() {{ const xs = [1]; const ys = [{zeros}]; take(...xs); }}");
            assert_eq!(classify(&source, Language::JavaScript), want, "{n}");
        }
    }

    #[test]
    fn rest_yield_computed_method_parameter_guard() {
        for (key, want) in [("[side()]", false), ("[`m`]", true), ("['m']", true)] {
            let source = format!("class C {{ {key}() {{}} }}");
            let parsed = ParsedFile::parse("fixture", &source, Language::JavaScript).unwrap();
            let method = parsed
                .all_functions()
                .into_iter()
                .find(|n| n.kind() == "method_definition")
                .unwrap();
            assert_eq!(
                inert_parameters(&parsed, method, &mut 96, true),
                want,
                "{key}"
            );
        }
    }

    #[test]
    fn rest_yield_enclosing_expression_body_guard_decides_a_decorated_class() {
        // The decorator sits on the export wrapper, so the module prefix sees only an empty class and
        // the enclosing arrow's expression body is the only check that reaches the method.
        let source =
            "@register(() => ({ main() { const xs = [1]; take(...xs); } })) export class C {}";
        assert_eq!(
            classify_call(source, "take(...xs)", Language::JavaScript),
            Yield::Unknown
        );
        let inert = "function main() { const xs = [1]; take(...xs); } export class C {}";
        assert_eq!(
            classify_call(inert, "take(...xs)", Language::JavaScript),
            Yield::Nonempty
        );
    }

    #[test]
    fn rest_yield_statement_budget_counts_each_prefix_statement() {
        for (empties, want) in [(80, Yield::Nonempty), (200, Yield::Unknown)] {
            let source = format!("const xs = [1];{} take(...xs);", ";".repeat(empties));
            assert_eq!(
                classify_call(&source, "take(...xs)", Language::JavaScript),
                want,
                "{empties}"
            );
        }
    }

    #[test]
    fn rest_yield_refuses_an_unsupported_statement_before_the_call() {
        for (stmt, want) in [
            ("if (ready) escape(xs);", Yield::Unknown),
            ("const ys = 0;", Yield::Nonempty),
        ] {
            let source = format!("function main() {{ const xs = [1]; {stmt} take(...xs); }}");
            assert_eq!(
                classify_call(&source, "take(...xs)", Language::JavaScript),
                want,
                "{source}"
            );
        }
    }

    #[test]
    fn rest_yield_enclosing_expression_body_inertness() {
        for (body, want) in [("({ main() {} })", false), ("0", true)] {
            let source = format!("const outer = () => {body};");
            let parsed = ParsedFile::parse("fixture", &source, Language::JavaScript).unwrap();
            let arrow = parsed
                .all_functions()
                .into_iter()
                .find(|n| n.kind() == "arrow_function")
                .unwrap();
            let expression = arrow.child_by_field_name("body").unwrap();
            assert_eq!(
                inert_expression(&parsed, expression, &mut 96),
                want,
                "{body}"
            );
        }
    }

    #[test]
    fn rest_yield_enclosing_callables_must_be_inert() {
        for (source, want) in [
            // An enclosing arrow with an expression body can run the owner at will.
            (
                "const outer = () => ({ main() { const xs = [1]; take(...xs); } });",
                Yield::Unknown,
            ),
            (
                "function outer() { function main() { const xs = [1]; take(...xs); } return main; }",
                Yield::Nonempty,
            ),
            // A computed method name runs when the class is defined.
            (
                "class C { [side()]() { const xs = [1]; take(...xs); } }",
                Yield::Unknown,
            ),
            (
                "class C { ['m']() { const xs = [1]; take(...xs); } }",
                Yield::Unknown,
            ),
        ] {
            assert_eq!(
                classify_call(source, "take(...xs)", Language::JavaScript),
                want,
                "{source}"
            );
        }
    }
}
