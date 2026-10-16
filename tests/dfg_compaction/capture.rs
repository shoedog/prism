//! See README.md for the clean-main overlay and rebind procedure.
use super::common::*;
use prism::cpg::dfg_compaction_support::{fieldwise_payload, lookup_indexes, payload};
use sha2::{Digest, Sha256};

#[test]
#[ignore = "writes manifests; run explicitly on clean main with the read-only adapter"]
fn capture_manifests() {
    prism::build_pool::install(|| {
        let out = std::path::PathBuf::from(std::env::var("DFG_PIN_DIR").expect("DFG_PIN_DIR"));
        std::fs::create_dir_all(&out).unwrap();
        let mut payloads = BTreeMap::new();
        let mut indexes = BTreeMap::new();
        super::cases::visit(|name, files| {
            let graph = CodePropertyGraph::build(&files);
            let bytes = fieldwise_payload(&graph);
            assert_eq!(bytes, payload(&graph), "legacy codec control {name}");
            assert!(payloads
                .insert(name.clone(), format!("{:x}", Sha256::digest(bytes)))
                .is_none());
            indexes.insert(
                name,
                format!("{:x}", Sha256::digest(lookup_indexes(&graph))),
            );
        });
        assert_eq!(payloads.len(), 131);
        for (file, pins) in [("pins.sha256", payloads), ("indexes.sha256", indexes)] {
            let text: String = pins
                .into_iter()
                .map(|(name, hash)| format!("{hash}  {name}\n"))
                .collect();
            std::fs::write(out.join(file), text).unwrap();
        }
        eprintln!("CAPTURE 131 payloads + 131 ordered lookup indexes");
    });
}
