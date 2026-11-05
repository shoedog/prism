//! The grammar certificate (DESIGN.md 4.2, 18.2): checks 0-5 over the three grammars and the ten
//! negative controls. The Python reference (`census/check.py`) prints
//! `PASS: 0 problem(s); slots 142, rows 142, leaves 33, target kinds 78, order rows 186` on the pinned
//! grammars; the baseline test pins the same line, and each negative control pins the same problem count.

use super::certificate::{certify, Grammar, Report};
use super::schema;
use super::tables::{self, Tables};
use crate::languages::Language;

const BASELINE: &str =
    "PASS: 0 problem(s); slots 142, rows 142, leaves 33, target kinds 78, order rows 186";
const SIGS: [(&str, &str, Language); 3] = [
    (
        "javascript",
        include_str!("data/signatures/javascript.sig"),
        Language::JavaScript,
    ),
    (
        "typescript",
        include_str!("data/signatures/typescript.sig"),
        Language::TypeScript,
    ),
    (
        "tsx",
        include_str!("data/signatures/tsx.sig"),
        Language::Tsx,
    ),
];

macro_rules! neg {
    ($f:literal) => {
        include_str!(concat!(
            "../../../tests/fixtures/js_write_census/negative/",
            $f
        ))
    };
}

fn grammars(js_override: Option<&str>, signatures: bool) -> Vec<Grammar> {
    SIGS.iter()
        .map(|(name, sig, lang)| {
            let json = match (*name, js_override) {
                ("javascript", Some(j)) => j,
                _ => lang.node_types_json(),
            };
            Grammar {
                name,
                schema: schema::parse(json).expect("node-types.json parses"),
                recorded_signature: signatures.then(|| {
                    sig.lines()
                        .filter(|l| !l.is_empty())
                        .map(str::to_string)
                        .collect()
                }),
            }
        })
        .collect()
}

fn embedded() -> Tables {
    Tables::embedded()
}

fn run(js: Option<&str>, signatures: bool, t: &Tables) -> Report {
    certify(&grammars(js, signatures), t)
}

/// Problems whose diagnostic token (first word) is in `tokens`.
fn problems<'r>(r: &'r Report, tokens: &[&str]) -> Vec<&'r String> {
    r.problems
        .iter()
        .filter(|p| tokens.contains(&p.split(' ').next().unwrap_or("")))
        .collect()
}

const CHECK0: &[&str] = &["DUPLICATE-ROW", "MALFORMED-ROW"];
const CHECK1: &[&str] = &["SIGNATURE-ADDED", "SIGNATURE-REMOVED"];
const CHECK2: &[&str] = &["UNCLASSIFIED", "STALE", "BAD-ROLE", "BAD-ORDER"];
const CHECK3: &[&str] = &["UNCLASSIFIED-LEAF", "STALE-LEAF", "BIND-SET-DRIFT"];
const CHECK4: &[&str] = &[
    "TARGET-KIND-UNCLASSIFIED",
    "CARRYING-SLOT-MISSING",
    "TARGET-ROLE-LOSS",
    "STALE-TARGET",
];
const CHECK5: &[&str] = &[
    "UNCLASSIFIED-ORDER",
    "STALE-ORDER",
    "BAD-ORDER-CLASS",
    "ORDER-LEAF-MISMATCH",
];

#[test]
fn baseline_matches_the_python_reference() {
    let r = run(None, true, &embedded());
    assert_eq!(r.summary(), BASELINE, "{:?}", r.problems);
}

#[test]
fn check0_duplicates_none_in_embedded_tables() {
    assert!(embedded().diagnostics.is_empty());
    assert!(problems(&run(None, true, &embedded()), CHECK0).is_empty());
}

#[test]
fn check1_signature_matches_for_each_grammar() {
    for g in grammars(None, true) {
        let mut bad = Vec::new();
        super::certificate::check_signature(&g, &mut bad);
        assert!(bad.is_empty(), "{}: {bad:?}", g.name);
    }
}

#[test]
fn check2_coverage_exact_over_three_grammars() {
    assert!(problems(&run(None, true, &embedded()), CHECK2).is_empty());
}

#[test]
fn check3_leaves_and_bind_set() {
    let t = embedded();
    assert!(problems(&run(None, true, &t), CHECK3).is_empty());
    let name_kinds: Vec<_> = t
        .leaves
        .iter()
        .filter(|(_, l)| l.name_token)
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(name_kinds.len(), 8, "{name_kinds:?}");
}

#[test]
fn check4_target_closure() {
    assert!(problems(&run(None, true, &embedded()), CHECK4).is_empty());
}

