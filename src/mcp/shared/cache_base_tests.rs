//! Tests for the one canonical spelling of a configuration's cache base directory.
use super::*;
use crate::mcp::CacheMode;
use std::path::{Path, PathBuf};

/// A temp directory by its canonical path (on macOS the temp directory is reached through the
/// `/var` symlink).
fn canonical_temp() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    (dir, real)
}

fn dir(path: impl Into<PathBuf>) -> CacheMode {
    CacheMode::Dir(path.into())
}

/// A working directory that cannot be read. Every test that passes this shows that an
/// absolute base is resolved without asking for the working directory.
fn no_working_dir() -> std::io::Result<PathBuf> {
    Err(std::io::Error::from(std::io::ErrorKind::NotFound))
}

#[test]
fn a_cache_free_configuration_has_no_cache_base() {
    let outcome =
        effective_cache_base_in(&CacheMode::NoCache, Path::new("/unused"), no_working_dir);
    assert!(
        matches!(outcome, Err(CacheBaseError::NoCache)),
        "{outcome:?}"
    );
}

/// Spec §4.1 step 1 and §9: `CacheMode::Default` and the same directory given explicitly are
/// one base, and that is decided before the directory exists. Rules out comparing the strings
/// the user typed, and rules out creating the directory in order to canonicalize it.
#[test]
fn default_and_its_explicit_equivalents_are_one_base_before_the_directory_exists() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    let default_base = real.join("alias").join("nav-base");
    let expected = real.join("store").join("nav-base");

    let from_default =
        effective_cache_base_in(&CacheMode::Default, &default_base, no_working_dir).unwrap();
    assert_eq!(from_default, expected);

    let spellings = [
        dir(real.join("store/nav-base")),
        dir(real.join("store/nav-base/")),
        dir(real.join("store/./nav-base")),
        dir(real.join("alias/nav-base")),
        dir(real.join("store/not-made-yet/../nav-base")),
        dir(real.join("store/nav-base/deeper/..")),
    ];
    for spelling in &spellings {
        let resolved = effective_cache_base_in(spelling, Path::new("/unused"), no_working_dir)
            .unwrap_or_else(|error| panic!("{spelling:?}: {error}"));
        assert_eq!(resolved, expected, "{spelling:?}");
    }
    assert!(
        !expected.exists() && !real.join("store/not-made-yet").exists(),
        "resolving a cache base must not create it"
    );
}

#[test]
fn a_relative_base_is_resolved_against_the_working_directory() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("work")).unwrap();
    let resolved = effective_cache_base_in(&dir("../caches/nav"), Path::new("/unused"), || {
        Ok(real.join("work"))
    })
    .unwrap();
    assert_eq!(resolved, real.join("caches").join("nav"));
}

/// A process whose working directory has been removed can still name an absolute base (every
/// other test here passes `no_working_dir`); a relative base then has no meaning and is an
/// error that names it.
#[test]
fn only_a_relative_base_needs_the_working_directory() {
    let outcome = effective_cache_base_in(&dir("caches/nav"), Path::new("/unused"), no_working_dir);
    assert!(
        matches!(&outcome, Err(CacheBaseError::Io { path, .. }) if path == Path::new("caches/nav")),
        "{outcome:?}"
    );
    let relative_default =
        effective_cache_base_in(&CacheMode::Default, Path::new("caches"), no_working_dir);
    assert!(
        matches!(relative_default, Err(CacheBaseError::Io { .. })),
        "{relative_default:?}"
    );
}

#[test]
fn an_existing_base_is_canonical_through_symlinks_dots_and_slashes() {
    let (guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/base")).unwrap();
    std::os::unix::fs::symlink(real.join("a"), real.join("link")).unwrap();
    for spelling in [
        real.join("a/base"),
        real.join("a/base/"),
        real.join("a/./base/."),
        real.join("link/base"),
        real.join("a/base/../base"),
        // The uncanonical spelling of the temp directory itself (differs on macOS).
        guard.path().join("a/base"),
    ] {
        let resolved =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir).unwrap();
        assert_eq!(resolved, real.join("a/base"), "{}", spelling.display());
    }
}

