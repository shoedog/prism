use super::case::{Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const fn ou(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

pub(super) const CASES: &[Case] = &[
    Case {
        id: "bash-variable-assignment-uncertain",
        language: Language::Bash,
        kind: "variable_assignment",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    local x=clean\n    sink \"$x\"\n}\n",
        expect: &[("2:x", "4:x", ou(3))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "bash-declaration-command-uncertain",
        language: Language::Bash,
        kind: "declaration_command",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    local x=clean\n    sink \"$x\"\n}\n",
        expect: &[("2:x", "4:x", ou(3))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "bash-bare-local-declaration-uncertain",
        language: Language::Bash,
        kind: "declaration_command",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    local x\n    sink \"$x\"\n}\n",
        expect: &[("2:x", "4:x", ou(3))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "bash-bare-declare-and-multiple-names-uncertain",
        language: Language::Bash,
        kind: "declaration_command",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    y=$(source)\n    declare a x y\n    sink \"$x\"\n    sink \"$y\"\n}\n",
        expect: &[("2:x", "5:x", ou(4)), ("3:y", "6:y", ou(4))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
    },
    Case {
        id: "bash-bare-declaration-name-and-function-controls",
        language: Language::Bash,
        kind: "declaration_command",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    local y\n    sink \"$x\"\n    local x\n    sink \"$x\"\n}\ng() {\n    local x\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact), ("2:x", "6:x", ou(5))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "bash-for-variable-uncertain",
        language: Language::Bash,
        kind: "for_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    for x in a b; do :; done\n    sink \"$x\"\n}\n",
        expect: &[("2:x", "4:x", ou(3))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "bash-c-style-for-uncertain",
        language: Language::Bash,
        kind: "c_style_for_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "f() { for ((i=0; i<2; i++)); do :; done; }\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "bash-case-item-uncertain",
        language: Language::Bash,
        kind: "case_item",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "f() { case \"$x\" in one) x=clean ;; esac; }\n",
        expect: &[],
        expect_counter: None,
    },
];

#[test]
fn task27_bash_rows_are_census_only() {
    use crate::cpg::reaching::binding_table::{rows, Ruling};
    for row in rows(Language::Bash) {
        assert_eq!(
            row.ruling,
            Ruling::Uncertain {
                reason: "census-only language",
                revisit: "future:scripting",
            },
            "{}",
            row.kind
        );
    }
}

#[test]
fn task27_r1_bash_bare_declarations_fail_closed() {
    let mut failed = Vec::new();
    for case in CASES.iter().filter(|case| case.id.starts_with("bash-bare")) {
        if std::panic::catch_unwind(|| super::case::check_case(case)).is_err() {
            failed.push(case.id);
        }
    }
    assert!(failed.is_empty(), "Task 27 R1 case failures: {failed:?}");
}

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "variable_assignments" => "FIRST=one SECOND=two\n",
        _ => super::case::source(crate::languages::Language::Bash),
    }
}
