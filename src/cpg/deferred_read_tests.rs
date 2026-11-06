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

row!(
    def_in_inner_callable_use_in_enclosing_body_is_not_a_capture,
    ALL_JS,
    "function f(a) {\n  log();\n  g(() => {\n    h(() => { a = 2; });\n    sink(a);\n  });\n}\n",
    "f:a@4->a@5",
    "same_line"
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
