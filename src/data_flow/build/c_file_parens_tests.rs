//! #829 DESIGN-v2 5.3 / PLAN-v2 task 8: the exposed-owner test. Each test names the owners of a
//! source and whether the file's own parentheses (read over every branch of conditional
//! compilation) or a hidden opener (a macro with `paren_open` spelled before the owner) can
//! make it part of an argument list that starts outside it.
use super::c_file_parens::CFileParens;
use super::c_prune_brackets_tests::owner_nodes;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::languages::Language;
use std::collections::BTreeMap;

fn exposed(file: &str, src: &str) -> BTreeMap<String, bool> {
    let lang = if file.ends_with(".cc") {
        Language::Cpp
    } else {
        Language::C
    };
    let parsed = ParsedFile::parse(file, src, lang).unwrap();
    let census = CMacroCensus::of_file(&parsed);
    let fp = CFileParens::for_file(&parsed, &|n| census.get(n).is_some_and(|f| f.paren_open));
    owner_nodes(&parsed)
        .into_iter()
        .map(|(name, node)| (name, fp.exposed(node.start_byte(), node.end_byte())))
        .collect()
}

fn check(file: &str, src: &str, rows: &[(&str, bool)]) {
    let got = exposed(file, src);
    for (name, want) in rows {
        assert_eq!(
            got.get(*name),
            Some(want),
            "owner {name} of {src:?}: {got:?}"
        );
    }
}

const BODY: &str = "int probe(int c) {\n    int x = 41;\n    x = 2;\n    return x;\n}\n";

// ---- the interval test ----

#[test]
fn owner_after_an_unclosed_paren_at_file_scope_is_exposed() {
    let src = format!("#define J(p, q, r) p; q; r\nint b, e;\nJ(static int dummy;\n{BODY})\n");
    check("a.c", &src, &[("probe", true)]);
    // Control: the group is closed before the owner.
    let src = format!("#define J(p, q, r) p; q; r\nint b, e;\nJ(static int dummy;, 0, 1)\n{BODY}");
    check("a.c", &src, &[("probe", false)]);
}

