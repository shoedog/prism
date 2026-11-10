//! Full producer differential plus independently marked U/D obligations.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::path::PathBuf;

/// The S1 recall lane adds `Def(p.f) -> Use(ancestor)` edges that the frozen
/// reference never had; this differential compares everything else.
fn without_recall(mut refs: super::FileRefs) -> super::FileRefs {
    let recall = |from: &crate::data_flow::VarLocation, to: &crate::data_flow::VarLocation| {
        to.path != from.path && to.path.is_prefix_of(&from.path)
    };
    refs.edges.retain(|e| !recall(&e.from, &e.to));
    refs.labels.retain(|(from, to), _| !recall(from, to));
    refs
}

fn fixtures() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dfg_prune/go");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "go"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn go_prune_matrix() {
    let obligations: Vec<serde_json::Value> =
        include_str!("../../../tests/fixtures/dfg_prune/go/obligations.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
    let mut failures = Vec::new();
    for path in fixtures() {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let file = path.file_name().unwrap().to_str().unwrap().to_owned();
        let source = std::fs::read_to_string(&path).unwrap();
        let parsed = ParsedFile::parse(&file, &source, Language::Go).unwrap();
        let before = Producer::file_refs_frozen(&file, &parsed);
        let after = without_recall(Producer::file_refs(&file, &parsed));
        assert_eq!(
            before.edges.len() - after.edges.len(),
            after.rd_function_stats.dfg_pruned_write_only
                + after.rd_function_stats.dfg_pruned_dominated
                + after.rd_function_stats.dfg_pruned_scope
                + after.rd_function_stats.dfg_pruned_order,
            "{name}: prune counters must count removed expanded edges"
        );
        for (key, label) in &after.labels {
            assert_eq!(before.labels.get(key), Some(label), "D7: {name}: {key:?}");
        }
        for row in obligations.iter().filter(|r| r["case"] == name) {
            let has = |parts: &super::FileRefs| {
                parts.edges.iter().any(|e| {
                    e.from.path.to_string() == row["path"].as_str().unwrap()
                        && e.from.line == row["def_line"].as_u64().unwrap() as usize
                        && e.to.line == row["use_line"].as_u64().unwrap() as usize
                })
            };
            if !has(&before) {
                failures.push(format!("inadmissible missing baseline pair: {row}"));
            }
            if has(&after) != row["keep"].as_bool().unwrap() {
                failures.push(format!("{row}"));
            }
            if row["reason"] == "write_only" {
                let has_use = |parts: &super::FileRefs| {
                    parts.uses.values().flatten().any(|loc| {
                        loc.path.to_string() == row["path"].as_str().unwrap()
                            && loc.line == row["use_line"].as_u64().unwrap() as usize
                    })
                };
                assert!(
                    has_use(&before),
                    "baseline must mint the write anchor: {row}"
                );
                assert!(!has_use(&after), "U must not mint a write-only Use: {row}");
            }
        }
        if name == "01_case_scoped_var" || name == "03_switch_in_goto_loop" {
            let goto = name == "03_switch_in_goto_loop";
            let counts = if goto { (80, 15) } else { (79, 14) };
            if before.labels.len() != counts.0 || after.labels.len() != counts.1 {
                failures.push(format!(
                    "{name} complete set: {} -> {}, expected {counts:?}",
                    before.labels.len(),
                    after.labels.len()
                ));
            }
            let expected = [
                ("n", 5, 8),
                ("yyS", 5, 10),
                ("yyS", 5, 15),
                ("yyS", 5, 20),
                ("yyVAL", 6, 13),
                ("yyVAL", 6, 18),
                ("yyVAL", 6, 23),
                ("yyVAL", 6, 25),
                ("yyDollar", 10, 12),
                ("yyDollar", 20, 22),
                ("yyLOCAL", 12, 13),
                ("yyLOCAL", 17, 18),
                ("yyLOCAL", 21, 22),
                ("yyLOCAL", 22, 23),
            ];
            let actual: std::collections::BTreeSet<_> = after
                .labels
                .keys()
                .map(|(d, u)| (d.path.to_string(), d.line, u.line))
                .collect();
            let mut expected: std::collections::BTreeSet<_> = expected
                .into_iter()
                .map(|(p, d, u)| {
                    (
                        p.to_owned(),
                        if goto && d >= 10 { d + 1 } else { d },
                        if goto {
                            u + if u == 25 { 2 } else { 1 }
                        } else {
                            u
                        },
                    )
                })
                .collect();
            if goto {
                expected.insert(("n".to_owned(), 5, 26));
            }
            if actual != expected {
                failures.push(format!("case 1 full set: {actual:?}"));
            }
        }
        // Machine-readable baseline receipt; tests do not generate their own expected sets.
        if std::env::var_os("PRISM_DUMP_PRUNE_BASELINE").is_some() {
            for ((from, to), label) in &before.labels {
                println!(
                    "BASELINE {}",
                    serde_json::json!({"case":name,"from":from,"to":to,"label":label})
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} matrix failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn prune_subset_and_labels_across_fixture_tree() {
    crate::build_pool::install(|| {
        fn walk(dir: &std::path::Path, paths: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_str().unwrap();
                if name.starts_with("f-")
                    || name.starts_with("F-private")
                    || name == "private-corpus"
                    || name == "secbench"
                {
                    continue;
                }
                if path.is_dir() {
                    walk(&path, paths);
                } else if Language::from_path(path.to_str().unwrap()).is_some() {
                    paths.push(path);
                }
            }
        }
        let mut paths = Vec::new();
        walk(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
            &mut paths,
        );
        paths.sort();
        assert!(
            paths.len() > 100,
            "fixture walk must not silently select nothing"
        );
        let mut edges = 0;
        for path in &paths {
            let file = path.to_str().unwrap().to_owned();
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            let language = Language::from_path(&file).unwrap();
            let parsed = ParsedFile::parse(&file, &source, language).unwrap();
            let before = Producer::file_refs_frozen(&file, &parsed);
            let after = without_recall(Producer::file_refs(&file, &parsed));
            for edge in &after.edges {
                assert!(
                    before.edges.contains(edge),
                    "added edge in {file}: {edge:?}"
                );
            }
            if language == Language::Java {
                // Alias-twin lane slice 1: Java twins are removed at binding writes, so labels
                // and full equality may differ from the frozen reference; removal-only still
                // holds, edges (above) and uses.
                for (key, uses) in &after.uses {
                    for u in uses {
                        assert!(
                            before.uses.get(key).is_some_and(|b| b.contains(u)),
                            "added use in {file}: {key:?} {u:?}"
                        );
                    }
                }
                edges += after.edges.len();
                continue;
            }
            for (key, label) in &after.labels {
                assert_eq!(
                    before.labels.get(key),
                    Some(label),
                    "label changed in {file}: {key:?}"
                );
            }
            for (key, label) in &after.exact_labels {
                assert_eq!(
                    before.exact_labels.get(key),
                    Some(label),
                    "exact label changed in {file}: {key:?}"
                );
            }
            if language == Language::Python {
                // Opus F1 invariant: a removed edge into a minted use leaves another
                // surviving edge from a def of the path into that same use.
                let minted: std::collections::BTreeSet<_> = after.uses.values().flatten().collect();
                for edge in before.edges.iter().filter(|e| !after.edges.contains(e)) {
                    if minted.contains(&edge.to) {
                        assert!(
                            after.edges.iter().any(|e| e.to == edge.to
                                && e.from.path == edge.from.path
                                && e.from != edge.from)
                                // A prefix-path write (`self.x = b` after `self.x.y = a`) is
                                // the dominator of a `self`/`cls` attribute path; it reaches
                                // the same line's use of the prefix.
                                || (edge.from.path.fields.len() > 1
                                    && after.edges.iter().any(|e| e.to.line == edge.to.line
                                        && e.from.path.base == edge.from.path.base
                                        && e.from.path.fields.len() < edge.from.path.fields.len()
                                        && edge.from.path.fields.starts_with(&e.from.path.fields))),
                            "removed {edge:?} left no dominator edge in {file}"
                        );
                    }
                }
            }
            if language != Language::Go && language != Language::Python {
                assert_eq!(before.edges, after.edges, "non-Go/Python changed: {file}");
                assert_eq!(
                    before.uses, after.uses,
                    "non-Go/Python uses changed: {file}"
                );
            }
            edges += after.edges.len();
        }
        println!(
            "D7: {} files, {edges} surviving expanded edges",
            paths.len()
        );
    });
}

#[test]
fn prune_stats_cache_and_incremental_custody() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{self, CacheResult};
    use std::collections::{BTreeMap, BTreeSet};
    let mut sources = BTreeMap::from([
        (
            "a.go".to_owned(),
            "package p\nfunc a(x int) int {\nx = 1\nx = 2\nreturn x\n}\n".to_owned(),
        ),
        (
            "b.go".to_owned(),
            "package p\nfunc b(y int) int {\ny = 3\ny = 4\nreturn y\n}\n".to_owned(),
        ),
    ]);
    let parse = |sources: &BTreeMap<String, String>| {
        sources
            .iter()
            .map(|(f, s)| (f.clone(), ParsedFile::parse(f, s, Language::Go).unwrap()))
            .collect()
    };
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);
    assert!(cold.dfg_label_stats.dfg_pruned_write_only > 0);
    assert!(cold.dfg_label_stats.dfg_pruned_dominated > 0);
    assert_eq!(cold.dfg_label_stats.dfg_pruned_order, 0);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(cold.dfg_label_stats, hit.dfg_label_stats);
    assert_eq!(cold.dfg.rd_function_stats(), hit.dfg.rd_function_stats());
    sources.insert(
        "a.go".into(),
        "package p\nfunc a(x int) int {\nreturn x\n}\n".into(),
    );
    let edited_files = parse(&sources);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["a.go".into()]));
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited_files,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited_files);
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    assert_eq!(
        incremental.dfg.rd_function_stats()["a.go"].dfg_pruned_write_only,
        0
    );
    assert!(incremental.dfg.rd_function_stats()["b.go"].dfg_pruned_write_only > 0);
}

