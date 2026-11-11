//! C facts for rules U and D (#630 slice 6b): `.c` files, `function_definition` owners.
//! Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md` §5.1.
//!
//! Both rules read unexpanded source and drop only identifier paths (no field paths) of a
//! binding that resolves lexically to an automatic, unescaped local or parameter of the
//! owner. Gates G1-G6 switch an owner off; E1-E4 judge each occurrence. The in-repo macro
//! census (`CMacroCensus`) closes hidden reads, writes, address-taking and jumps; `incomplete`
//! disables every owner. Dominating writes are whole statements that are direct children of
//! a single-entry region, with no label or nested `case` between the write and the use.
//!
//! Out of model (disclosed): a macro defined outside the repository acts as a function (it
//! reads and writes only what its arguments spell, takes an address only where an argument
//! spells `&`, hides no jump); a `.c` file compiled as C++; the survivor-based fallback cap
//! (#634) lets a formerly capped owner regain its surviving edges.
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::{
    macro_call_arguments, normalized_texts, spelled_identifiers, CMacroCensus,
};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Why an owner is off (the ledger names these).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Off {
    /// G1: the file is not a `.c` file of language C (a header, C++).
    G1File,
    /// G1: the owner is not a `function_definition`.
    G1Kind,
    /// G1: the owner has a parse error.
    G1Error,
    /// G1: the owner holds a nested function.
    G1Nested,
    /// G2: `#define`/`#undef`/`#include` or an OpenMP/OpenACC pragma in the body.
    G2,
    /// G3: asm or SEH.
    G3,
    /// G3: a Microsoft-extension node (`ms_*`: `__declspec`, calling conventions, `__restrict`,
    /// pointer modifiers); refused whole, split out in the ledger.
    G3Ms,
    /// G4: a returns-twice name.
    G4,
    /// G5: an invoked in-repo macro is poisoned.
    G5,
    /// The census could not read some `#define` line.
    Incomplete,
}

/// Names whose call can return a second time (G4).
const RETURNS_TWICE: &[&str] = &[
    "setjmp",
    "_setjmp",
    "sigsetjmp",
    "__sigsetjmp",
    "__builtin_setjmp",
    "savectx",
    "vfork",
    "getcontext",
];
/// Builtin varargs macros write or read the list named by their argument.
const VA_NAMES: &[&str] = &[
    "va_start",
    "va_end",
    "va_copy",
    "va_arg",
    "__builtin_va_start",
    "__builtin_va_end",
    "__builtin_va_copy",
    "__builtin_va_arg",
];
const STORAGE_OR_QUALIFIER: &[&str] = &[
    "static",
    "extern",
    "volatile",
    "_Atomic",
    "__thread",
    "_Thread_local",
    "thread_local",
];

/// Owner-level facts, present only when G1-G5 hold.
pub(super) struct COwner {
    /// G6: every `goto` target is a visible label and no control macro is invoked (D only).
    pub(super) d_ok: bool,
    /// E3: names escaped anywhere in the owner (by name, any binding).
    escaped: BTreeSet<String>,
    /// E4: roots of a member or subscript access anywhere in the owner.
    member_roots: BTreeSet<String>,
    /// Names that are in-repo macros (the spelling is ineligible).
    macro_names: BTreeSet<String>,
    /// Declarators found under a wrapper the resolver cannot see (E1; names are in `escaped`).
    pub(super) hidden_decls: usize,
}

fn ident_chars(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !ident_chars(c))
        .filter(|t| !t.is_empty())
}

pub(super) fn is_dot_c(parsed: &ParsedFile) -> bool {
    parsed.path.ends_with(".c")
}

