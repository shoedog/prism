//! JS/TS structured sink tables and their text-level semantic checks.

use crate::algorithms::taint::js_ts::bindings::{
    js_ts_identifier_binds_imported_member_at_call, js_ts_identifier_binds_module_at_call,
};
use crate::algorithms::taint_sinks;
use prism::ast::tree_helpers::call_arg_node;
use prism::ast::ParsedFile;
use prism::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use tree_sitter::Node;

fn js_spawn_uses_shell_true(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .is_ok_and(|text| text.contains("shell") && text.contains("true"))
}

fn js_yaml_load_uses_unsafe_schema(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .map_or(true, |text| !js_yaml_load_text_uses_safe_schema(text))
}

fn js_yaml_load_text_uses_safe_schema(text: &str) -> bool {
    let yaml_receiver = js_yaml_load_receiver(text);
    js_text_top_level_call_args(text)
        .get(1)
        .is_some_and(|arg| js_yaml_schema_arg_is_safe(arg, yaml_receiver))
}

fn js_yaml_load_receiver(text: &str) -> Option<&str> {
    let callee = text.split_once('(')?.0.trim();
    callee.strip_suffix(".load")
}

fn js_yaml_schema_arg_is_safe(arg: &str, yaml_receiver: Option<&str>) -> bool {
    let arg = arg.trim();
    js_yaml_schema_expr_is_exact_safe(arg, yaml_receiver)
        || js_trusted_object_property_value_text(arg, "schema")
            .is_some_and(|value| js_yaml_schema_expr_is_exact_safe(value, yaml_receiver))
}

fn js_yaml_schema_expr_is_exact_safe(expr: &str, yaml_receiver: Option<&str>) -> bool {
    let expr = expr.trim().trim_end_matches(';').trim();
    let Some(yaml_receiver) = yaml_receiver else {
        return false;
    };
    ["SAFE_SCHEMA", "FAILSAFE_SCHEMA", "JSON_SCHEMA"]
        .iter()
        .any(|schema| expr == format!("{yaml_receiver}.{schema}"))
}

pub(in crate::algorithms::taint) fn js_yaml_load_call_uses_unsafe_schema(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> bool {
    call_arg_node(call, 1).is_none_or(|arg| !js_yaml_schema_arg_node_is_safe(parsed, call, &arg))
}

fn js_yaml_schema_arg_node_is_safe(parsed: &ParsedFile, call: &Node<'_>, arg: &Node<'_>) -> bool {
    let text = parsed.node_text(arg).trim();
    js_yaml_schema_expr_text_is_safe(parsed, call, text)
        || js_trusted_object_property_value_text(text, "schema")
            .is_some_and(|value| js_yaml_schema_expr_text_is_safe(parsed, call, value))
}

fn js_yaml_schema_expr_text_is_safe(parsed: &ParsedFile, call: &Node<'_>, expr: &str) -> bool {
    let expr = expr.trim().trim_end_matches(';').trim();
    for schema in ["SAFE_SCHEMA", "FAILSAFE_SCHEMA", "JSON_SCHEMA"] {
        if expr == schema
            && js_ts_identifier_binds_imported_member_at_call(parsed, call, expr, "js-yaml", schema)
        {
            return true;
        }
        if let Some(receiver) = expr.strip_suffix(schema).and_then(|p| p.strip_suffix('.')) {
            if !receiver.is_empty()
                && js_ts_identifier_binds_module_at_call(parsed, call, receiver, "js-yaml")
            {
                return true;
            }
        }
    }
    false
}

fn js_trusted_object_property_value_text<'a>(object_text: &'a str, key: &str) -> Option<&'a str> {
    let object_text = object_text.trim();
    let inner = object_text.strip_prefix('{')?.strip_suffix('}')?;
    let mut value = None;
    for prop in js_split_top_level_commas(inner) {
        if prop.is_empty() {
            continue;
        }
        if prop.trim_start().starts_with("...") {
            return None;
        }
        let Some(colon) = js_find_top_level_colon(prop) else {
            continue;
        };
        let prop_key_text = prop[..colon].trim();
        if prop_key_text.starts_with('[') {
            return None;
        }
        let prop_key = prop_key_text.trim_matches(['"', '\'', '`']);
        if prop_key == key {
            if value.is_some() {
                return None;
            }
            value = Some(prop[colon + 1..].trim());
        }
    }
    value
}

fn js_text_top_level_call_args(text: &str) -> Vec<&str> {
    let Some(open) = text.find('(') else {
        return Vec::new();
    };
    let body = &text[open + 1..];
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut args = Vec::new();

    for (idx, ch) in body.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                args.push(body[start..idx].trim());
                return args;
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(body[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    args
}

pub(super) fn js_split_top_level_commas(text: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut parts = Vec::new();

    for (idx, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

pub(super) fn js_find_top_level_colon(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escape = false;
    for (idx, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn js_sql_call_is_not_parametrized(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .map_or(true, |text| {
            !(text.contains("bind") || text.contains("parameters"))
        })
}

pub const JS_CWE79_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "insertAdjacentHTML",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dangerouslySetInnerHTML",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE89_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "query",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_sql_call_is_not_parametrized),
    },
    SinkPattern {
        call_path: "execute",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_sql_call_is_not_parametrized),
    },
    SinkPattern {
        call_path: "raw",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "literal",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$where",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$queryRawUnsafe",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$executeRawUnsafe",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE918_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "fetch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "axios",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "got",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE502_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "unserialize",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "yaml.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_yaml_load_uses_unsafe_schema),
    },
    SinkPattern {
        call_path: "load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_yaml_load_uses_unsafe_schema),
    },
    SinkPattern {
        call_path: "deserialize",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "eval",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Function",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInNewContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInThisContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Script",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE78_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: Some(taint_sinks::js_exec_call_is_command),
    },
    SinkPattern {
        call_path: "execSync",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: Some(taint_sinks::js_exec_call_is_command),
    },
    SinkPattern {
        call_path: "spawn",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: Some(js_spawn_uses_shell_true),
    },
    SinkPattern {
        call_path: "spawnSync",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: Some(js_spawn_uses_shell_true),
    },
];

pub const JS_CWE22_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "readFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "readFileSync",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "writeFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "writeFileSync",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "createReadStream",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "createWriteStream",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "unlink",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "rm",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "rename",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "sendFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "download",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];
