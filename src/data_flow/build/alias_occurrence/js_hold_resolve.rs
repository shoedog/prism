//! Binding identities for the value-flow closure (`js_hold.rs`, alias-twin slice 4c fold 3).
//!
//! A reference at byte `b` spelled `n` resolves to the innermost declaration of `n` whose scope holds `b` (the write
//! census's Declare sites, scoped by `captured::scope_of`, the D2-c scope facts), else to the free identity of `n`.
//! The resolver fails closed by MERGING: a spelling whose identities cannot be told apart safely collapses into one
//! node, so it degrades to spelling behaviour for that spelling only (a wrong split would be unsound, a merge only
//! costs yield). Merged: a block-level function (Annex B), two declarations with one scope range (`var` redeclared,
//! `var` + function), a body `var` or function spelled like a formal of the same callable (non-simple parameter lists
//! initialise it from the formal), a declaration the scope facts cannot place (`enum`, `namespace`, imports), and a
//! spelling with a reference that resolves free although the spelling is declared in the owner.
//! `this`, `arguments` and `super` resolve by an ancestor walk to the nearest non-arrow callable; computed keys,
//! decorators and parameter decorators of a member run in the ENCLOSING context, and field initialisers and static
//! blocks belong to their class.
use crate::ast::ParsedFile;
use crate::js_write_census::captured::{scope_of, Scope, Shadow, Shadows};
use crate::js_write_census::tables::Tables;
use crate::js_write_census::walk::{Form, Site};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A resolved identity: its node key and the byte where its binding lives (`usize::MAX`: outside everything).
pub(super) struct Ident {
    pub key: String,
    pub home: usize,
}

pub(super) struct Resolver {
    shadows: Shadows,
    kinds: BTreeMap<(String, usize), String>,
    merged: BTreeSet<String>,
    declared: BTreeSet<String>,
}

pub(super) fn is_callable_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "function"
            | "generator_function"
            | "generator_function_declaration"
            | "arrow_function"
            | "method_definition"
    )
}

pub(super) fn is_class_kind(kind: &str) -> bool {
    matches!(kind, "class_declaration" | "class")
}

impl Resolver {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>, sites: &[Site]) -> Self {
        let t = Tables::shared();
        let root = parsed.tree.root_node();
        let mut by_base: BTreeMap<String, Vec<Shadow>> = BTreeMap::new();
        let mut kinds = BTreeMap::new();
        let mut merged = BTreeSet::new();
        let mut declared = BTreeSet::new();
        let mut targets = BTreeSet::new();
        let mut ranges: BTreeMap<(String, (usize, usize)), usize> = BTreeMap::new();
        let mut params: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut others: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for s in sites.iter().filter(|s| s.form == Form::Declare) {
            let base = s.path.base.clone();
            targets.insert(s.target_start);
            declared.insert(base.clone());
            kinds.insert((base.clone(), s.target_start), s.decl.clone());
            let node = root.descendant_for_byte_range(s.target_start, s.target_end);
            let scope = node.and_then(|n| hold_scope(t, &parsed.source, &s.decl, n));
            let Some(scope) = scope else {
                merged.insert(base);
                continue;
            };
            if s.decl == "fn-hoist" && node.is_some_and(block_level_function) {
                merged.insert(base.clone());
            }
            *ranges.entry((base.clone(), scope.range)).or_default() += 1;
            let end = scope.range.1;
            if s.decl == "param" {
                params.entry(base.clone()).or_default().push(end);
            } else {
                others.entry(base.clone()).or_default().push(end);
            }
            by_base.entry(base).or_default().push(Shadow {
                start: s.target_start,
                scope,
            });
        }
        merged.extend(
            ranges
                .into_iter()
                .filter(|(_, n)| *n > 1)
                .map(|((b, _), _)| b),
        );
        // A body `var` or function ending where a formal's callable ends (the parameter scope runs to the callable's
        // end) is initialised from the formal (non-simple parameter lists): one binding for our purposes.
        for (base, ends) in &params {
            if others
                .get(base)
                .is_some_and(|o| o.iter().any(|e| ends.contains(e)))
            {
                merged.insert(base.clone());
            }
        }
        let this = Self {
            shadows: Shadows::from_parts(by_base),
            kinds,
            merged,
            declared,
        };
        // A reference that resolves free although the spelling is declared in the owner merges it.
        let mut extra = BTreeSet::new();
        let mut stack = vec![owner];
        while let Some(n) = stack.pop() {
            if matches!(
                n.kind(),
                "identifier"
                    | "shorthand_property_identifier"
                    | "shorthand_property_identifier_pattern"
            ) && !targets.contains(&n.start_byte())
            {
                let base = parsed.node_text(&n).trim();
                if this.declared.contains(base)
                    && !this.merged.contains(base)
                    && this.shadows.innermost(base, n.start_byte()).is_none()
                {
                    extra.insert(base.to_string());
                }
            }
            let mut c = n.walk();
            stack.extend(n.children(&mut c));
        }
        let mut this = this;
        this.merged.extend(extra);
        this
    }

    /// The identity key of the name `base` referenced at byte `at`.
    pub(super) fn key(&self, base: &str, at: usize) -> Ident {
        if self.merged.contains(base) {
            return Ident {
                key: format!("m:{base}"),
                home: usize::MAX,
            };
        }
        match self.shadows.innermost(base, at) {
            Some(d) => Ident {
                key: format!("b:{base}@{}", d.start),
                home: d.start,
            },
            None if self.declared.contains(base) => Ident {
                key: format!("m:{base}"),
                home: usize::MAX,
            },
            None => Ident {
                key: format!("f:{base}"),
                home: usize::MAX,
            },
        }
    }

    /// True when a plain call of this identity stores nothing the owner can see: a free name or a formal.
    pub(super) fn is_opaque(&self, id: &Ident) -> bool {
        if id.key.starts_with("f:") {
            return true;
        }
        let Some(rest) = id.key.strip_prefix("b:") else {
            return false;
        };
        let Some((base, _)) = rest.rsplit_once('@') else {
            return false;
        };
        self.kinds
            .get(&(base.to_string(), id.home))
            .is_some_and(|k| k == "param")
    }

    /// Resolve an identifier-like node: a name, `this`, `super` or `arguments` (a declared `arguments` is a name).
    pub(super) fn ident(&self, parsed: &ParsedFile, node: Node<'_>, owner: Node<'_>) -> Ident {
        match node.kind() {
            "this" | "super" => this_ident(node, owner),
            _ => {
                let base = parsed.node_text(&node).trim();
                if base == "arguments"
                    && !self.merged.contains(base)
                    && self.shadows.innermost(base, node.start_byte()).is_none()
                    && !self.declared.contains(base)
                {
                    return args_ident(node, owner);
                }
                self.key(base, node.start_byte())
            }
        }
    }
}

