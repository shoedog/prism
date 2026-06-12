use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_version_includes_git_sha_and_dirty_flag() {
    // Identity grammar: see the "Build identity" comment in build.rs (single owner).
    // The eval harness (eval/tier_a/sut.py parse_version) pins the same grammar.
    Command::cargo_bin("prism")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(
                r"^slicing \d+\.\d+\.\d+ \(([0-9a-f]{12,40}(-dirty)?|unknown)\)\n$",
            )
            .unwrap(),
        )
        // in-repo builds are always git builds: "unknown" here would mean the
        // build-script git probe broke — fail loudly.
        .stdout(predicate::str::contains("(unknown)").not());
}
