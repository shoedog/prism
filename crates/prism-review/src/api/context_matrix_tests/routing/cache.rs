//! G18–G20: native cache lookup plus same-input source-bound build-path inference.
use super::*;
use prism::cpg_cache::{self, CacheResult};
use std::path::Path;

fn assert_native_hit(f: &Fixture, cache: &Path) {
    let hashes = cpg_cache::compute_file_hashes(&f.inputs.sources);
    let mut key =
        cpg_cache::compute_topology_key(&hashes, &f.inputs.scope_graph_inputs.manifest_hashes);
    if let Some(db) = f.inputs.type_db.as_ref() {
        key.insert("type_db:fingerprint".into(), db.cache_fingerprint());
    }
    assert!(
        matches!(
            cpg_cache::load_cache_with_topology(&hashes, &key, f.inputs.type_db.is_some(), cache),
            CacheResult::Hit(_)
        ),
        "inadmissible precondition: native cache lookup Hit"
    );
}
#[test]
fn g18_context_edit_after_native_cache_hit_is_fresh() {
    let mut f = Fixture::js(B);
    let cache = tempfile::tempdir().unwrap();
    f.options.no_cache = false;
    f.options.cache_dir = Some(cache.path().into());
    f.options.scoped_cpg = false;
    let _ = f.off(&taint());
    assert_native_hit(&f, cache.path());
    let first = f.on(&taint());
    assert_expanded(&first, "b.js");
    assert_eq!(context(&first).sources["b.js"], B);
    let changed = "export function g(x) {\n  eval(x);\n}\n";
    std::fs::write(f.dir.path().join("b.js"), changed).unwrap();
    assert_native_hit(&f, cache.path());
    let second = f.on(&taint());
    assert_expanded(&second, "b.js");
    assert_eq!(context(&second).sources["b.js"], changed);
    assert!(second.run.results[0]
        .findings
        .iter()
        .any(|v| v.file == "b.js" && v.line == 2));
    assert_ne!(bytes(&first.run.results[0]), bytes(&second.run.results[0]));
    let first_doc = doc(&f, &first);
    let second_doc = doc(&f, &second);
    let context_text = |document: &Documents| {
        document.json.value["results"][0]["slices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slice| slice["file"] == "b.js")
            .unwrap()["slice_text"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_ne!(context_text(&first_doc), context_text(&second_doc));
    assert!(context_text(&second_doc).contains("eval(x)"));
}
#[test]
fn g19_scoped_round_zero_does_not_scope_context() {
    let mut f = Fixture::js(B);
    let plain = f.on(&taint());
    assert_expanded(&plain, "b.js");
    f.options.scoped_cpg = true;
    let scoped = f.on(&taint());
    let off = f.off(&[SlicingAlgorithm::EchoSlice]);
    let mix = f.on(&[SlicingAlgorithm::Taint, SlicingAlgorithm::EchoSlice]);
    assert_expanded(&scoped, "b.js");
    assert_expanded(&mix, "b.js");
    same_result(&plain.run.results[0], &scoped.run.results[0]);
    same_result(&off.run.results[0], &mix.run.results[1]);
    assert_eq!(doc(&f, &plain).bytes(), doc(&f, &scoped).bytes());
}
#[test]
fn g20_no_cache_and_warm_cache_documents_and_evidence_agree() {
    let mut f = Fixture::js(B);
    let cold = f.on(&taint());
    assert_expanded(&cold, "b.js");
    let cache = tempfile::tempdir().unwrap();
    f.options.no_cache = false;
    f.options.cache_dir = Some(cache.path().into());
    f.options.scoped_cpg = false;
    let _ = f.off(&taint());
    assert_native_hit(&f, cache.path());
    let warm = f.on(&taint());
    assert_expanded(&warm, "b.js");
    same_result(&cold.run.results[0], &warm.run.results[0]);
    assert_eq!(doc(&f, &cold).bytes(), doc(&f, &warm).bytes());
}
