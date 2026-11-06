//! The table-driven write walk of one callable owner (#660 S0, DESIGN.md 4.4 and 7.1).
//!
//! Every decision comes from `table.tsv` (slot roles), `targets.tsv` (what a kind does in a target
//! position), `leaves.tsv` (NAME tokens for the escape precondition) and `order.tsv` (callables are the
//! `deferred` kinds). The walk reports census sites; it records nothing and has no caller yet.

use super::identity::{ident, Ident, Path};
use super::tables::Tables;
use crate::ast::{ParsedFile, StatementSpan};
use tree_sitter::Node;

/// How a site writes its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// Plain write (`=`, a pattern element, a `for` head without a declaration).
    Write,
    /// Reads then writes (`+=`, `++`).
    ReadWrite,
    /// `delete` of a member or subscript path.
    Delete,
    /// A binding declaration (`DECL:<kind>` role).
    Declare,
}

/// The write construct a site is positioned at: its moment is the construct's end byte.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Construct<'t> {
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
    node: Node<'t>,
}

/// Region flags of a site, relative to the owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Region {
    /// In the owner's own parameter list.
    pub own_params: bool,
    /// Inside a nested callable (its parameter list included).
    pub nested: bool,
    /// In the parameter list of a nested callable.
    pub nested_params: bool,
    /// In an instance field initializer value (deferred).
    pub instance_field: bool,
    /// In the body of a `with` statement.
    pub with_body: bool,
}

/// Why an owner (or one of its targets) is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `ERROR` or `MISSING` node in the owner's subtree.
    Parse,
    /// A NAME token containing `\` in the owner's subtree.
    Escape,
    /// A `REFUSE` or unclassified kind in a write position.
    Target(String),
}

impl Refusal {
    pub(crate) fn label(&self) -> String {
        match self {
            Refusal::Parse => "owner-refuse:parse".to_string(),
            Refusal::Escape => "owner-refuse:escape".to_string(),
            Refusal::Target(k) => format!("owner-refuse:target:{k}"),
        }
    }
}

/// One census site: a write or declaration of `path`.
#[derive(Clone, Debug)]
pub(crate) struct Site {
    pub path: Path,
    /// 1-indexed line of the target node.
    pub line: usize,
    /// Start byte of the target node (wrappers peeled).
    pub target_start: usize,
    pub form: Form,
    /// `DECL:` kind for `Form::Declare` sites, empty otherwise.
    pub decl: String,
    /// False for `var a;` and other declarations that store no value.
    pub initialized: bool,
    pub construct_kind: Option<&'static str>,
    /// Write moment: the construct's end byte, or the target's end when there is no construct.
    pub moment: usize,
    /// Start byte of the write construct (tie-break: the larger construct writes later).
    pub construct_start: Option<usize>,
    /// A `for (x of ..)` or `for (x in ..)` head target without a declaration.
    pub for_head: bool,
    pub region: Region,
    /// Kinds from the construct (or target) up to the mapped statement span; `None` when unmapped.
    pub chain: Option<Vec<&'static str>>,
    /// The mapped statement span `(start_byte, end_byte)`.
    pub span: Option<(usize, usize)>,
}

/// The census of one owner.
#[derive(Debug, Default)]
pub(crate) struct Census {
    pub refusals: Vec<Refusal>,
    pub sites: Vec<Site>,
    /// The owner's statement spans `(start_byte, end_byte)` the sites were mapped through.
    pub spans: Vec<(usize, usize)>,
}

impl Census {
    pub(crate) fn refused(&self) -> bool {
        !self.refusals.is_empty()
    }
}

/// Callable kinds: exactly the `deferred` rows of `order.tsv`.
pub(crate) fn is_callable(t: &Tables, kind: &str) -> bool {
    t.order_class(kind) == Some("deferred")
}

fn text<'s>(src: &'s [u8], n: Node<'_>) -> &'s str {
    std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("")
}

