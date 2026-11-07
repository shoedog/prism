//! #660 S1a: reads that run later than the owner's straight-line code. A nested callable's
//! parameter list, an instance field initializer and the innermost-region belt are `NameOnly(CfgIncomplete)`
//! for JS/TS/TSX; parameter decorators, computed keys, static blocks and static fields are eager and keep
//! their label. Label-only: the DataFlow edge set must not move. Python, Go and Rust keep main's rule.
use super::*;
use crate::ast::ParsedFile;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::BTreeMap;

const JS: &[(Language, &str)] = &[(Language::JavaScript, "js")];
const ALL_JS: &[(Language, &str)] = &[
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];
const TS_ONLY: &[(Language, &str)] = &[(Language::TypeScript, "ts"), (Language::Tsx, "tsx")];
const PY: &[(Language, &str)] = &[(Language::Python, "py")];
const GO: &[(Language, &str)] = &[(Language::Go, "go")];

/// Every label row, `owner:from@line->to@line:label`, sorted; plus the edge count.
fn rows(language: Language, ext: &str, source: &str) -> Vec<String> {
    let name = format!("a.{ext}");
    let file = ParsedFile::parse(&name, source, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{language:?}: {source}");
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(name, file)]));
    let mut rows: Vec<String> = label_snapshot(&cpg.dfg)
        .into_iter()
        .map(|((from, to), label)| {
            format!(
                "{}:{}@{}->{}@{}:{label:?}",
                from.function, from.path, from.line, to.path, to.line
            )
        })
        .collect();
    rows.sort();
    rows
}

fn label_name(token: &str) -> &'static str {
    match token {
        "exact" => "Exact",
        "cfg_incomplete" => "NameOnly(CfgIncomplete)",
        "same_line" => "NameOnly(SameLine)",
        other => panic!("unknown label token {other}"),
    }
}

/// `edge` is `owner:from@line->to@line`; the row must exist with exactly the expected label.
fn check(langs: &[(Language, &str)], src: &str, edge: &str, expected: &str) {
    for &(language, ext) in langs {
        let all = rows(language, ext, src);
        let want = format!("{edge}:{}", label_name(expected));
        let seen: Vec<&String> = all
            .iter()
            .filter(|r| r.starts_with(&format!("{edge}:")))
            .collect();
        assert!(
            all.contains(&want),
            "{language:?}: want {want}; edge rows {seen:?}; all {all:?}"
        );
    }
}

macro_rules! row {
    ($name:ident, $langs:expr, $src:expr, $edge:expr, $expected:expr) => {
        #[test]
        fn $name() {
            check($langs, $src, $edge, $expected);
        }
    };
}

// ---- FIX rows: JS/TS/TSX move from exact to NameOnly(CfgIncomplete) --------------------------------
row!(
    m32_function_expression_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function (z = y) { return z; });\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_arrow_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g((z = y) => z);\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_object_method_default,
    ALL_JS,
    "function f(y) {\n  log();\n  const o = { m(z = y) { return z; } };\n  return o;\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_class_method_default,
    ALL_JS,
    "function f(y) {\n  log();\n  const C = class { m(z = y) { return z; } };\n  return C;\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_generator_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function* (z = y) { yield z; });\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_async_arrow_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g(async (z = y) => z);\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_async_method_default,
    ALL_JS,
    "function f(y) {\n  log();\n  const o = { async m(z = y) { return z; } };\n  return o;\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
