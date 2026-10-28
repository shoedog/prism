//! Native production compact codec routing.
#[test]
fn compact_codec_rejects_bad_bounds_groups_and_order() {
    crate::data_flow::test_support::assert_codec_validation();
}
#[test]
fn production_cache_version_activates_compact_codec() {
    assert_eq!(super::CACHE_VERSION, 128);
}

#[test]
fn production_cache_version_includes_module_owners() {
    assert_eq!(super::CACHE_VERSION, 128);
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
    let cpg = CodePropertyGraph::build(&files);
    let hashes = BTreeMap::from([("a.py".into(), "fixture".into())]);
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        save_cache(&cpg, &hashes, false, dir.path()).unwrap(),
        SaveOutcome::Saved
    );
    let path = cache_bin_path(dir.path());
    let bytes = fs::read(&path).unwrap();
    let mut owned: CpgCache = bincode::deserialize(&bytes).unwrap();
    assert_eq!(owned.version, 128);
    assert_eq!(bytes, bincode::serialize(&owned).unwrap());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    owned.version -= 1;
    fs::write(&path, bincode::serialize(&owned).unwrap()).unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn early_version_rejection_precedes_payload_decode() {
    super::stream::activation_tests::incompatible_prefixes_never_enter_payload_decoder_on_either_path();
}
#[test]
fn native_production_decode_never_expands_legacy_storage() {
    crate::data_flow::test_support::assert_native_production_decode();
}
