//! Tests for where the runtime directory is and for the validation run on every use. Each
//! validation test names the wrong implementation it rules out.
use super::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn mode_of(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).unwrap().mode() & 0o777
}

fn private_dir(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

// ── Where the directory is ──

#[test]
fn the_override_is_the_base_and_the_directory_name_is_joined_to_it() {
    let dir = runtime_dir_from(
        Some(OsString::from("/override/base")),
        Some(PathBuf::from("/run/user/501")),
        PathBuf::from("/tmp"),
    )
    .unwrap();
    assert_eq!(dir, PathBuf::from("/override/base/prism-mcpd"));
}

#[test]
fn without_an_override_the_user_runtime_dir_is_the_base_then_the_temp_dir() {
    let with_user_dir = runtime_dir_from(
        None,
        Some(PathBuf::from("/run/user/501")),
        PathBuf::from("/tmp"),
    )
    .unwrap();
    assert_eq!(with_user_dir, PathBuf::from("/run/user/501/prism-mcpd"));

    let temp_only = runtime_dir_from(None, None, PathBuf::from("/var/folders/x/T/")).unwrap();
    assert_eq!(temp_only, PathBuf::from("/var/folders/x/T/prism-mcpd"));
}

#[test]
fn an_empty_override_counts_as_unset() {
    let dir = runtime_dir_from(Some(OsString::new()), None, PathBuf::from("/tmp")).unwrap();
    assert_eq!(dir, PathBuf::from("/tmp/prism-mcpd"));
}

/// Rules out resolving a relative base against the working directory: two shims started in
/// different directories would then look for the same daemon in two places.
#[test]
fn a_relative_base_is_refused_whichever_source_it_came_from() {
    let from_override = runtime_dir_from(
        Some(OsString::from("run/prism")),
        None,
        PathBuf::from("/tmp"),
    );
    assert!(
        matches!(&from_override, Err(RuntimeDirError::NotAbsolute { path }) if path == Path::new("run/prism")),
        "{from_override:?}"
    );
    let from_temp = runtime_dir_from(None, None, PathBuf::from("tmp"));
    assert!(
        matches!(from_temp, Err(RuntimeDirError::NotAbsolute { .. })),
        "{from_temp:?}"
    );
}

#[test]
fn the_environment_reader_agrees_with_the_pure_resolver() {
    let expected = runtime_dir_from(
        std::env::var_os(RUNTIME_DIR_ENV),
        dirs::runtime_dir(),
        std::env::temp_dir(),
    )
    .unwrap();
    assert_eq!(runtime_dir().unwrap(), expected);
    assert!(expected.ends_with(RUNTIME_DIR_NAME));
}

// ── Validation against the real filesystem ──

/// Rules out `create_dir`/`create_dir_all` with the default mode: under the usual umask that
/// makes a 0755 directory, which this same validation must then refuse.
#[test]
fn an_absent_directory_is_created_0700_and_passes() {
    let base = tempfile::tempdir().unwrap();
    let dir = base.path().join("prism-mcpd");
    validate_runtime_dir(&dir).expect("created and valid");
    assert!(std::fs::symlink_metadata(&dir).unwrap().is_dir());
    assert_eq!(mode_of(&dir), 0o700);
    validate_runtime_dir(&dir).expect("valid on the next use too");
}

/// Rules out `create_dir_all`: a mistyped override must not grow a directory tree.
#[test]
fn a_missing_base_is_an_error_and_nothing_is_created() {
    let base = tempfile::tempdir().unwrap();
    let dir = base.path().join("no-such-base").join("prism-mcpd");
    let outcome = validate_runtime_dir(&dir);
    assert!(
        matches!(&outcome, Err(RuntimeDirError::Io { op: "create", .. })),
        "{outcome:?}"
    );
    assert!(!base.path().join("no-such-base").exists());
}

#[test]
fn an_existing_private_directory_passes_and_its_contents_are_untouched() {
    let base = tempfile::tempdir().unwrap();
    let dir = private_dir(base.path(), "prism-mcpd");
    std::fs::write(dir.join("0123456789abcdef.pid"), "41\n").unwrap();
    validate_runtime_dir(&dir).expect("valid");
    assert_eq!(
        std::fs::read_to_string(dir.join("0123456789abcdef.pid")).unwrap(),
        "41\n"
    );
}

/// Rules out `fs::metadata` (which follows the link and sees a perfectly good directory) and
/// rules out `lstat` on a spelling with a trailing slash (which also follows it).
#[test]
fn a_symlink_to_a_private_directory_is_refused_with_or_without_a_trailing_slash() {
    let base = tempfile::tempdir().unwrap();
    let target = private_dir(base.path(), "elsewhere");
    let link = base.path().join("prism-mcpd");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let plain = validate_runtime_dir(&link);
    assert!(
        matches!(&plain, Err(RuntimeDirError::Symlink { path }) if path == &link),
        "{plain:?}"
    );
    let mut with_slash = link.clone().into_os_string();
    with_slash.push("/");
    let slashed = validate_runtime_dir(Path::new(&with_slash));
    assert!(
        matches!(slashed, Err(RuntimeDirError::Symlink { .. })),
        "{slashed:?}"
    );
}

/// Rules out checking only "not world-writable", only the group bits, or only the other bits,
/// and rules out repairing the mode instead of refusing (a directory that was open may already
/// hold files someone else put there).
#[test]
fn any_mode_other_than_0700_is_refused_and_left_as_it_was() {
    for mode in [0o750, 0o705, 0o710, 0o701, 0o755, 0o770, 0o777, 0o500] {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("prism-mcpd");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).unwrap();

        let outcome = validate_runtime_dir(&dir);
        assert!(
            matches!(&outcome, Err(RuntimeDirError::InsecureMode { mode: found, .. }) if *found == mode),
            "mode {mode:o}: {outcome:?}"
        );
        assert_eq!(mode_of(&dir), mode, "mode {mode:o} must not be repaired");
    }
}

