use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

use super::super::case::{Case, CaseClass};

const KILLED_4: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 });
const KILLED_5: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 });
const KILLED_6: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 });
const KILLED_7: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 });

pub(super) const CASES: &[Case] = &[
    Case {
        id: "java-block-shadow",
        language: Language::Java,
        kind: "block",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    {\n      int x = clean();\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "6:x", KILLED_5), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-nested-block-exit",
        language: Language::Java,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    {\n      int x = clean();\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("5:x", "6:x", FlowConfidence::Exact), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    Case {
        id: "java-catch-param-masks",
        language: Language::Java,
        kind: "catch_formal_parameter",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    RuntimeException x = source();\n    try { work(); }\n    catch (RuntimeException x) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "6:x", KILLED_5), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-catch-clause-scope",
        language: Language::Java,
        kind: "catch_clause",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    RuntimeException x = source();\n    try { work(); }\n    catch (RuntimeException x) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "6:x", KILLED_5), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-enhanced-for-header",
        language: Language::Java,
        kind: "enhanced_for_statement",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(Iterable<Item> items) {\n    Item x = source();\n    for (Item x : items) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "5:x", KILLED_4), ("3:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-enhanced-for-multiline-iterable",
        language: Language::Java,
        kind: "enhanced_for_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  Item[] x;\n  void f() {\n    x = source();\n    for (Item x :\n          x) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "6:x", FlowConfidence::Exact), ("4:x", "7:x", KILLED_5), ("4:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-for-init-scope",
        language: Language::Java,
        kind: "for_statement",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    for (int x = clean(); x > 0; x--) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "5:x", KILLED_4), ("3:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-for-body-assignment-reuses-outer",
        language: Language::Java,
        kind: "for_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    for (int i = 0; i < 1; i++) {\n      x += clean();\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("5:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "java-sequential-loops-preserve-outer",
        language: Language::Java,
        kind: "for_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(Iterable<Item> items) {\n    int x = source();\n    for (int i = 0; i < 1; i++) {\n      x *= 31;\n      x = ~~x;\n    }\n    int y = 1;\n    for (Item item : items) {\n      y += item.hashCode();\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("6:x", "12:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "java-try-with-resources",
        language: Language::Java,
        kind: "try_with_resources_statement",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    AutoCloseable x = source();\n    try (var x = open()) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "5:x", KILLED_4), ("3:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-resource-finally-scope",
        language: Language::Java,
        kind: "try_with_resources_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  AutoCloseable x;\n  void f() {\n    x = source();\n    try (var x = open()) {\n      sink(x);\n    } finally {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "6:x", KILLED_5), ("4:x", "8:x", FlowConfidence::Exact), ("4:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-resource-multiline-declarators",
        language: Language::Java,
        kind: "try_with_resources_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  AutoCloseable x;\n  void f() {\n    x = source();\n    try (var a = open();\n         var x = open();\n         var b = wrap(x)) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "7:x", KILLED_6), ("4:x", "8:x", KILLED_6), ("4:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    Case {
        id: "java-existing-resource-is-use",
        language: Language::Java,
        kind: "try_with_resources_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    AutoCloseable x = source();\n    try (x) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[
            (
                "3:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ),
            ("3:x", "5:x", FlowConfidence::Exact),
            ("3:x", "7:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "java-switch-group-scope",
        language: Language::Java,
        kind: "switch_block_statement_group",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(int tag) {\n    int x = source();\n    switch (tag) {\n      case 1:\n        int x = clean();\n        sink(x);\n      case 2:\n        sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "9:x", KILLED_6), ("3:x", "11:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-switch-block-scope",
        language: Language::Java,
        kind: "switch_block",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(int tag) {\n    int x = source();\n    switch (tag) {\n      case 1:\n        int x = clean();\n        sink(x);\n      case 2:\n        sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "9:x", KILLED_6), ("3:x", "11:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-switch-rule-block-scope",
        language: Language::Java,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(int tag) {\n    int x = source();\n    switch (tag) {\n      case 1 -> {\n        int x = clean();\n        sink(x);\n      }\n      default -> work();\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "7:x", KILLED_6), ("3:x", "11:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-local-without-initializer",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    {\n      int x;\n      x = clean();\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "7:x", KILLED_5), ("3:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-local-multi-declarator",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    {\n      int a = 1, x = 2;\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "6:x", KILLED_5), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-local-multiline-declarators",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  int x;\n  void f() {\n    x = source();\n    {\n      int a = 1,\n          x = clean(),\n          b = x;\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "8:x", KILLED_7), ("4:x", "9:x", KILLED_7), ("4:x", "11:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    Case {
        id: "java-for-init-multiline-declarators",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  int x;\n  void f() {\n    x = source();\n    for (int a = 1,\n             x = clean(),\n             b = x; a < 3; a++) {\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "7:x", KILLED_6), ("4:x", "8:x", KILLED_6), ("4:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    Case {
        id: "java-lambda-body-block-scope",
        language: Language::Java,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    Runnable r = () -> {\n      int x = clean();\n      sink(x);\n    };\n    sink(x);\n  }\n}\n",
        expect: &[("5:x", "3:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("3:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "java-lambda-params-not-outer",
        language: Language::Java,
        kind: "lambda_expression",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    String x = source();\n    Mapper fn = (String x) -> x.length();\n    sink(x);\n  }\n}\n",
        expect: &[("3:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("3:x", "5:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "java-formal-parameter-masks",
        language: Language::Java,
        kind: "formal_parameters",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C { void f(int x) {\n  sink(x);\n  {\n    int x = clean();\n    sink(x);\n  }\n  sink(x);\n} }\n",
        expect: &[("1:x", "2:x", FlowConfidence::Exact), ("1:x", "5:x", KILLED_4), ("1:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-spread-parameter-masks",
        language: Language::Java,
        kind: "spread_parameter",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C { void f(String... x) {\n  sink(x);\n  {\n    String[] x = clean();\n    sink(x);\n  }\n  sink(x);\n} }\n",
        expect: &[("1:x", "2:x", FlowConfidence::Exact), ("1:x", "5:x", KILLED_4), ("1:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-declaration-assignment-reuses-name",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f(int a, int b) {\n    int x = a;\n    x = b;\n    sink(x);\n  }\n}\n",
        expect: &[("4:x", "5:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "java-adjacent-declaration-preserves-other-name",
        language: Language::Java,
        kind: "local_variable_declaration",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int x = source();\n    {\n      int x;\n      int y = 0;\n      x = clean();\n      sink(x);\n    }\n    sink(x);\n  }\n}\n",
        expect: &[
            ("3:x", "8:x", KILLED_5),
            ("5:x", "7:x", FlowConfidence::Exact),
            ("7:x", "8:x", FlowConfidence::Exact),
            ("3:x", "10:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 3)),
    },
    Case {
        id: "java-uninitialized-local-condition-reuse",
        language: Language::Java,
        kind: "block",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "class C {\n  void f() {\n    int nRead;\n    long total = 0;\n    while ((nRead = read()) != -1) {\n      sink(nRead);\n      total += nRead;\n    }\n    sink(total);\n  }\n}\n",
        expect: &[
            ("3:nRead", "5:nRead", FlowConfidence::Exact),
            ("3:nRead", "6:nRead", KILLED_5),
            ("3:nRead", "7:nRead", KILLED_5),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
];

fn check_named_case(id: &str) {
    let case = CASES
        .iter()
        .find(|case| case.id == id)
        .unwrap_or_else(|| panic!("missing case {id}"));
    super::super::case::check_case(case);
}

#[test]
fn astra_r1_w1_enhanced_for_multiline_iterable() {
    check_named_case("java-enhanced-for-multiline-iterable");
}

#[test]
fn astra_r1_w2_resource_finally_scope() {
    check_named_case("java-resource-finally-scope");
}

#[test]
fn astra_r1_w3_local_multiline_declarators() {
    check_named_case("java-local-multiline-declarators");
}

#[test]
fn astra_r1_w3_for_init_multiline_declarators() {
    check_named_case("java-for-init-multiline-declarators");
}

#[test]
fn astra_r1_w3_resource_multiline_declarators() {
    check_named_case("java-resource-multiline-declarators");
}

#[test]
fn java_declaration_rows_own_only_introduced_names() {
    use crate::cpg::reaching::binding_table::select_binding_row;
    use crate::cpg::reaching::transfer::row_introduced_identifiers;

    let parsed = super::super::super::parsed(
        "class C { void f(int a, int b) { int x = a, y; x = b; } }",
        Language::Java,
    );
    let declaration = super::super::case::nodes_of_kind(&parsed, "local_variable_declaration")
        .into_iter()
        .next()
        .expect("local declaration");
    let row = select_binding_row(&parsed, declaration).expect("local declaration row");
    let introduced: Vec<_> = row_introduced_identifiers(&parsed, declaration, row.fields)
        .into_iter()
        .map(|identifier| parsed.node_text(&identifier))
        .collect();
    assert_eq!(introduced, ["x", "y"]);

    let resources = super::super::super::parsed(
        "class C { void f(AutoCloseable x) { try (var declared = open(); x) {} } }",
        Language::Java,
    );
    let owner = super::super::case::nodes_of_kind(&resources, "try_with_resources_statement")
        .into_iter()
        .next()
        .expect("try-with-resources");
    let row = select_binding_row(&resources, owner).expect("resource row");
    let introduced: Vec<_> = row_introduced_identifiers(&resources, owner, row.fields)
        .into_iter()
        .map(|identifier| resources.node_text(&identifier))
        .collect();
    assert_eq!(introduced, ["declared"]);

    let adjacent = super::super::super::parsed(
        "class C {\n  void f() {\n    int nRead;\n    long total = 0;\n  }\n}\n",
        Language::Java,
    );
    let declarations = super::super::case::nodes_of_kind(&adjacent, "local_variable_declaration");
    let introduced: Vec<_> = declarations
        .into_iter()
        .map(|declaration| {
            let row = select_binding_row(&adjacent, declaration).expect("adjacent declaration row");
            let names: Vec<_> = row_introduced_identifiers(&adjacent, declaration, row.fields)
                .into_iter()
                .map(|identifier| adjacent.node_text(&identifier))
                .collect();
            (declaration.start_position().row + 1, names)
        })
        .collect();
    assert_eq!(introduced, [(3, vec!["nRead"]), (4, vec!["total"])]);
}

#[test]
fn java_declaration_seed_rejects_mismatched_legacy_path() {
    use crate::access_path::AccessPath;
    use crate::cpg::reaching::{DefId, DefSite, RdOutcome};

    let parsed = super::super::super::parsed(
        "class C {\n  void f() {\n    int nRead;\n    long total = 0;\n    while ((nRead = read()) != -1) {}\n  }\n}\n",
        Language::Java,
    );
    let function = super::super::super::function(&parsed);
    let mut defs = super::super::super::collect_defs(&parsed);
    assert!(
        !defs
            .iter()
            .any(|def| def.line == 4 && def.path.base == "nRead"),
        "exact helper inventory must not already contain the legacy mismatch"
    );
    let real = defs
        .iter()
        .find(|def| def.line == 3 && def.path.base == "nRead")
        .expect("real nRead declaration")
        .clone();
    defs.push(DefSite {
        id: DefId(u32::try_from(defs.len()).expect("bounded test definitions")),
        path: AccessPath::simple("nRead"),
        line: 4,
        start_byte: super::super::super::byte_on_line(&parsed, 4, "total", 0),
        alias_derived: true,
        synthetic: false,
    });
    let edge = super::super::super::edge(&parsed, &function, &real, 5);
    let (outcome, _) = super::super::super::run(&parsed, &defs, std::slice::from_ref(&edge));
    let RdOutcome::Available(result) = outcome else {
        panic!("expected available reaching result");
    };
    assert_eq!(
        result.labels.get(&(edge.from, edge.to)),
        Some(&FlowConfidence::Exact)
    );
}
