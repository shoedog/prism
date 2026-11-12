mod algo_test;
mod alias_kill_trace_test;
mod api_defaults_test;
mod build_profile_fold_test;
mod build_profile_round2_test;
mod build_profile_test;
mod call_stats_jsx_intrinsic_test;
mod call_stats_local_export_test;
mod call_stats_test;
mod call_stats_wrapped_export_test;
mod confidence_test;
mod diagram_e2e_test;
mod edges_bytes_test;
mod location_seed_test;
mod member_only_param_test;
mod min_confidence_admission_test;
mod min_confidence_test;
mod nav_compat_test;
mod nav_sidecar_cache_parity_test;
mod output_test;
mod owner_activation_test;
mod readme_test;
mod review_compact_test;
mod review_no_diagrams_test;
mod sarif_shape_test;
mod sarif_test;
mod skills_test;
mod taint_reaches_test;
mod taint_sink_strength_view_test;
mod targets_test;
mod validation_test;
mod version_test;

/// The `prism` binary with the repository root as its working directory: cargo runs this package's
/// tests from `crates/prism-review`, and these tests pass repository-relative paths (#701 S5).
pub fn prism_bin() -> Result<assert_cmd::Command, assert_cmd::cargo::CargoError> {
    assert_cmd::Command::cargo_bin("prism").map(|mut command| {
        command.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        command
    })
}

/// `std::process::Command` form of [`prism_bin`].
pub fn prism_std() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_prism"));
    command.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    command
}