#[test]
fn a_regular_file_in_the_directory_s_place_is_refused() {
    let base = tempfile::tempdir().unwrap();
    let file = base.path().join("prism-mcpd");
    std::fs::write(&file, "").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
    let outcome = validate_runtime_dir(&file);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotADirectory { .. })),
        "{outcome:?}"
    );
}

/// Rules out a validation with no ownership check. The directory is real and 0700; only the
/// uid it is compared against differs, so nothing but the owner comparison can refuse it.
#[test]
fn a_private_directory_owned_by_a_different_uid_is_refused() {
    let base = tempfile::tempdir().unwrap();
    let dir = private_dir(base.path(), "prism-mcpd");
    let me = current_uid();
    let someone_else = me.wrapping_add(1);
    let outcome = validate_runtime_dir_with(&RealDirAccess, &dir, someone_else);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::NotOwned { owner, current, .. })
                if *owner == me && *current == someone_else
        ),
        "{outcome:?}"
    );
}

/// The same check with nothing injected: `/` belongs to someone else (root, or the id an
/// unmapped root shows as in a rootless container). Skipped when `/` is the caller's own.
#[test]
fn a_directory_that_really_belongs_to_someone_else_is_refused() {
    let root_owner = std::fs::symlink_metadata("/").unwrap().uid();
    if root_owner == current_uid() {
        eprintln!("skipped: `/` is owned by the current user");
        return;
    }
    let outcome = validate_runtime_dir(Path::new("/"));
    assert!(
        matches!(&outcome, Err(RuntimeDirError::NotOwned { owner, .. }) if *owner == root_owner),
        "{outcome:?}"
    );
}

/// A real inspection failure that is not "absent": a path below a regular file is `ENOTDIR`.
/// Rules out reading every `lstat` error as "the directory does not exist yet", which would
/// go on to `mkdir` and report the wrong operation.
#[test]
fn a_real_inspection_error_is_reported_as_one() {
    let parent = tempfile::tempdir().unwrap();
    let file = parent.path().join("a-file");
    std::fs::write(&file, b"").unwrap();
    let outcome = validate_runtime_dir(&file.join("prism-mcpd"));
    assert!(
        matches!(&outcome, Err(RuntimeDirError::Io { op: "inspect", .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_relative_directory_is_refused_before_the_filesystem_is_touched() {
    let fake = FakeDirs::new(vec![], Ok(()));
    let outcome = validate_runtime_dir_with(&fake, Path::new("prism-mcpd"), 501);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotAbsolute { .. })),
        "{outcome:?}"
    );
    assert!(fake.calls().is_empty());
}

#[test]
fn the_current_uid_is_the_owner_of_what_this_process_creates() {
    let base = tempfile::tempdir().unwrap();
    assert_eq!(
        std::fs::symlink_metadata(base.path()).unwrap().uid(),
        current_uid()
    );
}

// ── Validation with injected filesystem outcomes ──

type InspectOutcome = std::io::Result<Option<DirFacts>>;

struct FakeDirs {
    inspections: RefCell<VecDeque<InspectOutcome>>,
    creation: RefCell<Option<std::io::Result<()>>>,
    calls: RefCell<Vec<String>>,
}

impl FakeDirs {
    fn new(inspections: Vec<InspectOutcome>, creation: std::io::Result<()>) -> Self {
        Self {
            inspections: RefCell::new(inspections.into()),
            creation: RefCell::new(Some(creation)),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl DirAccess for FakeDirs {
    fn inspect(&self, _path: &Path) -> InspectOutcome {
        self.calls.borrow_mut().push("inspect".into());
        self.inspections
            .borrow_mut()
            .pop_front()
            .expect("an inspection the test did not script")
    }

    fn create_private(&self, _path: &Path) -> std::io::Result<()> {
        self.calls.borrow_mut().push("create".into());
        self.creation
            .borrow_mut()
            .take()
            .expect("a second creation the test did not script")
    }
}

const UID: u32 = 501;
const GOOD: DirFacts = DirFacts {
    is_symlink: false,
    is_dir: true,
    owner: UID,
    mode: 0o40700,
};

fn denied() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::PermissionDenied)
}

fn check(fake: &FakeDirs) -> Result<(), RuntimeDirError> {
    validate_runtime_dir_with(fake, Path::new("/run/user/501/prism-mcpd"), UID)
}

/// Rules out treating "could not look" as "not there": that would go on to create the
/// directory, or to use one nobody inspected.
#[test]
fn an_inspection_error_is_reported_and_nothing_is_created() {
    let fake = FakeDirs::new(vec![Err(denied())], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "inspect", source, .. })
                if source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect"]);
}

#[test]
fn a_creation_error_is_reported_as_an_unwritable_runtime_directory() {
    let fake = FakeDirs::new(vec![Ok(None)], Err(denied()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "create", source, .. })
                if source.kind() == std::io::ErrorKind::PermissionDenied
        ),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect", "create"]);
}

/// Rules out trusting a directory because this process just created it: between the `mkdir`
/// and the first use, whoever can write the parent can put something else there.
#[test]
fn what_is_found_after_a_successful_creation_is_still_checked() {
    let swapped = DirFacts {
        is_symlink: true,
        is_dir: false,
        ..GOOD
    };
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(swapped))], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(outcome, Err(RuntimeDirError::Symlink { .. })),
        "{outcome:?}"
    );
    assert_eq!(fake.calls(), ["inspect", "create", "inspect"]);
}

