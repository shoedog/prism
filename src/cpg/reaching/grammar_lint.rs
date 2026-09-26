use super::binding_table::{
    guard_span_row, java_binding_identifiers, java_statement_cannot_complete_normally,
    select_binding_row, Ruling, Visibility,
};
use super::scope::{binding_scope_rule, RootFunction};
use super::Line;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use tree_sitter::Node;

#[derive(serde::Deserialize)]
struct NodeTypeSchema {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    subtypes: Vec<NodeTypeRef>,
    #[serde(default)]
    fields: BTreeMap<String, NodeTypeMembers>,
    #[serde(default)]
    children: Option<NodeTypeMembers>,
}

#[derive(serde::Deserialize)]
struct NodeTypeMembers {
    #[serde(default)]
    types: Vec<NodeTypeRef>,
}

#[derive(serde::Deserialize)]
struct NodeTypeRef {
    #[serde(rename = "type")]
    kind: String,
}

pub(super) type IntroducingFields = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ByteSpan {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl ByteSpan {
    pub(super) fn contains(self, byte: usize) -> bool {
        self.start <= byte && byte < self.end
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SpanKind {
    Known(Vec<ByteSpan>),
    Empty,
    Unknown,
}

pub(super) fn visibility_span(parsed: &ParsedFile, binder_node: Node<'_>) -> SpanKind {
    if let Some(row) = guard_span_row(parsed.language, binder_node.kind()) {
        let Some(guard) = ancestor_of_kind(binder_node, row.guard_kind) else {
            return SpanKind::Unknown;
        };
        let Some(owner) = ancestor_of_kind(guard, row.owner_kind) else {
            return SpanKind::Unknown;
        };
        let Some(condition) = owner.child_by_field_name(row.condition_field) else {
            return SpanKind::Unknown;
        };
        if !(condition.start_byte() <= guard.start_byte()
            && guard.end_byte() <= condition.end_byte())
        {
            return SpanKind::Unknown;
        }
        let negated = guard_is_negated(parsed, guard, condition);
        let mut spans = Vec::new();
        if !negated && row.positive_condition_tail && binder_node.end_byte() < condition.end_byte()
        {
            spans.push(ByteSpan {
                start: binder_node.end_byte(),
                end: condition.end_byte(),
            });
        }
        let fields = if negated {
            row.negative_fields
        } else {
            row.positive_fields
        };
        for field in fields {
            if let Some(node) = owner.child_by_field_name(field) {
                spans.push(ByteSpan {
                    start: node.start_byte(),
                    end: node.end_byte(),
                });
            }
        }
        if negated
            && java_statement_cannot_complete_normally(owner.child_by_field_name("consequence"))
        {
            if let Some(parent) = owner.parent() {
                if owner.end_byte() < parent.end_byte() {
                    spans.push(ByteSpan {
                        start: owner.end_byte(),
                        end: parent.end_byte(),
                    });
                }
            }
        }
        return if spans.is_empty() {
            SpanKind::Empty
        } else {
            SpanKind::Known(spans)
        };
    }

    let scoped = match parsed.language {
        Language::Rust => rust_visibility_node(binder_node),
        Language::Python => ancestor_field(binder_node, "if_statement", "consequence"),
        Language::Go | Language::JavaScript | Language::TypeScript | Language::Tsx => {
            nearest_field(binder_node, "body")
        }
        _ => None,
    };
    let Some(scoped) = scoped else {
        return SpanKind::Unknown;
    };
    if scoped.named_child_count() == 0 {
        SpanKind::Empty
    } else {
        SpanKind::Known(vec![ByteSpan {
            start: scoped.start_byte(),
            end: scoped.end_byte(),
        }])
    }
}

pub(super) fn uncertain_binding_nodes<'a>(parsed: &'a ParsedFile, name: &str) -> Vec<Node<'a>> {
    fn walk<'a>(parsed: &'a ParsedFile, node: Node<'a>, name: &str, out: &mut Vec<Node<'a>>) {
        let uncertain = select_binding_row(parsed, node)
            .is_some_and(|row| matches!(row.ruling, Ruling::Uncertain { .. }));
        if uncertain && node_introduces_name(parsed, node, name) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(parsed, child, name, out);
        }
    }

    let mut nodes = Vec::new();
    walk(parsed, parsed.tree.root_node(), name, &mut nodes);
    nodes
}

