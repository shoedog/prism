//! #693/#697 slice 2 fold 1: the loop plan is a pure function of one file, so a file holding one admitted and
//! one refused loop tree gives the same labels and the same counters (`dfg_loop_plan` included) on a cold
//! build, a direct full cache hit and a partial hit that rebuilds the file.
use super::*;
use crate::ast::ParsedFile;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

/// `ok` is admitted (a plain multi-line `while`); `bad` holds a `while` whose body starts on its head line
/// (`HeadCollapse`), so it is refused and its owner labelled `cfg_incomplete`.
const SOURCE: &str = "function ok(c) {\n  let z = 1;\n  let x = 0;\n  while (c()) {\n    x = g();\n  }\n  sink(x, z);\n}\nfunction bad(c) {\n  let z = 1;\n  while (c()) x = g();\n  sink(x, z);\n}\n";

fn summary(g: &CodePropertyGraph) -> String {
    format!("{:#?}\n{:#?}", label_snapshot(&g.dfg), g.dfg_label_stats,)
}

/// Slice 3 (#693): the three shapes whose admission changed. `wrapped` ends an `if` arm in a `try` holding the
/// loop (was `wrapper_entry_then_end`), `cb` holds a `do` in a nested callable (was `foreign_admitted`), `decl`
/// holds a hoisted function declaration in a loop body (was a placement violation). All three trees are
/// admitted now, so their universes, edges, labels and counters must survive a cache round trip unchanged.
const SLICE3: &str = "function wrapped(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        x = 1;\n      }\n    } catch (e) {\n      x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\nfunction cb(xs) {\n  return xs.map((k) => {\n    let y = 0;\n    do {\n      use(y);\n      y = f();\n    } while ((y = g(k)));\n    return y;\n  });\n}\nfunction decl(n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    function g() { return z; }\n    z = g();\n  }\n  sink(z);\n}\n";

#[test]
fn an_admitted_and_a_refused_tree_agree_on_cold_full_hit_and_partial_hit() {
    parity(SOURCE, |plan| {
        assert_eq!(plan.get("trees"), Some(&2), "{plan:?}");
        assert_eq!(plan.get("admitted"), Some(&1), "{plan:?}");
        assert_eq!(plan.get("owners_refused"), Some(&1), "{plan:?}");
    });
}

#[test]
fn trees_whose_admission_changed_in_slice_3_agree_on_cold_full_hit_and_partial_hit() {
    parity(SLICE3, |plan| {
        assert_eq!(plan.get("trees"), Some(&3), "{plan:?}");
        // `wrapped` holds a `try`: since T2 its owner is refused whole (`TryFlow`); `cb` and `decl` stay admitted.
        assert_eq!(plan.get("admitted"), Some(&2), "{plan:?}");
        assert_eq!(plan.get("refused.try_flow"), Some(&1), "{plan:?}");
        assert_eq!(plan.get("owners_refused"), Some(&1), "{plan:?}");
    });
}

/// Slice 4 (#693): `sib` (a sibling-field update) was admitted and is refused now (`ForInit`, its owner keeps main's
/// labels); `same` (an exact-path update) stays admitted. Both survive a cache round trip unchanged.
const SLICE4: &str = "function sib(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.y++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\nfunction same(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.x++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n";

#[test]
fn trees_whose_admission_changed_in_slice_4_agree_on_cold_full_hit_and_partial_hit() {
    parity(SLICE4, |plan| {
        assert_eq!(plan.get("trees"), Some(&2), "{plan:?}");
        assert_eq!(plan.get("admitted"), Some(&1), "{plan:?}");
        assert_eq!(plan.get("refused.legacy.for_init"), Some(&1), "{plan:?}");
        assert_eq!(plan.get("owners_refused"), Some(&1), "{plan:?}");
    });
}

fn parity(source: &str, check_plan: impl Fn(&BTreeMap<String, usize>)) {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    let sources = BTreeMap::from([("a.js".to_string(), source.to_string())]);
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(p, s)| {
            (
                p.clone(),
                ParsedFile::parse(p, s, Language::JavaScript).unwrap(),
            )
        })
        .collect();
    let cold = CodePropertyGraph::build(&files);
    check_plan(&cold.dfg_label_stats.dfg_loop_plan);
    let hashes = compute_file_hashes(&sources);
    let dir = tempfile::tempdir().unwrap();
    save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(full) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let CacheResult::Hit(loaded) = load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss");
    };
    let partial = CodePropertyGraph::build_incremental(
        loaded.call_graph,
        loaded.dfg,
        &BTreeSet::from(["a.js".to_string()]),
        &files,
        None,
    );
    let expected = summary(&cold);
    assert_eq!(summary(&full), expected, "direct full hit");
    assert_eq!(summary(&partial), expected, "partial hit");
}
