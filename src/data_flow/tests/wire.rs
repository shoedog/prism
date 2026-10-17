use super::super::compact::{
    wire::{WireDfg, WireDfgRef},
    *,
};
use super::super::*;
use super::representatives::adversarial_parts;
use test_support::{compact::storage, parts};

#[test]
fn compact_stream_and_owned_bytes_match() {
    let p = adversarial_parts();
    let c = CompactDfg::from_parts(p.clone()).unwrap();
    let borrowed = bincode::serialize(&WireDfgRef::from(&c)).unwrap();
    let owned = bincode::serialize(&WireDfg::from(c.clone())).unwrap();
    let mut stream = Vec::new();
    bincode::serialize_into(&mut stream, &c).unwrap();
    assert_eq!(borrowed, owned);
    assert_eq!(stream, owned);
    let decoded: CompactDfg = bincode::deserialize(&borrowed).unwrap();
    parts::assert_inputs(
        &p,
        &DataFlowGraph {
            storage: decoded.clone(),
        },
    );
    parts::compact_roundtrip(&decoded);
    assert_eq!(storage(&c), storage(&decoded));
}

#[test]
fn sparse_primary_states_roundtrip_without_normalization() {
    for selected in 0..=7 {
        let mut p = adversarial_parts();
        if selected != 0 {
            p.edges.clear();
        }
        if selected != 1 {
            p.defs.clear();
        }
        if selected != 2 {
            p.uses.clear();
        }
        if selected != 3 {
            p.labels.clear();
        }
        if selected != 4 {
            p.exact_labels.clear();
        }
        if selected != 5 {
            p.adjacency = Some((BTreeMap::new(), BTreeMap::new()));
        }
        if selected != 6 {
            p.rd_function_stats.clear();
        }
        let c = CompactDfg::from_parts(p.clone()).unwrap();
        let bytes = bincode::serialize(&c).unwrap();
        let decoded: CompactDfg = bincode::deserialize(&bytes).unwrap();
        parts::assert_inputs(
            &p,
            &DataFlowGraph {
                storage: decoded.clone(),
            },
        );
        parts::compact_roundtrip(&decoded);
        assert_eq!(bytes, bincode::serialize(&decoded).unwrap());
    }
}

#[test]
fn colliding_label_merge_allows_unreferenced_rows_until_update() {
    use super::identity::loc;
    let label = |start| {
        let mut p = test_support::parts::empty();
        p.labels.insert(
            (
                loc("x", VarAccessKind::Def, start),
                loc("x", VarAccessKind::Use, start + 1),
            ),
            FlowConfidence::Exact,
        );
        p
    };
    let p = label(10);
    let mut c = CompactDfg::from_parts(p.clone()).unwrap();
    let incoming = CompactDfg::from_parts(label(20)).unwrap();
    c.merge(incoming);
    assert_eq!(c.arena.rows().len(), 4);
    let live: BTreeSet<_> = c.labels.values().flat_map(|l| [l.from, l.to]).collect();
    assert_eq!(live.len(), 2);
    assert_eq!(
        c.arena.rows().len() - live.len(),
        2,
        "losing label endpoints remain until update"
    );
    let mut decoded: CompactDfg = bincode::deserialize(&bincode::serialize(&c).unwrap()).unwrap();
    assert_eq!(decoded.arena.rows().len(), 4);
    parts::assert_inputs(
        &p,
        &DataFlowGraph {
            storage: decoded.clone(),
        },
    );
    parts::compact_roundtrip(&decoded);
    decoded.remove_files(&BTreeSet::new());
    assert_eq!(decoded.arena.rows().len(), 2);
    parts::assert_inputs(
        &p,
        &DataFlowGraph {
            storage: decoded.clone(),
        },
    );
    parts::compact_roundtrip(&decoded);
}

#[test]
fn rank_representative_may_use_another_exact_occurrence_in_its_group() {
    let c = CompactDfg::from_parts(adversarial_parts()).unwrap();
    let expected = storage(&c);
    let mut w = WireDfg::from(c);
    let (i, g) = w
        .legacy_by_occ
        .iter()
        .enumerate()
        .find(|(i, g)| w.legacy_reps[g.0 as usize].0 as usize != *i)
        .unwrap();
    w.legacy_reps[g.0 as usize] = OccId(i as u32);
    let decoded = CompactDfg::try_from(w).unwrap();
    assert_eq!(expected, storage(&decoded));
}
