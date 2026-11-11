//! Fold 3 of #630 slice 6a: round-3 reviewer inputs (sol 1-6, opus F1-F5, Q2).
use super::macro_census::CMacroCensus;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::repo_loader::load_repo;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn files(src: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    src.iter()
        .map(|(f, s)| {
            let lang = if f.ends_with(".cpp") || f.ends_with(".hpp") {
                Language::Cpp
            } else {
                Language::C
            };
            (f.to_string(), ParsedFile::parse(f, s, lang).unwrap())
        })
        .collect()
}

fn census(src: &[(&str, &str)]) -> CMacroCensus {
    CMacroCensus::from_files(&files(src))
}

fn set(v: &[&str]) -> BTreeSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn has(c: &CMacroCensus, name: &str, ident: &str) {
    let m = c.get(name).unwrap_or_else(|| panic!("{name} missing"));
    assert!(m.free.contains(ident), "{name}.free = {:?}", m.free);
}

/// Strict-C view: `R` expands to nothing and `R"(` is no raw string, while the C++ lexer
/// opens a raw string at it and desyncs.
const DESYNC: &str = "#define R\nconst char *a = R\"(\";\n#define Q(x) (x)\"/*\"\n";

#[test]
fn f3_sol1_comment_in_directive_prefix_in_every_pass() {
    for (i, prefix) in ["#/**/define", "# /* c */ define", "%:/**/define"]
        .iter()
        .enumerate()
    {
        let src = format!("{DESYNC}{prefix} M{i} hidden\nconst char *b = \"*/\";\n");
        let c = census(&[("a.h", &src)]);
        has(&c, &format!("M{i}"), "hidden");
    }
    // A candidate that looks like a define but names no macro fails closed, in any pass.
    let src = format!("{DESYNC}#/**/define\nconst char *b = \"*/\";\n");
    assert!(census(&[("a.h", &src)]).incomplete());
}

#[test]
fn f3_sol2_soft_poison_is_per_source_definition() {
    let src = "#define R\n#define M 0\n#undef M\nconst char *a = R\"(\";\n#define Q(x) (x)\"/*\"\n#define M /* multi\n*/ hidden\nconst char *b = \"*/\";\n";
    let c = census(&[("a.h", src)]);
    let m = c.get("M").unwrap();
    assert!(
        m.free.contains("hidden") || m.poisoned || c.incomplete(),
        "M lost its second definition silently: {m:?}"
    );
    // A clean multi-line-comment body is still not poisoned (recovered by the stripped pass).
    let c = census(&[("a.h", "#define FOUR /* multi\nline */ w\n")]);
    assert!(!c.get("FOUR").unwrap().poisoned && !c.incomplete());
}

#[test]
fn f3_exotic_directive_class_fails_closed() {
    // Non-ASCII whitespace or a UCN or non-ASCII byte or a trigraph inside a directive line.
    for src in [
        "#define\u{a0}M hidden\n",
        "#define M\u{a0}hidden\n",
        "#define caf\u{e9} hidden\n#define NEXT caf\\u00E9\n",
        "#define READ \u{e9}\n",
        "#define M a ??( b\n",
        "#define M 1 // caf\u{e9}\n",
        "??=define M hidden\n",
    ] {
        assert!(census(&[("a.h", src)]).incomplete(), "{src:?}");
    }
    // Lines that are not directives are not affected.
    let c = census(&[(
        "a.h",
        "const char *s = \"caf\\u00e9 ??( \u{e9}\";\n// \u{e9}\n#define A 1\n",
    )]);
    assert!(!c.incomplete());
    // Another directive (not a define) with a plain name is clean too.
    assert!(!census(&[("a.h", "#include \"x.h\"\n#define A 1\n")]).incomplete());
}

#[test]
fn f3_opus_f4_definition_behind_a_comment_after_a_dialect_desync() {
    let src = "#if 0\nrev 2'b/*'\n#endif\n/* live */ #define NEXT y\n";
    let c = census(&[("a.h", src)]);
    has(&c, "NEXT", "y");
}