fn node_introduces_name(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    if parsed.language == Language::Java {
        return java_node_introduces_name(parsed, node, name);
    }
    let special_target = match (parsed.language, node.kind()) {
        (Language::Rust, "tuple_struct_pattern") => Some(node),
        (Language::Bash, "for_statement") => node.child_by_field_name("variable"),
        _ => None,
    };
    if let Some(target) = special_target {
        return target_introduces_name(parsed, node, target, name);
    }
    grammar_introducing_fields(parsed.language)
        .get(node.kind())
        .is_some_and(|fields| {
            fields.iter().any(|field| {
                node.child_by_field_name(field)
                    .is_some_and(|target| target_introduces_name(parsed, node, target, name))
            })
        })
}

fn java_node_introduces_name(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    let Some(root_function) = parsed
        .all_functions()
        .into_iter()
        .filter(|function| {
            function.start_byte() <= node.start_byte() && node.end_byte() <= function.end_byte()
        })
        .min_by_key(|function| function.end_byte() - function.start_byte())
    else {
        return false;
    };
    let root = RootFunction::new(parsed, root_function);
    java_binding_identifiers(node).into_iter().any(|target| {
        parsed.node_text(&target) == name
            && !introduction_is_classified(parsed, node, target, &root)
    })
}

fn target_introduces_name(
    parsed: &ParsedFile,
    construct: Node<'_>,
    target: Node<'_>,
    name: &str,
) -> bool {
    let Some(root_function) = parsed
        .all_functions()
        .into_iter()
        .filter(|function| {
            function.start_byte() <= construct.start_byte()
                && construct.end_byte() <= function.end_byte()
        })
        .min_by_key(|function| function.end_byte() - function.start_byte())
    else {
        return false;
    };
    let root = RootFunction::new(parsed, root_function);
    subtree_has_name(parsed, target, name)
        && !introduction_is_classified(parsed, construct, target, &root)
}

fn subtree_has_name(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
    if parsed.language.is_identifier_node(node.kind()) && parsed.node_text(&node) == name {
        return true;
    }
    if node.named_child_count() == 0 && parsed.node_text(&node) == name {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| subtree_has_name(parsed, child, name));
    found
}

fn ancestor_of_kind<'a>(mut node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    loop {
        if node.kind() == kind {
            return Some(node);
        }
        node = node.parent()?;
    }
}

fn ancestor_field<'a>(mut node: Node<'a>, kind: &str, field: &str) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if parent.kind() == kind {
            return parent.child_by_field_name(field);
        }
        node = parent;
    }
    None
}

fn nearest_field<'a>(mut node: Node<'a>, field: &str) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if let Some(scoped) = parent.child_by_field_name(field) {
            return Some(scoped);
        }
        node = parent;
    }
    None
}

fn rust_visibility_node(binder_node: Node<'_>) -> Option<Node<'_>> {
    let mut current = binder_node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "match_arm" => return parent.child_by_field_name("value"),
            "if_expression" => return parent.child_by_field_name("consequence"),
            "while_expression" => return parent.child_by_field_name("body"),
            _ => current = parent,
        }
    }
    None
}

fn guard_is_negated(parsed: &ParsedFile, guard: Node<'_>, condition: Node<'_>) -> bool {
    let mut current = guard;
    let mut negated = false;
    while let Some(parent) = current.parent() {
        if parent.id() == condition.id() {
            break;
        }
        if parent.kind() == "unary_expression" {
            let is_not = parent
                .child_by_field_name("operator")
                .is_some_and(|operator| parsed.node_text(&operator) == "!");
            negated ^= is_not;
        }
        current = parent;
    }
    negated
}

