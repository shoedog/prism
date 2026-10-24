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

macro_rules! behavior_case {
    ($id:literal, $kind:literal, $row_variant:expr, $src:literal, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::Python,
            kind: $kind,
            variant: None,
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
const CASE_ANCESTOR: Option<Predicate> = Some(Predicate::AncestorKindIs {
    kind: "case_clause",
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
        "def f(value):\n    x = source()\n    match value:\n        case [x]:\n            sink(x)\n    sink(x)\n",
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
        "py-pattern-list-non-case-transparent",
        "pattern_list",
        None,
        "def f(c, v):\n    a = source()\n    if c:\n        a, b = v\n    sink(a)\n",
        &[("2:a", "4:a", EXACT), ("2:a", "5:a", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "py-tuple-pattern-uncertain",
        "tuple_pattern",
        CASE_ANCESTOR,
        "def f(value):\n    x = source()\n    match value:\n        case (x,):\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-list-pattern-uncertain",
        "list_pattern",
        CASE_ANCESTOR,
        "def f(value):\n    x = source()\n    match value:\n        case [x]:\n            sink(x)\n    sink(x)\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "6:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    case!(
        "py-tuple-pattern-non-case-transparent",
        "tuple_pattern",
        None,
        "def f(c, v):\n    a = source()\n    if c:\n        (a, b) = v\n    sink(a)\n",
        &[("2:a", "4:a", EXACT), ("2:a", "5:a", EXACT)],
        ("dfg_label_exact", 2)
    ),
    case!(
        "py-list-pattern-non-case-transparent",
        "list_pattern",
        None,
        "def f(c, v):\n    a = source()\n    if c:\n        [a, b] = v\n    sink(a)\n",
        &[("2:a", "4:a", EXACT), ("2:a", "5:a", EXACT)],
        ("dfg_label_exact", 2)
    ),
];

const W2_CASES: &[Case] = &[
    behavior_case!(
        "py-definitions-in-both-match-arms-share-function-scope",
        "match_statement",
        None,
        "def f(v):\n    match v:\n        case 1:\n            r = a()\n        case _:\n            r = b()\n    return r\n",
        &[("6:r", "7:r", EXACT)],
        ("dfg_label_exact", 1)
    ),
    behavior_case!(
        "py-definition-in-case-clause-survives-arm",
        "case_clause",
        None,
        "def f(value):\n    match value:\n        case [y]:\n            g = y + 1\n    return g\n",
        &[("4:g", "5:g", EXACT)],
        ("dfg_label_exact", 1)
    ),
    behavior_case!(
        "py-match-subject-walrus-survives",
        "match_statement",
        None,
        "def f(g):\n    match (y := g()):\n        case 1:\n            pass\n    sink(y)\n",
        &[("2:y", "5:y", EXACT)],
        ("dfg_label_exact", 1)
    ),
    behavior_case!(
        "py-match-arm-lambda-definition-survives",
        "case_clause",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case [x]:\n            g = lambda: sink(x)\n    sink(x)\n    return g\n",
        &[("5:g", "7:g", EXACT)],
        ("dfg_label_exact", 1)
    ),
    behavior_case!(
        "py-match-arm-assignment-reaches-after",
        "case_clause",
        None,
        "def f(value):\n    x = source()\n    match value:\n        case 1:\n            x = clean()\n    sink(x)\n",
        &[("5:x", "6:x", EXACT)],
        ("dfg_label_exact", 1)
    ),
];

const W3_CASES: &[Case] = &[behavior_case!(
    "py-case-guard-read-fails-closed",
    "as_pattern",
    CASE_PARENT,
    "def f(value):\n    x = source()\n    match value:\n        case [y] as x if x:\n            sink(x)\n    sink(x)\n",
    &[
        ("2:x", "4:x", uncertain(4)),
        ("2:x", "5:x", uncertain(4)),
        ("2:x", "6:x", EXACT),
    ],
    ("dfg_label_nameonly_ownership_uncertain", 2)
)];

pub(crate) const CURATED: &[(&str, &str)] = &[
    ("match_statement", "py-match-statement-scope"),
    ("case_clause", "py-case-clause-scope"),
    ("case_pattern", "py-case-pattern-uncertain"),
    ("class_pattern", "py-class-pattern-uncertain"),
    ("as_pattern", "py-case-as-pattern-inside-arm"),
    ("keyword_pattern", "py-keyword-pattern-uncertain"),
    ("pattern_list", "py-pattern-list-non-case-transparent"),
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
fn task12_fix1_w1_non_case_patterns() {
    let killed_at_3 = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 });
    assert_supplemental_edges(
        "def f(c, v):\n    a = source()\n    if c:\n        a, b = v\n    sink(a)\n",
        &[(2, "a", 4, EXACT), (2, "a", 5, EXACT)],
    );
    assert_supplemental_edges(
        "def f(c, v):\n    a = source()\n    if c:\n        [a, b] = v\n    sink(a)\n",
        &[(2, "a", 4, EXACT), (2, "a", 5, EXACT)],
    );
    assert_supplemental_edges(
        "def f(items):\n    a = source()\n    for i in items:\n        sink(a)\n        a, b = i, a\n    sink(a)\n",
        &[
            (2, "a", 4, EXACT),
            (2, "a", 5, EXACT),
            (2, "a", 6, EXACT),
        ],
    );
    assert_supplemental_edges(
        "def f(items):\n    k = source(); x = source(); y = source()\n    for k, x in items:\n        sink(k); sink(x); sink(y)\n    sink(k); sink(x); sink(y)\n",
        &[
            (2, "k", 3, EXACT),
            (2, "k", 4, killed_at_3),
            (2, "k", 5, EXACT),
        ],
    );
    assert_supplemental_edges(
        "def f(items):\n    a = source()\n    out = [a for a, b in items]\n    sink(a)\n    return out\n",
        &[(2, "a", 3, killed_at_3), (2, "a", 4, EXACT)],
    );
    assert_supplemental_edges(
        "def f():\n    x = source(); y = 1\n    x, y = y, x\n    sink(x); sink(y)\n",
        &[(2, "x", 3, EXACT), (2, "x", 4, killed_at_3)],
    );
}

fn assert_supplemental_edges(src: &str, expected: &[(usize, &str, usize, FlowConfidence)]) {
    let parsed = super::super::super::parsed(src, Language::Python);
    let function = super::super::super::function(&parsed);
    let defs = super::super::super::collect_defs(&parsed);
    let edges: Vec<_> = expected
        .iter()
        .map(|(from_line, name, to_line, _)| {
            let def = defs
                .iter()
                .find(|def| def.line == *from_line && def.path.base == *name)
                .unwrap_or_else(|| panic!("missing {from_line}:{name}"));
            super::super::super::edge(&parsed, &function, def, *to_line)
        })
        .collect();
    let (outcome, _) = super::super::super::run(&parsed, &defs, &edges);
    for (edge, (_, _, _, label)) in edges.iter().zip(expected) {
        super::super::super::assert_label(&outcome, edge, *label);
    }
}

#[test]
fn task12_fix1_w2_match_arms_are_not_scopes() {
    for case in W2_CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn task12_fix1_w3_guard_reads_fail_closed() {
    for case in W3_CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn py_case_outside_arm_preserved() {
    let case = CASES
        .iter()
        .find(|case| case.id == "py-case-pattern-uncertain")
        .expect("case-pattern regression");
    super::super::case::check_case(&Case {
        id: "py-case-outside-arm-preserved",
        variant: None,
        ..*case
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
        class: CaseClass::Behavioral,
        src: "def f():\n    x = source()\n    fn = lambda y: sink(x)\n    return fn\n",
        expect: &[(
            "2:x",
            "3:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    });
}
