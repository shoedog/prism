use super::super::compact::*;
use super::super::*;
pub(super) fn loc(name: &str, kind: VarAccessKind, start: usize) -> VarLocation {
    VarLocation {
        file: "u.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 2,
        path: AccessPath {
            base: name.into(),
            fields: vec![],
        },
        start_byte: start,
        end_byte: start + 1,
        kind,
    }
}
pub(super) fn arena(mut rows: Vec<VarLocation>) -> Arena {
    rows.sort_by(exact_cmp);
    rows.dedup_by(|b, a| exact_cmp(a, b).is_eq());
    Arena::from_sorted(rows).unwrap()
}
#[test]
fn legacy_identity_excludes_bytes_exact_identity_does_not() {
    let a = loc("x", VarAccessKind::Use, 10);
    let b = loc("x", VarAccessKind::Use, 30);
    let g = arena(vec![a.clone(), b.clone()]);
    assert_eq!(a, b);
    assert_eq!(g.legacy(&a), g.legacy(&b));
    assert_ne!(g.exact(&a), g.exact(&b));
    let mut p = test_support::compact::parts(oracle::LegacyDataFlowGraph::empty());
    p.uses.insert(
        (a.file.clone(), a.function.clone(), 1, a.path.clone()),
        vec![a, b],
    );
    let frozen = CompactDfg::from_parts(p).unwrap();
    assert_eq!(
        frozen.arena.rows.len(),
        2,
        "consuming interner must also distinguish bytes"
    );
}
#[test]
fn kind_precedes_bytes_only_in_legacy_order() {
    let d = loc("x", VarAccessKind::Def, 30);
    let u = loc("x", VarAccessKind::Use, 10);
    let g = arena(vec![d.clone(), u.clone()]);
    assert!(g.exact(&u) < g.exact(&d));
    assert!(g.legacy(&d) < g.legacy(&u));
}
#[test]
fn unknown_exact_query_can_still_find_legacy_key() {
    let a = loc("x", VarAccessKind::Use, 10);
    let b = loc("x", VarAccessKind::Use, 900);
    let g = arena(vec![a]);
    assert!(g.exact(&b).is_none());
    assert!(g.legacy(&b).is_some());
}
#[test]
fn duplicate_or_unsorted_cache_arena_is_rejected() {
    let a = loc("x", VarAccessKind::Use, 10);
    let b = loc("x", VarAccessKind::Use, 20);
    assert!(Arena::from_sorted(vec![a.clone(), a.clone()]).is_err());
    assert!(Arena::from_sorted(vec![b, a]).is_err());
}
#[test]
fn empty_arena_has_no_matches() {
    let g = Arena::from_sorted(vec![]).unwrap();
    assert!(g.legacy(&loc("x", VarAccessKind::Def, 1)).is_none());
}
#[test]
fn id_boundary_is_checked_without_allocating_billions() {
    assert_eq!(checked_id(u32::MAX as usize), Ok(u32::MAX));
    assert!(checked_id(u32::MAX as usize + 1).is_err());
}
