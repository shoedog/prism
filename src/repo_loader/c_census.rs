//! The files the C/C++ macro census reads besides the parsed C/C++ sources (#630 slice 6a,
//! reshaped by slice 6b: the hybrid input rule).
//!
//! The preprocessor reads every file a `#include` names. Two inputs feed the census:
//!
//! * **Include discovery.** The targets spelled by every parsed C/C++ file, and
//!   transitively by every census text, are normalized (`.` and `..` resolved lexically,
//!   `\` read as `/`, repeated separators collapsed) and matched against EVERY in-repo
//!   directory: a repository file whose path equals the target or ends with `/<target>`
//!   (quoted targets too, so `-I` directories are covered; over-matching only adds facts).
//!   A leading `..` that cannot be resolved is dropped for quoted and angle targets alike
//!   (the include directory can be any in-repo directory, so the match is by suffix). A
//!   resolved target that is not a parsed C/C++ file is read as census text whatever its
//!   extension or language.
//! * **The C-like-extension net.** Every in-repo file with a C-like extension (see
//!   [`C_LIKE`]) that is not a parsed C/C++ file is read too, whether or not anything
//!   visibly includes it (computed includes, unresolved spellings).
//!
//! Census texts are read with the PERMISSIVE scan (`macro_census::collect_permissive`): no
//! string tracking (comments are read kept and erased), and never `incomplete` from lexer
//! state. Only an unreadable file or directory, a symlink that points outside the repository
//! (or a symlinked directory the walk does not cover), or a non-C input over
//! [`CENSUS_MAX_BYTES`] that holds any directive leader (`#`, `%:`, `??=`), is a loss
//! (`incomplete`). A symlink with a C-like extension or that an include resolves to is read
//! through the link when its target lies in the repository. A non-UTF-8 file is decoded lossily, UTF-16 with a byte-order mark as
//! UTF-16. A header with an unusual extension reached only through a form prism cannot
//! resolve (a computed include, a generated header absent from the repository) acts like an
//! external header (the boundary D2 draws; #813, #812).
//!
//! The walk touches the directory listing, the parsed files, the resolved targets and the
//! C-like files; it never reads every file. Without a parsed C/C++ file nothing is walked.
//!
//! The cache identity is one aggregate `c-census:` entry of `LoadedRepo::manifest_hashes`:
//! the digest of the facts the texts yield, the files read (discovered and net), the files
//! that contributed facts and the files lost. An unrelated edit does not move it.
//!
//! Review mode parses only the diff files: [`review_gap`] reports whether the repository
//! holds census inputs the review did not read, in which case the census fails closed.
use super::{rel, SkipReason, SkippedFile};
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::{
    source_include_targets, text_has_facts, text_include_targets, texts_digest,
};
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// Files up to this size are read whole and classified; larger non-C inputs are scanned by
/// streaming for a directive leader (`#`, `%:`, `??=`, also NUL-interleaved): one is a loss.
pub(crate) const CENSUS_MAX_BYTES: u64 = 64 * 1024 * 1024;

fn is_c_family_path(path: &str) -> bool {
    matches!(Language::from_path(path), Some(Language::C | Language::Cpp))
}

fn is_c_family_parsed(p: &ParsedFile) -> bool {
    matches!(p.language, Language::C | Language::Cpp)
}