/// The textual belt of G2: a logical line that starts with `#` (or `%:`) and names a
/// directive that can change names or the code the body means. Lines are read after
/// translation phases 1-2 and comment stripping, in both dialects and splice modes, so a
/// comment (`#pragma /*gap*/ omp`) or a continuation cannot hide the directive word.
fn body_has_forbidden_directive(text: &str) -> bool {
    normalized_texts(text).iter().any(|t| {
        t.lines().any(|line| {
            let t = line.trim_start();
            let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
                return false;
            };
            let rest = rest.trim_start();
            let word: String = rest.chars().take_while(|c| ident_chars(*c)).collect();
            match word.as_str() {
                "define" | "undef" | "include" | "include_next" | "import" => true,
                "pragma" => {
                    let arg = rest["pragma".len()..].trim_start();
                    let first: String = arg.chars().take_while(|c| ident_chars(*c)).collect();
                    matches!(first.as_str(), "omp" | "acc")
                }
                _ => false,
            }
        })
    })
}

/// The name a call's callee ends in: the identifier, the field of `a.f` / `a->f`, or either
/// inside parentheses. A function-like macro can be spelled after `->` or `.`, or in
/// parentheses, and the preprocessor still expands it.
fn callee_last_name<'a>(parsed: &'a ParsedFile, mut f: Node<'a>) -> Option<&'a str> {
    loop {
        match f.kind() {
            "identifier" | "field_identifier" => return Some(parsed.node_text(&f)),
            "field_expression" => f = f.child_by_field_name("field")?,
            "parenthesized_expression" => f = f.named_child(0)?,
            _ => return None,
        }
    }
}

/// Names declared by a `declaration` that is not a direct child of a block, a direct `case`,
/// or a `for` header (a declaration under `#if`/`#ifdef`/`#else`/`#elif`, a label or a nested
/// `case` is invisible to the lexical resolver, so the occurrence could bind an outer local).
fn hidden_declaration(node: Node<'_>) -> bool {
    if node.kind() != "declaration" {
        return false;
    }
    match node.parent() {
        Some(p) => match p.kind() {
            "compound_statement" | "for_statement" | "function_definition" | "translation_unit" => {
                false
            }
            "case_statement" => !p
                .parent()
                .is_some_and(|b| b.kind() == "compound_statement" && is_switch_body(b)),
            _ => true,
        },
        None => false,
    }
}

fn is_name_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier" | "type_identifier" | "field_identifier" | "statement_identifier"
    )
}

/// Root identifier of an address-of or member chain: parentheses, `.` access and subscripts.
fn chain_root<'a>(parsed: &ParsedFile, mut node: Node<'a>) -> Option<Node<'a>> {
    loop {
        match node.kind() {
            "identifier" => return Some(node),
            "parenthesized_expression" => node = node.named_child(0)?,
            "field_expression" => {
                node = node.child_by_field_name("argument")?;
                let _ = parsed;
            }
            "subscript_expression" => node = node.child_by_field_name("argument")?,
            _ => return None,
        }
    }
}

fn all_identifiers(node: Node<'_>, parsed: &ParsedFile, out: &mut BTreeSet<String>) {
    if node.kind() == "identifier" {
        out.insert(parsed.node_text(&node).to_owned());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        all_identifiers(child, parsed, out);
    }
}

/// G1-G5 and the owner-wide sets. `Err` names the gate that switched the owner off.
pub(super) fn scan(
    parsed: &ParsedFile,
    owner: Node<'_>,
    census: &CMacroCensus,
) -> Result<COwner, Off> {
    if parsed.language != crate::languages::Language::C || !is_dot_c(parsed) {
        return Err(Off::G1File);
    }
    if owner.kind() != "function_definition" {
        return Err(Off::G1Kind);
    }
    if owner.has_error() {
        return Err(Off::G1Error);
    }
    if census.incomplete() {
        return Err(Off::Incomplete);
    }
    let text = parsed.node_text(&owner);
    if body_has_forbidden_directive(text) {
        return Err(Off::G2);
    }
    let mut out = COwner {
        d_ok: true,
        escaped: BTreeSet::new(),
        member_roots: BTreeSet::new(),
        macro_names: BTreeSet::new(),
        hidden_decls: 0,
    };
    let mut labels = BTreeSet::new();
    let mut gotos = Vec::new();
    let mut off = None;
    // Macro spellings from the owner's preprocessing tokens, whatever node kind the parser
    // gave them (`#define const x = saved;` makes `const` a macro use).
    let Some(spelled) = spelled_identifiers(text) else {
        return Err(Off::G2);
    };
    for name in &spelled {
        note_name(name, census, &mut out, &mut off);
    }
    // Every identifier inside the parentheses of an in-repo macro (or varargs builtin)
    // invocation escapes, found in the token stream, not in call-expression nodes.
    let Some(args) =
        macro_call_arguments(text, &|n| census.get(n).is_some() || VA_NAMES.contains(&n))
    else {
        return Err(Off::G2);
    };
    out.escaped.extend(args);
    walk(
        parsed,
        owner,
        owner,
        census,
        &mut out,
        &mut labels,
        &mut gotos,
        &mut off,
    );
    if let Some(off) = off {
        return Err(off);
    }
    if gotos.iter().any(|g| !labels.contains(g)) {
        out.d_ok = false;
    }
    Ok(out)
}

