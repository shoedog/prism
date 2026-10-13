//! Slice R (owner decision 2026-10-13): a bound name whose value is unknown gets no name search.
//! Each case builds a small repo and pins the FULL record set at one line (origin, target,
//! kind, grade or drop reason): rule 1 (no repo-wide search for a `MayCall` binding), rule 2
//! (a same-file hit must be written into the binding) and rule 3 (the textual function-pointer
//! path respects bindings in JS/TS). The controls pin what must not move.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

fn graph_n(files: &[(&str, &str)]) -> CallGraph {
    let mut parsed = BTreeMap::new();
    for (path, src) in files {
        let lang = Language::from_path(path).unwrap();
        parsed.insert(
            (*path).to_string(),
            ParsedFile::parse(path, src, lang).unwrap(),
        );
    }
    CallGraph::build(&parsed)
}

/// Every record of `caller_file` at `line`, as `callee origin: target kind confidence` (or
/// `drop reason`), sorted.
fn records_at(files: &[(&str, &str)], caller_file: &str, line: u64) -> Vec<String> {
    let cg = graph_n(files);
    let mut out = Vec::new();
    for row in prism::navigation::queries::call_site_dump(&cg) {
        if row["caller"]["file"] != caller_file || row["source_span"]["line"] != line {
            continue;
        }
        let shown = match row["resolved_targets"].as_array().filter(|t| !t.is_empty()) {
            Some(targets) => targets
                .iter()
                .map(|t| {
                    format!(
                        "{}:{}@{}-{} {} {}",
                        t["function_id"]["file"].as_str().unwrap(),
                        t["function_id"]["name"].as_str().unwrap(),
                        t["function_id"]["start_line"],
                        t["function_id"]["end_line"],
                        t["kind"].as_str().unwrap(),
                        t["confidence"].as_str().unwrap()
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
            None => format!("drop {}", row["drop"].as_str().unwrap_or("?")),
        };
        out.push(format!(
            "{} {}: {shown}",
            row["callee_text"].as_str().unwrap(),
            row["origin"].as_str().unwrap()
        ));
    }
    out.sort();
    out
}

const DECOY_H: (&str, &str) = ("decoy.js", "export function h() {\n  return 2;\n}\n");
const DECOY_H2: (&str, &str) = ("decoy2.js", "export function h() {\n  return 3;\n}\n");

// ---- rule 1: no repo-wide name search for a bound name -------------------------------------

/// `h` is bound in the enclosing function; the call is in a nested one, so the caller-locals
/// guard does not see it and the repo-wide rung is what used to bind the decoy.
fn enclosing(decl: &str) -> String {
    format!("export function outer(o) {{\n  {decl}\n  return function inner() {{\n    return h();\n  }};\n}}\n")
}

#[test]
fn rule1_alias_binding_never_binds_a_unique_repo_wide_decoy() {
    let src = enclosing("const h = o.pick;");
    assert_eq!(
        records_at(&[("caller.js", &src), DECOY_H], "caller.js", 4),
        ["h Source: drop LocalBindingUnproven"]
    );
}

#[test]
fn rule1_alias_binding_never_demotes_to_several_repo_wide_decoys() {
    let src = enclosing("const h = o.pick;");
    assert_eq!(
        records_at(&[("caller.js", &src), DECOY_H, DECOY_H2], "caller.js", 4),
        ["h Source: drop LocalBindingUnproven"]
    );
}

#[test]
fn rule1_written_binding_never_binds_a_repo_wide_decoy() {
    let src = enclosing("let h; h = o.pick;");
    for decoys in [vec![DECOY_H], vec![DECOY_H, DECOY_H2]] {
        let mut files = vec![("caller.js", src.as_str())];
        files.extend(decoys);
        assert_eq!(
            records_at(&files, "caller.js", 4),
            ["h Source: drop LocalBindingUnproven"]
        );
    }
}

#[test]
fn rule1_holds_in_typescript_and_jsx_elements() {
    let ts = "export function outer(o: any) {\n  const h = o.pick as any;\n  \
        return function inner() {\n    return h();\n  };\n}\n";
    assert_eq!(
        records_at(
            &[("caller.ts", ts), ("decoy.ts", DECOY_H.1)],
            "caller.ts",
            4
        ),
        ["h Source: drop LocalBindingUnproven"]
    );
    let jsx = "export function Outer(o) {\n  const H = o.pick;\n  \
        return function Inner() {\n    return <H />;\n  };\n}\n";
    let decoy = "export function H() {\n  return null;\n}\n";
    assert_eq!(
        records_at(
            &[("caller.jsx", jsx), ("decoy.jsx", decoy)],
            "caller.jsx",
            4
        ),
        ["H Source: drop LocalBindingUnproven"]
    );
}

#[test]
fn rule1_control_unbound_names_keep_the_repo_wide_rung() {
    // Module and script: no binding at all, so the global question stays as it was.
    let call = "export function run() {\n  return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", call), DECOY_H], "caller.js", 2),
        ["h Source: decoy.js:h@1-3 free_single exact"]
    );
    let script = "function run() {\n  return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", script), DECOY_H], "caller.js", 2),
        ["h Source: decoy.js:h@1-3 free_single exact"]
    );
    assert_eq!(
        records_at(&[("caller.js", call), DECOY_H, DECOY_H2], "caller.js", 2),
        ["h Source: decoy.js:h@1-3 free_multi name_only, decoy2.js:h@1-3 free_multi name_only"]
    );
}