/// Does this loader skip lose census input? Everything the walk reads itself (size,
/// encoding, other languages) is judged there; the loader's own refusals that the walk
/// cannot see are an unreadable entry and a C/C++ file it could not parse or follow.
fn is_loss(s: &SkippedFile) -> bool {
    match s.reason {
        SkipReason::Unreadable => true,
        SkipReason::ParseFailed => is_c_family_path(&s.path),
        SkipReason::Symlink => is_c_like(&s.path),
        SkipReason::NotUtf8
        | SkipReason::TooLarge { .. }
        | SkipReason::Unsupported
        | SkipReason::Ignored
        | SkipReason::GoTestdata
        | SkipReason::Hidden => false,
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

/// Does `b` hold a directive leader: `#`, `%:` or `??=`, as ASCII or with NUL bytes
/// interleaved (UTF-16LE/BE)? Conservative: `%` followed by `:` directly or after a NUL.
fn has_leader(b: &[u8]) -> bool {
    b.iter().enumerate().any(|(i, &c)| match c {
        b'#' => true,
        b'%' => matches!(
            (b.get(i + 1), b.get(i + 2)),
            (Some(b':'), _) | (Some(0), Some(b':'))
        ),
        b'?' => {
            matches!(
                (b.get(i + 1), b.get(i + 2), b.get(i + 3), b.get(i + 4)),
                (Some(b'?'), Some(b'='), _, _) | (Some(0), Some(b'?'), Some(0), Some(b'='))
            )
        }
        _ => false,
    })
}

/// File extensions (lowercase) of the C-like net.
const C_LIKE: &[&str] = &[
    "c", "h", "cc", "cpp", "cxx", "hh", "hpp", "hxx", "h++", "c++", "inl", "inc", "ipp", "tcc",
    "tpp", "txx", "def", "cu", "cuh", "x",
];

fn is_c_like(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| C_LIKE.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Decode a census text: UTF-16 with a byte-order mark as UTF-16, anything else lossily as
/// UTF-8 (the permissive scan also reads a NUL-stripped variant).
pub(crate) fn decode(bytes: &[u8]) -> String {
    let utf16 = |rest: &[u8], le: bool| {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| {
                if le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Scan an oversize file in chunks for a directive leader.
fn stream_has_leader(path: &Path) -> std::io::Result<bool> {
    const CHUNK: usize = 1 << 20;
    // The longest pattern (`??=` in UTF-16, 5 bytes); keep 4 across chunks.
    const CARRY: usize = 4;
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; CHUNK + CARRY];
    let mut carried = 0usize;
    loop {
        let n = f.read(&mut buf[carried..carried + CHUNK])?;
        if n == 0 {
            return Ok(false);
        }
        let end = carried + n;
        if has_leader(&buf[..end]) {
            return Ok(true);
        }
        let keep = end.min(CARRY);
        buf.copy_within(end - keep..end, 0);
        carried = keep;
    }
}

/// What the read found.
#[derive(Default)]
struct Scan {
    /// Census inputs that yield a fact, `(path, decoded text)`.
    texts: BTreeMap<String, String>,
    /// Every non-parsed file read (discovered and net), contributing or not.
    read: BTreeSet<String>,
    /// Files and directories whose content is lost to the census, with the reason.
    lost: BTreeMap<String, String>,
}

/// Is `rel` a directory the census walk does not enter? `.git` at any depth and the root's
/// `eval/secbench` (an evidence corpus, not source).
fn excluded_dir(rel: &str, name: &str) -> bool {
    name == ".git" || rel == "eval/secbench"
}

/// Is `target` (an existing path) inside the canonical `canon_root`?
fn inside_root(canon_root: Option<&Path>, target: &Path) -> bool {
    match (canon_root, std::fs::canonicalize(target)) {
        (Some(r), Ok(t)) => t.starts_with(r),
        _ => false,
    }
}

/// A directory listing: the regular files, the symlinked files, the in-repo directory links
/// `(link, target)` (relative paths) and the losses. No file is read.
type Listing = (
    Vec<String>,
    Vec<String>,
    Vec<(String, String)>,
    BTreeMap<String, String>,
);

fn list_tree(root: &Path) -> Listing {
    let canon_root = std::fs::canonicalize(root).ok();
    let mut dir_links: Vec<(String, String)> = Vec::new();
    let mut listed = Vec::new();
    let mut links = Vec::new();
    let mut lost: BTreeMap<String, String> = BTreeMap::new();
    let dir_name = |root: &Path, dir: &Path| {
        let p = rel(root, dir);
        if p.is_empty() || p == "/" {
            ".".to_string()
        } else {
            p
        }
    };
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => {
                lost.entry(dir_name(root, &dir))
                    .or_insert_with(|| "Unreadable".into());
                continue;
            }
        };
        for entry in entries {
            let Ok(entry) = entry else {
                lost.entry(dir_name(root, &dir))
                    .or_insert_with(|| "Unreadable".into());
                continue;
            };
            let path = entry.path();
            let relp = rel(root, &path);
            let Ok(kind) = entry.file_type() else {
                lost.entry(relp).or_insert_with(|| "Unreadable".into());
                continue;
            };
            if kind.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !excluded_dir(relp.trim_end_matches('/'), &name) {
                    stack.push(path);
                }
            } else if kind.is_symlink() {
                match std::fs::metadata(&path) {
                    // A directory link into the repository is an alias: the walk lists its
                    // target's files, and `scan_tree` also indexes them under the link's name
                    // so `#include "link/x.h"` resolves. A link outside the root, or into an
                    // excluded directory, is lost.
                    Ok(m) if m.is_dir() => match dir_link_target(canon_root.as_deref(), &path) {
                        Some(target) => dir_links.push((relp, target)),
                        None => {
                            lost.entry(relp).or_insert_with(|| "SymlinkDir".into());
                        }
                    },
                    Ok(_) => links.push(relp),
                    Err(_) => {
                        if is_c_like(&relp) {
                            lost.entry(relp.clone()).or_insert_with(|| "Symlink".into());
                        }
                        links.push(relp);
                    }
                }
            } else if kind.is_file() {
                listed.push(relp);
            }
        }
    }
    (listed, links, dir_links, lost)
}

/// The in-repo, non-excluded target of a directory link, relative to the root.
fn dir_link_target(canon_root: Option<&Path>, link: &Path) -> Option<String> {
    let (r, t) = (canon_root?, std::fs::canonicalize(link).ok()?);
    let rest = t.strip_prefix(r).ok()?;
    let rel = rest.to_string_lossy().replace('\\', "/");
    let excluded = rel == "eval/secbench"
        || rel.starts_with("eval/secbench/")
        || rest.components().any(|c| c.as_os_str() == ".git");
    (!excluded).then_some(rel)
}

/// Normalize an include target lexically: `\` is `/`, `.` and empty segments go, `..`
/// pops. A `..` with nothing to pop is dropped, for angle and quoted targets alike: it is
/// relative to a directory the match does not know (the includer's, or any include directory).
fn normalize_target(target: &str) -> Option<String> {
    let t = target.replace('\\', "/");
    let mut segs: Vec<&str> = Vec::new();
    for s in t.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    (!segs.is_empty()).then(|| segs.join("/"))
}

/// Every in-repo file a target can name: a path (real, or an alias through an in-repo
/// directory link) equal to it or ending in `/<target>`. Yields the real paths.
fn resolve<'a>(
    by_name: &'a HashMap<&str, Vec<(&'a str, &'a str)>>,
    target: &str,
) -> impl Iterator<Item = &'a str> {
    let norm = normalize_target(target);
    let base = norm
        .as_deref()
        .map(|n| n.rsplit('/').next().unwrap_or(n).to_owned());
    let suffix = norm.as_ref().map(|n| format!("/{n}"));
    base.and_then(|b| by_name.get(b.as_str()))
        .into_iter()
        .flatten()
        .filter(move |(p, _)| {
            let n = norm.as_deref().unwrap_or("");
            *p == n || suffix.as_deref().is_some_and(|s| p.ends_with(s))
        })
        .map(|(_, real)| *real)
}

