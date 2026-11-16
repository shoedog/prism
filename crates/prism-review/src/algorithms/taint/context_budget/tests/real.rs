//! REAL tests use the captured loader and its actual read outcomes.
use super::*;
use prism::api::{load_review_inputs, ReviewContextLoader, ReviewOptions};

fn fixture(files: &[(&str, &str)]) -> (tempfile::TempDir, prism::api::ReviewInputs) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.js"), "export const main = 1;\n").unwrap();
    for (name, source) in files {
        std::fs::write(dir.path().join(name), source).unwrap();
    }
    let diff = r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[]}]}"#;
    let inputs = load_review_inputs(&ReviewOptions::new(dir.path()), diff).unwrap();
    (dir, inputs)
}
fn source(n: usize) -> String {
    format!("//{}\n", "x".repeat(n - 3))
}

#[test]
fn real_capacity_prefix() {
    let data = source(40 * 1024);
    let (_dir, inputs) = fixture(&[("a.js", &data), ("b.js", &data), ("c.js", &data)]);
    let loader =
        ReviewContextLoader::new(&inputs).expect("inadmissible precondition: captured loader");
    for p in ["a.js", "b.js"] {
        assert!(
            matches!(
                loader.read(p, 100 * 1024).outcome,
                prism::api::ContextReadOutcome::Loaded(_)
            ),
            "inadmissible precondition: listed and clean {p}"
        );
    }
    let mut c = AttemptCounters::default();
    let s = admit_prefix(
        &targets(&["a.js", "b.js", "c.js"]),
        &limits(5, 64 * 1024),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, r| report_from_attempt(loader.read(p, r)),
    );
    assert_eq!(s.accepted.len(), 1);
    assert_eq!(
        s.bound,
        Some(AdmissionBound::SourceBytes {
            first_skipped: 1,
            skipped: 2
        })
    );
    assert_eq!(c.attempted_read_bytes, 64 * 1024 + 1);
}
#[test]
fn real_oversize_then_small() {
    let big = source(2 * 1024 * 1024 + 1);
    let small = "export const small = 1;\n";
    let (dir, inputs) = fixture(&[("big.js", small), ("small.js", small)]);
    std::fs::write(dir.path().join("big.js"), &big).unwrap();
    let loader =
        ReviewContextLoader::new(&inputs).expect("inadmissible precondition: captured loader");
    assert!(
        matches!(
            loader.read("big.js", 8 * 1024 * 1024).outcome,
            prism::api::ContextReadOutcome::Skipped(prism::api::ContextSkip::TooLarge { .. })
        ),
        "inadmissible precondition: listed oversized file"
    );
    assert!(
        matches!(
            loader.read("small.js", 100).outcome,
            prism::api::ContextReadOutcome::Loaded(_)
        ),
        "inadmissible precondition: listed clean small file"
    );
    let mut c = AttemptCounters::default();
    let s = admit_prefix(
        &targets(&["big.js", "small.js"]),
        &limits(4, 8 * 1024 * 1024),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, r| report_from_attempt(loader.read(p, r)),
    );
    assert_eq!(s.accepted.len(), 1);
    assert_eq!(s.accepted[0].0, "small.js");
    assert_eq!(
        c.attempted_read_bytes,
        2 * 1024 * 1024 + 1 + small.len() as u64
    );
    assert!(matches!(c.skips[0].1, SkipReason::TooLarge { .. }));
}
#[test]
fn real_refusals_cost_no_bytes() {
    let (_dir, inputs) = fixture(&[]);
    let loader =
        ReviewContextLoader::new(&inputs).expect("inadmissible precondition: captured loader");
    assert!(
        matches!(
            loader.read("main.js", 10).outcome,
            prism::api::ContextReadOutcome::Skipped(prism::api::ContextSkip::OriginalFile)
        ),
        "inadmissible precondition: original"
    );
    assert!(
        matches!(
            loader.read("absent.js", 10).outcome,
            prism::api::ContextReadOutcome::Skipped(prism::api::ContextSkip::NotListed)
        ),
        "inadmissible precondition: unlisted"
    );
    let mut c = AttemptCounters::default();
    let s = admit_prefix(
        &targets(&["main.js", "absent.js"]),
        &limits(4, 10),
        Published::default(),
        &mut c,
        &guard(),
        &mut |p, r| report_from_attempt(loader.read(p, r)),
    );
    assert_eq!(c.attempted_files, 2);
    assert_eq!(c.attempted_read_bytes, 0);
    assert_eq!(c.skips.len(), 2);
    assert!(s.accepted.is_empty());
}
