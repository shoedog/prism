//! Review regressions built from raw bytes, before map/set collection.
use super::super::compact::{wire::WireDfg, *};
use super::super::*;
use super::identity::{arena, loc};

type Stats = (
    usize,
    usize,
    usize,
    Vec<(&'static str, usize)>,
    Vec<(&'static str, usize)>,
    Vec<(&'static str, usize, &'static str, usize)>,
);
fn stats(keys: Vec<(&'static str, usize)>) -> Stats {
    (0, keys.len(), 0, vec![], keys, vec![])
}
fn raw_rd(rows: Vec<(&str, Stats)>) -> Vec<u8> {
    // Ten empty sequence fields followed by RD filename/stat rows. Encoding the
    // raw sequences preserves duplicates and order through the serde boundary.
    let mut bytes = vec![0; 10 * 8];
    bytes.extend(bincode::serialize(&rows).unwrap());
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
            s.1 = s.4.iter().collect::<BTreeSet<_>>().len();
            if $over {
                s = (s.1, 0, 0, s.4, vec![], vec![]);
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
fn rd_no_body_entry_duplicate_keys_raw() {
    let mut s = stats(vec![]);
    s.2 = 1;
    s.5 = vec![("f", 1, "body", 2), ("f", 1, "body", 2)];
    reject_raw(vec![("a.py", s)], "DFG RD set order/duplicate");
}
#[test]
fn rd_no_body_entry_reversed_keys_raw() {
    let mut s = stats(vec![]);
    s.2 = 2;
    s.5 = vec![("g", 1, "body", 2), ("f", 1, "body", 2)];
    reject_raw(vec![("a.py", s)], "DFG RD set order/duplicate");
}
#[test]
fn rd_unique_keys_raw_roundtrip_bytes() {
    let rows = vec![
        (
            "a.py",
            (
                2,
                2,
                1,
                vec![("f", 1), ("f", 2)],
                vec![("f", 1), ("g", 1)],
                vec![("f", 1, "body", 2)],
            ),
        ),
        ("b.py", stats(vec![])),
    ];
    let bytes = raw_rd(rows);
    let decoded: CompactDfg = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded.rd_function_stats()["a.py"].functions_without_cfg, 2);
    assert_eq!(decoded.rd_function_stats()["a.py"].no_body_entry, 1);
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
