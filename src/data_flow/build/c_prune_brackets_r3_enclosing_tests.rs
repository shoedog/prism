//! #829 chain B, round 3 (opus3, class E): the owner sits INSIDE an invocation of a macro that
//! splits its arguments at the owner's own commas (`G2-opus3-01 .. 10`). Every owner here spells
//! no macro and parses cleanly (`probe`); the compiler's expansion `p; if (c) { q; } r` turns
//! the overwrite `x = 2` into a conditional one, so `x = 41 -> return x` is live when
//! `c == 0` (compiled by the enumerators: `41 2` or `42 3`). Pair identity is by OWNER
//! (`probe`): a nested owner's pair has a duplicate `cfg_incomplete` copy in its enclosing
//! owner that survives on base. Required behaviour only, never an `Off::` reason. Single
//! files, one shape each (the group is left open or closed in a sibling), so the rows are
//! dedicated tests and not inline matrix cases (TESTS.md "Chain B").
//! Kinds: S live pair dropped on base (obs: compiled), C conservative (rules), K control.
use super::c_prune_brackets_r3_support::{kept_guard, pruned_guard, Batch};
use super::c_prune_oracle_rows::declare;

const J: &str = "int b, e;\n#define J(p, q, r) p; if (c) { q; } r\n";
const BODY_C: &str = "int probe(int c) {\n    int x = 41;\n    int a, b;\n    x = 2;\n    int d, e;\n    return x;\n}\n";
const BODY_CC: &str = "int probe(int c) {\n    int x = 41;\n    int a, b;\n    x = 2;\n    int d, e;\n    return x + 1;\n}\n";

fn body(cpp: bool) -> &'static str {
    if cpp {
        BODY_CC
    } else {
        BODY_C
    }
}

fn ext(cpp: bool) -> &'static str {
    if cpp {
        "a.cc"
    } else {
        "a.c"
    }
}

// ---- G2-opus3-01: a clean parse, no ERROR node anywhere (S, C++, obs: `42 3`) ----

const LAM: &str = "#define J(p, q, r) p; if (c) { q; } r\nint b, e;\nstatic int reg = J([] { struct S { int probe(int c) {\n    int x = 41;\n    int a, b;\n    x = 2;\n    int d, e;\n    return x + 1;\n} }; return 0; }());\n";

#[test]
fn row_g2_opus3_01_lambda_local_class_method_in_a_clean_parse() {
    kept_guard("opus3-01 lam", "a.cc", LAM, Some("probe"));
}

/// Control (K, rules): `J(...) __VA_ARGS__` rejoins the pieces, so the compiled program equals
/// the written one and the overwrite dominates: pruning must survive.
#[test]
fn row_g2_opus3_01_variadic_control_prunes() {
    let src = LAM.replace(
        "#define J(p, q, r) p; if (c) { q; } r",
        "#define J(...) __VA_ARGS__",
    );
    pruned_guard("opus3-01 control", "a.cc", &src, Some("probe"));
}

// ---- G2-opus3-01b: the same lambda inside a function; the OUTER owner keeps a copy ----

const LAM2: &str = "#define J(p, q, r) p; if (c) { q; } r\nint b, e;\nint outer(int c0) {\n    return J([&] { struct S { int probe(int c) {\n        int x = 41;\n        int a, b;\n        x = 2;\n        int d, e;\n        return x + 1;\n    } }; S s; return s.probe(c0); }());\n}\n";

/// The inner owner's edge, not `outer`'s `cfg_incomplete` copy (which a from/to check finds on
/// base): `x@5 -> x@9` of the owner `probe`.
#[test]
fn row_g2_opus3_01b_lambda_inside_a_function_inner_owner_edge() {
    kept_guard("opus3-01b lam2", "a.cc", LAM2, Some("probe"));
}

// ---- G2-opus3-01c (C, rules): a GNU nested function in a compound-statement argument ----

const GNEST: &str = "#define J(p, q, r) p; if (c) { q; } r\nint b, e;\nvoid outer(void) {\n    J({ int probe(int c) {\n        int x = 41;\n        int a, b;\n        x = 2;\n        int d, e;\n        return x;\n    } sink(probe(0)); });\n}\n";

#[test]
fn row_g2_opus3_01c_gnu_nested_function_inner_owner_edge() {
    kept_guard("opus3-01c gnest", "a.c", GNEST, Some("probe"));
}

// ---- G2-opus3-02: a macro with no bracket, no comma and no `;` deletes the overwrite ----

fn dropp(cpp: bool, name: &str, body: &str) -> String {
    format!(
        "#define {name}(p, q, r) {body}\n{name}(static int dummy;\nint probe(int c) {{\n    int x = 41;\n    int t[2] = {{0, 0}};\n    x = 2;\n    int u[2] = {{0, 0}};\n    return {};\n}}\n)\n",
        if cpp { "x + 1" } else { "x" }
    )
}

