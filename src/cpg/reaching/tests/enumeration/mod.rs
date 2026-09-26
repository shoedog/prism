fn cap_violation(path: &std::path::Path, source: &str) -> Option<String> {
    let n = source.lines().count();
    (n > 600).then(|| format!("{}: {n} lines", path.display()))
}

#[test]
fn census_cap_rejects_601_lines() {
    assert_eq!(
        cap_violation(std::path::Path::new("fixture.json"), &"x\n".repeat(601)),
        Some("fixture.json: 601 lines".to_string())
    );
}

#[test]
fn reaching_module_files_are_under_the_cap() {
    // dynamic: any new file under src/cpg/reaching/ is covered with no edit (W6)
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().is_some_and(|x| x == "rs" || x == "json") {
                out.push(p)
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let mut files = vec![root.join("reaching.rs")];
    walk(&root.join("reaching"), &mut files);
    assert!(
        files.len() >= 6,
        "expected the reaching module tree, found {}",
        files.len()
    );
    let violations: Vec<_> = files
        .iter()
        .filter_map(|f| cap_violation(f, &std::fs::read_to_string(f).unwrap()))
        .collect();
    assert!(
        violations.is_empty(),
        "over the 600-line cap:\n{}",
        violations.join("\n")
    );
}

mod barrier;
mod bash;
mod c;
mod case;
mod cpp;
mod go;
mod java;
mod javascript;
mod lua;
mod parameter_capture;
mod python;
mod rust;
mod terraform;
mod tsx;
mod typescript;
