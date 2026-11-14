//! #829 chain A, round 2: loader-path rows. The macro definitions sit physically inside the
//! repository, but the loader does not (or cannot) hand them to the census: a computed include
//! (G2-terra-02), an absolute include (G2-sol2-01), `..` after a directory symlink
//! (G2-sol2-02), an include name whose case differs on a case-insensitive filesystem
//! (G2-sol2-03) and a header injected by the build (G2-sol2-04). Every test named `boundary_*`
//! is a BOUNDARY row: placement in #829, in #813 or split is decided later (a row deferred to
//! #813 will be marked `#[ignore = "#813"]`). The required behaviour is the goal's: the live
//! pair `x = 41` to `return x` is kept (compiled by the enumerators: `41 2`, both dialects).
//! Real files live in a tempdir built from Rust strings, never under `tests/fixtures`.
//! Host-dependent rows (symlinks, case-insensitive names) skip with a printed reason.
use super::c_prune_brackets_r1_tests::loaded::write;
use super::Producer;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::repo_loader::load_repo;
use std::path::Path;

/// The shared body of the sol2 rows; the prefix is the include line(s).
const BODY: &str = "int probe(int c) {\n    int x = 41;\n    if (c) OPEN;\n    x = 2;\n    CLOSE;\n    return x;\n}\nint main(void) { return probe(0) != 41 || probe(1) != 2; }\n";
const PAYLOAD: &str = "#define OPEN {\n#define CLOSE }\n";

/// `(before, after)`: does the pair `x@from -> x@to` exist before the rules and after them,
/// with the census the loader builds for `d`.
fn pair(d: &Path, target: &str, from: usize, to: usize) -> (bool, bool) {
    let repo = load_repo(d).unwrap();
    let census = CMacroCensus::from_files(&repo.files);
    let p = &repo.files[target];
    let file = target.to_owned();
    let has = |r: &super::FileRefs| {
        r.edges
            .iter()
            .any(|e| e.from.path.to_string() == "x" && e.from.line == from && e.to.line == to)
    };
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &census);
    (has(&before), has(&after))
}

/// Premise of the control rows: the loader's census holds the payload's macros.
fn assert_seen(d: &Path) {
    let repo = load_repo(d).unwrap();
    let census = CMacroCensus::from_files(&repo.files);
    for n in ["OPEN", "CLOSE"] {
        assert!(census.get(n).is_some(), "{n} not read by the loader");
    }
}

fn assert_kept(d: &Path, target: &str, from: usize, to: usize, what: &str) {
    let (before, after) = pair(d, target, from, to);
    assert!(
        before,
        "{what}: inadmissible, the baseline has no x@{from} -> x@{to}"
    );
    assert!(
        after,
        "{what}: the live pair x@{from} -> x@{to} was dropped"
    );
}

/// `case.<ext>` = `prefix` + the shared body, plus the extra `(path, contents)` files.
fn repo(ext: &str, prefix: &str, extra: &[(&str, &str)]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), &format!("case.{ext}"), &format!("{prefix}{BODY}"));
    for (rel, body) in extra {
        write(d.path(), rel, body);
    }
    d
}

// ---- G2-terra-02: a computed include of a file the net does not collect ----

fn computed(ext: &str, name: &str) -> tempfile::TempDir {
    // Pair lines: probe at 3, `int x = 41;` at 4, `return x;` at 8.
    let d = tempfile::tempdir().unwrap();
    let src = format!("#define HEADER \"{name}\"\n#include HEADER\n{BODY}");
    write(d.path(), &format!("case.{ext}"), &src);
    write(d.path(), name, PAYLOAD);
    d
}

#[test]
fn boundary_g2_terra_02_computed_include_of_a_non_net_extension() {
    for ext in ["c", "cc"] {
        let d = computed(ext, "defs.payload");
        assert_kept(d.path(), &format!("case.{ext}"), 4, 8, ext);
    }
}

/// G2-terra-03: the same computed include of `defs.inc`, which the C-like-extension net does
/// collect (`read=1`): the definitions are seen, so the pair must be kept (red on base only
/// because the facts are stubs).
#[test]
fn g2_terra_03_computed_include_of_a_net_file() {
    for ext in ["c", "cc"] {
        let d = computed(ext, "defs.inc");
        assert_seen(d.path());
        assert_kept(d.path(), &format!("case.{ext}"), 4, 8, ext);
    }
}

// ---- G2-sol2-01 .. 04 (BOUNDARY): the other ways an in-repo header is not read ----

