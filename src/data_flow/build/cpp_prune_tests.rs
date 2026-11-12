//! C++ and `.h` U/D obligations (#630 slice 6c): frozen-reference differential plus marked rows.
//! Rows: `tests/fixtures/dfg_prune/cpp/obligations.jsonl` (generated from the markers in the
//! fixtures and in `cpp_prune_inline_cases.rs` by `gen_obligations.py`).
use super::cpp_prune_inline_cases::INLINE_CASES;
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

/// A `.h` file is parsed as C (#529); every other C++ extension is C++.
fn language_of(file: &str) -> Language {
    if file.ends_with(".h") {
        Language::C
    } else {
        Language::Cpp
    }
}

fn collect(dir: &std::path::Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, exts, out);
        } else if path
            .extension()
            .is_some_and(|e| exts.iter().any(|x| e == *x))
        {
            out.push(path);
        }
    }
}

fn fixtures(subdir: &str, exts: &[&str]) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(subdir);
    let mut paths = Vec::new();
    collect(&root, exts, &mut paths);
    paths.sort();
    paths
}

fn pairs(refs: &super::FileRefs) -> std::collections::BTreeSet<(String, usize, usize)> {
    refs.edges
        .iter()
        .map(|e| (e.from.path.to_string(), e.from.line, e.to.line))
        .collect()
}

/// Owner-keyed view: a collapsed key lets an enclosing owner's copy of an edge hide a nested
/// owner's loss, so a keep row needs every owner that had the pair to still have it and a
/// drop row needs the selected owners to have had it and none to have it after.
type Owners = std::collections::BTreeSet<(String, usize)>;
fn owner_pairs(
    refs: &super::FileRefs,
) -> std::collections::BTreeMap<(String, usize, usize), Owners> {
    let mut out: std::collections::BTreeMap<_, Owners> = Default::default();
    for e in &refs.edges {
        out.entry((e.from.path.to_string(), e.from.line, e.to.line))
            .or_default()
            .insert((e.from.function.clone(), e.from.function_start_line));
    }
    out
}

fn check_subset(name: &str, source: &str, language: Language) -> (usize, usize) {
    let parsed = ParsedFile::parse(name, source, language).unwrap();
    let file = name.to_owned();
    let before = Producer::file_refs_frozen(&file, &parsed);
    let after = Producer::file_refs(&file, &parsed);
    let (b, a) = (pairs(&before), pairs(&after));
    assert!(a.is_subset(&b), "{name}: edges_after must be a subset");
    for (key, label) in &after.labels {
        assert_eq!(before.labels.get(key), Some(label), "{name}: {key:?}");
    }
    assert_eq!(
        before.edges.len() - after.edges.len(),
        after.rd_function_stats.dfg_pruned_write_only
            + after.rd_function_stats.dfg_pruned_dominated,
        "{name}: counters must count removed expanded edges"
    );
    (before.edges.len(), after.edges.len())
}

const SWEEP_EXTS: &[&str] = &[
    "cc", "cpp", "cxx", "hpp", "hh", "hxx", "h", "C", "H", "inl", "ipp", "tcc",
];

/// `edges_after ⊆ edges_before` over every C++ and header file of the fixture tree
/// (differential), and every inline case. The frozen producer is the control.
#[test]
fn cpp_prune_is_removal_only_over_fixture_tree() {
    // The tree holds a deeply nested table (`cpg_stack_regression`) that overflows the default
    // test thread while parsing, so the sweep runs on a large stack.
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(removal_only_sweep)
        .unwrap()
        .join()
        .unwrap();
}

fn removal_only_sweep() {
    let mut files = 0;
    for path in fixtures("tests/fixtures", SWEEP_EXTS) {
        if path.to_string_lossy().contains("secbench") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .to_string_lossy()
            .into_owned();
        check_subset(&rel, &source, language_of(&rel));
        files += 1;
    }
    assert!(
        files > 10,
        "the fixture sweep found {files} C++/header files"
    );
    for (name, source) in INLINE_CASES {
        check_subset(name, source, language_of(name));
    }
}

/// Every matrix input as `(case name, file name, source)`.
fn matrix_inputs() -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> =
        fixtures("tests/fixtures/dfg_prune/cpp", &["cc", "h", "cpp"])
            .into_iter()
            .map(|p| {
                let file = p.file_name().unwrap().to_str().unwrap().to_owned();
                (file.clone(), file, std::fs::read_to_string(&p).unwrap())
            })
            .collect();
    out.extend(
        INLINE_CASES
            .iter()
            .map(|(n, s)| (n.to_string(), n.to_string(), s.to_string())),
    );
    out.sort();
    out
}

