//! #630 slice 6b hybrid census inputs: include discovery (every in-repo directory, `..`/`.`
//! normalized, any extension or language), the C-like-extension net, and the permissive scan
//! of census texts. The previous "every file is a header" rule is gone: a file nothing
//! resolves to and with no C-like extension is not read.
use super::macro_census::{macro_call_arguments, CMacroCensus};
use crate::languages::Language;
use crate::repo_loader::c_census::tree_walks;
use crate::repo_loader::{current_c_census_identity, load_repo, LoadedRepo};
use std::fs;
use std::path::Path;

fn write(d: &Path, rel: &str, bytes: &[u8]) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn loaded(d: &Path) -> (CMacroCensus, LoadedRepo) {
    let repo = load_repo(d).unwrap();
    (CMacroCensus::from_files(&repo.files), repo)
}

const OWNER: &[u8] = b"int f(void) { return 0; }\n";
const RESTORE: &[u8] = b"#define RESTORE() x = saved\n";

fn assert_reads_restore(d: &Path, what: &str) {
    let (c, _) = loaded(d);
    assert!(c.get("RESTORE").is_some(), "{what}: RESTORE not read");
    assert!(!c.incomplete(), "{what}: {:?}", c.incomplete_reasons());
}

#[test]
fn f3b_dotdot_and_dot_angle_paths_are_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include <cfg/../macros.payload>\n#include <cfg/./other.payload>\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "cfg/keep.txt", b"x\n");
    write(dir.path(), "macros.payload", RESTORE);
    write(dir.path(), "cfg/other.payload", b"#define OTHER() y\n");
    assert_reads_restore(dir.path(), "dotdot");
    assert!(loaded(dir.path()).0.get("OTHER").is_some());
}

#[test]
fn f3b_msvc_backslash_and_repeated_separators_are_normalized() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include <cfg\\macros.payload>\n#include \"cfg//other.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "inc/cfg/macros.payload", RESTORE);
    write(dir.path(), "cfg/other.payload", b"#define OTHER() y\n");
    assert_reads_restore(dir.path(), "backslash");
    assert!(loaded(dir.path()).0.get("OTHER").is_some());
}

#[test]
fn f3b_a_file_parsed_as_another_language_is_still_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include <macros.py>\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "macros.py", RESTORE);
    let (c, repo) = loaded(dir.path());
    assert_eq!(
        repo.files["macros.py"].language,
        Language::Python,
        "premise"
    );
    assert!(c.get("RESTORE").is_some());
}

#[test]
fn f3b_include_after_a_multiline_comment_is_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"/* leading\n*/ #include <macros.payload>\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "macros.payload", RESTORE);
    assert_reads_restore(dir.path(), "multi-line comment");
}

#[test]
fn f3b_quoted_include_through_an_include_directory_is_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include \"restore.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "include/restore.payload", RESTORE);
    assert_reads_restore(dir.path(), "-I quoted");
}

#[test]
fn f3b_angle_and_quoted_leading_dotdot_are_suffix_matched() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include <../include/restore.payload>\n#include \"../common/x.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "include/restore.payload", RESTORE);
    write(dir.path(), "common/x.payload", b"#define OTHER() y\n");
    let (c, _) = loaded(dir.path());
    assert!(
        c.get("RESTORE").is_some(),
        "angle `..` is include-dir relative"
    );
    assert!(c.get("OTHER").is_some(), "quoted `..` is includer-relative");
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
}

#[test]
fn f3b_transitive_includes_of_text_inputs_are_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include \"m1.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "m1.payload", b"#import \"sub/m2.payload\"\n");
    write(dir.path(), "sub/m2.payload", b"#include_next <m3.tbl>\n");
    write(dir.path(), "z/m3.tbl", RESTORE);
    assert_reads_restore(dir.path(), "transitive");
}

