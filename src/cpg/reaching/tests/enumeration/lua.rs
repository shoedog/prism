use super::case::{Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const fn ownership_uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
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
        id: "lua-r2-repeat-break-before-withholds-mandatory-body",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  repeat\n    if c then break end\n    x = clean()\n    observe(x)\n  until done()\n  sink(x)\nend\n",
        expect: &[("2:x", "6:x", killed(5)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-repeat-break-after-withholds-mandatory-body",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  repeat\n    x = clean()\n    observe(x)\n    if c then break end\n  until done()\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-repeat-nested-do-break-withholds-mandatory-body",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  repeat\n    do\n      if c then break end\n      x = clean()\n      observe(x)\n    end\n  until done()\n  sink(x)\nend\n",
        expect: &[("2:x", "7:x", killed(6)), ("2:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-do-goto-out-withholds-mandatory-body",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  do\n    if c then goto done end\n    x = clean()\n    observe(x)\n  end\n  ::done::\n  sink(x)\nend\n",
        expect: &[("2:x", "6:x", killed(5)), ("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-goto-elsewhere-withholds-do-mandatory-body",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f(c)\n  x = source()\n  if c then goto again end\n  ::again::\n  do\n    x = clean()\n    observe(x)\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "7:x", killed(6)), ("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-safe-nested-do-stays-mandatory",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  observe(x)\n  do\n    do\n      x = clean()\n    end\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "3:x", FlowConfidence::Exact), ("2:x", "9:x", killed(6))],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-break-inside-do-inside-while-structural",
        language: Language::Lua,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f(c)\n  while c do\n    do\n      if c then break end\n    end\n  end\nend\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "lua-r2-ambiguous-masked-header-fails-closed",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f()\n  x = source()\n  for x = 1, 2 do\n    for i = x, x do\n      sink(x)\n    end\n  end\nend\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "lua-r2-single-masked-header-use-stays-killed",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for x = 1, 2 do\n    for i = start(x), 2 do\n      sink(x)\n    end\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "4:x", killed(3)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "lua-r2-split-masked-header-uses-stay-killed",
        language: Language::Lua,
        kind: "for_numeric_clause",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "function f()\n  x = source()\n  for x = 1, 2 do\n    for i =\n      x,\n      x\n    do\n      sink(x)\n    end\n  end\n  sink(x)\nend\n",
        expect: &[("2:x", "5:x", killed(3)), ("2:x", "6:x", killed(3)), ("2:x", "11:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    Case {
        id: "lua-r2-duplicate-read-without-mask-stays-exact",
        language: Language::Lua,
        kind: "assignment_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f()\n  x = source()\n  sink(x, x)\nend\n",
        expect: &[],
        expect_counter: None,
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

#[test]
fn task26_r2_cap_fold_cases() {
    let mut failed = Vec::new();
    for case in CASES.iter().filter(|case| case.id.starts_with("lua-r2-")) {
        if std::panic::catch_unwind(|| super::case::check_case(case)).is_err() {
            failed.push(case.id);
        }
    }
    assert!(failed.is_empty(), "Task 26 R2 case failures: {failed:?}");
}

#[test]
fn task26_r2_ambiguous_masked_lookup_fails_closed() {
    fn zero_width_label(source: &str, use_line: usize) -> FlowConfidence {
        let parsed = super::super::parsed(source, Language::Lua);
        let function = super::super::function(&parsed);
        let defs = super::super::collect_defs(&parsed);
        let definition = defs
            .iter()
            .find(|def| def.path.base == "x" && def.line == 2)
            .expect("outer x definition");
        let mut edge = super::super::edge(&parsed, &function, definition, use_line);
        let anchor = parsed.line_start_byte(use_line);
        edge.to.start_byte = anchor;
        edge.to.end_byte = anchor;
        let (outcome, _) = super::super::run(&parsed, &defs, std::slice::from_ref(&edge));
        let super::super::RdOutcome::Available(result) = outcome else {
            panic!("expected available reaching result, got {outcome:?}");
        };
        result
            .labels
            .get(&(edge.from, edge.to))
            .copied()
            .expect("queried edge label")
    }

    let nested_different = "function f()\n  x = source()\n  for x = 1, 2 do\n    for i = x, x do\n      sink(x)\n    end\n  end\nend\n";
    assert_eq!(
        zero_width_label(nested_different, 4),
        ownership_uncertain(3)
    );

    let nested_single = "function f()\n  x = source()\n  for x = 1, 2 do\n    for i = start(x), 2 do\n      sink(x)\n    end\n  end\nend\n";
    assert_eq!(zero_width_label(nested_single, 4), killed(3));

    let nested_split = "function f()\n  x = source()\n  for x = 1, 2 do\n    for i =\n      x,\n      x\n    do\n      sink(x)\n    end\n  end\nend\n";
    assert_eq!(zero_width_label(nested_split, 5), killed(3));
    assert_eq!(zero_width_label(nested_split, 6), killed(3));

    let duplicate_without_mask = "function f()\n  x = source()\n  sink(x, x)\nend\n";
    assert_eq!(
        zero_width_label(duplicate_without_mask, 3),
        FlowConfidence::Exact
    );
}