#[test]
fn rule1_control_unproven_positions_keep_the_repo_wide_rung() {
    // A merged namespace body, an enum-merged namespace and a decorator on a class expression
    // are `Position` states: the walk could not decide, and base behavior stays.
    let merged = "namespace N {\n  export const a = 1;\n}\nnamespace N {\n  export function \
        run() {\n    return h();\n  }\n}\n";
    let enum_merged = "enum E {\n  A,\n}\nnamespace E {\n  export function run() {\n    return \
        h();\n  }\n}\n";
    let decorated = "export const K = @dec class Named {\n  m() {\n    return h();\n  }\n};\n";
    for (label, src, line) in [
        ("merged namespace", merged, 6),
        ("enum-merged namespace", enum_merged, 6),
    ] {
        let got = records_at(
            &[("caller.ts", src), ("decoy.ts", DECOY_H.1)],
            "caller.ts",
            line,
        );
        assert!(
            got.iter().any(|r| r.contains("free_single exact")),
            "{label}: {got:?}"
        );
    }
    // The decorator row only needs to keep today's outcome whatever that is: it must not drop
    // with `LocalBindingUnproven`.
    let got = records_at(
        &[("caller.ts", decorated), ("decoy.ts", DECOY_H.1)],
        "caller.ts",
        3,
    );
    assert!(
        !got.iter().any(|r| r.contains("LocalBindingUnproven")),
        "{got:?}"
    );
}

#[test]
fn rule1_control_a_parameter_bound_name_still_drops_as_before() {
    let src = "export function run(h) {\n  return function inner() {\n    return h();\n  };\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src), DECOY_H], "caller.js", 3),
        ["h Source: drop LocalBindingUnproven"]
    );
}

// ---- rule 2: a same-file by-name hit must belong to the binding ----------------------------

const SHADOWED: &str = "function h() {\n  return 0;\n}\n";

#[test]
fn rule2_a_shadowed_same_file_function_is_not_the_aliases_value() {
    let alias =
        format!("{SHADOWED}export function run(o) {{\n  const h = o.pick;\n  return h();\n}}\n");
    assert_eq!(
        records_at(&[("caller.js", &alias)], "caller.js", 6),
        ["h Source: drop LocalBindingUnproven"]
    );
    let destructured =
        format!("{SHADOWED}export function run(o) {{\n  const {{ h }} = o;\n  return h();\n}}\n");
    assert_eq!(
        records_at(&[("caller.js", &destructured)], "caller.js", 6),
        ["h Source: drop LocalBindingUnproven"]
    );
    let reassigned = format!(
        "{SHADOWED}export function run(o) {{\n  let h = o.a;\n  if (o.b) h = o.c;\n  return h();\n}}\n"
    );
    assert_eq!(
        records_at(&[("caller.js", &reassigned)], "caller.js", 7),
        ["h Source: drop LocalBindingUnproven"]
    );
}

#[test]
fn rule2_keep_an_inline_function_in_the_initializer() {
    let src = "export function run() {\n  const h = debounce(() => {\n    return 1;\n  });\n  \
        return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 5),
        ["h Source: caller.js:h@2-4 local_def exact"]
    );
}

#[test]
fn rule2_keep_a_function_expression_nested_in_a_call_argument() {
    // A function expression directly in the call argument, and a pair-named arrow nested one
    // level deeper inside an object argument: both lie inside the declaration's initializer.
    let direct =
        "export function run() {\n  const h = wrap(function () {\n    return 1;\n  });\n  \
        return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", direct)], "caller.js", 5),
        ["h Source: caller.js:h@2-4 local_def exact"]
    );
    let nested = "export function run() {\n  const h = make({\n    h: () => {\n      return 1;\n    },\n  });\n  \
        return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", nested)], "caller.js", 7),
        ["h Source: caller.js:h@3-5 local_def exact"]
    );
}