#[test]
fn prune_uncertain_paths_and_mixed_line_keep_controls() {
    let cases = [
        (
            "indexed",
            "package p\nfunc f(a []int) {\na[0] = 1\na[1] = 2\nuse(a[0])\n}\n",
            "a",
            2,
            4,
        ),
        (
            "indexed_call_receiver",
            "package p\nfunc f(a []S) {\na[0].cell().value = 1\na[1].cell().value = 2\nuse(a[0].cell())\n}\n",
            "a[]",
            3,
            5,
        ),
        (
            "dereference",
            "package p\nfunc f(p *int) {\n*p = 1\n*p = 2\nuse(*p)\n}\n",
            "p",
            3,
            5,
        ),
        (
            "mixed_line",
            "package p\nfunc f(x int) {\nx = 1\nx = 2; use(x)\n}\n",
            "x",
            3,
            4,
        ),
        (
            "self_read",
            "package p\nfunc f(x int) {\nx = 1\nx = x + 1\n}\n",
            "x",
            3,
            4,
        ),
        (
            "syntax_error_owner",
            "package p\nfunc f(x int) {\nx = 1\nx = 2\nuse(x)\n@\n}\n",
            "x",
            3,
            5,
        ),
    ];
    let mut failures = Vec::new();
    for (name, source, path, def_line, use_line) in cases {
        let file = format!("{name}.go");
        let parsed = ParsedFile::parse(&file, source, Language::Go).unwrap();
        if name == "syntax_error_owner" {
            assert!(
                parsed.all_functions().iter().any(|owner| owner.has_error()),
                "must exercise an erroneous owner"
            );
        }
        let before = Producer::file_refs_frozen(&file, &parsed);
        let after = Producer::file_refs(&file, &parsed);
        let has = |parts: &super::FileRefs| {
            parts.edges.iter().any(|e| {
                e.from.path.to_string() == path && e.from.line == def_line && e.to.line == use_line
            })
        };
        if !has(&before) {
            failures.push(format!("inadmissible: {name}: {:?}", before.edges));
        }
        if !has(&after) {
            failures.push(format!("missing after: {name}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "explicit receipt export; requires PRISM_PRUNE_PROBE_ROOT"]
fn export_go_prune_probe_pairs() {
    use super::lexical_prune::LexicalFacts;
    let root = PathBuf::from(std::env::var_os("PRISM_PRUNE_PROBE_ROOT").expect("probe root"));
    let mut count = 0;
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with("go_"))
        .collect();
    paths.sort();
    for dir in paths {
        let path = std::fs::read_dir(&dir)
            .unwrap()
            .map(|p| p.unwrap().path())
            .find(|p| p.extension().is_some_and(|e| e == "go"))
            .expect("Go probe");
        let file = path.file_name().unwrap().to_str().unwrap().to_owned();
        let parsed = ParsedFile::parse(
            &file,
            &std::fs::read_to_string(&path).unwrap(),
            Language::Go,
        )
        .unwrap();
        let before = Producer::file_refs_frozen(&file, &parsed);
        let after = Producer::file_refs(&file, &parsed);
        for (key, label) in &after.labels {
            assert_eq!(before.labels.get(key), Some(label), "probe D7");
        }
        for ((from, to), label) in &before.labels {
            let kept = after.labels.contains_key(&(from.clone(), to.clone()));
            let reason = if kept {
                "keep"
            } else {
                let owner = parsed
                    .all_functions()
                    .into_iter()
                    .find(|n| {
                        parsed.node_line_range(n).0 == from.function_start_line
                            && parsed.dfg_owner_name(n).as_deref() == Some(&from.function)
                    })
                    .expect("owner");
                let (start, end) = parsed.node_line_range(&owner);
                let lines = (start..=end).collect();
                let lvalues = parsed.assignment_lvalue_spans_on_lines(&owner, &lines);
                let rvalues = parsed.rvalue_identifier_spans_on_lines(&owner, &lines);
                let facts = LexicalFacts::new(&parsed, owner, &lvalues, &rvalues);
                let mut stats = crate::cpg::RdFileStats::default();
                if facts.drop_write_only(&from.path, to.line, &mut stats) {
                    "write_only"
                } else {
                    assert!(!facts.keep_def(
                        &from.path,
                        from.start_byte,
                        from.end_byte,
                        to.line,
                        &mut stats
                    ));
                    "dominated"
                }
            };
            println!(
                "PROBE {}",
                serde_json::json!({"probe":dir.file_name().unwrap().to_str().unwrap(),"from":from,"to":to,"label":label,"keep":kept,"reason":reason})
            );
        }
        count += 1;
    }
    assert_eq!(count, 3, "must select all three Go probes");
}

#[test]
fn python_prune_stats_cache_and_incremental_custody() {
    use crate::cpg::CodePropertyGraph;
    use crate::cpg_cache::{self, CacheResult};
    use std::collections::{BTreeMap, BTreeSet};
    // Each file has a dominated def (D) and a write-only line (U).
    let mut sources = BTreeMap::from([
        (
            "a.py".to_owned(),
            "def a():\n    x = 1\n    x = 2\n    use(x)\n".to_owned(),
        ),
        (
            "b.py".to_owned(),
            "def b():\n    y = 3\n    y = 4\n    use(y)\n".to_owned(),
        ),
    ]);
    let parse = |sources: &BTreeMap<String, String>| {
        sources
            .iter()
            .map(|(f, s)| {
                (
                    f.clone(),
                    ParsedFile::parse(f, s, Language::Python).unwrap(),
                )
            })
            .collect()
    };
    let files = parse(&sources);
    let cold = CodePropertyGraph::build(&files);
    assert!(cold.dfg_label_stats.dfg_pruned_write_only > 0);
    assert!(cold.dfg_label_stats.dfg_pruned_dominated > 0);
    assert_eq!(cold.dfg_label_stats.dfg_pruned_order, 0);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected full hit");
    };
    assert_eq!(cold.dfg_label_stats, hit.dfg_label_stats);
    assert_eq!(cold.dfg.rd_function_stats(), hit.dfg.rd_function_stats());
    sources.insert("a.py".into(), "def a():\n    return 1\n".into());
    let edited_files = parse(&sources);
    let hashes = cpg_cache::compute_file_hashes(&sources);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&hashes, false, dir.path())
    else {
        panic!("expected partial hit");
    };
    assert_eq!(changed_files, BTreeSet::from(["a.py".into()]));
    let incremental = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &edited_files,
        None,
    );
    let rebuilt = CodePropertyGraph::build(&edited_files);
    assert_eq!(incremental.dfg_label_stats, rebuilt.dfg_label_stats);
    assert_eq!(
        incremental.dfg.rd_function_stats(),
        rebuilt.dfg.rd_function_stats()
    );
    let stats = incremental.dfg.rd_function_stats();
    assert_eq!(stats["a.py"].dfg_pruned_write_only, 0);
    assert_eq!(stats["a.py"].dfg_pruned_dominated, 0);
    assert!(stats["b.py"].dfg_pruned_write_only > 0);
    assert!(stats["b.py"].dfg_pruned_dominated > 0);
}