#[test]
fn f3b_unincluded_c_like_extension_files_are_read_the_net() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    write(dir.path(), "x/never.inl", b"#define FROM_INL inl_free\n");
    write(dir.path(), "x/never.X", b"#define FROM_X x_free\n");
    write(dir.path(), "x/never.def", b"#define FROM_DEF def_free\n");
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    for (m, free) in [
        ("FROM_INL", "inl_free"),
        ("FROM_X", "x_free"),
        ("FROM_DEF", "def_free"),
    ] {
        assert!(c.get(m).unwrap().free.contains(free), "{m}");
    }
}

#[test]
fn f3b_net_reaches_ignored_hidden_and_build_directories() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    write(dir.path(), ".gitignore", b"generated/\n");
    write(dir.path(), "generated/g.inc", b"#define GEN_M() gen_free\n");
    write(dir.path(), ".hidden/h.inc", b"#define HID_M() hid_free\n");
    write(dir.path(), "build/b.inl", b"#define BLD_M() bld_free\n");
    write(
        dir.path(),
        "node_modules/n.inc",
        b"#define NODE_M() node_free\n",
    );
    let (c, _) = loaded(dir.path());
    for (m, free) in [
        ("GEN_M", "gen_free"),
        ("HID_M", "hid_free"),
        ("BLD_M", "bld_free"),
        ("NODE_M", "node_free"),
    ] {
        assert!(c.get(m).unwrap().free.contains(free), "{m}");
    }
}

#[test]
fn f3b_git_directory_and_secbench_are_not_walked() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    write(dir.path(), ".git/hooks/h.inc", b"#define IN_GIT 1\n");
    write(dir.path(), "sub/.git/x.inc", b"#define IN_NESTED_GIT 1\n");
    write(
        dir.path(),
        "eval/secbench/s.inc",
        b"#define IN_SECBENCH 1\n",
    );
    let (c, _) = loaded(dir.path());
    for m in ["IN_GIT", "IN_NESTED_GIT", "IN_SECBENCH"] {
        assert!(c.get(m).is_none(), "{m}");
    }
}

#[test]
fn f3b_computed_include_of_a_c_like_header_is_read_by_the_net() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#define H \"restore.h\"\n#include H\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "gen/restore.h", RESTORE);
    assert_reads_restore(dir.path(), "computed .h");
}

#[test]
fn f3b_computed_include_of_an_unresolvable_payload_is_not_read_documented_residual() {
    // The boundary D2 draws: a header with an unusual extension reached only through a form
    // prism cannot resolve acts like an external header (#813).
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#define H \"restore.payload\"\n#include H\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "gen/restore.payload", RESTORE);
    let (c, _) = loaded(dir.path());
    assert!(c.get("RESTORE").is_none(), "not read: the residual");
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
}

#[test]
fn f3b_a_makefile_target_with_comment_openers_does_not_set_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include \"rules.mk\"\n#include <doc.rst>\n#include \"notes.inc\"\nint f(void) { return 0; }\n",
    );
    write(
        dir.path(),
        "rules.mk",
        b"CFLAGS = -DX=\"/*\"\nOUT := $(shell echo '/* start')\nR = R\"zz(\nTAG = caf\xc3\xa9\n",
    );
    write(
        dir.path(),
        "doc.rst",
        b"Options\n=======\n/* unterminated in prose\n``#define`` is shown as `# define`, caf\xc3\xa9 \\u00e9 ??= \"open\n",
    );
    write(
        dir.path(),
        "notes.inc",
        b"/* never closed\n#define IN_UNCLOSED() body_free\n",
    );
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(
        c.get("IN_UNCLOSED").is_some(),
        "a define after an unclosed comment still counts"
    );
}

#[test]
fn f3b_python_text_with_a_non_name_define_is_no_fact() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include <helper.py>\nint f(void) { return 0; }\n",
    );
    write(
        dir.path(),
        "helper.py",
        b"# define 0**0 == NaN\n# define 1abc 2\n#define\n",
    );
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("0").is_none() && c.get("1abc").is_none());
    assert_eq!(c.len(), 0);
}