#[test]
fn check5_order_rows_match_kinds_and_leaves() {
    assert!(problems(&run(None, true, &embedded()), CHECK5).is_empty());
}

#[test]
fn closed_vocabularies_are_complete() {
    for row in embedded().slots.values() {
        assert!(
            tables::role_is_valid(&row.role) && tables::ORDER_HINTS.contains(&row.order.as_str())
        );
    }
}

fn expect(r: &Report, n: usize, tokens: &[&str]) {
    assert_eq!(r.problems.len(), n, "{:#?}", r.problems);
    for t in tokens {
        assert!(r.has(t), "missing {t} in {:#?}", r.problems);
    }
}

#[test]
fn n1_new_composite_in_target() {
    let r = run(
        Some(neg!("n1_new_composite_in_target/javascript.json")),
        true,
        &embedded(),
    );
    expect(
        &r,
        9,
        &[
            "TARGET-KIND-UNCLASSIFIED javascript future_target via assignment_expression left",
            "UNCLASSIFIED-ORDER future_target",
            "SIGNATURE-ADDED javascript KIND\tfuture_target",
        ],
    );
    let r = run(
        Some(neg!("n1_new_composite_in_target/javascript.json")),
        false,
        &embedded(),
    );
    expect(
        &r,
        2,
        &[
            "TARGET-KIND-UNCLASSIFIED",
            "UNCLASSIFIED-ORDER future_target",
        ],
    );
}

#[test]
fn n2_widened_target_slot_existing_kind() {
    let js = Some(neg!("n2_widened_target_slot_existing_kind/javascript.json"));
    expect(
        &run(js, true, &embedded()),
        1,
        &["SIGNATURE-ADDED javascript ALT\tassignment_expression\tleft\tcall_expression"],
    );
    // Only the signature guards this mutation: without it the other checks pass.
    expect(&run(js, false, &embedded()), 0, &[]);
}

#[test]
fn n3_widened_slot_gains_identifier() {
    let js = Some(neg!("n3_widened_slot_gains_identifier/javascript.json"));
    expect(
        &run(js, true, &embedded()),
        2,
        &["UNCLASSIFIED labeled_statement label", "SIGNATURE-ADDED"],
    );
    expect(
        &run(js, false, &embedded()),
        1,
        &["UNCLASSIFIED labeled_statement label"],
    );
}

#[test]
fn n4_field_rename_drops_target_role() {
    let js = Some(neg!("n4_field_rename_drops_target_role/javascript.json"));
    expect(
        &run(js, true, &embedded()),
        6,
        &[
            "CARRYING-SLOT-MISSING javascript assignment_pattern left",
            "UNCLASSIFIED assignment_pattern target",
            "SIGNATURE-REMOVED",
        ],
    );
    expect(&run(js, false, &embedded()), 2, &["CARRYING-SLOT-MISSING"]);
}

#[test]
fn n5_new_wrapper_inside_wrapper() {
    let js = Some(neg!("n5_new_wrapper_inside_wrapper/javascript.json"));
    expect(
        &run(js, true, &embedded()),
        7,
        &[
            "TARGET-KIND-UNCLASSIFIED javascript future_wrap via parenthesized_expression",
            "UNCLASSIFIED future_wrap",
            "UNCLASSIFIED-ORDER future_wrap",
        ],
    );
    expect(&run(js, false, &embedded()), 3, &[]);
}

fn with_tables(table: &str, order: &str, order_label: &str) -> Tables {
    Tables::load(
        ("table.tsv", table),
        ("leaves.tsv", tables::LEAVES_TSV),
        ("targets.tsv", tables::TARGETS_TSV),
        (order_label, order),
    )
}

#[test]
fn n6_table_role_loss_in_all_three_languages() {
    let t = with_tables(
        neg!("n6_table_role_loss.tsv"),
        tables::ORDER_TSV,
        "order.tsv",
    );
    let r = run(None, true, &t);
    expect(&r, 3, &[]);
    for lang in ["javascript", "typescript", "tsx"] {
        assert!(r.has(&format!(
            "TARGET-ROLE-LOSS {lang} parenthesized_expression <children> role R action WRAP"
        )));
    }
}

#[test]
fn n7_order_row_missing() {
    let t = with_tables(tables::TABLE_TSV, neg!("n7_order_row_missing.tsv"), "n7");
    expect(
        &run(None, true, &t),
        1,
        &["UNCLASSIFIED-ORDER for_in_statement"],
    );
}

