//! Tests for the identity key: what goes into it, that the encoding loses nothing, and that
//! one configuration has one key however it is spelled.
use super::*;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, StartupMode};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn sample() -> DaemonIdentity {
    DaemonIdentity {
        repo_root: PathBuf::from("/work/repo"),
        root_fs: FsIdentity {
            dev: 16_777_233,
            ino: 4_242,
        },
        build: BuildIdentity {
            package_version: "3.1.2".into(),
            grammar_fingerprint: "grammars-v1".into(),
            cache_build_identity: "build-aaaa".into(),
        },
        protocol: 1,
        cache_base: PathBuf::from("/home/u/.cache"),
        refresh_policy: RefreshPolicy::WarnOnly,
    }
}

/// A decoder written from the format's description, not from the encoder: a sequence of
/// fields, each an 8-byte little-endian length followed by that many bytes.
fn decode_fields(mut rest: &[u8]) -> Vec<Vec<u8>> {
    let mut fields = Vec::new();
    while !rest.is_empty() {
        let (length, tail) = rest.split_at(8);
        let length = u64::from_le_bytes(length.try_into().unwrap()) as usize;
        let (field, tail) = tail.split_at(length);
        fields.push(field.to_vec());
        rest = tail;
    }
    fields
}

fn expected_fields(identity: &DaemonIdentity) -> Vec<Vec<u8>> {
    vec![
        b"prism-mcpd identity".to_vec(),
        identity.repo_root.as_os_str().as_bytes().to_vec(),
        identity.root_fs.dev.to_le_bytes().to_vec(),
        identity.root_fs.ino.to_le_bytes().to_vec(),
        identity.build.package_version.as_bytes().to_vec(),
        identity.build.grammar_fingerprint.as_bytes().to_vec(),
        identity.build.cache_build_identity.as_bytes().to_vec(),
        identity.protocol.to_le_bytes().to_vec(),
        identity.cache_base.as_os_str().as_bytes().to_vec(),
        refresh_policy_name(identity.refresh_policy)
            .as_bytes()
            .to_vec(),
    ]
}

// ── The encoding and the hash ──

/// The shim and the daemon are separate processes that must compute the same key, so the key
/// is a fixed function of its inputs. Rules out a per-process hasher (`RandomState`), a
/// non-cryptographic one, and any accidental change of field order or width. The expected
/// digest was computed outside this crate:
///
/// ```text
/// import hashlib, struct
/// field = lambda b: struct.pack('<Q', len(b)) + b
/// fields = [b'prism-mcpd identity', b'/work/repo', struct.pack('<Q', 16777233),
///           struct.pack('<Q', 4242), b'3.1.2', b'grammars-v1', b'build-aaaa',
///           struct.pack('<I', 1), b'/home/u/.cache', b'warn-only']
/// print(hashlib.sha256(b''.join(map(field, fields))).hexdigest())
/// ```
#[test]
fn the_key_is_sha256_of_the_length_prefixed_fields() {
    assert_eq!(
        sample().key().to_hex(),
        "70434641ffa9426029dab705875d9c4ad19cba621e148bf560bc8a060e0a88d8"
    );
}

/// Spec §4.3 "lossless": every input can be read back out of the encoding, so two different
/// inputs cannot encode alike. The second identity uses bytes a lossy conversion would
/// destroy: invalid UTF-8, a NUL, a newline, and empty strings.
#[test]
fn every_input_can_be_read_back_from_the_encoding() {
    let nasty = DaemonIdentity {
        repo_root: PathBuf::from(OsStr::from_bytes(b"/work/\xff\xfe/re\0po\n")),
        root_fs: FsIdentity {
            dev: u64::MAX,
            ino: 0,
        },
        build: BuildIdentity {
            package_version: String::new(),
            grammar_fingerprint: String::new(),
            cache_build_identity: "x".into(),
        },
        protocol: u32::MAX,
        cache_base: PathBuf::from(OsStr::from_bytes(b"/c\xc3\x28")),
        refresh_policy: RefreshPolicy::AutoIncremental,
    };
    for identity in [sample(), nasty] {
        assert_eq!(
            decode_fields(&identity.encode()),
            expected_fields(&identity)
        );
    }
}

