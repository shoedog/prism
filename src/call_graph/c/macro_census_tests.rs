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
    assert_eq!(c.get("M").unwrap().free, set(&["sink", "x"]));
    assert!(c.get("G").unwrap().control);
    assert_eq!(c.get("P").unwrap().free, set(&["secret"]));
    assert_eq!(c.get("Q").unwrap().free, set(&["after"]));
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
