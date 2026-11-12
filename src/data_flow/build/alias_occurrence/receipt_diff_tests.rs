//! Attributed receipt diff of the alias-twin policy over every Go, Rust and Python fixture of
//! `tests/fixtures`: policy off (main's behaviour) against policy on, per file. Each changed
//! expectation is named by occurrence and attributed: a removed Def is an alias twin (another Def
//! of a different path sits at the same bytes), a removed edge starts at a removed twin Def, and a
//! changed label sits in an owner that lost a twin Def on or between its lines, or whose `Killed`
//! doubt names the line of a removed twin. The golden is `tests/fixtures/alias_twin_receipt_diff/
//! diff.jsonl` (labels pinned, not only presence); `PRISM_BLESS_ALIAS_TWIN_DIFF=1` rewrites it.
use super::without_policy;
use crate::ast::ParsedFile;
use crate::data_flow::build::{FileRefs, Producer};
use crate::data_flow::VarLocation;
use crate::languages::Language;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

type Key = (String, usize, usize, usize, String);

fn key(loc: &VarLocation) -> Key {
    (
        loc.function.clone(),
        loc.line,
        loc.start_byte,
        loc.end_byte,
        loc.path.to_string(),
    )
}

fn all_defs(refs: &FileRefs) -> BTreeSet<Key> {
    refs.defs.values().flatten().map(key).collect()
}

fn all_uses(refs: &FileRefs) -> BTreeSet<Key> {
    refs.uses.values().flatten().map(key).collect()
}

fn fixture_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        if name.starts_with("f-")
            || name.starts_with("F-private")
            || name == "private-corpus"
            || name == "secbench"
        {
            continue;
        }
        if path.is_dir() {
            fixture_files(&path, out);
        } else if let Some(language) = Language::from_path(path.to_str().unwrap()) {
            if matches!(
                language,
                Language::Go | Language::Rust | Language::Python | Language::C | Language::Cpp
            ) {
                out.push(path);
            }
        }
    }
}

fn fmt_key(k: &Key) -> Value {
    json!({"fn": k.0, "line": k.1, "bytes": [k.2, k.3], "path": k.4})
}

