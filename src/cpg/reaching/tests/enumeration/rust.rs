use super::super::super::binding_table::{rows, DeclarationKind, Role, Ruling, Visibility};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "e0a-rs-block",
        language: Language::Rust,
        kind: "block",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        let x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-rs-let_declaration",
        language: Language::Rust,
        kind: "let_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        let x = clean(x);\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact), ("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-rs-const_item",
        language: Language::Rust,
        kind: "const_item",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        sink(x);\n        const x: i32 = 1;\n        consume(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", killed(5)), ("2:x", "6:x", killed(5)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "e0a-rs-static_item",
        language: Language::Rust,
        kind: "static_item",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        sink(x);\n        static x: i32 = 1;\n        consume(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", killed(5)), ("2:x", "6:x", killed(5)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "e0a-x-rs-parameters",
        language: Language::Rust,
        kind: "parameters",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(x: i32) {\n    {\n        let x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("1:x", "4:x", killed(3)), ("1:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-unsafe-block-shadow",
        language: Language::Rust,
        kind: "unsafe_block",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    unsafe { let x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", killed(3)), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-match-arm-scope",
        language: Language::Rust,
        kind: "match_arm",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    match opt {\n        Some(_) => {\n            let x = clean();\n            sink(x);\n        }\n        None => {}\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "6:x", killed(5)), ("2:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-if-let-header-scope",
        language: Language::Rust,
        kind: "let_condition",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(y) = opt {\n        sink(x);\n        consume(y);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "rs-let-chain-header-scope",
        language: Language::Rust,
        kind: "let_chain",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(a: Option<i32>, b: Option<i32>) {\n    let x = source();\n    if let Some(y) = a && let Some(z) = b {\n        sink(x);\n        consume(y + z);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "rs-closure-parameters-deferred",
        language: Language::Rust,
        kind: "closure_parameters",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    let closure = |x: i32| {\n        sink(x);\n    };\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "rs-try-block",
        language: Language::Rust,
        kind: "try_block",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    let _: Result<i32, ()> = try {\n        let x = clean();\n        sink(x);\n        x\n    };\n    sink(x);\n}\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-const-block",
        language: Language::Rust,
        kind: "const_block",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    let _ = const {\n        let x = clean();\n        sink(x);\n        x\n    };\n    sink(x);\n}\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-let-else-after-introduction",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        let x = clean() else { return; };\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "5:x", killed(4)), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-uninitialized-let-after-introduction",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    {\n        let x: i32;\n        x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "6:x", killed(4)), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-d10",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let mut x = source();\n    {\n        let Some(v) = o else {\n            sink(x);\n            x = clean();\n            return;\n        };\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "5:x", FlowConfidence::Exact), ("2:x", "9:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("2:x", "11:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("6:x", "11:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 3)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-d09",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(v: Vec<Option<i32>>) {\n    let mut x = source();\n    for o in v {\n        let Some(w) = o else {\n            x = clean();\n            continue;\n        };\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "8:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("2:x", "10:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("5:x", "10:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 3)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-a15",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let mut y = source();\n    let Some(x) = o else {\n        y = clean();\n        return;\n    };\n    sink(y);\n    sink(x);\n}\n",
        expect: &[("2:y", "7:y", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("4:y", "7:y", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-c12",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let mut x = source();\n    let Some(v) = o else {\n        x = clean();\n        panic!();\n    };\n    sink(x);\n}\n",
        expect: &[("2:x", "7:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("4:x", "7:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-e07-field",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let mut s = S { f: source() };\n    let Some(v) = o else {\n        s.f = clean();\n        return;\n    };\n    sink(s.f);\n}\n",
        expect: &[("2:s", "7:s", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("4:s.f", "7:s.f", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-e08-deref",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>, r: &mut i32) {\n    *r = source();\n    let Some(v) = o else {\n        *r = clean();\n        return;\n    };\n    sink(*r);\n}\n",
        expect: &[("2:r", "7:r", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("4:r", "7:r", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-f04-index",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let mut a = source_vec();\n    let Some(v) = o else {\n        a[0] = clean();\n        return;\n    };\n    sink(a);\n}\n",
        expect: &[("2:a", "7:a", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-compound-deref",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>, r: &mut i32) {\n    *r = source();\n    let Some(v) = o else {\n        *r += 1;\n        return;\n    };\n    sink(*r);\n}\n",
        expect: &[("2:r", "7:r", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("4:r", "7:r", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 2)),
    },
    super::case::Case {
        id: "rs-let-else-cfg-gap-e19-use-before",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(v: Vec<Option<i32>>) {\n    let mut x = source();\n    for o in v {\n        sink(x);\n        let Some(w) = o else {\n            x = clean();\n            continue;\n        };\n    }\n}\n",
        expect: &[("6:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "rs-let-pattern-wrapper-g07",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(w: W, z: W) {\n    let W(a) = w;\n    let W(b) = z;\n    sink(a);\n    sink(b);\n}\n",
        expect: &[("1:z", "3:z", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "rs-let-pattern-wrapper-g04",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>, p: Option<i32>) {\n    let Some(v) = o;\n    let Some(w) = p;\n    sink(w);\n}\n",
        expect: &[("1:p", "3:p", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "rs-let-else-no-assignment-control",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let x = source();\n    let Some(v) = o else {\n        return;\n    };\n    consume(v);\n    sink(x);\n}\n",
        expect: &[("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "rs-let-else-different-assignment-control",
        language: Language::Rust,
        kind: "let_declaration",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "fn f(o: Option<i32>) {\n    let x = source();\n    let mut y = clean();\n    let Some(v) = o else {\n        y = other();\n        return;\n    };\n    consume(v + y);\n    sink(x);\n}\n",
        expect: &[("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("block", "e0a-rs-block"),
    ("let_declaration", "e0a-rs-let_declaration"),
    ("const_item", "e0a-rs-const_item"),
    ("static_item", "e0a-rs-static_item"),
    ("parameters", "e0a-x-rs-parameters"),
    ("unsafe_block", "rs-unsafe-block-shadow"),
    ("match_arm", "rs-match-arm-scope"),
    ("let_condition", "rs-if-let-header-scope"),
    ("let_chain", "rs-let-chain-header-scope"),
    ("closure_parameters", "rs-closure-parameters-deferred"),
    ("try_block", "rs-try-block"),
    ("const_block", "rs-const-block"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "generic_pattern" => "fn f<T>(value: T) { match value { Choice::<T> => {}, _ => {} } }",
        "generic_type_with_turbofish" => "fn f<T>() { let value = Type::<T> { field: item }; }",
        "reference_pattern" => "fn f(value: &i32) { let &inner = value; }",
        _ => super::case::source(Language::Rust),
    }
}

#[test]
fn task14_rust_binding_cases() {
    for case in CASES {
        super::case::check_case(case);
    }
}

macro_rules! task14_fix2_case {
    ($test:ident, $id:literal) => {
        #[test]
        fn $test() {
            let case = CASES.iter().find(|case| case.id == $id).unwrap();
            super::case::check_case(case);
        }
    };
}

task14_fix2_case!(fix2_e07_field_target, "rs-let-else-cfg-gap-e07-field");
task14_fix2_case!(fix2_e08_deref_target, "rs-let-else-cfg-gap-e08-deref");
task14_fix2_case!(fix2_f04_index_target, "rs-let-else-cfg-gap-f04-index");
task14_fix2_case!(
    fix2_compound_deref_target,
    "rs-let-else-cfg-gap-compound-deref"
);
task14_fix2_case!(
    fix2_e19_use_before_let,
    "rs-let-else-cfg-gap-e19-use-before"
);
task14_fix2_case!(fix2_g07_wrapper_pattern, "rs-let-pattern-wrapper-g07");
task14_fix2_case!(fix2_g04_some_pattern, "rs-let-pattern-wrapper-g04");

#[test]
fn rust_declaration_rows_introduce_only_their_owned_fields() {
    use super::super::super::binding_table::select_binding_row;
    use super::super::super::transfer::row_introduced_identifiers;

    let parsed = super::super::parsed(
        "fn f() {\n    let mut other = 0;\n    let owned = { other = 1; other };\n    const OWNED_CONST: i32 = { let mut nested = 0; nested = 1; nested };\n    static OWNED_STATIC: i32 = { let mut nested = 0; nested = 1; nested };\n    let closure = |parameter: i32| parameter;\n}\n",
        Language::Rust,
    );
    let cases = [
        ("let_declaration", "let owned", vec!["owned"]),
        ("const_item", "OWNED_CONST", vec!["OWNED_CONST"]),
        ("static_item", "OWNED_STATIC", vec!["OWNED_STATIC"]),
        ("parameters", "()", vec![]),
        ("closure_parameters", "parameter", vec![]),
    ];
    for (kind, marker, expected) in cases {
        let node = super::case::nodes_of_kind(&parsed, kind)
            .into_iter()
            .find(|node| parsed.node_text(node).contains(marker))
            .unwrap_or_else(|| panic!("missing {kind} containing {marker}"));
        let row = select_binding_row(&parsed, node).expect("curated Rust declaration row");
        let actual: Vec<_> = row_introduced_identifiers(&parsed, node, row.fields)
            .into_iter()
            .map(|identifier| parsed.node_text(&identifier))
            .collect();
        assert_eq!(
            actual, expected,
            "{kind}: nested assignments are not declarations"
        );
    }
}

#[test]
fn rust_rows_reproduce_the_curated_scope_and_binding_rules() {
    let rust_rows = rows(Language::Rust);
    let expected = [
        ("block", true, false, None, Visibility::WholeScope),
        (
            "let_declaration",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
        ),
        (
            "const_item",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::WholeScope,
        ),
        (
            "static_item",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::WholeScope,
        ),
        (
            "parameters",
            false,
            true,
            Some(DeclarationKind::Parameter),
            Visibility::WholeScope,
        ),
        ("unsafe_block", true, false, None, Visibility::WholeScope),
        ("const_block", true, false, None, Visibility::WholeScope),
        ("try_block", true, false, None, Visibility::WholeScope),
        ("match_arm", true, false, None, Visibility::WholeScope),
        ("let_condition", true, false, None, Visibility::Header),
        ("let_chain", true, false, None, Visibility::Header),
        (
            "closure_parameters",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::WholeScope,
        ),
    ];

    for (kind, creates_scope, is_binding, declaration, visibility) in expected {
        let row = rust_rows
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing row {kind}"));
        assert_eq!(row.roles.has(Role::Scope), creates_scope, "{kind}");
        assert_eq!(row.roles.has(Role::Binding), is_binding, "{kind}");
        assert_eq!(row.declaration, declaration, "{kind}");
        assert_eq!(row.visibility, visibility, "{kind}");
        assert_eq!(row.ruling, Ruling::Classified, "{kind}");
    }
}
