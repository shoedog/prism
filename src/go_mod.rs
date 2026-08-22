#[derive(Debug, PartialEq, Eq)]
enum Token {
    Word(String),
    LeftParen,
    RightParen,
    Newline,
}

pub(crate) fn parse_module_path(go_mod: &str) -> Option<String> {
    let tokens = tokenize(go_mod)?;
    let mut module_path = None;
    let mut index = 0;

    while index < tokens.len() {
        while matches!(tokens.get(index), Some(Token::Newline)) {
            index += 1;
        }
        if index == tokens.len() {
            break;
        }

        let line_end = tokens[index..]
            .iter()
            .position(|token| *token == Token::Newline)
            .map(|offset| index + offset)
            .unwrap_or(tokens.len());
        if !matches!(tokens.get(index), Some(Token::Word(word)) if word == "module") {
            index = line_end.saturating_add(1);
            continue;
        }
        if module_path.is_some() {
            return None;
        }

        match tokens.get(index + 1) {
            Some(Token::Word(path)) if index + 2 == line_end => {
                module_path = Some(path.clone());
                index = line_end.saturating_add(1);
            }
            Some(Token::LeftParen) if index + 2 == line_end => {
                let (path, next_index) = parse_parenthesized_path(&tokens, line_end + 1)?;
                module_path = Some(path);
                index = next_index;
            }
            _ => return None,
        }
    }

    module_path.filter(|path| valid_module_path(path))
}

fn parse_parenthesized_path(tokens: &[Token], mut index: usize) -> Option<(String, usize)> {
    while matches!(tokens.get(index), Some(Token::Newline)) {
        index += 1;
    }
    let Token::Word(path) = tokens.get(index)? else {
        return None;
    };
    let path = path.clone();
    index += 1;
    while matches!(tokens.get(index), Some(Token::Newline)) {
        index += 1;
    }
    if !matches!(tokens.get(index), Some(Token::RightParen)) {
        return None;
    }
    index += 1;
    match tokens.get(index) {
        Some(Token::Newline) => index += 1,
        None => {}
        _ => return None,
    }
    Some((path, index))
}

fn tokenize(source: &str) -> Option<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' => index += 1,
            b'\r' => {
                index += 1;
                if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
                tokens.push(Token::Newline);
            }
            b'\n' => {
                index += 1;
                tokens.push(Token::Newline);
            }
            b'(' => {
                index += 1;
                tokens.push(Token::LeftParen);
            }
            b')' => {
                index += 1;
                tokens.push(Token::RightParen);
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => return None,
            b'"' => {
                let (word, next) = parse_interpreted_string(source, index + 1)?;
                tokens.push(Token::Word(word));
                index = next;
            }
            b'`' => {
                let rest = source.get(index + 1..)?;
                let end = rest.find('`')? + index + 1;
                let word = source.get(index + 1..end)?.replace('\r', "");
                tokens.push(Token::Word(word));
                index = end + 1;
            }
            _ => {
                let start = index;
                while index < bytes.len()
                    && !matches!(bytes[index], b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')')
                {
                    if matches!(bytes[index], b'"' | b'`')
                        || (bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*'))
                    {
                        return None;
                    }
                    index += 1;
                }
                tokens.push(Token::Word(source.get(start..index)?.to_string()));
            }
        }
    }

    Some(tokens)
}

