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

/// The root package's `tests/` plus every workspace member's `crates/<name>/tests`
/// (each member declares its own umbrella targets in its Cargo.toml; #731).
fn umbrella_roots(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut roots = vec![manifest_dir.join("tests")];
    let mut members: Vec<PathBuf> = fs::read_dir(manifest_dir.join("crates"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("tests"))
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    roots.extend(members);
    roots
}

#[test]
fn umbrella_main_files_include_all_sibling_tests() {
    let dirs: Vec<PathBuf> = umbrella_roots(Path::new(env!("CARGO_MANIFEST_DIR")))
        .iter()
        .flat_map(|root| test_dirs(root))
        .collect();
    // 24 root umbrellas + tests/mcp of crates/prism-mcp + tests/cli of crates/prism-review.
    assert!(
        dirs.len() >= 26,
        "expected >=26 umbrella targets, found {} — recursive walk regressed?",
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
