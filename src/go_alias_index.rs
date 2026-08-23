//! Roadmap #14 slice 4 (spec §5): profile/clause-scoped type-alias index.
//!
//! Every `type A = <expr>` is recorded per declaring
//! `(package_dir, package_clause, type_name)` alongside `Defined` specs, with
//! the RHS kept as a CANONICALIZED TYPE EXPRESSION (never a named leaf).
//! Expansion substitutes the entire canonical RHS before `Local`/`Qualified`
//! tokens are produced, transitively and cycle-guarded, and is allowed ONLY
//! when every EXACTLY visible declaration variant is an `Alias` whose RHS
//! canonicalizes identically. Anything else fails closed with an
//! `AliasUnresolvedReason`.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GoAliasUnresolvedReason {
    /// At least one exactly-visible variant is a `Defined` type while others
    /// are `Alias` (or the aliases disagree) — cannot prove one identity.
    DefinedVariant,
    /// Build-profile visibility of some exactly-matching variant is provable
    /// only as "visible", not "exact".
    ProfileUncertain,
    /// The alias graph reachable from this leaf contains a cycle.
    Cycle,
    /// A parameterized alias was instantiated with the wrong number of type
    /// arguments.
    Arity,
    /// The alias RHS never canonicalized (unsupported constraint, unknown
    /// syntax) or its provider provenance is incomplete.
    Unresolvable,
}

impl GoAliasUnresolvedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefinedVariant => "defined_variant",
            Self::ProfileUncertain => "profile_uncertain",
            Self::Cycle => "cycle",
            Self::Arity => "arity",
            Self::Unresolvable => "unresolvable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoAliasKind {
    /// `type A = <rhs>` (or a parameterized alias). `rhs` is the canonicalized
    /// type expression in the declaring file's import context; parameter
    /// references appear as `%N%` placeholders. `None` = the RHS failed to
    /// canonicalize (unsupported constraint / unknown syntax) → any expansion
    /// attempt fails closed.
    Alias {
        rhs: Option<String>,
        type_params: usize,
    },
    /// `type A T` (a definition, not an alias).
    Defined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoAliasVariant {
    pub kind: GoAliasKind,
    pub declaring_file: String,
}

/// `(package_dir, package_clause, type_name)` -> declaration variants (one per
/// declaring file/build profile).
pub type GoAliasVariants = BTreeMap<(String, String, String), Vec<GoAliasVariant>>;

#[derive(Debug, Default)]
pub struct GoAliasIndex {
    pub variants: GoAliasVariants,
    /// Effective import path -> package dirs proving it (P10 identities).
    pub dirs_by_path: BTreeMap<String, BTreeSet<String>>,
    /// Package dir -> declared clauses (exact-key lookup; a clause-RANGE scan
    /// would match every name in the directory because tuple ordering
    /// compares the clause before the type name).
    pub clauses_by_dir: BTreeMap<String, BTreeSet<String>>,
}

/// Shared live telemetry counters (one per provider build; `Rc` so each
/// consuming file can hold its own expansion context while aggregating).
#[derive(Clone, Default)]
pub struct AliasTelemetry {
    pub expanded: std::rc::Rc<std::cell::Cell<usize>>,
    pub unresolved: std::rc::Rc<std::cell::RefCell<BTreeMap<GoAliasUnresolvedReason, usize>>>,
}

/// A successfully classified alias: its canonical RHS and arity.
#[derive(Debug, Clone)]
pub struct GoAliasRhs {
    pub text: String,
    pub type_params: usize,
}

/// Per-consumer expansion context: who is consuming (which file's visibility
/// rules apply) plus shared live telemetry counters.
pub struct AliasExpansionCtx<'a> {
    pub index: &'a GoAliasIndex,
    pub consumer_file: &'a str,
    pub consumer_dir: &'a str,
    pub consumer_clause: &'a str,
    pub profiles: &'a BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    pub telemetry: &'a AliasTelemetry,
}

enum Resolution {
    /// Not an alias visible here — treat the leaf normally.
    NotAnAlias,
    /// Fully-expanded canonical RHS.
    Expanded(String),
}

type Guard = BTreeSet<(String, String, String)>;

impl<'a> AliasExpansionCtx<'a> {
    fn record_unresolved(&self, reason: GoAliasUnresolvedReason) {
        *self
            .telemetry
            .unresolved
            .borrow_mut()
            .entry(reason)
            .or_insert(0) += 1;
    }

    fn own_key(&self, name: &str) -> (String, String, String) {
        (
            self.consumer_dir.to_string(),
            self.consumer_clause.to_string(),
            name.to_string(),
        )
    }

