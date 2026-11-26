//! Receiver identity-set and private-brand premises.
use super::proto_writes_guard_coverage_tests::{run, witness, Case};

/// An identity set must hold only fresh, privately used entries and be tested against the receiver.
#[test]
fn identity_set_premises() {
    let base = witness("t2-118.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos entries read in a returned object",
            edits: &[(
                r#"  return panels;
}"#,
                r#"  return { panels, profile };
}"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg entry read by a call",
            edits: &[(
                r#"  return panels;
}"#,
                r#"  use(profile);
  return panels;
}"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg entry is not fresh",
            edits: &[(
                r#"  const writable = new WeakSet([profile, display]);"#,
                r#"  const extra = { shared: 1 };
  const writable = new WeakSet([profile, display, extra]);"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg two-step receiver",
            edits: &[(
                r#"const target = panels[section];"#,
                r#"const target = panels[section][field];"#,
            )],
            states: "P",
            refusals: &[],
        },
        Case {
            label: "neg receiver reassigned",
            edits: &[(
                r#"const target = panels[section];"#,
                r#"let target = panels[section];
  if (field) target = panels.profile;"#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("identity_set_premises", &base, &cases);
}

/// A private brand check needs a closed class, a single static method and one construction.
#[test]
fn private_brand_premises() {
    let base = witness("t2-153.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg public in check",
            edits: &[(r#"#editable in item"#, r#""editable" in item"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg two-step receiver",
            edits: &[(
                r#"const item = panels[key];"#,
                r#"const item = panels[key][field];"#,
            )],
            states: "P",
            refusals: &[],
        },
        Case {
            label: "neg receiver reassigned",
            edits: &[(
                r#"const item = panels[key];"#,
                r#"let item = panels[key];
      if (field) item = panels.preview;"#,
            )],
            states: "P",
            refusals: &[("wrong_version", 1)],
        },
        Case {
            label: "neg extra class member",
            edits: &[(
                r#"    #editable;
"#,
                r#"    #editable;
    other() {}
"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg class referenced elsewhere",
            edits: &[(
                r#"  Panel.mark(panels, section);"#,
                r#"  Panel.mark(panels, section);
  const alias = Panel;"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg method is not static",
            edits: &[(r#"static mark"#, r#"mark"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
    ];
    run("private_brand_premises", &base, &cases);
}

/// A one-step receiver identity check must compare the BARE receiver variable:
/// `o.tag === Object.prototype` tests a property, so it excludes nothing.
#[test]
fn one_identity_compares_the_bare_receiver() {
    let base = witness("g29.js");
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg member of the receiver",
            edits: &[("o === Object.prototype", "o.tag === Object.prototype")],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "neg index of the receiver",
            edits: &[("o === Object.prototype", "o[0] === Object.prototype")],
            states: "NN",
            refusals: &[],
        },
    ];
    run("one_identity bare receiver", &base, &cases);
}
