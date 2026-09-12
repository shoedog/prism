use crate::cpg::{CodePropertyGraph, FlowConfidence, FlowDoubt};
use crate::data_flow::VarLocation;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[test]
fn every_adjacent_pair_is_ordered() {
    let ordered = [
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 11 }),
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 37 }),
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
        FlowConfidence::NameOnly(FlowDoubt::SameLine),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
    ];

    for pair in ordered.windows(2) {
        let better = pair[0];
        let worse = pair[1];
        assert_eq!(better.worst(worse), worse, "{better:?} < {worse:?}");
        assert_eq!(worse.worst(better), worse, "{worse:?} > {better:?}");
    }
}

fn site(location: &VarLocation) -> String {
    format!(
        "{}:{}:{}:{}:{}:{:?}",
        location.file,
        location.function,
        location.function_start_line,
        location.line,
        location.path,
        location.kind
    )
}

fn real_folded_paths(fixture: &str) -> BTreeMap<(String, String), FlowConfidence> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("eval/fixtures")
        .join(fixture);
    let repo = crate::repo_loader::load_repo(&root).unwrap();
    let cpg = crate::build_pool::install(|| CodePropertyGraph::build(&repo.files));
    let defs: BTreeSet<VarLocation> = cpg.dfg.defs.values().flatten().cloned().collect();
    let mut paths = BTreeMap::new();

    for from in defs {
        let (reachable, labels) = cpg.dfg_forward_reachable_labeled(&from);
        for to in reachable {
            let target = cpg.var_node_for_location(&to).unwrap();
            paths.insert((site(&from), site(&to)), labels[&target]);
        }
    }
    paths
}

#[test]
fn ruled_path_label_delta_is_exact() {
    type Row = (
        &'static str,
        &'static str,
        &'static str,
        FlowConfidence,
        FlowConfidence,
    );
    const ALIAS: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);
    const CFG: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    macro_rules! row {
        ($fixture:literal, $from:literal, $to:literal) => {
            ($fixture, $from, $to, ALIAS, CFG)
        };
    }

    let expected: [Row; 31] = [
        row!(
            "go/dfg_reaching_alias_conservative",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:p:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:p:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:p.x:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:q.x:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:6:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:5:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:q:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_conservative",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:p:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:p:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:p.x:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:q.x:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:5:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:4:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:q:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "rust/dfg_reaching_alias_conservative",
            "main.rs:f:1:2:q:Def",
            "main.rs:f:1:4:q.x:Use"
        ),
    ];

    let mut snapshots = BTreeMap::new();
    let observed: Vec<Row> = expected
        .iter()
        .map(|&(fixture, from, to, old, new)| {
            let paths = snapshots
                .entry(fixture)
                .or_insert_with(|| real_folded_paths(fixture));
            let actual = paths
                .get(&(from.to_string(), to.to_string()))
                .copied()
                .unwrap_or_else(|| panic!("missing real folded path: {fixture}: {from} -> {to}"));
            assert_ne!(old, new, "delta row must change: {fixture}: {from} -> {to}");
            (fixture, from, to, old, actual)
        })
        .collect();

    assert_eq!(observed, expected);
}
