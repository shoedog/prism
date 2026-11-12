//! Unit tests of the C/C++ macro census (#630 slice 6a).
use super::macro_census::CMacroCensus;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

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

#[test]
fn parameters_and_free_identifiers() {
    let c = census(&[(
        "a.h",
        "#define ADD(a, b) ((a) + (b) + offset)\n#define NEXT (x + 1)\n#define VA(fmt, ...) log(fmt, __VA_ARGS__, tag)\n",
    )]);
    let add = c.get("ADD").unwrap();
    assert_eq!(add.params, set(&["a", "b"]));
    assert_eq!(add.free, set(&["offset"]));
    assert!(!add.control && !add.poisoned);
    let next = c.get("NEXT").unwrap();
    assert!(next.params.is_empty());
    assert_eq!(next.free, set(&["x"]));
    let va = c.get("VA").unwrap();
    assert_eq!(va.params, set(&["__VA_ARGS__", "fmt"]));
    assert_eq!(va.free, set(&["log", "tag"]));
    assert!(c.get("offset").is_none());
}

#[test]
fn literals_comments_and_numbers_carry_no_identifier() {
    let c = census(&[(
        "a.h",
        "#define K \"hidden\" 'c' 0x1fUL 1e+5 /* ghost */ real // tail\n#define L 1 /* note */\n#define M(a) a /* c */ + b\n",
    )]);
    assert_eq!(c.get("K").unwrap().free, set(&["real"]));
    assert!(!c.get("K").unwrap().poisoned);
    assert!(c.get("L").unwrap().free.is_empty());
    assert_eq!(c.get("M").unwrap().free, set(&["b"]));
}

#[test]
fn transitive_closure_over_in_repo_macros() {
    let c = census(&[
        ("a.h", "#define A B + a_own\n#define B C(1)\n"),
        ("b.h", "#define C(n) (n + deep)\n"),
    ]);
    assert_eq!(c.get("A").unwrap().free, set(&["B", "C", "a_own", "deep"]));
    assert_eq!(c.get("B").unwrap().free, set(&["C", "deep"]));
    assert_eq!(c.get("C").unwrap().free, set(&["deep"]));
}

#[test]
fn control_flag_is_transitive() {
    let c = census(&[(
        "a.h",
        "#define JUMP goto out\n#define LC_SET case __LINE__\n#define WRAP JUMP\n#define SJ setjmp(env)\n#define ASM __asm__(\"nop\")\n#define PLAIN 1\n",
    )]);
    for name in ["JUMP", "LC_SET", "WRAP", "SJ", "ASM"] {
        assert!(c.get(name).unwrap().control, "{name}");
    }
    assert!(!c.get("PLAIN").unwrap().control);
    assert_eq!(c.control_count(), 5);
}

#[test]
fn poisoned_by_paste_unterminated_and_propagation() {
    let c = census(&[(
        "a.h",
        "#define CAT(a, b) a ## b\n#define BAD \"unterminated\n#define USES CAT(x, y)\n#define OPEN /* never closed\n",
    )]);
    assert!(c.get("CAT").unwrap().poisoned);
    assert!(
        c.get("USES").unwrap().poisoned,
        "poison propagates to callers"
    );
    // A malformed body is present and poisoned, never silently absent.
    assert!(c.get("BAD").unwrap().poisoned);
    // An unterminated comment is not a macro fact: the census fails closed.
    assert!(c.incomplete());
}

#[test]
fn recursion_is_poisoned() {
    let c = census(&[(
        "a.h",
        "#define SELF SELF + 1\n#define PING PONG\n#define PONG PING\n#define TOP PING\n#define LEAF 1\n",
    )]);
    for name in ["SELF", "PING", "PONG", "TOP"] {
        assert!(c.get(name).unwrap().poisoned, "{name}");
    }
    assert!(!c.get("LEAF").unwrap().poisoned);
    assert_eq!(c.poisoned_count(), 4);
}

