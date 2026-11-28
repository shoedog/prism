//! A `switch` value has no boolean polarity, so it never credits a guard.
use super::proto_writes::GuardState;
use crate::{ast::ParsedFile, languages::Language};

const LANGUAGES: [(&str, Language); 3] = [
    ("js", Language::JavaScript),
    ("ts", Language::TypeScript),
    ("tsx", Language::Tsx),
];

const PROBE: &str = "function put(root, raw, field) {
  const key = String(raw);
  SWITCH
  root[key][field] = 1;
}";

const DENY: &str = "if (a === '__proto__' || a === 'constructor' || b === '__proto__' || b === 'constructor') return;";

const NAMES: &str = "function set(root, a, b, v) {
  if (typeof a !== 'string' || typeof b !== 'string') return;
  GUARD
  root[a][b] = v;
}";

const OWN: &str = include_str!("../../../tests/proto_writes_matrix/cases/t2-120.js");
const NORMALISED: &str = include_str!("../../../tests/proto_writes_matrix/cases/t2-152.js");
const WEAKSET: &str = include_str!("../../../tests/proto_writes_matrix/cases/t2-118.js");
const ALLOW_SET: &str = include_str!("../../../tests/proto_writes_matrix/cases/t2-073.js");
const BRAND: &str = include_str!("../../../tests/proto_writes_matrix/cases/t2-153.js");

fn states(source: &str, ext: &str, language: Language) -> Vec<GuardState> {
    let file = ParsedFile::parse(&format!("case.{ext}"), source, language).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    file.js_ts_proto_writes().iter().map(|f| f.guard).collect()
}

fn pair(control: &str, negative: &str, expected: GuardState) {
    for (ext, language) in LANGUAGES {
        let got = states(control, ext, language);
        assert!(
            got.contains(&GuardState::Sufficient),
            "{ext} control {got:?}\n{control}"
        );
        let got = states(negative, ext, language);
        assert!(!got.is_empty(), "{ext} no fact\n{negative}");
        assert!(
            got.iter().all(|state| *state == expected),
            "{ext} negative {got:?}, want {expected:?}\n{negative}"
        );
    }
}

#[test]
fn a_switch_over_deny_names_is_not_a_deny_list() {
    let value =
        "a === '__proto__' || a === 'constructor' || b === '__proto__' || b === 'constructor'";
    let control = NAMES.replace("GUARD", DENY);
    let negative = NAMES.replace(
        "GUARD",
        &format!("switch ({value}) {{ case true: console.warn('reserved'); break; }}"),
    );
    pair(&control, &negative, GuardState::None);
    let control = PROBE.replace(
        "SWITCH",
        "if (key === '__proto__' || key === 'constructor') return;",
    );
    let negative = PROBE.replace(
        "SWITCH",
        "switch (key === '__proto__' || key === 'constructor') { case true: break; }",
    );
    pair(&control, &negative, GuardState::None);
}

#[test]
fn a_switch_over_a_typeof_test_proves_no_string() {
    let strong = NORMALISED;
    let negative = NORMALISED.replace(
        "if (typeof section !== \"string\") return sections;",
        "switch (typeof section !== \"string\") {}",
    );
    assert_ne!(strong, negative);
    pair(strong, &negative, GuardState::None);
}

#[test]
fn a_switch_over_has_own_is_not_an_own_check() {
    let negative = OWN.replace(
        "if (!Object.hasOwn(panels, key)) return panels;",
        "switch (!Object.hasOwn(panels, key)) {}",
    );
    pair(OWN, &negative, GuardState::None);
}

#[test]
fn a_switch_over_a_set_or_brand_test_is_not_a_receiver_check() {
    let negative = WEAKSET.replace(
        "if (!writable.has(target)) return panels;",
        "switch (!writable.has(target)) {}",
    );
    assert_ne!(WEAKSET, negative);
    pair(WEAKSET, &negative, GuardState::None);
    let control = BRAND.replace(
        "if (#editable in item) item[field] = true;",
        "if (!(#editable in item)) return;\n      item[field] = true;",
    );
    let negative = BRAND.replace(
        "if (#editable in item) item[field] = true;",
        "switch (!(#editable in item)) {}\n      item[field] = true;",
    );
    pair(&control, &negative, GuardState::None);
}

#[test]
fn a_switch_over_a_reserved_name_test_is_not_even_a_partial_check() {
    let control = PROBE.replace("SWITCH", "if (key === '__proto__') return;");
    let negative = PROBE.replace("SWITCH", "switch (key === '__proto__') {}");
    for (ext, language) in LANGUAGES {
        assert_eq!(
            states(&control, ext, language),
            [GuardState::Partial],
            "{ext}"
        );
        assert_eq!(
            states(&negative, ext, language),
            [GuardState::None],
            "{ext}"
        );
    }
}

fn refusals(
    source: &str,
    ext: &str,
    language: Language,
) -> std::collections::BTreeMap<&'static str, usize> {
    let file = ParsedFile::parse(&format!("case.{ext}"), source, language).unwrap();
    file.js_ts_proto_write_guard_refusals()
}

#[test]
fn refusals_are_counted_only_for_facts_below_sufficient() {
    for (ext, language) in LANGUAGES {
        for source in [OWN, NORMALISED, WEAKSET, BRAND] {
            let file = ParsedFile::parse(&format!("case.{ext}"), source, language).unwrap();
            let facts = file.js_ts_proto_writes();
            assert!(
                facts.iter().all(|f| f.guard == GuardState::Sufficient),
                "{ext}"
            );
            assert_eq!(
                file.js_ts_proto_write_guard_refusals(),
                Default::default(),
                "{ext}"
            );
        }
    }
}

#[test]
fn unrecognised_predicate_shapes_are_counted() {
    let helper = NORMALISED.replace("text.trim().toLowerCase()", "text.trim()");
    let set = WEAKSET.replace("new WeakSet(", "new Set(");
    for (ext, language) in LANGUAGES {
        for (source, what) in [(&helper, "normalise"), (&set, "identity set")] {
            let states = states(source, ext, language);
            assert!(
                states.iter().all(|s| *s != GuardState::Sufficient),
                "{ext} {what}"
            );
            let got = refusals(source, ext, language);
            assert!(
                got.contains_key("unsupported_predicate"),
                "{ext} {what} {got:?}"
            );
        }
    }
}

#[test]
fn a_refusal_discharged_by_a_later_form_is_not_counted() {
    let source = ALLOW_SET.replace(
        "if (!allowed.has(key)) return root;",
        "if (key === \"__proto__\") return root;\n    if (!allowed.has(key)) return root;",
    );
    assert_ne!(source, ALLOW_SET);
    for (ext, language) in LANGUAGES {
        assert_eq!(
            states(&source, ext, language),
            [GuardState::Sufficient],
            "{ext}"
        );
        assert_eq!(
            refusals(&source, ext, language),
            Default::default(),
            "{ext}"
        );
    }
}
