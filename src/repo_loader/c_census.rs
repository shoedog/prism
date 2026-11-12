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
//!   An angle target that climbs out of the repository root is external; a quoted one drops
//!   the leading `..` it cannot resolve (it is relative to the including directory). A
//!   resolved target that is not a parsed C/C++ file is read as census text whatever its
//!   extension or language.
//! * **The C-like-extension net.** Every in-repo file with a C-like extension (see
//!   [`C_LIKE`]) that is not a parsed C/C++ file is read too, whether or not anything
//!   visibly includes it (computed includes, unresolved spellings).
//!
//! Census texts are read with the PERMISSIVE scan (`macro_census::collect_permissive`): no
//! comment or string tracking, and never `incomplete` from lexer state. Only an unreadable
//! file or directory, or a file over [`CENSUS_MAX_BYTES`] that contains `define`, is a loss
//! (`incomplete`). A non-UTF-8 file is decoded lossily, UTF-16 with a byte-order mark as
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

/// Files up to this size are read whole and classified; larger ones are scanned by
/// streaming for the `define`/`undef` byte patterns.
pub(crate) const CENSUS_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The directive words whose bytes make an unreadable file a loss.
const WORDS: [&[u8]; 2] = [b"define", b"undef"];

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
        SkipReason::ParseFailed | SkipReason::Symlink => is_c_family_path(&s.path),
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

/// Does `b` hold a directive word as ASCII (`ascii`) or as UTF-16LE/BE (`utf16`)?
fn has_word(b: &[u8], ascii: bool, utf16: bool) -> bool {
    for (i, &c) in b.iter().enumerate() {
        if c != b'd' && c != b'u' {
            continue;
        }
        for w in WORDS.iter().filter(|w| w[0] == c) {
            if ascii && b[i..].starts_with(w) {
                return true;
            }
            if !utf16 {
                continue;
            }
            // LE: `d 0 e 0 ..`; BE: `0 d 0 e ..` (the NUL before the first letter).
            let le = (0..w.len()).all(|k| {
                b.get(i + 2 * k) == Some(&w[k])
                    && (k + 1 == w.len() || b.get(i + 2 * k + 1) == Some(&0))
            });
            let be = i >= 1
                && (0..w.len())
                    .all(|k| b.get(i - 1 + 2 * k) == Some(&0) && b.get(i + 2 * k) == Some(&w[k]));
            if le || be {
                return true;
            }
        }
    }
    false
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

/// Scan an oversize file in chunks for the directive-word bytes.
fn stream_has_word(path: &Path) -> std::io::Result<bool> {
    const CHUNK: usize = 1 << 20;
    // The longest pattern (`define` in UTF-16) is 12 bytes; keep 11 across chunks.
    const CARRY: usize = 11;
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; CHUNK + CARRY];
    let mut carried = 0usize;
    loop {
        let n = f.read(&mut buf[carried..carried + CHUNK])?;
        if n == 0 {
            return Ok(false);
        }
        let end = carried + n;
        if has_word(&buf[..end], true, true) {
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

/// A directory listing: the regular files (relative paths) and the losses. No file is read.
fn list_tree(root: &Path) -> (Vec<String>, BTreeMap<String, String>) {
    let mut listed = Vec::new();
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
                if is_c_family_path(&relp) {
                    lost.entry(relp).or_insert_with(|| "Symlink".into());
                }
            } else if kind.is_file() {
                listed.push(relp);
            }
        }
    }
    (listed, lost)
}

/// Normalize an include target lexically: `\` is `/`, `.` and empty segments go, `..`
/// pops. A `..` with nothing to pop makes an angle target external (`None`); a quoted target
/// drops it (it is relative to the including directory, which the match does not know).
fn normalize_target(target: &str, quoted: bool) -> Option<String> {
    let t = target.replace('\\', "/");
    let mut segs: Vec<&str> = Vec::new();
    for s in t.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                if segs.pop().is_none() && !quoted {
                    return None;
                }
            }
            s => segs.push(s),
        }
    }
    (!segs.is_empty()).then(|| segs.join("/"))
}

/// Every in-repo file a target can name: path equal to it, or ending in `/<target>`.
fn resolve<'a>(
    by_name: &'a HashMap<&str, Vec<&str>>,
    target: &str,
    quoted: bool,
) -> impl Iterator<Item = &'a str> {
    let norm = normalize_target(target, quoted);
    let base = norm
        .as_deref()
        .map(|n| n.rsplit('/').next().unwrap_or(n).to_owned());
    let suffix = norm.as_ref().map(|n| format!("/{n}"));
    base.and_then(|b| by_name.get(b.as_str()))
        .into_iter()
        .flatten()
        .copied()
        .filter(move |p| {
            let n = norm.as_deref().unwrap_or("");
            *p == n || suffix.as_deref().is_some_and(|s| p.ends_with(s))
        })
}

/// Discover and read the census inputs under `root` for the parsed `files`.
fn scan_tree(root: &Path, files: &BTreeMap<String, ParsedFile>) -> Scan {
    TREE_WALKS.with(|c| c.set(c.get() + 1));
    let (listed, lost) = list_tree(root);
    let mut scan = Scan {
        lost,
        ..Scan::default()
    };
    let mut by_name: HashMap<&str, Vec<&str>> = HashMap::new();
    for p in &listed {
        by_name
            .entry(p.rsplit('/').next().unwrap_or(p))
            .or_default()
            .push(p);
    }
    let parsed_c = |p: &str| files.get(p).is_some_and(is_c_family_parsed);
    let mut queue: Vec<String> = listed
        .iter()
        .filter(|p| is_c_like(p) && !parsed_c(p))
        .cloned()
        .collect();
    let push_targets =
        |targets: BTreeSet<(String, bool)>, queue: &mut Vec<String>, read: &BTreeSet<String>| {
            for (t, quoted) in targets {
                for p in resolve(&by_name, &t, quoted) {
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
        if meta.len() > CENSUS_MAX_BYTES {
            match stream_has_word(&full) {
                Ok(true) => {
                    scan.lost
                        .entry(path)
                        .or_insert_with(|| "OversizeDefine".into());
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