#[test]
fn f3_raw_string_dialect_mismatch_keeps_the_definition() {
    // C view and C++ view disagree on where the literal ends.
    let src = "#define R\nconst char *a = R\"x(\";\n#define Q \"/*\"\n#define M hidden\nconst char *b = \")x\" \"*/\";\n";
    has(&census(&[("a.h", src)]), "M", "hidden");
}

#[test]
fn f3_opus_f5_control_substrings() {
    for (i, body) in [
        "if (__mingw_setjmp(b) == 0)",
        "_setjmp3(b)",
        "__intrinsic_setjmpex(b)",
        "wrap_vfork_now()",
        "xgetcontext",
        "pre_savectx",
    ]
    .iter()
    .enumerate()
    {
        let c = census(&[("a.h", &format!("#define J{i} {body}\n"))]);
        assert!(c.get(&format!("J{i}")).unwrap().control, "{body}");
    }
}

// ---- Loader: skipped files, text-only includes, cache identity. ----

fn write(d: &Path, rel: &str, bytes: &[u8]) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn loaded_census(d: &Path) -> CMacroCensus {
    CMacroCensus::from_files(&load_repo(d).unwrap().files)
}

#[test]
fn f3_cache_full_hit_tracks_the_skipped_c_file_aggregate() {
    use crate::cpg_cache::{
        compute_topology_key, load_cache_with_topology, save_cache_with_topology, CacheResult,
    };
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"#define A 1\nint f(void){return A;}\n");
    let save = |dir: &Path| {
        let repo = load_repo(dir).unwrap();
        let topology = compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
        let ctx = crate::cpg::CpgContext::build_with_scope_graph_inputs(
            &repo.files,
            None,
            repo.scope_graph_inputs.as_ref(),
        );
        save_cache_with_topology(&ctx.cpg, &repo.file_hashes, &topology, false, cache.path())
            .unwrap();
        (repo.file_hashes.clone(), topology)
    };
    let probe = |dir: &Path| {
        let repo = load_repo(dir).unwrap();
        let topology = compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
        let miss = matches!(
            load_cache_with_topology(&repo.file_hashes, &topology, false, cache.path()),
            CacheResult::Miss
        );
        (repo.file_hashes, miss)
    };
    let (clean_hashes, _) = save(dir.path());
    assert!(!probe(dir.path()).1, "the unchanged repository hits");
    // A non-UTF-8 header appears; every parsed file is unchanged.
    write(dir.path(), "legacy.h", b"/* \xA9 */\n#define NEXT x\n");
    let (hashes, miss) = probe(dir.path());
    assert_eq!(hashes, clean_hashes, "parsed files unchanged");
    assert!(miss, "a new skipped C file must not replay the old census");
    assert!(loaded_census(dir.path()).incomplete());
    save(dir.path());
    // The last skipped file disappears.
    fs::remove_file(dir.path().join("legacy.h")).unwrap();
    let (hashes, miss) = probe(dir.path());
    assert_eq!(hashes, clean_hashes);
    assert!(
        miss,
        "removing the last skipped C file must not replay either"
    );
    assert!(!loaded_census(dir.path()).incomplete());
}

