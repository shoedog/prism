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
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::cell::RefCell;
use std::collections::HashMap;
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

/// Where the reads of one line-keyed reference identity lie.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Placement {
    /// Every read lies outside every region.
    Eager,
    /// Every read lies in this innermost region.
    Region(usize),
    /// Reads in different regions, or both eagerly and in a region: the use cannot be placed.
    Ambiguous,
}

impl Placement {
    fn merge(self, other: Placement) -> Placement {
        if self == other {
            self
        } else {
            Placement::Ambiguous
        }
    }
}

pub(super) struct JsRegions<'p> {
    parsed: &'p ParsedFile,
    kinds: Vec<RegionKind>,
    /// Sorted by start ascending, then end descending, then depth: ancestors precede descendants.
    spans: Vec<Span>,
    /// Every read of the owner (eager and in regions), sorted by line. Paths are built on demand.
    reads: Vec<Candidate>,
    /// Per queried line: the line-keyed reference identity (path, line) of each read on it with where
    /// its occurrences lie; empty when the line holds no region read.
    lines: RefCell<HashMap<usize, Vec<(AccessPath, Placement)>>>,
}

/// A read candidate: (1-indexed line, start byte, end byte, is a member expression).
pub(super) type Candidate = (usize, usize, usize, bool);

#[derive(Clone, Copy)]
struct Ctx {
    depth: usize,
}

struct Builder<'a, 'p, 't> {
    parsed: &'p ParsedFile,
    boundary: &'a [&'a str],
    kinds: Vec<RegionKind>,
    /// (start, end, region id, depth)
    raw: Vec<(usize, usize, usize, usize)>,
    owner: Node<'t>,
}

pub(super) fn js_regions<'p, 't>(parsed: &'p ParsedFile, func_node: Node<'t>) -> JsRegions<'p> {
    let boundary = parsed.language.callable_boundary_node_types();
    let mut b = Builder {
        parsed,
        boundary: &boundary,
        kinds: Vec::new(),
        raw: Vec::new(),
        owner: func_node,
    };
    b.children(func_node, Ctx { depth: 0 });
    b.finish()
}

impl<'a, 'p, 't> Builder<'a, 'p, 't> {
    fn finish(self) -> JsRegions<'p> {
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
        // One pass over the owner collects every read; a read's region is the innermost span holding
        // its byte, so reads in a parameter decorator's hole are eager (or the enclosing region's).
        let mut reads = Vec::new();
        if !spans.is_empty() {
            Collector::new(&mut reads).walk_children(self.owner);
            reads.sort_by_key(|c| c.0);
        }
        JsRegions {
            parsed: self.parsed,
            kinds: self.kinds,
            spans,
            reads,
            lines: RefCell::new(HashMap::new()),
        }
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

    fn enter(&mut self, kind: RegionKind, ranges: &[(usize, usize)], ctx: Ctx) -> Ctx {
        let id = self.kinds.len();
        self.kinds.push(kind);
        for &(start, end) in ranges {
            if start < end {
                self.raw.push((start, end, id, ctx.depth + 1));
            }
        }
        Ctx {
            depth: ctx.depth + 1,
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
        let mut cursor = list.walk();
        for param in list.children(&mut cursor) {
            let mut inner = param.walk();
            for part in param.children(&mut inner) {
                if part.kind() == "decorator" {
                    holes.push((part.start_byte(), part.end_byte()));
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
        let inner = self.enter(RegionKind::Param, &ranges, ctx);
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
                );
                self.visit(child, inner);
            } else {
                self.visit(child, ctx);
            }
        }
    }
}

impl JsRegions<'_> {
    /// The line-keyed reference identity of a use: by its full path when any read on the line (eager
    /// or in a region) spells it, otherwise by its base name (a member read of `o` is found by the
    /// token `o` whatever wraps it). `None` when the line holds no region read, or no read on it
    /// spells either. Reads of one key both eagerly and in a region are `Placement::Ambiguous`.
    pub(super) fn line_region(&self, path: &AccessPath, line: usize) -> Option<Placement> {
        let mut lines = self.lines.borrow_mut();
        let placements = lines
            .entry(line)
            .or_insert_with(|| self.line_placements(line));
        resolve_key(path, |wanted| {
            placements
                .iter()
                .find(|(p, _)| p == wanted)
                .map(|(_, v)| *v)
        })
    }