/// Rules out removing `..` textually: `jump/..` is the parent of what `jump` points at, which
/// is where the cache layer's `create_dir_all` would put the directory.
#[test]
fn dot_dot_after_a_symlink_is_the_parent_of_the_link_target() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/b")).unwrap();
    std::os::unix::fs::symlink(real.join("a/b"), real.join("jump")).unwrap();
    let resolved = effective_cache_base_in(
        &dir(real.join("jump/../leaf")),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, real.join("a/leaf"));
}

/// A `..` that climbs out of a part that does not exist lands back on real ground, and the
/// components after it are resolved through symlinks again.
#[test]
fn a_symlink_after_climbing_out_of_a_missing_part_is_still_resolved() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    let resolved = effective_cache_base_in(
        &dir(real.join("missing/../alias/nav-base")),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, real.join("store/nav-base"));
}

/// The claim behind all of the above, checked against the operation the cache layer performs
/// (`create_dir_all` on a path under the base): the spelling resolved before anything exists
/// is the canonical path of what then gets created. Rules out any normalization that
/// disagrees with the filesystem, whichever way it disagrees.
#[test]
fn the_resolved_base_is_where_create_dir_all_then_puts_it() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir_all(real.join("a/b")).unwrap();
    std::os::unix::fs::symlink(real.join("a/b"), real.join("jump")).unwrap();
    for (index, tail) in [
        "plain/base",
        "jump/base",
        "jump/../from-link-parent",
        "missing/../jump/after-missing",
        "deep/er/../../shallow/./base/",
        "a/b/../b/x/y/..",
    ]
    .into_iter()
    .enumerate()
    {
        let spelling = real.join(tail);
        let resolved =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir)
                .unwrap_or_else(|error| panic!("case {index} ({tail}): {error}"));
        assert!(!resolved.exists(), "case {index} ({tail}) starts absent");
        std::fs::create_dir_all(&spelling).unwrap();
        assert_eq!(
            std::fs::canonicalize(&spelling).unwrap(),
            resolved,
            "case {index} ({tail})"
        );
    }
}

/// The daemon is started with the base the shim resolved, and resolves it again; the two must
/// agree or every connection would be answered `MISMATCH`.
#[test]
fn resolving_a_resolved_base_changes_nothing() {
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("store")).unwrap();
    std::os::unix::fs::symlink("store", real.join("alias")).unwrap();
    for spelling in [real.join("alias/x/../nav"), real.join("alias")] {
        let once =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir).unwrap();
        let twice =
            effective_cache_base_in(&dir(&once), Path::new("/unused"), no_working_dir).unwrap();
        assert_eq!(once, twice);
    }
}