/// Gates and census facts of one spelled name (a node of a name kind, or a preprocessing
/// token).
fn note_name(name: &str, census: &CMacroCensus, out: &mut COwner, off: &mut Option<Off>) {
    let mut set_off = |o: Off| {
        if off.is_none() {
            *off = Some(o);
        }
    };
    match name {
        "asm" | "__asm" | "__asm__" | "__try" | "__except" | "__finally" | "__leave" => {
            set_off(Off::G3)
        }
        "_Pragma" => set_off(Off::G2),
        n if RETURNS_TWICE.contains(&n) => set_off(Off::G4),
        _ => {}
    }
    if let Some(facts) = census.get(name) {
        out.macro_names.insert(name.to_owned());
        if facts.poisoned {
            set_off(Off::G5);
        }
        if facts.control {
            out.d_ok = false;
        }
        out.escaped.extend(facts.free.iter().cloned());
        out.escaped.extend(facts.params.iter().cloned());
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    parsed: &ParsedFile,
    owner: Node<'_>,
    node: Node<'_>,
    census: &CMacroCensus,
    out: &mut COwner,
    labels: &mut BTreeSet<String>,
    gotos: &mut Vec<String>,
    off: &mut Option<Off>,
) {
    let kind = node.kind();
    let mut set_off = |o: Off| {
        if off.is_none() {
            *off = Some(o);
        }
    };
    if node.id() != owner.id() && kind == "function_definition" {
        // A GNU nested function can write a captured local at a call site.
        set_off(Off::G1Nested);
    }
    if matches!(
        kind,
        "preproc_def" | "preproc_function_def" | "preproc_include"
    ) {
        set_off(Off::G2);
    }
    if kind.starts_with("ms_") {
        set_off(Off::G3Ms);
    } else if kind.contains("asm") || kind.starts_with("seh_") {
        set_off(Off::G3);
    }
    if is_name_kind(kind) {
        note_name(parsed.node_text(&node), census, out, off);
    }
    if hidden_declaration(node) {
        let mut cursor = node.walk();
        for d in node.children_by_field_name("declarator", &mut cursor) {
            all_identifiers(d, parsed, &mut out.escaped);
            out.hidden_decls += 1;
        }
    }
    match kind {
        "labeled_statement" => {
            if let Some(l) = node.child_by_field_name("label") {
                labels.insert(parsed.node_text(&l).to_owned());
            }
        }
        "goto_statement" => {
            if let Some(l) = node.child_by_field_name("label") {
                gotos.push(parsed.node_text(&l).to_owned());
            }
        }
        // `&x`, `&(x)`, `&x.f`, `&x[i]`.
        "pointer_expression" => {
            let amp = node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op) == "&");
            if amp {
                if let Some(arg) = node.child_by_field_name("argument") {
                    if let Some(root) = chain_root(parsed, arg) {
                        out.escaped.insert(parsed.node_text(&root).to_owned());
                    }
                }
            }
        }
        "call_expression" => {
            if let (Some(f), Some(args)) = (
                node.child_by_field_name("function"),
                node.child_by_field_name("arguments"),
            ) {
                // Any occurrence inside an in-repo function-like macro invocation (also
                // spelled `it->MACRO(x)` or `(MACRO)(x)`), and the varargs builtins that
                // write their list argument.
                if callee_last_name(parsed, f)
                    .is_some_and(|name| census.get(name).is_some() || VA_NAMES.contains(&name))
                {
                    all_identifiers(args, parsed, &mut out.escaped);
                }
            }
        }
        "field_expression" | "subscript_expression" => {
            if let Some(root) = node
                .child_by_field_name("argument")
                .and_then(|a| chain_root(parsed, a))
            {
                out.member_roots.insert(parsed.node_text(&root).to_owned());
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(parsed, owner, child, census, out, labels, gotos, off);
    }
}

/// A declarator chain's identifier and shape.
struct Declared<'a> {
    ident: Node<'a>,
    pointer: bool,
    /// An array or function declarator, or an unknown wrapper: never eligible.
    exotic: bool,
}

