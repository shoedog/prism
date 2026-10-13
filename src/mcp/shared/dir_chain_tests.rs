//! Tests for "could another user replace this directory, or something on the way to it".
//! The rule is tested as a pure function for the cases that need a second user, and through
//! the real filesystem for the ones a single user can set up.
use super::*;
use crate::mcp::shared::runtime_dir::{current_uid, validate_runtime_dir};
use std::os::unix::fs::PermissionsExt;

const ME: u32 = 501;
const SOMEONE_ELSE: u32 = 777;

fn directory(owner: u32, mode: u32) -> DirFacts {
    DirFacts {
        is_symlink: false,
        is_dir: true,
        owner,
        mode: 0o040000 | mode,
    }
}

// ── The rule ──

#[test]
fn a_directory_only_its_trusted_owner_can_write_protects_any_entry() {
    for owner in [ME, ROOT] {
        for mode in [0o700, 0o755, 0o711, 0o500] {
            for entry_owner in [ME, ROOT, SOMEONE_ELSE] {
                assert_eq!(
                    exposure(directory(owner, mode), entry_owner, ME),
                    None,
                    "owner {owner} mode {mode:o} entry owner {entry_owner}"
                );
            }
        }
    }
}

/// Rules out trusting the mode alone: the owner of a directory can rename what is in it
/// whatever the mode says.
#[test]
fn a_directory_owned_by_another_user_protects_nothing() {
    for mode in [0o700, 0o755, 0o1777] {
        assert_eq!(
            exposure(directory(SOMEONE_ELSE, mode), ME, ME),
            Some(Exposure::ForeignOwner {
                owner: SOMEONE_ELSE
            }),
            "mode {mode:o}"
        );
    }
}

/// Rules out checking only the "other" bit, and rules out treating a set-group-id bit as the
/// sticky bit.
#[test]
fn group_or_world_write_without_the_sticky_bit_protects_nothing() {
    for mode in [0o777, 0o775, 0o770, 0o720, 0o702, 0o2775] {
        assert_eq!(
            exposure(directory(ROOT, mode), ME, ME),
            Some(Exposure::WritableByOthers { mode }),
            "mode {mode:o}"
        );
    }
}

/// `/tmp`: world-writable and sticky. An entry is protected exactly when it is this user's or
/// root's. Rules out accepting any entry of a sticky directory.
#[test]
fn a_sticky_directory_protects_only_entries_of_this_user_or_root() {
    let tmp = directory(ROOT, 0o1777);
    assert_eq!(exposure(tmp, ME, ME), None);
    assert_eq!(exposure(tmp, ROOT, ME), None);
    assert_eq!(
        exposure(tmp, SOMEONE_ELSE, ME),
        Some(Exposure::ForeignEntryInSticky {
            entry_owner: SOMEONE_ELSE
        })
    );
}

// ── The walk, on the real filesystem ──

/// A temp directory under the system temp directory, by its real path.
fn private_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap();
    (dir, real)
}

