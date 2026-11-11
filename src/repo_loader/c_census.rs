//! The files the C/C++ macro census reads besides the parsed C/C++ sources (#630 slice 6a).
//!
//! The preprocessor reads every file a `#include` names, parsed or not. The census therefore
//! also reads, as text only (never parsed, indexed or hashed into the parsed-file map),
//! in-repo files with the extensions in [`TEXT_EXTS`] and every in-repo target of a quoted
//! include whose extension is not mapped to C/C++, and every in-repo file an angle include
//! (`<p>`, resolved against every in-repo directory: a path equal to `p` or ending in `/p`)
//! can name, transitively. The loader decides the set,
//! deterministically, so full, partial and scoped builds agree; each file's content hash and
//! the aggregate of the skipped C/C++ files join the cache topology key
//! (`c-census:` entries of `LoadedRepo::manifest_hashes`), so a change to either misses.
//!
//! Review mode parses only the diff files: [`review_gap`] reports whether the repository
//! holds census inputs the review did not read, in which case the census fails closed.
use super::{rel, skip_dir, SkipReason, SkippedFile};
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::{angle_includes, quoted_includes};
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

/// Extensions of includable files the loader does not map to C/C++ (case-sensitive).
pub(crate) const TEXT_EXTS: &[&str] = &[
    "inl", "inc", "ipp", "tcc", "def", "cu", "cuh", "H", "C", "HPP", "CPP",
];

fn is_c_family_path(path: &str) -> bool {
    matches!(Language::from_path(path), Some(Language::C | Language::Cpp))
}

fn is_text_path(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| TEXT_EXTS.contains(&ext))
}

fn is_census_path(path: &str) -> bool {
    is_c_family_path(path) || is_text_path(path)
}

fn is_c_family_parsed(p: &ParsedFile) -> bool {
    matches!(p.language, Language::C | Language::Cpp)
}