fn diff_of(rel: &str, source: &str, language: Language) -> Option<Value> {
    let parsed = ParsedFile::parse(rel, source, language).ok()?;
    let off = without_policy(|| Producer::file_refs(&rel.to_string(), &parsed));
    let on = Producer::file_refs(&rel.to_string(), &parsed);
    let (off_defs, on_defs) = (all_defs(&off), all_defs(&on));
    let removed_defs: Vec<&Key> = off_defs.difference(&on_defs).collect();
    assert!(
        on_defs.difference(&off_defs).next().is_none(),
        "{rel}: the policy added a Def"
    );
    let removed_uses: Vec<Key> = all_uses(&off).difference(&all_uses(&on)).cloned().collect();
    // Attribution of Defs: an alias twin shares its bytes with a surviving or removed Def of
    // another path of the same owner.
    for d in &removed_defs {
        assert!(
            off_defs
                .iter()
                .any(|o| o.0 == d.0 && o.2 == d.2 && o.3 == d.3 && o.4 != d.4),
            "{rel}: removed Def is not an alias twin: {d:?}"
        );
    }
    let removed_def_set: BTreeSet<&Key> = removed_defs.iter().copied().collect();
    let on_edges: BTreeSet<(Key, Key)> = on
        .edges
        .iter()
        .map(|e| (key(&e.from), key(&e.to)))
        .collect();
    let mut removed_edges = Vec::new();
    for e in &off.edges {
        let k = (key(&e.from), key(&e.to));
        if !on_edges.contains(&k) {
            assert!(
                removed_def_set.contains(&k.0)
                    || removed_uses.contains(&k.1)
                    || removed_def_set.contains(&k.1),
                "{rel}: removed edge not attributable to a twin: {k:?}"
            );
            removed_edges.push(json!([fmt_key(&k.0), fmt_key(&k.1)]));
        }
    }
    // An edge only the policy-on run has: the removed twin was a dominating write that pruned the
    // pair (C rules U and D, #811), so a removed twin Def of the same function lies on a line
    // between the endpoints (or anywhere in the function for a loop-carried pair).
    let off_edges: BTreeSet<(Key, Key)> = off
        .edges
        .iter()
        .map(|e| (key(&e.from), key(&e.to)))
        .collect();
    let mut added_edges = Vec::new();
    for e in &on.edges {
        let k = (key(&e.from), key(&e.to));
        if off_edges.contains(&k) {
            continue;
        }
        let attributed = removed_defs
            .iter()
            .any(|d| d.0 == k.0 .0 && (k.1 .1 <= k.0 .1 || (k.0 .1..=k.1 .1).contains(&d.1)));
        assert!(
            attributed,
            "{rel}: added edge not attributable to a removed twin: {k:?}"
        );
        added_edges.push(json!([fmt_key(&k.0), fmt_key(&k.1)]));
    }
    let twin_lines: BTreeMap<&str, BTreeSet<usize>> =
        removed_defs.iter().fold(BTreeMap::new(), |mut m, d| {
            m.entry(d.0.as_str()).or_default().insert(d.1);
            m
        });
    let mut relabeled = Vec::new();
    for ((from, to), label) in &off.labels {
        let Some(new) = on.labels.get(&(from.clone(), to.clone())) else {
            continue;
        };
        if new == label {
            continue;
        }
        let lines = twin_lines.get(from.function.as_str());
        let killed = match label {
            crate::cpg::FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { kill_line }) => {
                Some(*kill_line)
            }
            _ => None,
        };
        let attributed = lines.is_some_and(|ls| {
            killed.is_some_and(|k| ls.contains(&(k as usize)))
                || ls
                    .iter()
                    .any(|l| (from.line..=to.line.max(from.line)).contains(l))
        });
        assert!(
            attributed,
            "{rel}: label change not attributable to a twin: {from:?} -> {to:?}"
        );
        relabeled.push(json!({
            "from": fmt_key(&key(from)), "to": fmt_key(&key(to)),
            "off": format!("{label:?}"), "on": format!("{new:?}")
        }));
    }
    if removed_defs.is_empty()
        && removed_uses.is_empty()
        && removed_edges.is_empty()
        && added_edges.is_empty()
        && relabeled.is_empty()
    {
        return None;
    }
    let mut row = json!({
        "file": rel,
        "removed_twin_defs": removed_defs.iter().map(|k| fmt_key(k)).collect::<Vec<_>>(),
        "removed_minted_uses": removed_uses.iter().map(fmt_key).collect::<Vec<_>>(),
        "removed_edges": removed_edges,
        "relabeled": relabeled,
    });
    // Only C and C++ ever add edges (rules U and D, #811); the key is omitted when empty so the
    // rows of the earlier languages are unchanged.
    if !added_edges.is_empty() {
        row["added_edges"] = json!(added_edges);
    }
    Some(row)
}

#[test]
fn the_alias_twin_receipt_diff_over_the_fixture_tree_matches_the_golden() {
    crate::build_pool::install(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut files = Vec::new();
        fixture_files(&root, &mut files);
        files.sort();
        assert!(files.len() > 100, "fixture walk selected too little");
        let mut rows = Vec::new();
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            let language = Language::from_path(path.to_str().unwrap()).unwrap();
            let rel = path
                .strip_prefix(&root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if let Some(row) = diff_of(&rel, &source, language) {
                rows.push(row.to_string());
            }
        }
        let actual = rows.join("\n") + "\n";
        let golden_path = root.join("alias_twin_receipt_diff/diff.jsonl");
        if std::env::var_os("PRISM_BLESS_ALIAS_TWIN_DIFF").is_some() {
            std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
            std::fs::write(&golden_path, &actual).unwrap();
            return;
        }
        let golden = std::fs::read_to_string(&golden_path).expect("golden diff is committed");
        assert_eq!(actual, golden, "attributed alias-twin diff changed");
    });
}
