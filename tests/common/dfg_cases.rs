use super::*;

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        let m = std::fs::symlink_metadata(&p).unwrap();
        assert!(!m.file_type().is_symlink());
        if m.is_dir() {
            walk(&p, out)
        } else if Language::from_path(p.to_str().unwrap()).is_some() {
            out.push(p)
        }
    }
}
pub fn visit(mut save: impl FnMut(String, BTreeMap<String, ParsedFile>)) {
    let mut paths = Vec::new();
    walk(Path::new("tests/fixtures"), &mut paths);
    let mut all = BTreeMap::new();
    for p in paths {
        let path = p.to_str().unwrap();
        let src = std::fs::read_to_string(&p).unwrap();
        let lang = Language::from_path(path).unwrap();
        all.insert(path.into(), ParsedFile::parse(path, &src, lang).unwrap());
        save(
            format!("fixture-{}", path.replace('/', "__")),
            BTreeMap::from([(path.into(), ParsedFile::parse(path, &src, lang).unwrap())]),
        );
    }
    save("fixture-tree".into(), all);
    for (name, (files, _sources, _diff)) in [
        ("python", make_python_test()),
        ("javascript", make_javascript_test()),
        ("go", make_go_test()),
        ("java", make_java_test()),
        ("typescript", make_typescript_test()),
        ("c", make_c_test()),
        ("c-multifile", make_c_multifile_test()),
        ("cpp", make_cpp_test()),
        ("tsx", make_tsx_test()),
        ("terraform", make_terraform_test()),
        ("bash", make_bash_test()),
    ] {
        save(format!("generator-{name}"), files);
    }
    for (name, (files, _diff)) in [
        ("snmp", make_snmp_overflow_test()),
        ("double-free", make_double_free_test()),
        ("ring", make_ring_overflow_test()),
        ("timer", make_timer_uaf_test()),
        ("large", make_large_function_test()),
        ("deep-switch", make_deep_switch_test()),
    ] {
        save(format!("generator-{name}"), files);
    }
    for (name, path, src, lang) in [
        (
            "lua",
            "compact.lua",
            "local function f(x)\n local y = x\n return y\nend\n",
            Language::Lua,
        ),
        (
            "rust",
            "compact.rs",
            "fn f(x: i32) -> i32 { let y = x; y }",
            Language::Rust,
        ),
        (
            "helper-python",
            "test.py",
            "def f(x):\n    return x\n",
            Language::Python,
        ),
        (
            "helper-rust",
            "test.rs",
            "fn f(x: i32) -> i32 { x }",
            Language::Rust,
        ),
    ] {
        save(
            format!("generator-{name}"),
            BTreeMap::from([(path.into(), ParsedFile::parse(path, src, lang).unwrap())]),
        );
    }
}

pub fn explicit(path: &str, src: &str, lang: Language) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(path.into(), ParsedFile::parse(path, src, lang).unwrap())])
}
