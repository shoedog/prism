use prism::navigation::onboarding::{build_report, MAX_CONNECTED_MODULES};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(path: &std::path::Path) -> NavigationSession {
    let repo = Arc::new(load_repo(path).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn overview_combines_inventory_module_and_call_facts_deterministically() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.py"),
        "import util\n\ndef run():\n    return util.helper()\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    std::fs::write(dir.path().join("extra.rs"), "pub fn extra() {}\n").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not indexed\n").unwrap();

    let session = session(dir.path());
    let first = build_report(&session).unwrap();
    let second = build_report(&session).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.schema_version, "1.0");
    assert_eq!(first.inventory.indexed_files, 3);
    assert_eq!(first.inventory.skipped_files, 1);
    assert_eq!(first.inventory.functions, 3);
    assert_eq!(first.inventory.languages.get("python"), Some(&2));
    assert_eq!(first.inventory.languages.get("rust"), Some(&1));
    assert_eq!(first.modules.nodes, 3);
    assert_eq!(first.modules.edges, 1);
    assert_eq!(first.modules.isolated_files, 1);
    assert_eq!(first.modules.connected[0].file, "main.py");
    assert_eq!(first.modules.connected[0].dependencies, 1);
    assert_eq!(first.calls.total_sites, 1);
    assert_eq!(first.calls.exact_edges, 1);
    assert_eq!(first.calls.name_only_edges, 0);
    assert!(first
        .warnings
        .iter()
        .any(|warning| warning.contains("1 source file(s) were skipped")));
}

#[test]
fn overview_empty_repo_reports_zero_without_fabricated_connected_modules() {
    let dir = tempfile::tempdir().unwrap();
    let report = build_report(&session(dir.path())).unwrap();

    assert_eq!(report.inventory.indexed_files, 0);
    assert_eq!(report.inventory.skipped_files, 0);
    assert_eq!(report.inventory.functions, 0);
    assert!(report.inventory.languages.is_empty());
    assert_eq!(report.modules.nodes, 0);
    assert_eq!(report.modules.edges, 0);
    assert_eq!(report.modules.isolated_files, 0);
    assert!(report.modules.connected.is_empty());
    assert_eq!(report.calls.total_sites, 0);
}

#[test]
fn overview_connected_module_ranking_is_bounded_and_stably_tied() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hub.py"), "def hit():\n    return 1\n").unwrap();
    for i in 0..13 {
        std::fs::write(
            dir.path().join(format!("leaf{i}.py")),
            format!("import hub\n\ndef leaf{i}():\n    return hub.hit()\n"),
        )
        .unwrap();
    }

    let report = build_report(&session(dir.path())).unwrap();
    assert_eq!(report.modules.edges, 13);
    assert_eq!(report.modules.connected.len(), MAX_CONNECTED_MODULES);
    assert_eq!(report.modules.connected[0].file, "hub.py");
    assert_eq!(report.modules.connected[0].dependents, 13);
    let tied: Vec<_> = report.modules.connected[1..]
        .iter()
        .map(|module| module.file.as_str())
        .collect();
    let mut sorted = tied.clone();
    sorted.sort_unstable();
    assert_eq!(tied, sorted);
}
