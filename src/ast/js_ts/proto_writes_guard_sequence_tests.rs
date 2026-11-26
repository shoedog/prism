//! Sequence, pointer-token and callback premises.
use super::proto_writes_guard_coverage_tests::{run, witness, Case};

/// The block-list test inside a callback must be on the BARE parameter: `k.name`
/// is a member of the element, so the callback removes or renames nothing.
#[test]
fn filter_callbacks_test_the_bare_parameter() {
    let filter = witness("g22.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "ctl parenthesised",
            edits: &[(r#".includes(k)"#, r#".includes((k))"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "neg member of the element",
            edits: &[(r#".includes(k)"#, r#".includes(k.name)"#)],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "neg index of the element",
            edits: &[(r#".includes(k)"#, r#".includes(k[0])"#)],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "neg optional member of the element",
            edits: &[(r#".includes(k)"#, r#".includes(k?.name)"#)],
            states: "NN",
            refusals: &[],
        },
    ];
    run("filter bare parameter", &filter, &cases);
}

/// The same rule for a `map` callback that renames reserved names.
#[test]
fn map_callbacks_test_the_bare_parameter() {
    let map = witness("g23.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "neg member of the element",
            edits: &[(r#".includes(k)"#, r#".includes(k.name)"#)],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "neg index of the element",
            edits: &[(r#".includes(k)"#, r#".includes(k[0])"#)],
            states: "NN",
            refusals: &[],
        },
    ];
    run("map bare parameter", &map, &cases);
}

/// A filter callback that removes reserved names is recognised in the shapes a predicate can take.
#[test]
fn pipeline_callbacks() {
    let base = witness("g22.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "var declared helper callback",
            edits: &[
                (
                    r#"const keys"#,
                    r#"function isSafe(k) { return !['__proto__', 'constructor', 'prototype'].includes(k); }
  const keys"#,
                ),
                (
                    r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                    r#"isSafe"#,
                ),
            ],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "var block-bodied callback",
            edits: &[(
                r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                r#"k => { return !['__proto__', 'constructor', 'prototype'].includes(k); }"#,
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "var undeclared callback",
            edits: &[(
                r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                r#"isSafe"#,
            )],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "var callback bound by const",
            edits: &[
                (
                    r#"const keys"#,
                    r#"const isSafe = k => !['__proto__', 'constructor', 'prototype'].includes(k);
  const keys"#,
                ),
                (
                    r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                    r#"isSafe"#,
                ),
            ],
            states: "PP",
            refusals: &[("incomplete_coverage", 1), ("unproved_native_identity", 2)],
        },
        Case {
            label: "var callback without parameters",
            edits: &[(
                r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                r#"function () { return true; }"#,
            )],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "var block without a return",
            edits: &[(
                r#"k => !['__proto__', 'constructor', 'prototype'].includes(k)"#,
                r#"k => { log(k); }"#,
            )],
            states: "NN",
            refusals: &[],
        },
    ];
    run("pipeline_callbacks", &base, &cases);
}

/// A pointer token unescape chain is recognised only as the exact ~1 and ~0 replacements, at bounded depth.
#[test]
fn raw_pointer_tokens() {
    let base = witness("t2-072.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("wrong_binding", 2)],
        },
        Case {
            label: "var other replacement text",
            edits: &[(r#".replace(/~1/g, "/")"#, r#".replace(/~1/g, "|")"#)],
            states: "N",
            refusals: &[("wrong_binding", 2)],
        },
        Case {
            label: "var other replacement pattern",
            edits: &[(r#".replace(/~1/g, "/")"#, r#".replace(/~2/g, "/")"#)],
            states: "N",
            refusals: &[("wrong_binding", 2)],
        },
        Case {
            label: "var unescape chain too deep",
            edits: &[(
                r#".replace(/~1/g, "/").replace(/~0/g, "~")"#,
                r#".replace(/~1/g, "/").replace(/~0/g, "~").replace(/~1/g, "/").replace(/~0/g, "~").replace(/~1/g, "/")"#,
            )],
            states: "N",
            refusals: &[("wrong_binding", 2)],
        },
    ];
    run("raw_pointer_tokens", &base, &cases);
}

/// A membership check before an await is read only in its exact every-over-a-set shape.
#[test]
fn checked_before_await_premises() {
    let base = witness("t2-082.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "N",
            refusals: &[("wrong_version", 1)],
        },
        Case {
            label: "var callback is not inline",
            edits: &[(
                r#"keys.every(key => allowed.has(key))"#,
                r#"keys.every(fn)"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var callback is not a set test",
            edits: &[(
                r#"keys.every(key => allowed.has(key))"#,
                r#"keys.every(key => key === "settings")"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1)],
        },
        Case {
            label: "var set test has two arguments",
            edits: &[(r#"allowed.has(key)"#, r#"allowed.has(key, 1)"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var some instead of every",
            edits: &[(r#"keys.every("#, r#"keys.some("#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var other receiver",
            edits: &[(r#"!keys.every("#, r#"!other.every("#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var every has two arguments",
            edits: &[(
                r#"keys.every(key => allowed.has(key))"#,
                r#"keys.every(key => allowed.has(key), this)"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var check is not negated",
            edits: &[(
                r#"!keys.every(key => allowed.has(key))"#,
                r#"keys.every(key => allowed.has(key))"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var set is not a Set",
            edits: &[(r#"new Set(["settings", "profile"])"#, r#"makeSet()"#)],
            states: "N",
            refusals: &[],
        },
    ];
    run("checked_before_await_premises", &base, &cases);
}

/// A forEach that returns early on names is read only in its exact callback shape over the same sequence.
#[test]
fn for_each_return_premises() {
    let base = witness("t2-081.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "N",
            refusals: &[("wrong_owner", 1)],
        },
        Case {
            label: "var callback is not inline",
            edits: &[(
                r#"keys.forEach(key => {
    if (key !== "profile" && key !== "settings") return;
    recognized++;
  });"#,
                r#"keys.forEach(helper);"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var other receiver",
            edits: &[(r#"keys.forEach"#, r#"other.forEach"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var forEach with a this argument",
            edits: &[(
                r#"    recognized++;
  });"#,
                r#"    recognized++;
  }, this);"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var first statement is not a check",
            edits: &[(
                r#"    if (key !=="#,
                r#"    log(key);
    if (key !=="#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var check does not return",
            edits: &[(r#""settings") return;"#, r#""settings") log(key);"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var check is not a name comparison",
            edits: &[(r#"key !== "profile" && key !== "settings""#, r#"key"#)],
            states: "N",
            refusals: &[],
        },
    ];
    run("for_each_return_premises", &base, &cases);
}

/// Every write form that can replace the String intrinsic denies the conversion proof.
#[test]
fn native_rebinding_forms() {
    let base = witness("t2-048.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg window property store",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"window.String = fake;
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg self property store",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"self.String = fake;
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg escaped reflective key",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"Reflect["\u0073et"](globalThis, "String", fake);
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg defineProperty",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"Object.defineProperty(globalThis, "String", { value: fake });
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg assign onto the global",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"Object.assign(globalThis, { String: fake });
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg computed key store",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"globalThis["St" + "ring"] = fake;
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg delete",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"delete globalThis.String;
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg alias of the global",
            edits: &[(
                r#"function set(root, keys, value) {"#,
                r#"const g = globalThis;
g.String = fake;
function set(root, keys, value) {"#,
            )],
            states: "P",
            refusals: &[("absent_domain", 1), ("intrinsic_rebound", 1)],
        },
    ];
    run("native_rebinding_forms", &base, &cases);
}