#[test]
fn f3b_define_inside_a_comment_or_string_of_a_text_counts_and_paste_poisons() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    write(
        dir.path(),
        "t.inc",
        b"/* #define IN_COMMENT() cm_free */\nchar *s = \"#define IN_STRING() st_free\";\n%: define DIGRAPH() dg_free\n??=define TRI() tr_free\nx # define MID() mid_free\n#define PASTE(a,b) a ## b\n#define CTL() goto done\n#undef GONE\n",
    );
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    for (m, free) in [
        ("IN_COMMENT", "cm_free"),
        ("IN_STRING", "st_free"),
        ("DIGRAPH", "dg_free"),
        ("TRI", "tr_free"),
        ("MID", "mid_free"),
    ] {
        assert!(c.get(m).unwrap().free.contains(free), "{m}");
    }
    assert!(c.get("PASTE").unwrap().poisoned);
    assert!(c.get("CTL").unwrap().control);
    assert!(c.get("GONE").is_some());
}

#[test]
fn f3b_utf16_texts_are_decoded() {
    let body = "#define TRACK() track(&x)\n";
    let le: Vec<u8> = body.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = body.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let mut bom_le = vec![0xFF, 0xFE];
    bom_le.extend_from_slice(&le);
    let mut bom_be = vec![0xFE, 0xFF];
    bom_be.extend_from_slice(&be);
    // The BOM-less forms are NUL-interleaved ASCII: the NUL-stripped variant reads them.
    for (name, bytes) in [
        ("le.inc", le),
        ("be.inc", be),
        ("bom_le.inc", bom_le),
        ("bom_be.inc", bom_be),
    ] {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.c", OWNER);
        write(dir.path(), name, &bytes);
        let (c, _) = loaded(dir.path());
        assert!(!c.incomplete(), "{name}");
        assert!(c.get("TRACK").unwrap().free.contains("track"), "{name}");
    }
}

#[test]
fn f3b_unreadable_discovered_target_and_oversize_define_are_the_only_losses() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.c",
            b"#include \"locked.payload\"\nint f(void) { return 0; }\n",
        );
        write(dir.path(), "locked.payload", RESTORE);
        let p = dir.path().join("locked.payload");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&p).is_err() {
            let c = loaded(dir.path()).0;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(c.incomplete());
        }
    }
}

#[test]
fn f3b_files_nothing_resolves_to_are_not_read() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    write(
        dir.path(),
        "Makefile",
        b"#define FROM_MAKEFILE 1\n/* open\n",
    );
    write(dir.path(), "docs/readme.md", b"# define IN_PROSE 1\n");
    write(dir.path(), "img/logo.png", b"\x89PNG \xff define \xff");
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert_eq!(c.len(), 0);
}

#[test]
fn f3b_cache_identity_ignores_unrelated_files_and_tracks_discovered_ones() {
    use crate::cpg_cache::compute_topology_key;
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include \"restore.payload\"\n#define A 1\nint f(void){return A;}\n",
    );
    write(dir.path(), "README.md", b"notes\n");
    write(dir.path(), "x/restore.payload", RESTORE);
    let key = |d: &Path| {
        let r = load_repo(d).unwrap();
        compute_topology_key(&r.file_hashes, &r.manifest_hashes)
    };
    let base = key(dir.path());
    write(
        dir.path(),
        "README.md",
        b"notes, edited, with `#include` words and # define X\n",
    );
    write(dir.path(), "docs/other.txt", b"a new unrelated file\n");
    assert_eq!(
        key(dir.path()),
        base,
        "unrelated edits do not churn the topology"
    );
    write(
        dir.path(),
        "x/restore.payload",
        b"#define RESTORE() y = saved\n",
    );
    let edited = key(dir.path());
    assert_ne!(
        edited, base,
        "editing a define of a discovered target moves it"
    );
    write(dir.path(), "y/other.inl", b"#define NEW_NET 1\n");
    assert_ne!(key(dir.path()), edited, "a new net file moves it");
    fs::remove_file(dir.path().join("y/other.inl")).unwrap();
    assert_eq!(key(dir.path()), edited);
}

