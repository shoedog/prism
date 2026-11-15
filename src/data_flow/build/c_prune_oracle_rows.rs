//! #829 test form, layer L1 (docs/superpowers/plans/2026-11-14-829-macro-brackets/TEST-FORM.md
//! sections 4 and 5): what a pair row asserts is DATA computed by the expansion-arm oracle
//! (`scripts/dfg_prune/xarm.py`, which needs clang at authoring time only) and frozen in
//! `tests/fixtures/dfg_prune/oracle/pair_rows.jsonl` (rows built from Rust sources, keyed by a
//! content hash) and in the `oracle` fields of the inline `obligations.jsonl` rows. These tests
//! read the data and need no clang.
//!
//! `assert` values: `kept` (the pair must survive: the proof fails on the compiler's token
//! stream), `dropped` (a validity control: the pair is droppable without the macro definitions,
//! which is what makes the twin `kept` row fail for the right reason), `pinned_hole` (a
//! BOUNDARY row whose truth needs an input prism cannot see: today's drop is asserted, so a fix
//! flips it visibly), `guide` (reported on stderr as `GUIDE ...`, never asserted: the oracle
//! says the proof holds, or the row is a control) and `no_basis` (the expansion is not a valid
//! program or the oracle could not judge; recorded, not asserted).
//!
//! Export mode: with `PRISM_829_EXPORT_DIR=<dir>` every pair row judged by a test is written to
//! `<dir>/rows/<key>/` (its files under `f/`, `row.json` beside them) and nothing is asserted;
//! the inline cases are written by `export_inline_cases` (ignored test, same variable). See
//! `scripts/dfg_prune/XARM.md`.
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::OnceLock;

/// The hand expectation of a row (what the old assertion said), kept for the report.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Want {
    Kept,
    Pruned,
}

/// The configuration a BOUNDARY row declares: the input the compiler had and prism cannot see.
#[derive(Clone, Debug, Default)]
pub(super) struct Declared {
    pub assumption: &'static str,
    pub flags: Vec<String>,
    /// `(file name, contents)` of headers the declared configuration provides outside the repo.
    pub prelude: Vec<(String, String)>,
}

struct RowCtx {
    files: BTreeMap<String, Vec<u8>>,
    links: BTreeMap<String, String>,
    texts: BTreeSet<String>,
    /// Spellings of the tempdir root, replaced in the key (an absolute include names it).
    roots: Vec<String>,
    target: String,
    owner: Option<String>,
    path: String,
    from: usize,
    to: usize,
}

thread_local! {
    static LAST: RefCell<Option<RowCtx>> = const { RefCell::new(None) };
    static DECL: RefCell<Option<Declared>> = const { RefCell::new(None) };
}

fn set_last(ctx: RowCtx) {
    LAST.with(|l| *l.borrow_mut() = Some(ctx));
}

/// Records an in-memory row (`files` plus census `texts`, stamped on the target).
#[allow(clippy::too_many_arguments)]
pub(super) fn record_files(
    files: &[(&str, &str)],
    texts: &[(&str, &str)],
    target: &str,
    owner: Option<&str>,
    path: &str,
    from: usize,
    to: usize,
) {
    let mut all: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (n, s) in files.iter().chain(texts.iter()) {
        all.insert(n.to_string(), s.as_bytes().to_vec());
    }
    set_last(RowCtx {
        files: all,
        links: BTreeMap::new(),
        texts: texts.iter().map(|(n, _)| n.to_string()).collect(),
        roots: Vec::new(),
        target: target.to_owned(),
        owner: owner.map(str::to_owned),
        path: path.to_owned(),
        from,
        to,
    });
}

fn walk(root: &Path, dir: &Path, ctx: &mut RowCtx) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let p = entry.path();
        let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
        let Ok(meta) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            if let Ok(t) = std::fs::read_link(&p) {
                ctx.links.insert(rel, t.to_string_lossy().into_owned());
            }
        } else if meta.is_dir() {
            walk(root, &p, ctx);
        } else if let Ok(b) = std::fs::read(&p) {
            ctx.files.insert(rel, b);
        }
    }
}

