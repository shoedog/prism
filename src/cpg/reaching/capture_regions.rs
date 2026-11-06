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
use std::collections::{BTreeMap, BTreeSet};
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

pub(super) struct JsRegions {
    kinds: Vec<RegionKind>,
    /// Sorted by start ascending, then end descending, then depth: ancestors precede descendants.
    spans: Vec<Span>,
    /// The line-keyed reference identity (path, 1-indexed line) of every read on a line that holds a
    /// region read, eager reads of those lines included, with where its occurrences lie.
    references: BTreeMap<(AccessPath, usize), Placement>,
}

/// A read candidate: (1-indexed line, start byte, end byte, is a member expression).
type Candidate = (usize, usize, usize, bool);

#[derive(Clone, Copy)]
struct Ctx {
    depth: usize,
    /// Inside a region whose reference identities were already collected.
    covered: bool,
}

struct Builder<'a, 'p, 't> {
    parsed: &'p ParsedFile,
    boundary: &'a [&'a str],
    kinds: Vec<RegionKind>,
    /// (start, end, region id, depth)
    raw: Vec<(usize, usize, usize, usize)>,
    occurrences: Vec<Candidate>,
    owner: Node<'t>,
}

pub(super) fn js_regions<'t>(parsed: &ParsedFile, func_node: Node<'t>) -> JsRegions {
    let boundary = parsed.language.callable_boundary_node_types();
    let mut b = Builder {
        parsed,
        boundary: &boundary,
        kinds: Vec::new(),
        raw: Vec::new(),
        occurrences: Vec::new(),
        owner: func_node,
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

impl<'a, 'p, 't> Builder<'a, 'p, 't> {
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
        let mut region_lines = BTreeSet::new();
        for candidate in &self.occurrences {
            let placement = match regions.innermost(candidate.1) {
                Some(region) => Placement::Region(region),
                None => Placement::Eager,
            };
            region_lines.insert(candidate.0);
            regions.note(self.parsed, candidate, placement);
        }
        // The same lines' eager reads: a use of an eager read must not be placed in a region by a
        // name a region also spells.
        if !region_lines.is_empty() {
            let mut eager = Vec::new();
            let mut collector = Collector::new(self.parsed, &[], &mut eager);
            collector.prune = Some(&regions);
            collector.walk_children(self.owner);
            for candidate in eager.iter().filter(|c| region_lines.contains(&c.0)) {
                regions.note(self.parsed, candidate, Placement::Eager);
            }
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
                Collector::new(self.parsed, holes, &mut self.occurrences).walk(*node);
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
        // A lone arrow parameter (`x => ..`) is a binding, not a read.
        let collect: &[Node<'_>] = if list.kind() == "formal_parameters" {
            &[list]
        } else {
            &[]
        };
        let inner = self.enter(RegionKind::Param, &ranges, ctx, collect, &holes);
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
    fn note(&mut self, parsed: &ParsedFile, candidate: &Candidate, placement: Placement) {
        let key = (candidate_path(parsed, candidate), candidate.0);
        self.references
            .entry(key)
            .and_modify(|seen| *seen = seen.merge(placement))
            .or_insert(placement);
    }

    /// The line-keyed reference identity of a use: by its full path when any read on the line (eager
    /// or in a region) spells it, otherwise by its base name (a member read of `o` is found by the
    /// token `o` whatever wraps it). `None` when no read on a line that holds a region read spells
    /// either. Reads of one key both eagerly and in a region are `Placement::Ambiguous`.
    pub(super) fn line_region(&self, path: &AccessPath, line: usize) -> Option<Placement> {
        if let Some(found) = self.references.get(&(path.clone(), line)) {
            return Some(*found);
        }
        if path.is_simple() {
            return None;
        }
        self.references
            .get(&(AccessPath::simple(&path.base), line))
            .copied()
    }

    /// End byte of the innermost span holding `byte`.
    fn containing_end(&self, byte: usize) -> Option<usize> {
        let mut at = self
            .spans
            .partition_point(|s| s.start <= byte)
            .checked_sub(1);
        while let Some(i) = at {
            let span = &self.spans[i];
            if byte < span.end {
                return Some(span.end);
            }
            at = span.parent;
        }
        None
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
fn candidate_path(parsed: &ParsedFile, &(_, start, end, member): &Candidate) -> AccessPath {
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
/// positions, a subtree that is exactly a hole, and the names a callable binds (its formals, a lone
/// arrow parameter, a function's own name); default values and computed keys inside parameter
/// patterns are reads.
struct Collector<'c, 'r> {
    parsed: &'c ParsedFile,
    holes: &'c [(usize, usize)],
    out: &'c mut Vec<Candidate>,
    /// When set, a subtree lying wholly inside a region is skipped (the eager walk).
    prune: Option<&'r JsRegions>,
}

impl<'c, 'r> Collector<'c, 'r> {
    fn new(
        parsed: &'c ParsedFile,
        holes: &'c [(usize, usize)],
        out: &'c mut Vec<Candidate>,
    ) -> Self {
        Self {
            parsed,
            holes,
            out,
            prune: None,
        }
    }

    fn walk_children(&mut self, node: Node<'_>) {
        for (child, field) in children_with_fields(node) {
            if skip_binding_field(node.kind(), field) {
                continue;
            }
            self.walk(child);
        }
    }

    fn walk(&mut self, node: Node<'_>) {
        if !self.holes.is_empty() && self.holes.contains(&(node.start_byte(), node.end_byte())) {
            return;
        }
        if self.parsed.language.is_erased_type_boundary(node) {
            return;
        }
        if let Some(regions) = self.prune {
            if regions
                .containing_end(node.start_byte())
                .is_some_and(|end| node.end_byte() <= end)
            {
                return;
            }
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
