//! #829 DESIGN-v2 5.2 / PLAN-v2 tasks 6 and 7: the owner gates G7 (`MacroBrackets`), G8
//! (`MacroBodySplit`) and G9 (`MacroArgSplit`, `MacroArgLoose`) through `c_prune::scan`, with a
//! control beside every refusal, plus the owner pass `owner_group_hazard` on bare text. The pair
//! rows of `c_prune_brackets_v2_tests` judge the live-pair consequence; these tests name the
//! mechanism, so a mutant of one premise has one test that kills it.
use super::c_prune::{Off, BLOCK_MACROS};
use super::c_prune_brackets_tests::{expect, verdicts};
use crate::call_graph::c::macro_census::{owner_group_hazard, GroupHazard};

/// One owner `probe` after `defs`; `body` is its statements around the live pair.
fn owner(defs: &str, body: &str) -> String {
    format!(
        "int one(int);\nint two(int, int);\nint ext(int);\nint c;\n{defs}int probe(int c) {{\n    int x = 41;\n{body}    x = 2;\n    return x;\n}}\n"
    )
}

fn verdict(file: &str, defs: &str, body: &str) -> Option<Off> {
    let v = verdicts(file, &owner(defs, body));
    *v.get("probe")
        .unwrap_or_else(|| panic!("no probe in {v:?}"))
}

fn gate(defs: &str, body: &str, want: Option<Off>) {
    assert_eq!(verdict("a.c", defs, body), want, "{defs:?} {body:?}");
}

// ---- G7 ----

#[test]
fn g7_spelled_in_repo_macro_with_unbalanced_body() {
    gate(
        "#define BEGIN {\n#define END }\n",
        "    BEGIN;\n",
        Some(Off::MacroBrackets),
    );
    // Control: a well-nested body.
    gate("#define WRAP(v) { one(v); }\n", "    WRAP(1);\n", None);
    gate("#define EMPTY\n", "    EMPTY;\n", None);
}

#[test]
fn g7_closure_over_the_macros_a_body_names() {
    gate(
        "#define BEGIN {\n#define ENTER BEGIN\n#define ENTER2 ENTER\n",
        "    ENTER2;\n",
        Some(Off::MacroBrackets),
    );
}

#[test]
fn g7_listed_name_spelled_directly() {
    gate(
        "",
        "    pthread_cleanup_push(0, 0);\n",
        Some(Off::MacroBrackets),
    );
    gate("", "    PG_TRY();\n", Some(Off::MacroBrackets));
    gate(
        "",
        "    NPY_BEGIN_ALLOW_THREADS;\n",
        Some(Off::MacroBrackets),
    );
    // Control: a similar name that is not listed.
    gate("", "    pthread_cleanup_pushed(0);\n", None);
}

/// The list is by name: a repository that defines the listed name balanced is refused too.
#[test]
fn g7_listed_name_the_repository_defines_balanced_is_still_off() {
    gate(
        "#define PG_TRY() do { } while (0)\n",
        "    PG_TRY();\n",
        Some(Off::MacroBrackets),
    );
}

#[test]
fn g7_wrapper_of_a_listed_name_through_the_free_set() {
    gate(
        "#define W(a) pthread_cleanup_push(a, 0)\n#define W3(a) W(a)\n",
        "    W3(0);\n",
        Some(Off::MacroBrackets),
    );
    gate(
        "#define AL pthread_cleanup_push\n#define AL2 AL\n",
        "    AL2(0, 0);\n",
        Some(Off::MacroBrackets),
    );
}

/// Round-1 amendment: a PARAMETER named like a listed macro is no free name of the macro.
#[test]
fn g7_parameter_named_like_a_listed_macro_stays_on() {
    gate(
        "#define PASS(pthread_cleanup_push) (pthread_cleanup_push)\n",
        "    PASS(c);\n",
        None,
    );
}

/// Every one of the 31 names: spelled directly, through a function-like wrapper and through an
/// object-like alias, the owner is off with the G7 reason.
#[test]
fn g7_every_listed_name_is_refused_directly_and_through_wrappers() {
    for name in BLOCK_MACROS {
        gate("", &format!("    {name}(0);\n"), Some(Off::MacroBrackets));
        gate(
            &format!("#define W1(a) {name}(a)\n#define W3(a) W1(a)\n"),
            "    W3(0);\n",
            Some(Off::MacroBrackets),
        );
        gate(
            &format!("#define AL {name}\n#define AL2 AL\n"),
            "    AL2(0);\n",
            Some(Off::MacroBrackets),
        );
    }
}