    /// Every read on `line` keyed by its path, empty when none of them lies in a region.
    fn line_placements(&self, line: usize) -> Vec<(AccessPath, Placement)> {
        let start = self.reads.partition_point(|c| c.0 < line);
        let on_line: Vec<(&Candidate, Placement)> = self.reads[start..]
            .iter()
            .take_while(|c| c.0 == line)
            .map(|c| {
                let placement = match self.innermost(c.1) {
                    Some(region) => Placement::Region(region),
                    None => Placement::Eager,
                };
                (c, placement)
            })
            .collect();
        if !on_line.iter().any(|(_, p)| *p != Placement::Eager) {
            return Vec::new();
        }
        let mut out: Vec<(AccessPath, Placement)> = Vec::new();
        for (candidate, placement) in on_line {
            let path = candidate_path(self.parsed, candidate);
            match out.iter_mut().find(|(p, _)| *p == path) {
                Some((_, seen)) => *seen = seen.merge(placement),
                None => out.push((path, placement)),
            }
        }
        out
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

/// The line-keyed identity lookup S1a and D3 share: the use's full path first, its base name only
/// when no read on the line spells the full path (a member read of `o` is found by the token `o`).
pub(super) fn resolve_key<T>(
    path: &AccessPath,
    find: impl Fn(&AccessPath) -> Option<T>,
) -> Option<T> {
    find(path).or_else(|| {
        (!path.is_simple())
            .then(|| find(&AccessPath::simple(&path.base)))
            .flatten()
    })
}

fn is_field_definition(kind: &str) -> bool {
    matches!(kind, "field_definition" | "public_field_definition")
}

fn candidate(node: Node<'_>) -> Option<Candidate> {
    let member = match node.kind() {
        "identifier" | "shorthand_property_identifier" | "this" => false,
        "member_expression" => true,
        _ => return None,
    };
    Some((
        node.start_position().row + 1,
        node.start_byte(),
        node.end_byte(),
        member,
    ))
}

/// A member read as the path the DFG gives it (`ParsedFile::bounded_member_path_at`, the bounded
/// normalizer the member index uses, so `(o as any).x` keys as `o.x`); an identifier as a simple path.
pub(super) fn candidate_path(
    parsed: &ParsedFile,
    &(_, start, end, member): &Candidate,
) -> AccessPath {
    if member {
        parsed
            .bounded_member_path_at(start, end)
            .unwrap_or_else(|| AccessPath::from_expr(&parsed.source[start..end]))
    } else {
        AccessPath::simple(&parsed.source[start..end])
    }
}

fn children_with_fields(node: Node<'_>) -> Vec<(Node<'_>, Option<&'static str>)> {
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
    kids
}

/// Collects the runtime reads below a node: every identifier and member read, through every
/// evaluated child (receivers, call arguments, indices, wrappers). Not reads: property names, type
/// positions, and the names a callable binds (its formals, a lone
/// arrow parameter, a function's own name); default values and computed keys inside parameter
/// patterns are reads.
pub(super) struct Collector<'c> {
    out: &'c mut Vec<Candidate>,
}

impl<'c> Collector<'c> {
    pub(super) fn new(out: &'c mut Vec<Candidate>) -> Self {
        Self { out }
    }

    pub(super) fn walk_children(&mut self, node: Node<'_>) {
        let kind = node.kind();
        // After the `as` / `satisfies` operator the child is a type: erased (what
        // `Language::is_erased_type_boundary` decides per node, without its parent walk).
        let mut after_type_operator = false;
        let mut cursor = node.walk();
        if !cursor.goto_first_child() {
            return;
        }
        loop {
            let child = cursor.node();
            let field = cursor.field_name();
            if matches!(kind, "as_expression" | "satisfies_expression")
                && matches!(child.kind(), "as" | "satisfies")
            {
                after_type_operator = true;
            } else if !after_type_operator && !skip_binding_field(kind, field) {
                self.walk(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    fn walk(&mut self, node: Node<'_>) {
        if is_erased_type_kind(node.kind()) {
            return;
        }
        if node.kind() == "formal_parameters" {
            self.formals(node);
            return;
        }
        if let Some(c) = candidate(node) {
            self.out.push(c);
        }
        self.walk_children(node);
    }

    /// A parameter list: the names it binds are not reads, its defaults and computed keys are.
    fn formals(&mut self, list: Node<'_>) {
        for param in list.children(&mut list.walk()) {
            if matches!(param.kind(), "required_parameter" | "optional_parameter") {
                for (part, field) in children_with_fields(param) {
                    match field {
                        Some("pattern") => self.binding(part),
                        _ => self.walk(part),
                    }
                }
            } else {
                self.binding(param);
            }
        }
    }

    /// A binding pattern: only default values and computed keys inside it are reads.
    fn binding(&mut self, node: Node<'_>) {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" | "this" => {}
            "assignment_pattern" | "object_assignment_pattern" => {
                for (part, field) in children_with_fields(node) {
                    match field {
                        Some("right") => self.walk(part),
                        _ => self.binding(part),
                    }
                }
            }
            "pair_pattern" => {
                for (part, field) in children_with_fields(node) {
                    match field {
                        Some("key") if part.kind() == "computed_property_name" => self.walk(part),
                        Some("key") => {}
                        _ => self.binding(part),
                    }
                }
            }
            "object_pattern" | "array_pattern" | "rest_pattern" => {
                for (part, _) in children_with_fields(node) {
                    self.binding(part);
                }
            }
            _ => self.walk(node),
        }
    }
}

/// The node kinds `Language::is_erased_type_boundary` decides by kind alone (TS/TSX only; JS has none).
fn is_erased_type_kind(kind: &str) -> bool {
    matches!(
        kind,
        "type_query"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "type_alias_declaration"
    )
}

/// Fields that hold a name the callable binds: a lone arrow parameter, a function's own name.
fn skip_binding_field(kind: &str, field: Option<&str>) -> bool {
    matches!(
        (kind, field),
        ("arrow_function", Some("parameter"))
            | (
                "function_expression"
                    | "generator_function"
                    | "function_declaration"
                    | "generator_function_declaration",
                Some("name")
            )
    )
}

#[cfg(test)]
#[path = "capture_regions_tests.rs"]
mod tests;
