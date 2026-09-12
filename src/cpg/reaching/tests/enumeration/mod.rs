#[test] fn reaching_module_files_are_under_the_cap() {   // dynamic: any new file under src/cpg/reaching/ is covered with no edit (W6)
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) { for e in std::fs::read_dir(dir).unwrap() { let p = e.unwrap().path();
        if p.is_dir() { walk(&p, out) } else if p.extension().is_some_and(|x| x == "rs") { out.push(p) } } }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let mut files = vec![root.join("reaching.rs")]; walk(&root.join("reaching"), &mut files);
    assert!(files.len() >= 6, "expected the reaching module tree, found {}", files.len());
    for f in files { let n = std::fs::read_to_string(&f).unwrap().lines().count(); assert!(n <= 600, "{}: {n} lines", f.display()); }
}
