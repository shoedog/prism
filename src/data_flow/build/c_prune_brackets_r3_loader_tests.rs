//! #829 chain B, round 3: loader rows. The macro definitions sit physically in the repository
//! (or in a file the build can read), but the loader does not hand them to the census, or
//! hands over a different file: a non-UTF-8 or UTF-16 payload (sonnet3-05, 06), a header
//! outside `--repo` (sonnet3-09), a compiler-mapped include (sol3-01), a backslash include
//! that collides with a directory path (sol3-02), a C++20 header-unit import (sol3-03), a
//! Unicode-normalization mismatch (opus3-11) and the excluded `eval/secbench` tree (opus3-12).
//! Every row asserts the REQUIRED behaviour: the live pair `x = 41 -> return` is kept, never a
//! specific `Off::` reason. `boundary_*` rows depend on an input the loader cannot see.
//! Real files live in a tempdir built from Rust strings, never under `tests/fixtures`.
//! Host-dependent rows (backslash and Unicode file names) check a capability first and print
//! `SKIP <row>: <reason>` when the host cannot express the input.
//! The cache lifecycle rows (terra3-03, and the converted g_sol_13) are at the end.
use super::c_prune_brackets_r3_support::{guard_pair, Batch};
use super::Producer;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::cpg::CodePropertyGraph;
use crate::cpg::CpgContext;
use crate::cpg_cache::{
    compute_topology_key, load_cache_with_topology, save_cache_with_topology, CacheResult,
};
use crate::repo_loader::load_repo;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const PAYLOAD: &str = "#define OPEN {\n#define CLOSE }\n";

fn wb(d: &Path, rel: &str, bytes: &[u8]) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn ret(cpp: bool) -> &'static str {
    if cpp {
        "sink(x + 1)"
    } else {
        "x"
    }
}

/// `{prefix}` + the shared guard owner (`OPEN`/`CLOSE` come from the include).
fn owner_text(prefix: &str, cpp: bool) -> String {
    format!(
        "{prefix}int sink(int);\nint probe(int c) {{\n    int x = 41;\n    if (c) OPEN;\n    x = 2;\n    CLOSE;\n    return {};\n}}\n",
        ret(cpp)
    )
}

/// `(before the rules, after the rules)` of `x@from -> x@to` of `owner` in `target`, with
/// the census and the census texts the loader builds for `d`.
fn loader_pair(d: &Path, target: &str, owner: &str, from: usize, to: usize) -> (bool, bool) {
    let repo = load_repo(d).unwrap();
    let census = CMacroCensus::from_files(&repo.files);
    let p = &repo.files[target];
    let file = target.to_owned();
    let has = |r: &super::FileRefs| {
        r.edges.iter().any(|e| {
            e.from.path.to_string() == "x"
                && e.from.line == from
                && e.to.line == to
                && e.from.function == owner
        })
    };
    let before = Producer::file_refs_frozen(&file, p);
    let after = super::alias_occurrence::without_policy(|| {
        Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &census)
    });
    (has(&before), has(&after))
}

/// Premise of the control rows: the loader's census holds the payload's macros.
fn assert_seen(d: &Path) {
    let census = CMacroCensus::from_files(&load_repo(d).unwrap().files);
    for n in ["OPEN", "CLOSE"] {
        assert!(census.get(n).is_some(), "{n} not read by the loader");
    }
}

/// `target` of `root` (the loader root) against the guard pair of its own text.
fn kept_in(b: &mut Batch, what: &str, root: &Path, target: &str, text: &str) {
    let (from, to) = guard_pair(text);
    let r = loader_pair(root, target, "probe", from, to);
    b.kept_result(what, r, Some("probe"), from, to);
}

