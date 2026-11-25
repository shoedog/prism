use crate::algorithms::taint::cleansing::node_in_ranges;
use crate::algorithms::taint::js_ts::source_ranges::js_ts_request_source_access_ranges_for_alias_on_line;
use prism::ast::ParsedFile;
use prism::data_flow::{FlowEdge, VarAccessKind, VarLocation};
use prism::languages::Language;
use std::iter::successors;
use tree_sitter::Node;

pub(super) fn exact_use<'a>(parsed: &'a ParsedFile, target: &VarLocation) -> Option<Node<'a>> {
    if target.file != parsed.path || target.kind != VarAccessKind::Use {
        return None;
    }
    let node = node_at(parsed, target)?;
    (node.start_position().row + 1 == target.line
        && !is_assignment_operand_target(parsed, node)
        && parsed.value_reference_path_at(node).as_ref() == Some(&target.path))
    .then_some(node)
}

fn is_assignment_operand_target(parsed: &ParsedFile, node: Node<'_>) -> bool {
    let language = parsed.language;
    successors(node.parent(), Node::parent)
        .take_while(|parent| !language.is_call_node(parent.kind()))
        .any(|parent| {
            language.is_assignment_node(parent.kind())
                && language
                    .assignment_target(&parent)
                    .is_some_and(|target| contains(target, node))
        })
}

pub(super) fn located_uses<'a>(
    parsed: &'a ParsedFile,
    line: usize,
    edge: &FlowEdge,
) -> Vec<Node<'a>> {
    let target = &edge.to;
    let Some(exact) = exact_use(parsed, target).filter(|_| target.line == line) else {
        return Vec::new();
    };
    let ranges =
        js_ts_request_source_access_ranges_for_alias_on_line(parsed, line, target.var_name());
    let boundaries = parsed.language.callable_boundary_node_types();
    let name = target.var_name();
    let bare = |node| is_bare_argument(parsed, node);

    if !matches!(
        parsed.language,
        Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Go
            | Language::Python
    ) {
        // The other grammars have no binding proof for redundant spellings.
        let root = parsed.tree.root_node();
        let competing = leaves_on_line(root, line).into_iter().any(|node| {
            parsed.node_text(&node) == name
                && !contains(exact, node)
                && !contains_span(node, &edge.from)
                && !parsed.language.is_comment_or_string_node(node.kind())
        });
        return if competing { Vec::new() } else { vec![exact] };
    }

    let Some(owner) = enclosing_callable(exact, &boundaries) else {
        return Vec::new();
    };
    let Some(def) = node_at(parsed, &edge.from).filter(|def| {
        edge.from.file == parsed.path
            && edge.from.kind == VarAccessKind::Def
            && edge.from.var_name() == name
            && enclosing_callable(*def, &boundaries) == Some(owner)
            && def.end_byte() <= exact.start_byte()
    }) else {
        return Vec::new();
    };
    let parameter =
        parsed
            .function_parameter_occurrences(&owner)
            .iter()
            .any(|(param, start, end)| {
                param == name && *start == def.start_byte() && *end == def.end_byte()
            });
    let declaration = def.parent().filter(|p| {
        p.kind() == "variable_declarator"
            && p.child_by_field_name("name") == Some(def)
            && p.child_by_field_name("value").is_some()
    });
    let immutable = declaration.and_then(|p| p.parent()).is_some_and(|decl| {
        decl.child_by_field_name("kind")
            .is_some_and(|kind| kind.kind() == "const")
    });

    let binding_matches = |read| {
        if enclosing_callable(read, &boundaries) != Some(owner) {
            return false;
        }
        match parsed.language {
            language if language.is_js_family() => {
                let Some(receiver) = parsed.value_reference_base_at(read) else {
                    return false;
                };
                let Some(def_site) = parsed.value_reference_base_at(def) else {
                    return false;
                };
                parsed.js_ts_same_binding(
                    receiver,
                    owner,
                    name,
                    def_site,
                    parameter || declaration.is_some(),
                )
            }
            Language::Go => {
                let short_decl = def.parent().and_then(|p| p.parent()).filter(|decl| {
                    decl.kind() == "short_var_declaration"
                        && decl.child_by_field_name("left").is_some_and(|left| {
                            left.named_child_count() == 1 && contains(left, def)
                        })
                });
                let scope = short_decl.and_then(|decl| decl.parent()).or_else(|| {
                    parameter
                        .then(|| owner.child_by_field_name("body"))
                        .flatten()
                });
                scope.is_some_and(|scope| {
                    successors(read.parent(), Node::parent).find(|n| {
                        matches!(
                            n.kind(),
                            "block"
                                | "statement_list"
                                | "for_statement"
                                | "if_statement"
                                | "expression_switch_statement"
                                | "type_switch_statement"
                                | "select_statement"
                        )
                    }) == Some(scope)
                })
            }
            Language::Python => {
                let assignment = def.parent().is_some_and(|node| {
                    node.kind() == "assignment"
                        && node.child_by_field_name("left") == Some(def)
                        && node.child_by_field_name("right").is_some()
                });
                (parameter || (assignment && python_function_scope(def, owner)))
                    && python_function_scope(read, owner)
            }
            _ => false,
        }
    };

    if owner.has_error()
        || !binding_matches(exact)
        || (!ranges.is_empty() && !node_in_ranges(&exact, &ranges))
    {
        return Vec::new();
    }
    // A mutable local can only be carried across calls if indirect writes are
    // excluded. A nested mention can capture it, Go can take its address, and
    // JS has direct eval and mapped arguments. Refuse those cases before reuse.
    if !local_value_isolated(parsed, owner, &boundaries, name, immutable) {
        return Vec::new();
    }
    let mentions: Vec<_> = leaves_on_line(owner, line)
        .into_iter()
        .filter(|node| {
            parsed.node_text(node) == name
                && !parsed.language.is_comment_or_string_node(node.kind())
        })
        .collect();
    let has_blocker = |start, end| {
        mentions.iter().copied().any(|node| {
            (start..end).contains(&node.start_byte()) && !(bare(node) && binding_matches(node))
        })
    };
    if has_blocker(def.end_byte(), exact.start_byte()) {
        return Vec::new();
    }
    if !target.path.fields.is_empty() || !ranges.is_empty() || !bare(exact) {
        return vec![exact];
    }
    let mut uses = vec![exact];
    uses.extend(mentions.iter().copied().filter(|&read| {
        read != exact
            && read.start_byte() >= def.end_byte()
            && bare(read)
            && binding_matches(read)
            && (immutable || same_or_adjacent_statement(exact, read))
            && !has_blocker(
                exact.start_byte().min(read.start_byte()),
                exact.start_byte().max(read.start_byte()),
            )
    }));
    uses
}

