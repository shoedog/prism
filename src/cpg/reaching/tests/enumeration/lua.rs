use super::case::{Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

pub(super) const CASES: &[Case] = &[
    Case {
        id: "lua-local-masks",
        language: Language::Lua,
        kind: "block",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  do\n    local x = clean()\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "e4e-1-lua-variable_declaration",
        language: Language::Lua,
        kind: "variable_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  do\n    local x = clean(x)\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            ("2:x", "5:x", killed(4)),
            ("2:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-assignment-reuse-bypass",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  if c then\n    x = clean()\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-local-assignment-not-reuse",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: Some("binding"),
        row_variant: Some(super::super::super::binding_table::Predicate::ParentKindIs {
            kind: "variable_declaration",
        }),
        class: CaseClass::Structural,
        src: "function f()\n  local x = source()\nend\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "lua-for-clause-header",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for x = 1, 3 do\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "4:x", killed(3)), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-for-generic-header",
        language: Language::Lua,
        kind: "for_generic_clause",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(items)\n  x = source()\n  for key, x in pairs(items) do\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "4:x", killed(3)), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-for-generic-initializer-uses-outer",
        language: Language::Lua,
        kind: "for_generic_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for key, x in iter(x) do\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "3:x", FlowConfidence::Exact),
            ("2:x", "4:x", killed(3)),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-for-numeric-multiline-initializers-use-outer",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for x =\n    start(x),\n    limit(x),\n    step(x)\n  do\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            ("2:x", "5:x", FlowConfidence::Exact),
            ("2:x", "6:x", FlowConfidence::Exact),
            ("2:x", "8:x", killed(3)),
            ("2:x", "10:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-nested-loop-initializers-use-enclosing-binding",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for i = first(), 1 do\n    sink(x)\n    for x = inner(x), 1 do\n      sink(x)\n    end\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            ("2:x", "5:x", FlowConfidence::Exact),
            ("2:x", "6:x", killed(5)),
            ("2:x", "8:x", FlowConfidence::Exact),
            ("2:x", "10:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "e0b-lua-function_declaration",
        language: Language::Lua,
        kind: "function_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f()\n  local function callback()\n    return 1\n  end\n  callback()\nend\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "lua-nested-function-body-scope",
        language: Language::Lua,
        kind: "function_definition",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  local callback = function()\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "lua-assignment-every-route-kills",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  observe(x)\n  if c then\n    x = left()\n  else\n    x = right()\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "3:x", FlowConfidence::Exact), ("2:x", "9:x", killed(5))],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-do-block-reassign-kills",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  observe(x)\n  do\n    x = clean()\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "3:x", FlowConfidence::Exact), ("2:x", "7:x", killed(5))],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-repeat-reassign-kills",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  observe(x)\n  repeat\n    x = clean()\n  until done()\n  sink(x)\nend\n",
        expect: &[("2:x", "3:x", FlowConfidence::Exact), ("2:x", "7:x", killed(5))],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-local-no-value",
        language: Language::Lua,
        kind: "variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  do\n    local x\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-local-multi-name",
        language: Language::Lua,
        kind: "variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  do\n    local a, x = pair()\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-while-body-local-dies",
        language: Language::Lua,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  while c do\n    local x = clean()\n    sink(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-repeat-until-sees-body-local",
        language: Language::Lua,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  repeat\n    local x = clean()\n    sink(x)\n  until done(x)\n  sink(x)\nend\n",
        expect: &[
            ("2:x", "5:x", killed(4)),
            ("2:x", "6:x", killed(4)),
            ("2:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
];
pub(super) const CURATED: &[(&str, &str)] = &[("parameters", "e0a-x-lua-parameters")];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "implicit_variable_declaration" => "global <const> *\nfunction f() end\n",
        _ => super::case::source(crate::languages::Language::Lua),
    }
}

#[test]
fn task26_lua_binding_cases() {
    let mut failed = Vec::new();
    for case in CASES {
        if std::panic::catch_unwind(|| super::case::check_case(case)).is_err() {
            failed.push(case.id);
        }
    }
    assert!(failed.is_empty(), "Task 26 case failures: {failed:?}");
}