fn make_dir(path: &Path, mode: u32) {
    std::fs::create_dir(path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn replaceable_container(outcome: &Result<(), RuntimeDirError>) -> Option<(&Path, Exposure)> {
    match outcome {
        Err(RuntimeDirError::Replaceable {
            container,
            exposure,
            ..
        }) => Some((container.as_path(), *exposure)),
        _ => None,
    }
}

/// The baseline every other walk test differs from: the temp directory of this host, and a
/// private directory in it, are accepted. If this fails the host's own temp directory is not
/// protected and the tests below prove nothing.
#[test]
fn a_private_directory_in_the_hosts_temp_directory_is_accepted() {
    let (_guard, root) = private_root();
    let dir = root.join("prism-mcpd");
    validate_runtime_dir(&dir).expect("a private chain");
    assert!(dir.is_dir());
}

/// The finding this module exists for: a directory that is itself private, below one that
/// others can write. Rules out checking the final directory only.
#[test]
fn a_world_writable_ancestor_without_the_sticky_bit_is_refused() {
    let (_guard, root) = private_root();
    let open = root.join("open");
    make_dir(&open, 0o777);
    let outcome = validate_runtime_dir(&open.join("prism-mcpd"));
    assert_eq!(
        replaceable_container(&outcome),
        Some((open.as_path(), Exposure::WritableByOthers { mode: 0o777 })),
        "{outcome:?}"
    );
    let message = outcome.unwrap_err().to_string();
    assert!(message.contains(open.to_str().unwrap()), "{message}");
    assert!(message.contains("0777"), "{message}");
    assert!(message.contains("PRISM_MCP_RUNTIME_DIR"), "{message}");
}

/// Rules out walking only the immediate parent.
#[test]
fn an_exposed_directory_several_levels_up_is_refused() {
    let (_guard, root) = private_root();
    let open = root.join("open");
    make_dir(&open, 0o770);
    make_dir(&open.join("a"), 0o700);
    make_dir(&open.join("a").join("b"), 0o700);
    let outcome = validate_runtime_dir(&open.join("a").join("b").join("prism-mcpd"));
    assert_eq!(
        replaceable_container(&outcome),
        Some((open.as_path(), Exposure::WritableByOthers { mode: 0o770 })),
        "{outcome:?}"
    );
}

/// A shared, sticky directory is where the default runtime directory lives on a Linux host
/// without `$XDG_RUNTIME_DIR`; it must stay usable.
#[test]
fn a_world_writable_sticky_ancestor_is_accepted_for_this_users_entries() {
    let (_guard, root) = private_root();
    let shared = root.join("shared");
    make_dir(&shared, 0o1777);
    validate_runtime_dir(&shared.join("prism-mcpd")).expect("sticky protects our own entry");
}

/// The path as written looks private, but one of its steps is a symlink whose target lies
/// under a directory others can write. Rules out checking only the directories of the path
/// as written, and rules out checking only the canonical path's directories when the hop
/// itself can be swapped.
#[test]
fn a_symlink_that_leads_through_an_exposed_directory_is_refused() {
    let (_guard, root) = private_root();
    let safe = root.join("safe");
    make_dir(&safe, 0o700);
    let open = root.join("open");
    make_dir(&open, 0o777);
    make_dir(&open.join("real"), 0o700);
    std::os::unix::fs::symlink(open.join("real"), safe.join("link")).unwrap();
    let outcome = validate_runtime_dir(&safe.join("link").join("prism-mcpd"));
    assert_eq!(
        replaceable_container(&outcome),
        Some((open.as_path(), Exposure::WritableByOthers { mode: 0o777 })),
        "{outcome:?}"
    );
}

/// The exposed directory is only on the way to a second symlink, not an ancestor of the
/// final directory: `safe/link -> open/hop -> safe/real`. Whoever can write `open` can
/// re-point `hop`. Rules out canonicalizing and checking the result's ancestors.
#[test]
fn an_exposed_directory_that_only_holds_an_intermediate_symlink_is_refused() {
    let (_guard, root) = private_root();
    let safe = root.join("safe");
    make_dir(&safe, 0o700);
    make_dir(&safe.join("real"), 0o700);
    let open = root.join("open");
    make_dir(&open, 0o777);
    std::os::unix::fs::symlink(safe.join("real"), open.join("hop")).unwrap();
    std::os::unix::fs::symlink(open.join("hop"), safe.join("link")).unwrap();
    let outcome = validate_runtime_dir(&safe.join("link").join("prism-mcpd"));
    assert_eq!(
        replaceable_container(&outcome),
        Some((open.as_path(), Exposure::WritableByOthers { mode: 0o777 })),
        "{outcome:?}"
    );
}

/// Symlinks in protected directories are fine, relative ones and `..` included.
#[test]
fn symlinks_through_protected_directories_are_followed_and_accepted() {
    let (_guard, root) = private_root();
    let a = root.join("a");
    make_dir(&a, 0o700);
    make_dir(&root.join("b"), 0o755);
    std::os::unix::fs::symlink("../b", a.join("to-b")).unwrap();
    validate_runtime_dir(&a.join("to-b").join("prism-mcpd")).expect("relative link, protected");
    assert!(root.join("b").join("prism-mcpd").is_dir());
}

/// `/tmp` on this host (a root-owned symlink on macOS, a sticky directory on Linux) must be
/// accepted, since it is the fallback base on a host with no user runtime directory.
#[test]
fn a_directory_under_slash_tmp_is_accepted() {
    let base = match tempfile::Builder::new()
        .prefix("s2chain")
        .tempdir_in("/tmp")
    {
        Ok(base) => base,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("skipped: this environment does not allow a directory under /tmp");
            return;
        }
        Err(error) => panic!("tempdir under /tmp: {error}"),
    };
    validate_runtime_dir(&base.path().join("prism-mcpd")).expect("/tmp is sticky");
}

#[test]
fn a_symlink_loop_is_an_inspection_error_not_a_hang() {
    let (_guard, root) = private_root();
    std::os::unix::fs::symlink(root.join("b"), root.join("a")).unwrap();
    std::os::unix::fs::symlink(root.join("a"), root.join("b")).unwrap();
    let outcome = check_resolution_chain(&root.join("a").join("x"), current_uid());
    assert!(
        matches!(&outcome, Err(RuntimeDirError::Io { op: "inspect", .. })),
        "{outcome:?}"
    );
}

/// `..` in the path as written steps back to the directory above, as the kernel does: the
/// walk of `private/x/../prism-mcpd` ends in `private`, and `x` being exposed does not matter
/// because nothing is reached through it.
#[test]
fn dot_dot_steps_back_out_of_a_directory() {
    let (_guard, root) = private_root();
    make_dir(&root.join("private"), 0o700);
    make_dir(&root.join("private").join("x"), 0o777);
    make_dir(&root.join("private").join("prism-mcpd"), 0o700);
    let dir = root.join("private").join("x").join("..").join("prism-mcpd");
    check_resolution_chain(&dir, current_uid()).expect("nothing is reached through x");
    // Reaching something through `x` is refused.
    make_dir(&root.join("private").join("x").join("prism-mcpd"), 0o700);
    let through = root.join("private").join("x").join("prism-mcpd");
    assert!(replaceable_container(&check_resolution_chain(&through, current_uid())).is_some());
}