#[test]
fn multiple_definitions_are_unioned() {
    let c = census(&[
        ("a.h", "#define M(a) (a + one)\n"),
        ("b.h", "#define M(b, c) (b + two)\n#define M goto L\n"),
    ]);
    let m = c.get("M").unwrap();
    assert_eq!(m.params, set(&["a", "b", "c"]));
    assert!(m.free.is_superset(&set(&["one", "two", "L"])));
    assert!(m.control);
    // A definition under `#if` counts as well.
    let c = census(&[(
        "a.h",
        "#ifdef X\n#define Q read_x\n#else\n#define Q 0\n#endif\n",
    )]);
    assert_eq!(c.get("Q").unwrap().free, set(&["read_x"]));
}

#[test]
fn only_c_and_cpp_files_count() {
    let mut f = files(&[("a.h", "#define A 1\n")]);
    f.insert(
        "x.py".into(),
        ParsedFile::parse("x.py", "# #define NOPE 1\nx = 1\n", Language::Python).unwrap(),
    );
    let c = CMacroCensus::from_files(&f);
    assert_eq!(c.len(), 1);
    assert!(CMacroCensus::from_files(&BTreeMap::new()).is_empty());
}

#[test]
fn digest_is_stable_and_tracks_every_fact() {
    let base = [("a.h", "#define A(p) (p + x)\n#define B A(1)\n")];
    let d = census(&base).digest();
    assert_eq!(d.len(), 64);
    assert_eq!(d, census(&base).digest(), "stable across builds");
    // File insertion order and extra blank lines do not matter.
    assert_eq!(
        d,
        census(&[("a.h", "\n\n#define B A(1)\n#define A(p) (p + x)\n")]).digest()
    );
    assert!(CMacroCensus::default().digest().is_empty());
    for edited in [
        "#define A(q) (q + x)\n#define B A(1)\n",        // parameter
        "#define A(p) (p + y)\n#define B A(1)\n",        // free identifier
        "#define A(p) (p + x)\n#define B A(1) goto L\n", // control
        "#define A(p) (p ## x)\n#define B A(1)\n",       // poisoned
        "#define A(p) (p + x)\n#define B A(1)\n#define C 1\n", // new macro
    ] {
        assert_ne!(d, census(&[("a.h", edited)]).digest(), "{edited}");
    }
    // A change that is not a fact (a numeric literal in the body) keeps the digest.
    assert_eq!(
        census(&[("a.h", "#define A 1\n")]).digest(),
        census(&[("a.h", "#define A 2\n")]).digest()
    );
}

fn cpp(src: &str) -> CMacroCensus {
    census(&[("a.hpp", src)])
}

#[test]
fn continuation_splits_identifiers_and_tokens() {
    let c = census(&[(
        "a.h",
        "#define NEXT hid\\\nden\n#define JUMP go\\\nto out\n#define CAT(a, b) a #\\\n# b\n#define CRLF fo\\\r\no\r\n",
    )]);
    assert_eq!(c.get("NEXT").unwrap().free, set(&["hidden"]));
    let j = c.get("JUMP").unwrap();
    assert!(j.control, "split goto");
    assert_eq!(j.free, set(&["goto", "out"]));
    assert!(c.get("CAT").unwrap().poisoned, "split ##");
    assert_eq!(c.get("CRLF").unwrap().free, set(&["foo"]));
    assert!(!c.incomplete());
}

#[test]
fn comment_before_name_and_between_hash_and_define() {
    let c = census(&[(
        "a.h",
        "#define /* c */ NEXT x\n#define/**/ TWO(a) (a + y)\n# /* c */ define THREE z\n#define FOUR /* multi\nline */ w\n",
    )]);
    assert_eq!(c.get("NEXT").unwrap().free, set(&["x"]));
    assert_eq!(c.get("TWO").unwrap().free, set(&["y"]));
    assert_eq!(c.get("THREE").unwrap().free, set(&["z"]));
    assert_eq!(c.get("FOUR").unwrap().free, set(&["w"]));
    assert!(!c.incomplete());
}

#[test]
fn raw_strings_do_not_hide_tokens() {
    let c = cpp(
        "#define M R\"(a\"b)\"; sink(x); R\"(c\"d)\"\n#define G R\"(a\"b)\"; goto out; R\"(c\"d)\"\n#define P u8R\"xy(\")xy\" + secret\n#define Q LR\"(q)\" + after\n",
    );
    // The C-dialect pass (no raw strings) is unioned in, so each free set may hold more.
    has(&c, "M", "sink");
    has(&c, "M", "x");
    assert!(c.get("G").unwrap().control);
    has(&c, "P", "secret");
    has(&c, "Q", "after");
    // An unterminated raw string fails closed.
    assert!(cpp("#define U R\"(open\n").get("U").unwrap().poisoned);
}