/// Unwrap a declarator through pointer and parenthesized declarators to its identifier.
fn declared<'a>(mut node: Node<'a>) -> Option<Declared<'a>> {
    let (mut pointer, mut exotic) = (false, false);
    loop {
        match node.kind() {
            "identifier" => {
                return Some(Declared {
                    ident: node,
                    pointer,
                    exotic,
                })
            }
            "pointer_declarator" => {
                pointer = true;
                node = node.child_by_field_name("declarator")?;
            }
            "parenthesized_declarator" => {
                node = node.named_child(0)?;
            }
            "array_declarator" | "function_declarator" => {
                exotic = true;
                node = node.child_by_field_name("declarator")?;
            }
            "attributed_declarator" => {
                exotic = true;
                node = node.named_child(0)?;
            }
            _ => return None,
        }
    }
}

/// `pure_path` for C: an identifier, or a pointer/parenthesized declarator around one.
pub(super) fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    let d = declared(node)?;
    if d.exotic || (d.ident.id() != node.id() && !is_plain_declarator_chain(node)) {
        return None;
    }
    Some(AccessPath::simple(parsed.node_text(&d.ident)))
}

fn is_plain_declarator_chain(mut node: Node<'_>) -> bool {
    loop {
        match node.kind() {
            "identifier" => return true,
            "pointer_declarator" => match node.child_by_field_name("declarator") {
                Some(n) => node = n,
                None => return false,
            },
            "parenthesized_declarator" => match node.named_child(0) {
                Some(n) => node = n,
                None => return false,
            },
            _ => return false,
        }
    }
}

/// The declaration (or parameter) a declarator identifier sits in, and the declarator
/// subtree directly under the declaration or `init_declarator`.
fn declaration_of<'a>(ident: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    let mut top = ident;
    loop {
        let parent = top.parent()?;
        match parent.kind() {
            "pointer_declarator" | "parenthesized_declarator" => top = parent,
            "array_declarator" | "function_declarator" | "attributed_declarator" => top = parent,
            "init_declarator" => {
                if parent.child_by_field_name("declarator")?.id() != top.id() {
                    return None;
                }
                let decl = parent.parent()?;
                return (decl.kind() == "declaration").then_some((decl, top));
            }
            "declaration" | "parameter_declaration" => {
                return Some((parent, top));
            }
            _ => return None,
        }
    }
}

/// E2 on one declarator identifier: automatic, unqualified, no macro type, not an array or
/// function. `Some(is_pointer)` when eligible.
fn eligible_declared(parsed: &ParsedFile, owner: &COwner, d: &Declared<'_>) -> Option<bool> {
    if d.exotic || owner.macro_names.contains(parsed.node_text(&d.ident)) {
        return None;
    }
    let (decl, top) = declaration_of(d.ident)?;
    // Specifier tokens (everything before the first declarator) must hold no storage class,
    // qualifier or in-repo macro name.
    let first_decl_start = {
        let mut cursor = decl.walk();
        let found = decl
            .children_by_field_name("declarator", &mut cursor)
            .map(|c| c.start_byte())
            .min();
        found.unwrap_or_else(|| top.start_byte())
    };
    let spec = &parsed.source[decl.start_byte()..first_decl_start.max(decl.start_byte())];
    if tokens(spec).any(|t| STORAGE_OR_QUALIFIER.contains(&t) || owner.macro_names.contains(t)) {
        return None;
    }
    // Qualifiers inside the declarator (`int *volatile p`).
    if tokens(parsed.node_text(&top)).any(|t| STORAGE_OR_QUALIFIER.contains(&t)) {
        return None;
    }
    Some(d.pointer)
}