#[test]
fn n8_order_class_invalid() {
    let t = with_tables(tables::TABLE_TSV, neg!("n8_order_bad_class.tsv"), "n8");
    expect(
        &run(None, true, &t),
        1,
        &["BAD-ORDER-CLASS for_in_statement later"],
    );
}

#[test]
fn n9_duplicate_row_with_valid_class() {
    let t = with_tables(
        tables::TABLE_TSV,
        neg!("n9_order_duplicate_valid.tsv"),
        "n9_order_duplicate_valid.tsv",
    );
    expect(
        &run(None, true, &t),
        1,
        &["DUPLICATE-ROW n9_order_duplicate_valid.tsv for_in_statement"],
    );
}

#[test]
fn n10_duplicate_row_with_invalid_class() {
    let t = with_tables(
        tables::TABLE_TSV,
        neg!("n10_order_duplicate_invalid.tsv"),
        "n10_order_duplicate_invalid.tsv",
    );
    // The first row wins and is valid, so the only problem is the duplicate itself.
    expect(
        &run(None, true, &t),
        1,
        &["DUPLICATE-ROW n10_order_duplicate_invalid.tsv for_in_statement"],
    );
}

/// Mutate one embedded table's text and expect exactly the named diagnostic.
fn mutated(which: &str, from: &str, to: &str) -> Tables {
    let sub = |name: &str, text: &str| {
        if name == which {
            assert!(
                text.contains(from),
                "mutation anchor {from:?} not in {name}"
            );
            text.replacen(from, to, 1)
        } else {
            text.to_string()
        }
    };
    Tables::load(
        ("table.tsv", &sub("table", tables::TABLE_TSV)),
        ("leaves.tsv", &sub("leaves", tables::LEAVES_TSV)),
        ("targets.tsv", &sub("targets", tables::TARGETS_TSV)),
        ("order.tsv", &sub("order", tables::ORDER_TSV)),
    )
}

#[test]
fn each_remaining_diagnostic_fires_on_its_own_mutation() {
    let cases: &[(&str, &str, &str, &str)] = &[
        (
            "table",
            "assignment_expression\tleft\tW\ttext",
            "assignment_expression\tleft\tQ\ttext",
            "BAD-ROLE assignment_expression left Q",
        ),
        (
            "table",
            "assignment_expression\tleft\tW\ttext",
            "assignment_expression\tleft\tW\tlater",
            "BAD-ORDER assignment_expression left later",
        ),
        (
            "table",
            "assignment_expression\tright\tR\tbefore-target\t",
            "bogus_kind\tbogus\tR\ttext\t",
            "UNCLASSIFIED assignment_expression right",
        ),
        (
            "table",
            "adding_type_annotation\t<children>",
            "malformed\trow\nadding_type_annotation\t<children>",
            "MALFORMED-ROW table.tsv malformed\trow",
        ),
        (
            "leaves",
            "comment\tNO\t-",
            "bogus_leaf\tNO\t-",
            "UNCLASSIFIED-LEAF comment",
        ),
        (
            "leaves",
            "accessibility_modifier\tNO",
            "malformed\trow\naccessibility_modifier\tNO",
            "MALFORMED-ROW leaves.tsv malformed\trow",
        ),
        (
            "leaves",
            "undefined\tBIND",
            "undefined\tNO",
            "BIND-SET-DRIFT",
        ),
        (
            "targets",
            "this\tNONE\t-",
            "bogus_target\tNONE\t-",
            "STALE-TARGET bogus_target",
        ),
        (
            "order",
            "identifier\tleaf",
            "identifier\tseq",
            "ORDER-LEAF-MISMATCH identifier seq",
        ),
        (
            "order",
            "for_in_statement\tloop",
            "bogus_order_kind\tseq",
            "STALE-ORDER bogus_order_kind",
        ),
    ];
    for (which, from, to, want) in cases {
        let r = run(None, true, &mutated(which, from, to));
        assert!(r.has(want), "{want}: {:#?}", r.problems);
        assert!(!r.problems.is_empty());
    }
    // A stale slot row (a row no grammar slot needs).
    let t = mutated(
        "table",
        "adding_type_annotation\t<children>",
        "bogus_kind\tbogus\tR\ttext\tx\nadding_type_annotation\t<children>",
    );
    assert!(run(None, true, &t).has("STALE bogus_kind bogus"));
    // A stale leaf row.
    let t = mutated(
        "leaves",
        "accessibility_modifier\tNO",
        "bogus_leaf\tNO\t-\tx\naccessibility_modifier\tNO",
    );
    assert!(run(None, true, &t).has("STALE-LEAF bogus_leaf"));
}
