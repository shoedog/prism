//! Helper-function premises: assertions and string normalisers.
use super::proto_writes_guard_coverage_tests::{erased, run, witness, Case};

/// A direct assertion helper must be a lone, throwing, string check on its parameter.
#[test]
fn assertion_helper_premises() {
    let base = erased("t2-146.ts");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg arrow helper",
            edits: &[(
                r#"  function assertDraft(key) {
    if (key !== "draft") throw new RangeError("Expected a draft section");
  }"#,
                r#"  const assertDraft = (key) => {
    if (key !== "draft") throw new RangeError("Expected a draft section");
  };"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg two parameters",
            edits: &[(
                r#"function assertDraft(key)"#,
                r#"function assertDraft(key, extra)"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg leading statement",
            edits: &[(
                r#"    if (key !== "draft") throw"#,
                r#"    log(key);
    if (key !== "draft") throw"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg trailing statement",
            edits: &[(
                r#"Expected a draft section");
  }"#,
                r#"Expected a draft section");
    log(key);
  }"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg else branch",
            edits: &[(
                r#"throw new RangeError("Expected a draft section");"#,
                r#"throw new RangeError("Expected a draft section"); else log(key);"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg other parameter",
            edits: &[(r#"if (key !== "draft")"#, r#"if (other !== "draft")"#)],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg consequence has two statements",
            edits: &[(
                r#"if (key !== "draft") throw new RangeError("Expected a draft section");"#,
                r#"if (key !== "draft") { log(key); throw new RangeError("Expected a draft section"); }"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "pos block consequence",
            edits: &[(
                r#"if (key !== "draft") throw new RangeError("Expected a draft section");"#,
                r#"if (key !== "draft") { throw new RangeError("Expected a draft section"); }"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg helper rebound",
            edits: &[(
                r#"  assertDraft(section);"#,
                r#"  assertDraft = null;
  assertDraft(section);"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg async helper",
            edits: &[(
                r#"function assertDraft(key)"#,
                r#"async function assertDraft(key)"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper is a call result",
            edits: &[(
                r#"  function assertDraft(key) {
    if (key !== "draft") throw new RangeError("Expected a draft section");
  }"#,
                r#"  const assertDraft = makeAssert();"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper written as a global",
            edits: &[(
                r#"  const defaults = Object.create(null);"#,
                r#"  globalThis.assertDraft = null;
  const defaults = Object.create(null);"#,
            )],
            states: "N",
            refusals: &[("wrong_version", 1)],
        },
    ];
    run("assertion_helper_premises", &base, &cases);
}

/// A normaliser must be trim then toLowerCase of its single parameter, applied to the checked value.
#[test]
fn normaliser_premises() {
    let base = witness("t2-152.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg key call has two arguments",
            edits: &[(
                r#"sections[normalize(section)]"#,
                r#"sections[normalize(section, 1)]"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg checked call has two arguments",
            edits: &[(
                r#"const checked = normalize(section);"#,
                r#"const checked = normalize(section, 1);"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper has two parameters",
            edits: &[(
                r#"const normalize = text => text.trim().toLowerCase();"#,
                r#"const normalize = (text, extra) => text.trim().toLowerCase();"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "pos block-bodied helper",
            edits: &[(
                r#"const normalize = text => text.trim().toLowerCase();"#,
                r#"const normalize = text => { return text.trim().toLowerCase(); };"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg block with two statements",
            edits: &[(
                r#"const normalize = text => text.trim().toLowerCase();"#,
                r#"const normalize = text => { log(text); return text.trim().toLowerCase(); };"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg block returns nothing",
            edits: &[(
                r#"const normalize = text => text.trim().toLowerCase();"#,
                r#"const normalize = text => { return; };"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg lower only",
            edits: &[(
                r#"const normalize = text => text.trim().toLowerCase();"#,
                r#"const normalize = text => text.toUpperCase().toLowerCase();"#,
            )],
            states: "N",
            refusals: &[("unsupported_predicate", 1)],
        },
        Case {
            label: "neg checked by another helper",
            edits: &[
                (
                    r#"const normalize = text => text.trim().toLowerCase();"#,
                    r#"const normalize = text => text.trim().toLowerCase();
  const other = text => text.trim().toLowerCase();"#,
                ),
                (
                    r#"const checked = normalize(section);"#,
                    r#"const checked = other(section);"#,
                ),
            ],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg checked is not a conversion",
            edits: &[(
                r#"const checked = normalize(section);"#,
                r#"const checked = section;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg predicate on the raw value",
            edits: &[(
                r#"checked !== "draft" && checked !== "final""#,
                r#"section !== "draft" && section !== "final""#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("normaliser_premises", &base, &cases);
}