/// Rules out concatenating the inputs without lengths: both identities below join to the same
/// bytes.
#[test]
fn moving_bytes_across_a_field_boundary_changes_the_key() {
    let mut left = sample();
    left.build.package_version = "3.1".into();
    left.build.grammar_fingerprint = "2grammars".into();
    let mut right = sample();
    right.build.package_version = "3.12".into();
    right.build.grammar_fingerprint = "grammars".into();
    assert_ne!(left.key(), right.key());

    let mut left = sample();
    left.repo_root = PathBuf::from("/a/b");
    left.cache_base = PathBuf::from("/c");
    let mut right = sample();
    right.repo_root = PathBuf::from("/a");
    right.cache_base = PathBuf::from("/b/c");
    assert_ne!(left.key(), right.key());
}

/// Rules out `to_string_lossy`: both roots become the same replacement character.
#[test]
fn roots_that_differ_only_in_invalid_utf8_bytes_have_different_keys() {
    let mut left = sample();
    left.repo_root = PathBuf::from(OsStr::from_bytes(b"/work/\xff"));
    let mut right = sample();
    right.repo_root = PathBuf::from(OsStr::from_bytes(b"/work/\xfe"));
    assert_ne!(left.key(), right.key());
}

/// Spec §4.3 and §9: each input changes the key. One change per row; every row must give a
/// key no other row gives. Rules out an encoder that forgets a field.
#[test]
fn every_input_changes_the_key() {
    type Change = (&'static str, fn(&mut DaemonIdentity));
    let changes: Vec<Change> = vec![
        ("repo root", |i| i.repo_root = PathBuf::from("/work/repo2")),
        ("root device", |i| i.root_fs.dev += 1),
        ("root inode", |i| i.root_fs.ino += 1),
        ("package version", |i| {
            i.build.package_version = "3.1.3".into()
        }),
        ("grammar fingerprint", |i| {
            i.build.grammar_fingerprint = "grammars-v2".into()
        }),
        ("analyzer build identity", |i| {
            i.build.cache_build_identity = "build-bbbb".into()
        }),
        ("protocol version", |i| i.protocol = 2),
        ("cache base", |i| {
            i.cache_base = PathBuf::from("/home/u/.cache2")
        }),
        ("refresh policy auto-full", |i| {
            i.refresh_policy = RefreshPolicy::AutoFull
        }),
        ("refresh policy auto-incremental", |i| {
            i.refresh_policy = RefreshPolicy::AutoIncremental
        }),
    ];
    let mut keys = BTreeSet::new();
    keys.insert(sample().key().to_hex());
    for (name, change) in &changes {
        let mut identity = sample();
        change(&mut identity);
        assert!(
            keys.insert(identity.key().to_hex()),
            "changing the {name} did not give a new key"
        );
    }
    assert_eq!(keys.len(), changes.len() + 1);
}

/// Rules out combining fields in an order-blind way (a sum or an XOR of per-field hashes).
#[test]
fn swapping_two_inputs_changes_the_key() {
    let mut swapped = sample();
    std::mem::swap(&mut swapped.root_fs.dev, &mut swapped.root_fs.ino);
    assert_ne!(swapped.key(), sample().key());

    let mut swapped = sample();
    std::mem::swap(&mut swapped.repo_root, &mut swapped.cache_base);
    assert_ne!(swapped.key(), sample().key());
}

#[test]
fn the_current_build_is_the_three_values_the_spec_names() {
    let build = BuildIdentity::current();
    assert_eq!(build.package_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(build.grammar_fingerprint, env!("GRAMMAR_FINGERPRINT"));
    assert_eq!(
        build.cache_build_identity,
        env!("PRISM_CACHE_BUILD_IDENTITY")
    );
    assert!(!build.cache_build_identity.is_empty());
}

#[test]
fn refresh_policies_are_keyed_by_their_command_line_names() {
    assert_eq!(refresh_policy_name(RefreshPolicy::WarnOnly), "warn-only");
    assert_eq!(refresh_policy_name(RefreshPolicy::AutoFull), "auto-full");
    assert_eq!(
        refresh_policy_name(RefreshPolicy::AutoIncremental),
        "auto-incremental"
    );
}

// ── The key as text ──

#[test]
fn a_key_prints_as_64_lowercase_hex_digits_and_reads_back() {
    let key = sample().key();
    let hex = key.to_hex();
    assert_eq!(hex.len(), 64);
    assert!(hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    assert_eq!(key.to_string(), hex);
    assert_eq!(IdentityKey::from_hex(&hex).unwrap(), key);
    assert_eq!(key.locator(), hex[..16]);
}

/// One key has one spelling, so comparing keys is comparing bytes.
#[test]
fn a_malformed_key_is_not_a_key() {
    let hex = sample().key().to_hex();
    let padded = format!(" {}", &hex[1..]);
    let non_ascii = format!("{}é", &hex[..62]);
    assert_eq!(non_ascii.len(), 64, "64 bytes, 63 characters");
    for bad in [
        "",
        &hex[..63],
        &format!("{hex}0"),
        &hex.to_uppercase(),
        &hex.replacen(|c: char| c.is_ascii_hexdigit(), "g", 1),
        &hex[..16],
        &padded,
        &non_ascii,
        &format!("+f{}", &hex[2..]),
    ] {
        assert_eq!(
            IdentityKey::from_hex(bad),
            Err(InvalidIdentityKey),
            "{bad:?}"
        );
    }
}

// ── Resolving an identity from a configuration ──

struct Checkout {
    _guard: tempfile::TempDir,
    /// The temp directory as `tempfile` names it (on macOS, through the `/var` symlink).
    as_given: PathBuf,
    /// The same directory by its canonical path.
    real: PathBuf,
}

/// A temp directory holding `repo/`, `repo/sub/`, `cache/` and the symlink `repo-link -> repo`.
fn checkout() -> Checkout {
    let guard = tempfile::tempdir().unwrap();
    let as_given = guard.path().to_path_buf();
    let real = std::fs::canonicalize(&as_given).unwrap();
    std::fs::create_dir_all(real.join("repo/sub")).unwrap();
    std::fs::create_dir(real.join("cache")).unwrap();
    std::os::unix::fs::symlink("repo", real.join("repo-link")).unwrap();
    Checkout {
        _guard: guard,
        as_given,
        real,
    }
}

fn config(root: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> ServerConfig {
    let mut cfg = ServerConfig::new(root.into());
    cfg.cache = CacheMode::Dir(cache.into());
    cfg
}

fn key_of(cfg: &ServerConfig) -> IdentityKey {
    DaemonIdentity::resolve(cfg).unwrap().key()
}

fn other_build() -> BuildIdentity {
    let mut build = BuildIdentity::current();
    build.cache_build_identity.push_str("-rebuilt");
    build
}

#[test]
fn resolving_reads_the_root_the_cache_base_the_policy_and_this_build() {
    let checkout = checkout();
    let mut cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    cfg.refresh_policy = RefreshPolicy::AutoFull;
    let identity = DaemonIdentity::resolve(&cfg).unwrap();

    assert_eq!(identity.repo_root(), checkout.real.join("repo"));
    assert_eq!(identity.cache_base(), checkout.real.join("cache"));
    assert_eq!(identity.refresh_policy(), RefreshPolicy::AutoFull);
    assert_eq!(identity.build(), &BuildIdentity::current());
    assert_eq!(
        identity.protocol(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION
    );
    assert_eq!(
        identity.root_fs(),
        FsIdentity::of(&checkout.real.join("repo")).unwrap()
    );
    let metadata = std::fs::metadata(checkout.real.join("repo")).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        identity.root_fs(),
        FsIdentity {
            dev: metadata.dev(),
            ino: metadata.ino()
        }
    );
}

/// Spec §4.1 step 1: the root is canonical "exactly as today".
#[test]
fn the_root_is_spelled_as_the_in_process_server_spells_it() {
    let checkout = checkout();
    let cfg = config(
        checkout.as_given.join("repo-link/sub/.."),
        checkout.real.join("cache"),
    );
    let canonical = crate::mcp::session::canonical_config(&cfg).unwrap();
    assert_eq!(
        DaemonIdentity::resolve(&cfg).unwrap().repo_root(),
        canonical.repo_root
    );
}

/// The same inputs spelled differently give the same key.
#[test]
fn one_checkout_and_one_cache_base_have_one_key_however_they_are_spelled() {
    let checkout = checkout();
    let reference = key_of(&config(
        checkout.real.join("repo"),
        checkout.real.join("cache"),
    ));

    let mut trailing_slash = checkout.real.join("repo").into_os_string();
    trailing_slash.push("/");
    let roots = [
        PathBuf::from(trailing_slash),
        checkout.real.join("repo/."),
        checkout.real.join("repo/sub/.."),
        checkout.real.join("repo-link"),
        checkout.as_given.join("repo"),
    ];
    for root in &roots {
        assert_eq!(
            key_of(&config(root, checkout.real.join("cache"))),
            reference,
            "root spelled {}",
            root.display()
        );
    }

    std::os::unix::fs::symlink("cache", checkout.real.join("cache-link")).unwrap();
    let bases = [
        checkout.real.join("cache/"),
        checkout.real.join("cache/."),
        checkout.real.join("cache/not-there/.."),
        checkout.real.join("cache-link"),
        checkout.as_given.join("cache"),
    ];
    for base in &bases {
        assert_eq!(
            key_of(&config(checkout.real.join("repo"), base)),
            reference,
            "cache base spelled {}",
            base.display()
        );
    }
}

/// Two checkouts, two cache bases, two builds, two protocol versions and two refresh policies
/// each give two keys. This is what keeps one index from answering for another checkout,
/// another analyzer build or another cache.
#[test]
fn each_configuration_difference_gives_a_different_key() {
    let checkout = checkout();
    std::fs::create_dir(checkout.real.join("repo2")).unwrap();
    let base_cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    let reference = key_of(&base_cfg);

    let other_checkout = config(checkout.real.join("repo2"), checkout.real.join("cache"));
    assert_ne!(key_of(&other_checkout), reference, "two checkouts");

    // A worktree nested in the checkout is its own checkout (spec §2).
    let nested = config(checkout.real.join("repo/sub"), checkout.real.join("cache"));
    assert_ne!(
        key_of(&nested),
        reference,
        "a directory inside the checkout"
    );

    let other_cache = config(checkout.real.join("repo"), checkout.real.join("cache2"));
    assert_ne!(key_of(&other_cache), reference, "two cache bases");

    for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
        let mut other_policy = base_cfg.clone();
        other_policy.refresh_policy = policy;
        assert_ne!(key_of(&other_policy), reference, "{policy:?}");
    }

    let rebuilt = DaemonIdentity::resolve_with(
        &base_cfg,
        other_build(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION,
    )
    .unwrap();
    assert_ne!(rebuilt.key(), reference, "two builds");

    let next_protocol = DaemonIdentity::resolve_with(
        &base_cfg,
        BuildIdentity::current(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION + 1,
    )
    .unwrap();
    assert_ne!(next_protocol.key(), reference, "two protocol versions");

    let same_again = DaemonIdentity::resolve_with(
        &base_cfg,
        BuildIdentity::current(),
        crate::mcp::shared::SHARED_PROTOCOL_VERSION,
    )
    .unwrap();
    assert_eq!(same_again.key(), reference, "and the same inputs agree");
}

/// Spec §4.3 "excluded by design" and §9 "`--first-call-wait` does not": connection-local
/// settings travel in the preamble. Rules out hashing the whole `ServerConfig`.
#[test]
fn connection_local_settings_do_not_change_the_key() {
    let checkout = checkout();
    let base_cfg = config(checkout.real.join("repo"), checkout.real.join("cache"));
    let reference = key_of(&base_cfg);

    let mut patient = base_cfg.clone();
    patient.first_call_wait = Duration::from_secs(600);
    assert_eq!(key_of(&patient), reference, "--first-call-wait");

    let mut warm = base_cfg.clone();
    warm.startup = StartupMode::Background;
    assert_eq!(key_of(&warm), reference, "--warm-at-startup");
}

/// Spec §4.3 and §4.6: a root that is renamed away and replaced is a different checkout at the
/// same pathname. Rules out a key made of the path alone.
#[test]
fn a_root_replaced_at_the_same_pathname_has_a_new_key() {
    let checkout = checkout();
    let root = checkout.real.join("repo");
    let cfg = config(&root, checkout.real.join("cache"));
    let before = DaemonIdentity::resolve(&cfg).unwrap();

    // Keep the old directory alive so its inode cannot be reused for the new one.
    std::fs::rename(&root, checkout.real.join("repo-moved")).unwrap();
    assert!(
        FsIdentity::of(&root).is_err(),
        "a missing root has no filesystem identity"
    );
    std::fs::create_dir(&root).unwrap();
    let after = DaemonIdentity::resolve(&cfg).unwrap();

    assert_eq!(before.repo_root(), after.repo_root());
    assert_ne!(before.root_fs(), after.root_fs());
    assert_ne!(before.key(), after.key());
    assert_eq!(
        FsIdentity::of(&checkout.real.join("repo-moved")).unwrap(),
        before.root_fs(),
        "the moved directory keeps its identity"
    );
}

/// Spec §9: "cache-base normalization (Default vs. explicit equivalent)".
#[test]
fn the_default_cache_and_the_same_directory_given_explicitly_have_one_key() {
    let checkout = checkout();
    let by_default = ServerConfig::new(checkout.real.join("repo"));
    let explicit = config(
        checkout.real.join("repo"),
        crate::mcp::shared::cache_base::default_cache_base(),
    );
    assert_eq!(key_of(&by_default), key_of(&explicit));
}

/// The shim starts the daemon with the canonical root, the canonical cache base and the
/// policy; the daemon resolves its own identity from those arguments. If the two disagreed,
/// every `HELLO` would be answered `MISMATCH`.
#[test]
fn a_daemon_started_from_an_identity_s_own_values_computes_the_same_key() {
    let checkout = checkout();
    std::os::unix::fs::symlink("cache", checkout.real.join("cache-link")).unwrap();
    let mut shim_cfg = config(
        checkout.as_given.join("repo-link/sub/.."),
        checkout.as_given.join("cache-link/later/.."),
    );
    shim_cfg.refresh_policy = RefreshPolicy::AutoIncremental;
    shim_cfg.startup = StartupMode::Background;
    shim_cfg.first_call_wait = Duration::from_secs(5);

    for shim_cfg in [shim_cfg, ServerConfig::new(checkout.as_given.join("repo"))] {
        let shim = DaemonIdentity::resolve(&shim_cfg).unwrap();
        let mut daemon_cfg = config(shim.repo_root(), shim.cache_base());
        daemon_cfg.refresh_policy = shim.refresh_policy();
        let daemon = DaemonIdentity::resolve(&daemon_cfg).unwrap();
        assert_eq!(daemon, shim);
        assert_eq!(daemon.key(), shim.key());
    }
}

#[test]
fn a_cache_free_configuration_has_no_identity() {
    let checkout = checkout();
    let mut cfg = ServerConfig::new(checkout.real.join("repo"));
    cfg.cache = CacheMode::NoCache;
    let outcome = DaemonIdentity::resolve(&cfg);
    assert!(
        matches!(
            outcome,
            Err(IdentityError::CacheBase(
                crate::mcp::shared::cache_base::CacheBaseError::NoCache
            ))
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_missing_root_is_an_error_that_names_it() {
    let checkout = checkout();
    let cfg = config(
        checkout.real.join("no-such-repo"),
        checkout.real.join("cache"),
    );
    let outcome = DaemonIdentity::resolve(&cfg);
    match outcome {
        Err(IdentityError::RepoRoot { path, source }) => {
            assert_eq!(path, checkout.real.join("no-such-repo"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("{other:?}"),
    }
}

/// Spec §9 test 5 expects a failed start to leave "no cache dir created anywhere"; resolving
/// the identity is the first thing a shim does, so it must create nothing.
#[test]
fn resolving_an_identity_creates_nothing() {
    let checkout = checkout();
    let cache = checkout.real.join("cache/not/made/yet");
    let cfg = config(checkout.real.join("repo"), &cache);
    let before = listing(&checkout.real);
    let identity = DaemonIdentity::resolve(&cfg).unwrap();
    assert_eq!(identity.cache_base(), cache);
    assert_eq!(listing(&checkout.real), before);
    assert!(!checkout.real.join("cache/not").exists());
}

/// Every path under `root`, without following symlinks.
fn listing(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            }
            found.insert(entry.path());
        }
    }
    found
}