#[test]
fn row_g2_opus3_02_a_macro_drops_the_overwriting_piece_c() {
    // `DROP(p, q, r) p + r`: `clang -E -P` leaves `int t[2] = {0 + 0}; return x;` (prints `41 41`).
    let src = dropp(false, "DROP", "p + r");
    kept_guard("opus3-02 c", "a.c", &src, Some("probe"));
}

#[test]
fn row_g2_opus3_02_a_macro_drops_the_overwriting_piece_cpp() {
    let src = dropp(true, "DROP", "p + r");
    kept_guard("opus3-02 cc", "a.cc", &src, Some("probe"));
}

/// Member (rules): a stringizing macro deletes the same piece (`#q[0]` is a char constant).
#[test]
fn row_g2_opus3_02_stringizing_member_c_and_cpp() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = dropp(cpp, "HIDE", "p + #q[0] + r");
        b.kept_guard("opus3-02 hide", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

// ---- G2-opus3-03 .. 05: where the group is opened or closed (S, C and C++, obs) ----

/// 03: the opener sits inside a preceding sibling owner that has a parse error (`first`).
#[test]
fn row_g2_opus3_03_opener_inside_a_sibling_owner() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = format!("{J}void first(void) {{ J(0;\n}}\n{})\n", body(cpp));
        b.kept_guard("opus3-03 sib", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// 04: the opener sits inside a declaration's initializer.
#[test]
fn row_g2_opus3_04_opener_inside_a_declaration_initializer() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = format!("{J}static int dummy = J(0;\n{})\n", body(cpp));
        b.kept_guard("opus3-04 decl", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// 05: the closing parenthesis sits inside a following owner (`last`).
#[test]
fn row_g2_opus3_05_closer_inside_a_following_owner() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = format!(
            "{J}J(static int dummy;\n{}void last(void) {{ )\n}}\n",
            body(cpp)
        );
        b.kept_guard("opus3-05 closer", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

// ---- G2-opus3-06 .. 09 (C++): the kind of owner inside the group ----

/// 06: an in-class member function (`42 3`).
#[test]
fn row_g2_opus3_06_in_class_member_function() {
    let src = format!(
        "{J}struct T {{\nJ(static const int dummy = 0;\n{})\n}};\n",
        BODY_CC
    );
    kept_guard("opus3-06 cls", "a.cc", &src, Some("probe"));
}

/// 07: a `template_declaration` owner (the pair is `cfg_incomplete` here, not `killed`).
#[test]
fn row_g2_opus3_07_template_declaration_owner() {
    let src = format!(
        "{J}J(static int dummy;\ntemplate <class T> {})\n",
        BODY_CC.replace("int probe(int c)", "int probe(T c)")
    );
    kept_guard("opus3-07 tpl", "a.cc", &src, Some("probe"));
}

/// 08: the splitting commas are template-argument commas; the macro keeps the first and last
/// pieces (`std::pair<int` + `,` + ` int> pq;`), `42 42`.
#[test]
fn row_g2_opus3_08_template_argument_commas_split_the_group() {
    let src = "#include <utility>\n#define KEEP13(p, q, r) p, r\nKEEP13(static int dummy;\nint probe(int c) {\n    int x = 41;\n    std::pair<int, int> pr; x = 2; std::pair<int, int> pq;\n    return x + 1;\n}\n)\n";
    kept_guard("opus3-08 tmpl", "a.cc", src, Some("probe"));
}

/// 09: a namespace opened inside the group; the errored outer node keeps a duplicate copy of
/// the pair, so the row names the owner `probe`.
#[test]
fn row_g2_opus3_09_namespace_inside_the_group_inner_owner_edge() {
    let src = format!("{J}J(namespace n {{\n{}}})\n", BODY_CC);
    kept_guard("opus3-09 ns", "a.cc", &src, Some("probe"));
}

// ---- G2-opus3-10 (BOUNDARY): the enclosing macro is external ----

/// `EJ(p, q, r)` is defined in a header the build preincludes (`-include ej.h`, outside the
/// repository). D2 says an external macro acts as a function; here the owner is entirely
/// inside the "call" and its own commas are the separators. Owner decision: keep the pair or
/// disclose the shape under D2.
#[test]
fn boundary_g2_opus3_10_external_enclosing_macro() {
    let src = format!("int b, e;\nEJ(static int dummy;\n{})\n", BODY_C);
    declare(
        "A-ext",
        &["-include", "{PRELUDE}/ej.h"],
        &[("ej.h", "#define EJ(p, q, r) p; if (c) { q; } r\n")],
    );
    kept_guard("opus3-10 extencl", "a.c", &src, Some("probe"));
}