#[test]
fn rule2_keep_an_inline_function_assigned_later() {
    let src = "export function run() {\n  let h;\n  h = () => {\n    return 1;\n  };\n  \
        return h();\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 6),
        ["h Source: caller.js:h@3-5 local_def exact"]
    );
}

#[test]
fn rule2_keep_only_the_function_written_into_the_binding() {
    // A shadowed `h` and the inline one: only the inline function (inside the initializer) stays.
    let src = format!(
        "{SHADOWED}export function run() {{\n  const h = debounce(() => {{\n    return 1;\n  }});\n  \
         return h();\n}}\n"
    );
    assert_eq!(
        records_at(&[("caller.js", &src)], "caller.js", 8),
        ["h Source: caller.js:h@5-7 local_def exact"]
    );
}

// ---- rule 3: the textual alias path respects bindings in JS/TS ------------------------------

const F_CALLEE: (&str, &str) = ("callee.js", "export function f(x) {\n  return x;\n}\n");

#[test]
fn rule3_an_outer_parameter_right_hand_name_emits_nothing() {
    let src =
        "import { f } from './callee';\nexport function outer(f) {\n  function inner() {\n    \
        const h = f;\n    return h(1);\n  }\n  return inner();\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 5),
        ["h Source: drop UnknownName"]
    );
}

#[test]
fn rule3_an_outer_const_holding_a_non_function_emits_nothing() {
    let src = "const f = 1;\nexport function run() {\n  const h = f;\n  return h(1);\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 4),
        ["h Source: drop UnknownName"]
    );
}

#[test]
fn rule3_the_callers_own_parameter_as_right_hand_name_emits_nothing() {
    let src = "export function run(f) {\n  const h = f;\n  return h(1);\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", src)], "caller.js", 3),
        ["h Source: drop UnknownName"]
    );
}

#[test]
fn rule3_an_alias_name_rebound_by_a_catch_parameter_emits_nothing() {
    let src =
        "function f() {\n  return 0;\n}\nexport function run() {\n  const h = f;\n  try {\n    \
        throw 1;\n  } catch (h) {\n    return h(1);\n  }\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 9),
        ["h Source: drop UnknownName"]
    );
}

#[test]
fn rule3_an_alias_declared_in_a_sibling_block_does_not_reach_a_parameter_read() {
    let src = "function f() {\n  return 0;\n}\nexport function run(h, c) {\n  if (c) {\n    \
        const h = f;\n  }\n  return h(1);\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 8),
        ["h Source: drop UnknownName"]
    );
}

#[test]
fn rule3_keep_a_let_alias_of_an_import_or_a_same_file_function() {
    let import_case = "import { f } from './callee';\nexport function run() {\n  let h = f;\n  \
        return h(1);\n}\n";
    assert_eq!(
        records_at(&[F_CALLEE, ("caller.js", import_case)], "caller.js", 4),
        [
            "f IndirectResolution: callee.js:f@1-3 import_member exact",
            "h Source: drop UnknownName",
        ]
    );
    let same_file = "function f() {\n  return 0;\n}\nexport function run() {\n  let h = f;\n  \
        return h(1);\n}\n";
    assert_eq!(
        records_at(&[("caller.js", same_file)], "caller.js", 6),
        [
            "f IndirectResolution: caller.js:f@1-3 local_def exact",
            "h Source: drop UnknownName",
        ]
    );
}

#[test]
fn rule3_keep_a_reassignment_alias_where_both_names_are_bound() {
    let src = "function f() {\n  return 0;\n}\nexport function run() {\n  let h;\n  h = f;\n  \
        return h(1);\n}\n";
    assert_eq!(
        records_at(&[("caller.js", src)], "caller.js", 7),
        [
            "f IndirectResolution: caller.js:f@1-3 local_def exact",
            "h Source: drop UnknownName",
        ]
    );
}

#[test]
fn rule3_does_not_touch_c_function_pointers() {
    let src =
        "int f(int x) { return x; }\nint run(void) {\n  int (*h)(int) = f;\n  return h(1);\n}\n";
    let got = records_at(&[("a.c", src)], "a.c", 4);
    assert!(
        got.contains(&"f IndirectResolution: a.c:f@1-1 local_def exact".to_string()),
        "{got:?}"
    );
}
