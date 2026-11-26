use super::proto_writes::{GuardState, ProtoWriteClass};
use crate::{ast::ParsedFile, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! witness {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/proto_writes_matrix/cases/",
            $file
        ))
    };
}

#[path = "proto_writes_guard_fixtures/names.rs"]
mod names;
#[path = "proto_writes_guard_fixtures/placement.rs"]
mod placement;
#[path = "proto_writes_guard_fixtures/receivers.rs"]
mod receivers;

type Variant = (&'static str, Language, (usize, usize));

struct Shape {
    source: &'static str,
    witness: &'static str,
    form: &'static str,
    class: ProtoWriteClass,
    store: &'static str,
    state: GuardState,
    refusal: Option<&'static str>,
    edits: &'static [(&'static str, &'static str)],
    variants: [Variant; 3],
}

impl Shape {
    fn fixture(&self, language: Language) -> (String, String) {
        let mut source = self.source.to_owned();
        for (before, after) in self.edits {
            assert!(source.contains(before), "{}: {before}", self.witness);
            source = source.replace(before, after);
        }
        let (target, rhs) = self.store.split_once(" = ").unwrap();
        assert_eq!(source.matches(self.store).count(), 1, "{}", self.witness);
        if language == Language::JavaScript {
            if self.witness == "t2-146" || self.witness == "t2-145" {
                // The JS control erases only syntax. Helper statements retain their
                // original runtime behavior, including the normally returning twin.
                for annotation in [
                    ": Record<string, Record<string, unknown> | undefined>",
                    ": asserts key is \"draft\"",
                    ": Record<string, unknown>",
                    ": [string, string]",
                    ": unknown",
                    ": string[]",
                    ": string",
                    ": key is \"draft\"",
                ] {
                    source = source.replace(annotation, "");
                }
            }
            return (source, target.to_owned());
        }

        // Angle-bracket assertions are TS-only. Both typed parsers support
        // `as` and non-null wrappers. JSX expression containers are TSX-only.
        let (receiver, keys) = target.split_once('[').unwrap();
        let target = format!("({receiver} as any)![{keys}");
        let store = if language == Language::Tsx {
            format!("void <>{{({target} = {rhs})}}</>")
        } else {
            format!("{target} = {rhs}")
        };
        (source.replace(self.store, &store), target)
    }
}

fn shapes() -> impl Iterator<Item = &'static Shape> {
    names::SHAPES
        .iter()
        .chain(receivers::SHAPES)
        .chain(placement::SHAPES)
}

#[test]
fn grammar_rows_cover_each_enabled_form_in_each_language() {
    let mut ids = BTreeSet::new();
    let mut forms = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for shape in shapes() {
        assert_eq!(
            shape.refusal.is_none(),
            shape.state == GuardState::Sufficient
        );
        for (id, language, _) in shape.variants {
            assert!(ids.insert(id), "duplicate grammar row {id}");
            let (extension, rank) = match language {
                Language::JavaScript => (".js", 0),
                Language::TypeScript => (".ts", 1),
                Language::Tsx => (".tsx", 2),
                other => panic!("{id}: unexpected language {other:?}"),
            };
            assert!(id.ends_with(&extension.replace('.', "-")), "{id}");
            *counts
                .entry(shape.state)
                .or_insert([0; 3])
                .get_mut(rank)
                .unwrap() += 1;
            if shape.state == GuardState::Sufficient {
                forms.entry(shape.form).or_insert([false; 3])[rank] = true;
            }
        }
    }
    // g01 is a deny_names for-in witness whose recursive facts belong to S3.
    assert_eq!(forms.len(), 12);
    assert!(forms.values().all(|coverage| *coverage == [true; 3]));
    assert_eq!(counts[&GuardState::None], [23; 3]);
    assert_eq!(counts[&GuardState::Partial], [25; 3]);
    assert_eq!(counts[&GuardState::Sufficient], [16; 3]);
}

fn parse_variant(shape: &Shape, (id, language, (start, end)): Variant) -> ParsedFile {
    let (source, target) = shape.fixture(language);
    assert_eq!(
        source.get(start..end),
        Some(target.as_str()),
        "{id}: target span"
    );
    let path = format!("{id}.{}", id.rsplit_once('-').unwrap().1);
    let parsed = ParsedFile::parse(&path, &source, language).unwrap();
    let tree = parsed.tree.root_node();
    assert!(!tree.has_error(), "{id}: {}", tree.to_sexp());
    parsed
}

