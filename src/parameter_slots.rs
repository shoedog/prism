//! Exact positional parameter-slot extraction.
//!
//! Binding occurrence extraction deliberately has a different contract: it may
//! expose names which cannot be mapped to an argument position. This module is
//! for consumers that need positional identity and therefore stops before an
//! unsafe boundary instead of compressing it away.

use crate::ast::{ParameterOccurrence, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Non-positional TS binding occurrences. Reuse the whole-list safety checks,
/// but do not compress this result into argument slots: unsupported parameters
/// are omitted here, whereas `slots` retains its independent prefix contract.
///
/// Additive default-occurrence class: when every parameter in the list is a
/// supported ordinary simple identifier — required/optional without a
/// default, or required with an INERT default — the INERT-default
/// parameters' identifier tokens are added in declaration order, preserving
/// the relative order of existing required/optional occurrences.
pub(crate) fn typescript_parameter_bindings(
    parsed: &ParsedFile,
    function: &Node<'_>,
) -> Vec<ParameterOccurrence> {
    let Some(params) = parsed.find_parameters_node(function) else {
        return Vec::new();
    };
    if contains_recovery(params) {
        return Vec::new();
    }
    // Source spelling is not canonical identity for escaped identifiers. An
    // escaped binding anywhere in the list could alias a supported binding.
    let mut bindings = BTreeSet::new();
    let mut duplicate = false;
    for parameter in named_children(params) {
        collect_js_ts_parameter_bindings(parsed, parameter, &mut bindings, &mut duplicate);
    }
    if duplicate || bindings.iter().any(|name| name.contains('\\')) {
        return Vec::new();
    }
    // Optional-parameter occurrences additionally require the whole signature
    // to carry no initializer anywhere (a sibling default, or a default
    // nested in a destructuring pattern): the runtime binding identity of a
    // `?` token has not been proven safe under sibling defaults, so the
    // entire list is conservatively refused for optional occurrences when
    // one is present. Required occurrences are unaffected and keep their
    // existing independent per-parameter contract. This scan is
    // deliberately broad (any `=` token anywhere under `params`, including
    // inside a type annotation) rather than enumerating every initializer
    // shape; a false refusal from a non-runtime `=` in a type position is an
    // acceptable, documented conservative cost.
    let optional_signature_clear = !params_list_has_initializer(params);
    let mut occurrences: Vec<ParameterOccurrence> = named_children(params)
        .into_iter()
        .filter_map(|parameter| {
            let is_optional = parameter.kind() == "optional_parameter";
            if parameter.kind() != "required_parameter" && !is_optional {
                return None;
            }
            if is_optional && !optional_signature_clear {
                return None;
            }
            let pattern = parameter.child_by_field_name("pattern")?;
            if pattern.kind() != "identifier" {
                return None;
            }
            let annotation = parameter.child_by_field_name("type");
            let mut cursor = parameter.walk();
            // Exact allowlist includes unnamed tokens: readonly is unnamed in
            // the pinned grammar, and so is optional_parameter's own `?`.
            // Defaults, decorators and parameter properties must not acquire
            // a Def just because they contain an identifier.
            if parameter.children(&mut cursor).any(|child| {
                child != pattern
                    && Some(child) != annotation
                    && child.kind() != "comment"
                    && !(is_optional && !child.is_named() && child.kind() == "?")
            }) {
                return None;
            }
            Some((
                parsed.node_text(&pattern).to_string(),
                pattern.start_byte(),
                pattern.end_byte(),
            ))
        })
        .collect();
    if let Some(defaults) = typescript_inert_default_occurrences(parsed, params) {
        occurrences.extend(defaults);
        occurrences.sort_by_key(|occurrence| occurrence.1);
    }
    occurrences
}

/// Whole-signature conservative initializer scan for the optional-occurrence
/// barrier above. Deliberately over-broad: matches a top-level parameter's
/// `value` field (always paired with a literal `=`), and destructuring
/// defaults (`assignment_pattern`/`object_assignment_pattern`, also always
/// paired with a literal `=`), by scanning for the `=` token itself rather
/// than enumerating each wrapper node. A `=` occurring inside a type
/// annotation (type syntax, not a runtime default) is scoped in the same
/// scan and yields a conservative false refusal, not a false Def.
fn params_list_has_initializer(node: Node<'_>) -> bool {
    if node.kind() == "=" {
        return true;
    }
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children.into_iter().any(params_list_has_initializer)
}

/// Whole-signature guard for the newly-admitted default-occurrence class.
/// Every parameter must be `required_parameter`/`optional_parameter` with an
/// `identifier` pattern and no decorator/accessibility/`this`/rest/
/// destructuring form; a bare `optional_parameter` may carry no default at
/// all (optional-with-default is refused as neither "optional-without-
/// default" nor "required with an inert default"); a `required_parameter`
/// may carry no default, or an INERT one. A single disqualifying parameter
/// returns `None`, refusing default occurrences for the entire list without
/// touching the required/optional occurrences computed independently above.
fn typescript_inert_default_occurrences(
    parsed: &ParsedFile,
    params: Node<'_>,
) -> Option<Vec<ParameterOccurrence>> {
    let mut defaults = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "comment" => continue,
            "required_parameter" => {
                let pattern = parameter.child_by_field_name("pattern")?;
                if pattern.kind() != "identifier" {
                    return None;
                }
                let annotation = parameter.child_by_field_name("type");
                let value = parameter.child_by_field_name("value");
                let mut cursor = parameter.walk();
                if parameter.children(&mut cursor).any(|child| {
                    child != pattern
                        && Some(child) != annotation
                        && Some(child) != value
                        && child.kind() != "comment"
                        && !(value.is_some() && !child.is_named() && child.kind() == "=")
                }) {
                    return None;
                }
                if let Some(value) = value {
                    if !is_inert_default_value(value) {
                        return None;
                    }
                    defaults.push((
                        parsed.node_text(&pattern).to_string(),
                        pattern.start_byte(),
                        pattern.end_byte(),
                    ));
                }
            }
            "optional_parameter" => {
                if parameter.child_by_field_name("value").is_some() {
                    return None;
                }
                let pattern = parameter.child_by_field_name("pattern")?;
                if pattern.kind() != "identifier" {
                    return None;
                }
                let annotation = parameter.child_by_field_name("type");
                let mut cursor = parameter.walk();
                if parameter.children(&mut cursor).any(|child| {
                    child != pattern
                        && Some(child) != annotation
                        && child.kind() != "comment"
                        && !(!child.is_named() && child.kind() == "?")
                }) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(defaults)
}

/// Whole-signature guard + collection for JavaScript's newly-admitted
/// default-occurrence class. Purely additive: required identifier
/// occurrences already reach the caller through the pre-existing per-child
/// path in `ParsedFile::function_parameter_occurrences` and are untouched
/// here (`assignment_pattern` has no arm in `extract_param_name_node`).
pub(crate) fn javascript_inert_default_occurrences(
    parsed: &ParsedFile,
    params: Node<'_>,
) -> Vec<ParameterOccurrence> {
    if contains_recovery(params) {
        return Vec::new();
    }
    let mut bindings = BTreeSet::new();
    let mut duplicate = false;
    for parameter in named_children(params) {
        collect_js_ts_parameter_bindings(parsed, parameter, &mut bindings, &mut duplicate);
    }
    if duplicate || bindings.iter().any(|name| name.contains('\\')) {
        return Vec::new();
    }
    let mut defaults = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "comment" => continue,
            "identifier" => continue,
            "assignment_pattern" => {
                let Some(left) = parameter.child_by_field_name("left") else {
                    return Vec::new();
                };
                if left.kind() != "identifier" {
                    return Vec::new();
                }
                let Some(right) = parameter.child_by_field_name("right") else {
                    return Vec::new();
                };
                if !is_inert_default_value(right) {
                    return Vec::new();
                }
                let mut cursor = parameter.walk();
                if parameter.children(&mut cursor).any(|child| {
                    child != left
                        && child != right
                        && child.kind() != "comment"
                        && child.kind() != "="
                }) {
                    return Vec::new();
                }
                defaults.push((
                    parsed.node_text(&left).to_string(),
                    left.start_byte(),
                    left.end_byte(),
                ));
            }
            _ => return Vec::new(),
        }
    }
    defaults
}