fn parse_interpreted_string(source: &str, mut index: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let mut out = String::new();
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some((out, index + 1)),
            b'\r' | b'\n' => return None,
            b'\\' => {
                index += 1;
                let escape = *bytes.get(index)?;
                match escape {
                    b'a' => out.push('\u{7}'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'v' => out.push('\u{b}'),
                    b'\\' => out.push('\\'),
                    b'"' => out.push('"'),
                    b'\'' => out.push('\''),
                    b'x' => {
                        let value = parse_fixed_radix(bytes, index + 1, 2, 16)?;
                        out.push(char::from_u32(value)?);
                        index += 2;
                    }
                    b'u' => {
                        let value = parse_fixed_radix(bytes, index + 1, 4, 16)?;
                        out.push(char::from_u32(value)?);
                        index += 4;
                    }
                    b'U' => {
                        let value = parse_fixed_radix(bytes, index + 1, 8, 16)?;
                        out.push(char::from_u32(value)?);
                        index += 8;
                    }
                    b'0'..=b'7' => {
                        let value = parse_fixed_radix(bytes, index, 3, 8)?;
                        if value > u8::MAX.into() {
                            return None;
                        }
                        out.push(char::from_u32(value)?);
                        index += 2;
                    }
                    _ => return None,
                }
                index += 1;
            }
            _ => {
                let ch = source.get(index..)?.chars().next()?;
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    None
}

fn parse_fixed_radix(bytes: &[u8], start: usize, len: usize, radix: u32) -> Option<u32> {
    let end = start.checked_add(len)?;
    let digits = std::str::from_utf8(bytes.get(start..end)?).ok()?;
    u32::from_str_radix(digits, radix).ok()
}

fn valid_module_path(path: &str) -> bool {
    if path.is_empty() || !path.is_ascii() {
        return false;
    }
    let elements: Vec<&str> = path.split('/').collect();
    if elements.iter().any(|element| !valid_path_element(element)) {
        return false;
    }

    let first = elements[0];
    if !first.contains('.')
        || first.starts_with('-')
        || first.bytes().any(|byte| {
            !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
        })
    {
        return false;
    }

    valid_major_suffix(path, elements.last().copied().unwrap_or_default())
}

fn valid_path_element(element: &str) -> bool {
    if element.is_empty()
        || element.starts_with('.')
        || element.ends_with('.')
        || element.contains("..")
        || element.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
        })
    {
        return false;
    }

    let windows_prefix = element
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(windows_prefix.as_str(), "con" | "prn" | "aux" | "nul")
        || matches!(windows_prefix.as_bytes(), [b'c', b'o', b'm', b'1'..=b'9'])
        || matches!(windows_prefix.as_bytes(), [b'l', b'p', b't', b'1'..=b'9'])
    {
        return false;
    }

    if let Some((_, suffix)) = element.rsplit_once('~') {
        if !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    true
}

fn valid_major_suffix(path: &str, final_element: &str) -> bool {
    if let Some(rest) = path.strip_prefix("gopkg.in/") {
        let Some((name, major)) = rest.rsplit_once(".v") else {
            return false;
        };
        return !name.is_empty() && !name.contains('/') && valid_numeric_major(major, true);
    }

    let Some(major) = final_element.strip_prefix('v') else {
        return true;
    };
    if major.is_empty()
        || !major
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return true;
    }
    valid_numeric_major(major, false)
}

fn valid_numeric_major(major: &str, allow_one: bool) -> bool {
    !major.is_empty()
        && major.bytes().all(|byte| byte.is_ascii_digit())
        && !major.starts_with('0')
        && (allow_one || major != "1")
}

#[cfg(test)]
mod tests {
    use super::parse_module_path;

    #[test]
    fn accepts_supported_module_directive_forms() {
        let cases = [
            ("module example.com/m\n", "example.com/m"),
            ("module \"example.com/m\\x2fv2\"\n", "example.com/m/v2"),
            ("module `example.com/m`\n", "example.com/m"),
            (
                "module example.com/m // trailing comment\n",
                "example.com/m",
            ),
            ("module (\n    example.com/m\n)\n", "example.com/m"),
            ("module example.com/m/v2\n", "example.com/m/v2"),
        ];

        for (source, expected) in cases {
            assert_eq!(
                parse_module_path(source).as_deref(),
                Some(expected),
                "source: {source:?}"
            );
        }
    }

    #[test]
    fn rejects_malformed_or_semantically_invalid_module_directives() {
        let cases = [
            "module example.com/m trailing\n",
            "module example.com/one\nmodule example.com/two\n",
            "module\n",
            "module \"example.com/my module\"\n",
            "module example.com/m /* not a go.mod comment */\n",
            "module bad!path\n",
            "module example.com/m/v1\n",
            "module example.com/CON/pkg\n",
            "module example.com/a..b\n",
            "module Example.com/m\n",
            "module example/m\n",
        ];

        for source in cases {
            assert_eq!(parse_module_path(source), None, "source: {source:?}");
        }
    }
}
