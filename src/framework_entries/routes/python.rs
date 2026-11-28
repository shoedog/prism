//! Source-only Python syntax helpers for route adapters.
use crate::ast::ParsedFile;
use std::{borrow::Cow, collections::BTreeSet};
use tree_sitter::Node;

pub(crate) fn literal_string(file: &ParsedFile, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "string" => decode_string(file.node_text(&node)),
        "concatenated_string" => {
            let mut result = String::new();
            for child in children(node) {
                result.push_str(&literal_string(file, child)?);
            }
            Some(result)
        }
        "parenthesized_expression" => literal_string(file, *children(node).first()?),
        "binary_operator" if file.node_text(&node.child_by_field_name("operator")?) == "+" => {
            Some(format!(
                "{}{}",
                literal_string(file, node.child_by_field_name("left")?)?,
                literal_string(file, node.child_by_field_name("right")?)?
            ))
        }
        _ => None,
    }
}

fn decode_string(text: &str) -> Option<String> {
    let quote = text.find(['\'', '"'])?;
    let prefix = text[..quote].to_ascii_lowercase();
    if !matches!(prefix.as_str(), "" | "u" | "r") {
        return None;
    }
    let rest = &text[quote..];
    let delimiter = if rest.starts_with("'''") || rest.starts_with("\"\"\"") {
        &rest[..3]
    } else {
        &rest[..1]
    };
    let body = rest.strip_prefix(delimiter)?.strip_suffix(delimiter)?;
    let body = if body.contains('\r') {
        Cow::Owned(body.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(body)
    };
    if prefix == "r" {
        return Some(body.into_owned());
    }
    let mut chars = body.chars().peekable();
    let mut result = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        let escaped = chars.next()?;
        match escaped {
            '\n' => (),
            '\\' | '\'' | '"' => result.push(escaped),
            'a' => result.push('\x07'),
            'b' => result.push('\x08'),
            'f' => result.push('\x0c'),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'v' => result.push('\x0b'),
            'x' | 'u' | 'U' => {
                let length = match escaped {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let digits: String = chars.by_ref().take(length).collect();
                if digits.len() != length {
                    return None;
                }
                result.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?);
            }
            '0'..='7' => {
                let mut digits = escaped.to_string();
                for _ in 0..2 {
                    if chars.peek().is_some_and(|c| matches!(c, '0'..='7')) {
                        digits.push(chars.next()?);
                    } else {
                        break;
                    }
                }
                result.push(char::from_u32(u32::from_str_radix(&digits, 8).ok()?)?);
            }
            // Unknown escapes survive in Python; named Unicode escapes require
            // a Unicode name database, so they remain unproved.
            'N' => return None,
            other => {
                result.push('\\');
                result.push(other);
            }
        }
    }
    Some(result)
}

pub(crate) mod flask;

use super::{MethodSet, Site};

pub(crate) struct PythonArgs<'tree>(Option<Node<'tree>>);

