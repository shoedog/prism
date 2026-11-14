//! C facts for rules U and D (#630 slice 6b): `.c` files, `function_definition` owners.
//! Slice 6c runs the same rules, plus `cpp_prune`'s scalar gate and escapes, for C++ owners (a
//! `.h` file is parsed as C, #529, and stays off: `G1File`).
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
use super::c_phantom::escape_phantom;
pub(super) use super::c_prune_resolve::{declaration_of, pure_path, resolve, ResolveCache};
use super::c_prune_resolve::{declared_of_ident, eligible_declared};
use super::c_prune_write::is_switch_body;
pub(super) use super::c_prune_write::{
    certified_candidate, classify_write, is_region, write_target_identifier,
};
use super::cpp_prune;
use super::prune_roots::{callee_last_name, chain_roots};
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::{
    macro_call_arguments, normalized_texts, spelled_identifiers, CMacroCensus,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Why an owner is off (the ledger names these).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Off {
    /// G1: the file is not a `.c` file of language C or a C++ file (a `.h`, parsed as C).
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
    /// 6c: a block-scope `using namespace` (any name may rebind).
    UsingDirective,
    /// 6c: a coroutine (implicit promise and awaiter calls can bind a parameter or operand).
    Coroutine,
    /// 6c: the owner spells an in-repo macro named like a C++ keyword (`#define int IR`).
    KeywordMacro,
    /// 6d-2s: the text holds a trigraph or a backslash followed by blanks and a newline, which
    /// the compiler's translation phases read differently from the parser (`// ??/` splices the
    /// next line into the comment).
    PhaseDivergence,
}

/// Text the compiler's translation phases read differently from tree-sitter: a trigraph
/// (`??` + one of `= / ' ( ) ! < > -`, `??/` being a backslash) or a backslash followed by
/// blanks (space, tab, VT, FF) and then a newline, which GCC and clang splice and the parser
/// does not. A line the compiler hides inside a comment can look like a declaration to the
/// parser. Conservative: string literals count too.
pub(super) fn phase_divergence(text: &str) -> bool {
    let b = text.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b'?' && b.get(i + 1) == Some(&b'?') {
            if let Some(&t) = b.get(i + 2) {
                if b"=/'()!<>-".contains(&t) {
                    return true;
                }
            }
        }
        if c == b'\\' {
            let mut j = i + 1;
            while matches!(b.get(j), Some(b' ' | b'\t' | 0x0b | 0x0c)) {
                j += 1;
            }
            if j > i + 1 && matches!(b.get(j), Some(b'\n' | b'\r')) {
                return true;
            }
        }
    }
    false
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
/// Builtins that write the list named by their argument (varargs) or take its address without
/// spelling `&` (`__builtin_addressof`, Clang, C and C++).
const VA_NAMES: &[&str] = &[
    "__builtin_addressof",
    "va_start",
    "va_end",
    "va_copy",
    "va_arg",
    "__builtin_va_start",
    "__builtin_va_end",
    "__builtin_va_copy",
    "__builtin_va_arg",
];
pub(super) const STORAGE_OR_QUALIFIER: &[&str] = &[
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
    pub(super) macro_names: BTreeSet<String>,
    /// Declarators found under a wrapper the resolver cannot see (E1; names are in `escaped`).
    pub(super) hidden_decls: usize,
    /// 6c: the C++ rules (scalar gate, C++ escapes) apply: a C++ owner.
    pub(super) cpp: bool,
    /// 6c ledger: the owner is a `template_declaration`.
    pub(super) template: bool,
    /// 6c ledger: declarations of the owner that fail the scalar gate.
    pub(super) scalar_fail: usize,
    /// 6c ledger: escaped names by C++ escape kind (trace only).
    pub(super) escape_kinds: BTreeMap<&'static str, BTreeSet<String>>,
    /// 6c: escapes that wait for the owner-wide escaped set to settle (`cpp_prune_operand::settle`).
    pub(super) deferred: Vec<super::cpp_prune_operand::Deferred>,
    /// #820: memo of block-scope declarations and entry points, read by every resolution.
    pub(super) cache: ResolveCache,
}

impl COwner {
    /// Record a C++ escape of `name` (by name, any binding).
    /// True when `name` was not escaped before.
    pub(super) fn escape(&mut self, kind: &'static str, name: &str) -> bool {
        let fresh = self.escaped.insert(name.to_owned());
        self.escape_kinds
            .entry(kind)
            .or_default()
            .insert(name.to_owned());
        fresh
    }

    pub(super) fn is_escaped(&self, name: &str) -> bool {
        self.escaped.contains(name)
    }

    pub(super) fn is_macro_name(&self, name: &str) -> bool {
        self.macro_names.contains(name)
    }

    /// `binding` without the escaped-set check: what resolution and E2 say about an occurrence,
    /// which no escape changes (6c `settle` resolves each occurrence once).
    pub(super) fn binding_unescaped(&self, parsed: &ParsedFile, occ: Node<'_>) -> bool {
        if self.macro_names.contains(parsed.node_text(&occ)) {
            return false;
        }
        let Some(ident) = resolve(parsed, &self.cache, occ) else {
            return false;
        };
        declared_of_ident(ident).is_some_and(|d| eligible_declared(parsed, self, &d).is_some())
    }
}

fn ident_chars(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

pub(super) fn tokens(text: &str) -> impl Iterator<Item = &str> {
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

/// Names declared by a `declaration` that is not a direct child of a block, a direct `case`,
/// or a `for` header (a declaration under `#if`/`#ifdef`/`#else`/`#elif`, a label or a nested
/// `case` is invisible to the lexical resolver, so the occurrence could bind an outer local).
fn hidden_declaration(node: Node<'_>, cpp: bool) -> bool {
    if node.kind() != "declaration" {
        return false;
    }
    match node.parent() {
        Some(p) => match p.kind() {
            "compound_statement" | "for_statement" | "function_definition" | "translation_unit" => {
                false
            }
            // C++: condition and init-statement declarations are resolved (see `resolve`).
            "init_statement" | "condition_clause" if cpp => false,
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

pub(super) fn all_identifiers(node: Node<'_>, parsed: &ParsedFile, out: &mut BTreeSet<String>) {
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
    use crate::languages::Language;
    let cpp = match parsed.language {
        Language::Cpp => true,
        Language::C if is_dot_c(parsed) => false,
        _ => return Err(Off::G1File),
    };
    // F6: a `template_declaration` owner holds its own inner `function_definition`, which is
    // not a nested boundary for it.
    let own_inner = if owner.kind() == "template_declaration" && parsed.language == Language::Cpp {
        let mut cursor = owner.walk();
        let inner = owner
            .named_children(&mut cursor)
            .find(|c| c.kind() == "function_definition")
            .map(|c| c.id());
        inner
    } else {
        None
    };
    if owner.kind() != "function_definition" && own_inner.is_none() {
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
    if phase_divergence(text) {
        return Err(Off::PhaseDivergence);
    }
    let mut out = COwner {
        d_ok: true,
        escaped: BTreeSet::new(),
        member_roots: BTreeSet::new(),
        macro_names: BTreeSet::new(),
        hidden_decls: 0,
        cpp,
        template: own_inner.is_some(),
        scalar_fail: 0,
        escape_kinds: BTreeMap::new(),
        deferred: Vec::new(),
        cache: ResolveCache::default(),
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
        (owner, own_inner),
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
    if out.cpp {
        super::cpp_prune_operand::settle(parsed, &mut out);
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
        // 6c: a coroutine keyword spelled only in a macro body.
        if out.cpp && cpp_prune::coroutine_macro(&facts.free) {
            set_off(Off::Coroutine);
        }
        if out.cpp && cpp_prune::is_keyword(name) {
            set_off(Off::KeywordMacro);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    parsed: &ParsedFile,
    (owner, own_inner): (Node<'_>, Option<usize>),
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
    if node.id() != owner.id() && kind == "function_definition" && Some(node.id()) != own_inner {
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
    if out.cpp {
        cpp_prune::note_node(parsed, node, out);
        if let Some(o) = cpp_prune::off_reason(node, owner) {
            if off.is_none() {
                *off = Some(o);
            }
        }
    }
    if hidden_declaration(node, out.cpp) {
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
                    let mut roots = Vec::new();
                    chain_roots(parsed, arg, &mut roots);
                    for root in roots {
                        out.escaped.insert(parsed.node_text(&root).to_owned());
                    }
                }
            }
        }
        "declaration" if !out.cpp => escape_phantom(parsed, node, &mut out.escaped),
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
            let mut roots = Vec::new();
            if let Some(a) = node.child_by_field_name("argument") {
                chain_roots(parsed, a, &mut roots);
            }
            for root in roots {
                out.member_roots.insert(parsed.node_text(&root).to_owned());
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(
            parsed,
            (owner, own_inner),
            child,
            census,
            out,
            labels,
            gotos,
            off,
        );
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
        let ident = resolve(parsed, &self.cache, occ)?;
        let d = declared_of_ident(ident)?;
        eligible_declared(parsed, self, &d)?;
        Some(ident.start_byte())
    }

    /// Why `binding` refuses an occurrence (ledger trace only; `None` when it accepts).
    pub(super) fn refusal(&self, parsed: &ParsedFile, occ: Node<'_>) -> Option<String> {
        let name = parsed.node_text(&occ);
        if self.macro_names.contains(name) {
            return Some("macro_name".into());
        }
        if self.escaped.contains(name) {
            let kinds: Vec<&str> = self
                .escape_kinds
                .iter()
                .filter(|(_, v)| v.contains(name))
                .map(|(k, _)| *k)
                .collect();
            return Some(if kinds.is_empty() {
                "escape:c".to_owned()
            } else {
                format!("escape:{}", kinds.join("+"))
            });
        }
        let Some(ident) = resolve(parsed, &self.cache, occ) else {
            return Some("e1_unresolved".into());
        };
        let Some(d) = declared_of_ident(ident) else {
            return Some("e1_ineligible_decl".into());
        };
        let saved = self.cpp;
        let mut as_c = COwner {
            cpp: false,
            template: false,
            scalar_fail: 0,
            escape_kinds: BTreeMap::new(),
            deferred: Vec::new(),
            cache: ResolveCache::default(),
            escaped: BTreeSet::new(),
            member_roots: BTreeSet::new(),
            macro_names: self.macro_names.clone(),
            d_ok: true,
            hidden_decls: 0,
        };
        let c_ok = eligible_declared(parsed, &as_c, &d).is_some();
        as_c.cpp = saved;
        if !c_ok {
            return Some("e2_storage".into());
        }
        eligible_declared(parsed, &as_c, &d)
            .is_none()
            .then(|| "e2_scalar_gate".to_owned())
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

/// `PRISM_C_PRUNE_TRACE` is set (read once).
pub(super) fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PRISM_C_PRUNE_TRACE").is_some())
}

/// `PRISM_C_PRUNE_TRACE=1` names every C owner's verdict on stderr (measurement ledger).
pub(super) fn trace(parsed: &ParsedFile, owner: Node<'_>, scanned: &Result<COwner, Off>) {
    if !trace_enabled() {
        return;
    }
    let status = match scanned {
        Ok(o) => format!(
            "{}\ttemplate={}\tcpp={}\tscalar_fail={}\tesc_kinds={}\tescaped={}\tmacro_names={}\tmember_roots={}\thidden_decls={}",
            if o.d_ok { "on" } else { "on-no-d:g6" },
            o.template,
            o.cpp,
            o.scalar_fail,
            o.escape_kinds
                .iter()
                .map(|(k, v)| format!("{k}:{}", v.len()))
                .collect::<Vec<_>>()
                .join(","),
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