#[test]
fn f3_text_only_includes_join_the_census_and_the_cache_identity() {
    use crate::cpg_cache::{
        compute_topology_key, load_cache_with_topology, save_cache_with_topology, CacheResult,
    };
    let dir = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include \"m.inl\"\n#include \"gen/data.tbl\"\nint f(void){return TOP;}\n",
    );
    write(dir.path(), "src/m.inl", b"#define TOP (stack[sp])\n");
    write(
        dir.path(),
        "src/gen/data.tbl",
        b"#define ROW(a) (a + tbl_off)\n",
    );
    write(dir.path(), "x/vm.inc", b"#define FROM_INC inc_var\n");
    write(dir.path(), "x/Big.H", b"#define FROM_UPPER up_var\n");
    write(dir.path(), "notes.txt", b"#define NOT_SEEN 1\n");
    let c = loaded_census(dir.path());
    has(&c, "TOP", "stack");
    has(&c, "ROW", "tbl_off");
    has(&c, "FROM_INC", "inc_var");
    has(&c, "FROM_UPPER", "up_var");
    assert!(c.get("NOT_SEEN").is_none());
    assert!(!c.incomplete());
    // The loader's set is deterministic.
    assert_eq!(c.digest(), loaded_census(dir.path()).digest());
    // Editing a text-only file misses a saved cache whose parsed files are unchanged.
    let repo = load_repo(dir.path()).unwrap();
    let topology = compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
    let ctx = crate::cpg::CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        None,
        repo.scope_graph_inputs.as_ref(),
    );
    save_cache_with_topology(&ctx.cpg, &repo.file_hashes, &topology, false, cache.path()).unwrap();
    assert!(matches!(
        load_cache_with_topology(&repo.file_hashes, &topology, false, cache.path()),
        CacheResult::Hit(_)
    ));
    write(
        dir.path(),
        "src/m.inl",
        b"#define TOP (stack[sp + hidden])\n",
    );
    let edited = load_repo(dir.path()).unwrap();
    assert_eq!(edited.file_hashes, repo.file_hashes);
    let topology2 = compute_topology_key(&edited.file_hashes, &edited.manifest_hashes);
    assert_ne!(topology, topology2);
    assert!(matches!(
        load_cache_with_topology(&edited.file_hashes, &topology2, false, cache.path()),
        CacheResult::Miss
    ));
    has(&CMacroCensus::from_files(&edited.files), "TOP", "hidden");
}

#[test]
fn f3_unreadable_directory_fails_closed_but_exclusions_do_not() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"#define A 1\n");
    write(dir.path(), "vendor/v.h", b"#define V 1\n");
    write(dir.path(), ".hidden/h.h", b"#define H 1\n");
    assert!(
        !loaded_census(dir.path()).incomplete(),
        "ignored and hidden directories are explicit exclusions"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        write(dir.path(), "include/m.h", b"#define M hidden\n");
        let sub = dir.path().join("include");
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o000)).unwrap();
        // Running as root, permissions may not deny: the case cannot be exercised then.
        if fs::read_dir(&sub).is_ok() {
            eprintln!("skipped: directory permissions do not deny (running as root)");
        } else {
            let incomplete = loaded_census(dir.path()).incomplete();
            fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(incomplete, "an unreadable directory loses census input");
        }
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn f3_review_mode_fails_closed_when_the_repository_has_unread_inputs() {
    use crate::api::{load_review_inputs, ReviewOptions};
    let diff = "diff --git a/src/a.c b/src/a.c\n--- a/src/a.c\n+++ b/src/a.c\n@@ -1,0 +1,4 @@\n+#include \"defs.h\"\n+#include \"table.tbl\"\n+int f(void) { int x = 1; int y = NEXT; x = 2; return x + y; }\n+\n";
    let review = |setup: &dyn Fn(&Path)| {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "src/a.c",
            b"#include \"defs.h\"\n#include \"table.tbl\"\nint f(void) { int x = 1; int y = NEXT; x = 2; return x + y; }\n\n",
        );
        setup(dir.path());
        let inputs = load_review_inputs(&ReviewOptions::new(dir.path()), diff).unwrap();
        (
            CMacroCensus::from_files(&inputs.files).incomplete(),
            inputs
                .scope_graph_inputs
                .manifest_hashes
                .contains_key("c-census:review-incomplete"),
        )
    };
    // A header outside the diff holds the macro: unread, so incomplete.
    assert_eq!(
        review(&|d| write(d, "src/defs.h", b"#define NEXT (x)\n")),
        (true, true)
    );
    // An `.inc` outside the diff, and a quoted include of an odd extension.
    assert!(review(&|d| write(d, "src/vm.inc", b"#define Z 1\n")).0);
    assert!(review(&|d| write(d, "src/table.tbl", b"#define Z 1\n")).0);
    // Nothing else in the repository: the diff's files are the repository's.
    assert_eq!(review(&|_| {}), (false, false));
}