fn exts() -> [(&'static str, bool); 2] {
    [("c", false), ("cc", true)]
}

// ---- G2-sonnet3-05: a Latin-1 byte in the payload ----

/// `/* caf\xE9 */` before the definitions: clang reads the comment and the program prints
/// `41 2`; the loader reads `defs.payload` (`read=1`, two macros) after a lossy decode, so the
/// facts must survive the decode.
#[test]
fn row_g2_sonnet3_05_latin1_payload_and_header() {
    let mut b = Batch::new();
    let defs: Vec<u8> = [b"/* caf\xE9 */\n".as_slice(), PAYLOAD.as_bytes()].concat();
    for (ext, cpp) in exts() {
        for name in ["defs.payload", "defs.h"] {
            let d = tempfile::tempdir().unwrap();
            let text = owner_text(&format!("#include \"{name}\"\n"), cpp);
            wb(d.path(), &format!("a.{ext}"), text.as_bytes());
            wb(d.path(), name, &defs);
            assert_seen(d.path());
            kept_in(
                &mut b,
                &format!("sonnet3-05 {name} (a.{ext})"),
                d.path(),
                &format!("a.{ext}"),
                &text,
            );
        }
    }
    b.finish();
}

// ---- G2-sonnet3-06 (C, rules from MSVC behaviour): UTF-16 payloads and headers ----

fn utf16(s: &str, big_endian: bool, bom: bool) -> Vec<u8> {
    let mut out = Vec::new();
    if bom {
        out.extend(if big_endian {
            [0xFE, 0xFF]
        } else {
            [0xFF, 0xFE]
        });
    }
    for u in s.encode_utf16() {
        out.extend(if big_endian {
            u.to_be_bytes()
        } else {
            u.to_le_bytes()
        });
    }
    out
}

const UTF16_TEXT: &str = "#define OPEN zzname {\n#define CLOSE }\n";

/// The bracket text in UTF-16 must give the UTF-8 outcome (owner off, or the pair kept). clang
/// cannot compile the files (BOM unsupported), MSVC reads the BOM forms: no liveness run.
#[test]
fn row_g2_sonnet3_06_utf16_payload_and_bom_header() {
    let mut b = Batch::new();
    let variants: [(&str, &str, Vec<u8>); 4] = [
        (
            "payload LE BOM",
            "defs.payload",
            utf16(UTF16_TEXT, false, true),
        ),
        (
            "payload BE BOM",
            "defs.payload",
            utf16(UTF16_TEXT, true, true),
        ),
        (
            "payload LE no BOM",
            "defs.payload",
            utf16(UTF16_TEXT, false, false),
        ),
        ("header LE BOM", "defs.h", utf16(UTF16_TEXT, false, true)),
    ];
    for (ext, cpp) in exts() {
        for (what, name, bytes) in &variants {
            let d = tempfile::tempdir().unwrap();
            let text = owner_text(&format!("#include \"{name}\"\n"), cpp);
            wb(d.path(), &format!("a.{ext}"), text.as_bytes());
            wb(d.path(), name, bytes);
            kept_in(
                &mut b,
                &format!("sonnet3-06 {what} (a.{ext})"),
                d.path(),
                &format!("a.{ext}"),
                &text,
            );
        }
    }
    b.finish();
}

/// Control (K, green on base): a UTF-16 header without a BOM cannot be parsed or decoded, so
/// the census input is lost and every C/C++ owner stays off (`off:Incomplete`): the pair is
/// kept under any design that fails closed on a lost input.
#[test]
fn row_g2_sonnet3_06_control_utf16_header_without_bom_keeps_the_pair() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"defs.h\"\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        wb(d.path(), "defs.h", &utf16(UTF16_TEXT, false, false));
        kept_in(
            &mut b,
            &format!("sonnet3-06 header LE no BOM (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

// ---- G2-sonnet3-09 (BOUNDARY): the header lives outside --repo ----

/// `mono/common/defs.h` holds the macros, `mono/pkg/a.c` includes `../common/defs.h`; prism
/// runs with `--repo mono/pkg` (a package of a monorepo): `read=0`. The macros are the
/// project's own but the analysis root excludes them, so under D2 they are "external". Same
/// for an angle include of a project header found through a `-I` directory outside the root.
/// Compiled with the real tree: `41 2`. Needs the owner's call.
#[test]
fn boundary_g2_sonnet3_09_header_outside_the_repo_root() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        for (what, include) in [
            ("quoted ../common", "#include \"../common/defs.h\"\n"),
            ("angle via -I outside the root", "#include <defs.h>\n"),
        ] {
            let d = tempfile::tempdir().unwrap();
            let text = owner_text(include, cpp);
            wb(d.path(), &format!("pkg/a.{ext}"), text.as_bytes());
            wb(d.path(), "common/defs.h", PAYLOAD.as_bytes());
            kept_in(
                &mut b,
                &format!("sonnet3-09 {what} (a.{ext})"),
                &d.path().join("pkg"),
                &format!("a.{ext}"),
                &text,
            );
        }
    }
    b.finish();
}

/// Control: the same layout analysed from the common root reads the header.
#[test]
fn row_g2_sonnet3_09_control_analysed_from_the_common_root() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"../common/defs.h\"\n", cpp);
        wb(d.path(), &format!("pkg/a.{ext}"), text.as_bytes());
        wb(d.path(), "common/defs.h", PAYLOAD.as_bytes());
        assert_seen(d.path());
        kept_in(
            &mut b,
            &format!("sonnet3-09 control (a.{ext})"),
            d.path(),
            &format!("pkg/a.{ext}"),
            &text,
        );
    }
    b.finish();
}

