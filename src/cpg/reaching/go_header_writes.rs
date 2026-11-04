//! Go root rebindings that carry no DefSite: `range` clauses, select receive
//! assignments and type-switch aliases. Contribution lane only; ordinary RD
//! never sees these.
use crate::ast::ParsedFile;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// Written on iteration entry only: the zero-iteration exit keeps the old value.
    Range { last_line: usize },
    /// Written when control reaches the anchor node itself.
    Anchor,
}

#[derive(Clone, Debug)]
pub(super) struct HeaderWrite {
    pub name: String,
    /// A byte at which the written binding is the visible one.
    pub probe: usize,
    pub line: usize,
    pub entry: Entry,
}

pub(super) fn collect(parsed: &ParsedFile, body: Node<'_>) -> Vec<HeaderWrite> {
    let mut out = Vec::new();
    walk(parsed, body, &mut out);
    out
}

fn names(
    parsed: &ParsedFile,
    left: Node<'_>,
    declares: bool,
    node_end: usize,
) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut push = |n: Node<'_>| {
        if n.kind() == "identifier" && parsed.node_text(&n) != "_" {
            let probe = if declares { node_end } else { n.start_byte() };
            out.push((parsed.node_text(&n).to_owned(), probe));
        }
    };
    if left.kind() == "expression_list" {
        let mut cursor = left.walk();
        for n in left.named_children(&mut cursor) {
            push(n);
        }
    } else {
        push(left);
    }
    out
}

fn walk(parsed: &ParsedFile, node: Node<'_>, out: &mut Vec<HeaderWrite>) {
    match node.kind() {
        "func_literal" | "function_declaration" | "method_declaration" => return,
        "range_clause" | "receive_statement" => {
            let mut cursor = node.walk();
            let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
            let entry = match node.parent() {
                Some(f) if node.kind() == "range_clause" => {
                    // The clause sits in a `for_statement`, possibly via `for_clause`-less header.
                    let mut owner = f;
                    while owner.kind() != "for_statement" {
                        match owner.parent() {
                            Some(p) => owner = p,
                            None => break,
                        }
                    }
                    Entry::Range {
                        last_line: owner.end_position().row + 1,
                    }
                }
                _ => Entry::Anchor,
            };
            if let Some(left) = node.child_by_field_name("left") {
                for (name, probe) in names(parsed, left, declares, node.end_byte()) {
                    out.push(HeaderWrite {
                        name,
                        probe,
                        line: node.start_position().row + 1,
                        entry,
                    });
                }
            }
        }
        "type_switch_statement" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                let mut cursor = node.walk();
                for clause in node.named_children(&mut cursor) {
                    if !matches!(clause.kind(), "type_case" | "default_case") {
                        continue;
                    }
                    let mut c = clause.walk();
                    let Some(colon) = clause.children(&mut c).find(|c| c.kind() == ":") else {
                        continue;
                    };
                    for (name, _) in names(parsed, alias, true, 0) {
                        out.push(HeaderWrite {
                            name,
                            probe: colon.end_byte(),
                            line: clause.start_position().row + 1,
                            entry: Entry::Anchor,
                        });
                    }
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(parsed, child, out);
    }
}