#[test]
fn owner_whose_group_was_opened_in_a_sibling_owner_is_exposed() {
    let src = format!("#define J(p, q, r) p; q; r\nvoid first(void) {{ J(0;\n}}\n{BODY})\n");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn owner_after_a_declaration_initializer_that_opens_a_group_is_exposed() {
    let src = format!("#define J(p, q, r) p; q; r\nstatic int dummy = J(0;\n{BODY})\n");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn closer_after_the_owner_with_the_opener_hidden_in_a_macro_is_exposed() {
    let src = format!(
        "#define J(p, q, r) p; q; r\n#define BEGIN_FNS J(\nBEGIN_FNS static int dummy;\n{BODY})\n"
    );
    check("a.c", &src, &[("probe", true)]);
}

/// A literal `(` before the owner that nothing closes: only the first half of the interval
/// test sees it (the file is an argument list that its includer closes).
#[test]
fn unclosed_paren_before_the_owner_is_exposed() {
    let src = format!("#define J(p) p\nJ(static int dummy;\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
    // Control: closed before the owner.
    let src = format!("#define J(p) p\nJ(static int dummy;)\n{BODY}");
    check("a.c", &src, &[("probe", false)]);
}

/// A literal `)` after the owner that nothing in the file opens (the opener sits in a file
/// that includes this one, or is hidden): only the second half of the interval test sees it.
#[test]
fn stray_closer_after_the_owner_is_exposed() {
    let src = format!("{BODY})\n");
    check("a.c", &src, &[("probe", true)]);
    // Control: the same closer, opened after the owner.
    let src = format!("{BODY}int g = (1 + 2);\n");
    check("a.c", &src, &[("probe", false)]);
}

/// `hull2`: an opener and its closer under `#else` branches of two conditionals.
#[test]
fn opener_and_closer_under_else_branches_expose_the_owner() {
    let src = format!(
        "#define W3(p, q, r) p; r\n#if defined(NEVER_829)\n#else\nW3(\n#endif\nint dummy829;\n{BODY}#if defined(NEVER_829)\n#else\n)\n#endif\n"
    );
    check("a.c", &src, &[("probe", true)]);
    // The first half alone: the opener before the owner, no closer anywhere.
    let src = format!("#if defined(N)\n#else\nW3(\n#endif\nint d;\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
}

/// Two branches that each open one parenthesis, closed after `#endif`: the owner after the
/// closer is not exposed (a plain `#else` is exhaustive, so the base is not a branch end).
#[test]
fn balanced_branches_closed_after_endif_do_not_expose() {
    let src = format!(
        "int g(int, int), h(int, int), a;\n#if A\nint u = g(1,\n#else\nint u = h(1,\n#endif\n 2);\n{BODY}"
    );
    check("a.c", &src, &[("probe", false)]);
}

/// `#if 0` prose with an apostrophe and an unbalanced parenthesis exposes the owners after it
/// (the pinned cost of the interval hull).
#[test]
fn if_zero_prose_with_an_unbalanced_parenthesis_exposes_later_owners() {
    let src = format!("#if 0\nthis ( prose isn't balanced\n#endif\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
    // Control: balanced prose.
    let src = format!("#if 0\nthis ( prose ) isn't balanced\n#endif\n{BODY}");
    check("a.c", &src, &[("probe", false)]);
}

#[test]
fn parentheses_in_directives_comments_strings_and_raw_strings_are_not_counted() {
    for src in [
        format!("#define OPEN(a) (a\n{BODY}"),
        format!("#if (1 && (2)\n#endif\n{BODY}"),
        format!("#error unbalanced ( text\n{BODY}"),
        format!("/* ( */ // (\nconst char *s = \"(\";\nchar ch = '(';\n{BODY}"),
        format!("#define LONG(a) \\\n  (a\n{BODY}"),
    ] {
        check("a.c", &src, &[("probe", false)]);
    }
    let src = format!("const char *s = R\"x(\n(\n)x\";\n{BODY}");
    check("a.cc", &src, &[("probe", false)]);
}

/// A nested owner (a local class method) is at depth 0 inside its outer function; a method
/// inside a lambda inside a call is not.
#[test]
fn nested_owners() {
    let src = "int outer(int c) {\n    struct L {\n        static int probe(int c) {\n            int x = 41;\n            x = 2;\n            return x;\n        }\n    };\n    return L::probe(c);\n}\n";
    check("a.cc", src, &[("probe", false), ("outer", false)]);
    let src = "void run(void (*)(int));\nvoid outer2(int c) {\n    run([&](int d) {\n        struct M { int probe2(int c) {\n            int x = 41;\n            x = 2;\n            return x;\n        } };\n    });\n}\n";
    check("a.cc", src, &[("probe2", true), ("outer2", false)]);
}

/// DESIGN 5.3 cascade (pinned cost): a parenthesis opened under `#ifdef X` and closed under a
/// later `#ifdef X` gets a wide interval, so the owners between and after are exposed.
#[test]
fn uncorrelated_same_condition_branches_cascade() {
    let src = format!(
        "#define J(p, q, r) p; q; r\n#ifdef X\nJ(static int d;\n#endif\n{BODY}#ifdef X\n)\n#endif\nint z;\n"
    );
    check("a.c", &src, &[("probe", true)]);
}

// ---- the hidden opener ----

const OPENER: &str = "#define W3(p, q, r) p; r\n#define BEGIN_IMPL W3(\n";

#[test]
fn macro_with_paren_open_spelled_at_file_scope_before_the_owner_exposes_it() {
    let src = format!("{OPENER}BEGIN_IMPL\nint dummy829;\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn alias_of_a_paren_open_macro_exposes_it_too() {
    let src = format!("{OPENER}#define ALIAS BEGIN_IMPL\nALIAS\nint dummy829;\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn macro_spelled_inside_a_sibling_owner_before_it_exposes_it() {
    let src = format!("{OPENER}int g(int c) {{\n    return c BEGIN_IMPL ; }}\n, {BODY}");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn macro_spelled_under_if_zero_before_the_owner_exposes_it() {
    let src = format!("{OPENER}#if 0\nBEGIN_IMPL\n#endif\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
}

/// Pinned cost: a literal `)` before the owner that closes the list does not help.
#[test]
fn closed_by_a_literal_paren_before_the_owner_is_still_exposed() {
    let src = format!("{OPENER}BEGIN_IMPL\nint d1;\n, int d2;\n, int d3;\n)\n{BODY}");
    check("a.c", &src, &[("probe", true)]);
}

#[test]
fn spelled_only_after_the_owner_in_a_define_a_comment_or_a_string_does_not_expose() {
    for tail in [
        "int tail;\nBEGIN_IMPL\n",
        "#define USE BEGIN_IMPL\n",
        "/* BEGIN_IMPL */\n",
        "const char *s = \"BEGIN_IMPL\";\n",
    ] {
        let src = format!("{OPENER}{BODY}{tail}");
        // After the owner only the interval half can speak: the tail holds no parenthesis.
        check("a.c", &src, &[("probe", false)]);
    }
}

#[test]
fn spelled_in_a_define_body_comment_or_string_before_the_owner_does_not_expose() {
    for head in [
        "#define USE BEGIN_IMPL\n",
        "/* BEGIN_IMPL */\n",
        "const char *s = \"BEGIN_IMPL\";\n",
        "#if BEGIN_IMPL_NEVER\n#endif\n",
    ] {
        let src = format!("{OPENER}{head}{BODY}");
        check("a.c", &src, &[("probe", false)]);
    }
}

/// A brace-only block macro opens no parenthesis.
#[test]
fn brace_only_block_macro_before_the_owner_does_not_expose() {
    let src = format!("#define BEGIN {{\n#define END }}\nBEGIN END\n{BODY}");
    check("a.c", &src, &[("probe", false)]);
}

/// The fact is by name over every definition: a local balanced redefinition of a name that
/// another census input defines with `paren_open` still exposes the owners after its use.
#[test]
fn name_union_cost_with_a_local_balanced_redefinition() {
    let src = format!(
        "#define CASE(op) op(\n#undef CASE\n#define CASE(op) {{ op; }}\nvoid use(void) {{ CASE(1); }}\n{BODY}"
    );
    check("a.c", &src, &[("probe", true)]);
}
