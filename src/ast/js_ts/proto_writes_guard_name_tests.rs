//! Name, set, path and switch premises.
use super::proto_writes_guard_coverage_tests::{run, witness, Case};

/// Deny-list atoms bind the checked value to the stored key and to a closed domain.
#[test]
fn deny_name_atoms() {
    let base = r#"function put(root, raw, field) {
  const key = String(raw);
  if (key === '__proto__' || key === 'constructor') return;
  root[key][field] = 1;
}
"#
    .to_owned();
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg checked value is a member",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (root.name === '__proto__' || root.name === 'constructor') return;"#,
            )],
            states: "N",
            refusals: &[("unstable_key", 2)],
        },
        Case {
            label: "neg checked value is undeclared",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (k === '__proto__' || k === 'constructor') return;"#,
            )],
            states: "N",
            refusals: &[("wrong_binding", 2)],
        },
        Case {
            label: "neg test on a non-literal regex",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (pattern.test(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg test without a value",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (/^__proto__$/.test()) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper bound by const",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"const isBad = k => k === '__proto__' || k === 'constructor';
  if (isBad(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper without parameters",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"function isBad() { return key === '__proto__'; }
  if (isBad(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper with an empty body",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"function isBad(k) {}
  if (isBad(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper returns nothing",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"function isBad(k) { return; }
  if (isBad(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper called with two arguments",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"function isBad(k) { return k === '__proto__' || k === 'constructor'; }
  if (isBad(key, 1)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg helper not declared",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (isBad(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg optional chained predicate",
            edits: &[(
                r#"if (key === '__proto__' || key === 'constructor') return;"#,
                r#"if (blocked?.has(key)) return;"#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("deny_name_atoms", &base, &cases);
}

/// A closed allow-list set must be a native Set of plain names and be tested with the key.
#[test]
fn allow_set_premises() {
    let base = witness("t2-073.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg has without argument",
            edits: &[(r#"allowed.has(key)"#, r#"allowed.has()"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg has another key",
            edits: &[(r#"allowed.has(key)"#, r#"allowed.has(property)"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg map instead of set",
            edits: &[(
                r#"new Set(["settings", "profile", "display"])"#,
                r#"new Map([["settings", 1], ["profile", 2]])"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg empty set",
            edits: &[(
                r#"new Set(["settings", "profile", "display"])"#,
                r#"new Set()"#,
            )],
            states: "P",
            refusals: &[("unproved_native_identity", 1)],
        },
        Case {
            label: "neg set with two arguments",
            edits: &[(
                r#"new Set(["settings", "profile", "display"])"#,
                r#"new Set(["settings"], 1)"#,
            )],
            states: "P",
            refusals: &[("unproved_native_identity", 1)],
        },
        Case {
            label: "neg cursor starts at a shared prototype",
            edits: &[(r#"let cursor = root;"#, r#"let cursor = Object.prototype;"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg cursor starts at a maybe shared prototype",
            edits: &[(
                r#"let cursor = root;"#,
                r#"let cursor = flag ? Object.prototype : {};"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg effect inside try with finally",
            edits: &[(
                r#"    if (!allowed.has(key)) return root;
    cursor = cursor[key];"#,
                r#"    try {
      if (!allowed.has(key)) return root;
      cursor = cursor[key];
    } finally {
      log();
    }"#,
            )],
            states: "N",
            refusals: &[("wrong_exit", 1)],
        },
        Case {
            label: "neg throw inside try with handler",
            edits: &[(
                r#"    if (!allowed.has(key)) return root;
    cursor = cursor[key];"#,
                r#"    try {
      if (!allowed.has(key)) throw root;
      cursor = cursor[key];
    } catch (error) {
      return root;
    }"#,
            )],
            states: "N",
            refusals: &[("wrong_exit", 1)],
        },
        Case {
            label: "pos labelled continue",
            edits: &[
                (r#"  for (const key"#, r#"  outer: for (const key"#),
                (
                    r#"if (!allowed.has(key)) return root;"#,
                    r#"if (!allowed.has(key)) continue outer;"#,
                ),
            ],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos earlier try with a throwing string check",
            edits: &[(
                r#"    cursor = cursor[key];
    if (cursor == null)"#,
                r#"    try { if (key !== "settings") throw root; } catch (e) {}
    cursor = cursor[key];
    if (cursor == null)"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos earlier try with a break",
            edits: &[(
                r#"    cursor = cursor[key];
    if (cursor == null)"#,
                r#"    try { if (key !== "settings") break; } finally {}
    cursor = cursor[key];
    if (cursor == null)"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos earlier try with a nested callable",
            edits: &[(
                r#"    cursor = cursor[key];
    if (cursor == null)"#,
                r#"    try { const f = () => 1; } catch (e) {}
    cursor = cursor[key];
    if (cursor == null)"#,
            )],
            states: "S",
            refusals: &[],
        },
    ];
    run("allow_set_premises", &base, &cases);
}

/// An exact path allow-list needs a split on a plain path with a non-empty delimiter and no reserved segment.
#[test]
fn exact_path_premises() {
    let base = witness("t2-083.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg empty delimiter",
            edits: &[(r#"path.split(".")"#, r#"path.split("")"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg split of a call result",
            edits: &[(r#"path.split(".")"#, r#"String(path).split(".")"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg reserved segment in the domain",
            edits: &[(
                r#"path !== "settings.display""#,
                r#"path !== "settings.__proto__""#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("exact_path_premises", &base, &cases);
}

/// A literal check after a lookup needs a literal table of fresh slots.
#[test]
fn literal_after_lookup_premises() {
    let base = witness("t2-121.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg table is a call",
            edits: &[(
                r#"const panels = { draft: {}, defaults };"#,
                r#"const panels = makePanels(defaults);"#,
            )],
            states: "N",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg slot is not fresh",
            edits: &[(r#"{ draft: {}, defaults }"#, r#"{ draft: 1, defaults }"#)],
            states: "N",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg duplicate slot",
            edits: &[(
                r#"{ draft: {}, defaults }"#,
                r#"{ draft: {}, draft: {}, defaults }"#,
            )],
            states: "N",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg spread in table",
            edits: &[(
                r#"{ draft: {}, defaults }"#,
                r#"{ draft: {}, ...defaults }"#,
            )],
            states: "N",
            refusals: &[("unproved_child", 1)],
        },
    ];
    run("literal_after_lookup_premises", &base, &cases);
}

/// A switch over the key blocks names only when every case returns.
#[test]
fn switch_cases() {
    let base = r#"function put(root, raw, field) {
  const key = String(raw);
  switch (key) { case '__proto__': case 'constructor': return; }
  root[key][field] = 1;
}
"#
    .to_owned();
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "P",
            refusals: &[("absent_domain", 1)],
        },
        Case {
            label: "var default case",
            edits: &[(r#"return; }"#, r#"return; default: log(); }"#)],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "var reserved case breaks",
            edits: &[(
                r#"case 'constructor': return;"#,
                r#"case 'constructor': break;"#,
            )],
            states: "N",
            refusals: &[("wrong_exit", 1)],
        },
    ];
    run("switch_cases", &base, &cases);
}