#[test]
fn digraph_paste_is_poisoned_directly_and_transitively() {
    let c = census(&[(
        "a.h",
        "#define CAT(a, b) a %:%: b\n#define WRAP CAT(x, y)\n#define STR(a) %:a\n",
    )]);
    assert!(c.get("CAT").unwrap().poisoned);
    assert!(c.get("WRAP").unwrap().poisoned);
    let s = c.get("STR").unwrap();
    assert!(!s.poisoned, "%: alone is stringizing");
    assert!(s.free.is_empty());
}

#[test]
fn digit_separators_are_one_number() {
    let c = cpp("#define SCALED (1'000 * scale + 2'000)\n#define HEX 0xFF'FFu + off\n");
    assert_eq!(c.get("SCALED").unwrap().free, set(&["scale"]));
    assert!(!c.get("SCALED").unwrap().poisoned);
    assert_eq!(c.get("HEX").unwrap().free, set(&["off"]));
}

#[test]
fn defines_the_grammar_does_not_expose_are_found() {
    let c = census(&[(
        "a.h",
        "enum {\n#define X(n) C_##n,\n#include \"x.def\"\n#undef X\n};\nint t[] = {\n#define Y hiddenvar\n1};\n%:define Z zed\nvoid f( {\n#define W wee\n",
    )]);
    assert!(c.get("X").unwrap().poisoned);
    assert_eq!(c.get("Y").unwrap().free, set(&["hiddenvar"]));
    assert_eq!(c.get("Z").unwrap().free, set(&["zed"]));
    assert_eq!(c.get("W").unwrap().free, set(&["wee"]));
    assert!(!c.incomplete());
}

#[test]
fn unparseable_define_sets_incomplete_and_the_digest() {
    for src in ["#define\n", "#define (x) 1\n", "#define 1abc x\n"] {
        let c = census(&[("a.h", src)]);
        assert!(c.incomplete(), "{src}");
        assert!(!c.digest().is_empty(), "digest tracks incomplete: {src}");
    }
    let c = census(&[("a.h", "#define F(a, b 1\n")]);
    assert!(c.incomplete());
    assert!(c.get("F").unwrap().poisoned);
    // Repo-wide: one bad file taints the whole census, and the digest differs.
    let clean = census(&[("a.h", "#define A 1\n")]);
    let tainted = census(&[("a.h", "#define A 1\n"), ("b.h", "#define\n")]);
    assert!(!clean.incomplete() && tainted.incomplete());
    assert_ne!(clean.digest(), tainted.digest());
}

// ---- Fold 2: round-2 reviewer inputs (sol 1-9, opus R2-W1..W7). ----

fn has(c: &CMacroCensus, name: &str, ident: &str) {
    let m = c.get(name).unwrap_or_else(|| panic!("{name} missing"));
    assert!(m.free.contains(ident), "{name}.free = {:?}", m.free);
}

#[test]
fn r2_sol1_raw_string_with_quote_delimiter() {
    let c = cpp("const char *s = R\"\"(/*)\"\";\n#define M hidden\n/* */\n");
    has(&c, "M", "hidden");
}

#[test]
fn r2_sol2_and_opus_w6_splice_inside_raw_string() {
    for src in [
        "const char *s = R\"(a)\\\n\"/*)\";\n#define M hidden\n/* */\n",
        "const char *s = R\"(x)\\\n\" /* )\";\n#define M hidden\n/* */\n",
    ] {
        let c = cpp(src);
        has(&c, "M", "hidden");
        assert!(c.incomplete(), "uncertain raw scan fails closed: {src}");
    }
}

#[test]
fn r2_opus_w6_comment_openers_the_compiler_does_not_open() {
    // R"( in strict C.
    let c = census(&[(
        "a.h",
        "const char *a = R\"(\";\n#define Q(x) (x)\"/*\"\n#define M hidden\nconst char *b = \"*/\";\n",
    )]);
    has(&c, "M", "hidden");
    // Pre-C23 multi-character constant.
    let c = census(&[("a.h", "x = 1'a/*';\n#define M hidden\n/* */\n")]);
    has(&c, "M", "hidden");
    // MSVC does not splice a backslash followed by a blank.
    let c = census(&[("a.h", "// note \\ \n#define M hidden\n")]);
    has(&c, "M", "hidden");
}

