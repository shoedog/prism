//! C U/D obligations (#630 slice 6b): frozen-reference differential plus marked rows.
//! Rows: `tests/fixtures/dfg_prune/c/obligations.jsonl` (generated from the markers in the
//! fixtures and in `c_prune_inline_cases.rs` by `gen_obligations.py`).
use super::c_prune_inline_cases::INLINE_CASES;
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "c") {
            out.push(path);
        }
    }
}

fn fixtures(subdir: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(subdir);
    let mut paths = Vec::new();
    collect(&root, &mut paths);
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

fn check_subset(name: &str, source: &str) -> (usize, usize) {
    let parsed = ParsedFile::parse(name, source, Language::C).unwrap();
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

/// `edges_after ⊆ edges_before` over every C file of the fixture tree (differential), and
/// every inline case. Z01: the frozen producer is the control for C (no C pruning in it).
#[test]
fn c_prune_is_removal_only_over_fixture_tree() {
    let mut files = 0;
    for path in fixtures("tests/fixtures") {
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
        check_subset(&rel, &source);
        files += 1;
    }
    assert!(files > 20, "the fixture sweep found {files} .c files");
    for (name, source) in INLINE_CASES {
        check_subset(name, source);
    }
}

/// Every matrix input as `(case name, file name, source)`.
fn matrix_inputs() -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = fixtures("tests/fixtures/dfg_prune/c")
        .into_iter()
        .map(|p| {
            let file = p.file_name().unwrap().to_str().unwrap().to_owned();
            (
                file.strip_suffix(".c").unwrap().to_owned(),
                file,
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.extend(INLINE_CASES.iter().map(|(n, s)| {
        (
            n.strip_suffix(".c").unwrap().to_owned(),
            n.to_string(),
            s.to_string(),
        )
    }));
    out.sort();
    out
}

fn rows() -> Vec<serde_json::Value> {
    include_str!("../../../tests/fixtures/dfg_prune/c/obligations.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn c_prune_matrix() {
    let obligations = rows();
    let mut failures = Vec::new();
    let mut seen_cases = std::collections::BTreeSet::new();
    for (name, file, source) in matrix_inputs() {
        let parsed = ParsedFile::parse(&file, &source, Language::C).unwrap();
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
        }
        check_subset(&file, &source);
        for row in obligations.iter().filter(|r| r["case"] == name.as_str()) {
            seen_cases.insert(name.clone());
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
fn c_prune_rows_name_known_cases() {
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

const PLAIN_OWNER: &str = "int src(void);\nvoid sink(int);\nvoid f(void) {\n    int x = src();\n    x = 1;\n    sink(x);\n}\n";

/// 6b rules for `.c`. Under 6c a `.h` and C++ run the C++ rules, where the bare `sink(x)`
/// argument is an escape, so this owner still keeps its pairs there; `.cc` parsed as C stays off.
#[test]
fn c_prune_only_for_dot_c_files() {
    let (b, a) = edge_count("f.c", PLAIN_OWNER, Language::C);
    assert!(a < b, "the control must prune: {b} -> {a}");
    for name in ["f.h", "f.cc", "f.C", "f.cpp"] {
        let (b, a) = edge_count(name, PLAIN_OWNER, Language::C);
        assert_eq!(b, a, "{name}: off, or kept by the bare-argument escape");
    }
    let (b, a) = edge_count("f.c", PLAIN_OWNER, Language::Cpp);
    assert_eq!(b, a, "C++ rules: the bare argument keeps the pairs");
}

fn with_census(files: &[(&str, &str)], target: &str) -> (usize, usize) {
    use crate::call_graph::c::macro_census::CMacroCensus;
    use crate::call_graph::rust::macro_args::RustMacroCensus;
    let parsed: std::collections::BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(n, s)| (n.to_string(), ParsedFile::parse(n, s, Language::C).unwrap()))
        .collect();
    let census = RustMacroCensus::default();
    let c_census = CMacroCensus::from_files(&parsed);
    let file = target.to_owned();
    let p = &parsed[target];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &census, &c_census);
    (before.edges.len(), after.edges.len())
}

/// macro_other_file / macro_redefined_union / repo-wide `incomplete`: the census is the whole
/// repository's, so a header's macro refuses names in another file's owner.
#[test]
fn c_prune_reads_the_repo_wide_census() {
    let owner = "int src(void);\nvoid sink(int);\nvoid f(void) {\n    int x = src();\n    x = 1;\n    sink(x);\n    sink(BUMP(x));\n}\n";
    let (b, a) = with_census(&[("a.c", owner)], "a.c");
    assert!(a < b, "no macro: prunes ({b} -> {a})");
    // The macro is defined in another file; the owner passes `x` to it.
    let (b, a) = with_census(
        &[("a.c", owner), ("h.c", "#define BUMP(v) ((v) + 1)\n")],
        "a.c",
    );
    assert_eq!(
        a, b,
        "a name passed to an in-repo macro defined elsewhere keeps its pairs"
    );
    // Two definitions are unioned: one of them names `x` freely.
    let owner2 = "int src(void);\nvoid sink(int);\nvoid f(void) {\n    int x = src();\n    x = 1;\n    sink(x);\n    sink(PEEK);\n}\n";
    let defs = "#define PEEK 0\n#define PEEK (x + 1)\n";
    let (b, a) = with_census(&[("a.c", owner2), ("h.c", defs)], "a.c");
    assert_eq!(a, b, "redefined macros are unioned");
    let (b, a) = with_census(&[("a.c", owner2), ("h.c", "#define PEEK 0\n")], "a.c");
    assert!(
        a < b,
        "a macro with no free local name leaves the owner on ({b} -> {a})"
    );
    // An unreadable `#define` in another file turns every C owner off.
    let (b, a) = with_census(&[("a.c", owner), ("z.c", "#define 9bad 1\n")], "a.c");
    assert_eq!(a, b, "incomplete census disables every owner");
}

/// Z05 (C-specific cap row): an owner over the 2,048-definition RD limit is capped on the
/// frozen producer (no edges); with C pruning the survivor-based cap (owner rule #634)
/// lets it regain its surviving edges, so `edges_after` is NOT a subset here. Pinned so a
/// change to the cap is a decision, not an accident. C++ and `.h` stay capped.
#[test]
fn c_prune_formerly_capped_owner_regains_survivors_per_634() {
    let mut src = String::from("void sink(int);\nint f(void) {\n    long x = 0;\n");
    for _ in 0..2050 {
        src.push_str("    x = 1;\n");
    }
    src.push_str("    sink(x);\n    return x;\n}\n");
    let (b, a) = edge_count("cap.c", &src, Language::C);
    assert!(
        a >= b,
        "C pruning can only regain survivors here: {b} -> {a}"
    );
    assert!(
        a > b,
        "the owner was capped on the frozen producer: {b} -> {a}"
    );
    let (b, a) = edge_count("cap.cc", &src, Language::Cpp);
    assert_eq!(a, b, "C++ keeps the cap (the bare argument escapes)");
    let (b, a) = edge_count("cap.h", &src, Language::C);
    assert_eq!(a, b, "a header keeps the cap");
}
