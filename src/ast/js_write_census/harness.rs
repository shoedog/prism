//! Test harness for the walker and order relation: parse a source, pick owners, find reads.

use super::order::Position;
use super::tables::Tables;
use super::walk::{census, Census, Site};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

/// Outermost callables in source order.
pub(super) fn owners<'p>(parsed: &'p ParsedFile, t: &Tables) -> Vec<Node<'p>> {
    let mut out = Vec::new();
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(n) = stack.pop() {
        if super::walk::is_callable(t, n.kind()) {
            out.push(n);
            continue;
        }
        let mut c = n.walk();
        let kids: Vec<_> = n.children(&mut c).collect();
        stack.extend(kids.into_iter().rev());
    }
    out
}

/// Parse `src` with exactly `lang` (cross-grammar rows name their grammar).
pub(super) fn parse_in(src: &str, lang: Language) -> ParsedFile {
    ParsedFile::parse("fixture", src, lang).expect("parses")
}

pub(super) struct Fx {
    pub parsed: ParsedFile,
    pub t: Tables,
}

impl Fx {
    /// JavaScript, explicitly (no grammar guessing).
    pub(super) fn new(src: &str) -> Fx {
        Fx::in_lang(src, Language::JavaScript)
    }

    pub(super) fn in_lang(src: &str, lang: Language) -> Fx {
        Fx {
            parsed: parse_in(src, lang),
            t: Tables::embedded(),
        }
    }

    /// `function f(a) { log(); <line> }` with the statement on its own line 3.
    pub(super) fn line(line: &str) -> Fx {
        Fx::line_in(Language::JavaScript, line)
    }

    pub(super) fn line_in(lang: Language, line: &str) -> Fx {
        Fx::in_lang(&format!("function f(a) {{\n  log();\n  {line}\n}}"), lang)
    }

    pub(super) fn first_census(&self) -> Census {
        let o = owners(&self.parsed, &self.t);
        census(&self.parsed, &self.t, o[0])
    }

    /// Reads of `base`: identifiers of that name that are neither a census target nor in the owner's
    /// own parameter list.
    pub(super) fn reads(&self, c: &Census, base: &str) -> Vec<(usize, Position)> {
        let o = owners(&self.parsed, &self.t)[0];
        let params = o
            .child_by_field_name("parameters")
            .map(|p| (p.start_byte(), p.end_byte()));
        let starts: Vec<usize> = c.sites.iter().map(|s| s.target_start).collect();
        let mut out = Vec::new();
        let mut stack = vec![o];
        while let Some(n) = stack.pop() {
            let mut cur = n.walk();
            let kids: Vec<_> = n.children(&mut cur).collect();
            stack.extend(kids.into_iter().rev());
            let in_params = params.is_some_and(|(a, b)| a <= n.start_byte() && n.end_byte() <= b);
            if n.kind() == "identifier"
                && self.parsed.node_text(&n) == base
                && !in_params
                && !starts.contains(&n.start_byte())
            {
                out.push((n.start_byte(), Position::of_read(n, &c.spans)));
            }
        }
        out.sort_by_key(|(b, _)| *b);
        out
    }
}

/// Counting-candidate writes of `base` (declarations of the owner's own parameters excluded).
pub(super) fn writes<'c>(c: &'c Census, base: &str) -> Vec<&'c Site> {
    c.sites
        .iter()
        .filter(|s| s.path.base == base && !s.region.own_params)
        .collect()
}