/// Records a loader row: a snapshot of the tempdir `d` (the loader's census is the input).
pub(super) fn record_dir(
    d: &Path,
    target: &str,
    owner: Option<&str>,
    path: &str,
    from: usize,
    to: usize,
) {
    let mut ctx = RowCtx {
        files: BTreeMap::new(),
        links: BTreeMap::new(),
        texts: BTreeSet::new(),
        roots: vec![d.to_string_lossy().into_owned()],
        target: target.to_owned(),
        owner: owner.map(str::to_owned),
        path: path.to_owned(),
        from,
        to,
    };
    if let Ok(c) = d.canonicalize() {
        ctx.roots.push(c.to_string_lossy().into_owned());
    }
    walk(d, d, &mut ctx);
    set_last(ctx);
}

/// Declares the configuration of the NEXT judged row (BOUNDARY rows): the assumption it
/// depends on, the compiler flags, and headers outside the repository.
pub(super) fn declare(assumption: &'static str, flags: &[&str], prelude: &[(&str, &str)]) {
    let d = Declared {
        assumption,
        flags: flags.iter().map(|s| s.to_string()).collect(),
        prelude: prelude
            .iter()
            .map(|(n, c)| (n.to_string(), c.to_string()))
            .collect(),
    };
    DECL.with(|c| *c.borrow_mut() = Some(d));
}

/// `bytes` with every occurrence of each root spelling replaced by `<ROOT>`.
fn unroot(bytes: &[u8], roots: &[String]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    for r in roots.iter().filter(|r| !r.is_empty()) {
        let needle = r.as_bytes();
        let mut i = 0;
        let mut next = Vec::with_capacity(out.len());
        while i < out.len() {
            if out[i..].starts_with(needle) {
                next.extend_from_slice(b"<ROOT>");
                i += needle.len();
            } else {
                next.push(out[i]);
                i += 1;
            }
        }
        out = next;
    }
    out
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
    *h ^= 0xff;
    *h = h.wrapping_mul(0x100000001b3);
}

impl RowCtx {
    /// Content key: every file, link and the pair; the declared configuration is metadata.
    fn key(&self) -> String {
        let mut h = 0xcbf29ce484222325u64;
        for (n, c) in &self.files {
            fnv(&mut h, n.as_bytes());
            fnv(&mut h, &unroot(c, &self.roots));
        }
        for (n, t) in &self.links {
            fnv(&mut h, n.as_bytes());
            fnv(&mut h, t.as_bytes());
        }
        for t in &self.texts {
            fnv(&mut h, t.as_bytes());
        }
        let pair = format!(
            "{}|{}|{}|{}|{}",
            self.target,
            self.owner.as_deref().unwrap_or("*"),
            self.path,
            self.from,
            self.to
        );
        fnv(&mut h, pair.as_bytes());
        format!("{h:016x}")
    }
}

fn policy_table() -> &'static HashMap<String, serde_json::Value> {
    static T: OnceLock<HashMap<String, serde_json::Value>> = OnceLock::new();
    T.get_or_init(|| {
        include_str!("../../../tests/fixtures/dfg_prune/oracle/pair_rows.jsonl")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                (v["key"].as_str().unwrap().to_owned(), v)
            })
            .collect()
    })
}

fn export(dir: &std::ffi::OsStr, ctx: &RowCtx, what: &str, want: Want, ba: (bool, bool)) {
    use std::fs;
    let key = ctx.key();
    let root = Path::new(dir).join("rows").join(&key);
    if root.join("row.json").exists() {
        return;
    }
    let decl = DECL.with(|c| c.borrow().clone());
    for (n, c) in &ctx.files {
        let p = root.join("f").join(n);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(p, c);
    }
    let row = serde_json::json!({
        "key": key,
        "test": std::thread::current().name().unwrap_or("?"),
        "what": what,
        "want": if want == Want::Kept { "kept" } else { "pruned" },
        "target": ctx.target,
        "owner": ctx.owner,
        "path": ctx.path,
        "from": ctx.from,
        "to": ctx.to,
        "texts": ctx.texts,
        "files": ctx.files.keys().collect::<Vec<_>>(),
        "links": ctx.links,
        "head_before": ba.0,
        "head_after": ba.1,
        "declared": decl.map(|d| serde_json::json!({
            "assumption": d.assumption, "flags": d.flags, "prelude": d.prelude,
        })),
    });
    let _ = fs::create_dir_all(&root);
    let _ = fs::write(
        root.join("row.json"),
        serde_json::to_string_pretty(&row).unwrap(),
    );
}