#[test]
fn climbing_above_the_root_stays_at_the_root() {
    let resolved = effective_cache_base_in(
        &dir("/../../prism-no-such-dir"),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(resolved, PathBuf::from("/prism-no-such-dir"));
}

/// Neither location can ever hold a cache, so neither names a base two processes could share.
#[test]
fn a_base_below_a_regular_file_or_through_a_dangling_symlink_is_an_error() {
    let (_guard, real) = canonical_temp();
    std::fs::write(real.join("file"), "").unwrap();
    std::os::unix::fs::symlink(real.join("nowhere"), real.join("dangling")).unwrap();
    for spelling in [
        real.join("file/base"),
        real.join("dangling/base"),
        real.join("dangling"),
    ] {
        let outcome =
            effective_cache_base_in(&dir(&spelling), Path::new("/unused"), no_working_dir);
        assert!(
            matches!(&outcome, Err(CacheBaseError::Io { .. })),
            "{}: {outcome:?}",
            spelling.display()
        );
    }
}

/// A regular file can only be the last thing a path names. Stepping back out of it with `..`
/// must not hide that the path as written cannot be created (`create_dir_all` fails with
/// "not a directory"), and a regular file is not a cache base either. Rules out resolving `..`
/// by dropping whatever the last component was.
#[test]
fn a_regular_file_is_never_stepped_through_or_accepted_as_the_base() {
    let (_guard, real) = canonical_temp();
    let file = real.join("a-file");
    std::fs::write(&file, b"").unwrap();
    std::os::unix::fs::symlink(&file, real.join("link-to-file")).unwrap();
    for base in [
        file.clone(),
        file.join("..").join("cache"),
        real.join("link-to-file"),
        real.join("link-to-file").join("..").join("cache"),
    ] {
        let outcome = effective_cache_base_in(&dir(&base), Path::new("/unused"), no_working_dir);
        assert!(
            matches!(&outcome, Err(CacheBaseError::Io { path, source })
                if path == &file && source.raw_os_error() == Some(libc::ENOTDIR)),
            "{}: {outcome:?}",
            base.display()
        );
        assert!(
            std::fs::create_dir_all(base.join("x")).is_err(),
            "the kernel refuses {} too",
            base.display()
        );
    }
    // The same shape through a directory is fine.
    std::fs::create_dir(real.join("a-dir")).unwrap();
    let through_dir = real.join("a-dir").join("..").join("cache");
    assert_eq!(
        effective_cache_base_in(&dir(through_dir), Path::new("/unused"), no_working_dir).unwrap(),
        real.join("cache")
    );
}

/// The default base is computed here and, separately, inside the navigation cache. This pins
/// the two together: a `CacheMode::Default` daemon caches exactly where the in-process server
/// does.
#[test]
fn the_default_base_is_the_one_the_navigation_cache_uses() {
    let repo_dir = tempfile::tempdir().unwrap();
    std::fs::write(repo_dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let repo = crate::repo_loader::load_repo(repo_dir.path()).unwrap();
    assert_eq!(
        crate::navigation::cache::nav_cache_dir(&repo),
        crate::navigation::cache::nav_cache_subdir(&default_cache_base(), &repo)
    );
}

/// A component that exists is spelled as the filesystem spells it, not as the caller typed it.
/// Only a case-insensitive filesystem can tell the difference, so the test skips itself on a
/// case-sensitive one. Rules out resolving a component only when it is a symlink.
#[test]
fn an_existing_component_typed_in_another_case_gets_its_on_disk_spelling() {
    use std::os::unix::fs::MetadataExt;
    let (_guard, real) = canonical_temp();
    std::fs::create_dir(real.join("MixedCase")).unwrap();
    let typed = real.join("mixedcase");
    let same_directory = match (
        std::fs::metadata(&typed),
        std::fs::metadata(real.join("MixedCase")),
    ) {
        (Ok(typed), Ok(on_disk)) => typed.ino() == on_disk.ino() && typed.dev() == on_disk.dev(),
        _ => false,
    };
    if !same_directory {
        eprintln!("skipped: this filesystem is case-sensitive");
        return;
    }
    let base = effective_cache_base_in(
        &dir(typed.join("later")),
        Path::new("/unused"),
        no_working_dir,
    )
    .unwrap();
    assert_eq!(base, real.join("MixedCase").join("later"));
}

#[test]
fn the_environment_reader_agrees_with_the_pure_resolver() {
    let (_guard, real) = canonical_temp();
    let explicit = dir(real.join("explicit"));
    assert_eq!(
        effective_cache_base(&explicit).unwrap(),
        real.join("explicit")
    );
    assert_eq!(
        effective_cache_base(&CacheMode::Default).unwrap(),
        effective_cache_base(&dir(default_cache_base())).unwrap()
    );
    assert!(matches!(
        effective_cache_base(&CacheMode::NoCache),
        Err(CacheBaseError::NoCache)
    ));
}
