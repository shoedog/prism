use crate::targets::inventory::FunctionBounds;
use prism::ast::ParsedFile;
use tree_sitter::Node;
use unicode_normalization::UnicodeNormalization;

pub(super) fn identifier(text: &str) -> String {
    text.nfkc().collect()
}

/// Python recognizes LF, CRLF and CR as line endings. Keep the raw source
/// alongside the parse view so every source span can be recovered verbatim.
pub(in crate::targets::inventory) struct Source<'a> {
    raw: &'a ParsedFile,
    normalized: Option<(ParsedFile, Vec<usize>)>,
}

impl<'a> Source<'a> {
    pub(in crate::targets::inventory) fn new(raw: &'a ParsedFile) -> Self {
        let normalized = raw.source.contains('\r').then(|| {
            let mut bytes = Vec::with_capacity(raw.source.len());
            let mut original_offsets = Vec::with_capacity(raw.source.len() + 1);
            let mut iter = raw.source.bytes().enumerate().peekable();
            while let Some((index, byte)) = iter.next() {
                original_offsets.push(index);
                if byte == b'\r' {
                    bytes.push(b'\n');
                    if iter.peek().is_some_and(|(_, next)| *next == b'\n') {
                        iter.next();
                    }
                } else {
                    bytes.push(byte);
                }
            }
            original_offsets.push(raw.source.len());
            let text = String::from_utf8(bytes).expect("newline normalization preserves UTF-8");
            let parsed = ParsedFile::parse(&raw.path, &text, raw.language)
                .expect("normalizing newlines preserves parser availability");
            (parsed, original_offsets)
        });
        Self { raw, normalized }
    }

    pub(in crate::targets::inventory) fn parsed(&self) -> &ParsedFile {
        self.normalized
            .as_ref()
            .map_or(self.raw, |(parsed, _)| parsed)
    }

    pub(in crate::targets::inventory) fn original_text(&self, node: Node<'_>) -> &str {
        let offset = |index| {
            self.normalized
                .as_ref()
                .map_or(index, |(_, map)| map[index])
        };
        &self.raw.source[offset(node.start_byte())..offset(node.end_byte())]
    }
}

pub(in crate::targets::inventory) fn children(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "comment" | "line_continuation"))
        .collect::<Vec<_>>()
        .into_iter()
}

pub(in crate::targets::inventory) fn ungroup(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        let mut inner = children(node);
        match (inner.next(), inner.next()) {
            (Some(child), None) => node = child,
            _ => break,
        }
    }
    node
}

pub(in crate::targets::inventory) fn outer_group(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node
        .parent()
        .filter(|parent| parent.kind() == "parenthesized_expression")
    {
        node = parent;
    }
    node
}

pub(in crate::targets::inventory) fn has_comma(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let has_comma = node.children(&mut cursor).any(|child| child.kind() == ",");
    has_comma
}

pub(in crate::targets::inventory) fn scalar_subscript(node: Node<'_>) -> Option<Node<'_>> {
    if has_comma(node) {
        return None;
    }
    node.child_by_field_name("subscript")
}

// The assignment grammar represents both grouping and unpacking as tuple_pattern.
pub(in crate::targets::inventory) fn ungroup_target(mut node: Node<'_>) -> Node<'_> {
    loop {
        node = ungroup(node);
        if node.kind() != "tuple_pattern" || has_comma(node) {
            return node;
        }
        let mut inner = children(node);
        match (inner.next(), inner.next()) {
            (Some(child), None) => node = child,
            _ => return node,
        }
    }
}

pub(super) fn bounds(parsed: &ParsedFile, node: Node<'_>) -> Option<FunctionBounds> {
    let name = identifier(parsed.node_text(&node.child_by_field_name("name")?));
    let end = last_token(node)?;
    Some(FunctionBounds {
        name,
        start_line: node.start_position().row + 1,
        end_line: end.end_position().row + 1,
    })
}

/// A bare comma-starred string can pull subsequent statements into an
/// erroneous expression (including the name and parameters of a later def).
/// Neutralize only the offending star token in a separate bounds parse,
/// retaining every line and column and leaving inventory call proof alone.
pub(super) fn recover_bounds_parse(parsed: &ParsedFile) -> Option<ParsedFile> {
    fn stars(node: Node<'_>, source: &str, repairs: &mut Vec<usize>) {
        if !node.has_error() || node.start_position().row == node.end_position().row {
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "*" && matches!(node.kind(), "ERROR" | "list_splat") {
                let byte = child.start_byte();
                let before = source[..byte].trim_end_matches([' ', '\t']);
                let after = source[child.end_byte()..].trim_start_matches([' ', '\t']);
                if before.ends_with(',') && after.starts_with(['"', '\'']) {
                    repairs.push(byte);
                }
            }
            stars(child, source, repairs);
        }
    }

    let mut repairs = Vec::new();
    stars(parsed.tree.root_node(), &parsed.source, &mut repairs);
    if repairs.is_empty() {
        return None;
    }
    let mut bytes = parsed.source.as_bytes().to_vec();
    for byte in repairs {
        bytes[byte] = b' ';
    }
    let input = String::from_utf8(bytes).expect("replacing ASCII preserves UTF-8");
    ParsedFile::parse(&parsed.path, &input, parsed.language).ok()
}

fn last_token(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_missing() || matches!(node.kind(), "comment" | "line_continuation") {
        return None;
    }
    let children = node.child_count();
    if children == 0 {
        return (node.end_byte() > node.start_byte()).then_some(node);
    }
    (0..children)
        .rev()
        .filter_map(|index| node.child(index))
        .find_map(last_token)
}

/// Normalize the structural dotted expression one identifier at a time.
/// Private names use the lexical class, including in nested function bodies.
pub(in crate::targets::inventory) fn name(text: &str, class: Option<&str>) -> String {
    let normalized = identifier(text);
    if let Some(class) = class {
        let class = class.trim_start_matches('_');
        if !class.is_empty() && normalized.starts_with("__") && !normalized.ends_with("__") {
            return format!("_{class}{normalized}");
        }
    }
    normalized
}

pub(in crate::targets::inventory) fn chain(
    parsed: &ParsedFile,
    node: Node<'_>,
    class: Option<&str>,
) -> Option<Vec<String>> {
    let node = ungroup(node);
    match node.kind() {
        "identifier" => Some(vec![name(parsed.node_text(&node), class)]),
        "attribute" => {
            let mut path = chain(parsed, node.child_by_field_name("object")?, class)?;
            let attribute = node.child_by_field_name("attribute")?;
            path.push(name(parsed.node_text(&attribute), class));
            Some(path)
        }
        _ => None,
    }
}