/// The INERT default-value allowlist, shared by JS and TS: `number`,
/// `string` (no-substitution templates are a distinct `template_string`
/// kind and excluded), `true`, `false`, `null`, and an empty `object`/
/// `array` literal (only delimiters and comments are tolerated; commas also
/// disqualify, because an array hole is not an empty array). Deliberately
/// narrower than the source-observer upper bound: unary numerics (`-1`),
/// `undefined` (its own leaf grammar kind, not `identifier`), identifiers,
/// calls, property reads, functions, parenthesized/asserted/`satisfies`
/// expressions, filled containers and spreads are all excluded.
fn is_inert_default_value(node: Node<'_>) -> bool {
    match node.kind() {
        "number" | "string" | "true" | "false" | "null" => true,
        "object" | "array" => {
            let mut cursor = node.walk();
            let (open, close) = if node.kind() == "array" {
                ("[", "]")
            } else {
                ("{", "}")
            };
            let empty = node.children(&mut cursor).all(|child| {
                matches!(child.kind(), "comment") || child.kind() == open || child.kind() == close
            });
            empty
        }
        _ => false,
    }
}

pub(crate) fn slots(parsed: &ParsedFile, function: &Node<'_>) -> Option<Vec<ParameterOccurrence>> {
    let params = parsed
        .find_parameters_node(function)
        .or_else(|| function.child_by_field_name("parameter"))?;
    if contains_recovery(params) || parsed.language == Language::Java {
        return None;
    }
    if has_duplicate_js_ts_bindings(parsed, params) {
        return None;
    }
    if params.kind() == "identifier" {
        return unique(parsed, vec![params]);
    }

    let nodes = match parsed.language {
        Language::JavaScript => javascript_slots(parsed, params),
        Language::TypeScript | Language::Tsx => typescript_slots(parsed, params),
        Language::Go => go_slots(parsed, params),
        Language::Rust => rust_slots(parsed, params),
        Language::Python => python_slots(parsed, params),
        _ => fallback_slots(parsed, params),
    }?;
    unique(parsed, nodes)
}

