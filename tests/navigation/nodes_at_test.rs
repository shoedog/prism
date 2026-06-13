use prism::navigation::types::{Source, SymbolRef, WarningKind};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    session: NavigationSession,
}

fn fixture(files: &[(&str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    for (file, src) in files {
        std::fs::write(dir.path().join(file), src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    Fixture {
        _dir: dir,
        session: NavigationSession { repo, index },
    }
}

fn session(src: &str, file: &str) -> Fixture {
    fixture(&[(file, src)])
}

#[test]
fn nodes_at_includes_enclosing_function() {
    let s = session("def f():\n    y = 1\n    return y\n", "a.py");
    let ev = queries::nodes_at(&s.session, "a.py", 2);
    // enclosing function `f` is reported as evidence
    assert!(ev
        .items
        .iter()
        .any(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f")));
    assert!(ev.items.iter().all(|i| i.source == Source::PrismCpg));
}

#[test]
fn nodes_at_enclosing_function_location_carries_function_bytes() {
    let s = session("def f():\n    y = 1\n    return y\n", "a.py");
    let ev = queries::nodes_at(&s.session, "a.py", 2);
    let item = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "f"))
        .expect("enclosing function evidence");

    match item.symbol.as_ref().unwrap() {
        SymbolRef::Function {
            start_byte,
            end_byte,
            ordinal,
            ..
        } => {
            assert!(*end_byte >= *start_byte);
            assert_eq!(*ordinal, 0, "reserved");
            assert_eq!(
                (*start_byte, *end_byte),
                (item.location.start_byte, item.location.end_byte)
            );
        }
        _ => panic!("function symbol"),
    }
}

#[test]
fn nodes_at_skipped_path_warns_empty() {
    let s = session("def f(): pass\n", "a.py");
    let ev = queries::nodes_at(&s.session, "missing.py", 1);
    assert!(ev.items.is_empty());
    assert!(ev
        .warnings
        .iter()
        .any(|w| matches!(&w.kind, WarningKind::SkippedPath)));
}

#[test]
fn nodes_at_skipped_file_reports_skip_reason() {
    let s = fixture(&[("a.py", "def f(): pass\n"), ("notes.md", "# hi\n")]);
    let ev = queries::nodes_at(&s.session, "notes.md", 1);
    assert!(ev.items.is_empty());
    assert!(ev.warnings.iter().any(|w| {
        matches!(&w.kind, WarningKind::SkippedPath)
            && w.message.contains("file excluded: Unsupported")
    }));
}
