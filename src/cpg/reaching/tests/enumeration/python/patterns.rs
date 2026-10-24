use super::super::case::{Case, CaseClass};
use crate::cpg::reaching::binding_table::{rows, Predicate, Visibility};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

macro_rules! case {
    ($id:literal, $kind:literal, $row_variant:expr, $src:literal, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::Python,
            kind: $kind,
            variant: Some("binding"),
            row_variant: $row_variant,
            class: CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
}

const EXACT: FlowConfidence = FlowConfidence::Exact;
const fn uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

const CASE_PARENT: Option<Predicate> = Some(Predicate::ParentKindIs {
    kind: "case_pattern",
});

pub(crate) const CASES: &[Case] = &[
    case!(
        "e0a-py-function_definition",
        "function_definition",
        None,
        "def f():\n    x = source(); y = source()\n    sink(x)\n    y = clean()\n    sink(y)\n",
        &[
            ("2:x", "3:x", EXACT),
            (
                "2:y",
                "5:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
        ],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "e0a-py-lambda",
        "lambda",
        None,
        "def f():\n    x = source()\n    fn = lambda y: y\n    sink(x)\n",
        &[("2:x", "4:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-py-class_definition",
        "class_definition",
        None,
        "def f():\n    x = source()\n    class C:\n        pass\n    sink(x)\n",
        &[("2:x", "5:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-py-list_comprehension",
        "list_comprehension",
        None,
        "def f(items):\n    x = source()\n    values = [item for item in items]\n    sink(x)\n",
        &[("2:x", "4:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-py-set_comprehension",
        "set_comprehension",
        None,
        "def f(items):\n    x = source()\n    values = {item for item in items}\n    sink(x)\n",
        &[("2:x", "4:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-py-dictionary_comprehension",
        "dictionary_comprehension",
        None,
        "def f(items):\n    x = source()\n    values = {item: item for item in items}\n    sink(x)\n",
        &[("2:x", "4:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-py-generator_expression",
        "generator_expression",
        None,
        "def f(items):\n    x = source()\n    values = (item for item in items)\n    sink(x)\n",
        &[("2:x", "4:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "e0a-x-py-parameters",
        "parameters",
        None,
        "def f(parameter):\n    x = source(); y = source()\n    sink(x)\n    y = clean()\n    sink(y)\n",
        &[
            ("2:x", "3:x", EXACT),
            (
                "2:y",
                "5:y",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
        ],
        ("dfg_label_nameonly_killed", 1)
    ),
    case!(
        "py-match-statement-scope",
        "match_statement",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case 0:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", EXACT), ("2:x", "6:x", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "py-case-clause-scope",
        "case_clause",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case 0:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", EXACT), ("2:x", "6:x", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "py-case-pattern-uncertain",
        "case_pattern",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case x:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-class-pattern-uncertain",
        "class_pattern",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case Box(x):\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-case-as-pattern-inside-arm",
        "as_pattern",
        CASE_PARENT,
        "def f(value):\n    x = source()\n    match value:\n        case [y] as x:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-as-pattern-non-case-transparent",
        "as_pattern",
        None,
        "def f(ctx):\n    y = source()\n    with ctx as x:\n        sink(y)\n",
        &[("2:y", "4:y", EXACT)],
        ("dfg_label_exact", 1)
    ),
    case!(
        "py-keyword-pattern-uncertain",
        "keyword_pattern",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case Box(item=x):\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-pattern-list-uncertain",
        "pattern_list",
        None,
        "def f(value):\n    x = source(); keep = source()\n    x, y = value\n    sink(x); sink(keep)\n",
        &[("2:x", "4:x", uncertain(3)), ("2:keep", "4:keep", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-tuple-pattern-uncertain",
        "tuple_pattern",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case (x,):\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-list-pattern-uncertain",
        "list_pattern",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case [x]:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
];

pub(crate) const CURATED: &[(&str, &str)] = &[
    ("match_statement", "py-match-statement-scope"),
    ("case_clause", "py-case-clause-scope"),
    ("case_pattern", "py-case-pattern-uncertain"),
    ("class_pattern", "py-class-pattern-uncertain"),
    ("as_pattern", "py-case-as-pattern-inside-arm"),
    ("keyword_pattern", "py-keyword-pattern-uncertain"),
    ("pattern_list", "py-pattern-list-uncertain"),
    ("tuple_pattern", "py-tuple-pattern-uncertain"),
    ("list_pattern", "py-list-pattern-uncertain"),
];

#[test]
fn task12_python_pattern_cases() {
    for case in CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn py_case_outside_arm_preserved() {
    super::super::case::check_case(&Case {
        id: "py-case-outside-arm-preserved",
        variant: None,
        ..CASES[12]
    });
}

#[test]
fn py_comprehension_target_first_iterable() {
    super::super::case::check_case(&Case {
        id: "py-comprehension-target-first-iterable",
        language: Language::Python,
        kind: "list_comprehension",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "def f():\n    x = source()\n    values = [item for item in x]\n    sink(x)\n",
        expect: &[("2:x", "3:x", EXACT), ("2:x", "4:x", EXACT)],
        expect_counter: Some(("dfg_label_exact", 2)),
    });
}

#[test]
fn comprehension_rows_record_special_visibility() {
    for kind in [
        "list_comprehension",
        "set_comprehension",
        "dictionary_comprehension",
        "generator_expression",
    ] {
        let row = rows(Language::Python)
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind}"));
        assert_eq!(row.visibility, Visibility::Comprehension, "{kind}");
    }
    for kind in ["function_definition", "lambda", "class_definition"] {
        let row = rows(Language::Python)
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind}"));
        assert_eq!(row.visibility, Visibility::WholeScope, "{kind}");
    }
}

#[test]
fn py_lambda_param_body_is_capture() {
    super::super::case::check_case(&Case {
        id: "py-lambda-param-body-is-capture",
        language: Language::Python,
        kind: "lambda",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    fn = lambda y: sink(x)\n    return fn\n",
        expect: &[(
            "2:x",
            "3:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    });
}