pub(crate) fn arguments(call: Node<'_>) -> PythonArgs<'_> {
    PythonArgs(call.child_by_field_name("arguments"))
}

#[derive(Clone, Copy)]
pub(crate) enum PythonArg<'tree> {
    Written(Node<'tree>),
    Absent,
    Unknown(Node<'tree>),
}

impl<'tree> PythonArg<'tree> {
    pub(crate) fn written(self) -> Option<Node<'tree>> {
        match self {
            Self::Written(node) => Some(node),
            _ => None,
        }
    }

    pub(crate) fn none_as_absent(self) -> Self {
        match self {
            Self::Written(node) if ungroup(node).kind() == "none" => Self::Absent,
            _ => self,
        }
    }
}

impl<'tree> PythonArgs<'tree> {
    pub(crate) fn keyword(&self, file: &ParsedFile, key: &str) -> PythonArg<'tree> {
        let mut hidden = PythonArg::Absent;
        for arg in self.nodes() {
            match arg.kind() {
                "keyword_argument"
                    if arg
                        .child_by_field_name("name")
                        .is_some_and(|name| file.node_text(&name) == key) =>
                {
                    return arg
                        .child_by_field_name("value")
                        .map_or(PythonArg::Unknown(arg), PythonArg::Written);
                }
                "dictionary_splat" => hidden = PythonArg::Unknown(arg),
                _ => {}
            }
        }
        hidden
    }

    pub(crate) fn get(&self, file: &ParsedFile, key: &str, position: usize) -> PythonArg<'tree> {
        let keyword = self.keyword(file, key);
        if matches!(keyword, PythonArg::Written(_)) {
            return keyword;
        }
        for (index, arg) in self
            .nodes()
            .into_iter()
            .filter(|arg| !matches!(arg.kind(), "keyword_argument" | "dictionary_splat"))
            .enumerate()
        {
            if arg.kind() == "list_splat" {
                return PythonArg::Unknown(arg);
            }
            if index == position {
                return PythonArg::Written(arg);
            }
        }
        keyword
    }

    pub(crate) fn accepts(&self, file: &ParsedFile, slots: &[&str], extra_keywords: bool) -> bool {
        if self.0.is_none_or(|args| args.has_error()) {
            return false;
        }
        let mut names = BTreeSet::new();
        let mut count = 0;
        let mut splat = false;
        for arg in self.nodes() {
            match arg.kind() {
                "keyword_argument" => {
                    let Some(key) = arg.child_by_field_name("name") else {
                        return false;
                    };
                    let key = file.node_text(&key);
                    if key == "self"
                        || (!extra_keywords && !slots.contains(&key))
                        || !names.insert(key)
                    {
                        return false;
                    }
                }
                "dictionary_splat" => (),
                "list_splat" => splat = true,
                _ => {
                    if count == slots.len() {
                        return false;
                    }
                    if !splat && !names.insert(slots[count]) {
                        return false;
                    }
                    count += 1;
                }
            }
        }
        true
    }

    pub(crate) fn unpacked(&self) -> bool {
        self.nodes()
            .iter()
            .any(|node| matches!(node.kind(), "list_splat" | "dictionary_splat"))
    }

    pub(crate) fn nodes(&self) -> Vec<Node<'tree>> {
        self.0.map_or_else(Vec::new, children)
    }
}

pub(crate) fn children(node: Node<'_>) -> Vec<Node<'_>> {
    node.named_children(&mut node.walk())
        .filter(|node| !node.is_extra())
        .collect()
}

pub(crate) fn walk(root: Node<'_>) -> Vec<Node<'_>> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        nodes.push(node);
        pending.extend(children(node).into_iter().rev());
    }
    nodes
}

pub(super) fn comprehension(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression"
    )
}

pub(super) fn first_iterable(clause: Node<'_>) -> bool {
    clause.kind() == "for_in_clause"
        && clause.parent().is_some_and(|parent| {
            comprehension(parent)
                && children(parent)
                    .into_iter()
                    .find(|node| node.kind() == "for_in_clause")
                    == Some(clause)
        })
}

pub(crate) fn ungroup(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        let Some(inner) = children(node).first().copied() else {
            break;
        };
        node = inner;
    }
    node
}

pub(crate) fn literal_methods(file: &ParsedFile, node: Node<'_>) -> MethodSet {
    let expression = ungroup(node);
    if matches!(expression.kind(), "list" | "tuple" | "set") {
        if let Some(methods) = children(expression)
            .into_iter()
            .map(|item| literal_string(file, item))
            .collect::<Option<Vec<_>>>()
        {
            return MethodSet::known(methods.into_iter().map(|method| method.to_uppercase()));
        }
    }
    MethodSet::unknown(file.node_text(&node))
}

pub(crate) fn site(file: &ParsedFile, node: Node<'_>) -> Site {
    Site {
        file: file.path.clone(),
        line: node.start_position().row + 1,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}

pub(crate) fn attribute(node: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    Some((
        node.child_by_field_name("object")?,
        node.child_by_field_name("attribute")?,
    ))
}
