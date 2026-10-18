use super::*;

fn fixture() -> (tempfile::TempDir, Edit) {
    let dir = tempfile::tempdir().unwrap();
    let before = "def run():\n    return 1\n".to_string();
    let after = "def run():\n    return 2\n".to_string();
    std::fs::write(dir.path().join("main.py"), &before).unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "main.py"]);
    git(&["commit", "-qm", "fixture"]);
    let edit = Edit {
        repo_root: dir.path().canonicalize().unwrap().display().to_string(),
        commit: git(&["rev-parse", "HEAD"]),
        path: "main.py".into(),
        before_sha256: hash(&before),
        after_sha256: hash(&after),
        before,
        after,
    };
    (dir, edit)
}

#[test]
fn invalid_edits_and_identities_rejected() {
    let (dir, edit) = fixture();
    let mut accepted = Vec::new();
    for case in 0..7 {
        let mut bad = edit.clone();
        match case {
            0 => bad.after.clear(),
            1 => {
                bad.after = bad.before.clone();
                bad.after_sha256 = bad.before_sha256.clone();
            }
            2 => {
                bad.before = "wrong starting content".into();
                bad.before_sha256 = hash(&bad.before);
            }
            3 => bad.after_sha256 = "0".repeat(64),
            4 => bad.commit = "0".repeat(40),
            5 => bad.repo_root = "/unavailable".into(),
            _ => bad.path = "../main.py".into(),
        }
        if bad.check(dir.path()).is_ok() {
            accepted.push(case);
        }
    }
    std::fs::write(dir.path().join("untracked.py"), "dirty").unwrap();
    if edit.check(dir.path()).is_ok() {
        accepted.push(7);
    }
    assert!(accepted.is_empty(), "accepted cases {accepted:?}");
}

#[test]
fn failed_refresh_still_restores() {
    let (dir, edit) = fixture();
    let result = edit.run::<()>(dir.path(), || {
        assert_eq!(
            std::fs::read_to_string(dir.path().join("main.py"))?,
            edit.after
        );
        anyhow::bail!("injected refresh failure")
    });
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("injected refresh failure"));
    assert_eq!(
        hash(std::fs::read(dir.path().join("main.py")).unwrap()),
        edit.before_sha256
    );
}

#[test]
fn failed_restore_is_hard_failure() {
    let (dir, edit) = fixture();
    let result = edit.run(dir.path(), || {
        std::fs::remove_file(dir.path().join("main.py"))?;
        std::fs::create_dir(dir.path().join("main.py"))?;
        Ok(())
    });
    assert!(result.unwrap_err().to_string().contains("restore failed"));
}