fn unique(parsed: &ParsedFile, nodes: Vec<Node<'_>>) -> Option<Vec<ParameterOccurrence>> {
    let mut names = BTreeSet::new();
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        let name = parsed.node_text(&node).to_string();
        if !names.insert(name.clone()) {
            return None;
        }
        out.push((name, node.start_byte(), node.end_byte()));
    }
    Some(out)
}

fn contains_recovery(node: Node<'_>) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    let has_recovery = node.children(&mut cursor).any(contains_recovery);
    has_recovery
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn identifier(node: Node<'_>) -> Option<Node<'_>> {
    (node.kind() == "identifier").then_some(node)
}

/// Duplicates invalidate a JS/TS parameter list even if an earlier pattern
/// already truncates the deterministic positional prefix. This must inspect
/// binding patterns (not all identifiers) so `a = initializer` does not treat
/// an initializer reference as a second parameter binding.
fn has_duplicate_js_ts_bindings(parsed: &ParsedFile, params: Node<'_>) -> bool {
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return false;
    }

    let mut names = BTreeSet::new();
    let mut duplicate = false;
    let parameter_nodes = if params.kind() == "identifier" {
        vec![params]
    } else {
        named_children(params)
    };
    for parameter in parameter_nodes {
        collect_js_ts_parameter_bindings(parsed, parameter, &mut names, &mut duplicate);
    }
    duplicate
}

fn collect_js_ts_parameter_bindings(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &mut BTreeSet<String>,
    duplicate: &mut bool,
) {
    if let Some(pattern) = node.child_by_field_name("pattern") {
        collect_js_ts_binding_pattern_names(parsed, pattern, names, duplicate);
        return;
    }
    if let Some(name) = node.child_by_field_name("name") {
        collect_js_ts_binding_pattern_names(parsed, name, names, duplicate);
        return;
    }
    if let Some(left) = node.child_by_field_name("left") {
        collect_js_ts_binding_pattern_names(parsed, left, names, duplicate);
        return;
    }

    match node.kind() {
        "object_pattern" | "array_pattern" | "identifier" | "rest_pattern" => {
            collect_js_ts_binding_pattern_names(parsed, node, names, duplicate);
        }
        kind if kind.contains("parameter") => {
            for child in named_children(node) {
                if !matches!(child.kind(), "type_annotation" | "return_type") {
                    collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
                }
            }
        }
        _ => {}
    }
}

fn collect_js_ts_binding_pattern_names(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &mut BTreeSet<String>,
    duplicate: &mut bool,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node).to_string();
            *duplicate |= !names.insert(name);
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_binding_pattern_names(parsed, value, names, duplicate);
            }
        }
        "rest_pattern" => {
            for child in named_children(node) {
                collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_js_ts_binding_pattern_names(parsed, left, names, duplicate);
            }
        }
        "object_pattern"
        | "array_pattern"
        | "parenthesized_expression"
        | "parenthesized_pattern" => {
            for child in named_children(node) {
                collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
            }
        }
        _ => {}
    }
}