/// The binding an occurrence resolves to: the declarator identifier node, found lexically,
/// innermost scope first. `None`: unresolved, ambiguous or not an owner-local declaration.
pub(super) fn resolve<'a>(parsed: &ParsedFile, occ: Node<'a>) -> Option<Node<'a>> {
    let name = parsed.node_text(&occ);
    let mut child = occ;
    loop {
        let parent = child.parent()?;
        let mut found: Vec<Node<'a>> = Vec::new();
        match parent.kind() {
            "compound_statement" => {
                scope_decls(parsed, parent, name, occ, &mut found);
                if parent
                    .parent()
                    .is_some_and(|p| p.kind() == "switch_statement")
                {
                    let mut cursor = parent.walk();
                    for case in parent.named_children(&mut cursor) {
                        if case.kind() == "case_statement" {
                            scope_decls(parsed, case, name, occ, &mut found);
                        }
                    }
                }
            }
            "for_statement" => {
                if let Some(init) = parent.child_by_field_name("initializer") {
                    if init.kind() == "declaration" {
                        decl_idents(parsed, init, name, occ, &mut found);
                    }
                }
            }
            "function_definition" => {
                let mut cursor = parent.walk();
                // K&R parameter declarations: not modelled.
                if parent
                    .named_children(&mut cursor)
                    .any(|c| c.kind() == "declaration")
                {
                    return None;
                }
                if let Some(params) = parent
                    .child_by_field_name("declarator")
                    .and_then(function_params)
                {
                    let mut cursor = params.walk();
                    for p in params.named_children(&mut cursor) {
                        if p.kind() == "parameter_declaration" {
                            decl_idents(parsed, p, name, occ, &mut found);
                        }
                    }
                }
                // The nearest function is the end of every scope walk.
                return (found.len() == 1).then(|| found[0]);
            }
            _ => {}
        }
        match found.len() {
            0 => {}
            1 => return Some(found[0]),
            _ => return None,
        }
        child = parent;
    }
}

/// The `parameter_list` of a function declarator chain.
fn function_params(mut declarator: Node<'_>) -> Option<Node<'_>> {
    loop {
        match declarator.kind() {
            "function_declarator" => {
                // `int (*f(int a))(int b)`: the innermost function declarator holds the
                // function's own parameters; outer ones belong to the returned type.
                let inner = declarator.child_by_field_name("declarator")?;
                if let Some(p) = function_params(inner) {
                    return Some(p);
                }
                return declarator.child_by_field_name("parameters");
            }
            "pointer_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator"
            | "array_declarator" => {
                declarator = declarator
                    .child_by_field_name("declarator")
                    .or_else(|| declarator.named_child(0))?;
            }
            _ => return None,
        }
    }
}

fn scope_decls<'a>(
    parsed: &ParsedFile,
    scope: Node<'a>,
    name: &str,
    occ: Node<'a>,
    out: &mut Vec<Node<'a>>,
) {
    let mut cursor = scope.walk();
    for child in scope.named_children(&mut cursor) {
        if child.kind() == "declaration" {
            decl_idents(parsed, child, name, occ, out);
        }
    }
}

