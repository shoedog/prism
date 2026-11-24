use prism_review::api::{
    TaintContextEdges, TaintContextMode, TaintContextOptions, DEFAULT_TAINT_CONTEXT_DEPTH,
    DEFAULT_TAINT_CONTEXT_MAX_FILES,
};
use std::process::Command;
use tempfile::TempDir;

fn command(targets: bool) -> Command {
    let mut cmd = crate::prism_std();
    if targets {
        cmd.arg("targets");
    }
    cmd
}

fn help_entry<'a>(help: &'a str, flag: &str) -> &'a str {
    let mut entries = help.split("\n\n").filter(|entry| {
        entry
            .lines()
            .any(|line| line.split_whitespace().next() == Some(flag))
    });
    let entry = entries.next().unwrap_or_else(|| panic!("missing {flag}"));
    assert!(entries.next().is_none(), "duplicate entry for {flag}");
    entry
}

#[test]
fn help_lists_four_flags_with_shared_defaults() {
    let defaults = TaintContextOptions::default();
    assert_eq!(defaults.mode, TaintContextMode::Off);
    assert_eq!(defaults.edges, TaintContextEdges::NameOnly);
    assert_eq!(defaults.depth, DEFAULT_TAINT_CONTEXT_DEPTH);
    assert_eq!(defaults.max_files, DEFAULT_TAINT_CONTEXT_MAX_FILES);
    for targets in [false, true] {
        let output = command(targets).arg("--help").output().unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        for (flag, value, possible) in [
            ("--taint-context", "off".into(), Some("off, imports")),
            (
                "--taint-context-edges",
                "nameonly".into(),
                Some("exact, nameonly"),
            ),
            ("--taint-context-depth", defaults.depth.to_string(), None),
            (
                "--taint-context-max-files",
                defaults.max_files.to_string(),
                None,
            ),
        ] {
            let entry = help_entry(&help, flag);
            assert!(entry.contains(&format!("[default: {value}]")), "{entry}");
            if let Some(possible) = possible {
                assert!(
                    entry.contains(&format!("[possible values: {possible}]")),
                    "{entry}"
                );
            }
            assert!(entry.contains("8 MiB") && entry.contains("20 s"), "{entry}");
        }
    }
}

#[test]
fn malformed_and_out_of_range_are_rejected_before_loading() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing");
    assert!(
        !missing.exists(),
        "inadmissible precondition: repository exists"
    );
    for targets in [false, true] {
        for (flag, value) in [
            ("--taint-context", "on"),
            ("--taint-context-edges", "fuzzy"),
            ("--taint-context-depth", "0"),
            ("--taint-context-depth", "5"),
            ("--taint-context-depth", "x"),
            ("--taint-context-max-files", "-1"),
            ("--taint-context-max-files", "x"),
            ("--taint-context-max-files", "0"),
        ] {
            let output = command(targets)
                .arg("--repo")
                .arg(&missing)
                .arg("--diff")
                .arg(&missing)
                .arg(format!("{flag}={value}"))
                .output()
                .unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert_eq!(output.status.code(), Some(2), "{stderr}");
            assert!(output.stdout.is_empty());
            assert!(
                stderr.contains(&format!("invalid value '{value}'")),
                "{stderr}"
            );
            assert!(stderr.contains(flag), "{stderr}");
        }
    }
}

#[test]
fn accepted_boundaries() {
    let repo = TempDir::new().unwrap();
    std::fs::write(
        repo.path().join("main.js"),
        "export function f(x) { return x; }\n",
    )
    .unwrap();
    let diff = repo.path().join("diff.json");
    std::fs::write(
        &diff,
        r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[1]}]}"#,
    )
    .unwrap();
    for targets in [false, true] {
        let run = |flags: &[&str]| {
            command(targets)
                .arg("--repo")
                .arg(repo.path())
                .arg("--diff")
                .arg(&diff)
                .args(["--algorithm", "absence", "--format", "json", "--no-cache"])
                .args(flags)
                .output()
                .unwrap()
        };
        let baseline = run(&[]);
        assert!(
            baseline.status.success(),
            "inadmissible precondition: baseline failed: {}",
            String::from_utf8_lossy(&baseline.stderr)
        );
        for (depth, edges) in [("1", "exact"), ("4", "nameonly")] {
            let out = run(&[
                "--taint-context-depth",
                depth,
                "--taint-context-edges",
                edges,
                "--taint-context-max-files",
                "1",
            ]);
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(out.stdout, baseline.stdout);
            assert_eq!(out.stderr, baseline.stderr);
        }
    }
}

#[test]
fn flags_are_not_on_nav() {
    let nav_help = crate::prism_std().args(["nav", "--help"]).output().unwrap();
    assert!(
        nav_help.status.success(),
        "inadmissible precondition: nav help failed"
    );
    assert!(!String::from_utf8(nav_help.stdout)
        .unwrap()
        .contains("--taint-context"));
    for (flag, value) in [
        ("--taint-context", "imports"),
        ("--taint-context-depth", "1"),
        ("--taint-context-edges", "exact"),
        ("--taint-context-max-files", "1"),
    ] {
        let output = crate::prism_std()
            .args(["nav", "repo-map", "--repo", ".", flag, value])
            .output()
            .unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(2), "{stderr}");
        assert!(
            stderr.contains(&format!("unexpected argument '{flag}'")),
            "{stderr}"
        );
    }
}
