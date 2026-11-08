//! Moved from `tests/integration/js_module_owner_test.rs` (#701 S5): it runs the `prism` binary.

#[test]
fn navigation_sidecar_cache_parity() {
    for threads in ["1", "4"] {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let file = root.path().join("a.js");
        std::fs::write(&file, "function f() {}\nf();").unwrap();
        let query = |cached| {
            let mut command = crate::prism_bin().unwrap();
            command
                .env("RAYON_NUM_THREADS", threads)
                .env("PRISM_NAV_EDGE_CACHE_LOAD_DIRTY", "1")
                .arg("nav");
            if cached {
                command.arg("--cache-dir").arg(cache.path());
            } else {
                command.arg("--no-cache");
            }
            command
                .args(["callers", "--symbol", "f", "--format", "json", "--repo"])
                .arg(root.path());
            command.assert().success().get_output().clone()
        };
        let cold = query(true);
        let warm = query(true);
        assert_eq!(cold.stdout, warm.stdout);
        assert!(String::from_utf8_lossy(&cold.stderr).contains("CPG cache miss"));
        assert!(!String::from_utf8_lossy(&warm.stderr).contains("rebuilding"));
        std::fs::write(&file, "function f() {}\nf(); f();").unwrap();
        let partial = query(true);
        assert!(String::from_utf8_lossy(&partial.stderr).contains("partial CPG cache hit"));
        assert_ne!(cold.stdout, partial.stdout);
        assert_eq!(partial.stdout, query(false).stdout);
    }
}
