// Admission binds named-tree bytes and typed receipts to the selected slots.
// Admission preserves independently supported claims and counts scoped refusals.
// Bindings preserve snapshot identity, raw observations, and extraction coverage.
// Impact retains both revision-local sides and each comparison's own occurrences.
// Impact stores each standing limit once and links supported affected claims.
// Saved status compares the same admitted element on the selected service side.
// Membership negatives require complete evidence across all applicable inputs.
// Both 0.1 schemas validate their own DTOs and enforce closed records.
//
// P4 or deferred companions from the binding design answers:
// A24 A26 A30 A37 A39 A40 A42 A43 A45 A46 A47
// B14 B16 B18 B25 B26 B34 B40 B43 B44 B45
// C19 C20 C23
// D15 (full ordering and replay perturbation)

#[path = "support/doc_admission.rs"]
mod doc_admission;
#[path = "support/doc_cases.rs"]
mod doc_cases;
#[path = "support/doc_saved.rs"]
mod doc_saved;
#[path = "support/doc_schema.rs"]
mod doc_schema;
#[path = "support/doc_wire.rs"]
mod doc_wire;
#[allow(dead_code)]
#[path = "support/matrix.rs"]
mod matrix;

use doc_cases::run;

#[test]
fn admission_receipt_and_slot_rows() {
    run(&["admission"], doc_admission::check);
}

#[test]
fn admission_document_decode_and_reference_rows() {
    run(&["decode", "semantic"], doc_wire::check);
}

#[test]
fn documents_bindings_and_impact_rows() {
    run(&["bindings", "impact", "round_trip"], doc_wire::check);
}

#[test]
fn documents_saved_scan_rows() {
    run(&["saved"], doc_saved::check);
}

#[test]
fn documents_membership_rows() {
    run(&["membership"], doc_saved::check);
}

#[test]
fn documents_schema_rows() {
    run(&["schema", "hashes"], doc_schema::check);
}

#[test]
fn cli_help_and_version() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_prism-boundary");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success(), "{:?}", help.stderr);
    let stdout = String::from_utf8(help.stdout).unwrap();
    assert!(stdout.contains("Usage: prism-boundary"));
    assert!(stdout.contains("--help") && stdout.contains("--version"));

    let version = Command::new(binary).arg("--version").output().unwrap();
    assert!(version.status.success(), "{:?}", version.stderr);
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim_end(),
        concat!("prism-boundary ", env!("CARGO_PKG_VERSION"))
    );
}