    /// True when EVERY recorded variant of `key` is `Defined` — an ordinary
    /// named type whose leaf behavior never involves alias visibility logic
    /// (SOL-W5: a build-tag certainty cap must not turn it into a gap).
    fn all_defined(&self, key: &(String, String, String)) -> bool {
        self.index.variants.get(key).is_some_and(|variants| {
            !variants.is_empty()
                && variants
                    .iter()
                    .all(|variant| variant.kind == GoAliasKind::Defined)
        })
    }

    /// True when ANY package-level declaration variant exists for `name` in
    /// the consumer's own (dir, clause) — used by canon_type to detect that a
    /// package declaration SHADOWS a predeclared name (SOL-W2).
    pub fn own_variants_exist(&self, name: &str) -> bool {
        self.index.variants.contains_key(&self.own_key(name))
    }

    /// Exactly-visible variants for an OWN-PACKAGE reference.
    fn own_variants(&self, name: &str) -> (Vec<&GoAliasVariant>, bool) {
        let key = self.own_key(name);
        if self.all_defined(&key) {
            return (Vec::new(), false);
        }
        self.filter_exact(key, |ctx, consumer_file, declaring_file| {
            crate::go_owner_partition::exact_declaration_visibility(
                &crate::resolution::GoOwnerIdentity {
                    package_dir: ctx.consumer_dir.to_string(),
                    package_clause: ctx.consumer_clause.to_string(),
                    name: name.to_string(),
                },
                consumer_file,
                crate::go_owner_partition::GoOwnerReferenceMode::Bare,
                declaring_file,
                ctx.profiles,
            )
        })
    }

    /// Exactly-visible variants for a QUALIFIED reference into the package
    /// addressed by `import_path` (test-clause declarations never export).
    fn qualified_variants(&self, import_path: &str, name: &str) -> (Vec<&GoAliasVariant>, bool) {
        let mut out = Vec::new();
        let mut uncertain = false;
        let Some(dirs) = self.index.dirs_by_path.get(import_path) else {
            return (out, false);
        };
        // EXACT keys only, via the clause index (never a clause-range scan).
        let mut saw_variant = false;
        let mut all_defined = true;
        for dir in dirs {
            let Some(clauses) = self.index.clauses_by_dir.get(dir) else {
                continue;
            };
            for clause in clauses {
                let Some(variants) =
                    self.index
                        .variants
                        .get(&(dir.clone(), clause.clone(), name.to_string()))
                else {
                    continue;
                };
                saw_variant = true;
                if variants.iter().any(|v| v.kind != GoAliasKind::Defined) {
                    all_defined = false;
                }
            }
        }
        if saw_variant && all_defined {
            return (out, false);
        }
        for dir in dirs {
            let Some(clauses) = self.index.clauses_by_dir.get(dir) else {
                continue;
            };
            for clause in clauses {
                let Some(variants) =
                    self.index
                        .variants
                        .get(&(dir.clone(), clause.clone(), name.to_string()))
                else {
                    continue;
                };
                let (mut exact, unc) =
                    self.filter_exact_with(variants, |ctx, consumer_file, declaring_file| {
                        crate::go_owner_partition::exact_cross_package_visibility(
                            consumer_file,
                            declaring_file,
                            ctx.profiles,
                        )
                    });
                out.append(&mut exact);
                uncertain |= unc;
            }
        }
        (out, uncertain)
    }

    fn filter_exact<'v>(
        &'v self,
        key: (String, String, String),
        visible: impl Fn(&AliasExpansionCtx<'a>, &str, &str) -> (bool, bool),
    ) -> (Vec<&'v GoAliasVariant>, bool) {
        match self.index.variants.get(&key) {
            Some(variants) => self.filter_exact_with(variants, visible),
            None => (Vec::new(), false),
        }
    }

    fn filter_exact_with<'v>(
        &'v self,
        variants: &'v [GoAliasVariant],
        visible: impl Fn(&AliasExpansionCtx<'a>, &str, &str) -> (bool, bool),
    ) -> (Vec<&'v GoAliasVariant>, bool) {
        let mut exact = Vec::new();
        let mut uncertain = false;
        for variant in variants {
            let (visible, is_exact) = visible(self, self.consumer_file, &variant.declaring_file);
            if !visible {
                continue;
            }
            if !is_exact {
                uncertain = true;
                continue;
            }
            exact.push(variant);
        }
        (exact, uncertain)
    }

