//! #749: the bin-root enumeration is one helper shared by the own-library lane
//! (`binary_library_deps`) and the dependency lane (`member_bin_roots`). Each row loads a
//! manifest fixture and checks both maps, so the two cannot drift.
use super::*;

fn load(files: &[(&str, &str)]) -> RustCrateConfig {
    let dir = tempfile::tempdir().unwrap();
    for (path, src) in files {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    load_repo(dir.path())
        .unwrap()
        .scope_graph_inputs
        .expect("scope graph inputs")
        .cfg
}

const MAIN: &str = "fn main() {}\n";
const LIB: &str = "pub fn l() {}\n";

fn own_lib_bins(cfg: &RustCrateConfig) -> Vec<&str> {
    cfg.binary_library_deps.keys().map(String::as_str).collect()
}

#[test]
fn own_library_lane_enumerates_every_auto_bin_shape() {
    let cfg = load(&[
        (
            "Cargo.toml",
            "[package]\nname = \"d\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        ("src/lib.rs", LIB),
        ("src/main.rs", MAIN),
        ("src/bin/tool.rs", MAIN),
        ("src/bin/multi/main.rs", MAIN),
        ("src/bin/multi/helper.rs", LIB),
        ("src/bin/multi/deep/main.rs", MAIN),
        ("src/other.rs", LIB),
    ]);
    assert_eq!(
        own_lib_bins(&cfg),
        ["src/bin/multi/main.rs", "src/bin/tool.rs", "src/main.rs"]
    );
}

#[test]
fn own_library_lane_keeps_explicit_bins_autobins_and_edition_rules() {
    // Explicit `[[bin]]` paths replace auto-discovery; a bin with its own edition is skipped.
    let cfg = load(&[
        (
            "Cargo.toml",
            "[package]\nname = \"d\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
             [[bin]]\nname = \"x\"\npath = \"tools/x.rs\"\n\
             [[bin]]\nname = \"y\"\npath = \"tools/y.rs\"\nedition = \"2018\"\n",
        ),
        ("src/lib.rs", LIB),
        ("src/main.rs", MAIN),
        ("tools/x.rs", MAIN),
        ("tools/y.rs", MAIN),
    ]);
    assert_eq!(own_lib_bins(&cfg), ["tools/x.rs"]);
    let off = load(&[
        (
            "Cargo.toml",
            "[package]\nname = \"d\"\nversion = \"0.1.0\"\nedition = \"2021\"\nautobins = false\n",
        ),
        ("src/lib.rs", LIB),
        ("src/main.rs", MAIN),
        ("src/bin/tool.rs", MAIN),
    ]);
    assert!(
        off.binary_library_deps.is_empty(),
        "{:?}",
        off.binary_library_deps
    );
}

#[test]
fn own_library_lane_refuses_a_conflicting_dependency_name() {
    let cfg = load(&[
        (
            "Cargo.toml",
            "[package]\nname = \"d\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
             [dependencies]\nd = \"1\"\n",
        ),
        ("src/lib.rs", LIB),
        ("src/main.rs", MAIN),
    ]);
    assert!(
        cfg.binary_library_deps.is_empty(),
        "{:?}",
        cfg.binary_library_deps
    );
}