fn field_or_first_named<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .or_else(|| named_children(node).into_iter().next())
}

fn simple_pattern(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" => Some(node),
        "assignment_pattern" => node.child_by_field_name("left").and_then(identifier),
        "mutable_pattern" => named_children(node).into_iter().find_map(identifier),
        _ => None,
    }
}

fn javascript_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "rest_pattern" | "object_pattern" | "array_pattern" => break,
            _ => match simple_pattern(parameter) {
                Some(name) => out.push(name),
                None => break,
            },
        }
    }
    Some(out)
}

fn typescript_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        if matches!(
            parameter.kind(),
            "rest_pattern" | "object_pattern" | "array_pattern"
        ) {
            break;
        }
        let pattern = match parameter.kind() {
            "required_parameter" | "optional_parameter" => {
                field_or_first_named(parameter, &["pattern", "name"])
            }
            _ => Some(parameter),
        };
        let Some(pattern) = pattern else { break };
        if pattern.kind() == "this" {
            continue;
        }
        let Some(name) = simple_pattern(pattern) else {
            break;
        };
        out.push(name);
    }
    Some(out)
}

fn go_slots<'a>(parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for declaration in named_children(params) {
        if declaration.kind() == "variadic_parameter_declaration" {
            break;
        }
        if declaration.kind() != "parameter_declaration" {
            break;
        }
        let Some(ty) = declaration.child_by_field_name("type") else {
            break;
        };
        let names: Vec<_> = named_children(declaration)
            .into_iter()
            .filter(|node| node.kind() == "identifier" && node.end_byte() <= ty.start_byte())
            .collect();
        if names.is_empty() {
            break;
        }
        let mut hit_blank = false;
        for name in names {
            if parsed.node_text(&name) == "_" {
                hit_blank = true;
                break;
            }
            out.push(name);
        }
        if hit_blank {
            break;
        }
    }
    Some(out)
}

fn rust_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "self_parameter" => continue,
            "parameter" => {
                let Some(pattern) = parameter.child_by_field_name("pattern") else {
                    break;
                };
                let Some(name) = simple_pattern(pattern) else {
                    break;
                };
                out.push(name);
            }
            "identifier" => {
                if _parsed.node_text(&parameter) == "_" {
                    break;
                }
                out.push(parameter);
            }
            _ => break,
        }
    }
    Some(out)
}

fn python_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "positional_separator" => continue,
            "keyword_separator" | "list_splat_pattern" | "dictionary_splat_pattern" => break,
            "identifier" => out.push(parameter),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                let Some(pattern) = field_or_first_named(parameter, &["name", "pattern"]) else {
                    break;
                };
                let Some(name) = simple_pattern(pattern) else {
                    break;
                };
                out.push(name);
            }
            _ => break,
        }
    }
    Some(out)
}

fn fallback_slots<'a>(parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        let Some(name) = parsed.extract_param_name_node(&parameter) else {
            break;
        };
        out.push(name);
    }
    Some(out)
}
