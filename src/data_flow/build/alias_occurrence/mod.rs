//! Alias-twin occurrence policy (alias-twin lane, slice 1: Java; slice 2a: Go, Rust, Python; slice 2b: C, C++; slice 4c: JS, TS, TSX).
//!
//! A DFG alias twin is a fabricated Def of an alias's target, minted at an lvalue occurrence of
//! the alias name (`u = o` makes `u.f = 1` also a write of `o.f`). The invariant this module
//! enforces, one language at a time:
//!
//! > A synthetic write to the target's storage requires an actual storage-write occurrence AND an
//! > admitted storage relation. Establishing or replacing an ordinary variable binding does not
//! > itself write the copied value's source storage.
//!
//! This module decides only the first half, per OCCURRENCE (never per normalised path): it reads
//! the original AST node the lvalue span covers plus scoped declaration evidence. The relation half
//! (is `u` really a copy of `o`) is untouched: alias discovery, the alias map and every rejected or
//! unsupported candidate are exactly as before, so no relation is resurrected (the stale-relation
//! hazard of rejecting candidates does not arise here, because no candidate is rejected).
//!
//! Dispositions: `BindingWrite` (declares, initialises or reassigns the variable named by the
//! occurrence: mint NO target twin, keep the alias binding's own def and the right-hand reads),
//! `StorageWrite` (writes the storage the name refers to: twin unchanged) and `Unsupported`
//! (missing or contradictory evidence: twin unchanged, never repaired opportunistically).
//!
//! One decision per occurrence, keyed by the span's start byte inside its owner, computed once per
//! owner ([`TwinPolicy`]) and read at all three producer integration points (twin Def registration,
//! the fallback anchor registration and twin edge emission in `file.rs`). A language that is not
//! enabled here returns an empty policy and runs no classifier, so its output is byte-identical.
//!
//! Refusal R1 (member-name tokens, an extraction defect owned by its own slice): the occurrence
//! extraction records a method name (`x.size()`) or another object's member name (`node.key`) as an
//! occurrence of the variable of that spelling. A twin def of the target currently, by accident,
//! kills edges that run through such a token; removing it would promote them to `Exact` although the
//! token does not name the variable (the gate found 18 such edges on guava). So a binding write keeps
//! its twin when the owner holds a method-name or non-`this` member-name token spelled like the alias's
//! resolved target root. Purely subtractive: the refused occurrence behaves exactly as before.
//! `this.key` is not such a token, although it names the field and not the variable when a local or
//! parameter shadows it. Residual non-variable identifier roles, un-masked like #790-#793 and absent
//! from the corpora: statement labels, `case` constants, annotation element keys, `this.N` /
//! `super.N` under a shadowing local, lambda parameters in other scopes.
//!
//! Slice 2a applies the same refusal to Go (selector field and method names, keyed literal keys,
//! labels), Rust (field and method names, scoped-path segments, macro tokens after `.`/`::` or before
//! a single `:` or a lone `=`, a named format argument) and Python (attribute names, keyword-argument
//! names). Rust also keeps the twins of a target the owner can write through a borrow the alias map does not
//! pair: `&mut <target>` in any position (call arguments included) and a `ref mut` binding of it (on a `let`
//! initialiser or a `match` scrutinee); the alias map pairs only identifiers, so the twin's kill used to hide
//! the resulting wrong `exact` (Go and Python probes show no such un-masking: their twins never killed). Residual roles not covered,
//! un-masked and absent from the corpora unless the ledger says otherwise: Go struct-field
//! declaration names are covered (`field_identifier`) but package-qualified identifiers
//! (`pkg.o`) are selector operands, not tokens; Rust `use` path segments, lifetimes and labels,
//! macro tokens not adjacent to `.`/`::`/`:`; Python import module names, decorator names and
//! parameter names of nested defs and lambdas; Go `u = append(u[:0], v)` writes the shared backing array
//! of the target, Python `import o` is not a def (pre-existing), Rust constants made by a glob import or a
//! macro, and interior mutability or a call mutating the target (out of model, as for `Exact`). Rust writes
//! the DFG cannot see and the borrow refusal does not cover: an autoref `&mut self` method call on the target
//! (`o.add_assign(3)`), a closure capturing it by unique borrow, struct-pattern shorthand `S { ref mut f }`,
//! interior mutability and call mutation. Inherited extraction defect (Refs #815): a zero-width line anchor
//! on a write-only target (`base = None;`) is recorded as a Use, so the def above it reaches it `exact`; main
//! emits the same edge with no alias involved, and the twin's kill used to hide it in clap `examples/git.rs`.
//!
//! Slice 2b (C and C++, `c_family.rs`, `c_family_hazards.rs`): rows `C-01..C-10`, `X-READ`, `X-DECL` of the table.
//! A name written by `=`, a compound operator or `++`/`--`, a declaration or parameter name and declarator syntax
//! (`*p`, `a[N]`, `&r`) are binding writes (C-01..C-04); C is always so, C++ only with positive evidence (a pointer
//! declarator or a keyword-only builtin type prefix visible at the occurrence; a `&` in the prefix is a reference and a
//! storage write; typedef/`using` names including `size_t`, `auto`, class and dependent types, members and undeclared
//! names are unsupported). `*u = ..`, `u->f`, `(*u).f`, `u[i]` through a pointer or array parameter and `*p++ = v` are
//! storage writes (C-05, C-06, C-08, C-10); `u.f` is unsupported except through a C++ reference (C-07). A `.h` header
//! (prism parses it as C) takes the C++ rules, and a C++ declaration in a `for` initialiser gives no builtin-keyword evidence (a
//! pointer declarator in it still does): both are more conservative than the table. Only the parameter list of a
//! definition's innermost declarator (or a lambda's) is visible evidence, never a prototype's or a function type's. R1 and the writes the DFG does not model are refused in `c_family_hazards.rs`
//! (struct fields, labels, enumerators, macros and macro parameters, type and namespace names; `&o` in any form, `o++`,
//! arrays, an undeclared target passed to a call, arguments of macro-looking or unproven callees, identifiers inside `ERROR`
//! nodes; an occurrence whose own statement holds a parse error is unsupported; in C++ also every call argument (bare pointers
//! included), methods, hazards on a proven reference alias, `>>`, reference-bound initialisers and by-reference lambda
//! captures). Residuals: a header macro that is not all-caps and is passed the target nested in an expression,
//! array fields decaying inside a call argument, a callee mutating the target through a pointer or global (out of model,
//! as for `Exact`; library `memcpy(o, ..)` writes the pointee), a typedef'd array target, GNU `asm` outputs, a copy constructor that writes its source, and the inherited X-READ extraction defect (a read
//! recorded as a Def: an `Exact` edge from the read of `Py_None` at one initialiser to the next, present in main
//! without any alias, now no longer doubted `sameline` where a twin collapsed the line). C and C++ produce no
//! fallback anchors (alias discovery pairs plain identifiers).
//!
//! Slice 4c (JavaScript, TypeScript, TSX, `js.rs`): rows `S-01..S-05` and the fallback-anchor row `A-01`. A name written by `=`, a
//! compound or logical assignment, `++`/`--`, a declarator name, a `for` head or a `catch` parameter, and a leaf of a declaration or
//! assignment destructuring pattern, only rebind the variable (S-01..S-03, S-05); a member or subscript target of an assignment is a
//! storage write (S-04); a default value inside a pattern, `u.x++`, `for (u.x of ..)` and every read position are unsupported. A
//! fallback anchor (the line-start def minted for a multi-line destructuring alias, `A-01`) is a binding write when its line starts in a
//! declaration. Unlike the other languages JS reaching definitions already ignore a twin's field writes for kills, but a binding twin of
//! the target's own path kills the target's earlier defs and collapses with same-line defs, and that spurious kill is what keeps a write
//! through an alias that option A (#783) does not certify off `exact`. Two refusals beyond R1 therefore keep the twin: R2, the syntactic
//! may-hold closure of `js_hold.rs` (the target's connected component in the owner's co-occurrence graph; the twins stay iff a write-census
//! written root other than the target, or a non-name write root, falls in it; a census-refused owner keeps every twin; out of model: module
//! state through calls, a captured name written in another owner, a `with` body), and `X-PARAM`,
//! every occurrence below a `formal_parameters` list of the owner (#761: `function f(o, u = o, v = u.x++)` keeps `o@1->o@3` off
//! `exact` through the binding twin, and option A excludes parameter defaults). R1 collects property names, object keys and shorthand, method and field names, labels
//! and type names, and import and export specifier names. Residuals: a call that mutates the alias (`Object.assign(u, ..)`, `u.m()`)
//! is out of model as for `Exact`; the twin's accidental doubt of those edges is gone.
//!
//! Specification: the Java rows `J-01..J-06`, Go rows `G-01..G-04`, Rust rows `R-01..R-05` and
//! Python rows `Y-01..Y-03`, C/C++ rows `C-01..C-10` and JS/TS rows `S-01..S-05`, `A-01` of the slice-0 classifier table (`CLASSIFIER.md`); `*_tests.rs` pin them,
//! including parity with the slice-0 oracle's pinned cases. Go `G-03`/`G-04` and Rust `R-04` are
//! `Unsupported` (the oracle's `storage-write` when pointer or reference evidence exists keeps the
//! twin the same way) and Rust `R-02` is always `Unsupported` (more conservative than the oracle).
use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod c_family;
#[cfg(test)]
mod c_family_effect_tests;
mod c_family_hazards;
#[cfg(test)]
mod c_family_tests;
mod go;
#[cfg(test)]
mod go_tests;
mod java;
#[cfg(test)]
mod java_tests;
mod js;
#[cfg(test)]
mod js_effect_tests;
mod js_hold;
#[cfg(test)]
mod js_tests;
mod python;
#[cfg(test)]
mod python_tests;
#[cfg(test)]
mod receipt_diff_tests;
mod rust;
#[cfg(test)]
mod rust_tests;
mod shared;
#[cfg(test)]
mod test_support;

