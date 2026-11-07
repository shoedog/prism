//! JS/TS exact-read admission: straight-line callables, plain required parameters, local declarations.

use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use tree_sitter::Node;

impl ParsedFile {
    /// Whether an ordinary named JS/TS/TSX function contains only the bounded
    /// sequential statement subset used by exact-read expansion.
    pub(crate) fn exact_read_callable_is_straight_line(&self, func_node: &Node<'_>) -> bool {
        fn named(node: Node<'_>) -> Vec<Node<'_>> {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .filter(|child| child.kind() != "comment")
                .collect()
        }
        fn claimed(children: &[Node<'_>], fields: &[Option<Node<'_>>]) -> bool {
            children.len() == fields.iter().flatten().count()
                && children.iter().all(|child| {
                    fields
                        .iter()
                        .flatten()
                        .any(|field| child.id() == field.id())
                })
        }
        fn tokens(node: Node<'_>) -> bool {
            let mut cursor = node.walk();
            let valid = node
                .children(&mut cursor)
                .filter(|child| !child.is_named())
                .all(|child| {
                    matches!(
                        (node.kind(), child.kind()),
                        ("function_declaration", "function")
                            | ("formal_parameters", "(" | ")" | ",")
                            | ("statement_block", "{" | "}")
                            | ("empty_statement", ";")
                            | ("expression_statement", ";")
                            | ("lexical_declaration", "let" | "const" | "," | ";")
                            | ("variable_declaration", "var" | "," | ";")
                            | ("variable_declarator", "=")
                            | ("return_statement", "return" | ";")
                            | ("parenthesized_expression", "(" | ")")
                            | ("arguments", "(" | ")" | ",")
                            | ("member_expression", ".")
                            | ("assignment_expression", "=")
                            | ("binary_expression", "+")
                            | ("non_null_expression", "!")
                    )
                });
            valid
        }
        fn atom(parsed: &ParsedFile, node: Node<'_>, kind: &str) -> bool {
            node.kind() == kind
                && named(node).is_empty()
                && !parsed.node_text(&node).contains('\\')
                && parsed.node_text(&node) != "eval"
        }
        fn expression(parsed: &ParsedFile, node: Node<'_>) -> bool {
            if node.is_error()
                || node.is_missing()
                || !tokens(node)
                || !ParsedFile::exact_read_runtime_kind_is_supported(node.kind())
            {
                return false;
            }
            let children = named(node);
            match node.kind() {
                "identifier" => atom(parsed, node, "identifier"),
                "number" | "true" | "false" | "null" | "undefined" => children.is_empty(),
                "arguments" => children.into_iter().all(|child| expression(parsed, child)),
                "non_null_expression" => children.len() == 1 && expression(parsed, children[0]),
                "parenthesized_expression" => {
                    let ty = node.child_by_field_name("type");
                    let runtime: Vec<_> = children
                        .iter()
                        .copied()
                        .filter(|child| ty.is_none_or(|ty| child.id() != ty.id()))
                        .collect();
                    runtime.len() == 1
                        && claimed(&children, &[Some(runtime[0]), ty])
                        && expression(parsed, runtime[0])
                }
                "call_expression" => {
                    let (Some(function), Some(arguments)) = (
                        node.child_by_field_name("function"),
                        node.child_by_field_name("arguments"),
                    ) else {
                        return false;
                    };
                    let types = node.child_by_field_name("type_arguments");
                    matches!(function.kind(), "identifier" | "member_expression")
                        && arguments.kind() == "arguments"
                        && claimed(&children, &[Some(function), Some(arguments), types])
                        && expression(parsed, function)
                        && expression(parsed, arguments)
                }
                "member_expression" | "assignment_expression" | "binary_expression" => {
                    let names = if node.kind() == "member_expression" {
                        ("object", "property")
                    } else {
                        ("left", "right")
                    };
                    let (Some(left), Some(right)) = (
                        node.child_by_field_name(names.0),
                        node.child_by_field_name(names.1),
                    ) else {
                        return false;
                    };
                    claimed(&children, &[Some(left), Some(right)])
                        && expression(parsed, left)
                        && if node.kind() == "member_expression" {
                            atom(parsed, right, "property_identifier")
                        } else {
                            expression(parsed, right)
                                && (node.kind() == "binary_expression"
                                    || matches!(left.kind(), "identifier" | "member_expression"))
                        }
                }
                _ => false,
            }
        }
        fn statement(parsed: &ParsedFile, node: Node<'_>, terminal: bool) -> bool {
            if node.is_error() || node.is_missing() || !tokens(node) {
                return false;
            }
            let children = named(node);
            match node.kind() {
                "empty_statement" => children.is_empty(),
                "expression_statement" => children.len() == 1 && expression(parsed, children[0]),
                "return_statement" => {
                    terminal
                        && children.len() <= 1
                        && children.into_iter().all(|n| expression(parsed, n))
                }
                "lexical_declaration" | "variable_declaration" => {
                    !children.is_empty()
                        && children.into_iter().all(|declaration| {
                            if declaration.kind() != "variable_declarator" || !tokens(declaration) {
                                return false;
                            }
                            let (Some(name), value) = (
                                declaration.child_by_field_name("name"),
                                declaration.child_by_field_name("value"),
                            ) else {
                                return false;
                            };
                            let ty = declaration.child_by_field_name("type");
                            claimed(&named(declaration), &[Some(name), value, ty])
                                && atom(parsed, name, "identifier")
                                && value.is_none_or(|value| expression(parsed, value))
                        })
                }
                _ => false,
            }
        }

        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || func_node.kind() != "function_declaration"
            || func_node.has_error()
            || !tokens(*func_node)
        {
            return false;
        }
        let (Some(name), Some(parameters), Some(body)) = (
            func_node.child_by_field_name("name"),
            func_node.child_by_field_name("parameters"),
            func_node.child_by_field_name("body"),
        ) else {
            return false;
        };
        let root_fields = [
            Some(name),
            Some(parameters),
            Some(body),
            func_node.child_by_field_name("type_parameters"),
            func_node.child_by_field_name("return_type"),
        ];
        if !claimed(&named(*func_node), &root_fields)
            || !atom(self, name, "identifier")
            || parameters.kind() != "formal_parameters"
            || body.kind() != "statement_block"
            || !tokens(parameters)
            || !tokens(body)
        {
            return false;
        }
        let parameters_valid = named(parameters)
            .into_iter()
            .all(|parameter| match self.language {
                Language::JavaScript => atom(self, parameter, "identifier"),
                Language::TypeScript | Language::Tsx => {
                    let pattern = parameter.child_by_field_name("pattern");
                    let ty = parameter.child_by_field_name("type");
                    parameter.kind() == "required_parameter"
                        && tokens(parameter)
                        && parameter.child_by_field_name("value").is_none()
                        && pattern.is_some_and(|pattern| atom(self, pattern, "identifier"))
                        && claimed(&named(parameter), &[pattern, ty])
                }
                _ => false,
            });
        let statements = named(body);
        let last = statements
            .iter()
            .rposition(|node| node.kind() != "empty_statement");
        parameters_valid
            && statements
                .into_iter()
                .enumerate()
                .all(|(index, node)| statement(self, node, Some(index) == last))
    }

    pub(crate) fn exact_read_runtime_kind_is_supported(kind: &str) -> bool {
        matches!(
            kind,
            "identifier"
                | "number"
                | "true"
                | "false"
                | "null"
                | "undefined"
                | "parenthesized_expression"
                | "call_expression"
                | "arguments"
                | "member_expression"
                | "assignment_expression"
                | "binary_expression"
                | "non_null_expression"
        )
    }

    /// Whether the byte span is the token of a plain required parameter.
    /// Optional, defaulted, rest, destructured, and recovered wrappers remain
    /// outside the byte-distinct caller-read expansion.
    pub(crate) fn exact_read_is_plain_required_parameter(
        &self,
        func_node: &Node<'_>,
        span: &PathSpan,
    ) -> bool {
        let Some(node) = func_node
            .descendant_for_byte_range(span.start_byte, span.end_byte)
            .filter(|node| {
                node.kind() == "identifier"
                    && node.start_byte() == span.start_byte
                    && node.end_byte() == span.end_byte
            })
        else {
            return false;
        };
        let Some(parent) = node.parent() else {
            return false;
        };
        // js-param-defs PR-A: the bare arrow formal is a plain required
        // parameter; a rest formal's parent is `rest_pattern` and stays out.
        if matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) && self
            .js_ts_bare_arrow_parameter(&parent)
            .is_some_and(|param| param.id() == node.id())
        {
            return true;
        }
        match self.language {
            Language::JavaScript => parent.kind() == "formal_parameters",
            Language::TypeScript | Language::Tsx => {
                parent.kind() == "required_parameter"
                    && parent.child_by_field_name("value").is_none()
                    && parent
                        .child_by_field_name("pattern")
                        .is_some_and(|pattern| {
                            pattern.id() == node.id() && pattern.kind() == "identifier"
                        })
            }
            _ => false,
        }
    }

    /// Whether a real simple lvalue span is the binding token of a local
    /// declaration rather than an assignment/update target.
    pub(crate) fn exact_read_is_simple_local_declaration(
        &self,
        func_node: &Node<'_>,
        span: &PathSpan,
    ) -> bool {
        if !span.path.is_simple() || span.start_byte >= span.end_byte {
            return false;
        }
        let Some(mut node) = func_node.descendant_for_byte_range(span.start_byte, span.end_byte)
        else {
            return false;
        };
        loop {
            if self.language.is_assignment_node(node.kind()) || node.kind() == "update_expression" {
                return false;
            }
            if self.language.is_declaration_node(node.kind()) {
                return self.language.declaration_name(&node).is_some_and(|name| {
                    name.kind() == "identifier"
                        && (name.start_byte(), name.end_byte()) == (span.start_byte, span.end_byte)
                        && self.node_text(&name) == span.path.base
                });
            }
            if node.id() == func_node.id() {
                return false;
            }
            let Some(parent) = node.parent() else {
                return false;
            };
            node = parent;
        }
    }
}