#[test]
fn g7_list_pins() {
    assert_eq!(BLOCK_MACROS.len(), 31);
    let mut sorted = BLOCK_MACROS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 31, "no duplicate name");
    for n in [
        "PG_TRY",
        "NPY_END_SUPPRESS_IPH",
        "Py_TRASHCAN_BEGIN_CONDITION",
        "pthread_cleanup_pop",
    ] {
        assert!(BLOCK_MACROS.contains(&n), "{n}");
    }
}

// ---- G8 ----

const F2: &str = "#define F(p, q) q if (c) p\n#define G(p, q) q p\n";

#[test]
fn g8_held_comma_in_a_body_group() {
    let defs = format!("{F2}#define HI F({{ int a, b; }})\n#define JI G({{ int a, b; }})\n");
    gate(
        &defs,
        "    HI;\n    x = 3;\n    JI;\n",
        Some(Off::MacroBodySplit),
    );
    // Control: the same groups with tight commas.
    let defs = format!("{F2}#define HT F((1, 2), 3)\n");
    gate(&defs, "    HT;\n", None);
}

#[test]
fn g8_forwarded_loose_macro_in_a_body_group() {
    let defs = format!("{F2}#define H(p) F(p)\n#define INIT {{ int a, b; }}\n#define HI H(INIT)\n");
    gate(&defs, "    HI;\n", Some(Off::MacroBodySplit));
    // Controls: the loose macro outside every group; a parameter named like it.
    let defs = format!("{F2}#define INIT {{ int a, b; }}\n#define B3 {{ INIT }}\n");
    gate(&defs, "    B3;\n", None);
    let defs = format!("{F2}#define INIT {{ int a, b; }}\n#define PL(INIT2) F(INIT2, 1)\n#define INIT2 {{ int a, b; }}\n");
    gate(&defs, "    PL(c);\n", None);
}

#[test]
fn g8_unknown_root_and_reach() {
    let defs = "#define INIT { int a, b; }\n#define HU ext(INIT)\n#define HU2 HU\n";
    gate(defs, "    HU;\n", Some(Off::MacroBodySplit));
    gate(defs, "    HU2;\n", Some(Off::MacroBodySplit));
}

#[test]
fn g8_named_variadic_fails_closed() {
    gate(
        "#define NV(args...) one(args)\n",
        "    NV(1);\n",
        Some(Off::MacroBodySplit),
    );
    gate("#define OV(...) one(__VA_ARGS__)\n", "    OV(1);\n", None);
}

#[test]
fn g8_vaopt_chain_root() {
    let defs =
        "#define H(...) __VA_OPT__(F)({ int a, b; }, __VA_ARGS__)\n#define F(p,q,r) q if(r) p\n";
    gate(defs, "    H(c);\n", Some(Off::MacroBodySplit));
}

// ---- G9: the owner pass on bare text ----

fn hazard(text: &str) -> Option<Option<GroupHazard>> {
    owner_group_hazard(text, &|n| n == "INIT" || n == "PAIR")
}

fn refuses(text: &str, want: GroupHazard) {
    assert_eq!(hazard(text), Some(Some(want)), "{text:?}");
}

fn admits(text: &str) {
    assert_eq!(hazard(text), Some(None), "{text:?}");
}

#[test]
fn g9_held_commas_inside_potential_groups() {
    refuses("F({ int a, b; }, c);", GroupHazard::Split);
    refuses("ext({ int a, b; });", GroupHazard::Split);
    refuses("SF(({ int a, b; }));", GroupHazard::Split);
    refuses("AF({ int a, b; }, c);", GroupHazard::Split);
    refuses("ID(F)({ int a, b; }, c);", GroupHazard::Split);
    refuses("ext()({ int a, b; });", GroupHazard::Split);
    refuses("ext(a)(b)({ int a, b; });", GroupHazard::Split);
    refuses("M(a[i, j]);", GroupHazard::Split);
    // No shield: unknown names and identity wrappers around the group hide nothing.
    refuses("one(K({ int a, b; }));", GroupHazard::Split);
    refuses("ID(one(K({ int a, b; })));", GroupHazard::Split);
    refuses("ID(sizeof E({ int a, b; }, c));", GroupHazard::Split);
    refuses("EXT(sizeof E({ int a, b; }, c));", GroupHazard::Split);
}