pub(super) fn collect_unclassified_binding_lines(
    parsed: &ParsedFile,
    node: Node<'_>,
    root: &RootFunction<'_>,
    callable_boundary_kinds: &[&str],
    introducing_fields: &IntroducingFields,
    lines: &mut BTreeMap<String, BTreeSet<Line>>,
) {
    #[cfg(test)]
    super::work_tests::bump(|counts| counts.binding_nodes += 1);
    let root_function_id = root.function.id();
    if node.id() != root_function_id && callable_boundary_kinds.contains(&node.kind()) {
        return;
    }

    if node.id() != root_function_id {
        if let Some(fields) = introducing_fields.get(node.kind()) {
            for field in fields {
                if let Some(target) = node.child_by_field_name(field) {
                    if !introduction_is_classified(parsed, node, target, root) {
                        let dominance_lines = dominance_route_lines(node, root_function_id);
                        collect_binding_identifiers(parsed, target, &dominance_lines, lines);
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_unclassified_binding_lines(
            parsed,
            child,
            root,
            callable_boundary_kinds,
            introducing_fields,
            lines,
        );
    }
}

fn collect_binding_identifiers(
    parsed: &ParsedFile,
    node: Node<'_>,
    dominance_lines: &[Line],
    lines: &mut BTreeMap<String, BTreeSet<Line>>,
) {
    let mut cursor = node.walk();
    let named_children: Vec<_> = node.named_children(&mut cursor).collect();
    if named_children.is_empty() {
        let name = parsed.node_text(&node);
        if is_binding_name(name) {
            let entry = lines.entry(name.to_string()).or_default();
            entry.insert(node.start_position().row + 1);
            entry.extend(dominance_lines.iter().copied());
        }
        return;
    }
    if parsed.language.is_identifier_node(node.kind()) {
        lines
            .entry(parsed.node_text(&node).to_string())
            .or_default()
            .insert(node.start_position().row + 1);
        return;
    }
    for child in named_children {
        collect_binding_identifiers(parsed, child, dominance_lines, lines);
    }
}

fn dominance_route_lines(node: Node<'_>, root_function_id: usize) -> Vec<Line> {
    #[cfg(test)]
    super::work_tests::bump(|counts| counts.dominance_calls += 1);
    let introduction_line = node.start_position().row + 1;
    let mut current = Some(node);
    while let Some(candidate) = current {
        if candidate.id() == root_function_id {
            break;
        }
        if let Some(body) = candidate.child_by_field_name("body") {
            let mut lines = BTreeSet::new();
            collect_lines_after(body, introduction_line, &mut lines);
            if !lines.is_empty() {
                return lines.into_iter().collect();
            }
        }
        current = candidate.parent();
    }
    Vec::new()
}

fn collect_lines_after(node: Node<'_>, introduction_line: Line, lines: &mut BTreeSet<Line>) {
    #[cfg(test)]
    super::work_tests::bump(|counts| counts.dominance_nodes += 1);
    let line = node.start_position().row + 1;
    if line > introduction_line {
        lines.insert(line);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_lines_after(child, introduction_line, lines);
    }
}

fn is_binding_name(text: &str) -> bool {
    text != "_"
        && !text.is_empty()
        && text.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphanumeric() && (index > 0 || !byte.is_ascii_digit())
        })
}

pub(super) fn introduction_is_classified(
    parsed: &ParsedFile,
    node: Node<'_>,
    target: Node<'_>,
    root: &RootFunction<'_>,
) -> bool {
    #[cfg(test)]
    super::work_tests::bump(|counts| counts.introductions += 1);
    let parameters = root.parameters(parsed);
    #[cfg(test)]
    let parameters = super::work_tests::parameters(parsed, root.function, parameters);
    if parameters.is_some_and(|parameters| {
        parameters.start_byte() <= target.start_byte() && target.end_byte() <= parameters.end_byte()
    }) {
        return true;
    }

    if parsed.language == Language::Python && is_python_comprehension_target(node) {
        return true;
    }

    if target.kind() == "identifier"
        && select_binding_row(parsed, node).is_some_and(|row| row.visibility == Visibility::Header)
    {
        return true;
    }

    let composite_pattern = matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx | Language::Rust
    ) && target.kind() != "identifier";
    let mut current = Some(node);
    while let Some(candidate) = current {
        if binding_scope_rule(parsed.language, candidate.kind())
            .declaration
            .is_some()
        {
            return !composite_pattern;
        }
        if candidate.id() == root.function.id() {
            break;
        }
        current = candidate.parent();
    }
    false
}

fn is_python_comprehension_target(node: Node<'_>) -> bool {
    if node.kind() != "for_in_clause" {
        return false;
    }
    let mut current = node.parent();
    while let Some(candidate) = current {
        if matches!(
            candidate.kind(),
            "list_comprehension"
                | "set_comprehension"
                | "dictionary_comprehension"
                | "generator_expression"
        ) {
            return true;
        }
        current = candidate.parent();
    }
    false
}

pub(super) fn grammar_introducing_fields(language: Language) -> &'static IntroducingFields {
    static BY_LANGUAGE: OnceLock<BTreeMap<Language, IntroducingFields>> = OnceLock::new();
    BY_LANGUAGE
        .get_or_init(|| {
            Language::all()
                .into_iter()
                .map(|language| (language, derive_introducing_fields(language)))
                .collect()
        })
        .get(&language)
        .expect("every supported language has node-type metadata")
}