// A nested function or class *declaration* line is not a CFG node, so main already labels these
// CfgIncomplete: pinned so the new rule cannot promote them.
row!(
    nested_function_declaration_default,
    ALL_JS,
    "function f(y) {\n  log();\n  function g(z = y) { return z; }\n  return g;\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    nested_destructured_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function ({ a = y }) { return a; });\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    f01_instance_field_after_write,
    ALL_JS,
    "function f(a) {\n  const C = class {\n    x = a;\n  };\n  a = 2;\n  return new C();\n}\n",
    "f:a@1->a@3",
    "cfg_incomplete"
);
row!(
    f03_instance_field_no_write,
    ALL_JS,
    "function f(a) {\n  const C = class {\n    x = a;\n  };\n  return new C();\n}\n",
    "f:a@1->a@3",
    "cfg_incomplete"
);
row!(
    instance_field_in_class_declaration,
    ALL_JS,
    "function f(a) {\n  log();\n  class C {\n    x = a;\n  }\n  a = 2;\n  return new C();\n}\n",
    "f:a@1->a@4",
    "cfg_incomplete"
);
row!(
    instance_private_field,
    JS,
    "function f(a) {\n  log();\n  const C = class {\n    #x = a;\n  };\n  return C;\n}\n",
    "f:a@1->a@4",
    "cfg_incomplete"
);
row!(
    callable_with_default_inside_instance_field,
    ALL_JS,
    "function f(a) {\n  log();\n  const C = class {\n    x = function (b = a) { return b; };\n  };\n  return C;\n}\n",
    "f:a@1->a@4",
    "cfg_incomplete"
);
row!(
    h05_cross_region_def_use,
    ALL_JS,
    "function f(a) {\n  log();\n  g(() => { a = 2; });\n  h(() => { sink(a); });\n}\n",
    "f:a@3->a@4",
    "cfg_incomplete"
);
// ---- Controls: unchanged on main and on head ------------------------------------------------------
row!(
    s11_one_node_nested_default,
    ALL_JS,
    "function h(y) {\n  return g(function (z = y) { return z; });\n}\n",
    "h:y@1->y@2",
    "cfg_incomplete"
);
row!(
    f02_static_field_eager,
    ALL_JS,
    "function f(a) {\n  const C = class {\n    static x = a;\n  };\n  a = 2;\n  return C;\n}\n",
    "f:a@1->a@3",
    "exact"
);
row!(
    a13_body_local_write_control,
    ALL_JS,
    "function f(a) {\n  log();\n  (function () { let a = 0; a++; })(); sink(a);\n}\n",
    "f:a@1->a@3",
    "cfg_incomplete"
);
row!(
    computed_method_key_is_eager,
    ALL_JS,
    "function f(y) {\n  log();\n  const o = { [y]() { return 1; } };\n  return o;\n}\n",
    "f:y@1->y@3",
    "exact"
);
row!(
    computed_class_method_key_is_eager,
    ALL_JS,
    "function f(y) {\n  log();\n  const C = class { [y]() { return 1; } };\n  return C;\n}\n",
    "f:y@1->y@3",
    "exact"
);
row!(
    static_block_is_eager,
    ALL_JS,
    "function f(y) {\n  log();\n  const C = class {\n    static { use(y); }\n  };\n  return C;\n}\n",
    "f:y@1->y@4",
    "exact"
);
row!(
    parameter_decorator_is_eager,
    TS_ONLY,
    "function f(y) {\n  log();\n  const C = class {\n    constructor(@dec(y) p) {}\n  };\n  return C;\n}\n",
    "f:y@1->y@4",
    "exact"
);
row!(
    method_decorator_is_eager,
    TS_ONLY,
    "function f(y) {\n  log();\n  const C = class {\n    @dec(y)\n    m() {}\n  };\n  return C;\n}\n",
    "f:y@1->y@4",
    "exact"
);
const SPLIT: &str = "function f(y, w) {\n  log();\n  const C = class {\n    constructor(\n      @dec(y) p,\n      q = w\n    ) {}\n  };\n  return C;\n}\n";
row!(
    parameter_decorator_beside_a_default_decorator_is_eager,
    TS_ONLY,
    SPLIT,
    "f:y@1->y@5",
    "exact"
);
row!(
    parameter_decorator_beside_a_default_default_is_deferred,
    TS_ONLY,
    SPLIT,
    "f:w@1->w@6",
    "cfg_incomplete"
);
row!(
    def_and_use_in_the_same_parameter_default,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function (\n    z = (a = 1,\n      a)\n  ) { return z; });\n}\n",
    "f:a@4->a@5",
    "same_line"
);
row!(
    def_and_use_in_the_same_nested_body,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function () {\n    a = 1;\n    use(a);\n  });\n}\n",
    "f:a@4->a@5",
    "same_line"
);
row!(
    def_and_use_in_the_same_field_initializer,
    ALL_JS,
    "function f(y) {\n  log();\n  const C = class {\n    x = (a = 1,\n      a);\n  };\n  return C;\n}\n",
    "f:a@4->a@5",
    "same_line"
);
row!(
    def_in_parameter_list_use_in_body_of_the_same_callable,
    ALL_JS,
    "function f(a) {\n  log();\n  g(function (z = (a = 1)) {\n    sink(a);\n  });\n}\n",
    "f:a@3->a@4",
    "cfg_incomplete"
);
row!(
    nested_body_read_is_still_a_capture,
    ALL_JS,
    "function f(y) {\n  log();\n  g(function (z) { return y; });\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    no_region_is_still_exact,
    ALL_JS,
    "function f(y) {\n  log();\n  g(y);\n}\n",
    "f:y@1->y@3",
    "exact"
);