#[test]
fn f3b_no_parsed_c_file_means_no_census_walk() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "m.py", b"def f():\n    return 1\n");
    write(dir.path(), "notes.inc", RESTORE);
    let before = tree_walks();
    let repo = load_repo(dir.path()).unwrap();
    let id = current_c_census_identity(&repo.root, &repo.files, &repo.skipped);
    assert_eq!(tree_walks(), before, "no census walk without C/C++");
    assert!(id.is_empty());
    assert!(!repo
        .manifest_hashes
        .keys()
        .any(|k| k.starts_with("c-census:")));
    // Control: a repository with a parsed C file walks once per gather.
    write(dir.path(), "a.c", OWNER);
    let before = tree_walks();
    let repo = load_repo(dir.path()).unwrap();
    assert_eq!(tree_walks(), before + 1);
    current_c_census_identity(&repo.root, &repo.files, &repo.skipped);
    assert_eq!(tree_walks(), before + 2);
}

#[test]
fn f3b_review_gap_sees_an_unread_net_file_but_not_prose() {
    use crate::api::{load_review_inputs, ReviewOptions};
    let diff = "diff --git a/src/a.c b/src/a.c\n--- a/src/a.c\n+++ b/src/a.c\n@@ -1,0 +1,1 @@\n+int f(void) { return 0; }\n";
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "src/a.c", b"int f(void) { return 0; }\n");
    write(dir.path(), "docs/macros.md", b"# define in prose only\n");
    let gap = |d: &Path| {
        load_review_inputs(&ReviewOptions::new(d), diff)
            .unwrap()
            .scope_graph_inputs
            .manifest_hashes
            .contains_key("c-census:review-incomplete")
    };
    assert!(!gap(dir.path()), "prose is no census input");
    write(dir.path(), "docs/macros.inc", b"#define M 1\n");
    assert!(gap(dir.path()), "a define-bearing C-like file is");
}

#[test]
fn f3b_chained_macro_invocation_escapes_every_group() {
    let is_macro = |n: &str| n == "ID" || n == "TRACKED";
    let got = |src: &str| macro_call_arguments(src, &is_macro).unwrap();
    assert!(got("void f(void) { ID(TRACKED)(x); }").contains("x"));
    assert!(got("void f(void) { ID(ID)(TRACKED)(y); }").contains("y"));
    assert!(
        got("void f(void) { ID(A) (z); }").contains("z"),
        "blank between groups"
    );
}

// ---- Confirmation fold: comments in directives, oversize leaders, symlinks. ----

#[test]
fn f3c_comments_inside_directives_of_a_text_input_are_erased() {
    for (i, body) in [
        &b"#/**/define RESTORE() x = saved\n"[..],
        b"#define/**/RESTORE() x = saved\n",
        b"#define /*\n*/ RESTORE() x = saved\n",
        b"#/*\n*/define RESTORE() x = saved\n",
        b"# /* c */ define /* d */ RESTORE() x = saved // tail\n",
    ]
    .iter()
    .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.c",
            b"#include <macros.payload>\nint f(void) { return 0; }\n",
        );
        write(dir.path(), "macros.payload", body);
        assert_reads_restore(dir.path(), &format!("comment form {i}"));
    }
}

#[test]
fn f3c_a_comment_inside_a_forwarding_include_still_discovers_the_target() {
    for (i, fwd) in [
        &b"#include /**/ <macros.payload>\n"[..],
        b"#include/**/<macros.payload>\n",
        b"#/**/include <macros.payload>\n",
        b"#include /*\n*/ <macros.payload>\n",
    ]
    .iter()
    .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.c",
            b"#include \"forward.payload\"\nint f(void) { return 0; }\n",
        );
        write(dir.path(), "forward.payload", fwd);
        write(dir.path(), "inc/macros.payload", RESTORE);
        assert_reads_restore(dir.path(), &format!("forward form {i}"));
    }
}

