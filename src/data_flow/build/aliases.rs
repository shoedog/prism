//! Shared per-file alias helpers.
use super::Producer;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};

impl Producer {
    /// Phase 3: Build a local alias map from simple assignments like `ptr = dev`.
    /// Returns (alias_map, raw_aliases) where alias_map maps alias name to
    /// fully resolved target, and raw_aliases are the original (alias, target, line) triples.
    pub(super) fn build_alias_map(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> (BTreeMap<String, String>, Vec<(String, String, usize)>) {
        let raw_aliases = parsed.collect_alias_assignments(func_node, lines);
        (Self::resolve_alias_chain(&raw_aliases), raw_aliases)
    }

    pub(super) fn resolve_alias_chain(
        raw_aliases: &[(String, String, usize)],
    ) -> BTreeMap<String, String> {
        let mut alias_map: BTreeMap<String, String> = BTreeMap::new();

        // Process in line order so earlier aliases are available for chain resolution
        for (alias, target, _line) in raw_aliases.iter() {
            // Follow chain: if target itself is an alias, resolve transitively
            let mut resolved = target.clone();
            let mut depth = 0;
            while let Some(next) = alias_map.get(&resolved) {
                resolved = next.clone();
                depth += 1;
                if depth > 10 {
                    break; // Prevent infinite loops in pathological cases
                }
            }

            // For dotted targets (from destructuring), also resolve the base component.
            // e.g., name → ptr.name where ptr → device  →  resolved becomes device.name
            let mut base_depth = 0;
            loop {
                let target_ap = AccessPath::from_expr(&resolved);
                if target_ap.fields.is_empty() {
                    break; // Simple target, already handled by chain resolution above
                }
                if let Some(base_target) = alias_map.get(&target_ap.base) {
                    let base_ap = AccessPath::from_expr(base_target);
                    let mut fields = base_ap.fields;
                    fields.extend(target_ap.fields);
                    resolved = std::iter::once(base_ap.base.as_str())
                        .chain(fields.iter().map(|f| f.as_str()))
                        .collect::<Vec<_>>()
                        .join(".");
                    base_depth += 1;
                    if base_depth > 5 {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Don't create self-aliases
            if alias != &resolved {
                alias_map.insert(alias.clone(), resolved);
            }
        }

        alias_map
    }

    /// Phase 3: If a path's base is aliased, return the resolved path.
    ///
    /// Simple alias: ptr → dev, path `ptr.field` resolves to `dev.field`.
    /// Destructuring alias: name → device.name, path `name` resolves to `device.name`.
    /// Destructuring + field: name → device.name, path `name.x` resolves to `device.name.x`.
    pub(super) fn resolve_path(
        alias_map: &BTreeMap<String, String>,
        path: &AccessPath,
    ) -> Option<AccessPath> {
        if let Some(target) = alias_map.get(&path.base) {
            let target_path = AccessPath::from_expr(target);
            // Combine: target_path's fields + path's original fields
            let mut fields = target_path.fields;
            fields.extend(path.fields.iter().cloned());
            Some(AccessPath {
                base: target_path.base,
                fields,
            })
        } else {
            None
        }
    }

    /// PR-B D11: a synthetic pass resolves an alias twin only on the line that
    /// establishes the alias, to that line's own target. Re-assigning the alias
    /// name elsewhere breaks the alias; it never writes the target, and a later
    /// alias of the same name never retro-applies (flow-insensitive twins are
    /// legacy parity, E11).
    pub(super) fn resolve_on_alias_span(
        alias_targets_by_byte: &BTreeMap<usize, String>,
        path: &AccessPath,
        start_byte: usize,
    ) -> Option<AccessPath> {
        let target = alias_targets_by_byte.get(&start_byte)?;
        let target_path = AccessPath::from_expr(target);
        let mut fields = target_path.fields;
        fields.extend(path.fields.iter().cloned());
        Some(AccessPath {
            base: target_path.base,
            fields,
        })
    }
}