// The capture rule does not fire (the def's region holds the use's); S1c's D2-c does: the write in the
// inner callable may run after any creation of it, before the read outside it.
row!(
    def_in_inner_callable_use_in_enclosing_body_is_refused_by_d2c_not_capture,
    ALL_JS,
    "function f(a) {\n  log();\n  g(() => {\n    h(() => { a = 2; });\n    sink(a);\n  });\n}\n",
    "f:a@4->a@5",
    "cfg_incomplete"
);
row!(
    use_in_inner_callable_def_in_enclosing_body_is_a_capture,
    ALL_JS,
    "function f(a) {\n  log();\n  g(() => {\n    a = 2;\n    h(() => { sink(a); });\n  });\n}\n",
    "f:a@4->a@5",
    "cfg_incomplete"
);

// ---- Other languages: main's rule, byte for byte --------------------------------------------------
row!(
    python_lambda_default_is_eager,
    PY,
    "def f(y):\n    log()\n    g(lambda z=y: z)\n",
    "f:y@1->y@3",
    "exact"
);
row!(
    python_def_default_matches_main,
    PY,
    "def f(y):\n    log()\n    def g(z=y):\n        return z\n    return g\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    python_nested_body_read_is_a_capture,
    PY,
    "def f(y):\n    log()\n    def g(z):\n        return y\n    return g\n",
    "f:y@1->y@4",
    "cfg_incomplete"
);
row!(
    go_func_literal_body_read_is_a_capture,
    GO,
    "package p\n\nfunc f(y int) {\n\tlog()\n\tg(func(z int) int { return z + y })\n}\n",
    "f:y@3->y@5",
    "cfg_incomplete"
);

/// Label-only: the DataFlow edge set (labels ignored) of a fixture exercising every new rule is pinned, and
/// is the same on main (the rows below were recorded on 31703c86 before the change).
#[test]
fn the_edge_set_does_not_move() {
    let src = "function f(y, w) {\n  log();\n  g(function (z = y) { return z + w; });\n  const C = class {\n    x = y;\n    static s = w;\n    constructor(q = w) {}\n  };\n  h(() => { y = 2; });\n  k(() => { sink(y); });\n  return new C();\n}\n";
    for &(language, ext) in ALL_JS {
        let edges: Vec<String> = rows(language, ext, src)
            .iter()
            .map(|r| r.rsplit_once(':').unwrap().0.to_string())
            .collect();
        assert_eq!(edges, EDGES, "{language:?}");
    }
}

const EDGES: [&str; 12] = [
    "f:C@4->C@11",
    "f:w@1->w@3",
    "f:w@1->w@6",
    "f:w@1->w@7",
    "f:y@1->y@10",
    "f:y@1->y@3",
    "f:y@1->y@5",
    "f:y@1->y@9",
    "f:y@9->y@10",
    "f:y@9->y@1",
    "f:y@9->y@3",
    "f:y@9->y@5",
];

// ---- Round 1 fold: one identity for region reads, whatever wraps the receiver ------------------------
// A DFG use is found by its identifier whatever expression carries it (`box(y).x`, `(o).x`, `o!.x`,
// `o[0].x`, `(o as any).x`), so the region map must find the same read.
const BOX_PARAM: &str =
    "function f(y) {\n  log();\n  g(function (z = box(y).x) { return z; });\n}\n";
