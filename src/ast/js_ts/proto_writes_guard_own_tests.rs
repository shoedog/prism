//! Own-slot, clone and diagonal premises.
use super::proto_writes_guard_coverage_tests::{run, witness, Case};

/// Own-slot proofs need a native hasOwn, a reflected key and a closed slot table.
#[test]
fn own_slot_premises() {
    let base = witness("t2-120.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg Object rebound",
            edits: &[(
                r#"  const displaySymbol"#,
                r#"  const Object = globalThis.Object;
  const displaySymbol"#,
            )],
            states: "P",
            refusals: &[("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg hasOwn written",
            edits: &[(
                r#"function annotateDisplay("#,
                r#"Object.hasOwn = () => true;
function annotateDisplay("#,
            )],
            states: "P",
            refusals: &[("intrinsic_rebound", 1)],
        },
        Case {
            label: "neg keys not reflected",
            edits: &[(
                r#"Reflect.ownKeys({ [section]: true })"#,
                r#"Object.keys({ [section]: true })"#,
            )],
            states: "P",
            refusals: &[("unstable_key", 1)],
        },
        Case {
            label: "neg ownKeys has two arguments",
            edits: &[(
                r#"Reflect.ownKeys({ [section]: true })"#,
                r#"Reflect.ownKeys({ [section]: true }, 1)"#,
            )],
            states: "P",
            refusals: &[("unstable_key", 1)],
        },
        Case {
            label: "neg key is not destructured",
            edits: &[(
                r#"const [key] = Reflect.ownKeys({ [section]: true });"#,
                r#"let key;
  [key] = Reflect.ownKeys({ [section]: true });"#,
            )],
            states: "P",
            refusals: &[("unstable_key", 1)],
        },
        Case {
            label: "neg key is an array element",
            edits: &[(
                r#"const [key] = Reflect.ownKeys({ [section]: true });"#,
                r#"const [key] = [section];"#,
            )],
            states: "P",
            refusals: &[("unstable_key", 1)],
        },
        Case {
            label: "neg duplicate slot",
            edits: &[(
                r#"{ display: {}, [displaySymbol]: {} }"#,
                r#"{ display: {}, display: {}, [displaySymbol]: {} }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg duplicate shorthand slot",
            edits: &[(
                r#"{ display: {}, [displaySymbol]: {} }"#,
                r#"{ display, display, [displaySymbol]: {} }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg prototype slot is an object",
            edits: &[(
                r#"{ display: {}, [displaySymbol]: {} }"#,
                r#"{ __proto__: {}, display: {}, [displaySymbol]: {} }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "pos prototype slot is null",
            edits: &[(
                r#"{ display: {}, [displaySymbol]: {} }"#,
                r#"{ __proto__: null, display: {}, [displaySymbol]: {} }"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg spread slot",
            edits: &[(
                r#"{ display: {}, [displaySymbol]: {} }"#,
                r#"{ ...extra, display: {}, [displaySymbol]: {} }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg keyFor symbol",
            edits: &[(r#"Symbol.for("display")"#, r#"Symbol.keyFor("display")"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg symbol rebound",
            edits: &[(
                r#"  const displaySymbol"#,
                r#"  const Symbol = globalThis.Symbol;
  const displaySymbol"#,
            )],
            states: "P",
            refusals: &[("intrinsic_rebound", 1), ("unproved_child", 1)],
        },
        Case {
            label: "neg dynamic symbol name",
            edits: &[(r#"Symbol.for("display")"#, r#"Symbol.for(section)"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "pos slot shares a child",
            edits: &[
                (
                    r#"  const panels"#,
                    r#"  const shared = {};
  const panels"#,
                ),
                (
                    r#"{ display: {}, [displaySymbol]: {} }"#,
                    r#"{ display: shared, other: shared, [displaySymbol]: {} }"#,
                ),
            ],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos labelled break",
            edits: &[(
                r#"  if (!Object.hasOwn(panels, key)) return panels;
  panels[key][field] = value;"#,
                r#"  guard: {
    if (!Object.hasOwn(panels, key)) break guard;
    panels[key][field] = value;
  }"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg unlabelled break",
            edits: &[(
                r#"  if (!Object.hasOwn(panels, key)) return panels;
  panels[key][field] = value;"#,
                r#"  for (const once of [0]) {
    if (!Object.hasOwn(panels, key)) break;
    panels[key][field] = value;
  }"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg break inside try around the store",
            edits: &[(
                r#"  if (!Object.hasOwn(panels, key)) return panels;
  panels[key][field] = value;"#,
                r#"  guard: {
    try {
      if (!Object.hasOwn(panels, key)) break guard;
      panels[key][field] = value;
    } finally {}
  }"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "pos legacy hasOwnProperty",
            edits: &[(
                r#"Object.hasOwn(panels, key)"#,
                r#"Object.prototype.hasOwnProperty.call(panels, key)"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg legacy call on another namespace",
            edits: &[(
                r#"Object.hasOwn(panels, key)"#,
                r#"Foo.prototype.hasOwnProperty.call(panels, key)"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "neg key is a parameter pattern",
            edits: &[
                (
                    r#"const [key] = Reflect.ownKeys({ [section]: true });"#,
                    r#""#,
                ),
                (
                    r#"function annotateDisplay(section, field, value)"#,
                    r#"function annotateDisplay(section, field, value, [key])"#,
                ),
            ],
            states: "P",
            refusals: &[("unstable_key", 1)],
        },
        Case {
            label: "neg break leaves a block after the store",
            edits: &[(
                r#"  if (!Object.hasOwn(panels, key)) return panels;
  panels[key][field] = value;"#,
                r#"  guard: {
    if (!Object.hasOwn(panels, key)) break guard;
  }
  panels[key][field] = value;"#,
            )],
            states: "N",
            refusals: &[],
        },
        Case {
            label: "pos throwing helper",
            edits: &[
                (
                    r#"  if (!Object.hasOwn(panels, key)) return panels;"#,
                    r#"  if (!Object.hasOwn(panels, key)) fail();"#,
                ),
                (
                    r#"function annotateDisplayAtPath("#,
                    r#"function fail() { throw new Error("no"); }

function annotateDisplayAtPath("#,
                ),
            ],
            states: "S",
            refusals: &[],
        },
    ];
    run("own_slot_premises", &base, &cases);
}

/// A fallback store in a block is read like a bare one but stays partial.
#[test]
fn own_fallback_blocks() {
    let base = include_str!("proto_writes_guard_fixtures/own_fallback.js").to_owned();
    let cases = [
        Case {
            label: "ctl",
            edits: &[],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unproved_child", 1)],
        },
        Case {
            label: "var block fallback",
            edits: &[(
                r#"if (!Object.hasOwn(table, key)) table[key] = {};"#,
                r#"if (!Object.hasOwn(table, key)) { table[key] = {}; }"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unproved_child", 1)],
        },
        Case {
            label: "var block of two statements",
            edits: &[(
                r#"if (!Object.hasOwn(table, key)) table[key] = {};"#,
                r#"if (!Object.hasOwn(table, key)) { log(key); table[key] = {}; }"#,
            )],
            states: "N",
            refusals: &[],
        },
    ];
    run("own_fallback_blocks", &base, &cases);
}

/// A structured clone needs a closed, privately held source graph.
#[test]
fn closed_clone_premises() {
    let base = witness("t2-170.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg clone has two arguments",
            edits: &[(r#"structuredClone(live)"#, r#"structuredClone(live, {})"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg other clone function",
            edits: &[(r#"structuredClone(live)"#, r#"copy(live)"#)],
            states: "P",
            refusals: &[("unproved_child", 1), ("unproved_native_identity", 1)],
        },
        Case {
            label: "neg spread member",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ defaults, mirror: defaults, card: Object.create(defaults), ...extra }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg duplicate shorthand",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ defaults, defaults, mirror: defaults, card: Object.create(defaults) }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg duplicate key",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ defaults, mirror: defaults, mirror: defaults, card: Object.create(defaults) }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg prototype slot is an object",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ __proto__: {}, defaults, mirror: defaults, card: Object.create(defaults) }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "pos prototype slot is null",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ __proto__: null, defaults, mirror: defaults, card: Object.create(defaults) }"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "pos leaves and null prototype",
            edits: &[(
                r#"{ defaults, mirror: defaults, card: Object.create(defaults) }"#,
                r#"{ defaults, mirror: defaults, card: Object.create(null), n: 1, s: "x", t: true, z: null }"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg unknown call",
            edits: &[(r#"card: Object.create(defaults)"#, r#"card: make()"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg unknown prototype",
            edits: &[(
                r#"card: Object.create(defaults)"#,
                r#"card: Object.create(unknownProto)"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg undeclared name",
            edits: &[(r#"card: Object.create(defaults)"#, r#"card: nothing"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg parameter value",
            edits: &[(r#"card: Object.create(defaults)"#, r#"card: section"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg array element is a call",
            edits: &[(
                r#"card: Object.create(defaults)"#,
                r#"card: Object.create(defaults), list: [1, make()]"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "pos array of leaves",
            edits: &[(
                r#"card: Object.create(defaults)"#,
                r#"card: Object.create(defaults), list: [1, 2]"#,
            )],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg function member",
            edits: &[(
                r#"card: Object.create(defaults)"#,
                r#"card: Object.create(defaults), run: () => 1"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg source is a call result",
            edits: &[(
                r#"const live = { defaults, mirror: defaults, card: Object.create(defaults) };"#,
                r#"const live = Object.freeze({ defaults, mirror: defaults, card: Object.create(defaults) });"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg source graph read elsewhere",
            edits: &[(
                r#"  const snapshot = structuredClone(live);"#,
                r#"  const snapshot = structuredClone(live);
  use(defaults);"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
    ];
    run("closed_clone_premises", &base, &cases);
}

/// A stable diagonal needs closed rows whose diagonal children are private.
#[test]
fn diagonal_premises() {
    let base = witness("t2-166.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg row is not a table",
            edits: &[(r#"right: { left: defaults, right }"#, r#"right: right"#)],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "neg diagonal child is a prototype",
            edits: &[(
                r#"left: { left, right: defaults }"#,
                r#"left: { left: defaults, right: defaults }"#,
            )],
            states: "P",
            refusals: &[("unproved_child", 1)],
        },
        Case {
            label: "pos symbol slot is skipped",
            edits: &[(
                r#"  const lanes = {"#,
                r#"  const mark = Symbol.for("mark");
  const lanes = {
    [mark]: { left },"#,
            )],
            states: "S",
            refusals: &[],
        },
    ];
    run("diagonal_premises", &base, &cases);
}