fn rows() -> Vec<serde_json::Value> {
    include_str!("../../../tests/fixtures/dfg_prune/cpp/obligations.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn cpp_prune_matrix() {
    let obligations = rows();
    let mut failures = Vec::new();
    for (name, file, source) in matrix_inputs() {
        let language = language_of(&file);
        let parsed = ParsedFile::parse(&file, &source, language).unwrap();
        let before = Producer::file_refs_frozen(&file, &parsed);
        let after = Producer::file_refs(&file, &parsed);
        let (b, a) = (pairs(&before), pairs(&after));
        let (bo, ao) = (owner_pairs(&before), owner_pairs(&after));
        if std::env::var_os("PRISM_DUMP_PRUNE_BASELINE").is_some() {
            for (p, d, u) in &b {
                println!(
                    "BASELINE {name} {p} {d} {u} kept={}",
                    a.contains(&(p.clone(), *d, *u))
                );
            }
            for ((p, d, u), owners) in &bo {
                println!("OWNERS {name} {p} {d} {u} {owners:?}");
            }
        }
        check_subset(&file, &source, language);
        for row in obligations.iter().filter(|r| r["case"] == name.as_str()) {
            let key = (
                row["path"].as_str().unwrap().to_owned(),
                row["def_line"].as_u64().unwrap() as usize,
                row["use_line"].as_u64().unwrap() as usize,
            );
            if !b.contains(&key) {
                failures.push(format!("inadmissible missing baseline pair: {row}"));
            }
            let only = |set: Option<&Owners>| -> Owners {
                set.into_iter()
                    .flatten()
                    .filter(|(f, _)| row.get("owner").is_none_or(|o| o == f.as_str()))
                    .cloned()
                    .collect()
            };
            let (had, has) = (only(bo.get(&key)), only(ao.get(&key)));
            let (had, has) = (&had, &has);
            let ok = if row["keep"].as_bool().unwrap() {
                !had.is_empty() && had.is_subset(has)
            } else {
                !had.is_empty() && has.is_empty()
            };
            if !ok {
                failures.push(format!("{row} owners before={had:?} after={has:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every obligation row names a known case (a misspelt `case` would pass vacuously).
#[test]
fn cpp_prune_rows_name_known_cases() {
    let known: std::collections::BTreeSet<String> =
        matrix_inputs().into_iter().map(|(n, _, _)| n).collect();
    for row in rows() {
        assert!(
            known.contains(row["case"].as_str().unwrap()),
            "unknown case in {row}"
        );
    }
}

fn edge_count(name: &str, source: &str, language: Language) -> (usize, usize) {
    let parsed = ParsedFile::parse(name, source, language).unwrap();
    let file = name.to_owned();
    let before = Producer::file_refs_frozen(&file, &parsed);
    let after = Producer::file_refs(&file, &parsed);
    (before.edges.len(), after.edges.len())
}

/// A scalar owner whose use is not a bare argument: it prunes under both rule sets.
const SCALAR_OWNER: &str =
    "int src(void);\nint f(void) {\n    int x = src();\n    x = 1;\n    return x + 1;\n}\n";
/// A bare argument: an escape under the C++ rules only (`f(int&)` may be declared elsewhere).
const BARE_ARG: &str = "int src(void);\nvoid sink(int);\nint f(void) {\n    int x = src();\n    x = 1;\n    sink(x);\n    return 0;\n}\n";
/// A typedef-named type: passes the C rules, fails the C++ scalar gate.
const TYPEDEF_OWNER: &str =
    "int src(void);\nint f(void) {\n    handle_t x = src();\n    x = 1;\n    return x + 1;\n}\n";

/// Which rule set runs: `.c` the C rules, `.h` and C++ the C++ rules, any other C-grammar
/// name none.
#[test]
fn cpp_rules_apply_to_headers_and_cpp_only() {
    for (name, language, scalar, bare, typedef) in [
        ("f.c", Language::C, true, true, true),
        ("f.h", Language::C, true, false, false),
        ("f.H", Language::C, true, false, false),
        ("f.cc", Language::Cpp, true, false, false),
        ("f.hpp", Language::Cpp, true, false, false),
        ("f.cc", Language::C, false, false, false),
        ("f.cpp", Language::C, false, false, false),
    ] {
        for (label, src, expect) in [
            ("scalar", SCALAR_OWNER, scalar),
            ("bare argument", BARE_ARG, bare),
            ("typedef", TYPEDEF_OWNER, typedef),
        ] {
            let (b, a) = edge_count(name, src, language);
            assert_eq!(a < b, expect, "{name} {language:?} {label}: {b} -> {a}");
        }
    }
}

fn with_census(files: &[(&str, &str)], target: &str) -> (usize, usize) {
    use crate::call_graph::c::macro_census::CMacroCensus;
    use crate::call_graph::rust::macro_args::RustMacroCensus;
    let parsed: std::collections::BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(n, s)| {
            (
                n.to_string(),
                ParsedFile::parse(n, s, language_of(n)).unwrap(),
            )
        })
        .collect();
    let census = RustMacroCensus::default();
    let c_census = CMacroCensus::from_files(&parsed);
    let file = target.to_owned();
    let p = &parsed[target];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &census, &c_census);
    (before.edges.len(), after.edges.len())
}

/// The census is the whole repository's (C and C++ together): a header's macro refuses
/// names in a C++ owner, and an unreadable `#define` anywhere turns every owner off.
#[test]
fn cpp_prune_reads_the_repo_wide_census() {
    let owner = "int src();\nint f() {\n    int x = src();\n    x = 1;\n    return x + 1 + BUMP(x + 0);\n}\n";
    let (b, a) = with_census(&[("a.cc", owner)], "a.cc");
    assert!(a < b, "no macro: prunes ({b} -> {a})");
    let (b, a) = with_census(
        &[("a.cc", owner), ("h.h", "#define BUMP(v) ((v) + 1)\n")],
        "a.cc",
    );
    assert_eq!(a, b, "a name passed to an in-repo macro keeps its pairs");
    let (b, a) = with_census(&[("a.cc", owner), ("z.c", "#define 9bad 1\n")], "a.cc");
    assert_eq!(a, b, "incomplete census disables every owner");
    let hdr = "int src(void);\nint f(void) {\n    int x = src();\n    x = 1;\n    return x + 1 + BUMP(x + 0);\n}\n";
    let (b, a) = with_census(&[("a.h", hdr)], "a.h");
    assert!(a < b, "`.h`, no macro: prunes ({b} -> {a})");
    let (b, a) = with_census(
        &[("a.h", hdr), ("m.h", "#define BUMP(v) ((v) + 1)\n")],
        "a.h",
    );
    assert_eq!(a, b, "`.h`: macro argument keeps its pairs");
    let (b, a) = with_census(&[("a.h", hdr), ("z.c", "#define 9bad 1\n")], "a.h");
    assert_eq!(a, b, "`.h`: incomplete census disables the owner");
}

fn survivors(source: &str, name: &str) -> std::collections::BTreeMap<usize, Vec<(usize, usize)>> {
    let parsed = ParsedFile::parse(name, source, Language::Cpp).unwrap();
    let after = Producer::file_refs(&name.to_owned(), &parsed);
    let mut by_owner: std::collections::BTreeMap<usize, Vec<(usize, usize)>> = Default::default();
    for e in &after.edges {
        by_owner
            .entry(e.from.function_start_line)
            .or_default()
            .push((e.from.line, e.to.line));
    }
    by_owner.values_mut().for_each(|v| {
        v.sort();
        v.dedup();
    });
    by_owner
}

/// T08 (F6): the two passes of a templated function (the `template_declaration` and its inner
/// `function_definition`) give identical survivors, whether the template header shares a line
/// with the function or not.
#[test]
fn t08_template_two_passes_agree() {
    let body =
        "    int x = src();\n    x = 1;\n    int y = src();\n    take(y);\n    return x + y + 1;\n";
    let two =
        format!("int src();\nvoid take(int&);\ntemplate <typename T>\nint f(T t) {{\n{body}}}\n");
    let one =
        format!("int src();\nvoid take(int&);\ntemplate <typename T> int f(T t) {{\n{body}}}\n");
    let s2 = survivors(&two, "t.cc");
    // Owners start at the template line (3) and at the function line (4).
    assert_eq!(s2.get(&3), s2.get(&4), "two-line form: {s2:?}");
    assert!(s2[&3].contains(&(6, 9)), "y's pair is kept: {s2:?}");
    assert!(!s2[&3].contains(&(5, 9)), "x's pair is dropped: {s2:?}");
    let s1 = survivors(&one, "t.cc");
    assert_eq!(s1.len(), 1, "one-line form has one owner start: {s1:?}");
    assert!(s1[&3].contains(&(5, 8)), "y's pair is kept: {s1:?}");
    assert!(!s1[&3].contains(&(4, 8)), "x's pair is dropped: {s1:?}");
    // The one pair set is the same shape in both forms.
    let norm =
        |m: &std::collections::BTreeMap<usize, Vec<(usize, usize)>>, k: usize, off: usize| {
            m[&k]
                .iter()
                .map(|(a, b)| (a - off, b - off))
                .collect::<Vec<_>>()
        };
    assert_eq!(norm(&s2, 3, 3), norm(&s1, 3, 2));
}

/// Z05 (C++ cap row): an owner over the 2,048-definition RD limit stays capped under C++ and
/// `.h` when its names escape (the bare `sink(x)` argument keeps every pair).
#[test]
#[ignore = "over 2,048 definitions: minutes in a debug build; run with --release -- --ignored"]
fn cpp_capped_owner_stays_capped_when_nothing_prunes() {
    let mut src = String::from("void sink(int);\nint f(void) {\n    long x = 0;\n");
    for _ in 0..2050 {
        src.push_str("    x = 1;\n");
    }
    src.push_str("    sink(x);\n    return x;\n}\n");
    let (b, a) = edge_count("cap.cc", &src, Language::Cpp);
    assert_eq!(a, b, "C++: x escapes, the cap stands");
    let (b, a) = edge_count("cap.h", &src, Language::C);
    assert_eq!(a, b, "`.h`: x escapes, the cap stands");
}
