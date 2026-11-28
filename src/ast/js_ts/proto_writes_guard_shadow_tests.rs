//! A helper's parameter is checked only by reads that BIND it. A hoisted
//! function declaration of the same name overrides the parameter binding, so
//! a comparison against that name checks nothing about the argument. The
//! control of each family keeps its (partial) credit; the negative neighbour
//! must give none.
use super::proto_writes_guard_coverage_tests::{run, run_js, witness, Case};

const SOME_TEST: &str =
    r#"keys.some(key => key === "__proto__" || key === "constructor" || key === "prototype")"#;
const HELPER_RETURN: &str = r#"key !== "constructor" && key !== "prototype";"#;

#[test]
fn denial_helper_checks_the_binding_parameter() {
    let base = witness("t2-029.js");
    let cases = [
        Case {
            label: "var unshadowed control",
            edits: &[],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg hoisted function shadows the parameter",
            edits: &[(
                HELPER_RETURN,
                r#"key !== "constructor" && key !== "prototype"; function key() {}"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "var function in a nested block does not shadow",
            edits: &[(
                HELPER_RETURN,
                r#"key !== "constructor" && key !== "prototype"; { function other() {} }"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
    ];
    run_js("denial helper shadow", &base, &cases);
}

#[test]
fn some_callback_checks_the_binding_parameter() {
    let base = witness("t3-024.js");
    let cases = [
        Case {
            label: "var unshadowed control",
            edits: &[],
            states: "P",
            refusals: &[("incomplete_coverage", 1)],
        },
        Case {
            label: "var block body control",
            edits: &[(
                SOME_TEST,
                r#"keys.some(key => { return key === "__proto__" || key === "constructor" || key === "prototype"; })"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1)],
        },
        Case {
            label: "neg hoisted function shadows the parameter",
            edits: &[(
                SOME_TEST,
                r#"keys.some(key => { return key === "__proto__" || key === "constructor" || key === "prototype"; function key() {} })"#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("some shadow", &base, &cases);
}

/// Known limits (constructed, zero incidence on the pinned corpora): a global
/// is selected by its SPELLING at the partial tier, so a file that declares its
/// own `WeakSet` or `Set` keeps Partial. The sufficient tier is proved by
/// `native` and refuses (`intrinsic_rebound`). Listed in `SPELLING_READS` as
/// NATIVE-PARTIAL; each case sits beside its unshadowed control.
#[test]
fn shadowed_native_collections_keep_partial_known_limit() {
    let base = witness("t2-118.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "var known limit own WeakSet class stays partial",
            edits: &[(
                "function annotateLocalPanel",
                "class WeakSet { has() { return true; } add() {} }\nfunction annotateLocalPanel",
            )],
            states: "P",
            refusals: &[("intrinsic_rebound", 1), ("unproved_child", 1)],
        },
    ];
    run("shadowed WeakSet", &base, &cases);
    let base = witness("g16.js");
    let cases = [
        Case {
            label: "var control",
            edits: &[],
            states: "PP",
            refusals: &[("incomplete_coverage", 3)],
        },
        Case {
            label: "var known limit own Set class stays partial",
            edits: &[(
                "const UNSAFE",
                "class Set { has() { return false; } }\nconst UNSAFE",
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 3)],
        },
    ];
    run("shadowed Set", &base, &cases);
}