/// Declarator identifiers of `decl` spelled `name` whose scope has begun at `occ` (the
/// declarator itself ends before the occurrence, or is the occurrence).
fn decl_idents<'a>(
    parsed: &ParsedFile,
    decl: Node<'a>,
    name: &str,
    occ: Node<'a>,
    out: &mut Vec<Node<'a>>,
) {
    let mut cursor = decl.walk();
    for d in decl.children_by_field_name("declarator", &mut cursor) {
        let top = if d.kind() == "init_declarator" {
            match d.child_by_field_name("declarator") {
                Some(t) => t,
                None => continue,
            }
        } else {
            d
        };
        if let Some(dd) = declared(top) {
            if parsed.node_text(&dd.ident) == name && dd.ident.start_byte() <= occ.start_byte() {
                out.push(dd.ident);
            }
        }
    }
}

impl COwner {
    /// E1-E3 for one occurrence: resolves to an eligible owner-local declaration that never
    /// escapes. Returns the declaration's identifier byte start.
    pub(super) fn binding(&self, parsed: &ParsedFile, occ: Node<'_>) -> Option<usize> {
        let name = parsed.node_text(&occ);
        if self.escaped.contains(name) || self.macro_names.contains(name) {
            return None;
        }
        let ident = resolve(parsed, occ)?;
        let d = declared_of_ident(ident)?;
        eligible_declared(parsed, self, &d)?;
        Some(ident.start_byte())
    }

    /// E4: a non-pointer declarator whose name has any member or subscript access in the
    /// owner may be sub-object-written after a whole write.
    pub(super) fn subobject_free(&self, parsed: &ParsedFile, ident_start: usize) -> bool {
        let Some(ident) = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(ident_start, ident_start)
        else {
            return false;
        };
        let name = parsed.node_text(&ident);
        let pointer = declared_of_ident(ident).is_some_and(|d| d.pointer);
        pointer || !self.member_roots.contains(name)
    }
}

