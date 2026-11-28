//! Reads one Go file with the prism core parser and classifies its `@Router` directives.

use super::directive::{self, Directive};
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Debug)]
pub(super) struct Decl {
    pub name: String,
    pub receiver: Option<String>,
    pub receiver_type: Option<String>,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub bodyless: bool,
}

#[derive(Debug)]
pub(super) enum Kind {
    /// A well-formed directive. `handler` indexes `Scan::decls` when attached.
    Directive(Directive),
    /// A block comment that mentions `@Router`.
    Block,
}

#[derive(Debug)]
pub(super) struct Found {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub raw: String,
    pub kind: Kind,
    pub handler: Option<usize>,
    pub unattached: Option<&'static str>,
}

#[derive(Debug, Default)]
pub(super) struct Scan {
    pub package: Option<String>,
    pub generated: bool,
    pub has_errors: bool,
    pub decls: Vec<Decl>,
    pub found: Vec<Found>,
}

pub(super) fn scan(path: &str, source: &str) -> Option<Scan> {
    let parsed = ParsedFile::parse(path, source, Language::Go).ok()?;
    let root = parsed.tree.root_node();
    let mut out = Scan {
        has_errors: parsed.parse_error_count > 0,
        ..Scan::default()
    };
    let mut package_start = usize::MAX;
    let mut attached: BTreeMap<usize, usize> = BTreeMap::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "package_clause" => {
                package_start = child.start_byte();
                out.package = child
                    .named_child(0)
                    .map(|name| text(name, source).to_string());
            }
            "function_declaration" | "method_declaration" => {
                if let Some(decl) = declaration(child, source) {
                    out.decls.push(decl);
                    let index = out.decls.len() - 1;
                    for comment in doc_run(child, source) {
                        attached.insert(comment.id(), index);
                    }
                }
            }
            _ => {}
        }
    }
    let mut comments = Vec::new();
    collect_comments(root, &mut comments);
    for comment in comments {
        let (start, end) = (comment.start_byte(), comment.end_byte());
        let raw = text(comment, source);
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let end = start + raw.len().min(end - start);
        if end <= package_start && directive::is_generated_header(raw) {
            out.generated = true;
        }
        let kind = if raw.starts_with("/*") {
            directive::mentions_in_block(raw).then_some(Kind::Block)
        } else {
            directive::recognise(raw).map(Kind::Directive)
        };
        let Some(kind) = kind else { continue };
        let handler = attached.get(&comment.id()).copied();
        let unattached = match handler {
            Some(_) => None,
            None if !line_prefix(source, start).trim().is_empty() => Some("trailing_comment"),
            None if inside_body(comment) => Some("inside_function_body"),
            None => Some("not_attached_to_function"),
        };
        out.found.push(Found {
            start,
            end,
            line: comment.start_position().row + 1,
            raw: raw.into(),
            kind,
            handler,
            unattached,
        });
    }
    Some(out)
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

fn line_prefix(source: &str, start: usize) -> &str {
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    &source[line_start..start]
}

fn inside_body(node: Node<'_>) -> bool {
    std::iter::successors(node.parent(), |n| n.parent()).any(|n| n.kind() == "block")
}

fn collect_comments<'t>(root: Node<'t>, out: &mut Vec<Node<'t>>) {
    let mut cursor = root.walk();
    let mut descend = true;
    loop {
        let node = cursor.node();
        if descend && node.kind() == "comment" {
            out.push(node);
        }
        if descend && cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            descend = true;
            continue;
        }
        if !cursor.goto_parent() {
            return;
        }
        descend = false;
    }
}

/// A declaration whose header (everything but the body) is error-free and named.
fn declaration(node: Node<'_>, source: &str) -> Option<Decl> {
    let name = node.child_by_field_name("name")?;
    if name.is_missing() || name.is_error() {
        return None;
    }
    let body = node.child_by_field_name("body");
    let mut cursor = node.walk();
    let header_broken = node
        .children(&mut cursor)
        .filter(|child| Some(child.id()) != body.map(|b| b.id()))
        .any(|child| child.is_error() || child.is_missing() || child.has_error());
    if header_broken {
        return None;
    }
    let receiver = node
        .child_by_field_name("receiver")
        .and_then(|list| list.named_child(0))
        .and_then(|param| param.child_by_field_name("type"))
        .map(|ty| text(ty, source).to_string());
    let receiver_type = receiver.as_deref().map(|ty| {
        let ty = ty.trim_start_matches(['*', ' ', '(']);
        ty.split(|c: char| c == '[' || c == ')' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_string()
    });
    Some(Decl {
        name: text(name, source).into(),
        receiver,
        receiver_type,
        start: node.start_byte(),
        end: node.end_byte(),
        line: node.start_position().row + 1,
        bodyless: body.is_none(),
    })
}

/// The `//` line comments that end on the row above `decl`, each alone on its line.
fn doc_run<'t>(decl: Node<'t>, source: &str) -> Vec<Node<'t>> {
    let mut run = Vec::new();
    let mut next_row = decl.start_position().row;
    let mut sibling = decl.prev_named_sibling();
    while let Some(comment) = sibling.filter(|n| n.kind() == "comment") {
        let start = comment.start_position();
        if start.row + 1 != next_row
            || !line_prefix(source, comment.start_byte()).trim().is_empty()
            || !text(comment, source).starts_with("//")
        {
            break;
        }
        run.push(comment);
        next_row = start.row;
        sibling = comment.prev_named_sibling();
    }
    run
}
