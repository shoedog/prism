use super::super::test_support::*;
use super::super::*;

#[test]
pub(in crate::data_flow) fn production_codec_shares_one_location_per_exact_identity() {
    for source in ["", "def f(x):\n y=x+x\n return y\n"] {
        let files = BTreeMap::from([(
            "a.py".into(),
            ParsedFile::parse("a.py", source, crate::languages::Language::Python).unwrap(),
        )]);
        let g = DataFlowGraph::build(&files);
        let encoded = bincode::serialize(&g).unwrap();
        // Behavioral activation seam: production bytes must be the native compact wire.
        super::super::compact::FROM_PARTS_CALLS.with(|n| n.set(0));
        let copy: super::super::compact::CompactDfg =
            bincode::deserialize(&encoded).expect("native compact production codec");
        assert_eq!(current_storage(&g), test_support::compact::storage(&copy));
        let exact: BTreeSet<_> = copy
            .arena
            .rows()
            .iter()
            .map(ExactVarLocation::from_legacy)
            .collect();
        assert_eq!(exact.len(), copy.arena.rows().len());
        let roundtrip: DataFlowGraph = bincode::deserialize(&encoded).unwrap();
        assert_eq!(current_storage(&g), current_storage(&roundtrip));
        assert_eq!(
            super::super::compact::FROM_PARTS_CALLS.with(|n| n.get()),
            0,
            "decode must not expand and reconvert"
        );
    }
}

#[test]
fn production_build_is_compact_before_parallel_collection() {
    use crate::data_flow::build::file::FILE_RESULTS;
    let files: BTreeMap<_, _> = ["__activate_a.py", "__activate_b.py"]
        .into_iter()
        .map(|p| {
            (
                p.to_string(),
                ParsedFile::parse(
                    p,
                    "def f(x):\n y=x+x\n return y\n",
                    crate::languages::Language::Python,
                )
                .unwrap(),
            )
        })
        .collect();
    *FILE_RESULTS.lock().unwrap() = Some(
        files
            .keys()
            .map(|f| (f.clone(), (0, 0, usize::MAX, vec![])))
            .collect(),
    );
    let g = DataFlowGraph::build(&files);
    let records = FILE_RESULTS.lock().unwrap().take().unwrap();
    assert_eq!(records.len(), files.len());
    for (file, parsed) in &files {
        let (rows, edges, expanded, observation) = &records[file];
        assert!(*rows > 0);
        assert!(*edges > 0);
        assert_eq!(*expanded, 0);
        let independent =
            oracle::LegacyDataFlowGraph::build(&BTreeMap::from([(file.clone(), parsed.clone())]));
        assert_eq!(*observation, old_storage(&independent));
    }
    assert_pair(&oracle::LegacyDataFlowGraph::build(&files), &g, &files);
    let empty = DataFlowGraph::build_subset(&files, &BTreeSet::new());
    assert_eq!(empty.edges().len(), 0);
    assert_eq!(empty.storage.arena.rows().len(), 0);
}

#[test]
fn production_has_only_compact_storage_and_native_decode() {
    // This is an architecture guard in the default/MCP suite as well as the feature gate.
    assert_eq!(
        std::mem::size_of::<DataFlowGraph>(),
        std::mem::size_of::<super::super::compact::CompactDfg>()
    );
    let facade = include_str!("../../data_flow.rs");
    assert!(facade.contains("mod compact;"));
    assert!(!facade.contains("mod legacy_storage;"));
    assert!(!facade.contains("#[cfg(any(test, feature = \"dfg-oracle\"))]\nmod compact;"));
    let storage = include_str!("../storage.rs");
    assert!(storage.contains("storage: compact::CompactDfg"));
    assert!(!storage.contains("LegacyDataFlowGraph"));
    assert!(!storage.contains("Vec<FlowEdge>"));
    let build = include_str!("../build/mod.rs");
    assert!(build.contains("Vec<super::compact::CompactDfg>"));
    assert!(!build.contains("Vec<FileRefs>"));
    assert!(!include_str!("../../cpg_cache/stream.rs").contains("LegacyDataFlowGraph"));
}