#[test]
fn r2_sol3_trigraphs() {
    let c = census(&[(
        "a.h",
        "??=define M hidden\n#define NEXT hid??/\nden\n#define CAT(a,b) a ??=??= b\n",
    )]);
    has(&c, "M", "hidden");
    has(&c, "NEXT", "hidden");
    assert!(c.get("CAT").unwrap().poisoned);
}

#[test]
fn r2_cr_only_and_mixed_line_endings() {
    let c = census(&[("a.h", "// header\r#define M hidden\r")]);
    has(&c, "M", "hidden");
    let c = census(&[("a.h", "#define A 1\r#define NEXT x\r")]);
    assert!(c.get("A").unwrap().free.is_empty());
    has(&c, "NEXT", "x");
    let c = census(&[("a.h", "#define M hid\\\rden\r")]);
    has(&c, "M", "hidden");
}

#[test]
fn r2_bom_hides_nothing() {
    let c = census(&[("a.h", "\u{feff}#define M hidden\n")]);
    has(&c, "M", "hidden");
}

#[test]
fn r2_ucn_and_non_ascii_identifiers() {
    let c = census(&[("a.h", "#define READ \\u00E9\n#define caf\\u00E9 hidden\n")]);
    has(&c, "READ", "\\u00E9");
    has(&c, "READ", "u00E9");
    let m = c.get("caf\\u00E9").expect("whole UCN name recorded");
    assert!(m.poisoned);
    assert!(c.get("caf").is_none());
    // Literal non-ASCII names and combining marks.
    let c = census(&[(
        "a.h",
        "#define caf\u{e9} x\n#define a\u{b7}b y\n#define NEXT caf\u{e9}\n",
    )]);
    assert!(c.get("caf\u{e9}").unwrap().poisoned);
    assert!(c.get("a\u{b7}b").unwrap().poisoned);
    has(&c, "NEXT", "caf");
    assert!(c.get("NEXT").unwrap().poisoned, "reaches a poisoned macro");
    // NBSP glued to an identifier still records the ASCII fragment.
    let c = census(&[("a.h", "#define M(a) a =\u{a0}secret\n")]);
    has(&c, "M", "secret");
    // A UCN or non-ASCII parameter poisons the macro.
    let c = census(&[("a.h", "#define F(\u{e9}) \u{e9} + z\n")]);
    assert!(c.get("F").unwrap().poisoned);
    has(&c, "F", "z");
}

#[test]
fn r2_splice_with_any_blank_before_newline() {
    for blank in ["\x0c", "\x0b", " ", "\t", " \t"] {
        let c = census(&[("a.h", &format!("#define M hid\\{blank}\nden\n"))]);
        has(&c, "M", "hidden");
    }
}

#[test]
fn r2_control_spellings() {
    for (i, body) in [
        "_asm { jmp target }",
        "if (_setjmpex(b) == 0)",
        "__vfork()",
        "__setjmp(e)",
        "_sigsetjmp(e, 1)",
        "qsetjmp(e)",
    ]
    .iter()
    .enumerate()
    {
        let c = census(&[("a.h", &format!("#define J{i} {body}\n"))]);
        assert!(c.get(&format!("J{i}")).unwrap().control, "{body}");
    }
    // Fold 3: any identifier holding a returns-twice stem is control.
    for (i, body) in [
        "__mingw_setjmp(b)",
        "_setjmp3(b)",
        "setjmpish",
        "my_getcontext_wrap()",
    ]
    .iter()
    .enumerate()
    {
        let c = census(&[("a.h", &format!("#define K{i} {body}\n"))]);
        assert!(c.get(&format!("K{i}")).unwrap().control, "{body}");
    }
    assert!(
        !census(&[("a.h", "#define P jmp_buf_size\n")])
            .get("P")
            .unwrap()
            .control
    );
}