/// Owner-level preconditions (DESIGN.md 4.4): parse quality and escape-free NAME tokens.
pub(crate) fn precondition(t: &Tables, owner: Node<'_>, src: &[u8]) -> Option<Refusal> {
    if owner.has_error() {
        return Some(Refusal::Parse);
    }
    let mut cursor = owner.walk();
    let mut descended = true;
    loop {
        let n = cursor.node();
        // Assignment-form `using` (TS grammars) is a declaration this census does not model.
        if descended
            && !n.is_named()
            && n.kind() == "using"
            && n.parent()
                .is_some_and(|p| p.kind() == "assignment_expression")
        {
            return Some(Refusal::Target("using".to_string()));
        }
        if descended
            && n.child_count() == 0
            && n.is_named()
            && t.is_name_token(n.kind())
            && text(src, n).contains('\\')
        {
            return Some(Refusal::Escape);
        }
        if descended && cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            descended = true;
            continue;
        }
        loop {
            if !cursor.goto_parent() || cursor.node().id() == owner.id() {
                return None;
            }
            if cursor.goto_next_sibling() {
                descended = true;
                break;
            }
        }
    }
}

/// Run the walk over `owner` (a callable node of `parsed`), every site mapped (E18, node chains).
pub(crate) fn census(parsed: &ParsedFile, t: &Tables, owner: Node<'_>) -> Census {
    census_with(parsed, t, owner, true)
}

/// The walk with site mapping optional: `map == false` leaves every site's `chain` and `span` unset
/// (production wiring maps only the few sites it needs, from `Census::spans`).
pub(crate) fn census_with(parsed: &ParsedFile, t: &Tables, owner: Node<'_>, map: bool) -> Census {
    let src = parsed.source.as_bytes();
    let mut out = Census::default();
    if let Some(r) = precondition(t, owner, src) {
        out.refusals.push(r);
        return out;
    }
    let mut spans = parsed.statement_spans_in_function(&owner);
    if let Some(s) = crate::cfg::js_single::expression_body_statement(parsed, &owner, spans.len()) {
        spans = vec![s];
    }
    out.spans = spans.iter().map(|s| (s.start_byte, s.end_byte)).collect();
    let mut w = Walker {
        t,
        src,
        owner,
        spans,
        out,
        param_outer: Vec::new(),
        mapping: map,
    };
    w.children(owner, Ctx::Read, Region::default(), None);
    w.out
}

#[derive(Clone, Copy)]
enum Ctx<'a> {
    Read,
    Target(Form),
    Bind(&'a str, bool),
    Skip,
}

struct Walker<'a, 't> {
    t: &'a Tables,
    src: &'a [u8],
    owner: Node<'t>,
    spans: Vec<StatementSpan>,
    out: Census,
    /// Regions enclosing the callables whose parameter lists are being walked (innermost last).
    param_outer: Vec<Region>,
    /// Compute each site's E18 mapping (`chain`, `span`) as it is emitted.
    mapping: bool,
}

/// The operand of a wrapper: the child a target context flows into.
pub(super) fn wrap_operand<'t>(n: Node<'t>) -> Option<Node<'t>> {
    let mut cursor = n.walk();
    let kids: Vec<Node<'t>> = n
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "comment")
        .collect();
    match n.kind() {
        "type_assertion" => kids.last().copied(),
        "as_expression" | "satisfies_expression" => kids.first().copied(),
        _ if kids.len() == 1 => kids.first().copied(),
        _ => None,
    }
}

