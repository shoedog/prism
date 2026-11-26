//! The assertion helper must check its BARE parameter: a member read, index,
//! call or other value derived from the parameter proves nothing about the
//! argument the caller passes.
use super::proto_writes_guard_coverage_tests::{erased, run, witness, Case};

const CHECK: &str = r#"if (key !== "draft")"#;

#[test]
fn assertion_helper_checks_the_bare_parameter() {
    let base = erased("t2-146.ts");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "control parenthesised",
            edits: &[(CHECK, r#"if ((key) !== "draft")"#)],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg member read",
            edits: &[(CHECK, r#"if (key.name !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg string index",
            edits: &[(CHECK, r#"if (key["name"] !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg length",
            edits: &[(CHECK, r#"if (key.length !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg conversion call",
            edits: &[(CHECK, r#"if (String(key) !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg method call",
            edits: &[(CHECK, r#"if (key.toString() !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg reversed operands",
            edits: &[(CHECK, r#"if ("draft" !== key.name)"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg optional member",
            edits: &[(CHECK, r#"if (key?.name !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg element read",
            edits: &[(CHECK, r#"if (key[0] !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
    ];
    run("assertion member", &base, &cases);
}

/// The normaliser helper shares the bare-parameter rule: `text.name.trim()...`
/// normalises a member of the argument, not the argument.
#[test]
fn normaliser_helper_normalises_the_bare_parameter() {
    let base = witness("t2-152.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg member normalised",
            edits: &[("text => text.trim()", "text => text.name.trim()")],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
    ];
    run("normaliser member", &base, &cases);
}