#[test]
fn grammar_rows_keep_exact_designated_facts() {
    let mut failures = Vec::new();
    for shape in shapes() {
        for (id, language, (start, end)) in shape.variants {
            let parsed = parse_variant(shape, (id, language, (start, end)));
            let facts = parsed.js_ts_proto_writes();
            if !facts.iter().any(|fact| {
                (fact.start_byte, fact.end_byte, fact.class) == (start, end, shape.class)
            }) {
                let observed: Vec<_> = facts
                    .iter()
                    .map(|fact| (fact.start_byte, fact.end_byte, fact.class))
                    .collect();
                failures.push(format!(
                    "{id} [{start}, {end}) {:?} vs {observed:?}",
                    shape.class
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} missing targets\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn grammar_rows_classify_the_designated_facts() {
    let mut failures = Vec::new();
    for shape in shapes() {
        for (id, language, (start, end)) in shape.variants {
            let parsed = parse_variant(shape, (id, language, (start, end)));
            let facts = parsed.js_ts_proto_writes();
            let actual = facts.iter().find(|fact| {
                (fact.start_byte, fact.end_byte, fact.class) == (start, end, shape.class)
            });
            let names = actual.map(|fact| serde_json::to_value(fact).unwrap());
            let forms = names.as_ref().and_then(|value| value.get("guard_forms"));
            let passed = match (actual, shape.state) {
                (Some(fact), GuardState::Sufficient) => {
                    fact.guard == shape.state && forms == Some(&serde_json::json!([shape.form]))
                }
                (Some(fact), _) => fact.guard == shape.state && forms.is_none(),
                (None, _) => false,
            };
            if !passed {
                failures.push(format!(
                    "{id} [{start}, {end}): {:?} {} {:?}, observed {:?} {forms:?}",
                    shape.state,
                    shape.form,
                    shape.refusal,
                    actual.map(|fact| fact.guard)
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} guard mismatches\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn intrinsic_refusals_count_the_conversion_occurrence_and_retain_the_fact() {
    const CONTROLS: &[&str] = &["deny-native-", "deny-runtime-", "deny-unrelated-member-"];
    let mut checked = 0;
    for shape in names::SHAPES {
        for (id, language, (start, end)) in shape.variants {
            let refused = id.starts_with("deny-") && shape.refusal == Some("intrinsic_rebound");
            if !refused && !CONTROLS.iter().any(|prefix| id.starts_with(prefix)) {
                continue;
            }
            let parsed = parse_variant(shape, (id, language, (start, end)));
            let facts = parsed.js_ts_proto_writes();
            let designated = facts.iter().find(|fact| {
                (fact.start_byte, fact.end_byte, fact.class) == (start, end, shape.class)
            });
            assert_eq!(designated.map(|fact| fact.guard), Some(shape.state), "{id}");
            assert_eq!(
                parsed
                    .js_ts_proto_write_guard_refusals()
                    .get("intrinsic_rebound")
                    .copied(),
                refused.then_some(1),
                "{id}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 24);
}

#[test]
fn grammar_rows_count_the_expected_refusals() {
    let mut failures = Vec::new();
    for shape in shapes() {
        let Some(expected) = shape.refusal else {
            continue;
        };
        for variant in shape.variants {
            let parsed = parse_variant(shape, variant);
            let actual = parsed.js_ts_proto_write_guard_refusals();
            if !actual.contains_key(expected) {
                failures.push(format!(
                    "{} expects {expected}, observed {actual:?}",
                    variant.0
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_visible_shared_receiver_defeats_the_name_proof() {
    let control = "function set(raw, field) {
        const table = { draft: {} };
        const key = String(raw);
        if (key === '__proto__' || key === 'constructor') return;
        table[key][field] = 1;
    }";
    for (child, state) in [
        ("{}", GuardState::Sufficient),
        ("Object.prototype", GuardState::Partial),
        ("Object['prototype']", GuardState::Partial),
        ("globalThis.Object['prototype']", GuardState::Partial),
    ] {
        let source = control.replace("draft: {}", &format!("draft: {child}"));
        let file = ParsedFile::parse("shared.js", &source, Language::JavaScript).unwrap();
        let facts = file.js_ts_proto_writes();
        let [fact] = facts.as_slice() else {
            panic!("{child}: {facts:?}")
        };
        assert_eq!(fact.guard, state, "{child}");
        if state == GuardState::Partial {
            assert!(
                file.js_ts_proto_write_guard_refusals()
                    .contains_key("unproved_child"),
                "{child}"
            );
        }
    }
}

#[test]
fn unrelated_try_does_not_create_a_guard_refusal() {
    for statements in [
        "try { observe(); } catch (error) { record(error); } child[field] = 1;",
        "try { observe(); } finally { child[field] = 1; }",
    ] {
        let source =
            format!("function f(root, key, field) {{ const child = root[key]; {statements} }}");
        let file = ParsedFile::parse("try.js", &source, Language::JavaScript).unwrap();
        let facts = file.js_ts_proto_writes();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].guard, GuardState::None);
        assert!(file.js_ts_proto_write_guard_refusals().is_empty());
    }
}
