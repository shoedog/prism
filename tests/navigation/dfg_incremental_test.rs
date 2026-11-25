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
        // One census read for the whole tree: a `build_with_census_files` call per file
        // recomputed it N times (~90% of this test's 130 s, #856).
        let mut singles: BTreeMap<_, _> =
            DataFlowGraph::build_each_file_with_census_files(&files, &files)
                .into_iter()
                .collect();
        let mut expected_edges = vec![];
        for file in order {
            expected_edges.extend(
                singles
                    .remove(file)
                    .unwrap()
                    .edges()
                    .iter()
                    .map(|e| e.to_owned()),
            );
        }
        let actual_edges: Vec<_> = current.dfg.edges().iter().map(|e| e.to_owned()).collect();
        // Per-file singleton builds read the Rust macro census from the whole tree
        // (`build_with_census_files`), as the assembled build does. They cannot see
        // sibling files for Go package fields, so they keep Go
        // receiver edges that the assembled repository refuses (a func-typed
        // field of the package, `go_package_fields`). The oracle is therefore
        // the singleton concatenation with only those edges removed, in place:
        // every other edge keeps its bytes and its order.
        let (mut kept, mut removed, mut at) = (vec![], vec![], 0);
        for edge in &expected_edges {
            if actual_edges.get(at) == Some(edge) {
                kept.push(edge.clone());
                at += 1;
            } else {
                removed.push(edge);
            }
        }
        assert_eq!(
            at,
            actual_edges.len(),
            "actual is not a subsequence of the singleton edges {path} delete={delete}"
        );
        assert_eq!(
            removed.len(),
            current.dfg_label_stats.dfg_recall_receiver_field_refused,
            "only refused receiver edges are removed {path} delete={delete}"
        );
        // Independent of the producer: each removed edge is a field write
        // reaching a whole-object use that is the receiver of `<use>.<name>(`
        // on its line, with `<name> func` declared in some Go file.
        for e in &removed {
            let go = |f: &str| f.ends_with(".go");
            assert!(go(&e.from.file) && go(&e.to.file), "{e:?} is not a Go edge");
            let (from, to) = (e.from.path.to_string(), e.to.path.to_string());
            assert!(
                from.len() > to.len() && from.starts_with(&format!("{to}.")),
                "removed edge is not field-write -> whole object: {from} -> {to}"
            );
            let source = &files[&e.to.file].source;
            let line = source.lines().nth(e.to.line - 1).unwrap_or_default();
            let call = format!("{to}.");
            let declared = line.match_indices(&call).any(|(i, _)| {
                let rest = &line[i + call.len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                !name.is_empty()
                    && rest[name.len()..].starts_with('(')
                    && files.iter().filter(|(f, _)| go(f)).any(|(_, f)| {
                        f.source.match_indices(name.as_str()).any(|(j, _)| {
                            let word_start = f.source[..j]
                                .chars()
                                .next_back()
                                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                            let after = &f.source[j + name.len()..];
                            let spaced = after.trim_start_matches([' ', '\t']);
                            word_start && spaced.len() < after.len() && spaced.starts_with("func")
                        })
                    })
            });
            assert!(
                declared,
                "removed edge {from} -> {to} at {}:{} is not a func-field receiver pair",
                e.to.file, e.to.line
            );
        }
        assert_eq!(
            bincode::serialize(&kept).unwrap(),
            bincode::serialize(&actual_edges).unwrap(),
            "incremental ordered edge bytes {path} delete={delete}"
        );
        // The cold reference goes through the same assembly (and refusals).
        let cold = CodePropertyGraph::build(&files).dfg;
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

/// `build_each_file_with_census_files` is `build_with_census_files` per file with the census
/// read once; the oracle in `partial_hit_multifile_edit_retain_delete` relies on that.
///
/// The comparison is over the serialized graph (spans, bindings, both label maps, per-file
/// statistics, counters and both census digests), not edge identities. The sample holds one Rust
/// file and one C file whose graph depends on a macro defined in ANOTHER file that is in the tree
/// but not in the sample, so a census read from the sample or a singleton instead of the whole
/// tree fails it through those files' edges as well as through the digests (#856).
#[test]
fn build_each_file_matches_per_file_builds_under_the_tree_census() {
    prism::build_pool::install(|| {
        let mut tree = None;
        common::dfg_cases::visit(|name, files| {
            if name == "fixture-tree" {
                tree = Some(files);
            }
        });
        let mut files = tree.unwrap();
        // A stride sample of the fixture tree, taken before the synthetic files exist.
        let mut sample: BTreeMap<_, _> = files
            .iter()
            .step_by(37)
            .map(|(p, f)| (p.clone(), f.clone()))
            .collect();
        // Cross-file macro users. The definitions (`xfile_macros/{a.rs,m.h}`) are in the tree but
        // NOT in the sample, so a census read from the sample or from the file alone does not know
        // them. Rust: with the definition `show!(x)` is an Allowed macro, `x = 2` dominates its
        // read and the `x = 1` pairs are pruned; without it the invocation is Refused and they are
        // kept. C: with `#define SHOW` the argument `x` escapes and the pairs are kept; without it
        // `SHOW(x)` is an ordinary call and they are pruned.
        let definers = [
            (
                "xfile_macros/a.rs",
                "macro_rules! show { ($e:expr) => { $e + 1 }; }\n",
                Language::Rust,
            ),
            (
                "xfile_macros/m.h",
                "#define SHOW(v) ((v) + 1)\n",
                Language::C,
            ),
        ];
        let users = [
            (
                "xfile_macros/b.rs",
                "fn f() -> i32 {\n    let mut x = 1;\n    x = 2;\n    let y = show!(x);\n    y\n}\n",
                Language::Rust,
            ),
            (
                "xfile_macros/c.c",
                "int f(void) {\n    int x = 1;\n    x = 2;\n    return SHOW(x);\n}\n",
                Language::C,
            ),
        ];
        for (path, src, lang) in definers.into_iter().chain(users) {
            files.insert(path.into(), ParsedFile::parse(path, src, lang).unwrap());
        }
        for (path, _, _) in users {
            sample.insert(path.into(), files[path].clone());
        }
        assert!(sample.len() > 20, "sample too small: {}", sample.len());
        let each = DataFlowGraph::build_each_file_with_census_files(&sample, &files);
        assert_eq!(
            each.iter().map(|(p, _)| p).collect::<Vec<_>>(),
            sample.keys().collect::<Vec<_>>(),
            "one graph per file, in path order"
        );
        let bytes = |g: &DataFlowGraph| bincode::serialize(g).unwrap();
        for (path, graph) in &each {
            let single = BTreeMap::from([(path.clone(), sample[path].clone())]);
            let reference = DataFlowGraph::build_with_census_files(&single, &files);
            assert!(
                bytes(graph) == bytes(&reference),
                "serialized graph differs from the per-file build for {path}"
            );
        }
        // Non-vacuity: for the macro users the tree census must matter, i.e. a census taken from
        // the file alone gives another graph.
        for path in ["xfile_macros/b.rs", "xfile_macros/c.c"] {
            let single = BTreeMap::from([(path.to_string(), sample[path].clone())]);
            let own_census = DataFlowGraph::build_with_census_files(&single, &single);
            let graph = &each.iter().find(|(p, _)| p == path).unwrap().1;
            let edges =
                |g: &DataFlowGraph| g.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>();
            let (tree_n, own_n) = (edges(graph).len(), edges(&own_census).len());
            // Rust: the Allowed macro lets the dominating write prune pairs (fewer edges); C: the
            // known macro escapes its argument (more edges).
            let expected = if path.ends_with(".rs") {
                tree_n < own_n
            } else {
                tree_n > own_n
            };
            assert!(
                expected,
                "{path}: the whole-tree census must change its edges ({tree_n} vs {own_n} with its own census), or the test cannot see a wrong census"
            );
        }
        assert!(
            each.iter().any(|(_, g)| !g.edges().is_empty()),
            "the sample must produce edges, or the comparison is vacuous"
        );
    });
}
