//! CLI surfaces: settings belong to serving; operator lookup requires neither setting.
use super::*;
use clap::Parser;

#[test]
fn shared_idle_cli_surface() {
    let c = Cli::try_parse_from([
        "prism-mcp",
        "--repo",
        ".",
        "--shared",
        "--index-idle-secs",
        "0",
        "--index-idle-pressure-relief",
    ])
    .unwrap();
    assert!(c.shared);
    assert_eq!(c.index_idle_secs.as_deref(), Some("0"));
    assert!(c.index_idle_pressure_relief);
    let plain = Cli::try_parse_from(["prism-mcp", "--repo", ".", "--shared"]).unwrap();
    assert!(plain.index_idle_secs.is_none());
    assert!(!plain.index_idle_pressure_relief);
    assert!(StatusCli::try_parse_from(["daemon-status", "--repo", "."]).is_ok());
    assert!(ShutdownCli::try_parse_from(["daemon-shutdown", "--repo", "."]).is_ok());
    for mode in ["--eager", "--owner-compiler", "--no-cache", "--standalone"] {
        assert!(Cli::try_parse_from(["prism-mcp", "--repo", ".", "--shared", mode]).is_err());
    }
}

#[test]
fn shared_idle_relief_applicability() {
    use prism::mcp::idle::IdlePolicy;
    let on = IdlePolicy::enabled(std::time::Duration::from_secs(1));
    assert_eq!(
        on.pressure_relief_note(true, true).is_none(),
        cfg!(target_os = "macos")
    );
    assert!(IdlePolicy::default()
        .pressure_relief_note(true, true)
        .is_some());
    assert!(on.pressure_relief_note(false, true).is_none());
}
