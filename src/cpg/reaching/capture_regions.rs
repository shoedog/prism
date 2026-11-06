//! #660 S1a: the deferred regions of a JS/TS/TSX owner.
//!
//! A region is code that runs later than the owner's straight-line statements: a nested callable's
//! parameter list (defaults run at call time), its body, and the value of an instance field
//! (runs at `new C()`). Everything else is eager: a method's name and decorators, a class's
//! decorators, static field initializers, static blocks, and TS parameter decorators, which run in
//! the scope enclosing the callable (design §4.4). A callable inside an eager position is still its
//! own region. Regions nest; `innermost` answers which one holds a byte.
//!
//! Only JS-family owners build this: Python evaluates parameter defaults eagerly at definition time.
use super::capture::{is_field_access_kind, leftmost_receiver_identifier};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RegionKind {
    /// A nested callable's parameter list (defaults, patterns), parameter decorators excluded.
    Param,
    /// A nested callable's body.
    Body,
    /// The value of an instance field.
    Field,
}

/// One byte range of a region. A region owns several when parameter decorators split its list.
struct Span {
    start: usize,
    end: usize,
    region: usize,
    parent: Option<usize>,
}

pub(super) struct JsRegions {
    kinds: Vec<RegionKind>,
    /// Sorted by start ascending, then end descending, then depth: ancestors precede descendants.
    spans: Vec<Span>,
    /// The line-keyed reference identity (path, 1-indexed line) of every identifier in a region, with
    /// the innermost region of its occurrences: `None` when occurrences lie in different regions.
    references: BTreeMap<(AccessPath, usize), Option<usize>>,
}

/// An identifier occurrence in a region: (path, 1-indexed line, start byte).
type Occurrence = (AccessPath, usize, usize);

#[derive(Clone, Copy)]
struct Ctx {
    depth: usize,
    /// Inside a region whose reference identities were already collected.
    covered: bool,
}

struct Builder<'a, 'p> {
    parsed: &'p ParsedFile,
    boundary: &'a [&'a str],
    kinds: Vec<RegionKind>,
    /// (start, end, region id, depth)
    raw: Vec<(usize, usize, usize, usize)>,
    occurrences: Vec<Occurrence>,
}

pub(super) fn js_regions(parsed: &ParsedFile, func_node: Node<'_>) -> JsRegions {
    let boundary = parsed.language.callable_boundary_node_types();
    let mut b = Builder {
        parsed,
        boundary: &boundary,
        kinds: Vec::new(),
        raw: Vec::new(),
        occurrences: Vec::new(),
    };
    b.children(
        func_node,
        Ctx {
            depth: 0,
            covered: false,
        },
    );
    b.finish()
}

impl<'a, 'p> Builder<'a, 'p> {
    fn finish(self) -> JsRegions {
        let mut raw = self.raw;
        raw.sort_by_key(|&(start, end, _, depth)| (start, std::cmp::Reverse(end), depth));
        let mut spans: Vec<Span> = Vec::with_capacity(raw.len());
        let mut stack: Vec<usize> = Vec::new();
        for (start, end, region, _) in raw {
            while stack.last().is_some_and(|&top| spans[top].end <= start) {
                stack.pop();
            }
            let parent = stack.last().copied();
            spans.push(Span {
                start,
                end,
                region,
                parent,
            });
            stack.push(spans.len() - 1);
        }
        let mut regions = JsRegions {
            kinds: self.kinds,
            spans,
            references: BTreeMap::new(),
        };
        for (path, line, byte) in self.occurrences {
            let region = regions.innermost(byte);
            regions
                .references
                .entry((path, line))
                .and_modify(|seen| {
                    if *seen != region {
                        *seen = None;
                    }
                })
                .or_insert(region);
        }
        regions
    }