fn oversize(dir: &Path, rel: &str, head: &[u8]) {
    use std::io::Write;
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut f = fs::File::create(p).unwrap();
    f.write_all(head).unwrap();
    f.set_len(crate::repo_loader::c_census::CENSUS_MAX_BYTES + 1)
        .unwrap();
}

#[test]
fn f3c_oversize_non_c_input_with_any_directive_leader_is_a_loss() {
    for head in [
        &b"#de\\\nfine RESTORE() x = saved\n"[..],
        b"#include <macros.payload>\n",
        b"%:define RESTORE() x\n",
        b"??=define RESTORE() x\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.c", b"int f(void) { return 0; }\n");
        oversize(dir.path(), "huge.inc", head);
        assert!(
            loaded(dir.path()).0.incomplete(),
            "{}",
            String::from_utf8_lossy(head)
        );
    }
}

#[test]
fn f3c_oversize_non_c_input_without_a_leader_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"int f(void) { return 0; }\n");
    oversize(dir.path(), "huge.inc", b"define undef 100% ?? = x\n");
    assert!(!loaded(dir.path()).0.incomplete());
}

#[cfg(unix)]
#[test]
fn f3c_symlinked_c_like_input_pointing_outside_the_repository_is_a_loss() {
    use std::os::unix::fs::symlink;
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("repo");
    write(
        &root,
        "src/a.c",
        b"#include \"restore.inl\"\nint f(void) { return 0; }\n",
    );
    write(outer.path(), "sdk/restore.inl", RESTORE);
    symlink(
        outer.path().join("sdk/restore.inl"),
        root.join("src/restore.inl"),
    )
    .unwrap();
    assert!(loaded(&root).0.incomplete());
}

#[cfg(unix)]
#[test]
fn f3c_symlinked_include_target_inside_the_repository_is_read_through_the_link() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include \"restore.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "gen/macros.txt", RESTORE);
    symlink("../gen/macros.txt", dir.path().join("src/restore.payload")).unwrap();
    assert_reads_restore(dir.path(), "in-repo symlink target");
}

#[cfg(unix)]
#[test]
fn f3c_symlinked_include_target_pointing_outside_is_a_loss() {
    use std::os::unix::fs::symlink;
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("repo");
    write(
        &root,
        "src/a.c",
        b"#include \"restore.payload\"\nint f(void) { return 0; }\n",
    );
    write(outer.path(), "sdk/macros.txt", RESTORE);
    symlink(
        outer.path().join("sdk/macros.txt"),
        root.join("src/restore.payload"),
    )
    .unwrap();
    assert!(loaded(&root).0.incomplete());
}

#[cfg(unix)]
#[test]
fn f3c_symlinked_directory_in_repo_is_an_alias_outside_is_a_loss() {
    use std::os::unix::fs::symlink;
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("repo");
    // `alias -> real` is in-repo: no loss, and an include spelled through the link name
    // resolves to the real file (aliased in the index).
    write(
        &root,
        "src/a.c",
        b"#include \"alias/m.payload\"\nint f(void) { return 0; }\n",
    );
    write(&root, "real/m.payload", b"#define RESTORE() x = saved\n");
    write(outer.path(), "shared/y.txt", b"y\n");
    symlink(root.join("real"), root.join("alias")).unwrap();
    let (c, _) = loaded(&root);
    assert!(
        !c.incomplete(),
        "an in-repo directory link is an alias, not a loss"
    );
    let r = c
        .get("RESTORE")
        .expect("include through the link name reads the real file");
    assert!(r.free.contains("saved"));
    // A link outside the repository is still a loss.
    symlink(outer.path().join("shared"), root.join("include")).unwrap();
    assert!(
        loaded(&root).0.incomplete(),
        "directory outside the repository"
    );
}
