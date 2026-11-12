//! C and C++ rows of the alias-twin occurrence table: oracle parity on the pinned fixtures, the
//! classifier negatives and the adversarial branches of `unit_cases.py`.
use super::c_family::classify;
use super::test_support::parse;
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

fn name(d: Disposition) -> &'static str {
    match d {
        Disposition::BindingWrite => "binding-write",
        Disposition::StorageWrite => "storage-write",
        Disposition::Unsupported => "unsupported",
    }
}

fn classified_with(
    parsed: &ParsedFile,
    classify: fn(&ParsedFile, &Node<'_>, &PathSpan) -> Classified,
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for func in parsed.all_functions() {
        let (start, end) = parsed.node_line_range(&func);
        let lines: BTreeSet<usize> = (start..=end).collect();
        for span in parsed.assignment_lvalue_spans_on_lines(&func, &lines) {
            let c = classify(parsed, &func, &span);
            out.entry(span.line).or_default().insert(format!(
                "{}:{}:{}",
                span.path,
                name(c.disposition),
                c.rule
            ));
        }
    }
    out
}

/// Every `EXPECT tok` token of the fixture is among the classifier's answers for its line (a
/// `storage-write` token also accepts `unsupported`: both keep the twin).
fn check_fixture(fixture: &str, source: &str, language: Language) {
    let got = classified_with(&parse(fixture, source, language), classify);
    let mut missing = Vec::new();
    let mut count = 0;
    for (i, line) in source.lines().enumerate() {
        let Some(pos) = line.find("EXPECT ") else {
            continue;
        };
        for token in line[pos + "EXPECT ".len()..].split_whitespace() {
            count += 1;
            let set = got.get(&(i + 1));
            let parts = token.split(':').collect::<Vec<_>>();
            let ok = set.is_some_and(|s| {
                s.contains(token)
                    || (parts.get(1) == Some(&"storage-write")
                        && s.iter()
                            .any(|g| g.starts_with(&format!("{}:unsupported:", parts[0]))))
            });
            if !ok {
                missing.push(format!("{fixture}:{} {token} (got {set:?})", i + 1));
            }
        }
    }
    assert!(count >= 10, "{fixture}: fixture pins too little");
    assert!(missing.is_empty(), "{missing:#?}");
}

/// Dispositions of the lvalue spans on `line` of a snippet.
fn d(src: &str, language: Language, line: usize) -> BTreeSet<String> {
    let file = if language == Language::C {
        "a.c"
    } else {
        "a.cc"
    };
    classified_with(&parse(file, src, language), classify)
        .remove(&line)
        .unwrap_or_default()
}

#[test]
fn oracle_pinned_cases_c() {
    check_fixture(
        "c.c",
        include_str!("../../../../tests/fixtures/alias_twin_c/c.c"),
        Language::C,
    );
}

#[test]
fn oracle_pinned_cases_cpp() {
    check_fixture(
        "cpp.cc",
        include_str!("../../../../tests/fixtures/alias_twin_cpp/cpp.cc"),
        Language::Cpp,
    );
}

/// Dispositions on the LAST line of `src` that contains `needle` (the use follows the declaration).
fn at(src: &str, language: Language, needle: &str) -> BTreeSet<String> {
    let all: Vec<&str> = src.lines().collect();
    let line = all
        .iter()
        .rposition(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}"))
        + 1;
    d(src, language, line)
}
fn is(set: &BTreeSet<String>, tok: &str, disposition: &str) -> bool {
    set.iter()
        .any(|s| s.starts_with(&format!("{tok}:{disposition}:")))
}

/// The adversarial branches of `classifier/unit_cases.py` for C and C++, as parsed sources:
/// every removal branch (`binding-write`) has a case where a naive reading sees a storage effect.
#[test]
fn unit_cases_cpp_value_kind_evidence() {
    // C-01 C++ typedef/using reference, class (proxy), auto, dependent type: never a binding write.
    for (what, src, needle, tok) in [
        (
            "using Ref = int&",
            "using Ref = int&;\nvoid t(int &o) {\n Ref a = o;\n a = 1;\n}\n",
            "a = 1",
            "a",
        ),
        (
            "proxy class",
            "struct Proxy { int *p; };\nvoid t(Proxy px) {\n Proxy q = px;\n q = px;\n}\n",
            "q = px",
            "q",
        ),
        (
            "auto",
            "void t(int &py) {\n auto z = py;\n z = py;\n}\n",
            "z = py",
            "z",
        ),
        (
            "dependent T",
            "template <class T> void t(T py) {\n T z = py;\n z = py;\n}\n",
            "z = py",
            "z",
        ),
        (
            "size_t spelling",
            "using size_t = int&;\nvoid t(int &o) {\n size_t sa = o;\n sa = 7;\n}\n",
            "sa = 7",
            "sa",
        ),
        (
            "uint32_t spelling",
            "void t(int &o) {\n uint32_t sa = o;\n sa = 7;\n}\n",
            "sa = 7",
            "sa",
        ),
        (
            "std::size_t spelling",
            "void t(int &o) {\n std::size_t sa = o;\n sa = 7;\n}\n",
            "sa = 7",
            "sa",
        ),
        (
            "class compound",
            "struct C { C& operator+=(const C&); };\nvoid t(C d) {\n C c = d;\n c += d;\n}\n",
            "c += d",
            "c",
        ),
    ] {
        let got = at(src, Language::Cpp, needle);
        assert!(!is(&got, tok, "binding-write"), "{what}: {got:?}");
    }
    // Controls: keyword types and pointer declarators are binding writes.
    let kw = "void t(int m) {\n int n = m;\n n = 7;\n}\n";
    assert!(is(&at(kw, Language::Cpp, "n = 7"), "n", "binding-write"));
    let ul = "void t(int o) {\n unsigned long sa = o;\n sa = 7;\n}\n";
    assert!(is(&at(ul, Language::Cpp, "sa = 7"), "sa", "binding-write"));
    let ptr = "void t(int *o) {\n size_t *sa = o;\n sa = o;\n}\n";
    assert!(is(&at(ptr, Language::Cpp, "sa = o"), "sa", "binding-write"));
    // A reference is a storage write.
    let rf = "void t(int &o) {\n int &r = o;\n r = 5;\n}\n";
    assert!(is(&at(rf, Language::Cpp, "r = 5"), "r", "storage-write"));
}

#[test]
fn unit_cases_cpp_visible_declarations_must_agree_and_hold() {
    // Two visible declarations that disagree (`int n` outside, a reference inside).
    let two =
        "using Ref = int&;\nvoid t(int &m) {\n int n = 0;\n {\n  Ref n = m;\n  n = 1;\n }\n}\n";
    assert!(!is(&at(two, Language::Cpp, "n = 1"), "n", "binding-write"));
    // A declaration whose scope ended before the occurrence.
    let out = "void t(int m) {\n { int n = m; }\n n = 1;\n}\n";
    assert!(!is(&at(out, Language::Cpp, "n = 1"), "n", "binding-write"));
    // A declaration after the occurrence is not visible.
    let after = "void t(int m) {\n n = 1;\n int n = m;\n}\n";
    assert!(!is(
        &at(after, Language::Cpp, "n = 1"),
        "n",
        "binding-write"
    ));
    // Control: one visible keyword declaration.
    let one = "void t(int m) {\n int n = m;\n { n = 1; }\n}\n";
    assert!(is(&at(one, Language::Cpp, "n = 1"), "n", "binding-write"));
}

#[test]
fn unit_cases_c_update_wrappers() {
    // C-10: the destination of `*p++ = v`, `*++p = v`, `(*p--) = v`, `*p++ += v` is storage.
    let src = "void t(char *buf0) {\n char *p = buf0;\n *p++ = 1;\n *++p = 2;\n (*p--) = 3;\n *p++ += 4;\n}\n";
    for needle in ["*p++ = 1", "*++p = 2", "(*p--) = 3", "*p++ += 4"] {
        let got = at(src, Language::C, needle);
        assert!(is(&got, "p", "storage-write"), "{needle}: {got:?}");
        assert!(!is(&got, "p", "binding-write"), "{needle}: {got:?}");
    }
    // `*p += 1` through a dereference target is storage (C-05).
    let deref = "void t(int *buf0) {\n int *p = buf0;\n *p += 1;\n}\n";
    assert!(is(&at(deref, Language::C, "*p += 1"), "p", "storage-write"));
    // C++: a class iterator has no pointer evidence, so `*it++ = v` is unsupported (not storage).
    let it = "struct Iter {};\nvoid t(char *buf0) {\n Iter it = buf0;\n *it++ = 1;\n}\n";
    let got = at(it, Language::Cpp, "*it++ = 1");
    assert!(is(&got, "it", "unsupported"), "{got:?}");
    // RHS-only `x = *p++` gives the name no lvalue span at all (no twin to remove or keep).
    let rhs = "void t(char *buf0, int xx) {\n char *p = buf0;\n xx = *p++;\n}\n";
    assert!(!at(rhs, Language::C, "xx = *p++")
        .iter()
        .any(|s| s.starts_with("p:")));
}

#[test]
fn unit_cases_declarator_text_that_is_an_expression_is_not_a_declarator() {
    use super::c_family::declarator_shaped;
    assert!(!declarator_shaped(
        "parenthesized_declarator",
        "(v->f != 0)"
    ));
    assert!(!declarator_shaped("pointer_declarator", "*u = o"));
    assert!(!declarator_shaped("array_declarator", "a[n = 1]"));
    assert!(declarator_shaped("pointer_declarator", "**const_p"));
    assert!(declarator_shaped("array_declarator", "*a[N]"));
    assert!(declarator_shaped("reference_declarator", "&&r"));
    assert!(declarator_shaped("parenthesized_declarator", "(*fp)"));
}

#[test]
fn x_read_forms_are_unsupported() {
    // A call argument, an initialiser value, a cast or binary operand and `sizeof` are reads that
    // the extraction records as lvalue spans.
    let src = "void t(int *o, int n) {\n int *u = o;\n int r = bar(u);\n int s = n + 1;\n int c = (long)n;\n}\n";
    let got = at(src, Language::C, "bar(u)");
    assert!(is(&got, "u", "unsupported"), "{got:?}");
    assert!(!is(&got, "u", "binding-write"));
}
