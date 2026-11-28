//! The assertion helper must check its BARE parameter: a member read, index,
//! call or other value derived from the parameter proves nothing about the
//! argument the caller passes.
use super::proto_writes_guard_coverage_tests::{erased, run, run_ts, witness, Case};

const HEAD: &str = "function assertDraft(key)";
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

/// Only a plain, required, single identifier parameter is the argument itself:
/// every other parameter form is refused, in all three grammars.
#[test]
fn assertion_helper_parameter_forms() {
    let base = erased("t2-146.ts");
    let cases = [
        Case {
            label: "neg destructured parameter",
            edits: &[(HEAD, "function assertDraft({ key })")],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg array pattern parameter",
            edits: &[(HEAD, "function assertDraft([key])")],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg rest parameter",
            edits: &[(HEAD, "function assertDraft(...key)")],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg defaulted parameter",
            edits: &[(HEAD, r#"function assertDraft(key = "draft")"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg defaulted parameter to undefined",
            edits: &[(HEAD, "function assertDraft(key = undefined)")],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg alias of the parameter",
            edits: &[(
                r#"    if (key !== "draft")"#,
                r#"    const alias = key;
    if (alias !== "draft")"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg alias assigned in the check",
            edits: &[(CHECK, r#"if ((alias = key) !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
    ];
    run("assertion parameter forms", &base, &cases);
    let typed = witness("t2-146.ts");
    let cases = [
        Case {
            label: "control typed",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg optional parameter",
            edits: &[("assertDraft(key: unknown)", "assertDraft(key?: unknown)")],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg typed defaulted parameter",
            edits: &[(
                "assertDraft(key: unknown)",
                r#"assertDraft(key: unknown = "draft")"#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run_ts("assertion parameter forms typed", &typed, &cases);
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