    /// Classify the exactly-visible variants of one name.
    fn classify(
        &self,
        variants: &[&GoAliasVariant],
        uncertain: bool,
    ) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        if uncertain {
            self.record_unresolved(GoAliasUnresolvedReason::ProfileUncertain);
            return Err(GoAliasUnresolvedReason::ProfileUncertain);
        }
        let mut kinds: Vec<(&GoAliasKind, Option<(&String, usize)>)> = Vec::new();
        for variant in variants {
            match &variant.kind {
                GoAliasKind::Alias { rhs, type_params } => {
                    kinds.push((&variant.kind, rhs.as_ref().map(|text| (text, *type_params))));
                }
                GoAliasKind::Defined => kinds.push((&variant.kind, None)),
            }
        }
        let all_alias = kinds
            .iter()
            .all(|(k, _)| matches!(k, GoAliasKind::Alias { .. }));
        if !all_alias {
            // Mixed Alias/Defined across exactly-visible profiles, or plainly a
            // Defined type. Only the MIX is a failure: an all-Defined name is
            // an ordinary defined type and keeps the existing leaf behavior.
            let any_alias = kinds
                .iter()
                .any(|(k, _)| matches!(k, GoAliasKind::Alias { .. }));
            if any_alias {
                self.record_unresolved(GoAliasUnresolvedReason::DefinedVariant);
                return Err(GoAliasUnresolvedReason::DefinedVariant);
            }
            return Ok(None);
        }
        // All Alias: every (RHS, arity) pair — including "failed to
        // canonicalize" — must agree across exactly-visible profiles.
        let first = kinds[0].1;
        if kinds[1..].iter().any(|other| other.1 != first) {
            self.record_unresolved(GoAliasUnresolvedReason::DefinedVariant);
            return Err(GoAliasUnresolvedReason::DefinedVariant);
        }
        match first {
            Some((text, type_params)) => Ok(Some(GoAliasRhs {
                text: text.clone(),
                type_params,
            })),
            None => {
                self.record_unresolved(GoAliasUnresolvedReason::Unresolvable);
                Err(GoAliasUnresolvedReason::Unresolvable)
            }
        }
    }

    /// Expand an unqualified `name` visible from the consumer's own package.
    /// `Ok(None)` = not an alias (leaf keeps existing behavior).
    pub fn expand_own(&self, name: &str) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        let (variants, uncertain) = self.own_variants(name);
        if variants.is_empty() && !uncertain {
            return Ok(None);
        }
        self.note_expansion(&self.classify(&variants, uncertain)?)
    }

    /// Expand a qualified `import_path.name` reference.
    pub fn expand_qualified(
        &self,
        import_path: &str,
        name: &str,
    ) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        let (variants, uncertain) = self.qualified_variants(import_path, name);
        if variants.is_empty() && !uncertain {
            return Ok(None);
        }
        self.note_expansion(&self.classify(&variants, uncertain)?)
    }

    fn note_expansion(
        &self,
        resolved: &Option<GoAliasRhs>,
    ) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        if resolved.is_some() {
            self.telemetry
                .expanded
                .set(self.telemetry.expanded.get() + 1);
        }
        Ok(resolved.clone())
    }

    /// Parameterized-alias instantiation: bind `args` (already-canonicalized,
    /// alias-expanded strings) positionally to `%N%` placeholders.
    pub fn instantiate(
        &self,
        rhs: &GoAliasRhs,
        args: &[String],
    ) -> Result<String, GoAliasUnresolvedReason> {
        if rhs.type_params != args.len() {
            self.record_unresolved(GoAliasUnresolvedReason::Arity);
            return Err(GoAliasUnresolvedReason::Arity);
        }
        let mut out = rhs.text.clone();
        for (index, arg) in args.iter().enumerate() {
            out = out.replace(&format!("%{index}%"), arg);
        }
        if out.contains('%') {
            // Unbound type-parameter reference left in the RHS.
            self.record_unresolved(GoAliasUnresolvedReason::Unresolvable);
            return Err(GoAliasUnresolvedReason::Unresolvable);
        }
        Ok(out)
    }

    /// Transitively expand every alias-named leaf inside an already-canonical
    /// type expression string. Cycle-guarded per alias key.
    pub fn expand_canonical(&self, text: &str) -> Result<String, GoAliasUnresolvedReason> {
        let mut guard: Guard = BTreeSet::new();
        self.expand_canonical_guarded(text, &mut guard)
    }

    fn fail(&self, reason: GoAliasUnresolvedReason) -> GoAliasUnresolvedReason {
        self.record_unresolved(reason);
        reason
    }

    fn expand_canonical_guarded(
        &self,
        text: &str,
        guard: &mut Guard,
    ) -> Result<String, GoAliasUnresolvedReason> {
        let tokens = lex_canonical(text)?;
        let mut out = String::new();
        for token in tokens {
            match token {
                CanonicalToken::Text(t) => out.push_str(&t),
                CanonicalToken::Name { marker, path, name } => {
                    let spliced = match marker {
                        None => self.expand_own(&name),
                        Some(_) => {
                            let resolved = self.resolve_qualified_name(&path, &name)?;
                            Ok(match resolved {
                                Resolution::NotAnAlias => None,
                                Resolution::Expanded(rhs) => {
                                    Some(crate::go_alias_index::GoAliasRhs {
                                        text: rhs,
                                        type_params: 0,
                                    })
                                }
                            })
                        }
                    };
                    match spliced {
                        Err(reason) => return Err(self.fail(reason)),
                        Ok(None) => {
                            if let Some(m) = marker {
                                out.push(m);
                                out.push_str(&path);
                                out.push_str("::");
                            }
                            out.push_str(&name);
                        }
                        Ok(Some(rhs)) => {
                            let key = self.alias_key_for(marker, &path, &name);
                            let inserted = key.as_ref().map(|key| guard.insert(key.clone()));
                            if inserted == Some(false) {
                                return Err(self.fail(GoAliasUnresolvedReason::Cycle));
                            }
                            // Path-scoped: the guard entry is removed on EVERY
                            // exit path so sibling leaves can reuse the alias.
                            let expanded = self.expand_canonical_guarded(&rhs.text, guard);
                            if let Some(key) = key {
                                guard.remove(&key);
                            }
                            out.push_str(&expanded?);
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn resolve_qualified_name(
        &self,
        path: &str,
        name: &str,
    ) -> Result<Resolution, GoAliasUnresolvedReason> {
        match self.expand_qualified(path, name) {
            Err(reason) => Err(reason),
            Ok(Some(rhs)) => Ok(Resolution::Expanded(rhs.text)),
            Ok(None) => Ok(Resolution::NotAnAlias),
        }
    }

    fn alias_key_for(
        &self,
        marker: Option<char>,
        path: &str,
        name: &str,
    ) -> Option<(String, String, String)> {
        match marker {
            None => Some(self.own_key(name)),
            Some('@') => self
                .index
                .dirs_by_path
                .get(path)
                .and_then(|dirs| dirs.iter().next())
                .map(|dir| (dir.clone(), String::new(), name.to_string())),
            Some('~') => Some((
                self.consumer_dir.to_string(),
                self.consumer_clause.to_string(),
                name.to_string(),
            )),
            _ => None,
        }
    }
}

enum CanonicalToken {
    Text(String),
    Name {
        /// `~` Local provenance, `@` Qualified, None = bare.
        marker: Option<char>,
        path: String,
        name: String,
    },
}

fn lex_canonical(text: &str) -> Result<Vec<CanonicalToken>, GoAliasUnresolvedReason> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let first = rest.chars().next().unwrap();
        if first == '~' || first == '@' {
            let after_marker = &rest[first.len_utf8()..];
            let Some(separator) = after_marker.find("::") else {
                return Err(GoAliasUnresolvedReason::Unresolvable);
            };
            let path = &after_marker[..separator];
            let after_separator = &after_marker[separator + 2..];
            let Some(name_len) = identifier_len(after_separator) else {
                return Err(GoAliasUnresolvedReason::Unresolvable);
            };
            tokens.push(CanonicalToken::Name {
                marker: Some(first),
                path: path.to_string(),
                name: after_separator[..name_len].to_string(),
            });
            rest = &after_separator[name_len..];
        } else if first == '_' || first.is_alphabetic() {
            let name_len = identifier_len(rest).ok_or(GoAliasUnresolvedReason::Unresolvable)?;
            tokens.push(CanonicalToken::Name {
                marker: None,
                path: String::new(),
                name: rest[..name_len].to_string(),
            });
            rest = &rest[name_len..];
        } else {
            let len = first.len_utf8();
            push_text(&mut tokens, &rest[..len]);
            rest = &rest[len..];
        }
    }
    Ok(tokens)
}

fn push_text(tokens: &mut Vec<CanonicalToken>, piece: &str) {
    if let Some(CanonicalToken::Text(last)) = tokens.last_mut() {
        last.push_str(piece);
    } else {
        tokens.push(CanonicalToken::Text(piece.to_string()));
    }
}

fn identifier_len(text: &str) -> Option<usize> {
    text.char_indices()
        .take_while(|(_, ch)| *ch == '_' || ch.is_alphanumeric())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
}