#[test]
fn g9_tight_commas_and_commas_outside_potential_groups_are_admitted() {
    admits("F((int a, b), c);");
    admits("ext((a, b));");
    admits("ID(two((1, 2)));");
    admits("H(f(a, b));");
    admits("{ int a, b; }");
    admits("x = (T){ a, b };");
    admits("int arr[] = { 1, 2 };");
    // A paren after punctuation is plain: no callee can take it on a left-to-right scan.
    admits("x = ({ int a, b; });");
    admits("y + ({ int a, b; });");
}

#[test]
fn g9_loose_macro_inside_a_potential_group() {
    refuses("H(INIT);", GroupHazard::Loose);
    refuses("ext(INIT);", GroupHazard::Loose);
    refuses("ID(two(PAIR));", GroupHazard::Loose);
    refuses("ext((INIT));", GroupHazard::Loose);
    // Controls: the loose macro outside every group.
    admits("int a[] = INIT;");
    admits("{ INIT; }");
    admits("H((a, b));");
}

#[test]
fn g9_mismatched_closer_and_digraph_inside_a_group_refuse() {
    refuses("f(a ] b);", GroupHazard::Split);
    refuses("f({ a ) });", GroupHazard::Split);
    refuses("f(<% a %>);", GroupHazard::Split);
    // Outside a group the same text is the parser's business, not this pass's.
    admits("a ] b;");
}

#[test]
fn g9_both_dialects_are_read() {
    // In C `'0, b = 2'` is one character literal that hides the held comma; in C++ it is a
    // digit separator and the comma is visible.
    refuses("F({ int a = 1'0, b = 2'0; });", GroupHazard::Split);
}

#[test]
fn g9_unreadable_text_is_none() {
    assert_eq!(hazard("f(\"unterminated"), None);
}

// ---- G9: through `scan` ----

#[test]
fn g9_owner_text_refuses_with_its_own_reason() {
    gate(F2, "    F({ int a, b; }, c);\n", Some(Off::MacroArgSplit));
    gate(
        "",
        "    ext(({ int a, b; 1; }));\n",
        Some(Off::MacroArgSplit),
    );
    gate(
        &format!("{F2}#define INIT {{ int a, b; }}\n#define H(p) F(p, c)\n"),
        "    H(INIT);\n",
        Some(Off::MacroArgLoose),
    );
    // Controls.
    gate(F2, "    F((c), 1);\n", None);
    gate(
        &format!("{F2}#define INIT {{ int a, b; }}\n"),
        "    int y[] = INIT;\n",
        None,
    );
}

/// G7 and G8 precede G9: an owner that trips both names the earlier gate.
#[test]
fn g9_comes_after_g7_and_g8() {
    gate(
        "#define BEGIN {\n",
        "    BEGIN;\n    ext({ int a, b; });\n",
        Some(Off::MacroBrackets),
    );
}

// ---- parser pins (DESIGN-v2 5.2, "Parser scope, corrected") ----

/// A direct expression argument interrupted by `#if` is a parse error (G1Error): clang accepts
/// the program, the owner is off before any macro gate runs.
#[test]
fn parser_pin_direct_conditional_argument_is_g1_error() {
    let src = "#define ID(v) v\nint probe(void) {\n    int x = 41;\n    int y = ID(\n#if 1\n        2\n#else\n        3\n#endif\n    );\n    x = 2;\n    return x + y;\n}\n";
    let v = verdicts("a.c", src);
    expect(&v, Some(Off::G1Error), &["probe"]);
}

/// A compound-statement argument holding complete `#if` block items parses and is admitted:
/// no macro gate fires on it (the three dropped pairs are CONFIRMED by the oracle).
#[test]
fn parser_pin_conditional_block_argument_is_admitted() {
    let src = "#define ID(v) v\nint probe(void) {\n    int x = 41;\n    ID({\n#if 1\n        int a = 0;\n#else\n        int a = 1;\n#endif\n        (void)a;\n    });\n    x = 2;\n    return x;\n}\n";
    let v = verdicts("a.c", src);
    expect(&v, None, &["probe"]);
}

/// Digraph owners keep their existing parser pin: the grammar does not read `<%` as a brace.
#[test]
fn parser_pin_digraph_owner_keeps_its_verdict() {
    let src = "int probe(int c) <%\n    int x = 41;\n    x = 2;\n    return x;\n%>\n";
    let v = verdicts("a.c", src);
    assert!(v.values().all(|o| o.is_some()), "{v:?}");
}
