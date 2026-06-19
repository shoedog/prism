//! Scope-honesty warnings for constructs the current reasoning model does not follow.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, Trace};
use crate::data_flow::VarLocation;
use crate::languages::Language;
use crate::navigation::types::{Location, ReasoningWarning, Warning, WarningKind};

/// Emit coarse scope-honesty telemetry for every function touched by a reasoning trace.
///
/// This is intentionally not a per-sink verdict contract. Plan-B callers that are attaching warnings
/// to a specific `SinkResult` should run [`warnings_for_function`] over that sink's function.
pub fn warnings_for_trace(
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    trace: &Trace,
) -> Vec<Warning> {
    let mut functions_by_file = BTreeMap::new();
    let mut touched_functions = BTreeSet::new();
    for idx in trace.frontier().into_iter().chain(
        trace
            .boundary
            .iter()
            .flat_map(|boundary| [boundary.from, boundary.to]),
    ) {
        let Some(loc) = cpg.to_var_location(idx) else {
            continue;
        };
        let Some(parsed) = files.get(&loc.file) else {
            continue;
        };
        let entries = functions_by_file
            .entry(loc.file.clone())
            .or_insert_with(|| function_entries(parsed));
        let Some(function) = function_for_var_location(entries, &loc) else {
            continue;
        };
        touched_functions.insert((loc.file, function.span.start_byte, function.span.end_byte));
    }

    let mut warnings = Vec::new();
    for (file, start_byte, end_byte) in touched_functions {
        let Some(parsed) = files.get(&file) else {
            continue;
        };
        let entries = functions_by_file
            .entry(file.clone())
            .or_insert_with(|| function_entries(parsed));
        let Some(function) = entries
            .iter()
            .find(|entry| entry.span.start_byte == start_byte && entry.span.end_byte == end_byte)
        else {
            continue;
        };
        warnings.extend(warnings_for_function(parsed, function.node));
    }
    warnings
}

/// Emit per-sink scope-honesty warnings for functions spanning `file:line`.
///
/// Line-only sinks are ambiguous on minified/same-line code, so this conservatively unions all
/// matching functions. Prefer [`warnings_for_function`] when the caller already has a sink function
/// identity.
pub fn warnings_for_sink_location(
    files: &BTreeMap<String, ParsedFile>,
    file: &str,
    line: usize,
) -> Vec<Warning> {
    let Some(parsed) = files.get(file) else {
        return Vec::new();
    };
    let entries = function_entries(parsed);
    let mut by_key = BTreeMap::new();
    for function in entries
        .iter()
        .filter(|entry| entry.span.contains_line(line))
    {
        for warning in warnings_for_function(parsed, function.node) {
            by_key.entry(warning_key(&warning)).or_insert(warning);
        }
    }
    by_key.into_values().collect()
}

/// Emit at most one per-sink warning per unmodeled construct class in `function`.
///
/// This is intentionally AST-level and read-only: reasoning callers can run it over the exact
/// function nodes they traversed and append the returned warnings to `Evidence.warnings`.
pub fn warnings_for_function(parsed: &ParsedFile, function: Node<'_>) -> Vec<Warning> {
    let function = canonical_function_root(parsed, function);
    let function_name = parsed
        .language
        .function_name(&function)
        .map(|name| parsed.node_text(&name).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let mut by_construct = BTreeMap::new();
    collect_warnings(
        parsed,
        function,
        function,
        &function_name,
        &mut by_construct,
    );
    by_construct.into_values().collect()
}

fn collect_warnings(
    parsed: &ParsedFile,
    root_function: Node<'_>,
    node: Node<'_>,
    function_name: &str,
    by_construct: &mut BTreeMap<String, Warning>,
) {
    let recorded_construct = if let Some(construct) = parsed
        .language
        .unmodeled_reasoning_construct(&node, &parsed.source)
    {
        by_construct
            .entry(construct.to_string())
            .or_insert_with(|| warning_for(parsed, node, function_name, construct));
        true
    } else {
        false
    };

    if is_nested_callable_boundary(parsed, root_function, node) {
        if !recorded_construct {
            let construct = nested_callable_construct(parsed.language);
            by_construct
                .entry(construct.to_string())
                .or_insert_with(|| warning_for(parsed, node, function_name, construct));
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_warnings(parsed, root_function, child, function_name, by_construct);
    }
}

#[derive(Clone, Debug)]
struct FunctionEntry<'a> {
    span: FunctionSpan,
    node: Node<'a>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FunctionSpan {
    name: Option<String>,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
}

impl FunctionSpan {
    fn contains_line(&self, line: usize) -> bool {
        self.start_line <= line && line <= self.end_line
    }

    fn contains_byte_range(&self, start_byte: usize, end_byte: usize) -> bool {
        self.start_byte <= start_byte && end_byte <= self.end_byte
    }

    fn width(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }
}

fn function_entries(parsed: &ParsedFile) -> Vec<FunctionEntry<'_>> {
    parsed
        .all_functions()
        .into_iter()
        .filter(|node| !is_python_decorated_inner_function(parsed, *node))
        .map(|node| FunctionEntry {
            span: function_span(parsed, node),
            node,
        })
        .collect()
}

fn canonical_function_root<'a>(parsed: &ParsedFile, node: Node<'a>) -> Node<'a> {
    if is_python_decorated_inner_function(parsed, node) {
        node.parent().unwrap_or(node)
    } else {
        node
    }
}