/// Judges the row recorded by the last `record_*` of this thread. `(before, after)` is whether
/// the pair exists before and after the rules. An inadmissible row (no baseline pair, so a
/// removal-only rule could not drop it) panics, as it always did, except in export mode.
pub(super) fn judge(what: &str, want: Want, (before, after): (bool, bool)) -> Result<(), String> {
    let ctx = LAST
        .with(|l| l.borrow_mut().take())
        .expect("a pair row must record its inputs (pair_in / loader_pair) before judging");
    if let Some(dir) = std::env::var_os("PRISM_829_EXPORT_DIR") {
        export(&dir, &ctx, what, want, (before, after));
        DECL.with(|c| *c.borrow_mut() = None);
        return Ok(());
    }
    DECL.with(|c| *c.borrow_mut() = None);
    let pair = format!(
        "{}@{} -> {}@{} (owner {:?})",
        ctx.path, ctx.from, ctx.path, ctx.to, ctx.owner
    );
    assert!(
        before,
        "{what}: inadmissible, the baseline has no {pair} ({:?})",
        ctx.target
    );
    let key = ctx.key();
    let Some(row) = policy_table().get(&key) else {
        return Err(format!(
            "{what}: unlabeled row {key}: run `python3 scripts/dfg_prune/xarm.py label` \
             over an export (see XARM.md) and regenerate pair_rows.jsonl"
        ));
    };
    apply(what, want, &pair, row, after)
}

fn apply(
    what: &str,
    want: Want,
    pair: &str,
    row: &serde_json::Value,
    after: bool,
) -> Result<(), String> {
    let oracle = row["oracle"].as_str().unwrap_or("?");
    match row["assert"].as_str().unwrap_or("?") {
        "kept" if !after => Err(format!(
            "{what}: the oracle ({oracle}) says the proof fails on the compiler's tokens, so \
             {pair} must be kept, and it was dropped"
        )),
        "dropped" if after => Err(format!(
            "{what}: validity control, {pair} is droppable without the macro definitions and \
             must be pruned"
        )),
        "pinned_hole" if after => Err(format!(
            "{what}: pinned hole {} closed (the pair is now kept): update the oracle data",
            row["assumption"].as_str().unwrap_or("?")
        )),
        "pinned_hole" => Ok(()),
        "guide" => {
            eprintln!("GUIDE {what}: want={want:?} oracle={oracle} kept={after} ({pair})");
            Ok(())
        }
        "kept" | "dropped" | "no_basis" => Ok(()),
        other => Err(format!(
            "{what}: unknown assert {other:?} in the oracle data"
        )),
    }
}

/// An inline obligation row (`obligations.jsonl`): `had`/`has` are the owners that had the
/// pair before and have it after. Rows with no oracle data keep the old keep/drop meaning.
pub(super) fn judge_inline(
    row: &serde_json::Value,
    had: &BTreeSet<(String, usize)>,
    has: &BTreeSet<(String, usize)>,
) -> Option<String> {
    let after = !had.is_empty() && had.is_subset(has);
    let dropped_all = !had.is_empty() && has.is_empty();
    let fail = |m: &str| Some(format!("{row} {m} owners before={had:?} after={has:?}"));
    match row.get("assert").and_then(|a| a.as_str()) {
        None => {
            let ok = if row["keep"].as_bool().unwrap() {
                after
            } else {
                dropped_all
            };
            (!ok).then(|| format!("{row} owners before={had:?} after={has:?}"))
        }
        Some("kept") => (!after).then(|| fail("oracle: must be kept")).flatten(),
        Some("dropped") => (!dropped_all)
            .then(|| fail("control: must be pruned"))
            .flatten(),
        Some("pinned_hole") => (!dropped_all)
            .then(|| fail("pinned hole closed, update the data"))
            .flatten(),
        Some("guide") => {
            eprintln!(
                "GUIDE {}:{} oracle={} kept={after}",
                row["case"].as_str().unwrap_or("?"),
                row["id"].as_str().unwrap_or("?"),
                row["oracle"].as_str().unwrap_or("?")
            );
            None
        }
        Some("no_basis") => None,
        Some(other) => Some(format!("{row} unknown assert {other:?}")),
    }
}

