use prism::cpg::CpgContext;
use prism::repo_loader::{load_repo, LoadedRepo};

fn one_thread_pool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
}

/// Corpus: a multi-file subset of the repo (src/navigation + src/cpg, ~15 Rust
/// files with real cross-file calls and data flow) instead of the whole repo —
/// 4 debug-mode whole-repo CPG builds made the gate take ~20 minutes, the exact
/// slow-gate failure mode the plan review warned about (spec plan note: "if
/// loading the whole prism repo makes the test slow, restrict to a subdir").
/// Plenty of files to surface any parallel-scheduling order divergence.
fn corpus() -> LoadedRepo {
    load_repo(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/navigation")).unwrap()
}

fn corpus2() -> LoadedRepo {
    load_repo(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg")).unwrap()
}

#[test]
fn cpg_build_parallel_matches_serial_reference_in_order() {
    let repo = corpus();
    let parallel = CpgContext::build(&repo.files, None);
    let serial = one_thread_pool().install(|| CpgContext::build(&repo.files, None));
    let dump = |c: &CpgContext<'_>| {
        let nodes: Vec<String> = c
            .cpg
            .node_indices()
            .map(|i| format!("{:?}", c.cpg.node(i)))
            .collect();
        let edges: Vec<String> = c.cpg.edge_dump();
        (nodes, edges)
    };
    assert_eq!(dump(&parallel), dump(&serial));
}

#[test]
fn cache_blob_bytes_identical_serial_vs_parallel() {
    let repo = corpus2();
    let d_par = tempfile::tempdir().unwrap();
    let d_ser = tempfile::tempdir().unwrap();
    prism::navigation::NavigationIndex::build_cached_under(&repo, d_par.path());
    one_thread_pool()
        .install(|| prism::navigation::NavigationIndex::build_cached_under(&repo, d_ser.path()));

    fn find_bin(d: &std::path::Path) -> std::path::PathBuf {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                return find_bin(&p);
            }
            if p.file_name().and_then(|n| n.to_str()) == Some("cpg-cache.bin") {
                return p;
            }
        }
        panic!("cpg-cache.bin not found under {}", d.display());
    }

    let b_par = std::fs::read(find_bin(d_par.path())).unwrap();
    let b_ser = std::fs::read(find_bin(d_ser.path())).unwrap();
    assert_eq!(b_par, b_ser);
}