fn function_span(parsed: &ParsedFile, node: Node<'_>) -> FunctionSpan {
    let (start_line, end_line) = parsed.node_line_range(&node);
    FunctionSpan {
        name: parsed
            .language
            .function_name(&node)
            .map(|name| parsed.node_text(&name).to_string()),
        start_line,
        end_line,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}

fn function_for_var_location<'a>(
    entries: &'a [FunctionEntry<'a>],
    loc: &VarLocation,
) -> Option<&'a FunctionEntry<'a>> {
    choose_entry_by_var_span(
        entries
            .iter()
            .filter(|entry| {
                entry.span.name.as_deref() == Some(loc.function.as_str())
                    && entry.span.start_line == loc.function_start_line
            })
            .collect(),
        loc,
    )
    .or_else(|| {
        choose_entry_by_var_span(
            entries
                .iter()
                .filter(|entry| {
                    entry.span.name.as_deref() == Some(loc.function.as_str())
                        && entry.span.contains_line(loc.line)
                })
                .collect(),
            loc,
        )
    })
    .or_else(|| {
        choose_entry_by_var_span(
            entries
                .iter()
                .filter(|entry| entry.span.contains_line(loc.line))
                .collect(),
            loc,
        )
    })
}

fn choose_entry_by_var_span<'a>(
    candidates: Vec<&'a FunctionEntry<'a>>,
    loc: &VarLocation,
) -> Option<&'a FunctionEntry<'a>> {
    candidates
        .iter()
        .copied()
        .filter(|entry| entry.span.contains_byte_range(loc.start_byte, loc.end_byte))
        .min_by_key(|entry| entry.span.width())
        .or_else(|| {
            candidates
                .into_iter()
                .min_by_key(|entry| entry.span.width())
        })
}

fn warning_key(warning: &Warning) -> (String, String, String, usize, usize) {
    let construct = match &warning.kind {
        WarningKind::Reasoning(ReasoningWarning::UnmodeledConstruct {
            construct,
            function,
            ..
        }) => format!("{function}:{construct}"),
        other => format!("{other:?}"),
    };
    let (file, start_byte, end_byte) = warning
        .location
        .as_ref()
        .map(|location| {
            (
                location.file.clone(),
                location.start_byte,
                location.end_byte,
            )
        })
        .unwrap_or_default();
    (
        construct,
        warning.message.clone(),
        file,
        start_byte,
        end_byte,
    )
}

fn opens_unmodeled_callable_scope(language: Language, kind: &str) -> bool {
    match language {
        Language::Python => matches!(
            kind,
            "function_definition" | "decorated_definition" | "lambda"
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => matches!(
            kind,
            "function_declaration"
                | "method_definition"
                | "arrow_function"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
                | "generator_function_expression"
        ),
        Language::Go => matches!(
            kind,
            "function_declaration" | "method_declaration" | "func_literal"
        ),
        Language::Rust => matches!(kind, "function_item" | "closure_expression" | "async_block"),
        Language::Java => matches!(
            kind,
            "method_declaration" | "constructor_declaration" | "lambda_expression"
        ),
        Language::Cpp => matches!(
            kind,
            "function_definition" | "template_declaration" | "lambda_expression"
        ),
        Language::C => kind == "function_definition",
        Language::Lua => matches!(kind, "function_declaration" | "function_definition"),
        Language::Terraform => kind == "block",
        Language::Bash => kind == "function_definition",
    }
}
fn nested_callable_construct(language: Language) -> &'static str {
    match language {
        Language::Go => "Go nested callable",
        Language::Python => "Python nested callable",
        Language::Rust => "Rust nested callable",
        Language::Java => "Java nested callable",
        Language::Cpp => "C++ nested callable",
        Language::JavaScript | Language::TypeScript | Language::Tsx => "JS/TS nested callable",
        _ => "Nested callable",
    }
}

fn is_nested_callable_boundary(
    parsed: &ParsedFile,
    root_function: Node<'_>,
    node: Node<'_>,
) -> bool {
    if same_node(root_function, node) || is_decorated_definition_body(parsed, root_function, node) {
        return false;
    }

    opens_unmodeled_callable_scope(parsed.language, node.kind())
}

fn is_decorated_definition_body(
    parsed: &ParsedFile,
    root_function: Node<'_>,
    node: Node<'_>,
) -> bool {
    parsed.language == Language::Python
        && root_function.kind() == "decorated_definition"
        && node.kind() == "function_definition"
        && node
            .parent()
            .is_some_and(|parent| same_node(parent, root_function))
}

fn is_python_decorated_inner_function(parsed: &ParsedFile, node: Node<'_>) -> bool {
    parsed.language == Language::Python
        && node.kind() == "function_definition"
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "decorated_definition")
}

fn warning_for(
    parsed: &ParsedFile,
    node: Node<'_>,
    function_name: &str,
    construct: &'static str,
) -> Warning {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    Warning {
        kind: WarningKind::Reasoning(ReasoningWarning::UnmodeledConstruct {
            language: format!("{:?}", parsed.language),
            construct: construct.to_string(),
            function: function_name.to_string(),
        }),
        message: format!(
            "{function_name} contains {construct}; reasoning may miss flows through this unmodeled construct"
        ),
        location: Some(Location {
            file: parsed.path.clone(),
            start_line,
            end_line,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        }),
    }
}

fn same_node(a: Node<'_>, b: Node<'_>) -> bool {
    a.kind_id() == b.kind_id() && a.start_byte() == b.start_byte() && a.end_byte() == b.end_byte()
}