impl<'a, 't> Walker<'a, 't> {
    fn construct(n: Node<'t>) -> Construct<'t> {
        Construct {
            kind: n.kind(),
            start: n.start_byte(),
            end: n.end_byte(),
            node: n,
        }
    }

    /// Visit `n` in context `ctx`.
    fn visit(&mut self, n: Node<'t>, ctx: Ctx<'a>, rg: Region, cons: Option<Construct<'t>>) {
        let kind = n.kind();
        // Trivia (`order.tsv`: `comment`, a leaf) can sit between the children of a pattern, a `for`
        // head or any other target position; it neither writes nor refuses (S1b-1: 856 excalidraw
        // edges were refused through `owner-refuse:target:comment` before this).
        if kind == "comment" {
            return;
        }
        let form = match ctx {
            Ctx::Read => None,
            Ctx::Skip => return,
            Ctx::Target(f) => Some(f),
            Ctx::Bind(..) => Some(Form::Declare),
        };
        let Some(form) = form else {
            return self.children(n, ctx, rg, cons);
        };
        let deleting = form == Form::Delete;
        match self.t.targets.get(kind).map(|r| r.action.as_str()) {
            Some("NAME") => {
                // `type_identifier` binds only as a declaration name; `delete x` writes nothing.
                if (kind == "type_identifier" && form != Form::Declare) || deleting {
                    return;
                }
                let path = Path {
                    base: text(self.src, n).to_string(),
                    comps: Vec::new(),
                };
                self.emit(n, path, ctx, rg, cons);
            }
            Some("PATH") if form != Form::Declare => {
                match ident(self.t, self.src, n, true) {
                    Ident::Path(path) => self.emit(n, path, ctx, rg, cons),
                    Ident::NoWrite => {}
                    Ident::Refuse(k) => self.refuse(k),
                }
                // The object and index are reads.
                self.children(n, Ctx::Read, rg, cons);
            }
            Some("PATTERN" | "WRAP" | "PARAM" | "IMPORTS" | "CONTAINER") => {
                self.children(n, ctx, rg, cons)
            }
            Some("TYPE" | "NONE") => {}
            // A refused or unclassified `delete` operand writes nothing, but it is still evaluated.
            _ if deleting => self.children(n, Ctx::Read, rg, cons),
            _ => self.refuse(kind.to_string()),
        }
    }

    fn refuse(&mut self, kind: String) {
        let r = Refusal::Target(kind);
        if !self.out.refusals.contains(&r) {
            self.out.refusals.push(r);
        }
    }

    /// Visit the named children of `n`; `ctx` is `n`'s own context (flowing through `INHERIT`/`WRAP`).
    fn children(&mut self, n: Node<'t>, ctx: Ctx<'a>, rg: Region, cons: Option<Construct<'t>>) {
        let kind = n.kind();
        // No runtime evaluation (type positions, signatures that never run): nothing to census.
        // `order.tsv` classes these `none`, and the certificate pins that table.
        if self.t.order_class(kind) == Some("none") {
            return;
        }
        let is_owner = n.id() == self.owner.id();
        let callable = !is_owner && is_callable(self.t, kind);
        let mut kids: Vec<(Node<'t>, Option<&'static str>)> = Vec::new();
        let mut cursor = n.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().is_named() {
                    kids.push((cursor.node(), cursor.field_name()));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        let is_static = {
            let mut c = n.walk();
            let found = n.children(&mut c).any(|k| k.kind() == "static");
            found
        };
        for (c, field) in kids {
            let slot = field.unwrap_or("<children>");
            let role = self.t.role(kind, slot);
            let is_params = matches!(field, Some("parameters" | "parameter"));
            let mut crg = rg;
            if is_owner && is_params {
                crg.own_params = true;
            }
            // `function g() {}` declares g in the enclosing region; the owner's own hoisted name is
            // outside the owner; everything else of a nested callable belongs to the callable.
            let outer_name = field == Some("name") && role == Some("DECL:fn-hoist");
            // A callable's deferred region is entered by child position: its parameters, its body and
            // a function expression's own name. A method's computed key and decorators are eager and
            // keep the enclosing region.
            let enters = is_params
                || field == Some("body")
                || (field == Some("name") && role == Some("DECL:fn-inner"));
            if callable && enters {
                crg.nested = true;
                crg.nested_params |= is_params;
            }
            // TS parameter decorators are eager and run in the scope enclosing the callable.
            let param_decorator = matches!(kind, "required_parameter" | "optional_parameter")
                && field == Some("decorator");
            if param_decorator {
                // Only the owner's own parameter decorators belong to the enclosing owner; a nested
                // callable's (even one inside the owner's parameter defaults) keep its enclosing region.
                let of_owner = n
                    .parent()
                    .and_then(|p| p.parent())
                    .is_some_and(|g| g.id() == self.owner.id());
                if of_owner {
                    continue;
                }
                crg = *self.param_outer.last().unwrap_or(&rg);
            }
            if kind == "with_statement" && slot == "body" {
                crg.with_body = true;
            }
            let owner_eager =
                kind == "method_definition" && matches!(field, Some("name" | "decorator"));
            if is_owner && (outer_name || owner_eager) {
                continue;
            }
            let mut ccons = cons;
            let mut cctx = Ctx::Read;
            match role {
                None | Some("R") => {}
                Some("RDEF") => crg.instance_field |= !is_static,
                Some(r @ ("W" | "RW")) => {
                    cctx = Ctx::Target(if r == "W" {
                        Form::Write
                    } else {
                        Form::ReadWrite
                    });
                    ccons = Some(Self::construct(n));
                }
                Some("DEL") => {
                    if n.child_by_field_name("operator")
                        .is_some_and(|o| o.kind() == "delete")
                    {
                        cctx = Ctx::Target(Form::Delete);
                        ccons = Some(Self::construct(n));
                    }
                }
                Some("FORHEAD") => {
                    ccons = None;
                    cctx = if n.child_by_field_name("kind").is_some() {
                        Ctx::Bind("declarator", true)
                    } else {
                        Ctx::Target(Form::Write)
                    };
                }
                Some("IMPORTNAME") => {
                    ccons = None;
                    cctx = if n.child_by_field_name("alias").is_some() {
                        Ctx::Skip
                    } else {
                        Ctx::Bind("import", true)
                    };
                }
                Some("INHERIT") => cctx = ctx,
                Some("WRAP") => {
                    let operand = wrap_operand(n).is_some_and(|o| o.id() == c.id());
                    cctx = match ctx {
                        Ctx::Target(_) | Ctx::Bind(..) if operand => ctx,
                        Ctx::Target(_) | Ctx::Bind(..) => Ctx::Skip,
                        other => other,
                    };
                }
                Some("N" | "T") => cctx = Ctx::Skip,
                Some(r) => {
                    if let Some(d) = r.strip_prefix("DECL:") {
                        let declarator = d == "declarator" && kind == "variable_declarator";
                        ccons = declarator.then(|| Self::construct(n));
                        // Only `var` without an initializer stores nothing; `let b;` stores undefined.
                        let is_var = n
                            .parent()
                            .is_some_and(|p| p.kind() == "variable_declaration");
                        let initialized =
                            !declarator || !is_var || n.child_by_field_name("value").is_some();
                        cctx = Ctx::Bind(d, initialized);
                    }
                }
            }
            let outer = callable && is_params;
            if outer {
                self.param_outer.push(rg);
            }
            self.visit(c, cctx, crg, ccons);
            if outer {
                self.param_outer.pop();
            }
        }
    }

    fn emit(
        &mut self,
        n: Node<'t>,
        path: Path,
        ctx: Ctx<'a>,
        rg: Region,
        cons: Option<Construct<'t>>,
    ) {
        let (form, decl, initialized) = match ctx {
            Ctx::Target(f) => (f, "", true),
            Ctx::Bind(d, init) => (Form::Declare, d, init),
            _ => return,
        };
        let anchor = cons.map_or(n, |c| c.node);
        let (chain, span) = if self.mapping {
            self.map(anchor)
        } else {
            (None, None)
        };
        let for_head = cons.is_none() && matches!(ctx, Ctx::Target(Form::Write));
        self.out.sites.push(Site {
            path,
            line: n.start_position().row + 1,
            target_start: n.start_byte(),
            form,
            decl: decl.to_string(),
            initialized,
            construct_kind: cons.map(|c| c.kind),
            moment: cons.map_or(n.end_byte(), |c| c.end),
            construct_start: cons.map(|c| c.start),
            for_head,
            region: rg,
            chain,
            span,
        });
    }

    /// E18: the innermost statement span containing `anchor` (by bytes) and the kinds from the anchor
    /// (or its parent) up to that span's statement.
    fn map(&self, anchor: Node<'t>) -> (Option<Vec<&'static str>>, Option<(usize, usize)>) {
        let (s, e) = (anchor.start_byte(), anchor.end_byte());
        let Some(span) = self
            .spans
            .iter()
            .filter(|sp| sp.start_byte <= s && e <= sp.end_byte)
            .min_by_key(|sp| {
                (
                    sp.end_byte - sp.start_byte,
                    std::cmp::Reverse(sp.start_byte),
                )
            })
        else {
            return (None, None);
        };
        let key = Some((span.start_byte, span.end_byte));
        let mut chain = Vec::new();
        let mut cur = Some(anchor);
        let mut first = true;
        while let Some(node) = cur {
            let is_span = node.start_byte() == span.start_byte && node.end_byte() == span.end_byte;
            if !first || is_span {
                chain.push(node.kind());
            }
            if is_span {
                return (Some(chain), key);
            }
            first = false;
            cur = node.parent();
        }
        (None, None)
    }
}
