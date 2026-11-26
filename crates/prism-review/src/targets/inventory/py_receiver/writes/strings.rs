use crate::targets::inventory::python::source::{children, ungroup};
use prism::ast::ParsedFile;
use tree_sitter::Node;

/// A syntactic string key can model a write even when its value can't be
/// decoded. Computed keys are outside the write index.
pub(super) fn write_key(parsed: &ParsedFile, node: Node<'_>) -> Option<Option<String>> {
    let node = ungroup(node);
    fn plain_text(parsed: &ParsedFile, node: Node<'_>) -> bool {
        match node.kind() {
            "concatenated_string" => children(node).all(|part| plain_text(parsed, part)),
            "string" => children(node).next().is_some_and(|start| {
                let delimiter = parsed.node_text(&start);
                let prefix = delimiter.trim_end_matches(['\'', '"']);
                matches!(prefix.to_ascii_lowercase().as_str(), "" | "r" | "u")
            }),
            _ => false,
        }
    }
    plain_text(parsed, node).then(|| literal(parsed, node))
}

pub(in crate::targets::inventory::py_receiver) fn literal(
    parsed: &ParsedFile,
    node: Node<'_>,
) -> Option<String> {
    let node = ungroup(node);
    match node.kind() {
        "concatenated_string" => children(node).try_fold(String::new(), |mut text, child| {
            text.push_str(&literal(parsed, child)?);
            Some(text)
        }),
        "string" => {
            let mut parts = children(node);
            let start = parts.next()?;
            let delimiter = parsed.node_text(&start);
            let quote = delimiter.find(['\'', '"'])?;
            let prefix = delimiter[..quote].to_ascii_lowercase();
            let end = parts.last().filter(|end| end.kind() == "string_end")?;
            let raw = parsed.source.get(start.end_byte()..end.start_byte())?;
            match prefix.as_str() {
                "r" => Some(raw.to_owned()),
                "u" | "" => decode(raw),
                _ => None,
            }
        }
        _ => None,
    }
}

fn decode(raw: &str) -> Option<String> {
    let mut input = raw.chars().peekable();
    let mut value = String::new();
    while let Some(ch) = input.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        let escaped = input.next()?;
        let decoded = match escaped {
            '\n' => continue,
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\x0b',
            '\'' | '"' | '\\' => escaped,
            'x' | 'u' | 'U' => {
                let count = match escaped {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let mut code = 0;
                for _ in 0..count {
                    code = code * 16 + input.next()?.to_digit(16)?;
                }
                char::from_u32(code)?
            }
            '0'..='7' => {
                let mut code = escaped.to_digit(8)?;
                for _ in 0..2 {
                    let Some(digit) = input.peek().and_then(|next| next.to_digit(8)) else {
                        break;
                    };
                    input.next();
                    code = code * 8 + digit;
                }
                char::from_u32(code)?
            }
            'N' => {
                if input.next()? != '{' {
                    return None;
                }
                let mut name = String::new();
                loop {
                    let ch = input.next()?;
                    if ch == '}' {
                        break;
                    }
                    name.push(ch.to_ascii_uppercase());
                }
                named_ascii_identifier_character(&name)?
            }
            _ => {
                value.push('\\');
                escaped
            }
        };
        value.push(decoded);
    }
    Some(value)
}

// Decode common ASCII names directly. An unrecognized name makes the write
// key unknown, so it can veto a matching namespace or object's lookup.
fn named_ascii_identifier_character(name: &str) -> Option<char> {
    for (prefix, base) in [
        ("LATIN CAPITAL LETTER ", b'A'),
        ("LATIN SMALL LETTER ", b'a'),
    ] {
        if let Some(suffix) = name.strip_prefix(prefix) {
            let [letter] = suffix.as_bytes() else {
                return None;
            };
            return letter
                .is_ascii_uppercase()
                .then(|| char::from(base + (letter - b'A')));
        }
    }
    if let Some(digit) = name.strip_prefix("DIGIT ") {
        const DIGIT_NAMES: [&str; 10] = [
            "ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE",
        ];
        return DIGIT_NAMES
            .iter()
            .position(|&value| value == digit)
            .map(|index| char::from(b'0' + index as u8));
    }
    (name == "LOW LINE").then_some('_')
}