fn node_at<'a>(parsed: &'a ParsedFile, loc: &VarLocation) -> Option<Node<'a>> {
    if loc.start_byte >= loc.end_byte || loc.end_byte > parsed.source.len() {
        return None;
    }
    parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(loc.start_byte, loc.end_byte)
        .filter(|node| node.byte_range() == (loc.start_byte..loc.end_byte))
}

fn enclosing_callable<'a>(node: Node<'a>, boundaries: &[&str]) -> Option<Node<'a>> {
    successors(node.parent(), Node::parent).find(|node| boundaries.contains(&node.kind()))
}

fn python_function_scope(node: Node<'_>, owner: Node<'_>) -> bool {
    !successors(node.parent(), Node::parent)
        .take_while(|node| *node != owner)
        .any(|node| {
            matches!(
                node.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
                    | "class_definition"
            )
        })
}

fn contains_span(node: Node<'_>, loc: &VarLocation) -> bool {
    node.start_byte() <= loc.start_byte
        && loc.start_byte < loc.end_byte
        && loc.end_byte <= node.end_byte()
}

fn contains(parent: Node<'_>, child: Node<'_>) -> bool {
    parent.start_byte() <= child.start_byte() && child.end_byte() <= parent.end_byte()
}

fn is_bare_argument(parsed: &ParsedFile, node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && node
            .parent()
            .and_then(|args| args.parent())
            .is_some_and(|call| {
                parsed.language.is_call_node(call.kind())
                    && parsed.language.call_arguments(&call) == node.parent()
            })
}

fn leaves_on_line(node: Node<'_>, line: usize) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut todo = vec![node];
    while let Some(node) = todo.pop() {
        if node.start_position().row + 1 > line || node.end_position().row + 1 < line {
            continue;
        }
        let mut cursor = node.walk();
        if node.named_child_count() == 0 {
            result.push(node);
        } else {
            todo.extend(node.named_children(&mut cursor));
        }
    }
    result.sort_unstable_by_key(Node::start_byte);
    result
}

fn local_value_isolated(
    parsed: &ParsedFile,
    owner: Node<'_>,
    boundaries: &[&str],
    name: &str,
    immutable: bool,
) -> bool {
    let mut todo = vec![owner];
    while let Some(node) = todo.pop() {
        let mut cursor = node.walk();
        if node.named_child_count() != 0 {
            todo.extend(node.named_children(&mut cursor));
            continue;
        }
        if parsed.language.is_comment_or_string_node(node.kind()) {
            continue;
        }
        let text = parsed.node_text(&node);
        if parsed.language.is_js_family()
            && (text.contains('\\') || (!immutable && matches!(text, "eval" | "arguments")))
        {
            return false;
        }
        if !immutable
            && text == name
            && (enclosing_callable(node, boundaries) != Some(owner)
                || successors(node.parent(), Node::parent)
                    .take_while(|n| *n != owner)
                    .any(|n| {
                        matches!(n.kind(), "nonlocal_statement" | "global_statement")
                            || (parsed.language == Language::Go && go_may_address(n, node))
                    }))
        {
            return false;
        }
    }
    true
}

fn go_may_address(parent: Node<'_>, name: Node<'_>) -> bool {
    (parent.kind() == "unary_expression"
        && parent
            .child_by_field_name("operator")
            .is_some_and(|op| op.kind() == "&"))
        || (parent.kind() == "selector_expression"
            && parent
                .child_by_field_name("operand")
                .is_some_and(|operand| contains(operand, name)))
}

fn same_or_adjacent_statement(left: Node<'_>, right: Node<'_>) -> bool {
    fn statement(node: Node<'_>) -> Option<Node<'_>> {
        successors(node.parent(), Node::parent).find(|node| {
            matches!(
                node.kind(),
                "expression_statement"
                    | "assignment_statement"
                    | "short_var_declaration"
                    | "lexical_declaration"
            )
        })
    }
    let (Some(a), Some(b)) = (statement(left), statement(right)) else {
        return false;
    };
    a == b || {
        let (a, b) = if a.start_byte() < b.start_byte() {
            (a, b)
        } else {
            (b, a)
        };
        successors(a.next_named_sibling(), Node::next_named_sibling).find(|node| !node.is_extra())
            == Some(b)
    }
}
