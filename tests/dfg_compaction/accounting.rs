//! PR C measurement checkpoint; no allocator or real-corpus acceptance claim.
#[test]
fn fixture_and_generator_storage_accounting() {
    prism::build_pool::install(|| {
        let registered = super::cases::registered_names();
        let mut rows = Vec::new();
        super::cases::visit(|name, files| {
            // Discovery may see new main fixtures before the controller re-pins.
            // Their individual registration is enforced by the census in pins::.
            if !registered.contains(&name) {
                return;
            }
            let mut row = prism::data_flow::test_support::accounting::fixture(&files);
            row["case"] = name.into();
            rows.push(row);
        });
        assert_eq!(rows.len(), registered.len());
        for (name, mut row) in prism::data_flow::test_support::accounting::synthetic() {
            row["case"] = format!("synthetic-{name}").into();
            rows.push(row);
        }
        if let Ok(dir) = std::env::var("DFG_ACCOUNTING_DIR") {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            let text = rows
                .iter()
                .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
                .collect::<String>();
            std::fs::write(dir.join("fixture-accounting.jsonl"), text).unwrap();
        }
        eprintln!(
            "DFG_ACCOUNTING {} fixture/generator + 3 synthetic rows",
            rows.len() - 3
        );
    });
}

#[test]
fn fixture_addition_preserves_accounting_but_requires_registration() {
    let root = tempfile::tempdir().unwrap();
    for path in super::corpus::REGISTERED {
        let dest = root.path().join(path);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(path, dest).unwrap();
    }
    let run = |args: &[&str]| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .current_dir(root.path())
            .env_remove("DFG_ACCOUNTING_DIR")
            .args(args)
            .output()
            .unwrap();
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for summary in text.lines().filter(|line| line.starts_with("test result:")) {
            eprintln!("{args:?}: {summary}");
        }
        (output.status.success(), text)
    };
    let census = ["--exact", "pins::census_every_fixture_is_registered"];
    let (ok, text) = run(&census);
    assert!(ok && text.contains("1 passed"), "control census: {text}");

    std::fs::write(
        root.path()
            .join("tests/fixtures/unregistered-accounting.py"),
        "def identity(value):\n    return value\n",
    )
    .unwrap();
    let (ok, text) = run(&[
        "--exact",
        "accounting::fixture_and_generator_storage_accounting",
        "--skip",
        "pins::",
    ]);
    assert!(
        ok && text.contains("1 passed"),
        "added-fixture accounting: {text}"
    );
    let (ok, text) = run(&census);
    assert!(
        !ok && text.contains("register every fixture as a named case")
            && text.contains("unregistered-accounting.py"),
        "added-fixture census: {text}"
    );
}
