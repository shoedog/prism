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
    /// Declaring scope of the resolved alias: bare leaves inside `text`
    /// resolve in THIS package, not the consumer's (sol-r2-3).
    pub resolved_dir: String,
    pub resolved_clause: String,
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

type Guard = BTreeSet<(String, String, String)>;

fn ctx_profile<'p>(
    ctx: &'p AliasExpansionCtx<'_>,
    file: &str,
) -> Option<&'p crate::go_build_profile::GoBuildProfile> {
    ctx.profiles.get(file)
}

impl<'a> AliasExpansionCtx<'a> {
    fn record_unresolved(&self, reason: GoAliasUnresolvedReason) {
        if std::env::var("PRISM_ALIAS_DEBUG").is_ok() {
            eprintln!(
                "DBG unresolved {:?} at {}::{}",
                reason, self.consumer_dir, self.consumer_clause
            );
        }
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
    /// True when an EXACTLY VISIBLE package-level declaration of `name`
    /// exists for the consumer (terra-r2-1 / sol-r2-2: an invisible Linux- or
    /// test-only declaration does NOT shadow a predeclared name).
    /// True when an EXACTLY VISIBLE package-level declaration of `name`
    /// exists for the consumer (terra-r2-1 / sol-r2-2: an invisible Linux- or
    /// test-only declaration does NOT shadow a predeclared name). Deliberately
    /// bypasses the all-`Defined` expansion shortcut: a Defined declaration
    /// named `byte`/`rune` still SHADOWS the predeclared identifier.
    pub fn own_variants_exist(&self, name: &str) -> bool {
        let key = self.own_key(name);
        let Some(variants) = self.index.variants.get(&key) else {
            return false;
        };
        let Some(consumer) = self.profiles.get(self.consumer_file) else {
            return false;
        };
        variants.iter().any(|variant| {
            let Some(declaring) = self.profiles.get(&variant.declaring_file) else {
                return false;
            };
            shadows_consumer_profile(consumer, declaring)
        })
    }

    /// Exactly-visible variants for an OWN-PACKAGE reference.
    fn own_variants_in(&self, dir: &str, clause: &str, name: &str) -> (Vec<&GoAliasVariant>, bool) {
        let key = (dir.to_string(), clause.to_string(), name.to_string());
        if self.all_defined(&key) {
            return (Vec::new(), false);
        }
        let (variants, mut uncertain) =
            self.filter_exact(key, |ctx, consumer_file, declaring_file| {
                // A `_test`-clause declaration is never visible to a production
                // consumer of the same package (and vice versa).
                let consumer_is_test = ctx
                    .profiles
                    .get(consumer_file)
                    .is_some_and(|profile| profile.is_test_file);
                let declaring_is_test = ctx
                    .profiles
                    .get(declaring_file)
                    .is_some_and(|profile| profile.is_test_file);
                if consumer_is_test != declaring_is_test {
                    return (false, false);
                }
                crate::go_owner_partition::exact_declaration_visibility(
                    &crate::resolution::GoOwnerIdentity {
                        package_dir: dir.to_string(),
                        package_clause: clause.to_string(),
                        name: name.to_string(),
                    },
                    consumer_file,
                    crate::go_owner_partition::GoOwnerReferenceMode::Bare,
                    declaring_file,
                    ctx.profiles,
                )
            });
        // Profile-uniformity gate: an expansion is only sound when every
        // exactly-visible variant applies everywhere the consumer compiles,
        // or when a set of DISTINCTLY-constrained variants jointly covers the
        // consumer's build space with identical RHS. A single constrained
        // variant under an unconstrained consumer leaves the leaf's meaning
        // platform-dependent -> fail closed (terra-r2-1).
        if !variants.is_empty() {
            let consumer = ctx_profile(self, self.consumer_file);
            if let Some(consumer) = consumer {
                let unconstrained = |profile: &crate::go_build_profile::GoBuildProfile| {
                    profile.goos.is_none()
                        && profile.goarch.is_none()
                        && profile.build_expr.is_none()
                };
                let applies_everywhere = |profile: &crate::go_build_profile::GoBuildProfile| {
                    unconstrained(profile)
                        || (profile.goos == consumer.goos
                            && profile.goarch == consumer.goarch
                            && profile.build_expr == consumer.build_expr)
                };
                let any_universal = variants.iter().any(|variant| {
                    ctx_profile(self, &variant.declaring_file).is_some_and(applies_everywhere)
                });
                if !any_universal && unconstrained(consumer) {
                    let distinct_sets: BTreeSet<(
                        Option<String>,
                        Option<String>,
                        Option<crate::go_build_profile::BuildExpr>,
                    )> = variants
                        .iter()
                        .filter_map(|variant| {
                            ctx_profile(self, &variant.declaring_file).map(|profile| {
                                (
                                    profile.goos.clone(),
                                    profile.goarch.clone(),
                                    profile.build_expr.clone(),
                                )
                            })
                        })
                        .collect();
                    if distinct_sets.len() <= 1 {
                        uncertain = true;
                    }
                } else if !variants.iter().any(|variant| {
                    ctx_profile(self, &variant.declaring_file).is_some_and(applies_everywhere)
                }) && !unconstrained(consumer)
                {
                    // Constrained consumer whose own platform none of the
                    // variants speak for: fail closed.
                    uncertain = true;
                }
            }
        }
        (variants, uncertain)
    }

    fn own_variants(&self, name: &str) -> (Vec<&GoAliasVariant>, bool) {
        self.own_variants_in(self.consumer_dir, self.consumer_clause, name)
    }

    /// Exactly-visible variants for a QUALIFIED reference into the package
    /// addressed by `import_path` (test-clause declarations never export).
    fn qualified_variants(
        &self,
        import_path: &str,
        name: &str,
    ) -> (Vec<&GoAliasVariant>, bool, Option<(String, String)>) {
        let mut out = Vec::new();
        let mut uncertain = false;
        let mut scope: Option<(String, String)> = None;
        let Some(dirs) = self.index.dirs_by_path.get(import_path) else {
            return (out, false, None);
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
            return (out, false, None);
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
                if scope.is_none() && !variants.is_empty() {
                    scope = Some((dir.clone(), clause.clone()));
                }
                let (mut exact, unc) =
                    self.filter_exact_with(variants, |ctx, consumer_file, declaring_file| {
                        // sol-r2-1: an IMPORTED package never exposes
                        // test-clause declarations — including an external-test
                        // package's own same-directory re-declarations.
                        if ctx
                            .profiles
                            .get(declaring_file)
                            .is_some_and(|profile| profile.is_test_file)
                        {
                            return (false, false);
                        }
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
        (out, uncertain, scope)
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
        resolved_dir: &str,
        resolved_clause: &str,
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
                resolved_dir: resolved_dir.to_string(),
                resolved_clause: resolved_clause.to_string(),
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
        self.expand_scoped(self.consumer_dir, self.consumer_clause, name)
    }

    /// Expand an unqualified leaf as seen from `(dir, clause)` — used when
    /// walking an alias RHS, whose bare leaves live in the DECLARING package.
    pub fn expand_scoped(
        &self,
        dir: &str,
        clause: &str,
        name: &str,
    ) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        // Inside an aliased RHS the leaf belongs to the DECLARING package:
        // consumer-relative cross-package visibility does not apply (it would
        // hide the package's own names). Mixed variants still fail closed in
        // classify.
        let in_consumer_scope = dir == self.consumer_dir && clause == self.consumer_clause;
        let key = (dir.to_string(), clause.to_string(), name.to_string());
        let (variants, uncertain) = if in_consumer_scope {
            self.own_variants(name)
        } else if self.all_defined(&key) {
            (Vec::new(), false)
        } else {
            match self.index.variants.get(&key) {
                Some(variants) => (variants.iter().collect(), false),
                None => (Vec::new(), false),
            }
        };
        if variants.is_empty() && !uncertain {
            return Ok(None);
        }
        self.note_expansion(&self.classify(&variants, uncertain, dir, clause)?)
    }

    /// Expand a qualified `import_path.name` reference.
    pub fn expand_qualified(
        &self,
        import_path: &str,
        name: &str,
    ) -> Result<Option<GoAliasRhs>, GoAliasUnresolvedReason> {
        let (variants, uncertain, scope) = self.qualified_variants(import_path, name);
        if variants.is_empty() && !uncertain {
            return Ok(None);
        }
        let Some((dir, clause)) = scope else {
            return Ok(None);
        };
        self.note_expansion(&self.classify(&variants, uncertain, &dir, &clause)?)
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
        self.expand_canonical_guarded(
            text,
            &mut guard,
            ScopeRef {
                dir: self.consumer_dir,
                clause: self.consumer_clause,
            },
        )
    }

    fn fail(&self, reason: GoAliasUnresolvedReason) -> GoAliasUnresolvedReason {
        self.record_unresolved(reason);
        reason
    }

    fn expand_canonical_guarded(
        &self,
        text: &str,
        guard: &mut Guard,
        scope: ScopeRef<'_>,
    ) -> Result<String, GoAliasUnresolvedReason> {
        let mut out = String::new();
        let mut rest = text;
        while !rest.is_empty() {
            let first = rest.chars().next().unwrap();
            let name_scan = if first == '~' || first == '@' {
                let after_marker = &rest[first.len_utf8()..];
                let Some(separator) = after_marker.find("::") else {
                    return Err(self.fail(GoAliasUnresolvedReason::Unresolvable));
                };
                let path = &after_marker[..separator];
                after_marker[separator + 2..]
                    .chars()
                    .next()
                    .filter(|ch| *ch == '_' || ch.is_alphanumeric())
                    .map(|_| Some(first))
                    .unwrap_or(None)
                    .map(|marker| (Some(marker), path.to_string()))
            } else if first == '_' || first.is_alphabetic() {
                Some((None, String::new()))
            } else {
                None
            };
            let Some((marker, path)) = name_scan else {
                out.push(first);
                rest = &rest[first.len_utf8()..];
                continue;
            };
            // Consume the identifier.
            let ident_text = if marker.is_some() {
                &rest[(1 + path.len() + 2)..] // marker + path + "::"
            } else {
                rest
            };
            let name_len =
                identifier_len(ident_text).ok_or(GoAliasUnresolvedReason::Unresolvable)?;
            let name = &ident_text[..name_len];
            let after_name = &ident_text[name_len..];

            // Generic application lookahead: `Name[...]` (balanced brackets).
            let (args_inner, after_args) = if after_name.starts_with('[') {
                match balanced_bracket_span(after_name) {
                    Some((inner, remainder)) => (Some(inner), remainder),
                    None => return Err(self.fail(GoAliasUnresolvedReason::Unresolvable)),
                }
            } else {
                (None, after_name)
            };

            // Resolve the base name against the alias index. Bare leaves and
            // local tokens resolve in the CURRENT SCOPE — the DECLARING
            // package when walking an aliased RHS, not always the consumer.
            let resolved = match marker {
                None | Some('~') => self.expand_scoped(scope.dir, scope.clause, name),
                Some('@') => self.expand_qualified(&path, name),
                _ => unreachable!(),
            };
            match resolved {
                Err(reason) => return Err(self.fail(reason)),
                Ok(None) => {
                    if let Some(m) = marker {
                        out.push(m);
                        out.push_str(&path);
                        out.push_str("::");
                    }
                    out.push_str(name);
                    if let Some(inner) = args_inner {
                        out.push('[');
                        out.push_str(&self.expand_canonical_guarded(inner, guard, scope)?);
                        out.push(']');
                    }
                    rest = after_args;
                }
                Ok(Some(rhs)) => {
                    // sol-r2-3: the guard keys by the RESOLVED DECLARATION's
                    // identity (dir, clause, name).
                    let key = (
                        rhs.resolved_dir.clone(),
                        rhs.resolved_clause.clone(),
                        name.to_string(),
                    );
                    if !guard.insert(key.clone()) {
                        return Err(self.fail(GoAliasUnresolvedReason::Cycle));
                    }
                    // SOL-r2-4: a parameterized alias reached THROUGH another
                    // alias's canonical string still instantiates its type
                    // arguments (arity-checked), then expands transitively.
                    // Arguments were written at the CURRENT scope.
                    let next_scope = ScopeRef {
                        dir: &rhs.resolved_dir,
                        clause: &rhs.resolved_clause,
                    };
                    let expanded = if rhs.type_params > 0 {
                        let Some(inner) = args_inner else {
                            guard.remove(&key);
                            return Err(self.fail(GoAliasUnresolvedReason::Arity));
                        };
                        let args = split_top_level_args(inner)?;
                        let mut expanded_args = Vec::with_capacity(args.len());
                        for arg in &args {
                            expanded_args.push(self.expand_canonical_guarded(arg, guard, scope)?);
                        }
                        let spliced = self
                            .instantiate(&rhs, &expanded_args)
                            .map_err(|reason| self.fail(reason));
                        match spliced {
                            Err(reason) => {
                                guard.remove(&key);
                                return Err(reason);
                            }
                            Ok(spliced) => {
                                self.expand_canonical_guarded(&spliced, guard, next_scope)
                            }
                        }
                    } else {
                        let expanded = self.expand_canonical_guarded(&rhs.text, guard, next_scope);
                        if expanded.is_err() {
                            guard.remove(&key);
                        }
                        expanded
                    };
                    guard.remove(&key);
                    out.push_str(&expanded?);
                    rest = after_args;
                }
            }
        }
        Ok(out)
    }
}

/// The package a canonical string's bare leaves resolve in.
#[derive(Clone, Copy)]
struct ScopeRef<'s> {
    dir: &'s str,
    clause: &'s str,
}

/// Does `declaring`'s build profile apply EVERYWHERE the consumer compiles?
/// Only then does it shadow a predeclared identifier for that consumer
/// (terra-r2-1 / sol-r2-2): a linux-only `type byte = …` does NOT shadow
/// `byte` inside unconstrained (both-platforms) files, and a `_test`-only
/// declaration never shadows production code.
fn shadows_consumer_profile(
    consumer: &crate::go_build_profile::GoBuildProfile,
    declaring: &crate::go_build_profile::GoBuildProfile,
) -> bool {
    if consumer.package_clause != declaring.package_clause {
        return false;
    }
    if declaring.is_test_file != consumer.is_test_file {
        // An internal `_test` consumer still sees ordinary declarations.
        return !declaring.is_test_file;
    }
    declaring.goos.is_none() && declaring.goarch.is_none() && declaring.build_expr.is_none()
        || (declaring.goos == consumer.goos
            && declaring.goarch == consumer.goarch
            && declaring.build_expr == consumer.build_expr)
}

/// Split `[a, map[k]v, []T]`-style top-level arguments on commas at bracket
/// depth zero.
fn split_top_level_args(inner: &str) -> Result<Vec<String>, GoAliasUnresolvedReason> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in inner.chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if trimmed.is_empty() {
                    return Err(GoAliasUnresolvedReason::Unresolvable);
                }
                args.push(trimmed.to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if trimmed.is_empty() {
        return Err(GoAliasUnresolvedReason::Unresolvable);
    }
    args.push(trimmed.to_string());
    Ok(args)
}

/// Given text starting with `[`, return `(inner, remainder-after-matching-])`.
fn balanced_bracket_span(text: &str) -> Option<(&str, &str)> {
    debug_assert!(text.starts_with('['));
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((&text[1..index], &text[index + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn identifier_len(text: &str) -> Option<usize> {
    text.char_indices()
        .take_while(|(_, ch)| *ch == '_' || ch.is_alphanumeric())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
}