/// Two shims start together; the loser of the `mkdir` race must accept the winner's directory
/// if (and only if) it validates.
#[test]
fn losing_the_creation_race_to_a_valid_directory_passes() {
    let exists = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(GOOD))], Err(exists));
    check(&fake).expect("the winner's directory is valid");

    let exists = std::io::Error::from(std::io::ErrorKind::AlreadyExists);
    let foreign = DirFacts {
        owner: UID + 1,
        ..GOOD
    };
    let fake = FakeDirs::new(vec![Ok(None), Ok(Some(foreign))], Err(exists));
    let outcome = check(&fake);
    assert!(
        matches!(outcome, Err(RuntimeDirError::NotOwned { .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_directory_that_vanishes_after_creation_is_an_error() {
    let fake = FakeDirs::new(vec![Ok(None), Ok(None)], Ok(()));
    let outcome = check(&fake);
    assert!(
        matches!(
            &outcome,
            Err(RuntimeDirError::Io { op: "inspect", source, .. })
                if source.kind() == std::io::ErrorKind::NotFound
        ),
        "{outcome:?}"
    );
}

/// The file-type and set-id bits `lstat` reports beside the permission bits are not access
/// rights: a 0700 directory under a set-group-id parent is created `02700` on Linux and is
/// exactly as private.
#[test]
fn only_the_nine_permission_bits_are_compared() {
    let setgid = DirFacts {
        mode: 0o42700,
        ..GOOD
    };
    check(&FakeDirs::new(vec![Ok(Some(setgid))], Ok(()))).expect("02700 is private");

    let group_readable = DirFacts {
        mode: 0o42740,
        ..GOOD
    };
    let outcome = check(&FakeDirs::new(vec![Ok(Some(group_readable))], Ok(())));
    assert!(
        matches!(
            outcome,
            Err(RuntimeDirError::InsecureMode { mode: 0o740, .. })
        ),
        "{outcome:?}"
    );
}

#[test]
fn every_refusal_names_the_path_and_how_to_point_elsewhere() {
    let path = PathBuf::from("/tmp/prism-mcpd");
    let errors = [
        RuntimeDirError::NotAbsolute { path: path.clone() },
        RuntimeDirError::Io {
            op: "create",
            path: path.clone(),
            source: denied(),
        },
        RuntimeDirError::Symlink { path: path.clone() },
        RuntimeDirError::NotADirectory { path: path.clone() },
        RuntimeDirError::NotOwned {
            path: path.clone(),
            owner: 0,
            current: 501,
        },
        RuntimeDirError::InsecureMode {
            path: path.clone(),
            mode: 0o755,
        },
    ];
    for error in errors {
        let text = error.to_string();
        assert!(text.contains("/tmp/prism-mcpd"), "{text}");
        assert!(text.contains(RUNTIME_DIR_ENV), "{text}");
        assert!(!text.contains('\n'), "one line: {text}");
    }
    let owned = RuntimeDirError::NotOwned {
        path,
        owner: 0,
        current: 501,
    }
    .to_string();
    assert!(
        owned.contains("uid 0") && owned.contains("uid 501"),
        "{owned}"
    );
}