fn derive_introducing_fields(language: Language) -> IntroducingFields {
    let schemas: Vec<NodeTypeSchema> = serde_json::from_str(language.node_types_json())
        .expect("tree-sitter node-types.json must be valid");
    let members: BTreeMap<_, _> = schemas
        .iter()
        .map(|schema| {
            let mut kinds: Vec<_> = schema
                .subtypes
                .iter()
                .map(|member| member.kind.as_str())
                .collect();
            if let Some(children) = &schema.children {
                kinds.extend(children.types.iter().map(|member| member.kind.as_str()));
            }
            for field in schema.fields.values() {
                kinds.extend(field.types.iter().map(|member| member.kind.as_str()));
            }
            (schema.kind.as_str(), kinds)
        })
        .collect();
    schemas
        .iter()
        .filter_map(|schema| {
            let fields: BTreeSet<_> = schema
                .fields
                .iter()
                .filter(|(field, members_for_field)| {
                    is_name_introducing_field(field)
                        && (is_intrinsically_introducing_field(field)
                            || looks_name_introducing_kind(&schema.kind))
                        && members_for_field.types.iter().any(|member| {
                            type_can_contain_identifier(
                                &member.kind,
                                &members,
                                &mut BTreeSet::new(),
                            )
                        })
                })
                .map(|(field, _)| field.clone())
                .collect();
            (!fields.is_empty()).then_some((schema.kind.clone(), fields))
        })
        .collect()
}

fn looks_name_introducing_kind(kind: &str) -> bool {
    [
        "declaration",
        "declarator",
        "parameter",
        "pattern",
        "clause",
        "for_",
        "for_statement",
        "with_",
        "except_",
        "catch_",
        "binding",
        "variable",
    ]
    .iter()
    .any(|part| kind.contains(part))
        || kind.ends_with("_item")
        || kind.ends_with("_arm")
}

fn is_name_introducing_field(field: &str) -> bool {
    matches!(
        field,
        "name" | "pattern" | "left" | "target" | "alias" | "parameter" | "parameters"
    ) || field.ends_with("_name")
        || field.ends_with("_pattern")
        || field.ends_with("_target")
}

fn is_intrinsically_introducing_field(field: &str) -> bool {
    matches!(field, "pattern" | "target" | "parameter" | "parameters")
        || field.ends_with("_pattern")
        || field.ends_with("_target")
}

fn type_can_contain_identifier<'a>(
    kind: &'a str,
    members: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
) -> bool {
    if kind.contains("identifier")
        || kind.ends_with("_target")
        || matches!(kind, "variable_name" | "name")
    {
        return true;
    }
    if !visiting.insert(kind) {
        return false;
    }
    let result = members.get(kind).is_some_and(|children| {
        children
            .iter()
            .any(|child| type_can_contain_identifier(child, members, visiting))
    });
    visiting.remove(kind);
    result
}
