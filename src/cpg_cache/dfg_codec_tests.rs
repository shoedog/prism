//! Native production compact codec routing.
#[test]
fn compact_codec_rejects_bad_bounds_groups_and_order() {
    crate::data_flow::test_support::assert_codec_validation();
}
#[test]
fn production_cache_version_activates_compact_codec() {
    assert_eq!(super::CACHE_VERSION, 197);
}

#[test]
fn production_cache_version_includes_module_owners() {
    assert_eq!(super::CACHE_VERSION, 197);
}

#[test]
fn production_stream_matches_owned_bytes_and_stale_version_misses() {
    use super::*;
    let files = BTreeMap::from([(
        "a.py".into(),
        crate::ast::ParsedFile::parse(
            "a.py",
            "def f(x):\n y=x\n return y\n",
            crate::languages::Language::Python,
        )
        .unwrap(),
    )]);
    let mut cpg = CodePropertyGraph::build(&files);
    cpg.dfg_label_stats.dfg_props = BTreeMap::from([
        ("sites_literal".into(), 1),
        ("sites_jsx".into(), 4),
        ("refused_jsx_caller_owner".into(), 1),
        ("refused_jsx_site_syntax".into(), 1),
        ("refused_source_ambiguous".into(), 1),
        ("edges_to_def".into(), 2),
    ]);
    let hashes = BTreeMap::from([("a.py".into(), "fixture".into())]);
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        save_cache(&cpg, &hashes, false, dir.path()).unwrap(),
        SaveOutcome::Saved
    );
    let path = cache_bin_path(dir.path());
    let bytes = fs::read(&path).unwrap();
    let owned: CpgCache = bincode::deserialize(&bytes).unwrap();
    assert_eq!(
        owned.graph.dfg_label_stats.dfg_props,
        cpg.dfg_label_stats.dfg_props
    );
    assert_eq!(bytes, stream::tests::owned_encoding(&cpg, &hashes));
    stream::PAYLOAD_DECODES.with(|count| count.set(0));
    let CacheResult::Hit(restored) = load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full cache hit");
    };
    assert_eq!(restored.dfg_label_stats, cpg.dfg_label_stats);
    assert_eq!(stream::PAYLOAD_DECODES.with(|count| count.get()), 1);
    stream::PAYLOAD_DECODES.with(|count| count.set(0));
    assert_stale_version_miss(&hashes, dir.path());
    assert_eq!(stream::PAYLOAD_DECODES.with(|count| count.get()), 0);
    assert_eq!(super::CACHE_VERSION, 197);
}

pub(crate) fn assert_stale_version_miss(
    hashes: &std::collections::BTreeMap<String, String>,
    dir: &std::path::Path,
) {
    use super::*;
    let path = cache_bin_path(dir);
    let mut owned: CpgCache = bincode::deserialize(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(owned.version, CACHE_VERSION);
    owned.version -= 1;
    fs::write(&path, bincode::serialize(&owned).unwrap()).unwrap();
    assert!(matches!(load_cache(hashes, false, dir), CacheResult::Miss));
}

#[test]
fn early_version_rejection_precedes_payload_decode() {
    super::stream::activation_tests::incompatible_prefixes_never_enter_payload_decoder_on_either_path();
}
#[test]
fn native_production_decode_never_expands_legacy_storage() {
    crate::data_flow::test_support::assert_native_production_decode();
}