#[test]
fn boundary_g2_sol2_01_absolute_include_of_an_in_repo_file() {
    for ext in ["c", "cc"] {
        let d = repo(ext, "", &[("defs.payload", PAYLOAD)]);
        // The prefix needs the canonical absolute path of the file it names.
        let abs = d.path().canonicalize().unwrap().join("defs.payload");
        write(
            d.path(),
            &format!("case.{ext}"),
            &format!("#include \"{}\"\n{BODY}", abs.display()),
        );
        assert_kept(d.path(), &format!("case.{ext}"), 3, 7, ext);
    }
}

#[test]
fn boundary_g2_sol2_02_parent_after_a_directory_symlink() {
    #[cfg(unix)]
    for ext in ["c", "cc"] {
        // a/link -> ../actual/deep; actual/defs.payload; there is no a/defs.payload.
        let d = repo(
            ext,
            "#include \"a/link/../defs.payload\"\n",
            &[("actual/defs.payload", PAYLOAD), ("actual/deep/keep", "")],
        );
        std::fs::create_dir_all(d.path().join("a")).unwrap();
        if let Err(e) = std::os::unix::fs::symlink("../actual/deep", d.path().join("a/link")) {
            eprintln!("SKIP boundary_g2_sol2_02: cannot create a symlink here ({e})");
            return;
        }
        assert!(
            d.path().join("a/link/../defs.payload").exists(),
            "capability: the filesystem must resolve link/.. through the link target"
        );
        assert_kept(d.path(), &format!("case.{ext}"), 3, 7, ext);
    }
    #[cfg(not(unix))]
    eprintln!("SKIP boundary_g2_sol2_02: needs unix symlinks");
}

#[test]
fn boundary_g2_sol2_03_include_name_differs_in_case() {
    for ext in ["c", "cc"] {
        let d = repo(
            ext,
            "#include \"DEFS.PAYLOAD\"\n",
            &[("defs.payload", PAYLOAD)],
        );
        if !d.path().join("DEFS.PAYLOAD").exists() {
            eprintln!(
                "SKIP boundary_g2_sol2_03: case-sensitive filesystem, `DEFS.PAYLOAD` does not \
                 resolve to `defs.payload` here, so the compiler would not read it either"
            );
            return;
        }
        assert_kept(d.path(), &format!("case.{ext}"), 3, 7, ext);
    }
}

/// Control for sol2-03: the exact-case spelling is discovered (green once the facts exist).
#[test]
fn g2_sol2_03_control_exact_case_include() {
    for ext in ["c", "cc"] {
        let d = repo(
            ext,
            "#include \"defs.payload\"\n",
            &[("defs.payload", PAYLOAD)],
        );
        assert_seen(d.path());
        assert_kept(d.path(), &format!("case.{ext}"), 3, 7, ext);
    }
}

/// G2-sol2-04. prism has no way to be told a build's `-include <file>` (no loader or CLI input
/// carries forced includes, `--compile-commands` only feeds type enrichment), so the row is
/// expressed as the input prism does receive: the source names no header, and the file that
/// defines OPEN/CLOSE sits in the repository under a non-net extension. The compiled program
/// with `-include defs.payload` prints `41 2`.
#[test]
fn boundary_g2_sol2_04_header_injected_by_the_build() {
    for ext in ["c", "cc"] {
        let d = repo(ext, "", &[("defs.payload", PAYLOAD)]);
        assert_kept(d.path(), &format!("case.{ext}"), 2, 6, ext);
    }
}

/// Companion of sol2-04: the same file named `defs.inc` is collected by the extension net
/// without any include, so the definitions are seen.
#[test]
fn g2_sol2_04_companion_net_file_is_collected_without_an_include() {
    for ext in ["c", "cc"] {
        let d = repo(ext, "", &[("defs.inc", PAYLOAD)]);
        assert_seen(d.path());
        assert_kept(d.path(), &format!("case.{ext}"), 2, 6, ext);
    }
}

/// Discriminator of the sol2 rows: a plain relative literal include of the same payload is
/// read (`read=1`), so the misses above are not caused by the unusual extension alone.
#[test]
fn g2_sol2_relative_literal_include_control() {
    for ext in ["c", "cc"] {
        let d = repo(
            ext,
            "#include \"defs.payload\"\n",
            &[("defs.payload", PAYLOAD)],
        );
        assert_seen(d.path());
        assert_kept(d.path(), &format!("case.{ext}"), 3, 7, ext);
    }
}