#[cfg(test)]
thread_local! {
    static POLICY_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Test-only seam: run `f` with the occurrence policy disabled on this thread, so a differential
/// against the frozen reference isolates what the policy does not touch (the #630 pruning rules).
#[cfg(test)]
pub(crate) fn without_policy<R>(f: impl FnOnce() -> R) -> R {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            POLICY_OFF.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(POLICY_OFF.with(|c| c.replace(true)));
    f()
}

/// What an lvalue occurrence does to storage (closed set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    BindingWrite,
    StorageWrite,
    Unsupported,
}

/// A classified occurrence with the classifier-table row that decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Classified {
    pub disposition: Disposition,
    pub rule: &'static str,
}

/// The per-owner twin decisions, computed once and shared by registration, the fallback anchors
/// and edge emission.
#[derive(Default)]
pub(super) struct TwinPolicy {
    binding_writes: BTreeSet<usize>,
    anchors: BTreeSet<(String, usize)>,
}

impl TwinPolicy {
    /// Classify every lvalue span of the owner whose base is an alias. Java, Go, Rust, Python, C, C++
    /// and JS/TS/TSX are enabled; any other language yields the empty policy without inspecting a single span.
    pub(super) fn for_owner(
        parsed: &ParsedFile,
        owner: &Node<'_>,
        lvalue_spans: &[PathSpan],
        alias_map: &BTreeMap<String, String>,
        raw_aliases: &[(String, String, usize)],
    ) -> Self {
        let mut policy = Self::default();
        #[cfg(test)]
        if POLICY_OFF.with(|c| c.get()) {
            return policy;
        }
        let lang = parsed.language;
        if !matches!(
            lang,
            Language::Java
                | Language::Go
                | Language::Rust
                | Language::Python
                | Language::C
                | Language::Cpp
                | Language::JavaScript
                | Language::TypeScript
                | Language::Tsx
        ) || alias_map.is_empty()
        {
            return policy;
        }
        // Rust: the non-local item names visible to the owner, computed once on first use.
        let items = std::cell::OnceCell::new();
        // C and C++: the enumerators, macros and functions the file declares, computed once on first use.
        let defined = std::cell::OnceCell::new();
        let classify = |span: &PathSpan| match lang {
            Language::Go => go::classify(parsed, span),
            Language::Rust => rust::classify_with(
                parsed,
                span,
                items.get_or_init(|| rust::item_names(parsed, owner)),
            ),
            Language::Python => python::classify(parsed, span),
            Language::C | Language::Cpp => c_family::classify(parsed, owner, span),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                js::classify_in(parsed, Some(owner), span)
            }
            _ => java::classify(parsed, span),
        };
        let collect = || match lang {
            Language::Go => go::member_name_tokens(parsed, owner),
            Language::Rust => rust::member_name_tokens(parsed, owner),
            Language::Python => python::member_name_tokens(parsed, owner),
            Language::C | Language::Cpp => c_family_hazards::tokens(
                parsed,
                owner,
                defined.get_or_init(|| c_family_hazards::file_names(parsed)),
            ),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                js::member_name_tokens(parsed, owner)
            }
            _ => java::member_name_tokens(parsed, owner),
        };
        let mut member_tokens: Option<BTreeSet<String>> = None;
        // JS/TS R2: the syntactic may-hold closure of the alias's target base (`js_hold.rs`); a census-refused
        // owner keeps every twin.
        let hold = std::cell::RefCell::new(None::<js_hold::Hold>);
        let written_through = |alias: &str| {
            parsed.language.is_js_family()
                && alias_map.get(alias).is_some_and(|t| {
                    let target = AccessPath::from_expr(t).base;
                    hold.borrow_mut()
                        .get_or_insert_with(|| js_hold::Hold::new(parsed, owner))
                        .keeps(&target)
                })
        };
        for span in lvalue_spans {
            let Some(target) = alias_map.get(&span.path.base) else {
                continue;
            };
            if !span.path.is_simple()
                || classify(span).disposition != Disposition::BindingWrite
                || written_through(&span.path.base)
            {
                continue;
            }
            // R1: refuse when a member-name token of the target's spelling exists in the owner.
            let tokens = member_tokens.get_or_insert_with(collect);
            if !tokens.contains(&AccessPath::from_expr(target).base) {
                policy.binding_writes.insert(span.start_byte);
            }
        }
        // A-01: fallback anchors (JS/TS multi-line destructuring declarations).
        if parsed.language.is_js_family() {
            for (alias, _target, line) in raw_aliases {
                let Some(resolved) = alias_map.get(alias) else {
                    continue;
                };
                let root = AccessPath::from_expr(resolved);
                if !root.has_fields()
                    || !js::anchor_is_declaration(parsed, *line)
                    || written_through(alias)
                {
                    continue;
                }
                let tokens = member_tokens.get_or_insert_with(collect);
                if !tokens.contains(&root.base) {
                    policy.anchors.insert((alias.clone(), *line));
                }
            }
        }
        policy
    }

    /// True when the occurrence at `start_byte` only (re)binds the alias variable: it mints no
    /// target twin def and no twin edge.
    pub(super) fn suppresses_span(&self, start_byte: usize) -> bool {
        self.binding_writes.contains(&start_byte)
    }

    /// Fallback anchor decision, keyed by the alias line. Anchors exist only for aliases whose
    /// target has fields (JS/TS destructuring); alias discovery for the other enabled languages
    /// only pairs a plain identifier with a plain identifier, so only JS/TS/TSX suppress one (`A-01`).
    pub(super) fn suppresses_anchor(&self, alias: &str, line: usize) -> bool {
        self.anchors.contains(&(alias.to_string(), line))
    }
}