/// Discover and read the census inputs under `root` for the parsed `files`.
fn scan_tree(root: &Path, files: &BTreeMap<String, ParsedFile>) -> Scan {
    TREE_WALKS.with(|c| c.set(c.get() + 1));
    let (listed, links, dir_links, lost) = list_tree(root);
    let canon_root = std::fs::canonicalize(root).ok();
    let link_set: BTreeSet<&str> = links.iter().map(String::as_str).collect();
    let mut scan = Scan {
        lost,
        ..Scan::default()
    };
    // Aliases: a file under an in-repo directory link's target is also named through the
    // link (one level; links inside links are not chased).
    let aliases: Vec<(String, &str)> = dir_links
        .iter()
        .flat_map(|(link, target)| {
            let prefix = format!("{target}/");
            listed.iter().chain(&links).filter_map(move |p| {
                p.strip_prefix(&prefix)
                    .map(|rest| (format!("{link}/{rest}"), p.as_str()))
            })
        })
        .collect();
    let mut by_name: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for p in listed.iter().chain(&links) {
        by_name
            .entry(p.rsplit('/').next().unwrap_or(p))
            .or_default()
            .push((p.as_str(), p.as_str()));
    }
    for (alias, real) in &aliases {
        by_name
            .entry(alias.rsplit('/').next().unwrap_or(alias))
            .or_default()
            .push((alias.as_str(), real));
    }
    let parsed_c = |p: &str| files.get(p).is_some_and(is_c_family_parsed);
    let mut queue: Vec<String> = listed
        .iter()
        .chain(&links)
        .filter(|p| is_c_like(p) && !parsed_c(p))
        .cloned()
        .collect();
    let push_targets =
        |targets: BTreeSet<(String, bool)>, queue: &mut Vec<String>, read: &BTreeSet<String>| {
            for (t, _quoted) in targets {
                for p in resolve(&by_name, &t) {
                    if !parsed_c(p) && !read.contains(p) {
                        queue.push(p.to_owned());
                    }
                }
            }
        };
    for p in files.values().filter(|p| is_c_family_parsed(p)) {
        push_targets(source_include_targets(&p.source), &mut queue, &scan.read);
    }
    while let Some(path) = queue.pop() {
        if !scan.read.insert(path.clone()) {
            continue;
        }
        let full = root.join(&path);
        let Ok(meta) = std::fs::metadata(&full) else {
            scan.lost.entry(path).or_insert_with(|| "Unreadable".into());
            continue;
        };
        // A symlink is read through the link only when its target lies in the repository.
        if link_set.contains(path.as_str()) && !inside_root(canon_root.as_deref(), &full) {
            scan.lost.entry(path).or_insert_with(|| "Symlink".into());
            continue;
        }
        if meta.len() > CENSUS_MAX_BYTES {
            match stream_has_leader(&full) {
                Ok(true) => {
                    scan.lost
                        .entry(path)
                        .or_insert_with(|| "OversizeDirective".into());
                }
                Ok(false) => {}
                Err(_) => {
                    scan.lost.entry(path).or_insert_with(|| "Unreadable".into());
                }
            }
            continue;
        }
        let Ok(bytes) = std::fs::read(&full) else {
            scan.lost.entry(path).or_insert_with(|| "Unreadable".into());
            continue;
        };
        let text = decode(&bytes);
        push_targets(text_include_targets(&text), &mut queue, &scan.read);
        if text_has_facts(&text) {
            scan.texts.insert(path, text);
        }
    }
    scan
}