const BOX_FIELD: &str =
    "function f(y) {\n  log();\n  const C = class {\n    x = box(y).value;\n  };\n  return C;\n}\n";
const AS_FIELD_AFTER_WRITE: &str = "function f(o) {\n  log();\n  o.x = 1;\n  const C = class {\n    x = (o as any).x;\n  };\n  return C;\n}\n";
const AS_DEFAULT_AFTER_WRITE: &str =
    "function f(o) {\n  log();\n  o.x = 1;\n  g((z = (o as any).x) => z);\n}\n";
row!(
    call_receiver_in_parameter_default,
    ALL_JS,
    BOX_PARAM,
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    call_receiver_in_instance_field,
    ALL_JS,
    BOX_FIELD,
    "f:y@1->y@4",
    "cfg_incomplete"
);
row!(
    asserted_member_composite_edge_in_instance_field,
    TS_ONLY,
    AS_FIELD_AFTER_WRITE,
    "f:o.x@3->o.x@5",
    "cfg_incomplete"
);
row!(
    asserted_member_base_edge_in_instance_field,
    TS_ONLY,
    AS_FIELD_AFTER_WRITE,
    "f:o@1->o@5",
    "cfg_incomplete"
);
row!(
    asserted_member_composite_edge_in_parameter_default,
    TS_ONLY,
    AS_DEFAULT_AFTER_WRITE,
    "f:o.x@3->o.x@4",
    "cfg_incomplete"
);
row!(
    non_null_receiver_in_parameter_default,
    TS_ONLY,
    "function f(o) {\n  log();\n  g((z = o!.x) => z);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);
row!(
    asserted_receiver_in_instance_field,
    TS_ONLY,
    "function f(o) {\n  log();\n  const C = class {\n    v = (o as any).y;\n  };\n  return C;\n}\n",
    "f:o@1->o@4",
    "cfg_incomplete"
);
row!(
    parenthesized_receiver_in_parameter_default,
    ALL_JS,
    "function f(o) {\n  log();\n  g((z = (o).x) => z);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);
row!(
    subscript_receiver_in_parameter_default,
    ALL_JS,
    "function f(o) {\n  log();\n  g((z = o[0].x) => z);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);
row!(
    parenthesized_receiver_in_instance_field,
    ALL_JS,
    "function f(o) {\n  log();\n  const C = class {\n    v = (o).y;\n  };\n  return C;\n}\n",
    "f:o@1->o@4",
    "cfg_incomplete"
);
// The same gap in nested bodies is inherited from main (capture-body in the trace).
row!(
    parenthesized_receiver_in_nested_body,
    ALL_JS,
    "function f(o) {\n  log();\n  g(() => (o).x);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);
row!(
    call_receiver_in_nested_body,
    ALL_JS,
    "function f(y) {\n  log();\n  g(() => box(y).x);\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
// Eager equivalents keep their label.
row!(
    static_asserted_member_composite_is_eager,
    TS_ONLY,
    "function f(o) {\n  log();\n  o.x = 1;\n  const C = class {\n    static x = (o as any).x;\n  };\n  return C;\n}\n",
    "f:o.x@3->o.x@5",
    "exact"
);
row!(
    static_call_receiver_is_eager,
    ALL_JS,
    "function f(y) {\n  log();\n  const C = class {\n    static x = box(y).value;\n  };\n  return C;\n}\n",
    "f:y@1->y@4",
    "exact"
);
row!(
    no_region_member_chain_is_exact,
    ALL_JS,
    "function f(y) {\n  log();\n  return box(y).x;\n}\n",
    "f:y@1->y@3",
    "exact"
);
// Branches the first round left without a row; already NameOnly on c137b212, pinned.
row!(
    callable_inside_a_parameter_decorator,
    TS_ONLY,
    "function f(y) {\n  log();\n  const C = class {\n    constructor(@dec(() => y) p) {}\n  };\n  return C;\n}\n",
    "f:y@1->y@4",
    "cfg_incomplete"
);
row!(
    member_base_read_in_parameter_default,
    ALL_JS,
    "function f(o) {\n  use(o);\n  g((z = o.m()) => z);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);
row!(
    member_base_read_in_instance_field,
    ALL_JS,
    "function f(o) {\n  use(o);\n  const C = class {\n    v = o.k;\n  };\n  return C;\n}\n",
    "f:o@1->o@4",
    "cfg_incomplete"
);
// The capture check precedes every other doubt: a killed pair becomes CfgIncomplete, as main does for
// bodies. Labels move toward NameOnly(CfgIncomplete) from exact, sameline or another NameOnly doubt;
// no NameOnly -> Exact move exists.
row!(
    killed_pair_into_a_parameter_default_becomes_cfg_incomplete,
    ALL_JS,
    "function f(a) {\n  log();\n  a = 2;\n  g((z = a) => z);\n}\n",
    "f:a@1->a@4",
    "cfg_incomplete"
);
// Not reads: an object key, and a name in a type position. An eager read of the same name on the line stays exact.
row!(
    object_key_in_a_field_is_not_a_read,
    ALL_JS,
    "function f(x) {\n  log();\n  const C = class { v = { x: 1 }; }; use(x);\n}\n",
    "f:x@1->x@3",
    "exact"
);
row!(
    type_query_in_a_parameter_is_not_a_read,
    TS_ONLY,
    "function f(o) {\n  log();\n  use(o); g((z: typeof o) => 1);\n}\n",
    "f:o@1->o@3",
    "exact"
);

// ---- Post-cap fix: formal bindings are not reads; a member use is placed by its full path first ----
// Hand-check of cloud r2 (PR #683 head 7a6766f8): an eager read beside a nested formal of the same
// name was refused (`a.then((a) => ..)`), and an eager `this.installer` beside an arrow reading
// `this.handle` was placed in the arrow by the base name `this`.
row!(
    shadowing_formal_is_not_a_read,
    ALL_JS,
    "function f(a) {\n  log();\n  a.then((a) => {\n    use(1);\n  });\n}\n",
    "f:a@1->a@3",
    "exact"
);
row!(
    lone_arrow_parameter_is_not_a_read,
    ALL_JS,
    "function f(x) {\n  log();\n  x.then(x => 1);\n}\n",
    "f:x@1->x@3",
    "exact"
);
row!(
    shadowing_formal_after_an_assignment_target,
    ALL_JS,
    "function f(host) {\n  log();\n  host.createProgram = (n, host) => {\n    use(n);\n  };\n}\n",
    "f:host@1->host@3",
    "exact"
);
row!(
    shadowing_formal_after_an_optional_receiver,
    ALL_JS,
    "function f(a) {\n  log();\n  a?.forEach(a => {\n    use(a);\n  });\n}\n",
    "f:a@1->a@3",
    "exact"
);
row!(
    shadowing_formal_beside_another_formal,
    ALL_JS,
    "function f(p) {\n  log();\n  p.forEach((p, k) => {\n    use(k);\n  });\n}\n",
    "f:p@1->p@3",
    "exact"
);
row!(
    function_expression_formal_and_own_name,
    ALL_JS,
    "function f(n) {\n  log();\n  n.forEachChild(function cb(n) {\n    use(1);\n  });\n}\n",
    "f:n@1->n@3",
    "exact"
);
row!(
    function_expression_own_name_is_not_a_read,
    ALL_JS,
    "function f(n) {\n  log();\n  n.go(function n() {\n    use(1);\n  });\n}\n",
    "f:n@1->n@3",
    "exact"
);
row!(
    rest_formal_is_not_a_read,
    ALL_JS,
    "function f(a) {\n  log();\n  a.go((...a) => {\n    use(1);\n  });\n}\n",
    "f:a@1->a@3",
    "exact"
);
row!(
    destructured_formals_are_not_reads,
    ALL_JS,
    "function f(a) {\n  log();\n  a.go(({ a, k: [b] }, { c = 1 }) => {\n    use(1);\n  });\n}\n",
    "f:a@1->a.go@3",
    "exact"
);
row!(
    typed_and_optional_formals_are_not_reads,
    TS_ONLY,
    "function f(a: any) {\n  log();\n  a.go((a: number, b?: string) => {\n    use(1);\n  });\n}\n",
    "f:a@1->a@3",
    "exact"
);
// Controls: a default value and a computed key inside a parameter pattern stay reads.
row!(
    default_value_still_refused_beside_a_shadowing_formal,
    ALL_JS,
    "function f(x, y) {\n  log();\n  x.then((x = y) => 1);\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
row!(
    computed_key_in_a_parameter_pattern_is_still_a_read,
    ALL_JS,
    "function f(y) {\n  log();\n  g(({ [y]: v }) => 1);\n}\n",
    "f:y@1->y@3",
    "cfg_incomplete"
);
// C7. Hypothesis (written before the first run on 7a6766f8): the eager use `this.installer` is a
// composite path no region spells on line 4, so the base-name fallback finds the token `this` in the
// arrow and places the use in the arrow's region; the def on line 3 is not in it, so the edge is
// cfg_incomplete on 7a6766f8 and exact on main. It is falsified if the first row is exact on
// 7a6766f8, or if the second (the arrow does not mention `this`) is also cfg_incomplete there.
const C7_THIS: &str = "class A {\n  start() {\n    this.installer = fork();\n    this.installer.on(\"message\", m => this.handle(m));\n  }\n}\n";
const C7_NO_THIS: &str = "class A {\n  start() {\n    this.installer = fork();\n    this.installer.on(\"message\", m => handle(m));\n  }\n}\n";
row!(
    c7_eager_this_member_beside_an_arrow_reading_another_member,
    ALL_JS,
    C7_THIS,
    "start:this.installer@3->this.installer@4",
    "exact"
);
row!(
    c7_arrow_that_does_not_mention_this,
    ALL_JS,
    C7_NO_THIS,
    "start:this.installer@3->this.installer@4",
    "exact"
);
row!(
    eager_member_beside_an_arrow_reading_its_base,
    ALL_JS,
    "function f(o) {\n  log();\n  o.inst = 1;\n  o.inst.on(1, m => o.go(m));\n}\n",
    "f:o.inst@3->o.inst@4",
    "exact"
);
row!(
    eager_member_beside_a_region_read_of_its_bare_base,
    ALL_JS,
    "function f(o) {\n  log();\n  o.x = 1;\n  use(o.x); g(() => o);\n}\n",
    "f:o.x@3->o.x@4",
    "exact"
);
row!(
    same_name_eager_and_in_a_region_on_one_line_fails_toward_capture,
    ALL_JS,
    "function f(o) {\n  log();\n  use(o); g(() => o);\n}\n",
    "f:o@1->o@3",
    "cfg_incomplete"
);

// Mutant survivors of the cloud probes (round 3).
// A type-position read (`typeof x`, issue #684: main's DFG emits an `exact` edge into it, which is #684's to
// remove) with no other read of the name on the line: a region read only if the collector wrongly treats the
// type position as a read. Expected on head: the edge keeps main's label, exact.
row!(
    typeof_in_a_nested_parameter_annotation_is_not_a_region_read,
    TS_ONLY,
    "function f(x) {\n  log();\n  g((z: typeof x) => z);\n}\n",
    "f:x@1->x@3",
    "exact"
);
// A callable inside a TS parameter decorator, with its def inside it and the use in a default of the same
// list on its own line. Expected on head: the use is in the list's parameter region, the def is not in it
// (the callable is a region of its own, parented outside the list), so cfg_incomplete. The probe that walks
// decorator parts with the parameter context is equivalent (see capture_regions_tests).
row!(
    def_in_a_parameter_decorator_callable_use_in_a_default_of_the_same_list,
    TS_ONLY,
    "function f(a) {\n  log();\n  const C = class {\n    constructor(\n      @dec(() => { a = 2; }) p,\n      q = a,\n    ) {}\n  };\n  return C;\n}\n",
    "f:a@5->a@6",
    "cfg_incomplete"
);