fn hold_scope(t: &Tables, src: &str, decl: &str, node: Node<'_>) -> Option<Scope> {
    if decl == "class-inner" {
        let class = node.parent()?;
        return Some(Scope {
            range: (class.start_byte(), class.end_byte()),
            holes: Vec::new(),
        });
    }
    scope_of(t, src, decl, node)
}

/// A function declaration that is not a direct child of a function body, a static block or the program (Annex B).
fn block_level_function(name: Node<'_>) -> bool {
    let Some(f) = name.parent() else { return true };
    let Some(p) = f.parent() else { return true };
    match p.kind() {
        "program" => false,
        "statement_block" => !p
            .parent()
            .is_some_and(|g| is_callable_kind(g.kind()) || g.kind() == "class_static_block"),
        "export_statement" => false,
        _ => true,
    }
}

/// The ancestor that supplies `this`/`arguments` to a reference: the nearest non-arrow callable, a class (field
/// initialisers and static blocks), skipping the member whose computed key or decorator holds the reference.
/// `None`: the owner is reached without one.
fn context<'t>(node: Node<'t>, owner: Node<'t>, want_class: bool) -> Option<Node<'t>> {
    let mut child = node;
    let mut skip = false;
    let mut cur = node.parent();
    while let Some(p) = cur {
        let kind = p.kind();
        if kind == "decorator" {
            skip = true;
        }
        let name_slot = ["name", "property"].iter().any(|f| {
            p.child_by_field_name(f)
                .is_some_and(|n| n.id() == child.id())
        });
        let member = matches!(
            kind,
            "method_definition" | "field_definition" | "public_field_definition"
        );
        if member && name_slot {
            skip = true;
        }
        let holder = match kind {
            "function_declaration"
            | "function_expression"
            | "function"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition" => Some(p),
            "field_definition" | "public_field_definition" | "class_static_block" if want_class => {
                p.parent().and_then(|b| b.parent())
            }
            _ => None,
        };
        if let Some(h) = holder {
            if skip {
                skip = false;
            } else {
                return Some(h);
            }
        }
        if p.id() == owner.id() {
            return None;
        }
        child = p;
        cur = p.parent();
    }
    None
}

fn this_ident(node: Node<'_>, owner: Node<'_>) -> Ident {
    match context(node, owner, true) {
        Some(h) => Ident {
            key: format!("this@{}", h.start_byte()),
            home: h.start_byte(),
        },
        None => Ident {
            key: "f:this".to_string(),
            home: usize::MAX,
        },
    }
}

fn args_ident(node: Node<'_>, owner: Node<'_>) -> Ident {
    match context(node, owner, false) {
        Some(h) => Ident {
            key: format!("args@{}", h.start_byte()),
            home: h.start_byte(),
        },
        None => Ident {
            key: "f:arguments".to_string(),
            home: usize::MAX,
        },
    }
}
