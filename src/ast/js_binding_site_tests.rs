//! S1b-3 unit rows (SPEC §7 C-1, C-3, C-4, C-7, C-8, C-10–C-14, C-17, C-38): the full
//! end-to-end classification at a call site (`js_ts_site_binding`), through nested scopes.
use super::{JsBinding, JsBindingCache};
use crate::ast::js_binding::JsTerminal;
use crate::ast::js_binding_helper_tests::{ident, parse};

fn callable(local: &str, start_line: usize, end_line: usize) -> JsBinding {
    JsBinding::Callable(JsTerminal {
        local: local.to_string(),
        start_line,
        end_line,
        wrapped: false,
    })
}

const BOTH: [&str; 2] = ["a.js", "a.tsx"];
const MAY: JsBinding = JsBinding::MayCall;
const NC: JsBinding = JsBinding::Refused("not_callable");

/// `(id, source, name, occurrence index, want)`.
fn check(paths: &[&str], rows: Vec<(&str, &str, &str, usize, JsBinding)>) {
    let mut wrong = Vec::new();
    for path in paths {
        for (id, src, name, at, want) in &rows {
            let p = parse(path, src);
            let mut cache = JsBindingCache::default();
            let site = ident(&p, name, *at);
            let got = p.js_ts_site_binding(site, name, &mut cache);
            if got != *want {
                wrong.push(format!("{id} {path} `{name}`: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong bindings:\n{}", wrong.join("\n"));
}

#[test]
fn c2_nested_producers_bind_by_the_enclosing_declaration() {
    let src = "function outer() {\n  function inner() {\n    return inner();\n  }\n\
        return inner;\n}\n";
    check(
        &BOTH,
        vec![("C-2", src, "inner", 1, callable("inner", 2, 4))],
    );
}

#[test]
fn c3_shadowing_parameter_drops() {
    let src = "function f() {\n  return 1;\n}\nfunction g(f) {\n  return f();\n}\n";
    check(&BOTH, vec![("C87", src, "f", 2, NC)]);
}

#[test]
fn c_m4_single_arrow_parameter_shadows() {
    // F3 (S1b-1): a single unparenthesized arrow parameter (`x => x()`) is recorded at the
    // arrow's own `parameter` field, not the (absent) `parameters` list; a same-named module
    // function must not leak through it (C-M4).
    let src = "function f() {\n  return 1;\n}\nconst g = f => f();\n";
    check(&BOTH, vec![("C-M4", src, "f", 2, NC)]);
}

#[test]
fn c122_default_parameter_sees_a_sibling_default() {
    // T3: the parameter environment holds every parameter, so a default expression can see a
    // sibling parameter's binding, which the site walk resolves at the `formal_parameters`
    // scope (a parameter is never callable).
    let src = "function g(f, h = f) {\n  return h;\n}\n";
    check(&BOTH, vec![("C122", src, "f", 1, NC)]);
}

#[test]
fn c4_implicit_arguments_drops_global_keeps() {
    let global_arrow = "const arguments = 1;\nconst g = () => arguments;\n";
    let function_arguments = "function g() {\n  return arguments.length;\n}\n";
    check(
        &BOTH,
        vec![
            (
                "C88 arrow",
                global_arrow,
                "arguments",
                1,
                callable_or_alias(),
            ),
            ("C113 function", function_arguments, "arguments", 0, NC),
        ],
    );
}

/// `const arguments = 1;` binds a value alias literal (`1` is NoFn), so `not_callable`.
fn callable_or_alias() -> JsBinding {
    NC
}

#[test]
fn c_m8_d7_taint_marks_a_catch_shadowed_hoisted_var() {
    // Annex B.3.4 (D7): a `var` hoisted through a `catch` clause whose parameter binds the
    // same name is a non-callable marker, not a real declaration, even though its own value is
    // a proper arrow function (C-M8: without the taint, this would wrongly verify).
    let src = "try {} catch (f) {\n  var f = () => 1;\n}\nf();\n";
    check(&BOTH, vec![("C-M8", src, "f", 2, NC)]);
}

#[test]
fn c92_catch_for_of_class_with_shadow_and_drop() {
    let f = "function f() {\n  return 1;\n}\n";
    let catch = format!("{f}try {{}} catch (f) {{\n  f();\n}}\n");
    let for_of = format!("{f}for (const f of xs) {{\n  f();\n}}\n");
    let class = format!("{f}class f {{\n  m() {{\n    f();\n  }}\n}}\n");
    let with = format!("{f}with (o) {{\n  f();\n}}\n");
    let rows = vec![
        ("C92 catch", catch.as_str(), "f", 1, NC),
        ("C92 for-of", for_of.as_str(), "f", 1, NC),
        ("C92 class", class.as_str(), "f", 2, NC),
    ];
    check(&BOTH, rows);
    check(
        &BOTH,
        vec![(
            "C92 with",
            with.as_str(),
            "f",
            1,
            JsBinding::Refused("with"),
        )],
    );
}

#[test]
fn c93_c111_block_function_inside_and_outside() {
    let inside = "function run() {\n  if (true) {\n    function f() {\n      return 1;\n    }\n\
        return f();\n  }\n}\n";
    let outside_script = "function f() {\n  return 1;\n}\nfunction run() {\n  if (true) {\n    \
        function f() {\n      return 2;\n    }\n  }\n  return f();\n}\n";
    let outside_module = format!("export {{}};\n{outside_script}");
    check(
        &BOTH,
        vec![("C93 inside", inside, "f", 1, callable("f", 3, 5))],
    );
    // Unknown strictness (a script with no directive): unproven, base behavior (Option K).
    check(
        &BOTH,
        vec![(
            "C93 outside script",
            outside_script,
            "f",
            2,
            JsBinding::Unchecked("annex_b_strictness"),
        )],
    );
    // C111 (r3, sol W1): a module is strict, so the block function is not an Annex-B marker;
    // the outer `f` binds.
    check(
        &BOTH,
        vec![(
            "C111 outside module",
            &outside_module,
            "f",
            2,
            callable("f", 2, 4),
        )],
    );
}

#[test]
fn c115_c116_static_block_var_scope() {
    let callable_var = "class C {\n  static {\n    var f = () => 1;\n    f();\n  }\n}\n";
    let written_var = "class C {\n  static {\n    var f = 1;\n    f = 2;\n    f();\n  }\n}\n";
    check(
        &BOTH,
        vec![("C115", callable_var, "f", 1, callable("f", 3, 3))],
    );
    check(&BOTH, vec![("C116", written_var, "f", 2, MAY)]);
}

#[test]
fn c135_switch_case_lexical_scope() {
    let src = "switch (x) {\n  case 1: {\n    let f = () => 1;\n    f();\n  }\n  case 2: {\n    \
        let f = () => 2;\n    f();\n  }\n}\n";
    check(
        &BOTH,
        vec![
            ("C135 case 1", src, "f", 1, callable("f", 3, 3)),
            ("C135 case 2", src, "f", 3, callable("f", 7, 7)),
        ],
    );
}

#[test]
fn c38_m2_any_declaration_kind_kept_written() {
    // Owner M2 ruling (SPEC §3.8 (3), C169): a written class, `for (var …)` head or parameter
    // keeps base (`MayCall`); its unwritten twin stays `not_callable`.
    let written_class = "class f {}\nf = 2;\nf();\n";
    let unwritten_class = "class f {}\nf();\n";
    let written_for = "for (var f in o) {\n  f = 2;\n}\nf();\n";
    let unwritten_for = "for (var f in o) {}\nf();\n";
    let written_param = "function g(f) {\n  f = 2;\n  f();\n}\n";
    let unwritten_param = "function g(f) {\n  f();\n}\n";
    check(
        &BOTH,
        vec![
            ("C169 class", written_class, "f", 2, MAY),
            ("class unwritten", unwritten_class, "f", 1, NC),
            ("C169 for", written_for, "f", 1, MAY),
            ("C136 for unwritten", unwritten_for, "f", 1, NC),
            ("C169 param", written_param, "f", 1, MAY),
            ("C87 param unwritten", unwritten_param, "f", 0, NC),
        ],
    );
}

#[test]
fn c_fold_default_bearing_nofn_pattern_shadows_at_a_call_site() {
    // Fold C (C-M43): the same default-bearing NoFn guard through the site-walk entry point
    // (`js_ts_site_binding`), not just the module terminal: a nested default-bearing pattern
    // with a NoFn value shadows an outer same-named function as an alias (keeps base), not
    // `not_callable`.
    let src = "function f() {\n  return 1;\n}\nfunction holder() {\n  const { f = 0 } = {};\n  \
        return f();\n}\n";
    check(
        &BOTH,
        vec![("C-fold-C site", src, "f", 1, JsBinding::Alias)],
    );
}

#[test]
fn c46_c178_parameter_list_error_seals_the_whole_function() {
    // SPEC §3.8 (6), spec r1 Opus W2: a `formal_parameters` error seals only when the site is
    // outside the whole sealer (parameters are visible from the body). A reference to an
    // outer, otherwise-clean name from inside a function whose own parameter list is broken is
    // refused, even though the site is nowhere near the parameter list itself (mutant C-M41,
    // containment tested against just the parameter list, would wrongly verify since the site
    // sits in the body).
    let src = "function g(a, x ==) {\n  return outer();\n}\nfunction outer() {\n  return 1;\n}\n\
        export { outer };\n";
    check(
        &BOTH,
        vec![(
            "C178",
            src,
            "outer",
            0,
            JsBinding::Refused("parse_recovery"),
        )],
    );
}

#[test]
fn b_t3_shared_binding_writes_cross_the_formal_parameters_function_split() {
    // Fold B (gpt-5.6-sol W1, gpt-6.1-sol W1): a `formal_parameters` node (T3) and its owning
    // function (T2) index the same shared binding (a parameter, `arguments`, a named function
    // expression's own name) at two distinct node ids; a write's resolved scope and a query's
    // resolved scope must be compared canonically, not by raw id, in both directions.
    let default_writes_body =
        "function target() {\n  return 1;\n}\nfunction g(f = (f = target)) {\n  f();\n}\n";
    let body_writes_default =
        "function target() {\n  return 1;\n}\nfunction g(f, h = (f = 2)) {\n  f();\n}\n";
    let named_fe_default_writes_body =
        "function target() {\n  return 1;\n}\nconst q = function f(x = (f = target)) {\n  \
        f();\n};\n";
    let arguments_default_writes_body =
        "function g(a = (arguments = 1)) {\n  return arguments;\n}\n";
    // Negative: inner's own default-write must not leak into outer's distinct `f` parameter.
    let shadow_negative = "function outer(f) {\n  function inner(f = (f = 1)) {\n    return 1;\n  \
        }\n  return f;\n}\n";
    check(
        &BOTH,
        vec![
            ("Bold-1", default_writes_body, "f", 2, MAY),
            ("Bold-2", body_writes_default, "f", 2, MAY),
            ("Bold-3", named_fe_default_writes_body, "f", 2, MAY),
            ("Bold-4", arguments_default_writes_body, "arguments", 1, MAY),
            ("Bold-5 shadow negative", shadow_negative, "f", 3, NC),
        ],
    );
}

#[test]
fn e_fold_escaped_class_name_bypasses_b0() {
    // Fold E (gpt-6.1-sol r1 W2): a class's own inner name (D2/T8) is a `type_identifier`,
    // which B0's escape check (`js_binding_checks.rs`) missed; an escaped spelling there must
    // refuse the scope, same as an escaped `identifier`/pattern, not silently fall through to
    // an outer same-named function.
    let escaped = "function C() {\n  return 1;\n}\nconst x = class \\u0043 {\n  m() {\n    \
        C();\n  }\n};\n";
    let unescaped_control =
        "function C() {\n  return 1;\n}\nconst x = class C {\n  m() {\n    C();\n  }\n};\n";
    // E-2 (unescaped control): the class's own inner name is a real declaration in its own
    // scope (T8) and correctly shadows the outer function; a class is never callable (D2), so
    // the call refuses `not_callable` — never `Callable(outer C)`, the escaped bug's answer.
    check(
        &BOTH,
        vec![
            (
                "E-1 escaped",
                escaped,
                "C",
                1,
                JsBinding::Refused("escaped_identifier"),
            ),
            ("E-2 unescaped control", unescaped_control, "C", 2, NC),
        ],
    );
}