    fn children(&mut self, node: Node<'_>, ctx: Ctx) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(child, ctx);
        }
    }

    fn visit(&mut self, node: Node<'_>, ctx: Ctx) {
        if self.boundary.contains(&node.kind()) {
            self.callable(node, ctx);
        } else if is_field_definition(node.kind()) {
            self.field(node, ctx);
        } else {
            self.children(node, ctx);
        }
    }

    fn enter(
        &mut self,
        kind: RegionKind,
        ranges: &[(usize, usize)],
        ctx: Ctx,
        collect: &[Node<'_>],
        holes: &[(usize, usize)],
    ) -> Ctx {
        let id = self.kinds.len();
        self.kinds.push(kind);
        for &(start, end) in ranges {
            if start < end {
                self.raw.push((start, end, id, ctx.depth + 1));
            }
        }
        if !ctx.covered {
            for node in collect {
                collect_occurrences(self.parsed, *node, holes, &mut self.occurrences);
            }
        }
        Ctx {
            depth: ctx.depth + 1,
            covered: true,
        }
    }

    /// A nested callable: its parameter list(s) and body are regions, the rest is eager.
    fn callable(&mut self, node: Node<'_>, ctx: Ctx) {
        let mut cursor = node.walk();
        let mut kids = Vec::new();
        if cursor.goto_first_child() {
            loop {
                kids.push((cursor.node(), cursor.field_name()));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        for (child, field) in kids {
            match field {
                Some("parameters" | "parameter") => self.params(child, ctx),
                Some("body") => {
                    let inner = self.enter(
                        RegionKind::Body,
                        &[(child.start_byte(), child.end_byte())],
                        ctx,
                        &[child],
                        &[],
                    );
                    self.visit(child, inner);
                }
                _ => self.visit(child, ctx),
            }
        }
    }

    /// A parameter list. TS parameter decorators are eager, so they are holes in its range and are
    /// walked in the enclosing context.
    fn params(&mut self, list: Node<'_>, ctx: Ctx) {
        let mut holes: Vec<(usize, usize)> = Vec::new();
        let mut decorators: Vec<Node<'_>> = Vec::new();
        let mut cursor = list.walk();
        for param in list.children(&mut cursor) {
            let mut inner = param.walk();
            for part in param.children(&mut inner) {
                if part.kind() == "decorator" {
                    holes.push((part.start_byte(), part.end_byte()));
                    decorators.push(part);
                }
            }
        }
        holes.sort_unstable();
        let mut ranges = Vec::new();
        let mut at = list.start_byte();
        for &(start, end) in &holes {
            ranges.push((at, start));
            at = end;
        }
        ranges.push((at, list.end_byte()));
        let inner = self.enter(RegionKind::Param, &ranges, ctx, &[list], &holes);
        let mut cursor = list.walk();
        for param in list.children(&mut cursor) {
            let mut parts = param.walk();
            for part in param.children(&mut parts) {
                let eager = part.kind() == "decorator";
                self.visit(part, if eager { ctx } else { inner });
            }
        }
    }

    /// An instance field's value is deferred; a static field's, its key and its decorators are eager.
    fn field(&mut self, node: Node<'_>, ctx: Ctx) {
        let mut cursor = node.walk();
        let is_static = node.children(&mut cursor).any(|k| k.kind() == "static");
        let mut kids = Vec::new();
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                kids.push((cursor.node(), cursor.field_name()));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        for (child, field) in kids {
            if field == Some("value") && !is_static {
                let inner = self.enter(
                    RegionKind::Field,
                    &[(child.start_byte(), child.end_byte())],
                    ctx,
                    &[child],
                    &[],
                );
                self.visit(child, inner);
            } else {
                self.visit(child, ctx);
            }
        }
    }
}

impl JsRegions {
    /// The line-keyed reference identity: `None` when no identifier of a region spells (path, line);
    /// otherwise the one innermost region holding every such occurrence, `Some(None)` when they lie
    /// in different regions (the use cannot be placed).
    pub(super) fn line_region(&self, path: &AccessPath, line: usize) -> Option<Option<usize>> {
        self.references.get(&(path.clone(), line)).copied()
    }

    /// The innermost region holding `byte`, if any.
    pub(super) fn innermost(&self, byte: usize) -> Option<usize> {
        let mut at = self
            .spans
            .partition_point(|s| s.start <= byte)
            .checked_sub(1);
        while let Some(i) = at {
            let span = &self.spans[i];
            if byte < span.end {
                return Some(span.region);
            }
            at = span.parent;
        }
        None
    }

    /// Whether `byte` lies in `region`, directly or inside a region nested in it.
    pub(super) fn within(&self, byte: usize, region: usize) -> bool {
        let mut at = self
            .spans
            .partition_point(|s| s.start <= byte)
            .checked_sub(1);
        while let Some(i) = at {
            let span = &self.spans[i];
            if byte < span.end && span.region == region {
                return true;
            }
            at = span.parent;
        }
        false
    }

    pub(super) fn kind(&self, region: usize) -> RegionKind {
        self.kinds[region]
    }
}

fn is_field_definition(kind: &str) -> bool {
    matches!(kind, "field_definition" | "public_field_definition")
}

/// The identifier reference identities below `node` with their start bytes, as
/// `capture::collect_reference_identities` spells them, skipping any subtree that is exactly a hole.
fn collect_occurrences(
    parsed: &ParsedFile,
    node: Node<'_>,
    holes: &[(usize, usize)],
    out: &mut Vec<Occurrence>,
) {
    if !holes.is_empty() && holes.contains(&(node.start_byte(), node.end_byte())) {
        return;
    }
    let line = |n: &Node<'_>| n.start_position().row + 1;
    if is_field_access_kind(node.kind()) {
        out.push((
            AccessPath::from_expr(parsed.node_text(&node)),
            line(&node),
            node.start_byte(),
        ));
        if let Some(base) = leftmost_receiver_identifier(parsed, node) {
            out.push((
                AccessPath::simple(parsed.node_text(&base)),
                line(&base),
                base.start_byte(),
            ));
        }
        return;
    }
    if parsed.language.is_identifier_node(node.kind()) {
        out.push((
            AccessPath::simple(parsed.node_text(&node)),
            line(&node),
            node.start_byte(),
        ));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_occurrences(parsed, child, holes, out);
    }
}

#[cfg(test)]
#[path = "capture_regions_tests.rs"]
mod tests;
