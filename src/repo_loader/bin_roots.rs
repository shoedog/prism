//! Binary target roots of a Cargo package, for the two lanes that consume them (#749): the own
//! library (`binary_library_deps`) and the package's in-repo dependencies
//! (`member_bin_roots`). One enumeration, so the lanes cannot drift.
use super::{join_manifest_rel, normalize_repo_rel};
use crate::ast::ParsedFile;
use crate::name_resolution::rust_populator::MemberBins;
use std::collections::{BTreeMap, BTreeSet};

/// A package's own library target as an `extern` name, with the checks that make it
/// nameable from its binaries (no `[lib] edition`, not `autolib = false`, not a proc-macro or
/// non-rlib crate type, an existing root file). The dependency-name conflict is the
/// callers' rule.
struct OwnLibrary {
    name: String,
    root: String,
}

fn own_library(
    manifest: &toml::Value,
    dir: &str,
    files: &BTreeMap<String, ParsedFile>,
) -> Option<OwnLibrary> {
    let package = manifest.get("package")?;
    let lib = manifest.get("lib");
    if lib.is_some_and(|l| l.get("edition").is_some()) {
        return None;
    }
    if (lib.is_none() && package.get("autolib").and_then(|v| v.as_bool()) == Some(false))
        || lib
            .and_then(|l| l.get("proc-macro"))
            .and_then(|v| v.as_bool())
            == Some(true)
        || lib
            .and_then(|l| l.get("crate-type"))
            .and_then(|v| v.as_array())
            .is_some_and(|types| {
                !types
                    .iter()
                    .any(|t| matches!(t.as_str(), Some("lib" | "rlib" | "dylib")))
            })
    {
        return None;
    }
    let name = lib
        .and_then(|l| l.get("name"))
        .or_else(|| package.get("name"))
        .and_then(|v| v.as_str())?
        .replace('-', "_");
    let lib_rel = lib
        .and_then(|l| l.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("src/lib.rs");
    let root = normalize_repo_rel(&join_manifest_rel(dir, lib_rel))?;
    files
        .contains_key(&root)
        .then_some(OwnLibrary { name, root })
}

/// The binary target roots Cargo gives a package, as far as prism models them: without
/// `autobins = false` and without an explicit `[[bin]]` list, `src/main.rs`, `src/bin/*.rs`
/// and `src/bin/*/main.rs`; explicit `[[bin]]` paths replace auto-discovery (inferring
/// Cargo's name-based overrides belongs to a broader target model); a target with its own
/// edition is outside the uniform-edition graph model. The ONE enumeration for the
/// own-library lane and the dependency lane (#749), so the two cannot drift.
fn package_bin_roots(
    manifest: &toml::Value,
    dir: &str,
    files: &BTreeMap<String, ParsedFile>,
) -> BTreeSet<String> {
    let mut binaries = BTreeSet::new();
    let Some(package) = manifest.get("package") else {
        return binaries;
    };
    let explicit_bins = manifest.get("bin").and_then(|v| v.as_array());
    if explicit_bins.is_none() && package.get("autobins").and_then(|v| v.as_bool()) != Some(false) {
        let main = join_manifest_rel(dir, "src/main.rs");
        if files.contains_key(&main) {
            binaries.insert(main);
        }
        let prefix = join_manifest_rel(dir, "src/bin/");
        for file in files.keys() {
            if let Some(rest) = file.strip_prefix(&prefix) {
                if (!rest.contains('/') && rest.ends_with(".rs"))
                    || rest
                        .strip_suffix("/main.rs")
                        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
                {
                    binaries.insert(file.clone());
                }
            }
        }
    }
    if let Some(bins) = explicit_bins {
        for bin in bins {
            // Target-specific edition is outside the uniform-edition graph model.
            if bin.get("edition").is_some() {
                continue;
            }
            if let Some(path) = bin.get("path").and_then(|v| v.as_str()) {
                if let Some(path) = normalize_repo_rel(&join_manifest_rel(dir, path)) {
                    if files.contains_key(&path) {
                        binaries.insert(path);
                    }
                }
            }
        }
    }
    binaries
}

/// Cargo binaries can reference their own package library, without a dependency
/// table entry. Keep that authority path-addressed and separate from directory-
/// guessed crate names and the legacy single `lib_path` field.
pub(super) fn collect_binary_library_deps(
    manifest: &toml::Value,
    dir: &str,
    files: &BTreeMap<String, ParsedFile>,
    out: &mut BTreeMap<String, (String, String)>,
) {
    let Some(library) = own_library(manifest, dir, files) else {
        return;
    };
    if manifest
        .get("dependencies")
        .and_then(|v| v.as_table())
        .is_some_and(|deps| deps.keys().any(|k| k.replace('-', "_") == library.name))
    {
        return; // Do not choose a winner for conflicting extern names.
    }
    for binary in package_bin_roots(manifest, dir, files) {
        if binary != library.root {
            out.insert(binary, (library.name.clone(), library.root.clone()));
        }
    }
}

/// A member's binary roots and its own library's extern name (when it has a nameable one),
/// for the dependency lane (#749): the Builder keys each root with the member's in-repo
/// `[dependencies]` except one named like the own library (no winner for a name).
pub(super) fn collect_member_bin_roots(
    manifest: &toml::Value,
    dir: &str,
    files: &BTreeMap<String, ParsedFile>,
    out: &mut BTreeMap<String, MemberBins>,
) {
    let library = own_library(manifest, dir, files);
    let roots: Vec<String> = package_bin_roots(manifest, dir, files)
        .into_iter()
        .filter(|b| library.as_ref().is_none_or(|l| *b != l.root))
        .collect();
    if !roots.is_empty() {
        out.insert(
            dir.to_string(),
            MemberBins {
                own_lib_name: library.map(|l| l.name),
                roots,
            },
        );
    }
}