/// Is the census text `text` still what the disk holds for `rel`? (A freshness probe's
/// byte comparison, through the same decoding the census applied.)
pub fn text_is_current(root: &Path, rel: &str, text: &str) -> bool {
    std::fs::read(root.join(rel)).is_ok_and(|b| decode(&b) == text)
}

thread_local! {
    /// Whole-tree census walks started on this thread (a test seam: a repository with no
    /// parsed C/C++ file must start none).
    static TREE_WALKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Census walks started on this thread so far.
#[cfg(test)]
pub(crate) fn tree_walks() -> usize {
    TREE_WALKS.with(std::cell::Cell::get)
}

/// What the loader found for the census.
#[derive(Default)]
pub(crate) struct CensusInputs {
    /// Census inputs yielding a fact or an `incomplete` trigger, `(path, text)`.
    pub(crate) texts: BTreeMap<String, String>,
    /// Paths of lost inputs (the census is `incomplete`).
    pub(crate) lost: BTreeSet<String>,
    /// The aggregate `c-census:` topology entry (absent without a parsed C/C++ file).
    pub(crate) identity: BTreeMap<String, String>,
    /// A census input was lost.
    pub(crate) incomplete: bool,
}

impl CensusInputs {
    /// Stamp every parsed C/C++ file with the loss flag and the text files.
    pub(crate) fn apply(&self, files: &mut BTreeMap<String, ParsedFile>) {
        let texts = Arc::new(self.texts.clone());
        let lost = Arc::new(self.lost.clone());
        for parsed in files.values_mut().filter(|p| is_c_family_parsed(p)) {
            if self.incomplete {
                parsed.mark_c_family_skipped_in_repo();
            }
            if !self.texts.is_empty() {
                parsed.set_c_census_texts(Arc::clone(&texts));
            }
            if !self.lost.is_empty() {
                parsed.set_c_census_lost(Arc::clone(&lost));
            }
        }
    }
}

/// The aggregate identity: the facts of the text inputs, the files read, the files that
/// contributed facts and the files lost, `path\0reason`.
fn aggregate(scan: &Scan, lost: &BTreeMap<String, String>) -> String {
    let mut s = String::from("c-census-aggregate-v4\nfacts\n");
    s.push_str(&texts_digest(&scan.texts));
    s.push_str("\nread\n");
    for p in &scan.read {
        s.push_str(p);
        s.push('\n');
    }
    s.push_str("contributors\n");
    for p in scan.texts.keys() {
        s.push_str(p);
        s.push('\n');
    }
    s.push_str("lost\n");
    for (p, why) in lost {
        s.push_str(p);
        s.push('\0');
        s.push_str(why);
        s.push('\n');
    }
    sha(s.as_bytes())
}

pub(crate) fn gather(
    root: &Path,
    files: &BTreeMap<String, ParsedFile>,
    skipped: &[SkippedFile],
) -> CensusInputs {
    let mut g = CensusInputs::default();
    // Without a parsed C/C++ file there is no census to feed: no walk at all.
    if !files.values().any(is_c_family_parsed) {
        return g;
    }
    let mut scan = scan_tree(root, files);
    let mut lost: BTreeMap<String, String> = skipped
        .iter()
        .filter(|s| is_loss(s))
        .map(|s| (s.path.clone(), format!("{:?}", s.reason)))
        .collect();
    for (p, why) in std::mem::take(&mut scan.lost) {
        lost.entry(p).or_insert(why);
    }
    if !lost.is_empty() {
        trace_reason(
            "census_input_lost",
            &lost
                .iter()
                .map(|(p, w)| format!("{p} {w}"))
                .collect::<Vec<_>>(),
        );
        g.incomplete = true;
    }
    if std::env::var_os("PRISM_C_PRUNE_TRACE").is_some() {
        eprintln!(
            "c-census inputs: read={} texts={}",
            scan.read.len(),
            scan.texts.len()
        );
        for p in &scan.texts.keys().cloned().collect::<Vec<_>>() {
            eprintln!("c-census text\t{p}");
        }
    }
    g.identity
        .insert("c-census:aggregate".into(), aggregate(&scan, &lost));
    g.lost = lost.into_keys().collect();
    g.texts = scan.texts;
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
    // Every other repository file the review did not parse is read the same way: one that
    // yields a fact, or is lost to the census, is an unread census input.
    let scan = scan_tree(root, files);
    gap.extend(scan.texts.into_keys());
    gap.extend(scan.lost.into_iter().map(|(p, w)| format!("{p}\0{w}")));
    if !gap.is_empty() {
        trace_reason(
            "review_gap",
            &gap.iter().take(20).cloned().collect::<Vec<_>>(),
        );
    }
    (!gap.is_empty()).then(|| sha(gap.into_iter().collect::<Vec<_>>().join("\n").as_bytes()))
}
