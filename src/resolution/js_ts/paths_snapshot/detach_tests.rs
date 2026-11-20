//! #602 S4b1 detached-snapshot controls. Every snapshot is a REAL `capture` of a temporary
//! repository. Cells are populated through the real methods (`kind`, `first_pass_absent`,
//! `input_references_covered`); the absent/unsafe cells have no public writer, so they are
//! filled through the private `record_input` seam with SYNTHETIC `TypeInput` arguments.
use super::detach::MemoValues;
use super::{JsPathsSnapshot, TypeInput};
use std::collections::BTreeSet;
use std::sync::Arc;

fn captured() -> (tempfile::TempDir, JsPathsSnapshot) {
    let d = tempfile::TempDir::new().unwrap();
    for (name, text) in [
        ("tsconfig.json", r#"{"compilerOptions":{"paths":{}}}"#),
        ("a.ts", "export const a = 1;"),
    ] {
        std::fs::write(d.path().join(name), text).unwrap();
    }
    let s = JsPathsSnapshot::capture(d.path());
    assert!(
        s.complete && !s.configs.is_empty(),
        "inadmissible precondition: the capture is not a complete configured snapshot"
    );
    (d, s)
}

/// Adds `tag`-specific entries to all five cells.
fn populate(s: &JsPathsSnapshot, tag: &str) {
    s.kind(&format!("{tag}.ts"));
    s.first_pass_absent(&format!("{tag}-missing.ts"));
    s.record_input("", &format!("{tag}-absent"), &TypeInput::Absent);
    s.record_input("", &format!("{tag}-unsafe"), &TypeInput::Unsafe);
    s.input_references_covered(&format!("{tag}.d.ts"), None, &mut BTreeSet::new());
}

fn cells(v: &MemoValues) -> [(&'static str, String); 5] {
    [
        ("probes", format!("{:?}", v.probes)),
        ("first_pass_facts", format!("{:?}", v.first_pass_facts)),
        ("absent_inputs", format!("{:?}", v.absent_inputs)),
        ("unsafe_inputs", format!("{:?}", v.unsafe_inputs)),
        ("closure_memo", format!("{:?}", v.closure_memo)),
    ]
}

fn assert_cells_equal(a: &MemoValues, b: &MemoValues, what: &str) {
    for ((name, x), (_, y)) in cells(a).into_iter().zip(cells(b)) {
        assert_eq!(x, y, "cell {name}: {what}");
    }
}

fn assert_cells_differ(a: &MemoValues, b: &MemoValues, what: &str) {
    for ((name, x), (_, y)) in cells(a).into_iter().zip(cells(b)) {
        assert_ne!(x, y, "inadmissible precondition: cell {name}: {what}");
    }
}

fn shared(a: &JsPathsSnapshot, b: &JsPathsSnapshot) -> [(&'static str, bool); 5] {
    [
        ("probes", Arc::ptr_eq(&a.probes, &b.probes)),
        (
            "first_pass_facts",
            Arc::ptr_eq(&a.first_pass_facts, &b.first_pass_facts),
        ),
        (
            "absent_inputs",
            Arc::ptr_eq(&a.absent_inputs, &b.absent_inputs),
        ),
        (
            "unsafe_inputs",
            Arc::ptr_eq(&a.unsafe_inputs, &b.unsafe_inputs),
        ),
        (
            "closure_memo",
            Arc::ptr_eq(&a.closure_memo, &b.closure_memo),
        ),
    ]
}

#[test]
fn detached_preserves_every_memo_value() {
    let (_d, s) = captured();
    populate(&s, "seed");
    let seed = s.memo_values();
    for (name, empty) in [
        ("probes", seed.probes.is_empty()),
        ("first_pass_facts", seed.first_pass_facts.is_empty()),
        ("absent_inputs", seed.absent_inputs.is_empty()),
        ("unsafe_inputs", seed.unsafe_inputs.is_empty()),
        ("closure_memo", seed.closure_memo.is_empty()),
    ] {
        assert!(
            !empty,
            "inadmissible precondition: cell {name} is empty before the detach"
        );
    }
    let copy = s.detached();
    assert_cells_equal(&copy.memo_values(), &seed, "value lost by the detach");
}

#[test]
fn detached_cells_are_independent_both_ways() {
    let (_d, original) = captured();
    populate(&original, "seed");
    let copy = original.detached();
    let seed = original.memo_values();
    assert_eq!(
        copy.memo_values(),
        seed,
        "inadmissible precondition: the copy starts unequal"
    );

    populate(&original, "orig");
    let grown = original.memo_values();
    assert_cells_differ(&grown, &seed, "the write to the original changed nothing");
    assert_cells_equal(
        &copy.memo_values(),
        &seed,
        "a write to the original reached the copy",
    );

    let before_copy_write = copy.memo_values();
    populate(&copy, "copy");
    assert_cells_differ(
        &copy.memo_values(),
        &before_copy_write,
        "the write to the copy changed nothing",
    );
    assert_cells_equal(
        &original.memo_values(),
        &grown,
        "a write to the copy reached the original",
    );
    for (name, same) in shared(&original, &copy) {
        assert!(!same, "cell {name}: still the same Arc after the detach");
    }
}

#[test]
fn detached_keeps_plain_and_unserialized_fields() {
    let (_d, mut s) = captured();
    populate(&s, "seed");
    // SYNTHETIC: `type_metadata_bytes`, `excluded_paths` and `module_packages` are
    // `#[serde(skip)]` plain fields; set them directly so the round trip below has
    // something to lose.
    s.type_metadata_bytes = 7;
    s.excluded_paths.insert("skip-me".into());
    s.module_packages.insert("pkg".into(), Some(b"{}".to_vec()));
    let round_trip: JsPathsSnapshot =
        serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(
        round_trip.type_metadata_bytes == 0
            && round_trip.excluded_paths.is_empty()
            && round_trip.module_packages.is_empty(),
        "inadmissible precondition: a serde round trip keeps the skipped fields"
    );
    let copy = s.detached();
    assert_eq!(copy.topology(), s.topology());
    assert_eq!(copy.type_metadata_bytes, 7);
    assert!(copy.excluded_paths.contains("skip-me"));
    assert_eq!(copy.module_packages, s.module_packages);
    assert_eq!(copy.configs, s.configs);
    assert_eq!(copy.entries, s.entries);
    assert_eq!(copy.complete, s.complete);
}

/// Control that the hazard is real: `clone()` shares every cell.
#[test]
fn plain_clone_shares_cells() {
    let (_d, original) = captured();
    let seed = original.memo_values();
    let clone = original.clone();
    populate(&original, "orig");
    assert_cells_differ(&original.memo_values(), &seed, "the write changed nothing");
    assert_cells_equal(
        &clone.memo_values(),
        &original.memo_values(),
        "a clone must see the original's write",
    );
    for (name, same) in shared(&original, &clone) {
        assert!(same, "cell {name}: a clone must share the Arc");
    }
}
