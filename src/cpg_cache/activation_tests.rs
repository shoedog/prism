//! Prefix and native-codec activation controls, exercised on both actual reader paths.
use super::tests::{fixture, owned_encoding};
use super::*;

#[test]
pub(in crate::cpg_cache) fn incompatible_prefixes_never_enter_payload_decoder_on_either_path() {
    let (graph, hashes) = fixture();
    let current = owned_encoding(&graph, &hashes);
    let mut cases: Vec<Vec<u8>> = (0..4).map(|n| current[..n].to_vec()).collect();
    for version in [
        super::super::CACHE_VERSION - 1,
        super::super::CACHE_VERSION + 1,
    ] {
        let mut malformed = bincode::serialize(&version).unwrap();
        malformed.extend([255; 16]);
        cases.push(malformed);
    }
    for mapped in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        if mapped {
            File::create(dir.path().join(LOCK_NAME)).unwrap();
        }
        let path = dir.path().join(CACHE_FILE_NAME);
        for bytes in &cases {
            fs::write(&path, bytes).unwrap();
            PAYLOAD_DECODES.with(|c| c.set(0));
            assert!(read_cache_file(&path).is_none());
            assert_eq!(PAYLOAD_DECODES.with(|c| c.get()), 0);
        }
        fs::write(&path, &current).unwrap();
        PAYLOAD_DECODES.with(|c| c.set(0));
        assert!(read_cache_file(&path).is_some());
        assert_eq!(PAYLOAD_DECODES.with(|c| c.get()), 1);
        // A current-version malformed payload attempts exactly one native decode and misses.
        fs::write(&path, &current[..8]).unwrap();
        PAYLOAD_DECODES.with(|c| c.set(0));
        assert!(read_cache_file(&path).is_none());
        assert_eq!(PAYLOAD_DECODES.with(|c| c.get()), 1);
    }
}

#[test]
fn borrowed_and_owned_encoders_pin_the_fixed_version_prefix() {
    let (graph, hashes) = fixture();
    let owned = owned_encoding(&graph, &hashes);
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        super::super::save_cache(&graph, &hashes, false, dir.path()).unwrap(),
        super::super::SaveOutcome::Saved
    );
    let borrowed = fs::read(dir.path().join(CACHE_FILE_NAME)).unwrap();
    assert_eq!(&owned[..4], &super::super::CACHE_VERSION.to_le_bytes());
    assert_eq!(&borrowed[..4], &owned[..4]);
    assert_eq!(borrowed, owned);
}

#[test]
fn corrupt_current_occurrence_id_is_a_miss_on_both_reader_paths() {
    let (graph, hashes) = fixture();
    let encoded = owned_encoding(&graph, &hashes);
    let dfg = bincode::serialize(&graph.dfg).unwrap();
    let damaged = crate::data_flow::test_support::corrupt_production_dfg(&graph.dfg);
    assert_eq!(dfg.len(), damaged.len());
    let offsets: Vec<_> = encoded
        .windows(dfg.len())
        .enumerate()
        .filter_map(|(n, b)| (b == dfg).then_some(n))
        .collect();
    assert_eq!(offsets.len(), 1);
    let mut corrupt = encoded.clone();
    corrupt[offsets[0]..offsets[0] + dfg.len()].copy_from_slice(&damaged);
    for mapped in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        if mapped {
            File::create(dir.path().join(LOCK_NAME)).unwrap();
        }
        let path = dir.path().join(CACHE_FILE_NAME);
        fs::write(&path, &corrupt).unwrap();
        PAYLOAD_DECODES.with(|c| c.set(0));
        assert!(read_cache_file(&path).is_none());
        assert_eq!(PAYLOAD_DECODES.with(|c| c.get()), 1);
        fs::write(&path, &encoded).unwrap();
        assert!(read_cache_file(&path).is_some());
    }
}

#[test]
fn stale_prefix_diagnostic_child() {
    let Ok(value) = std::env::var("DFG_TEST_PREFIX_VERSION") else {
        return;
    };
    let version: u32 = value.parse().unwrap();
    assert!(decode_current_cache(&bincode::serialize(&version).unwrap()).is_none());
}

#[test]
fn old_and_future_prefixes_emit_the_existing_diagnostic_exactly_once() {
    for version in [
        super::super::CACHE_VERSION - 1,
        super::super::CACHE_VERSION + 1,
    ] {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cpg_cache::stream::activation_tests::stale_prefix_diagnostic_child",
                "--nocapture",
            ])
            .env("DFG_TEST_PREFIX_VERSION", version.to_string())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8(out.stderr).unwrap(),
            format!(
                "Cache version mismatch (cached: {}, current: {}), rebuilding\n",
                version,
                super::super::CACHE_VERSION
            )
        );
        assert!(String::from_utf8(out.stdout).unwrap().contains("1 passed"));
    }
}
