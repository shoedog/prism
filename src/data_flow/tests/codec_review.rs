//! Review regressions built from raw bytes, before map/set collection.
use super::super::compact::{wire::WireDfg, *};
use super::super::*;
use super::identity::{arena, loc};

type Stats = (
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    Vec<(&'static str, usize)>,
    Vec<(&'static str, usize)>,
    Vec<((&'static str, usize, usize), usize)>,
);
fn stats(keys: Vec<(&'static str, usize)>) -> Stats {
    (0, keys.len(), 0, 0, 0, 0, 0, 0, vec![], keys, vec![])
}
fn raw_rd(rows: Vec<(&str, Stats)>) -> Vec<u8> {
    // Ten empty sequence fields followed by RD filename/stat rows. Encoding the
    // raw sequences preserves duplicates and order through the serde boundary.
    let mut bytes = vec![0; 10 * 8];
    bytes.extend(bincode::serialize(&rows.len()).unwrap());
    for (file, stats) in rows {
        bytes.extend(bincode::serialize(&(file, stats)).unwrap());
        // Independent recall owner rows, added in the current cache schema.
        bytes.extend(bincode::serialize(&0usize).unwrap());
        // Receiver-edge ledger (fold 1): empty.
        bytes.extend(bincode::serialize(&0usize).unwrap());
        // Loop-plan counter map (#693 slice 2): empty.
        bytes.extend(bincode::serialize(&0usize).unwrap());
    }
    // Macro census digests (fold 3 Rust, #630 slice 6a C/C++): both empty.
    bytes.extend(bincode::serialize("").unwrap());
    bytes.extend(bincode::serialize("").unwrap());
    bytes
}
fn reject_raw(rows: Vec<(&str, Stats)>, expected: &str) {
    let error = bincode::deserialize::<CompactDfg>(&raw_rd(rows)).unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
}
#[test]
fn rd_duplicate_differing_stats_raw() {
    reject_raw(
        vec![("a.py", stats(vec![("f", 1)])), ("a.py", stats(vec![]))],
        "DFG RD map order/duplicate",
    );
}
#[test]
fn rd_duplicate_hides_malformed_first_raw() {
    let mut malformed = stats(vec![]);
    malformed.1 = 1;
    reject_raw(vec![("a.py", malformed), ("a.py", stats(vec![]))], "DFG RD");
}
#[test]
fn rd_reversed_filenames_raw() {
    reject_raw(
        vec![("b.py", stats(vec![])), ("a.py", stats(vec![]))],
        "DFG RD map order/duplicate",
    );
}
macro_rules! set_case {
    ($name:ident, $over:expr, $keys:expr) => {
        #[test]
        fn $name() {
            let mut s = stats($keys);
            // Match the post-dedup count so the old count guard cannot mask this.
            s.1 = s.9.iter().collect::<BTreeSet<_>>().len();
            if $over {
                s = (s.1, 0, 0, 0, 0, 0, 0, 0, s.9, vec![], vec![]);
            }
            reject_raw(vec![("a.py", s)], "DFG RD set order/duplicate");
        }
    };
}
set_case!(
    rd_without_cfg_duplicate_keys_raw,
    false,
    vec![("f", 1), ("f", 1)]
);
set_case!(
    rd_over_cap_duplicate_keys_raw,
    true,
    vec![("f", 1), ("f", 1)]
);
set_case!(
    rd_without_cfg_reversed_keys_raw,
    false,
    vec![("g", 1), ("f", 1)]
);
set_case!(
    rd_over_cap_reversed_keys_raw,
    true,
    vec![("g", 1), ("f", 1)]
);
#[test]
fn rd_unique_keys_raw_roundtrip_bytes() {
    let rows = vec![
        (
            "a.py",
            (
                2,
                2,
                1,
                23,
                11,
                0,
                17,
                0,
                vec![("f", 1), ("f", 2)],
                vec![("f", 1), ("g", 1)],
                vec![(("f", 1, 0), 23)],
            ),
        ),
        ("b.py", stats(vec![])),
    ];
    let bytes = raw_rd(rows);
    let decoded: CompactDfg = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.rd_function_stats()["a.py"].functions_without_cfg, 2);
    let stats = &decoded.rd_function_stats()["a.py"];
    assert_eq!(stats.fallback_functions_capped, 1);
    assert_eq!(stats.fallback_edges_capped, 23);
    assert_eq!(stats.dfg_pruned_write_only, 11);
    assert_eq!(stats.dfg_pruned_dominated, 17);
    assert_eq!(bytes, bincode::serialize(&decoded).unwrap());
}
#[test]
fn decoded_storage_vecs_have_no_spare_capacity() {
    // Exercise all four top-level Vecs beyond their serde cautious reservation,
    // as well as each kind of retained nested ID Vec.
    let n = 262_145;
    let a = arena((0..n).map(|i| loc("x", VarAccessKind::Use, i)).collect());
    let mut w = WireDfg::from(CompactDfg::empty());
    w.arena = a.rows().clone();
    // One representative per exact row is invalid for this shared identity;
    // use distinct legacy identities to exercise legacy_reps at the threshold.
    for (i, row) in w.arena.iter_mut().enumerate() {
        row.line = i;
    }
    let a = Arena::from_sorted(w.arena).unwrap();
    w.arena = a.rows().clone();
    w.legacy_reps = a.legacy_reps().clone();
    w.legacy_by_occ = a.legacy_by_occ().clone();
    w.edges = vec![(OccId(0), OccId(1)); 131_073];
    let key = ("u.py".into(), "f".into(), 1, w.arena[0].path.clone());
    w.defs.push((key.clone(), vec![OccId(0); n]));
    w.uses.push((key, vec![OccId(1); n]));
    w.forward.push((
        LegacyId(0),
        Adj {
            representative: OccId(0),
            neighbors: vec![OccId(1); n],
        },
    ));
    w.backward.push((
        LegacyId(1),
        Adj {
            representative: OccId(1),
            neighbors: vec![OccId(0); n],
        },
    ));
    let bytes = bincode::serialize(&w).unwrap();
    let decoded: CompactDfg = bincode::deserialize(&bytes).unwrap();
    let capacities = [
        (
            "arena",
            decoded.arena.rows().len(),
            decoded.arena.rows().capacity(),
        ),
        (
            "legacy_reps",
            decoded.arena.legacy_reps().len(),
            decoded.arena.legacy_reps().capacity(),
        ),
        (
            "legacy_by_occ",
            decoded.arena.legacy_by_occ().len(),
            decoded.arena.legacy_by_occ().capacity(),
        ),
        ("edges", decoded.edges.len(), decoded.edges.capacity()),
        (
            "defs",
            decoded.defs.values().next().unwrap().len(),
            decoded.defs.values().next().unwrap().capacity(),
        ),
        (
            "uses",
            decoded.uses.values().next().unwrap().len(),
            decoded.uses.values().next().unwrap().capacity(),
        ),
        (
            "forward",
            decoded.forward.values().next().unwrap().neighbors.len(),
            decoded
                .forward
                .values()
                .next()
                .unwrap()
                .neighbors
                .capacity(),
        ),
        (
            "backward",
            decoded.backward.values().next().unwrap().neighbors.len(),
            decoded
                .backward
                .values()
                .next()
                .unwrap()
                .neighbors
                .capacity(),
        ),
    ];
    assert!(
        capacities.iter().all(|(_, len, cap)| len == cap),
        "{capacities:?}"
    );
}

#[test]
fn decoded_access_path_vecs_have_no_spare_capacity() {
    let mut row = loc("x", VarAccessKind::Def, 1);
    row.path.fields = vec![String::new(); 43_691];
    let a = arena(vec![row.clone()]);
    let mut w = WireDfg::from(CompactDfg::empty());
    w.arena = a.rows().clone();
    w.legacy_reps = a.legacy_reps().clone();
    w.legacy_by_occ = a.legacy_by_occ().clone();
    let key = (row.file, row.function, 1, row.path);
    w.defs.push((key.clone(), vec![OccId(0)]));
    w.uses.push((key, vec![]));
    let decoded: CompactDfg = bincode::deserialize(&bincode::serialize(&w).unwrap()).unwrap();
    let paths = [
        &decoded.arena.rows()[0].path,
        &decoded.defs.keys().next().unwrap().3,
        &decoded.uses.keys().next().unwrap().3,
    ];
    for path in paths {
        assert_eq!(path.fields.capacity(), path.fields.len());
    }
}

#[test]
fn rd_fallback_cap_duplicate_and_reversed_keys_raw() {
    for keys in [
        vec![(("f", 1, 0), 3), (("f", 1, 0), 4)],
        vec![(("g", 1, 0), 3), (("f", 1, 0), 4)],
    ] {
        let mut s = stats(vec![]);
        s.2 = 2;
        s.3 = 7;
        s.10 = keys;
        reject_raw(vec![("a.py", s)], "DFG RD cap map order/duplicate");
    }
}

#[test]
fn rd_fallback_cap_count_corruption_and_overflow_raw() {
    for (functions, edges, keys) in [
        (2, 3, vec![(("f", 1, 0), 3)]),
        (1, 4, vec![(("f", 1, 0), 3)]),
        (2, 0, vec![(("f", 1, 0), usize::MAX), (("g", 1, 0), 1)]),
    ] {
        let mut s = stats(vec![]);
        s.2 = functions;
        s.3 = edges;
        s.10 = keys;
        reject_raw(vec![("a.py", s)], "DFG RD");
    }
}

#[test]
fn go_field_recall_codec_rejects_raw_duplicates_order_and_overflow() {
    for (keys, expected) in [
        (
            vec![
                (("f", 1usize, 0usize), (0u32, 2usize)),
                (("f", 1, 0), (0, 2)),
            ],
            "DFG recall map order/duplicate",
        ),
        (
            vec![(("g", 1, 0), (0, 2)), (("f", 1, 0), (0, 2))],
            "DFG recall map order/duplicate",
        ),
        (
            vec![(("f", 1, 0), (0, usize::MAX)), (("g", 1, 0), (0, 1))],
            "DFG recall count overflow",
        ),
    ] {
        let mut raw = raw_rd(vec![("a.go", stats(vec![]))]);
        // Drop the recall rows, the receiver ledger, the loop-plan map and the two empty census digests (40 bytes),
        // then re-append them.
        raw.truncate(raw.len() - 40);
        raw.extend(bincode::serialize(&keys).unwrap());
        raw.extend(bincode::serialize(&0usize).unwrap());
        raw.extend(bincode::serialize(&0usize).unwrap());
        raw.extend(bincode::serialize("").unwrap());
        raw.extend(bincode::serialize("").unwrap());
        let error = bincode::deserialize::<CompactDfg>(&raw)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}
