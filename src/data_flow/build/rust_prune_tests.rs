//! Rust U/D obligations (#630 slice 5, Rust): frozen-reference differential plus marked rows.
use super::rust_prune_inline_cases::INLINE_CASES;
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
        } else if path.extension().is_some_and(|e| e == "rs") {
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

/// Owner-keyed view: `(path, def_line, use_line)` to the owners (`function`, start line)
/// holding that pair. A collapsed key lets an enclosing owner's copy of an edge hide a
/// nested owner's loss, so a keep row needs every owner that had the pair to still have
/// it and a drop row needs the selected owners to have had it and none to have it after.
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
    let parsed = ParsedFile::parse(name, source, Language::Rust).unwrap();
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

/// `edges_after ⊆ edges_before` over every Rust file of the fixture tree (differential).
#[test]
fn rust_prune_is_removal_only_over_fixture_tree() {
    // The alias-twin policy removes twin edges the frozen reference has; it is off here so this
    // differential isolates the pruning rules (the policy has its own tests).
    super::alias_occurrence::without_policy(|| {
        for path in fixtures("tests/fixtures") {
            if path.to_string_lossy().contains("secbench") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            check_subset(path.file_name().unwrap().to_str().unwrap(), &source);
        }
        for (name, source) in INLINE_CASES {
            check_subset(name, source);
        }
    });
}

/// Every matrix input as `(file name, source)`: the on-disk fixtures plus the inline cases.
fn matrix_inputs() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = fixtures("tests/fixtures/dfg_prune/rust")
        .into_iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_str().unwrap().to_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.extend(
        INLINE_CASES
            .iter()
            .map(|(n, s)| (n.to_string(), s.to_string())),
    );
    out.sort();
    out
}

#[test]
fn rust_prune_matrix() {
    // The alias-twin policy removes twin edges the frozen reference has; it is off here so this
    // differential isolates the pruning rules (the policy has its own tests).
    super::alias_occurrence::without_policy(|| {
        let obligations: Vec<serde_json::Value> =
            include_str!("../../../tests/fixtures/dfg_prune/rust/obligations.jsonl")
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
        let mut failures = Vec::new();
        for (file, source) in matrix_inputs() {
            let name = file.strip_suffix(".rs").unwrap();
            let parsed = ParsedFile::parse(&file, &source, Language::Rust).unwrap();
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
            for row in obligations.iter().filter(|r| r["case"] == name) {
                let key = (
                    row["path"].as_str().unwrap().to_owned(),
                    row["def_line"].as_u64().unwrap() as usize,
                    row["use_line"].as_u64().unwrap() as usize,
                );
                if !b.contains(&key) {
                    failures.push(format!("inadmissible missing baseline pair: {row}"));
                }
                // An optional `owner` field narrows a row to one owner (function name).
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
                    // The filtered owner must have held the pair, or a misspelt `owner` passes.
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
    });
}
