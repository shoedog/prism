//! Exercise the real partial-hit route, retaining files through edits and deletion.
#[path = "../common/mod.rs"]
mod common;
use common::dfg_roundtrip::{cpg_bytes, lookup_indexes};
use common::*;
use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};

fn transitions(mut files: BTreeMap<String, ParsedFile>) {
    assert!(files.len() > 1);
    let paths: Vec<_> = files.keys().cloned().collect();
    let mut current = CodePropertyGraph::build(&files);
    // Shift existing spans/lines near the top and add a function, retain all
    // companions, then delete a different file. Append-only edits miss stale spans.
    for (path, delete) in [(&paths[0], false), (&paths[1], true)] {
        let cache = TempDir::new().unwrap();
        let sources = files
            .iter()
            .map(|(p, f)| (p.clone(), f.source.clone()))
            .collect();
        let before = compute_file_hashes(&sources);
        save_cache(&current, &before, false, cache.path()).unwrap();
        if delete {
            files.remove(path);
        } else {
            let parsed = &files[path];
            let suffix = match parsed.language {
                Language::Python => "\ndef compaction_added(x):\n    y = x\n    return y\n",
                Language::C | Language::Cpp => {
                    "\nint compaction_added(int x) { int y = x; return y; }\n"
                }
                Language::Go => "\nfunc compaction_added(x int) int { y := x; return y }\n",
                Language::JavaScript | Language::TypeScript | Language::Tsx => {
                    "\nfunction compaction_added(x) { let y = x; return y; }\n"
                }
                Language::Java => {
                    "\nclass CompactionAdded { static int f(int x) { int y = x; return y; } }\n"
                }
                Language::Rust => "\nfn compaction_added(x: i32) -> i32 { let y = x; y }\n",
                Language::Lua => "\nlocal function compaction_added(x) local y = x; return y end\n",
                Language::Terraform => "\nlocals { compaction_added = 7 }\n",
                Language::Bash => "\ncompaction_added() { local y=$1; echo \"$y\"; }\n",
            };
            let prefix = match parsed.language {
                Language::Python | Language::Terraform | Language::Bash => "# shifted spans\n",
                Language::Lua => "-- shifted spans\n",
                _ => "// shifted spans\n",
            };
            // Keep an interpreter directive as the first line when present.
            let source = if parsed.source.starts_with("#!") {
                let (first, rest) = parsed.source.split_once('\n').unwrap();
                format!("{first}\n{prefix}{rest}{suffix}")
            } else {
                format!("{prefix}{}{suffix}", parsed.source)
            };
            files.insert(
                path.clone(),
                ParsedFile::parse(path, &source, parsed.language).unwrap(),
            );
        }
        let sources = files
            .iter()
            .map(|(p, f)| (p.clone(), f.source.clone()))
            .collect();
        let after = compute_file_hashes(&sources);
        let expected_changed = BTreeSet::from([path.clone()]);
        let (cg, dfg, changed) = if delete {
            // Cache policy rejects a changed file set. Still exercise deletion
            // through the public incremental builder using the pre-delete hit.
            assert!(matches!(
                load_cache(&after, false, cache.path()),
                CacheResult::Miss
            ));
            match load_cache(&before, false, cache.path()) {
                CacheResult::Hit(g) => (g.call_graph, g.dfg, expected_changed.clone()),
                _ => panic!("expected pre-delete hit"),
            }
        } else {
            match load_cache(&after, false, cache.path()) {
                CacheResult::PartialHit {
                    cached_call_graph,
                    cached_dfg,
                    changed_files,
                } => (cached_call_graph, cached_dfg, changed_files),
                _ => panic!("expected real PartialHit for edited {path}"),
            }
        };
        assert_eq!(changed, expected_changed);
        current = CodePropertyGraph::build_incremental(cg, dfg, &changed, &files, None);
        // The two authored transitions have a fixed expected file sequence:
        // editing the first file puts it last; deleting the second retains that order.
        let order = paths
            .iter()
            .skip(if delete { 2 } else { 1 })
            .chain(std::iter::once(&paths[0]));
        let mut expected_edges = vec![];
        for file in order {
            let singleton = BTreeMap::from([(file.clone(), files[file].clone())]);
            expected_edges.extend(
                DataFlowGraph::build(&singleton)
                    .edges()
                    .iter()
                    .map(|e| e.to_owned()),
            );
        }
        let actual_edges: Vec<_> = current.dfg.edges().iter().map(|e| e.to_owned()).collect();
        assert_eq!(
            bincode::serialize(&expected_edges).unwrap(),
            bincode::serialize(&actual_edges).unwrap(),
            "incremental ordered edge bytes {path} delete={delete}"
        );
        let cold = DataFlowGraph::build(&files);
        let bindings = |g: &DataFlowGraph| {
            bincode::serialize(&(
                g.defs().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
                g.uses().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
                g.labels()
                    .map(|(e, c)| ((e.from, e.to), c))
                    .collect::<Vec<_>>(),
            ))
            .unwrap()
        };
        assert_eq!(
            bindings(&cold),
            bindings(&current.dfg),
            "incremental bindings/label representatives {path}"
        );
        assert!(current
            .dfg
            .edges()
            .iter()
            .all(|e| files.contains_key(&e.from.file) && files.contains_key(&e.to.file)));
        // Native cache roundtrip must preserve both payload and public lookup behavior.
        let cache = TempDir::new().unwrap();
        save_cache(&current, &after, false, cache.path()).unwrap();
        let CacheResult::Hit(reference) = load_cache(&after, false, cache.path()) else {
            panic!("expected incremental full hit")
        };
        assert!(
            cpg_bytes(&reference) == cpg_bytes(&current),
            "incremental payload {path} delete={delete}"
        );
        assert_eq!(
            lookup_indexes(&reference),
            lookup_indexes(&current),
            "incremental indexes {path}"
        );
    }
}
#[test]
fn partial_hit_multifile_edit_retain_delete() {
    prism::build_pool::install(|| {
        let mut count = 0;
        common::dfg_cases::visit(|name, files| {
            if files.len() > 1 {
                eprintln!("DFG_PARTIAL {name}: edit/retain, delete/retain");
                transitions(files);
                count += 1;
            }
        });
        assert!(count > 1, "multi-file fixture tree and generators must run");
        eprintln!(
            "DFG_PARTIAL {count} multi-file cases, {} transitions",
            count * 2
        );
    });
}
#[test]
fn partial_hit_javascript_exact_labels() {
    prism::build_pool::install(|| {
        let source = "function outer(value){sink(value);return sink(value);}\n";
        let files = ["a.js", "b.js", "retained.js"]
            .into_iter()
            .map(|p| {
                (
                    p.into(),
                    ParsedFile::parse(p, source, Language::JavaScript).unwrap(),
                )
            })
            .collect();
        transitions(files);
    });
}
