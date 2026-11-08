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

const WS: &str = "[workspace]\nmembers = [\"a\", \"d\"]\n";
const A_TOML: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const D_HEAD: &str = "[package]\nname = \"d-pkg\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const D_DEPS: &str = "[dependencies]\na = { path = \"../a\" }\n";

fn member_roots(cfg: &RustCrateConfig) -> (Option<&str>, Vec<&str>) {
    let bins = &cfg.member_bin_roots["d"];
    (
        bins.own_lib_name.as_deref(),
        bins.roots.iter().map(String::as_str).collect(),
    )
}

#[test]
fn dependency_lane_uses_the_same_enumeration_as_the_own_library_lane() {
    let cfg = load(&[
        ("Cargo.toml", WS),
        ("a/Cargo.toml", A_TOML),
        ("a/src/lib.rs", LIB),
        ("d/Cargo.toml", &format!("{D_HEAD}{D_DEPS}")),
        ("d/src/lib.rs", LIB),
        ("d/src/main.rs", MAIN),
        ("d/src/bin/tool.rs", MAIN),
        ("d/src/bin/multi/main.rs", MAIN),
        ("d/src/bin/multi/helper.rs", LIB),
    ]);
    let (name, roots) = member_roots(&cfg);
    assert_eq!(name, Some("d_pkg"));
    assert_eq!(
        roots,
        [
            "d/src/bin/multi/main.rs",
            "d/src/bin/tool.rs",
            "d/src/main.rs"
        ]
    );
    // The own-library lane lists exactly the same roots.
    let own: Vec<&str> = cfg
        .binary_library_deps
        .keys()
        .filter(|k| k.starts_with("d/"))
        .map(String::as_str)
        .collect();
    assert_eq!(own, roots);
}

#[test]
fn dependency_lane_covers_a_bin_only_member_and_a_conflicting_name() {
    let bin_only = load(&[
        ("Cargo.toml", WS),
        ("a/Cargo.toml", A_TOML),
        ("a/src/lib.rs", LIB),
        ("d/Cargo.toml", &format!("{D_HEAD}{D_DEPS}")),
        ("d/src/main.rs", MAIN),
    ]);
    assert_eq!(member_roots(&bin_only), (None, vec!["d/src/main.rs"]));
    assert!(bin_only.binary_library_deps.is_empty());

    // A dependency named like the own library: the own-library lane refuses the whole
    // package, the dependency lane keeps the roots and the name for the Builder's rule.
    let conflict = load(&[
        ("Cargo.toml", WS),
        ("a/Cargo.toml", A_TOML),
        ("a/src/lib.rs", LIB),
        (
            "d/Cargo.toml",
            &format!("{D_HEAD}[lib]\nname = \"x\"\n[dependencies]\nx = {{ package = \"a\", path = \"../a\" }}\n"),
        ),
        ("d/src/lib.rs", LIB),
        ("d/src/main.rs", MAIN),
    ]);
    assert_eq!(member_roots(&conflict), (Some("x"), vec!["d/src/main.rs"]));
    assert!(conflict.binary_library_deps.is_empty());
}

#[test]
fn dependency_lane_skips_members_without_in_repo_dependencies_and_old_editions() {
    let none = load(&[
        ("Cargo.toml", WS),
        ("a/Cargo.toml", A_TOML),
        ("a/src/lib.rs", LIB),
        ("d/Cargo.toml", D_HEAD),
        ("d/src/main.rs", MAIN),
    ]);
    assert!(none.member_bin_roots.is_empty());
    let old = load(&[
        ("Cargo.toml", WS),
        ("a/Cargo.toml", "[package]\nname = \"a\"\nedition = \"2015\"\n"),
        ("a/src/lib.rs", LIB),
        (
            "d/Cargo.toml",
            "[package]\nname = \"d\"\nedition = \"2015\"\n[dependencies]\na = { path = \"../a\" }\n",
        ),
        ("d/src/main.rs", MAIN),
    ]);
    assert!(old.member_bin_roots.is_empty());
}