/// Re-derive the declarator chain around a declarator identifier.
fn declared_of_ident(ident: Node<'_>) -> Option<Declared<'_>> {
    let (_, top) = declaration_of(ident)?;
    let d = declared(top)?;
    (d.ident.id() == ident.id()).then_some(d)
}

/// Region: a `compound_statement` that is not a switch body, or a `case_statement` that is a
/// direct child of a switch body.
pub(super) fn is_region(node: Node<'_>) -> bool {
    match node.kind() {
        "compound_statement" => node.parent().is_none_or(|p| p.kind() != "switch_statement"),
        "case_statement" => node
            .parent()
            .is_some_and(|b| b.kind() == "compound_statement" && is_switch_body(b)),
        _ => false,
    }
}

fn is_switch_body(compound: Node<'_>) -> bool {
    compound
        .parent()
        .is_some_and(|p| p.kind() == "switch_statement")
}

/// A label, or a `case` that is not a direct child of a switch body, starts in
/// `[from, to)` at any depth: control can enter the window there.
pub(super) fn entry_in_range(node: Node<'_>, from: usize, to: usize) -> bool {
    if node.end_byte() <= from || node.start_byte() >= to {
        return false;
    }
    let inside = node.start_byte() >= from && node.start_byte() < to;
    match node.kind() {
        "labeled_statement" if inside => return true,
        "case_statement" if inside => {
            let direct = node
                .parent()
                .is_some_and(|b| b.kind() == "compound_statement" && is_switch_body(b));
            if !direct {
                return true;
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if entry_in_range(child, from, to) {
            return true;
        }
    }
    false
}

/// The write statement of an lvalue occurrence and whether it is a plain `=` / declarator
/// write. The statement is the node whose parent must be the region for a dominating write:
/// the `expression_statement` of a bare assignment, the `declaration` of an initialised
/// declarator; every other write (nested assignment, no initializer) gets a node that is
/// never a region child.
pub(super) fn classify_write<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    owner: &COwner,
) -> Option<(Node<'a>, bool)> {
    let _ = owner;
    let parent = node.parent()?;
    // Declarator position: identifier or pointer/parenthesized chain.
    if matches!(
        node.kind(),
        "identifier" | "pointer_declarator" | "parenthesized_declarator"
    ) {
        let mut top = node;
        while let Some(p) = top.parent() {
            if matches!(p.kind(), "pointer_declarator" | "parenthesized_declarator") {
                top = p;
            } else {
                break;
            }
        }
        // The span must be the whole top declarator chain (or an identifier inside it that
        // the chain's top has as its only content).
        if top.id() == node.id() || node.kind() != "identifier" {
            let p = top.parent()?;
            match p.kind() {
                "init_declarator" if p.child_by_field_name("declarator")?.id() == top.id() => {
                    let decl = p.parent()?;
                    if decl.kind() == "declaration" {
                        return Some((decl, true));
                    }
                }
                "declaration" => {
                    // No initializer: a write that never dominates.
                    return Some((top, true));
                }
                _ => {}
            }
        }
    }
    if node.kind() == "identifier" && parent.kind() == "assignment_expression" {
        let left = parent.child_by_field_name("left")?;
        if left.id() != node.id() {
            return None;
        }
        let plain = parent
            .child_by_field_name("operator")
            .is_some_and(|op| parsed.node_text(&op) == "=");
        let statement = match parent.parent() {
            Some(s) if s.kind() == "expression_statement" => s,
            _ => parent,
        };
        return Some((statement, plain));
    }
    None
}

/// The variable-write certificate of a D candidate (F1): the raw span is an identifier (or
/// a declarator chain around one) spelled as the path's base, in the `left` of an assignment
/// (any operator), in a declarator position of a declaration, or a parameter. Returns the
/// binding's declarator byte start.
pub(super) fn certified_candidate(
    parsed: &ParsedFile,
    start: usize,
    end: usize,
    path: &AccessPath,
) -> Option<usize> {
    let node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    if node.start_byte() != start || node.end_byte() != end {
        return None;
    }
    let d = declared(node)?;
    if d.exotic
        || !path.fields.is_empty()
        || parsed.node_text(&d.ident) != path.base
        || !is_plain_declarator_chain(node)
    {
        return None;
    }
    if node.kind() == "identifier" {
        let parent = node.parent()?;
        if parent.kind() == "assignment_expression" {
            return (parent.child_by_field_name("left")?.id() == node.id())
                .then(|| resolve(parsed, node))
                .flatten()
                .map(|n| n.start_byte());
        }
    }
    // A declarator position: the chain's top sits directly under a declaration,
    // `init_declarator` (as its declarator, never its value) or parameter.
    let mut top = node;
    while let Some(p) = top.parent() {
        if matches!(p.kind(), "pointer_declarator" | "parenthesized_declarator") {
            top = p;
        } else {
            break;
        }
    }
    let p = top.parent()?;
    let ok = match p.kind() {
        "init_declarator" => p
            .child_by_field_name("declarator")
            .is_some_and(|d| d.id() == top.id()),
        "declaration" | "parameter_declaration" => true,
        _ => false,
    };
    ok.then(|| d.ident.start_byte())
}

/// The identifier a write span names: the span itself, or the identifier at the end of a
/// pointer/parenthesized declarator chain.
pub(super) fn write_target_identifier(node: Node<'_>) -> Option<Node<'_>> {
    let d = declared(node)?;
    (!d.exotic).then_some(d.ident)
}

/// `PRISM_C_PRUNE_TRACE=1` names every C owner's verdict on stderr (measurement ledger).
pub(super) fn trace(parsed: &ParsedFile, owner: Node<'_>, scanned: &Result<COwner, Off>) {
    if std::env::var_os("PRISM_C_PRUNE_TRACE").is_none() {
        return;
    }
    let status = match scanned {
        Ok(o) => format!(
            "{}\tescaped={}\tmacro_names={}\tmember_roots={}\thidden_decls={}",
            if o.d_ok { "on" } else { "on-no-d:g6" },
            o.escaped.len(),
            o.macro_names.len(),
            o.member_roots.len(),
            o.hidden_decls
        ),
        Err(off) => format!("off:{off:?}"),
    };
    eprintln!(
        "c-prune-owner\t{status}\t{}\t{}",
        parsed.path,
        owner.start_position().row + 1
    );
}