// ---- G2-sol3-01 (BOUNDARY): a compiler-provided include mapping ----

/// The source includes `<api/guard.payload>`, which a Clang VFS overlay maps to
/// `private/implementation.data` in the repository (a different path and basename). prism
/// cannot be given the overlay (no loader or CLI input carries VFS overlays, search-path maps or
/// header maps), so the row documents the input prism does receive: an angle include that
/// resolves to nothing in the repository, and an unrelated data file. Compiled with the
/// overlay: `41 2` (C11 and C++17).
#[test]
fn boundary_g2_sol3_01_vfs_mapped_include() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include <api/guard.payload>\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        wb(d.path(), "private/implementation.data", PAYLOAD.as_bytes());
        kept_in(
            &mut b,
            &format!("sol3-01 (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

/// Control (`vfs_direct`): the same physical file included directly is read.
#[test]
fn row_g2_sol3_01_control_direct_include_of_the_same_file() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"private/implementation.data\"\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        wb(d.path(), "private/implementation.data", PAYLOAD.as_bytes());
        assert_seen(d.path());
        kept_in(
            &mut b,
            &format!("sol3-01 control (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

// ---- G2-sol3-02: a backslash include that a decoy directory path answers instead ----

/// POSIX: `#include "dir\defs.payload"` names ONE file whose name holds a backslash; the
/// loader reads `\` as `/` and finds the balanced decoy `dir/defs.payload` instead (compiled
/// `41 2` in both dialects). Capability: the host must hold a file name with a backslash.
#[test]
fn row_g2_sol3_02_backslash_include_with_a_balanced_decoy_directory_path() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"dir\\defs.payload\"\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        if fs::write(d.path().join("dir\\defs.payload"), PAYLOAD).is_err()
            || !d.path().join("dir\\defs.payload").is_file()
        {
            eprintln!(
                "SKIP row_g2_sol3_02: this filesystem cannot hold a backslash in a file name"
            );
            return;
        }
        wb(
            d.path(),
            "dir/defs.payload",
            b"#define OPEN\n#define CLOSE\n",
        );
        kept_in(
            &mut b,
            &format!("sol3-02 decoy (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

/// No-decoy member (control, green on base): the backslash file alone is `Unreadable`, the
/// owner is refused and the pair kept; it must stay kept.
#[test]
fn row_g2_sol3_02_control_backslash_file_without_the_decoy() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"dir\\defs.payload\"\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        if fs::write(d.path().join("dir\\defs.payload"), PAYLOAD).is_err() {
            eprintln!("SKIP row_g2_sol3_02 control: no backslash file names on this filesystem");
            return;
        }
        kept_in(
            &mut b,
            &format!("sol3-02 no decoy (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

// ---- G2-sol3-03 (BOUNDARY): a C++20 header-unit import publishes macros ----

/// `import "defs.payload";` makes the macros of an in-repo file visible (`clang++ -std=c++20
/// -fcxx-modules`, header unit precompiled: prints `41 2`). prism has no `import` of a header
/// unit in its include discovery; the row documents the input prism does receive.
#[test]
fn boundary_g2_sol3_03_cxx20_header_unit_import() {
    let d = tempfile::tempdir().unwrap();
    let text = owner_text("import \"defs.payload\";\n", true);
    wb(d.path(), "a.cc", text.as_bytes());
    wb(d.path(), "defs.payload", PAYLOAD.as_bytes());
    let mut b = Batch::new();
    kept_in(&mut b, "sol3-03", d.path(), "a.cc", &text);
    b.finish();
}

// ---- G2-opus3-11 (BOUNDARY, macOS): the file name and the include differ in normalization ----

/// APFS looks names up ignoring Unicode normalization, so clang finds an NFD-named file from
/// an NFC include (`41 2`). The loader matches strings. Capability: the host must resolve the
/// NFC spelling to the NFD file (else SKIP).
#[test]
fn boundary_g2_opus3_11_include_differs_in_unicode_normalization() {
    let (nfc, nfd) = ("caf\u{e9}.payload", "cafe\u{301}.payload");
    let d = tempfile::tempdir().unwrap();
    let text = owner_text(&format!("#include \"{nfc}\"\n"), false);
    wb(d.path(), "a.c", text.as_bytes());
    wb(d.path(), nfd, PAYLOAD.as_bytes());
    if !d.path().join(nfc).is_file() {
        eprintln!(
            "SKIP boundary_g2_opus3_11: this filesystem does not resolve the NFC spelling to \
             the NFD-named file, so the compiler would not read it either"
        );
        return;
    }
    let mut b = Batch::new();
    kept_in(&mut b, "opus3-11 nfd", d.path(), "a.c", &text);
    b.finish();
}

/// Control (`nfc`): file name and include in the same normalization are read.
#[test]
fn row_g2_opus3_11_control_same_normalization() {
    let nfc = "caf\u{e9}.payload";
    let d = tempfile::tempdir().unwrap();
    let text = owner_text(&format!("#include \"{nfc}\"\n"), false);
    wb(d.path(), "a.c", text.as_bytes());
    wb(d.path(), nfc, PAYLOAD.as_bytes());
    assert_seen(d.path());
    let mut b = Batch::new();
    kept_in(&mut b, "opus3-11 nfc", d.path(), "a.c", &text);
    b.finish();
}

// ---- G2-opus3-12: a literal include of a file under `eval/secbench` (excluded from the census) ----

/// prism excludes `eval/secbench` for resolved include targets and for the C-like net, so the
/// definitions are dropped without `incomplete` (IMMATERIAL outside prism's own tree). Compiled:
/// `41 2`. Required: the pair kept, or the exclusion recorded as a lost input.
#[test]
fn row_g2_opus3_12_include_of_a_file_under_eval_secbench() {
    let mut b = Batch::new();
    for (ext, cpp) in exts() {
        let d = tempfile::tempdir().unwrap();
        let text = owner_text("#include \"eval/secbench/defs.payload\"\n", cpp);
        wb(d.path(), &format!("a.{ext}"), text.as_bytes());
        wb(d.path(), "eval/secbench/defs.payload", PAYLOAD.as_bytes());
        kept_in(
            &mut b,
            &format!("opus3-12 (a.{ext})"),
            d.path(),
            &format!("a.{ext}"),
            &text,
        );
    }
    b.finish();
}

/// Control (`secb2`): the same payload under another directory is read.
#[test]
fn row_g2_opus3_12_control_same_payload_elsewhere() {
    let mut b = Batch::new();
    let d = tempfile::tempdir().unwrap();
    let text = owner_text("#include \"x/defs.payload\"\n", false);
    wb(d.path(), "a.c", text.as_bytes());
    wb(d.path(), "x/defs.payload", PAYLOAD.as_bytes());
    assert_seen(d.path());
    kept_in(&mut b, "opus3-12 control", d.path(), "a.c", &text);
    b.finish();
}

// ---- cache lifecycle: a text input that appears, disappears or changes (cz_parity_in style) ----

fn pair_in_dfg(g: &crate::data_flow::DataFlowGraph, file: &str, from: usize, to: usize) -> bool {
    g.edges().iter().any(|e| {
        e.from.file == file
            && e.from.path.to_string() == "x"
            && e.from.line == from
            && e.to.line == to
    })
}

/// `setup0` builds the first state of the tree in `d`, `setup1` edits it. Saves the first
/// state's cache, then asserts that after the edit (a) a fresh build has the pair iff
/// `kept1` and (b) a cache-backed load never replays the old graph: a `Hit` or a rebuilt
/// `PartialHit` must give the same answer, a `Miss` is a rebuild. The first state's own
/// answer is asserted as the premise (`kept0`).
fn lifecycle(
    what: &str,
    target: &str,
    from: usize,
    to: usize,
    kept0: bool,
    kept1: bool,
    setup0: &dyn Fn(&Path),
    setup1: &dyn Fn(&Path),
) {
    let (d, cache) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    setup0(d.path());
    let repo0 = load_repo(d.path()).unwrap();
    let topology0 = compute_topology_key(&repo0.file_hashes, &repo0.manifest_hashes);
    let ctx0 = CpgContext::build_with_scope_graph_inputs(
        &repo0.files,
        None,
        repo0.scope_graph_inputs.as_ref(),
    );
    assert_eq!(
        pair_in_dfg(&ctx0.cpg.dfg, target, from, to),
        kept0,
        "{what}: the first state must {} x@{from} -> x@{to}",
        if kept0 { "keep" } else { "prune" }
    );
    save_cache_with_topology(
        &ctx0.cpg,
        &repo0.file_hashes,
        &topology0,
        false,
        cache.path(),
    )
    .unwrap();

    setup1(d.path());
    let repo1 = load_repo(d.path()).unwrap();
    let topology1 = compute_topology_key(&repo1.file_hashes, &repo1.manifest_hashes);
    let fresh = CpgContext::build_with_scope_graph_inputs(
        &repo1.files,
        None,
        repo1.scope_graph_inputs.as_ref(),
    );
    assert_eq!(
        pair_in_dfg(&fresh.cpg.dfg, target, from, to),
        kept1,
        "{what}: after the edit a fresh build must {} x@{from} -> x@{to}",
        if kept1 { "keep" } else { "prune" }
    );
    let backed = match load_cache_with_topology(&repo1.file_hashes, &topology1, false, cache.path())
    {
        CacheResult::Hit(hit) => Some(pair_in_dfg(&hit.dfg, target, from, to)),
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            let inc = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &repo1.files,
                None,
            );
            Some(pair_in_dfg(&inc.dfg, target, from, to))
        }
        CacheResult::Miss => None,
    };
    eprintln!(
        "{what}: cache-backed load {}",
        match backed {
            Some(_) => "hit (judged)",
            None => "miss (rebuilt)",
        }
    );
    if let Some(has) = backed {
        assert_eq!(
            has, kept1,
            "{what}: the cache-backed load replayed the old graph (a Hit or a partial hit must \
             agree with the fresh build)"
        );
    }
    // The edit left every parsed file alone: only the text input moved.
    let parsed: BTreeSet<_> = repo0.file_hashes.keys().collect();
    assert_eq!(parsed, repo1.file_hashes.keys().collect::<BTreeSet<_>>());
}

const HAS_INCLUDE_OWNER: &str = "#if __has_include(\"defs.payload\")\n#include \"defs.payload\"\n#else\n#define OPEN\n#define CLOSE\n#endif\n";

fn has_include_owner(cpp: bool) -> String {
    owner_text(HAS_INCLUDE_OWNER, cpp)
}

fn put_owner(d: &Path, ext: &str, cpp: bool) {
    wb(d, &format!("a.{ext}"), has_include_owner(cpp).as_bytes());
}

/// G2-terra3-03, add direction: `defs.payload` appears, `a.c` unchanged. `__has_include`
/// now selects the include; compiled (`-DEXPECT=41`) the program prints 41 for `probe(0)`.
#[test]
fn row_g2_terra3_03_a_newly_resolvable_include_target_appears() {
    for (ext, cpp) in exts() {
        let text = has_include_owner(cpp);
        let (from, to) = guard_pair(&text);
        lifecycle(
            &format!("terra3-03 add (a.{ext})"),
            &format!("a.{ext}"),
            from,
            to,
            false,
            true,
            &|d| put_owner(d, ext, cpp),
            &|d| wb(d, "defs.payload", PAYLOAD.as_bytes()),
        );
    }
}

/// Control (K, green on base): the appearing payload is balanced, so the dead pair is pruned
/// before and after, and the cache-backed load agrees (this exercises the lifecycle plumbing
/// without depending on the gates).
#[test]
fn row_g2_terra3_03_control_a_balanced_include_target_appears() {
    for (ext, cpp) in exts() {
        let text = has_include_owner(cpp);
        let (from, to) = guard_pair(&text);
        lifecycle(
            &format!("terra3-03 control (a.{ext})"),
            &format!("a.{ext}"),
            from,
            to,
            false,
            false,
            &|d| put_owner(d, ext, cpp),
            &|d| {
                wb(
                    d,
                    "defs.payload",
                    b"#define OPEN (void)0\n#define CLOSE (void)0\n",
                )
            },
        );
    }
}

/// Reverse: `defs.payload` is deleted from a newly saved cache of the including state; the
/// fresh build prunes the now-dead pair and the cache-backed load must agree.
#[test]
fn row_g2_terra3_03_a_resolvable_include_target_is_deleted() {
    for (ext, cpp) in exts() {
        let text = has_include_owner(cpp);
        let (from, to) = guard_pair(&text);
        lifecycle(
            &format!("terra3-03 delete (a.{ext})"),
            &format!("a.{ext}"),
            from,
            to,
            true,
            false,
            &|d| {
                put_owner(d, ext, cpp);
                wb(d, "defs.payload", PAYLOAD.as_bytes());
            },
            &|d| fs::remove_file(d.join("defs.payload")).unwrap(),
        );
    }
}

/// Converted from `g_sol_13` (which pinned the aggregate identity, a mechanism): the only
/// edit is to a text input that sits on a rescan split, `{ int a; }` to `{ int a, b; }` (the
/// comma is brace-held). The pair must follow the edit through a cache-backed load, in both
/// directions.
const SPLIT_DEFS: &str =
    "#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n";

fn split_owner(cpp: bool) -> String {
    format!(
        "#include \"defs.payload\"\nint sink(int);\nint probe(int c) {{\n    int x = 41;\n    int b = 0;\n    {{\n        H(INIT);\n        x = 2;\n        J(INIT);\n        return {};\n    }}\n}}\n",
        ret(cpp)
    )
}

fn split_payload(init: &str) -> String {
    format!("#define INIT {init}\n{SPLIT_DEFS}")
}

#[test]
fn row_g2_sol_13_converted_edit_of_a_split_text_input_forward_and_reverse() {
    for (ext, cpp) in exts() {
        let text = split_owner(cpp);
        let (from, to) = guard_pair(&text);
        let target = format!("a.{ext}");
        let (tight, loose) = (split_payload("{ int a; }"), split_payload("{ int a, b; }"));
        let put = |payload: String| {
            let (text, ext) = (text.clone(), ext);
            move |d: &Path| {
                wb(d, &format!("a.{ext}"), text.as_bytes());
                wb(d, "defs.payload", payload.as_bytes());
            }
        };
        let edit = |payload: String| move |d: &Path| wb(d, "defs.payload", payload.as_bytes());
        lifecycle(
            &format!("sol-13 conv forward (a.{ext})"),
            &target,
            from,
            to,
            false,
            true,
            &put(tight.clone()),
            &edit(loose.clone()),
        );
        lifecycle(
            &format!("sol-13 conv reverse (a.{ext})"),
            &target,
            from,
            to,
            true,
            false,
            &put(loose),
            &edit(tight),
        );
    }
}
