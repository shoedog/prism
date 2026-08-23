//! Roadmap #14 slice 4 part B (and the #17-narrow R1(b) record): an
//! owner/profile-keyed promoted-selector SNAPSHOT.
//!
//! Keyed by the outer owner's P10 identity `(package_dir, package_clause,
//! name)`; per DECLARING FILE/PROFILE it records the unordered set of embedded
//! fields as `(pointer-ness, RESOLVED embedded owner identity, selector name)`
//! (a `type A = B` embedded as `S{A}` exposes selector `A` with resolved
//! identity B), ordinary field selector names, own-method names, and — for
//! each promoted method — target owner identity, depth, field-shadow result
//! and `value_method_set` bit.
//!
//! FOUNDATION ONLY: nothing here is consumed for routing in this slice. The
//! four known profile-safety axes (embed tuple, ordinary fields, own methods,
//! embedded-alias selector names) are NECESSARY, not proven sufficient; any
//! fifth axis discovered must become a conflict.

use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoPromotionVerdict {
    /// Every declaration of this owner (and every owner on every promotion
    /// path) carries identical facts across build profiles.
    Consistent,
    /// At least one profile-safety axis disagrees (or cannot be proven) on
    /// this owner or any owner along a promotion path. Fail closed.
    ProfileConflict,
}

/// One embedded field: pointer-ness, resolved embedded OWNER identity (None =
/// unresolvable/anonymous → conflict), and the SELECTOR name as written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct GoEmbedKey {
    pub is_pointer: bool,
    /// Resolved embedded OWNER identity (None = unresolvable/anonymous, or an
    /// embedded INTERFACE — those defer to interface dispatch, not conflict).
    pub resolved_owner: Option<GoOwnerIdentity>,
    pub selector: String,
    /// True when this embedded field names a known in-repo INTERFACE.
    pub is_interface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct GoOwnMethodInfo {
    pub function_id: crate::call_graph::FunctionId,
    pub is_pointer_receiver: bool,
}