/// Writes every inline case (C and C++) to `<dir>/inline/{c,cpp}/<file>` for the oracle.
/// `PRISM_829_EXPORT_DIR=/tmp/x cargo test -p prism --lib export_inline_cases -- --ignored`
#[test]
#[ignore = "authoring tool (XARM.md): needs PRISM_829_EXPORT_DIR"]
fn export_inline_cases() {
    use super::c_prune_inline_cases as c;
    use super::cpp_prune_inline_cases as cpp;
    let dir = std::env::var("PRISM_829_EXPORT_DIR").expect("set PRISM_829_EXPORT_DIR");
    for (sub, cases) in [
        (
            "c",
            c::INLINE_CASES
                .iter()
                .map(|(n, s)| (n.to_string(), c::inline_source(s)))
                .collect::<Vec<_>>(),
        ),
        (
            "cpp",
            cpp::INLINE_CASES
                .iter()
                .map(|(n, s)| (n.to_string(), cpp::inline_source(s)))
                .collect(),
        ),
    ] {
        let out = Path::new(&dir).join("inline").join(sub);
        std::fs::create_dir_all(&out).unwrap();
        for (name, src) in cases {
            std::fs::write(out.join(name), src).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(assert: &str) -> serde_json::Value {
        serde_json::json!({"assert": assert, "oracle": "CONTRADICTED", "assumption": "A-inputs",
            "case": "c", "id": "r", "keep": true})
    }

    #[test]
    fn kept_fails_exactly_when_the_pair_is_dropped() {
        let r = row("kept");
        assert!(apply("w", Want::Kept, "p", &r, true).is_ok());
        assert!(apply("w", Want::Kept, "p", &r, false).is_err());
    }

    #[test]
    fn pinned_hole_and_validity_control_assert_the_drop() {
        assert!(apply("w", Want::Kept, "p", &row("pinned_hole"), false).is_ok());
        assert!(apply("w", Want::Kept, "p", &row("pinned_hole"), true).is_err());
        assert!(apply("w", Want::Pruned, "p", &row("dropped"), false).is_ok());
        assert!(apply("w", Want::Pruned, "p", &row("dropped"), true).is_err());
    }

    #[test]
    fn guide_and_no_basis_never_fail() {
        for a in ["guide", "no_basis"] {
            for after in [true, false] {
                assert!(apply("w", Want::Kept, "p", &row(a), after).is_ok());
            }
        }
        assert!(apply("w", Want::Kept, "p", &row("bogus"), true).is_err());
    }

    #[test]
    fn inline_rows_without_data_keep_the_old_meaning() {
        let owners = |v: &[(&str, usize)]| -> BTreeSet<(String, usize)> {
            v.iter().map(|(n, l)| (n.to_string(), *l)).collect()
        };
        let had = owners(&[("f", 1)]);
        let keep = serde_json::json!({"keep": true});
        assert!(judge_inline(&keep, &had, &had).is_none());
        assert!(judge_inline(&keep, &had, &owners(&[])).is_some());
        let drop = serde_json::json!({"keep": false});
        assert!(judge_inline(&drop, &had, &owners(&[])).is_none());
        assert!(judge_inline(&drop, &had, &had).is_some());
    }

    #[test]
    fn the_key_depends_on_content_and_pair() {
        let mk = |body: &str, from: usize| {
            record_files(&[("a.c", body)], &[], "a.c", Some("f"), "x", from, 9);
            LAST.with(|l| l.borrow_mut().take().unwrap().key())
        };
        assert_eq!(mk("int x;", 5), mk("int x;", 5));
        assert_ne!(mk("int x;", 5), mk("int y;", 5));
        assert_ne!(mk("int x;", 5), mk("int x;", 6));
    }
}
