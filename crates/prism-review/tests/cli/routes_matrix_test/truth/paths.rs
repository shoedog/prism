//! Adapter-specific keys for paths in saved runtime receipts.
use serde_json::{json, Value};

fn word_len(text: &str) -> usize {
    text.bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count()
}

fn identifier_len(text: &str) -> usize {
    match text.as_bytes().first() {
        Some(first) if first.is_ascii_alphabetic() || *first == b'_' => word_len(text),
        _ => 0,
    }
}

fn flask_parameter(after: &str) -> Option<(usize, &str)> {
    let name_len = identifier_len(after);
    if name_len == 0 {
        return None;
    }
    let converter = &after[..name_len];
    let tail = &after[name_len..];
    let variable_end = |binding: &str| {
        let len = identifier_len(binding);
        (len != 0 && binding.as_bytes().get(len) == Some(&b'>')).then_some(len + 1)
    };
    if tail.starts_with('>') {
        return Some((name_len + 1, "default"));
    }
    if let Some(binding) = tail.strip_prefix(':') {
        return Some((name_len + 1 + variable_end(binding)?, converter));
    }
    // Use the first closing parenthesis followed by a valid variable.
    // A colon or closing angle bracket inside the arguments is ordinary text.
    let args = tail.strip_prefix('(')?.split('\n').next()?;
    args.match_indices("):").find_map(|(index, _)| {
        let binding = &args[index + 2..];
        Some((name_len + 1 + index + 2 + variable_end(binding)?, converter))
    })
}

fn fastapi_parameter(after: &str) -> Option<(usize, &str)> {
    let end = after.find('}')?;
    let name = &after[..end];
    let (name, converter) = name.split_once(':').unwrap_or((name, "str"));
    if name.is_empty()
        || converter.is_empty()
        || identifier_len(name) != name.len()
        || identifier_len(converter) != converter.len()
    {
        return None;
    }
    Some((end + 1, converter))
}

pub(super) fn native_tokens(framework: &str, path: &str) -> Option<Vec<Value>> {
    let open = match framework {
        "flask" => '<',
        "fastapi" => '{',
        "express" if !path.contains(['*', '?', '+', '(', ')', '[', ']', '\\', '|', '^', '$']) => {
            ':'
        }
        _ => return None,
    };
    let mut tokens = Vec::new();
    let mut literal_start = 0;
    for (start, _) in path.match_indices(open) {
        if start < literal_start {
            continue;
        }
        let after = &path[start + 1..];
        let length = match framework {
            "flask" => {
                let (length, converter) = flask_parameter(after)?;
                if !matches!(
                    converter,
                    "default" | "string" | "int" | "float" | "uuid" | "any"
                ) {
                    return None;
                }
                length
            }
            "fastapi" => {
                let Some((length, converter)) = fastapi_parameter(after) else {
                    continue;
                };
                if !matches!(converter, "str" | "int" | "float" | "uuid") {
                    return None;
                }
                length
            }
            _ => {
                let length = word_len(after);
                if length == 0 {
                    continue;
                }
                length
            }
        };
        if start > literal_start {
            tokens.push(json!(["literal", &path[literal_start..start]]));
        }
        tokens.push(json!(["parameter"]));
        literal_start = start + 1 + length;
    }
    if literal_start < path.len() {
        tokens.push(json!(["literal", &path[literal_start..]]));
    }
    Some(tokens)
}