/// Declaration facts for ONE declaring file/profile of an owner.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct GoDeclarationFacts {
    pub embeds: BTreeSet<GoEmbedKey>,
    pub field_names: BTreeSet<String>,
    pub own_methods: BTreeMap<String, GoOwnMethodInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GoPromotedMethodInfo {
    pub target_owner: GoOwnerIdentity,
    pub function_id: crate::call_graph::FunctionId,
    pub depth: usize,
    pub shadowed_by_field: bool,
    pub value_method_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GoOwnerSnapshotEntry {
    /// Declaring file -> facts (one entry per declaring file/profile).
    pub declarations: BTreeMap<String, GoDeclarationFacts>,
    pub verdict: GoPromotionVerdict,
    /// Which axes conflicted (diagnostic; empty when Consistent).
    pub conflict_axes: BTreeSet<String>,
    /// Promoted methods — populated only when the whole path is Consistent.
    pub promoted: BTreeMap<String, GoPromotedMethodInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GoPromotedSelectorSnapshot {
    pub owners: BTreeMap<GoOwnerIdentity, GoOwnerSnapshotEntry>,
}

impl GoPromotedSelectorSnapshot {
    pub fn owner_count(&self) -> usize {
        self.owners.len()
    }

    pub fn conflict_count(&self) -> usize {
        self.owners
            .values()
            .filter(|owner| owner.verdict == GoPromotionVerdict::ProfileConflict)
            .count()
    }

    pub fn promoted_method_count(&self) -> usize {
        self.owners.values().map(|owner| owner.promoted.len()).sum()
    }
}

/// Inputs to the snapshot computation, all provider projections captured at
/// dispatch-build time.
pub struct SnapshotInputs<'a> {
    pub struct_declarations: &'a crate::go_owner_partition::GoStructDeclarations,
    pub method_declarations: &'a crate::go_owner_partition::GoMethodDeclarations,
    /// Proven local embed targets: ((embedding owner, selector)) -> target.
    pub field_targets: &'a BTreeMap<(GoOwnerIdentity, String), crate::resolution::GoFieldTarget>,
    pub profiles: &'a BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    /// Effective import path -> package dirs proving it (P10 identities).
    pub dirs_by_path: &'a BTreeMap<String, BTreeSet<String>>,
    /// Declaring-file import alias -> effective path (signature semantics).
    pub imports_by_file: &'a BTreeMap<String, BTreeMap<String, String>>,
    /// Owners known to be INTERFACES (embedded interfaces defer to interface
    /// dispatch instead of counting as unresolvable embeds).
    pub interface_owners: &'a BTreeSet<GoOwnerIdentity>,
    /// Per-file interface declarations (for embedded-interface profile
    /// comparison: the method-name surface must not vary by profile).
    pub interface_declarations: &'a crate::go_owner_partition::GoInterfaceDeclarations,
    /// The slice-4 alias index, for resolving `type A = B` embedded selectors
    /// to their owner identity (SOL-W6).
    pub alias_index: &'a crate::go_alias_index::GoAliasIndex,
}

/// The interface's method-name surface is identical across every declaring
/// file/profile.
fn interface_profile_consistent(inputs: &SnapshotInputs<'_>, iface: &GoOwnerIdentity) -> bool {
    let Some(declarations) = inputs.interface_declarations.get(iface) else {
        return false;
    };
    let mut surfaces: BTreeSet<BTreeSet<String>> = BTreeSet::new();
    for declaration in declarations {
        surfaces.insert(declaration.methods.iter().cloned().collect());
        if surfaces.len() > 1 {
            return false;
        }
    }
    true
}

/// Resolve a LOCAL embedded selector through the alias index: `type A = B`
/// embedded as `S{A}` resolves its owner identity to B while keeping the
/// written selector name (SOL-W6).
fn resolve_local_alias_embed(
    inputs: &SnapshotInputs<'_>,
    owner: &GoOwnerIdentity,
    selector: &str,
) -> Option<GoOwnerIdentity> {
    let variants = inputs.alias_index.variants.get(&(
        owner.package_dir.clone(),
        owner.package_clause.clone(),
        selector.to_string(),
    ))?;
    // Every variant must be an Alias whose RHS canonicalizes identically;
    // anything else fails closed (None → unresolved_embed axis).
    let first = variants.first()?;
    if !variants.iter().all(|variant| variant.kind == first.kind) {
        return None;
    }
    let crate::go_alias_index::GoAliasKind::Alias {
        rhs: Some(text),
        type_params: 0,
    } = &first.kind
    else {
        return None;
    };
    // Single local leaf (`B` or `~path::B`) in the owning package.
    let compact = text.trim();
    let name = if let Some(rest) = compact.strip_prefix('~') {
        rest.rsplit("::").next()
    } else if !compact.is_empty() && compact.chars().all(|ch| ch == '_' || ch.is_alphanumeric()) {
        Some(compact)
    } else {
        None
    }?;
    if compact.starts_with('@') {
        return None; // qualified alias targets go through resolve_qualified_embed
    }
    Some(GoOwnerIdentity {
        package_dir: owner.package_dir.clone(),
        package_clause: owner.package_clause.clone(),
        name: name.to_string(),
    })
}

/// Unique ordinary package clause per directory (`_test` clauses excluded);
/// directories with several ordinary clauses are absent (ambiguous).
fn unique_clause_per_dir(
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> BTreeMap<String, String> {
    let mut clauses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, profile) in profiles {
        if profile.is_test_file || profile.package_clause.trim().is_empty() {
            continue;
        }
        clauses
            .entry(crate::resolution::dir_of(path).to_string())
            .or_default()
            .insert(profile.package_clause.clone());
    }
    clauses
        .into_iter()
        .filter(|(_, set)| set.len() == 1)
        .map(|(dir, set)| (dir, set.into_iter().next().unwrap()))
        .collect()
}

/// Resolve a QUALIFIED embedded field (`pkg.Name`) to its owner identity via
/// the declaring file's import map and the module graph's path->dir proof.
/// Ambiguity anywhere fails closed (None).
fn resolve_qualified_embed(
    inputs: &SnapshotInputs<'_>,
    declaring_file: &str,
    raw_type: &str,
) -> Option<GoOwnerIdentity> {
    let compact = raw_type.trim_start_matches('*').trim();
    let (qualifier, name) = compact.rsplit_once('.')?;
    if qualifier.is_empty()
        || name.is_empty()
        || !name.chars().all(|ch| ch == '_' || ch.is_alphanumeric())
    {
        return None;
    }
    let imports = inputs.imports_by_file.get(declaring_file)?;
    let import_path = imports.get(qualifier)?;
    let dirs = inputs.dirs_by_path.get(import_path)?;
    if dirs.len() != 1 {
        return None;
    }
    let dir = dirs.iter().next()?;
    let clauses = unique_clause_per_dir(inputs.profiles);
    let clause = clauses.get(dir)?;
    Some(GoOwnerIdentity {
        package_dir: dir.clone(),
        package_clause: clause.clone(),
        name: name.to_string(),
    })
}

/// Compute the full snapshot for every outer struct owner.
pub fn compute(inputs: &SnapshotInputs<'_>) -> GoPromotedSelectorSnapshot {
    let mut owners: BTreeMap<GoOwnerIdentity, GoOwnerSnapshotEntry> = BTreeMap::new();

    for (owner, declarations) in inputs.struct_declarations {
        let mut facts_by_file: BTreeMap<String, GoDeclarationFacts> = BTreeMap::new();
        let mut axes: BTreeSet<String> = BTreeSet::new();
        for declaration in declarations {
            let mut facts = GoDeclarationFacts::default();
            for (selector, raw_type) in &declaration.embedded_fields {
                let compact = raw_type.replace(char::is_whitespace, "");
                let is_pointer = compact.starts_with('*');
                if compact.trim_start_matches('*').starts_with("struct{")
                    || compact.trim_start_matches('*') == "struct"
                {
                    axes.insert("anonymous_embed".to_string());
                    facts.embeds.insert(GoEmbedKey {
                        is_pointer,
                        resolved_owner: None,
                        selector: selector.clone(),
                        is_interface: false,
                    });
                    continue;
                }
                let mut resolved = inputs
                    .field_targets
                    .get(&(owner.clone(), selector.clone()))
                    .map(|target| target.owner.clone());
                let interface_candidate = GoOwnerIdentity {
                    package_dir: owner.package_dir.clone(),
                    package_clause: owner.package_clause.clone(),
                    name: selector.clone(),
                };
                let mut embedded_interface_owner: Option<GoOwnerIdentity> = None;
                if inputs.interface_owners.contains(&interface_candidate) {
                    // Local embedded interface (selector IS the iface name).
                    embedded_interface_owner = Some(interface_candidate);
                }
                let mut is_interface = embedded_interface_owner.is_some();
                if resolved.is_none() && !is_interface {
                    // SOL-W6: `type A = B` embedded as S{A} resolves to owner B
                    // through the alias index while keeping selector "A".
                    resolved = resolve_local_alias_embed(inputs, owner, selector);
                }
                if resolved.is_none() && !is_interface {
                    match resolve_qualified_embed(inputs, &declaration.defining_file, raw_type) {
                        Some(qualified) if inputs.interface_owners.contains(&qualified) => {
                            // SOL-W7 converse: a QUALIFIED embed that resolves
                            // to an interface identity is reclassified as an
                            // interface, never walked as a struct.
                            embedded_interface_owner = Some(qualified.clone());
                            is_interface = true;
                            resolved = None;
                        }
                        qualified => resolved = qualified,
                    }
                }
                if resolved.is_none() && !is_interface {
                    axes.insert("unresolved_embed".to_string());
                }
                // fix-3 / SOL-W7: an embedded INTERFACE whose method surface
                // varies across its declaring profiles poisons this owner.
                if let Some(iface) = &embedded_interface_owner {
                    if !interface_profile_consistent(inputs, iface) {
                        axes.insert("embedded_interface_profile".to_string());
                    }
                }
                facts.embeds.insert(GoEmbedKey {
                    is_pointer,
                    resolved_owner: resolved,
                    selector: selector.clone(),
                    is_interface,
                });
                facts.field_names.insert(selector.clone());
            }
            for field in declaration.fields.keys() {
                facts.field_names.insert(field.clone());
            }
            // Own-method names contributed by THIS profile: every method
            // declaration of the owner whose file is EXACTLY visible from the
            // declaring file (build/test-profile compatible).
            if let Some(methods) = inputs.method_declarations.get(owner) {
                for method in methods {
                    let (visible, exact) = crate::go_owner_partition::exact_declaration_visibility(
                        owner,
                        &declaration.defining_file,
                        crate::go_owner_partition::GoOwnerReferenceMode::Bare,
                        &method.defining_file,
                        inputs.profiles,
                    );
                    if !(visible && exact) {
                        continue;
                    }
                    facts.own_methods.insert(
                        method.method_name.clone(),
                        GoOwnMethodInfo {
                            function_id: method.function_id.clone(),
                            is_pointer_receiver: method.is_pointer_receiver,
                        },
                    );
                }
            }
            facts_by_file.insert(declaration.defining_file.clone(), facts);
        }

        // Axis: own-method names contributed per build profile of the
        // contributing ordinary-clause files. If different profiles contribute
        // DIFFERENT name sets, the promotion surface is profile-dependent.
        let mut names_by_profile: BTreeMap<
            crate::go_build_profile::GoBuildProfile,
            BTreeSet<String>,
        > = BTreeMap::new();
        if let Some(methods) = inputs.method_declarations.get(owner) {
            for method in methods {
                if let Some(profile) = inputs.profiles.get(&method.defining_file) {
                    if !profile.is_test_file {
                        names_by_profile
                            .entry(profile.clone())
                            .or_default()
                            .insert(format!(
                                "{}:{}",
                                method.method_name, method.is_pointer_receiver
                            ));
                    }
                }
            }
        }
        let mut profile_views: Vec<&BTreeSet<String>> = names_by_profile.values().collect();
        profile_views.sort();
        profile_views.dedup();
        if profile_views.len() > 1 {
            axes.insert("own_methods".to_string());
        }

        // Axis comparison across THIS owner's declarations.
        if facts_by_file.len() > 1 {
            let mut baseline: Option<(&String, &GoDeclarationFacts)> = None;
            for (file, facts) in &facts_by_file {
                match baseline {
                    None => baseline = Some((file, facts)),
                    Some((_, first)) => {
                        if facts.embeds != first.embeds {
                            axes.insert("embed_identity".to_string());
                        }
                        if facts.field_names != first.field_names {
                            axes.insert("ordinary_fields".to_string());
                        }
                        // SOL-W8: compare method facts as (name,
                        // receiver-kind) pairs — name sets alone collapse
                        // profile-specific receiver shapes, while raw
                        // FunctionIds legitimately differ per declaring file.
                        let shape = |facts: &GoDeclarationFacts| {
                            facts
                                .own_methods
                                .iter()
                                .map(|(name, info)| (name.clone(), info.is_pointer_receiver))
                                .collect::<BTreeMap<_, _>>()
                        };
                        if shape(facts) != shape(first) {
                            axes.insert("own_methods".to_string());
                        }
                    }
                }
            }
        }

        owners.insert(
            owner.clone(),
            GoOwnerSnapshotEntry {
                declarations: facts_by_file,
                verdict: GoPromotionVerdict::Consistent,
                conflict_axes: axes,
                promoted: BTreeMap::new(),
            },
        );
    }

    // Verdict propagation: an owner is conflicted if its own axes conflict OR
    // any owner along its embedding paths is conflicted (every hop must be
    // profile-unique). Memoized over owners; cycle-guarded by construction of
    // the walk below (path set).
    let verdicts = resolve_verdicts(&owners);
    for (owner, verdict) in &verdicts {
        if let Some(entry) = owners.get_mut(owner) {
            entry.verdict = *verdict;
        }
    }

    // Promoted methods only where the whole reachable path is Consistent.
    for (owner, entry) in owners.iter_mut() {
        if entry.verdict != GoPromotionVerdict::Consistent {
            continue;
        }
        let mut walker = PromotionWalker::default();
        let mut field_depth = BTreeMap::new();
        let mut path = BTreeSet::new();
        path.insert(owner.clone());
        walker.walk(inputs, owner, owner, 0, &mut path, &mut field_depth);
        walker.field_depth = field_depth;
        let mut promoted = BTreeMap::new();
        for candidate in walker.candidates {
            let shadowed = walker
                .field_depth
                .get(&candidate.method_name)
                .is_some_and(|depth| *depth <= candidate.depth);
            promoted.insert(
                candidate.method_name,
                GoPromotedMethodInfo {
                    target_owner: candidate.target_owner,
                    function_id: candidate.function_id,
                    depth: candidate.depth,
                    shadowed_by_field: shadowed,
                    value_method_set: !candidate.is_pointer_receiver
                        || candidate.pointer_embed_path,
                },
            );
        }
        entry.promoted = promoted;
    }

    GoPromotedSelectorSnapshot { owners }
}

fn resolve_verdicts(
    owners: &BTreeMap<GoOwnerIdentity, GoOwnerSnapshotEntry>,
) -> BTreeMap<GoOwnerIdentity, GoPromotionVerdict> {
    let mut memo: BTreeMap<GoOwnerIdentity, GoPromotionVerdict> = BTreeMap::new();
    for owner in owners.keys() {
        resolve_one(owner, owners, &mut memo, &mut BTreeSet::new());
    }
    memo
}

fn resolve_one(
    owner: &GoOwnerIdentity,
    owners: &BTreeMap<GoOwnerIdentity, GoOwnerSnapshotEntry>,
    memo: &mut BTreeMap<GoOwnerIdentity, GoPromotionVerdict>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> GoPromotionVerdict {
    if let Some(verdict) = memo.get(owner) {
        return *verdict;
    }
    if !visiting.insert(owner.clone()) {
        return GoPromotionVerdict::ProfileConflict; // embed cycle: fail closed
    }
    let entry = match owners.get(owner) {
        Some(entry) => entry,
        None => return GoPromotionVerdict::ProfileConflict,
    };
    let mut verdict = if entry.conflict_axes.is_empty() {
        GoPromotionVerdict::Consistent
    } else {
        GoPromotionVerdict::ProfileConflict
    };
    'hops: for facts in entry.declarations.values() {
        for embed in facts.embeds.iter().filter(|embed| !embed.is_interface) {
            let Some(target) = &embed.resolved_owner else {
                verdict = GoPromotionVerdict::ProfileConflict;
                break 'hops;
            };
            if resolve_one(target, owners, memo, visiting) == GoPromotionVerdict::ProfileConflict {
                verdict = GoPromotionVerdict::ProfileConflict;
                break 'hops;
            }
        }
    }
    visiting.remove(owner);
    memo.insert(owner.clone(), verdict);
    verdict
}

struct PromotionCandidate {
    method_name: String,
    target_owner: GoOwnerIdentity,
    function_id: crate::call_graph::FunctionId,
    depth: usize,
    is_pointer_receiver: bool,
    pointer_embed_path: bool,
}

#[derive(Default)]
struct PromotionWalker {
    candidates: Vec<PromotionCandidate>,
    field_depth: BTreeMap<String, usize>,
}

impl PromotionWalker {
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        inputs: &SnapshotInputs<'_>,
        _outer: &GoOwnerIdentity,
        current: &GoOwnerIdentity,
        depth: usize,
        path: &mut BTreeSet<GoOwnerIdentity>,
        field_depth: &mut BTreeMap<String, usize>,
    ) {
        let Some(declarations) = inputs.struct_declarations.get(current) else {
            return;
        };
        let Some(declaration) = declarations.iter().next() else {
            return;
        };
        for field in declaration.fields.keys() {
            record_depth(field_depth, field, depth);
        }
        for selector in declaration.embedded_fields.keys() {
            record_depth(field_depth, selector, depth);
        }
        if depth >= 1 {
            if let Some(methods) = inputs.method_declarations.get(current) {
                for method in methods {
                    self.candidates.push(PromotionCandidate {
                        method_name: method.method_name.clone(),
                        target_owner: current.clone(),
                        function_id: method.function_id.clone(),
                        depth,
                        is_pointer_receiver: method.is_pointer_receiver,
                        pointer_embed_path: false,
                    });
                }
            }
        }
        for (selector, raw_type) in &declaration.embedded_fields {
            let is_pointer = raw_type.trim_start().starts_with('*');
            let mut target = inputs
                .field_targets
                .get(&(current.clone(), selector.clone()))
                .map(|target| target.owner.clone())
                .or_else(|| resolve_local_alias_embed(inputs, current, selector));
            // SOL-W7 converse: a qualified embed resolving to an INTERFACE has
            // no struct to walk into.
            if let Some(resolved) = &target {
                if inputs.interface_owners.contains(resolved) {
                    target = None;
                }
            }
            let Some(target) = target else {
                continue;
            };
            if !path.insert(target.clone()) {
                continue;
            }
            let before = self.candidates.len();
            self.walk(inputs, _outer, &target, depth + 1, path, field_depth);
            path.remove(&target);
            if is_pointer {
                for candidate in &mut self.candidates[before..] {
                    candidate.pointer_embed_path = true;
                }
            }
        }
    }
}

fn record_depth(field_depth: &mut BTreeMap<String, usize>, name: &str, depth: usize) {
    field_depth
        .entry(name.to_string())
        .and_modify(|existing| {
            if depth < *existing {
                *existing = depth;
            }
        })
        .or_insert(depth);
}
