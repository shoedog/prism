use std::fs;
use std::path::{Path, PathBuf};

fn test_dirs(root: &Path) -> Vec<PathBuf> {
    // recursive: umbrella main.rs files live both at tests/<dir>/ and nested
    // (tests/lang/<lang>/, tests/algo/<group>/) — a single-level walk silently
    // skipped 15 of 24 umbrellas (final-review fix-round REJECT finding)
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if path.join("main.rs").is_file() {
                    out.push(path);
                } else {
                    stack.push(path);
                }
            }
        }
    }
    out
}

fn declares_module(main: &str, stem: &str) -> bool {
    let needle = format!("mod {stem};");
    main.lines()
        .map(str::trim)
        .map(|line| line.split("//").next().unwrap_or("").trim())
        .any(|line| line == needle)
}

#[test]
fn umbrella_main_files_include_all_sibling_tests() {
    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let dirs = test_dirs(&tests_root);
    assert!(
        dirs.len() >= 24,
        "expected >=24 umbrella targets, found {} — recursive walk regressed?",
        dirs.len()
    );
    for dir in dirs {
        let main_rs = dir.join("main.rs");
        let main = fs::read_to_string(&main_rs).unwrap();
        let missing: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
            .filter(|path| path.file_name().and_then(|n| n.to_str()) != Some("main.rs"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(String::from)
            })
            .filter(|stem| !declares_module(&main, stem))
            .collect();

        assert!(
            missing.is_empty(),
            "{} missing mod declarations for {:?}",
            main_rs.display(),
            missing,
        );
    }
}