#[test]
fn r2_sol9_malformed_parameters_cannot_collide() {
    let hostile = "#define A(\x01fhidden\x00\x01\x00\x00\x02mB\x00py) 1\n";
    let clean = "#define A hidden\n#define B(y) 1\n";
    let h = census(&[("a.h", hostile)]);
    let k = census(&[("a.h", clean)]);
    assert!(h.incomplete(), "an invalid parameter fails closed");
    assert!(h.get("A").unwrap().poisoned);
    assert_ne!(h.digest(), k.digest());
    // Empty parameters between commas and stray text are invalid too.
    for src in [
        "#define F(a,,b) 1\n",
        "#define F(a b) 1\n",
        "#define F(1a) 1\n",
    ] {
        let c = census(&[("a.h", src)]);
        assert!(c.incomplete() && c.get("F").unwrap().poisoned, "{src}");
    }
    // Valid forms stay clean.
    let c = census(&[(
        "a.h",
        "#define F() 1\n#define G(a, ...) a\n#define H(args...) args\n",
    )]);
    assert!(!c.incomplete());
}

#[test]
fn r2_union_pass_does_not_taint_clean_macros() {
    // A multi-line comment in a body must not poison or set incomplete via the unstripped pass.
    let c = census(&[("a.h", "#define FOUR /* multi\nline */ w\n")]);
    assert!(!c.get("FOUR").unwrap().poisoned && !c.incomplete());
}

#[test]
fn r2_skipped_c_family_file_fails_closed() {
    let mut f = files(&[("a.h", "#define A 1\n")]);
    assert!(!CMacroCensus::from_files(&f).incomplete());
    for p in f.values_mut() {
        p.mark_c_family_skipped_in_repo();
    }
    let skipped = CMacroCensus::from_files(&f);
    assert!(skipped.incomplete());
    assert_ne!(
        skipped.digest(),
        census(&[("a.h", "#define A 1\n")]).digest()
    );
}

#[test]
fn r2_loader_stamps_every_skip_reason_that_loses_a_c_file() {
    use crate::repo_loader::load_repo;
    let census_of = |setup: &dyn Fn(&std::path::Path)| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ok.h"), "#define A 1\n").unwrap();
        setup(dir.path());
        let repo = load_repo(dir.path()).unwrap();
        CMacroCensus::from_files(&repo.files).incomplete()
    };
    assert!(!census_of(&|_| {}));
    // Latin-1 copyright sign (NotUtf8): the net reads it permissively, so it is no loss.
    assert!(!census_of(&|d| std::fs::write(
        d.join("legacy.h"),
        b"/* \xA9 */\n#define NEXT x\n"
    )
    .unwrap()));
    // A non-C file that is skipped does not matter.
    assert!(!census_of(&|d| std::fs::write(
        d.join("data.bin.txt"),
        b"\xff\xfe"
    )
    .unwrap()));
    #[cfg(unix)]
    assert!(census_of(&|d| std::os::unix::fs::symlink(
        d.join("ok.h"),
        d.join("link.h")
    )
    .unwrap()));
}

#[test]
fn incomplete_reasons_are_recorded_and_stay_out_of_the_digest() {
    // Two triggers in one repo: a non-ASCII byte in a directive's code position, and an
    // unclosed parameter list.
    let c = census(&[
        ("a.h", "#define caf\u{e9} 1\n"),
        ("b.h", "#define F(a, b 1\n"),
    ]);
    assert!(c.incomplete());
    let r = c.incomplete_reasons();
    assert_eq!(r.counts.get("non_ascii_directive"), Some(&1));
    assert!(
        r.counts.len() >= 2,
        "the unclosed parameter list is a second trigger: {:?}",
        r.counts
    );
    let hit = r
        .first
        .iter()
        .find(|x| x.kind == "non_ascii_directive")
        .unwrap();
    assert_eq!((hit.path.as_str(), hit.line), ("a.h", 1));
    assert!(hit.snippet.contains("caf"));
    assert!(c.trace_lines()[0].starts_with("c-census incomplete: "));
    // The reasons never enter the digest or equality: dropping them changes nothing.
    let mut bare = c.clone();
    bare.reasons = Default::default();
    assert_eq!(bare.digest(), c.digest());
    assert_eq!(bare, c);
    // A complete census records nothing.
    let ok = census(&[("a.h", "#define A 1\n")]);
    assert!(ok.incomplete_reasons().first.is_empty() && ok.trace_lines().is_empty());
}
