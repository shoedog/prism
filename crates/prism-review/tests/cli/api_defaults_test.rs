//! Moved from `tests/integration/api_test.rs` (#701 S5): it runs the `prism` binary.

use prism_review::api::{
    AlgorithmParams, DEFAULT_BARRIER_DEPTH, DEFAULT_SPIRAL_MAX_RING, DEFAULT_TEMPORAL_DAYS,
};

#[test]
fn defaults_are_shared_with_clap() {
    let params = AlgorithmParams::default();
    assert_eq!(params.barrier_depth, DEFAULT_BARRIER_DEPTH);
    assert_eq!(params.spiral_max_ring, DEFAULT_SPIRAL_MAX_RING);
    assert_eq!(params.temporal_days, DEFAULT_TEMPORAL_DAYS);

    let output = crate::prism_bin().unwrap().arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    let start = help.find("--barrier-depth").unwrap();
    let end = help[start..]
        .find("--barrier-symbols")
        .map(|offset| start + offset)
        .unwrap_or(help.len());
    assert!(
        help[start..end].contains(&format!("[default: {}]", DEFAULT_BARRIER_DEPTH)),
        "barrier-depth help block did not contain the shared default: {}",
        &help[start..end]
    );
}