/// Does this skip lose census input? Ignored, hidden and Go-testdata skips are explicit
/// exclusions; an unreadable entry (a file, or a directory whose contents are unknown) is a
/// loss whatever it is called.
fn is_loss(s: &SkippedFile) -> bool {
    match s.reason {
        SkipReason::Unreadable => true,
        SkipReason::NotUtf8
        | SkipReason::ParseFailed
        | SkipReason::TooLarge { .. }
        | SkipReason::Symlink => is_census_path(&s.path),
        SkipReason::Unsupported
        | SkipReason::Ignored
        | SkipReason::GoTestdata
        | SkipReason::Hidden => false,
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// `dir/target` with `.` and `..` folded lexically; `None` when it leaves the root.
fn join_normal(dir: &str, target: &str) -> Option<String> {
    if target.starts_with('/') || target.contains('\\') {
        return None;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The existing in-repo files a quoted include of `includer` can name: the target resolved
/// against the includer's directory and every ancestor up to the root.
fn existing_targets(
    root: &Path,
    includer: &str,
    target: &str,
    checked: &mut BTreeMap<String, bool>,
) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    let mut d = includer.rsplit_once('/').map_or("", |(d, _)| d);
    loop {
        dirs.push(d.to_owned());
        match d.rsplit_once('/') {
            Some((p, _)) => d = p,
            None if !d.is_empty() => d = "",
            None => break,
        }
    }
    let mut out = Vec::new();
    for dir in dirs {
        let Some(cand) = join_normal(&dir, target) else {
            continue;
        };
        let exists = *checked
            .entry(cand.clone())
            .or_insert_with(|| root.join(&cand).is_file());
        if exists {
            out.push(cand);
        }
    }
    out
}

/// In-repo file paths by final component, for resolving an angle include against every
/// in-repo directory.
struct PathIndex(BTreeMap<String, Vec<String>>);

impl PathIndex {
    fn new<'a>(paths: impl Iterator<Item = &'a str>) -> Self {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for p in paths.filter(|p| !p.ends_with('/')) {
            let base = p.rsplit('/').next().unwrap_or(p);
            m.entry(base.to_owned()).or_default().push(p.to_owned());
        }
        PathIndex(m)
    }

    /// Repository files equal to `target` or ending in `/target`.
    fn resolve(&self, target: &str) -> Vec<String> {
        if target.is_empty()
            || target.starts_with('/')
            || target.contains('\\')
            || target.split('/').any(|s| s == ".." || s.is_empty())
        {
            return Vec::new();
        }
        let target = target.strip_prefix("./").unwrap_or(target);
        let base = target.rsplit('/').next().unwrap_or(target);
        let suffix = format!("/{target}");
        self.0.get(base).map_or_else(Vec::new, |v| {
            v.iter()
                .filter(|p| p.as_str() == target || p.ends_with(&suffix))
                .cloned()
                .collect()
        })
    }
}

/// `PRISM_C_PRUNE_TRACE=1`: name why the loader makes the census incomplete (first 20).
fn trace_reason(kind: &str, items: &[String]) {
    if std::env::var_os("PRISM_C_PRUNE_TRACE").is_none() {
        return;
    }
    eprintln!("c-census incomplete: {kind}={}", items.len());
    for i in items.iter().take(20) {
        eprintln!("c-census reason\t{kind}\t{i}");
    }
}

/// The census reads an oversize census input (a generated table over the loader's
/// [`MAX_FILE_BYTES`]) as text up to this size; beyond it the census stays incomplete.
pub(crate) const CENSUS_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// A C/C++ or text-include file the loader skipped for size only: the census reads it as text.
fn is_oversize_census_input(s: &SkippedFile) -> bool {
    matches!(s.reason, SkipReason::TooLarge { .. }) && is_census_path(&s.path)
}

/// What the loader found for the census.
#[derive(Default)]
pub(crate) struct CensusInputs {
    /// Text-only include files, `(path, text)`.
    pub(crate) texts: BTreeMap<String, String>,
    /// `c-census:` topology entries: the skipped-file aggregate and each text file's hash.
    pub(crate) identity: BTreeMap<String, String>,
    /// A census input was lost (skipped C/C++ file, unreadable entry, unreadable text file).
    pub(crate) incomplete: bool,
}

impl CensusInputs {
    /// Stamp every parsed C/C++ file with the loss flag and the text files.
    pub(crate) fn apply(&self, files: &mut BTreeMap<String, ParsedFile>) {
        let texts = Arc::new(self.texts.clone());
        for parsed in files.values_mut().filter(|p| is_c_family_parsed(p)) {
            if self.incomplete {
                parsed.mark_c_family_skipped_in_repo();
            }
            if !self.texts.is_empty() {
                parsed.set_c_census_texts(Arc::clone(&texts));
            }
        }
    }
}

pub(crate) fn gather(
    root: &Path,
    files: &BTreeMap<String, ParsedFile>,
    skipped: &[SkippedFile],
) -> CensusInputs {
    let mut g = CensusInputs::default();
    // Without a parsed C/C++ file there is no census to feed, so an oversize file is a loss
    // exactly as before; with one, it is read as text below (and a failed read is a loss).
    let any_c = files.values().any(is_c_family_parsed);
    let mut lost: Vec<String> = skipped
        .iter()
        .filter(|s| is_loss(s) && !(any_c && is_oversize_census_input(s)))
        .map(|s| format!("{}\0{:?}", s.path, s.reason))
        .collect();
    if !lost.is_empty() {
        lost.sort();
        trace_reason(
            "loader_skip",
            &lost
                .iter()
                .map(|l| l.replace('\0', " "))
                .collect::<Vec<_>>(),
        );
        g.incomplete = true;
        g.identity
            .insert("c-census:skipped".into(), sha(lost.join("\n").as_bytes()));
    }
    if !any_c {
        return g;
    }
    let mut queue: Vec<String> = skipped
        .iter()
        .filter(|s| {
            (s.reason == SkipReason::Unsupported && is_text_path(&s.path))
                || is_oversize_census_input(s)
        })
        .map(|s| s.path.clone())
        .collect();
    type Sources = Vec<(String, BTreeSet<String>, BTreeSet<String>)>;
    let mut sources: Sources = files
        .iter()
        .filter(|(_, p)| is_c_family_parsed(p))
        .map(|(path, p)| {
            (
                path.clone(),
                quoted_includes(&p.source),
                angle_includes(&p.source),
            )
        })
        .collect();
    let index = PathIndex::new(
        files
            .keys()
            .map(String::as_str)
            .chain(skipped.iter().map(|s| s.path.as_str())),
    );
    let mut checked = BTreeMap::new();
    let mut known: BTreeSet<String> = queue.iter().cloned().collect();
    loop {
        for (includer, quoted, angle) in std::mem::take(&mut sources) {
            let cands = quoted
                .iter()
                .flat_map(|t| existing_targets(root, &includer, t, &mut checked))
                .chain(angle.iter().flat_map(|t| index.resolve(t)));
            for cand in cands.collect::<Vec<_>>() {
                if !files.contains_key(&cand)
                    && !is_c_family_path(&cand)
                    && known.insert(cand.clone())
                {
                    queue.push(cand);
                }
            }
        }
        let Some(path) = queue.pop() else { break };
        let full = root.join(&path);
        let too_big = std::fs::metadata(&full).map_or(false, |m| m.len() > CENSUS_MAX_BYTES);
        let read = if too_big {
            Ok(Vec::new())
        } else {
            std::fs::read(&full)
        };
        match read {
            Ok(bytes) if !too_big => match String::from_utf8(bytes) {
                Ok(text) => {
                    g.identity
                        .insert(format!("c-census:text:{path}"), sha(text.as_bytes()));
                    sources.push((path.clone(), quoted_includes(&text), angle_includes(&text)));
                    g.texts.insert(path, text);
                }
                // The loader refuses a non-UTF-8 C/C++ file; the census holds census text to
                // the same rule (a lossy read could hide a define behind an encoding the
                // compiler accepts), so it cannot be read and is a loss.
                Err(_) => {
                    trace_reason("text_include_failed", &[format!("{path} not-utf8")]);
                    g.incomplete = true;
                    g.identity.insert(
                        format!("c-census:text-failed:{path}"),
                        "not-utf8".to_string(),
                    );
                }
            },
            // A discovered text input that cannot be read still changes the census (it is
            // `incomplete`), so it joins the cache identity too: a full hit must not replay a
            // census built before this file appeared or became unreadable.
            Ok(_) => {
                trace_reason("text_include_failed", &[format!("{path} too-large")]);
                g.incomplete = true;
                g.identity.insert(
                    format!("c-census:text-failed:{path}"),
                    "too-large".to_string(),
                );
            }
            Err(e) => {
                trace_reason("text_include_failed", &[format!("{path} {:?}", e.kind())]);
                g.incomplete = true;
                g.identity.insert(
                    format!("c-census:text-failed:{path}"),
                    format!("unreadable:{:?}", e.kind()),
                );
            }
        }
    }
    g
}

/// Review mode (the review parses only the diff's files): does the repository hold C/C++
/// census inputs `files` lacks? `listing` is the repository's supported-path listing. `Some`
/// carries a digest of what is missing, for the review topology key.
pub(crate) fn review_gap(
    root: &Path,
    files: &BTreeMap<String, ParsedFile>,
    listing: Option<&BTreeSet<String>>,
) -> Option<String> {
    if !files.values().any(is_c_family_parsed) {
        return None;
    }
    let mut gap: BTreeSet<String> = BTreeSet::new();
    match listing {
        None => {
            gap.insert("<listing unavailable>".into());
        }
        Some(listing) => gap.extend(
            listing
                .iter()
                .filter(|p| is_c_family_path(p) && !files.contains_key(*p))
                .cloned(),
        ),
    }
    let mut checked = BTreeMap::new();
    for (includer, p) in files.iter().filter(|(_, p)| is_c_family_parsed(p)) {
        for t in quoted_includes(&p.source) {
            for cand in existing_targets(root, includer, &t, &mut checked) {
                if !files.contains_key(&cand) {
                    gap.insert(cand);
                }
            }
        }
    }
    let mut all: BTreeSet<String> = BTreeSet::new();
    if walk_text_paths(root, root, &mut gap, &mut all).is_err() {
        gap.insert("<walk failed>".into());
    }
    // Angle includes resolve against every in-repo directory: any repository file they can
    // name that the review did not parse is an unread census input.
    let index = PathIndex::new(all.iter().map(String::as_str));
    for (_, p) in files.iter().filter(|(_, p)| is_c_family_parsed(p)) {
        for t in angle_includes(&p.source) {
            gap.extend(
                index
                    .resolve(&t)
                    .into_iter()
                    .filter(|c| !files.contains_key(c)),
            );
        }
    }
    if !gap.is_empty() {
        trace_reason(
            "review_gap",
            &gap.iter().take(20).cloned().collect::<Vec<_>>(),
        );
    }
    (!gap.is_empty()).then(|| sha(gap.into_iter().collect::<Vec<_>>().join("\n").as_bytes()))
}

fn walk_text_paths(
    root: &Path,
    dir: &Path,
    out: &mut BTreeSet<String>,
    all: &mut BTreeSet<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !skip_dir(&name, dir == root) && !name.starts_with('.') {
                walk_text_paths(root, &entry.path(), out, all)?;
            }
        } else if file_type.is_file() {
            let relp = rel(root, &entry.path());
            all.insert(relp.clone());
            if is_text_path(&relp) {
                out.insert(relp);
            }
        }
    }
    Ok(())
}
