//! JS/TS request-source field-access matching.

use crate::algorithms::taint::cleansing::node_contains_line;
use crate::algorithms::taint::js_ts::source_ranges::js_ts_request_source_access_ranges_for_line;
use crate::ast::tree_helpers::is_js_ts_language;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn node_contains_js_ts_source_access(
    parsed: &ParsedFile,
    node: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> bool {
    let text = parsed.node_text(&node);
    if source_params
        .iter()
        .any(|param| js_ts_source_access_text_matches(text, framework, param))
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_source_access(parsed, child, framework, source_params) {
            return true;
        }
    }
    false
}

pub(super) fn node_contains_js_ts_source_access_with_request_object_aliases(
    parsed: &ParsedFile,
    node: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
    request_object_aliases: &BTreeSet<String>,
) -> bool {
    node_contains_js_ts_source_access(parsed, node, framework, source_params)
        || (framework == "koa"
            && node_contains_js_ts_request_object_source_access(
                parsed,
                node,
                request_object_aliases,
            ))
}

pub(super) fn node_contains_js_ts_source_access_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> bool {
    if !node_contains_line(&node, line) {
        return false;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let start_line = node.start_position().row + 1;
    if start_line == line {
        let text = parsed.node_text(&node);
        if source_params
            .iter()
            .any(|param| js_ts_source_access_text_matches(text, framework, param))
        {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_source_access_on_line(
            parsed,
            child,
            root_func_id,
            line,
            framework,
            source_params,
        ) {
            return true;
        }
    }
    false
}

pub(in crate::algorithms::taint) fn node_contains_js_ts_source_access_on_line_with_request_object_aliases(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    source_params: &BTreeSet<String>,
    request_object_aliases: &BTreeSet<String>,
) -> bool {
    node_contains_js_ts_source_access_on_line(
        parsed,
        node,
        root_func_id,
        line,
        framework,
        source_params,
    ) || (framework == "koa"
        && node_contains_js_ts_request_object_source_access_on_line(
            parsed,
            node,
            root_func_id,
            line,
            request_object_aliases,
        ))
}

fn node_contains_js_ts_request_object_source_access(
    parsed: &ParsedFile,
    node: Node<'_>,
    aliases: &BTreeSet<String>,
) -> bool {
    let text = parsed.node_text(&node);
    if aliases.iter().any(|alias| {
        JS_TS_REQUEST_DATA_FIELDS
            .iter()
            .any(|field| js_ts_field_access_text_matches(text, alias, field))
    }) {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_request_object_source_access(parsed, child, aliases) {
            return true;
        }
    }
    false
}

pub(super) fn node_contains_js_ts_request_object_source_access_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    aliases: &BTreeSet<String>,
) -> bool {
    if !node_contains_line(&node, line) {
        return false;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let start_line = node.start_position().row + 1;
    if start_line == line {
        let text = parsed.node_text(&node);
        if aliases.iter().any(|alias| {
            JS_TS_REQUEST_DATA_FIELDS
                .iter()
                .any(|field| js_ts_field_access_text_matches(text, alias, field))
        }) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_request_object_source_access_on_line(
            parsed,
            child,
            root_func_id,
            line,
            aliases,
        ) {
            return true;
        }
    }
    false
}

pub(super) const JS_TS_REQUEST_DATA_FIELDS: &[&str] = &[
    "body",
    "query",
    "params",
    "headers",
    "cookies",
    "files",
    "url",
    "path",
    "originalUrl",
    "host",
    "hostname",
];

const JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS: &[&str] = &[
    "query",
    "params",
    "headers",
    "cookies",
    "files",
    "url",
    "path",
    "originalUrl",
    "host",
    "hostname",
];

pub(super) fn js_ts_request_field_allowed(framework: &str, field: &str) -> bool {
    match framework {
        "fastify" | "express" => JS_TS_REQUEST_DATA_FIELDS.contains(&field),
        "koa" => JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS.contains(&field),
        _ => false,
    }
}

pub(in crate::algorithms::taint) fn js_ts_request_source_identifier(text: &str) -> bool {
    JS_TS_REQUEST_DATA_FIELDS.contains(&text) || text == "request"
}

fn js_ts_koa_request_base_texts(param: &str) -> [String; 6] {
    [
        format!("{}.request", param),
        format!("{}?.request", param),
        format!("{}[\"request\"]", param),
        format!("{}['request']", param),
        format!("{}?.[\"request\"]", param),
        format!("{}?.['request']", param),
    ]
}

fn js_ts_koa_request_field_access_text_matches(text: &str, param: &str, field: &str) -> bool {
    js_ts_koa_request_base_texts(param)
        .iter()
        .any(|base| js_ts_field_access_text_matches(text, base, field))
}

pub(super) fn js_ts_normalized_property_key(parsed: &ParsedFile, key: &Node<'_>) -> Option<String> {
    let text = parsed.node_text(key).trim();
    let text = if let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let inner = inner.trim();
        inner
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?
    } else {
        text
    };
    let text = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(text);
    (!text.is_empty()).then(|| text.to_string())
}

pub(super) fn js_ts_exact_field_access_text_matches(text: &str, base: &str, field: &str) -> bool {
    let text = text.trim();
    if js_ts_exact_field_access_compact_text_matches(text, base, field) {
        return true;
    }
    if !text.chars().any(char::is_whitespace) {
        return false;
    }
    let compact = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    js_ts_exact_field_access_compact_text_matches(&compact, base, field)
}

fn js_ts_exact_field_access_compact_text_matches(text: &str, base: &str, field: &str) -> bool {
    [
        format!("{}.{}", base, field),
        format!("{}[\"{}\"]", base, field),
        format!("{}['{}']", base, field),
        format!("{}?.{}", base, field),
        format!("{}?.[\"{}\"]", base, field),
        format!("{}?.['{}']", base, field),
    ]
    .iter()
    .any(|candidate| text == candidate)
}

pub(super) fn js_ts_field_access_text_matches(text: &str, base: &str, field: &str) -> bool {
    let text = text.trim();
    if js_ts_field_access_compact_text_matches(text, base, field) {
        return true;
    }
    if !text.chars().any(char::is_whitespace) {
        return false;
    }
    let compact = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    js_ts_field_access_compact_text_matches(&compact, base, field)
}

fn js_ts_field_access_compact_text_matches(text: &str, base: &str, field: &str) -> bool {
    let prefixes = [
        format!("{}.{}", base, field),
        format!("{}[\"{}\"]", base, field),
        format!("{}['{}']", base, field),
        format!("{}?.{}", base, field),
        format!("{}?.[\"{}\"]", base, field),
        format!("{}?.['{}']", base, field),
    ];
    prefixes.iter().any(|prefix| {
        text == prefix
            || text.starts_with(&format!("{}.", prefix))
            || text.starts_with(&format!("{}[", prefix))
            || text.starts_with(&format!("{}?.", prefix))
            || text.starts_with(&format!("{}?.[", prefix))
    })
}

pub(super) fn js_ts_source_access_text_matches(text: &str, framework: &str, param: &str) -> bool {
    let text = text.trim();
    match framework {
        "nestjs" => {
            text == param
                || text.starts_with(&format!("{}.", param))
                || text.starts_with(&format!("{}[", param))
        }
        "fastify" | "express" => JS_TS_REQUEST_DATA_FIELDS
            .iter()
            .any(|field| js_ts_field_access_text_matches(text, param, field)),
        "koa" => {
            JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS
                .iter()
                .any(|field| js_ts_field_access_text_matches(text, param, field))
                || JS_TS_REQUEST_DATA_FIELDS
                    .iter()
                    .any(|field| js_ts_koa_request_field_access_text_matches(text, param, field))
        }
        _ => false,
    }
}

pub(in crate::algorithms::taint) fn js_ts_framework_source_access_ranges_by_line(
    files: &BTreeMap<String, ParsedFile>,
    source_lines: &BTreeSet<(String, usize)>,
) -> BTreeMap<(String, usize), Vec<(usize, usize)>> {
    let mut out = BTreeMap::new();
    for (file, line) in source_lines {
        let Some(parsed) = files.get(file) else {
            continue;
        };
        if !is_js_ts_language(parsed.language) {
            continue;
        }
        let ranges = js_ts_request_source_access_ranges_for_line(parsed, *line);
        if !ranges.is_empty() {
            out.insert((file.clone(), *line), ranges);
        }
    }
    out
}
