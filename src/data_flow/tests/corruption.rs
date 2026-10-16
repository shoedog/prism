use super::super::compact::{wire::WireDfg, *};
use super::super::*;
use super::representatives::adversarial_parts;
fn valid() -> WireDfg {
    WireDfg::from(CompactDfg::from_parts(adversarial_parts()).unwrap())
}
fn rejected(w: WireDfg) {
    // Exercise both the typed validator and the actual serde entry point.
    let bytes = bincode::serialize(&w).unwrap();
    assert!(
        CompactDfg::try_from(w).is_err(),
        "typed validation accepted corruption"
    );
    assert!(
        bincode::deserialize::<CompactDfg>(&bytes).is_err(),
        "decode accepted corruption"
    );
}
macro_rules! corrupt {
    ($name:ident, $w:ident, $mutation:block) => { #[test] fn $name() { let mut $w = valid(); $mutation rejected($w); } };
}
corrupt!(duplicate_arena_row, w, {
    let i = w.arena.windows(2).position(|p| p[0] == p[1]).unwrap();
    w.arena[i + 1] = w.arena[i].clone();
});
corrupt!(reversed_arena_rows, w, {
    let i = w.arena.windows(2).position(|p| p[0] == p[1]).unwrap();
    w.arena.swap(i, i + 1);
});
corrupt!(rank_length, w, {
    w.legacy_by_occ.pop();
});
corrupt!(rank_representative_bounds, w, {
    w.legacy_reps[0] = OccId(u32::MAX);
});
corrupt!(duplicate_legacy_reps, w, {
    w.legacy_reps[1] = w.legacy_reps[0];
});
corrupt!(reversed_legacy_reps, w, {
    w.legacy_reps.swap(0, 1);
});
corrupt!(rank_group_bounds, w, {
    w.legacy_by_occ[0] = LegacyId(u32::MAX);
});
corrupt!(rank_wrong_group, w, {
    let (i, g) = w
        .legacy_by_occ
        .iter()
        .copied()
        .enumerate()
        .find(|(i, g)| w.legacy_reps[g.0 as usize].0 as usize != *i)
        .unwrap();
    w.legacy_by_occ[i] = LegacyId((g.0 + 1) % w.legacy_reps.len() as u32);
});
corrupt!(edge_from_bounds, w, {
    w.edges[0].0 = OccId(u32::MAX);
});
corrupt!(edge_to_bounds, w, {
    w.edges[0].1 = OccId(u32::MAX);
});
corrupt!(defs_member_bounds, w, {
    w.defs[0].1[0] = OccId(u32::MAX);
});
corrupt!(uses_member_bounds, w, {
    w.uses[0].1[0] = OccId(u32::MAX);
});
corrupt!(exact_from_bounds, w, {
    w.exact_labels[0].0 .0 = OccId(u32::MAX);
});
corrupt!(exact_to_bounds, w, {
    w.exact_labels[0].0 .1 = OccId(u32::MAX);
});
corrupt!(label_from_bounds, w, {
    w.labels[0].1.from = OccId(u32::MAX);
});
corrupt!(label_to_bounds, w, {
    w.labels[0].1.to = OccId(u32::MAX);
});
corrupt!(label_wrong_from_group, w, {
    w.labels.truncate(1);
    w.labels[0].0 .0 = LegacyId((w.labels[0].0 .0 .0 + 1) % w.legacy_reps.len() as u32);
});
corrupt!(label_wrong_to_group, w, {
    w.labels.truncate(1);
    w.labels[0].0 .1 = LegacyId((w.labels[0].0 .1 .0 + 1) % w.legacy_reps.len() as u32);
});
corrupt!(label_group_bounds, w, {
    w.labels[0].0 .0 = LegacyId(u32::MAX);
});
corrupt!(rd_over_cap_count, w, {
    w.rd_function_stats
        .get_mut("a.py")
        .unwrap()
        .functions_over_cap = 5;
});
corrupt!(rd_without_cfg_count, w, {
    w.rd_function_stats
        .get_mut("a.py")
        .unwrap()
        .functions_without_cfg = 5;
});
corrupt!(forward_representative_bounds, w, {
    w.forward[0].1.representative = OccId(u32::MAX);
});
corrupt!(forward_neighbor_bounds, w, {
    w.forward[0].1.neighbors.push(OccId(u32::MAX));
});
corrupt!(forward_group_bounds, w, {
    w.forward.truncate(1);
    w.forward[0].0 = LegacyId(u32::MAX);
});
corrupt!(forward_in_range_wrong_group, w, {
    w.forward.truncate(1);
    w.forward[0].0 = LegacyId((w.forward[0].0 .0 + 1) % w.legacy_reps.len() as u32);
});
corrupt!(backward_representative_bounds, w, {
    w.backward[0].1.representative = OccId(u32::MAX);
});
corrupt!(backward_neighbor_bounds, w, {
    w.backward[0].1.neighbors.push(OccId(u32::MAX));
});
corrupt!(backward_group_bounds, w, {
    w.backward.truncate(1);
    w.backward[0].0 = LegacyId(u32::MAX);
});
corrupt!(backward_in_range_wrong_group, w, {
    w.backward.truncate(1);
    w.backward[0].0 = LegacyId((w.backward[0].0 .0 + 1) % w.legacy_reps.len() as u32);
});
corrupt!(defs_duplicate_keys, w, {
    w.defs.insert(0, w.defs[0].clone());
});
corrupt!(defs_reversed_keys, w, {
    w.defs.swap(0, 1);
});
corrupt!(uses_duplicate_keys, w, {
    w.uses.insert(0, w.uses[0].clone());
});
corrupt!(uses_reversed_keys, w, {
    let mut key = w.uses[0].0.clone();
    key.0.push_str("zzz");
    w.uses.push((key, w.uses[0].1.clone()));
    w.uses.reverse();
});
corrupt!(forward_duplicate_keys, w, {
    w.forward.insert(0, w.forward[0].clone());
});
corrupt!(forward_reversed_keys, w, {
    w.forward.swap(0, 1);
});
corrupt!(backward_duplicate_keys, w, {
    w.backward.insert(0, w.backward[0].clone());
});
corrupt!(backward_reversed_keys, w, {
    let key = LegacyId((w.backward[0].0 .0 + 1) % w.legacy_reps.len() as u32);
    let mut row = w.backward[0].1.clone();
    row.representative = w.legacy_reps[key.0 as usize];
    w.backward.push((key, row));
    w.backward.sort_by_key(|r| r.0);
    w.backward.reverse();
});
corrupt!(labels_duplicate_keys, w, {
    w.labels.insert(0, w.labels[0].clone());
});
corrupt!(labels_reversed_keys, w, {
    w.labels.swap(0, 1);
});
corrupt!(exact_labels_duplicate_keys, w, {
    w.exact_labels.insert(0, w.exact_labels[0].clone());
});
corrupt!(exact_labels_reversed_keys, w, {
    let key = (OccId(0), OccId(0));
    assert_ne!(key, w.exact_labels[0].0);
    w.exact_labels.push((key, w.exact_labels[0].1));
    w.exact_labels.sort_by_key(|r| r.0);
    w.exact_labels.reverse();
});

// A test serializer records each top-level field boundary using the exact
// bincode encoding of that field; equality below checks the declaration order.
struct Recorded<'a>(&'a WireDfg, std::cell::RefCell<Vec<usize>>);
impl serde::Serialize for Recorded<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("WireDfg", 11)?;
        let mut offset = 0;
        self.1.borrow_mut().clear();
        macro_rules! field {
            ($name:ident) => {{
                s.serialize_field(stringify!($name), &self.0.$name)?;
                offset += bincode::serialized_size(&self.0.$name).unwrap() as usize;
                self.1.borrow_mut().push(offset);
            }};
        }
        field!(arena);
        field!(legacy_reps);
        field!(legacy_by_occ);
        field!(edges);
        field!(defs);
        field!(uses);
        field!(forward);
        field!(backward);
        field!(labels);
        field!(exact_labels);
        field!(rd_function_stats);
        s.end()
    }
}
#[test]
fn every_recorded_field_boundary_truncation_is_rejected() {
    let w = valid();
    let recording = Recorded(&w, Default::default());
    let bytes = bincode::serialize(&recording).unwrap();
    assert_eq!(bytes, bincode::serialize(&w).unwrap());
    assert_eq!(recording.1.borrow().len(), 11);
    for boundary in recording.1.borrow().iter().copied() {
        // Before the field ends, and between fields (except the complete wire).
        for end in [boundary - 1, boundary]
            .into_iter()
            .filter(|&n| n < bytes.len())
        {
            assert!(
                bincode::deserialize::<CompactDfg>(&bytes[..end]).is_err(),
                "truncated at {end}"
            );
        }
    }
}
#[test]
fn invalid_confidence_discriminants_are_rejected() {
    let mut p = test_support::compact::parts(oracle::LegacyDataFlowGraph::empty());
    let a = super::identity::loc("x", VarAccessKind::Def, 1);
    let b = super::identity::loc("x", VarAccessKind::Use, 2);
    p.exact_labels
        .insert(ExactFlowEdge::from_refs(&a, &b), FlowConfidence::Exact);
    let w = WireDfg::from(CompactDfg::from_parts(p).unwrap());
    let mut bytes = bincode::serialize(&w).unwrap();
    // Last field is the empty RD map (8 bytes); preceding Exact tag is u32.
    let tag = bytes.len() - 12;
    assert_eq!(&bytes[tag..tag + 4], &0u32.to_le_bytes());
    bytes[tag..tag + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(bincode::deserialize::<CompactDfg>(&bytes).is_err());
}

pub(in crate::data_flow) fn codec_validation_matrix() {
    duplicate_arena_row();
    reversed_arena_rows();
    rank_length();
    rank_representative_bounds();
    duplicate_legacy_reps();
    reversed_legacy_reps();
    rank_group_bounds();
    rank_wrong_group();
    edge_from_bounds();
    edge_to_bounds();
    defs_member_bounds();
    uses_member_bounds();
    exact_from_bounds();
    exact_to_bounds();
    label_from_bounds();
    label_to_bounds();
    label_wrong_from_group();
    label_wrong_to_group();
    label_group_bounds();
    rd_over_cap_count();
    rd_without_cfg_count();
    forward_representative_bounds();
    forward_neighbor_bounds();
    forward_group_bounds();
    forward_in_range_wrong_group();
    backward_representative_bounds();
    backward_neighbor_bounds();
    backward_group_bounds();
    backward_in_range_wrong_group();
    defs_duplicate_keys();
    defs_reversed_keys();
    uses_duplicate_keys();
    uses_reversed_keys();
    forward_duplicate_keys();
    forward_reversed_keys();
    backward_duplicate_keys();
    backward_reversed_keys();
    labels_duplicate_keys();
    labels_reversed_keys();
    exact_labels_duplicate_keys();
    exact_labels_reversed_keys();
    every_recorded_field_boundary_truncation_is_rejected();
    invalid_confidence_discriminants_are_rejected();
}
