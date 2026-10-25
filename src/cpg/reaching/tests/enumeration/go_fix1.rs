use super::super::case::{check_case, Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const EXACT: FlowConfidence = FlowConfidence::Exact;

const fn killed(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: line })
}

const fn ownership_uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

macro_rules! regression {
    ($test:ident, $id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr) => {
        #[test]
        fn $test() {
            const EXPECT: &[(&str, &str, FlowConfidence)] = $expect;
            check_case(&Case {
                id: $id,
                language: Language::Go,
                kind: $kind,
                variant: None,
                row_variant: None,
                class: CaseClass::Structural,
                src: $src,
                expect: EXPECT,
                expect_counter: Some($counter),
            });
        }
    };
}

regression!(
    w1_b02_for_clause_assignment,
    "fix1-w1-b02-for-clause-assignment",
    "for_clause",
    "package main\n\nfunc f(n int) {\n\ti := source()\n\tfor i = 0; i < n; i++ {\n\t\tsink(i)\n\t}\n\tsink(i)\n}\n",
    &[("4:i", "8:i", killed(5)), ("5:i", "8:i", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w1_f04_for_clause_multi_assignment,
    "fix1-w1-f04-for-clause-multi-assignment",
    "for_clause",
    "package main\n\nfunc f(n int) {\n\ti := source()\n\tj := 0\n\tfor i, j = 0, 1; i < n; i++ {\n\t\tsink(j)\n\t}\n\tsink(i)\n}\n",
    &[("4:i", "9:i", killed(6)), ("6:i", "9:i", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w1_f06_for_clause_empty_body,
    "fix1-w1-f06-for-clause-empty-body",
    "for_clause",
    "package main\n\nfunc f(n int) {\n\ti := source()\n\tfor i = 0; i < n; i++ {\n\t}\n\tsink(i)\n}\n",
    &[("4:i", "7:i", killed(5)), ("5:i", "7:i", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w1_f11_for_clause_assignment_update,
    "fix1-w1-f11-for-clause-assignment-update",
    "for_clause",
    "package main\n\nfunc f(n int) {\n\ti := source()\n\tfor i = 0; i < n; i = i + 1 {\n\t\tsink(i)\n\t}\n\tsink(i)\n}\n",
    &[("4:i", "8:i", killed(5))],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w1_g03_for_clause_initializer_read,
    "fix1-w1-g03-for-clause-initializer-read",
    "for_clause",
    "package main\n\nfunc f(n int) {\n\ti := source()\n\tfor i = i + 1; i < n; i++ {\n\t}\n\tsink(i)\n}\n",
    &[("4:i", "7:i", killed(5)), ("5:i", "7:i", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w2_f01_type_switch_assigns_both_arms,
    "fix1-w2-f01-type-switch-assigns-both-arms",
    "type_switch_statement",
    "package main\n\nfunc f(v any) {\n\tx := source()\n\tswitch v.(type) {\n\tcase int:\n\t\tx = clean()\n\t\tsink(x)\n\tdefault:\n\t\tx = clean()\n\t}\n\tsink(x)\n}\n",
    &[("7:x", "8:x", EXACT), ("10:x", "12:x", EXACT)],
    ("dfg_label_exact", 2)
);

regression!(
    w2_f02_type_switch_assignment_then_use,
    "fix1-w2-f02-type-switch-assignment-then-use",
    "type_switch_statement",
    "package main\n\nfunc f(v any) {\n\tx := 0\n\tswitch v.(type) {\n\tcase int:\n\t\tx = source()\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
    &[("4:x", "8:x", killed(7)), ("7:x", "8:x", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w2_g02_type_switch_nested_if_assignment,
    "fix1-w2-g02-type-switch-nested-if-assignment",
    "type_switch_statement",
    "package main\n\nfunc f(v any, c bool) {\n\tx := source()\n\tswitch v.(type) {\n\tcase int:\n\t\tif c {\n\t\t\tx = clean()\n\t\t\tsink(x)\n\t\t}\n\t}\n\tsink(x)\n}\n",
    &[("4:x", "9:x", killed(8)), ("4:x", "12:x", EXACT), ("8:x", "9:x", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w2_f09_type_switch_case_assignment,
    "fix1-w2-f09-type-switch-case-assignment",
    "type_switch_statement",
    "package main\n\nfunc f(v any) {\n\tx := source()\n\tswitch v.(type) {\n\tcase int:\n\t\ty := 1\n\t\tx = y\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
    &[("4:x", "9:x", killed(8)), ("8:x", "9:x", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);

regression!(
    w2_c05_type_switch_alias_case_assignment,
    "fix1-w2-c05-type-switch-alias-case-assignment",
    "type_switch_statement",
    "package main\n\nfunc f(v any) {\n\tx := source()\n\tswitch x := v.(type) {\n\tcase int:\n\t\tx = 3\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
    &[("7:x", "8:x", EXACT)],
    ("dfg_label_exact", 1)
);

regression!(
    w3_g01_type_switch_alias_back_edge,
    "fix1-w3-g01-type-switch-alias-back-edge",
    "type_switch_statement",
    "package main\n\nfunc f(v any) {\n\tx := 0\n\tfor i := 0; i < 3; i++ {\n\t\tswitch x := v.(type) {\n\t\tcase int:\n\t\t\tsink(x)\n\t\t}\n\t\tx = source()\n\t}\n}\n",
    &[("10:x", "8:x", ownership_uncertain(6))],
    ("dfg_label_nameonly_ownership_uncertain", 1)
);

regression!(
    s1_d10_select_case_scope,
    "fix1-s1-d10-select-case-scope",
    "communication_case",
    "package main\n\nfunc f(ch, dh chan int) {\n\tx := source()\n\tselect {\n\tcase <-ch:\n\t\tx := clean()\n\t\tsink(x)\n\tcase <-dh:\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
    &[("4:x", "8:x", killed(7)), ("4:x", "10:x", EXACT)],
    ("dfg_label_nameonly_killed", 1)
);
