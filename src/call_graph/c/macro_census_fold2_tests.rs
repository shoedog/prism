//! #630 slice 6b fold 2: delimited and named UCNs, strict UTF-8 census text, angle-include
//! discovery and the include-line belt, and the macro-argument token stream.
use super::macro_census::{macro_call_arguments, CMacroCensus};
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
        .map(|(f, s)| (f.to_string(), ParsedFile::parse(f, s, Language::C).unwrap()))
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

const OWNER: &[u8] = b"int f(void) { return 0; }\n";

// ---- Item 1: delimited and named UCNs. ----

#[test]
fn fold2_delimited_and_named_ucns_on_a_define_or_undef_stamp() {
    for src in [
        "#define TRACK() track(&\\u{e9})\n",
        "#define RESTORE() \\u{00E9} = saved\n",
        "#define BIG() \\U{1F600}\n",
        "#define NAMED() \\N{LATIN SMALL LETTER E WITH ACUTE} = 1\n",
        "#define N\\u{e9} 1\n",
        "#undef \\u{e9}\n",
        // Unclosed: the prefix alone is read as a UCN start.
        "#define U() \\u{e9\n",
    ] {
        assert!(
            census(&[("a.h", src)]).incomplete(),
            "must stay incomplete: {src:?}"
        );
    }
}

#[test]
fn fold2_bare_backslash_u_and_n_stay_allowed() {
    for src in [
        "#define P C:\\Users\\x \\u12\n",
        "#define Q \\N 1\n",
        "#define R \\U 1\n",
        "#define S \\u 1\n",
    ] {
        let c = census(&[("a.h", src)]);
        assert!(!c.incomplete(), "{src:?}: {:?}", c.incomplete_reasons());
    }
}

// ---- Item 2: macro arguments from the token stream. ----

#[test]
fn fold2_macro_call_arguments_are_read_from_tokens() {
    let is_macro = |n: &str| n == "BIND" || n == "M";
    let got = |src: &str| macro_call_arguments(src, &is_macro).unwrap();
    // A type position, a declarator position and a nested list.
    assert!(got("void f(void) { BIND(v) dummy; }").contains("v"));
    assert!(got("void f(void) { long M(x); }").contains("x"));
    assert!(got("void f(void) { BIND(g(a, (b))) c; }")
        .is_superset(&["g", "a", "b"].iter().map(|s| s.to_string()).collect()));
    // Not an invocation: no `(` after the name, a literal, or a comment.
    assert!(got("void f(void) { M; y = 1; }").is_empty());
    assert!(got("void f(void) { s = \"BIND(z)\"; /* M(w) */ }").is_empty());
    // An unclosed list runs to the end.
    assert!(got("BIND(a b").contains("b"));
}

// ---- Item 3: strict UTF-8 census text. ----

#[test]
fn fold2_non_utf8_census_text_is_decoded_not_lost() {
    // Hybrid rule: census texts are read permissively. UTF-16LE with a byte-order mark is
    // decoded as UTF-16 and yields its fact; Shift-JIS is decoded lossily (the trail byte's
    // backslash cannot hide the define: the scan tracks no comment). Neither is incomplete.
    let mut utf16 = vec![0xFF, 0xFE];
    for u in "#define TRACK() track(&x)\n".encode_utf16() {
        utf16.extend_from_slice(&u.to_le_bytes());
    }
    let sjis = b"// \x95\x5c\n#define TRACK() track(&x)\n";
    for (name, bytes) in [("gen.payload", utf16.as_slice()), ("sj.payload", &sjis[..])] {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.c",
            format!("#include \"{name}\"\nint f(void) {{ return 0; }}\n").as_bytes(),
        );
        write(dir.path(), name, bytes);
        let (c, _) = loaded(dir.path());
        assert!(!c.incomplete(), "{name}: {:?}", c.incomplete_reasons());
        assert!(c.get("TRACK").unwrap().free.contains("track"), "{name}");
    }
}

#[test]
fn fold2_non_utf8_c_like_header_is_read_lossily() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.c", OWNER);
    let mut v = b"// \x95\x5c\n#define TRACK() track(&x)\n".to_vec();
    while v.len() < 2 * 1024 * 1024 + 1024 {
        v.extend_from_slice(b"0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,\n");
    }
    write(dir.path(), "gen.h", &v);
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("TRACK").is_some());
    assert!(v.len() as u64 <= CENSUS_MAX_BYTES);
}

#[test]
fn fold2_utf8_bom_census_text_is_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        b"#include \"b.payload\"\nint f(void) { return 0; }\n",
    );
    write(dir.path(), "b.payload", b"\xEF\xBB\xBF#define BOMMED 1\n");
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("BOMMED").is_some());
}

// ---- Item 4: angle includes. ----

#[test]
fn fold2_non_ascii_angle_include_spelling_stamps() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "a.c",
        "#include <macros-\u{e9}.payload>\nint f(void) { return 0; }\n".as_bytes(),
    );
    write(
        dir.path(),
        "macros-\u{e9}.payload",
        b"#define RESTORE() x = saved\n",
    );
    assert!(loaded(dir.path()).0.incomplete());
    // A UCN spelling too.
    assert!(census(&[("b.c", "#include <m\\u00e9.payload>\n")]).incomplete());
    // A non-ASCII byte in a comment after the target, or in a quoted target, is no belt.
    let c = census(&[("c.c", "#include <stdio.h> // caf\u{e9}\n#include \"x.h\"\n")]);
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
}

#[test]
fn fold2_an_angle_target_is_matched_against_every_directory() {
    // Hybrid rule: the target resolves against every in-repo directory (path equal or
    // ending in `/<target>`); a file that merely shares a basename is not read.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "src/a.c",
        b"#include <cfg/macros.payload>\n#include <stdio.h>\nint f(void) { return 0; }\n",
    );
    write(
        dir.path(),
        "include/cfg/macros.payload",
        b"#define RESTORE() x = saved\n",
    );
    write(
        dir.path(),
        "other/lib/cfg/macros.payload",
        b"#define ALSO_READ 1\n",
    );
    write(
        dir.path(),
        "include/xcfg/macros.payload",
        b"#define NOT_READ 1\n",
    );
    let (c, _) = loaded(dir.path());
    assert!(!c.incomplete(), "{:?}", c.incomplete_reasons());
    assert!(c.get("RESTORE").is_some());
    assert!(c.get("ALSO_READ").is_some(), "every directory is searched");
    assert!(c.get("NOT_READ").is_none(), "basename alone is no match");
}
