//! #630 slice 6b: the three narrowings of the census's fail-closed `incomplete` triggers
//! (oversize generated tables, the exotic-directive belt, unreadable defines inside
//! comments). Each has an "allowed" test with the cpython shape and a "still incomplete" one.
use super::macro_census::CMacroCensus;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::repo_loader::c_census::CENSUS_MAX_BYTES;
use crate::repo_loader::load_repo;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn census(src: &[(&str, &str)]) -> CMacroCensus {
    let files: BTreeMap<String, ParsedFile> = src
        .iter()
        .map(|(f, s)| {
            let lang = if f.ends_with(".cpp") {
                Language::Cpp
            } else {
                Language::C
            };
            (f.to_string(), ParsedFile::parse(f, s, lang).unwrap())
        })
        .collect();
    CMacroCensus::from_files(&files)
}

fn write(d: &Path, rel: &str, bytes: &[u8]) {
    let p = d.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn loaded(d: &Path) -> (CMacroCensus, crate::repo_loader::LoadedRepo) {
    let repo = load_repo(d).unwrap();
    (CMacroCensus::from_files(&repo.files), repo)
}

/// A generated table over the loader's 2 MiB limit with a `#define` in front.
fn big_header(define: &str) -> Vec<u8> {
    let mut v = format!("{define}\nstatic const unsigned char db[] = {{\n").into_bytes();
    while v.len() < 2 * 1024 * 1024 + 1024 {
        v.extend_from_slice(b"0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,\n");
    }
    v.extend_from_slice(b"};\n");
    v
}

// ---- Narrowing 1: oversize census inputs are read as text. ----

#[test]
fn s6b_oversize_generated_header_is_read_and_does_not_stamp_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Modules/a.c", b"int f(void) { return 0; }\n");
    write(
        dir.path(),
        "Modules/unicodename_db.h",
        &big_header("#define UNIDB_TOP 7"),
    );
    let (c, repo) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("UNIDB_TOP").is_some(), "the oversize define is read");
    let key = "c-census:text:Modules/unicodename_db.h";
    let before = repo
        .manifest_hashes
        .get(key)
        .expect("content hash in the identity")
        .clone();
    // Same size, different content: the identity moves.
    let mut edited = big_header("#define UNIDB_TOP 7");
    let n = edited.len();
    edited[n - 4] = b'x';
    write(dir.path(), "Modules/unicodename_db.h", &edited);
    let (_, again) = loaded(dir.path());
    assert_ne!(again.manifest_hashes.get(key), Some(&before));
}

#[test]
fn s6b_oversize_define_with_a_hazard_still_stamps() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"int f(void) { return 0; }\n");
    write(dir.path(), "big.h", &big_header("#define caf\u{e9} 1"));
    assert!(loaded(dir.path()).0.incomplete());
}

#[test]
fn s6b_oversize_beyond_the_cap_still_stamps_and_joins_the_identity() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"int f(void) { return 0; }\n");
    let f = fs::File::create(dir.path().join("huge.h")).unwrap();
    f.set_len(CENSUS_MAX_BYTES + 1).unwrap();
    let (c, repo) = loaded(dir.path());
    assert!(c.incomplete());
    assert!(repo
        .manifest_hashes
        .contains_key("c-census:text-failed:huge.h"));
}

#[cfg(unix)]
#[test]
fn s6b_oversize_unreadable_file_still_stamps() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", b"int f(void) { return 0; }\n");
    write(dir.path(), "big.h", &big_header("#define A 1"));
    let p = dir.path().join("big.h");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&p).is_ok() {
        return; // running as root: the file stays readable, nothing to observe
    }
    let c = loaded(dir.path()).0;
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(c.incomplete());
}

// ---- Narrowing 2: the exotic belt is for define/undef lines; a UCN needs its hex digits. ----

#[test]
fn s6b_nbsp_and_non_ascii_on_non_define_directives_are_allowed() {
    // Lib/test/clinic.test.c:3764 shape.
    let c = census(&[(
        "clinic.test.c",
        "#define A 1\n#if CONDITION_E ||\u{a0}CONDITION_F\n#endif\n#ifdef caf\u{e9}\n#endif\n#pragma once \u{a0}\n#error caf\u{e9} \\u00E9\n",
    )]);
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("A").is_some());
}

#[test]
fn s6b_bare_backslash_u_in_a_hash_comment_line_is_not_a_ucn() {
    // Lib/idlelib/config-main.def:22 shape: a `#` comment line of a text include.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include \"config-main.def\"\nint f(void) { return 0; }\n",
    );
    write(
        dir.path(),
        "config-main.def",
        b"# C:\\Users\\<username>\n[General]\n",
    );
    let c = loaded(dir.path()).0;
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    // A bare `\U` / `\u` in code position of a define is no UCN either.
    assert!(!census(&[("b.h", "#define P C:\\Users\\x \\u12\n")]).incomplete());
}

#[test]
fn s6b_non_ascii_or_ucn_on_a_define_or_undef_still_stamps() {
    for src in [
        "#define A\u{a0}1\n",
        "#define caf\u{e9} 1\n",
        "#define caf\\u00E9 1\n",
        "#define BIG \\U0001F600x 1\n",
        "#undef caf\u{e9}\n",
        "#undef caf\\u00e9\n",
        "# define caf\u{e9} 1\n",
        "#/*c*/define caf\u{e9} 1\n",
        "%:define caf\u{e9} 1\n",
        // The directive name itself is split by a trigraph-spelled splice.
        "#d??/\nefine X 1\n",
        "??=define X ??/\n1\n",
    ] {
        assert!(
            census(&[("a.h", src)]).incomplete(),
            "must stay incomplete: {src:?}"
        );
    }
}

// ---- Narrowing 3: unreadable defines inside comments. ----

#[test]
fn s6b_define_inside_a_c_comment_is_allowed() {
    // Modules/posixmodule.c:2741/2743: a Python snippet inside a C comment.
    let c = census(&[(
        "posixmodule.c",
        "/*\n#define {s}_DIR_FD_CONVERTER dir_fd_converter\n    code = '''\n#define {s}_OTHER_X 1\n'''\n*/\n#define REAL 1\n// #define {s}_Y\n",
    )]);
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("REAL").is_some());
}

#[test]
fn s6b_unreadable_define_kept_by_one_dialect_still_stamps() {
    // C++ lexes `1'0` as one number and so opens the block comment that hides the define;
    // the C lexer reads `'0; /*` as a character literal and keeps the next line.
    let src = "int a = 1'0; /*\n#define {s}_X 1\n*/ int b;\n";
    assert!(census(&[("a.c", src)]).incomplete());
    // A raw-string desync: strict C strips the comment, the C++ lexer cannot say.
    let desync = "const char *a = R\"(\";\n/*\n#define {s}_X 1\n*/\n";
    assert!(census(&[("a.c", desync)]).incomplete());
    // A real unreadable define in code position.
    assert!(census(&[("a.c", "#define {s}_X 1\n")]).incomplete());
    // An unreadable define under an unterminated comment elsewhere in the file.
    assert!(census(&[(
        "a.c",
        "/*\n#define {s}_X 1\n*/\n#define OK 1\n/* never closed\n"
    )])
    .incomplete());
}
