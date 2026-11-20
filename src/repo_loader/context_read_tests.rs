//! #602 S4a reader controls. Kinds: REAL = real files, real parse; SYNTHETIC = scripted reader,
//! injected recheck result or counters set on a parsed file (a threshold control, not a claim
//! about real grammar error ratios). No sleeps, no chmod, no cwd change, no races.

use super::context_read::{
    admit_path, check_path_state, finish, read, read_bounded, read_stream, ContextFile,
    ContextReadAttempt, ContextReadOutcome, ContextSkip, ReadAuthority,
};
use super::{MAX_FILE_BYTES as M, SEVERE_PARSE_ERROR_RATE};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use super::context_read::read_with;

const JS: &str = "var a = 1;";

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// Valid JavaScript padded with spaces to exactly `len` bytes (`len >= JS.len()`).
fn padded(len: u64) -> String {
    format!("{JS}{}", " ".repeat(len as usize - JS.len()))
}

fn outcome(attempt: &ContextReadAttempt) -> String {
    match &attempt.outcome {
        ContextReadOutcome::Loaded(file) => format!("Loaded({})", file.path),
        ContextReadOutcome::Skipped(skip) => format!("Skipped({skip:?})"),
        ContextReadOutcome::SourceCapacityBound => "Bound".to_string(),
    }
}

fn check(attempt: &ContextReadAttempt, bytes: u64, want: &str) {
    assert_eq!(
        (attempt.attempted_read_bytes, outcome(attempt)),
        (bytes, want.to_string())
    );
}

fn too_large(bytes: u64) -> String {
    format!("Skipped(TooLarge {{ observed_at_least: {bytes} }})")
}

fn loaded(attempt: ContextReadAttempt) -> ContextFile {
    match attempt.outcome {
        ContextReadOutcome::Loaded(file) => file,
        _ => panic!("expected a loaded file, got {}", outcome(&attempt)),
    }
}

fn kind_of(attempt: &ContextReadAttempt) -> ErrorKind {
    match &attempt.outcome {
        ContextReadOutcome::Skipped(ContextSkip::Unreadable { kind, .. }) => *kind,
        _ => panic!("expected Unreadable, got {}", outcome(attempt)),
    }
}

/// SYNTHETIC reader: short reads, an optional leading `Interrupted`, a hook that runs when the
/// bytes are exhausted, then EOF or an I/O error.
struct Script {
    data: Vec<u8>,
    pos: usize,
    interrupt_first: bool,
    then: Option<ErrorKind>,
    at_end: Option<Box<dyn FnOnce()>>,
}

fn script(data: &[u8]) -> Script {
    Script {
        data: data.to_vec(),
        pos: 0,
        interrupt_first: false,
        then: None,
        at_end: None,
    }
}

impl Read for Script {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if std::mem::take(&mut self.interrupt_first) {
            return Err(ErrorKind::Interrupted.into());
        }
        if self.pos == self.data.len() {
            if let Some(at_end) = self.at_end.take() {
                at_end();
            }
            return match self.then {
                Some(kind) => Err(kind.into()),
                None => Ok(0),
            };
        }
        let n = buf.len().min(self.data.len() - self.pos).min(4);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// REAL fixture: files under a canonical temp root, every named file listed, none original.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
    listing: BTreeSet<String>,
    original: BTreeSet<String>,
}

fn repo(files: &[(&str, &str)]) -> Repo {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    Repo {
        _dir: dir,
        listing: files.iter().map(|(name, _)| name.to_string()).collect(),
        original: BTreeSet::new(),
        root,
    }
}

impl Repo {
    fn authority(&self) -> ReadAuthority<'_> {
        ReadAuthority {
            lexical_root: &self.root,
            canonical_root: &self.root,
            listing: &self.listing,
            original: &self.original,
        }
    }

    fn read(&self, path: &str, remaining: u64) -> ContextReadAttempt {
        read(&self.authority(), path, remaining)
    }
}

fn clean() -> ParsedFile {
    ParsedFile::parse("a.js", JS, Language::JavaScript).unwrap()
}

/// SYNTHETIC counters on a real parse, to reach exact rates deterministically.
fn with_counts(errors: usize, nodes: usize) -> ParsedFile {
    let mut parsed = clean();
    assert_eq!(
        parsed.parse_error_count, 0,
        "inadmissible precondition: the base fixture must parse without errors"
    );
    parsed.parse_error_count = errors;
    parsed.parse_node_count = nodes;
    parsed
}

// A04
#[test]
fn lexical_table_refuses_every_non_canonical_spelling_before_any_filesystem_access() {
    let listing = set(&["x.js", "a/x.js", "dir/nested/y.ts", "ünï/日本.py"]);
    let original = BTreeSet::new();
    let missing = Path::new("/prism-context-read-missing-root");
    let authority = ReadAuthority {
        lexical_root: missing,
        canonical_root: missing,
        listing: &listing,
        original: &original,
    };
    let invalid = [
        "",
        "/",
        "/x.js",
        "../x.js",
        "a/../x.js",
        "./x.js",
        "a/./x.js",
        "a//x.js",
        "a/",
        "x.js/",
        "a\\x.js",
        "a\0x.js",
    ];
    let windows: &[&str] = if cfg!(windows) {
        &["C:x.js", "C:\\x.js"]
    } else {
        &[]
    };
    for spelling in invalid.iter().chain(windows) {
        assert_eq!(
            admit_path(&authority, spelling),
            Err(ContextSkip::InvalidPath),
            "{spelling:?}"
        );
        // The root does not exist: any filesystem access would give another disposition.
        check(&read(&authority, spelling, 1000), 0, "Skipped(InvalidPath)");
    }
    for (spelling, language) in [
        ("x.js", Language::JavaScript),
        ("a/x.js", Language::JavaScript),
        ("dir/nested/y.ts", Language::TypeScript),
        ("ünï/日本.py", Language::Python),
    ] {
        assert_eq!(admit_path(&authority, spelling), Ok(language), "{spelling}");
    }
    assert_eq!(
        admit_path(&authority, "b/x.js"),
        Err(ContextSkip::NotListed)
    );
}

// A08
#[test]
fn a_directory_or_a_missing_file_where_a_listed_file_was_is_a_zero_byte_skip() {
    let repo = repo(&[("dir.js", JS), ("gone.js", JS)]);
    for name in ["dir.js", "gone.js"] {
        std::fs::remove_file(repo.root.join(name)).unwrap();
    }
    std::fs::create_dir(repo.root.join("dir.js")).unwrap();
    assert!(
        !repo.root.join("gone.js").exists() && repo.root.join("dir.js").is_dir(),
        "inadmissible precondition: replacement state"
    );
    check(&repo.read("dir.js", 1000), 0, "Skipped(NotRegular)");
    let gone = repo.read("gone.js", 1000);
    assert_eq!(gone.attempted_read_bytes, 0);
    assert_eq!(kind_of(&gone), ErrorKind::NotFound);
}

// A10, SYNTHETIC refusal injected through the post-read seam
#[test]
fn a_post_read_refusal_discards_the_payload_and_keeps_the_attempted_count() {
    let refusals = [
        ContextSkip::UnsafePath,
        ContextSkip::NotRegular,
        ContextSkip::Unreadable {
            kind: ErrorKind::NotFound,
            message: "gone".to_string(),
        },
    ];
    for refusal in refusals {
        // Invalid UTF-8: had the payload reached the decoder, the result would be NotUtf8.
        let attempt = read_stream(
            script(&[0xff, 0xfe, 0xfd]),
            "a.js",
            Language::JavaScript,
            1000,
            || Err(refusal.clone()),
        );
        check(&attempt, 3, &format!("Skipped({refusal:?})"));
    }
    let accepted = read_stream(
        script(JS.as_bytes()),
        "a.js",
        Language::JavaScript,
        1000,
        || Ok(()),
    );
    check(&accepted, JS.len() as u64, "Loaded(a.js)");
}

// A10, REAL filesystem state changed while the bytes are being read
#[cfg(unix)]
#[test]
fn a_link_substituted_during_the_read_is_refused_by_the_post_read_check() {
    let repo = repo(&[("a.js", JS)]);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("evil.js"), "var evil = 1;").unwrap();
    let authority = repo.authority();
    assert!(
        check_path_state(&authority, "a.js").is_ok(),
        "inadmissible precondition: the pre-read check must pass"
    );
    let (target, evil) = (repo.root.join("a.js"), outside.path().join("evil.js"));
    let mut reader = script(JS.as_bytes());
    reader.at_end = Some(Box::new(move || {
        std::fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(&evil, &target).unwrap();
    }));
    let attempt = read_stream(reader, "a.js", Language::JavaScript, 1000, || {
        check_path_state(&authority, "a.js").map(drop)
    });
    check(&attempt, JS.len() as u64, "Skipped(UnsafePath)");
}

/// REAL opened file for `read_with`: `at_end` runs once, when the stream reports end of file.
#[cfg(unix)]
struct AtEnd {
    inner: std::fs::File,
    at_end: Option<Box<dyn FnOnce()>>,
}

#[cfg(unix)]
impl Read for AtEnd {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 && !buf.is_empty() {
            if let Some(at_end) = self.at_end.take() {
                at_end();
            }
        }
        Ok(n)
    }
}

/// Runs the production `read_with` (its own post-read closure) over the REAL file of `path`,
/// wrapped so that `at_end` runs when the bytes are exhausted. Also reports whether it ran.
#[cfg(unix)]
fn read_at_end(
    repo: &Repo,
    path: &str,
    at_end: impl FnOnce() + 'static,
) -> (ContextReadAttempt, bool) {
    let fired = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = std::rc::Rc::clone(&fired);
    let wrap = move |file: std::fs::File| AtEnd {
        inner: file,
        at_end: Some(Box::new(move || {
            seen.set(true);
            at_end();
        })),
    };
    let attempt = read_with(&repo.authority(), path, 1000, wrap);
    (attempt, fired.get())
}

// A10, REAL wiring: the closure inside `read_with` is the one under test, not a copy of it
#[cfg(unix)]
#[test]
fn read_refuses_a_link_substituted_at_end_of_stream_with_the_real_attempted_bytes() {
    let repo = repo(&[("a.js", JS)]);
    let outside = tempfile::tempdir().unwrap();
    let evil = outside.path().join("evil.js");
    std::fs::write(&evil, "var evil = 1;").unwrap();
    let target = repo.root.join("a.js");
    let swapped = target.clone();
    let (attempt, fired) = read_at_end(&repo, "a.js", move || {
        std::fs::remove_file(&swapped).unwrap();
        std::os::unix::fs::symlink(&evil, &swapped).unwrap();
    });
    assert!(
        fired,
        "inadmissible precondition: the swap must run at end of stream"
    );
    let meta = std::fs::symlink_metadata(&target).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "inadmissible precondition: the path must be a link after the read"
    );
    // The bytes of the file that was opened were read in full, then the path state refused.
    check(&attempt, JS.len() as u64, "Skipped(UnsafePath)");
}

// A10 control, REAL: the same wrapper and path with no swap, and the production identity read
#[cfg(unix)]
#[test]
fn read_loads_an_unchanged_path_through_the_same_wrapper_and_the_identity_read() {
    let repo = repo(&[("a.js", JS)]);
    let (attempt, fired) = read_at_end(&repo, "a.js", || {});
    assert!(
        fired,
        "inadmissible precondition: the wrapper must sit in the read path"
    );
    check(&attempt, JS.len() as u64, "Loaded(a.js)");
    check(&repo.read("a.js", 1000), JS.len() as u64, "Loaded(a.js)");
}

// B01
#[test]
fn per_file_edges_are_decided_by_the_bytes_returned() {
    // SYNTHETIC streams against the bounded reader: limit M, one sentinel byte.
    for (len, returned) in [
        (0, 0),
        (1, 1),
        (M - 1, M - 1),
        (M, M),
        (M + 1, M + 1),
        (M + 50, M + 1),
    ] {
        let (bytes, result) = read_bounded(std::io::repeat(b'a').take(len), M);
        assert!(result.is_ok());
        assert_eq!(bytes.len() as u64, returned, "stream of {len}");
    }
    let overflow = read_stream(
        std::io::repeat(b'a').take(M + 1),
        "a.js",
        Language::JavaScript,
        M + 100,
        || Ok(()),
    );
    check(&overflow, M + 1, &too_large(M + 1));
}

// B01, REAL source through the whole reader
#[test]
fn real_source_padded_to_exactly_the_per_file_limit_loads_and_one_byte_more_does_not() {
    let (exact, over) = (padded(M), padded(M + 1));
    let repo = repo(&[("exact.js", exact.as_str()), ("over.js", over.as_str())]);
    for (name, len) in [("exact.js", M), ("over.js", M + 1)] {
        assert_eq!(
            std::fs::metadata(repo.root.join(name)).unwrap().len(),
            len,
            "inadmissible precondition: {name} size"
        );
    }
    let attempt = repo.read("exact.js", M + 100);
    check(&attempt, M, "Loaded(exact.js)");
    assert_eq!(loaded(attempt).source.len() as u64, M);
    check(&repo.read("over.js", M + 100), M + 1, &too_large(M + 1));
}

// B02, SYNTHETIC streams
#[test]
fn equality_with_the_per_file_limit_is_a_per_file_skip_never_a_total_bound() {
    let stream = || std::io::repeat(b'a').take(M + 1);
    let first = read_stream(stream(), "a.js", Language::JavaScript, M, || Ok(()));
    check(&first, M + 1, &too_large(M + 1));
    // The same unspent capacity still admits a later small file.
    let next = read_stream(
        script(JS.as_bytes()),
        "b.js",
        Language::JavaScript,
        M,
        || Ok(()),
    );
    check(&next, JS.len() as u64, "Loaded(b.js)");
    // One byte below the limit the same stream is a total bound.
    let below = read_stream(stream(), "a.js", Language::JavaScript, M - 1, || Ok(()));
    check(&below, M, "Bound");
}

// B03, SYNTHETIC streams
#[test]
fn remaining_capacity_below_the_limit_bounds_after_exactly_one_sentinel_byte() {
    let remaining = 1000u64;
    for len in [remaining - 1, remaining] {
        let attempt = read_stream(
            script(padded(len).as_bytes()),
            "a.js",
            Language::JavaScript,
            remaining,
            || Ok(()),
        );
        check(&attempt, len, "Loaded(a.js)");
    }
    let one_over = read_stream(
        script(padded(remaining + 1).as_bytes()),
        "a.js",
        Language::JavaScript,
        remaining,
        || Ok(()),
    );
    check(&one_over, remaining + 1, "Bound");
    // A much longer stream is never read on to find out its size.
    let long = std::io::repeat(b'a').take(2 * M);
    let attempt = read_stream(long, "a.js", Language::JavaScript, remaining, || Ok(()));
    check(&attempt, remaining + 1, "Bound");
    let empty = read_stream(script(b""), "a.js", Language::JavaScript, 0, || Ok(()));
    check(&empty, 0, "Loaded(a.js)");
    let one = read_stream(script(b"a"), "a.js", Language::JavaScript, 0, || Ok(()));
    check(&one, 1, "Bound");
}

// B04, REAL files whose metadata disagrees with what is delivered
#[test]
fn metadata_length_never_decides_the_bytes_returned() {
    let repo = repo(&[("g.js", JS), ("h.js", JS)]);
    // Listed small, grown past the per-file limit before the read.
    std::fs::write(repo.root.join("g.js"), padded(M + 1)).unwrap();
    check(&repo.read("g.js", M + 100), M + 1, &too_large(M + 1));
    // Three MiB on disk, but only the remaining capacity plus the sentinel is returned.
    let big = padded(3 * 1024 * 1024);
    std::fs::write(repo.root.join("h.js"), &big).unwrap();
    assert!(
        std::fs::metadata(repo.root.join("h.js")).unwrap().len() > M,
        "inadmissible precondition: h.js must exceed the per-file limit"
    );
    check(&repo.read("h.js", 100), 101, "Bound");
}

// B05, SYNTHETIC scripted reader
#[test]
fn partial_and_early_io_errors_count_exactly_the_bytes_returned() {
    let mut partial = script(b"abc");
    partial.then = Some(ErrorKind::PermissionDenied);
    let attempt = read_stream(partial, "a.js", Language::JavaScript, 1000, || Ok(()));
    assert_eq!(attempt.attempted_read_bytes, 3);
    assert_eq!(kind_of(&attempt), ErrorKind::PermissionDenied);

    let mut early = script(b"");
    early.then = Some(ErrorKind::Other);
    let attempt = read_stream(early, "a.js", Language::JavaScript, 1000, || Ok(()));
    assert_eq!(attempt.attempted_read_bytes, 0);
    assert_eq!(kind_of(&attempt), ErrorKind::Other);

    let mut interrupted = script(JS.as_bytes());
    interrupted.interrupt_first = true;
    let attempt = read_stream(interrupted, "a.js", Language::JavaScript, 1000, || Ok(()));
    check(&attempt, JS.len() as u64, "Loaded(a.js)");

    // The error after the sentinel byte is never requested.
    let mut beyond = script(&[b'a'; 21]);
    beyond.then = Some(ErrorKind::Other);
    let (bytes, result) = read_bounded(beyond, 20);
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(bytes.len(), 21);
    let mut beyond = script(&[b'a'; 21]);
    beyond.then = Some(ErrorKind::Other);
    let attempt = read_stream(beyond, "a.js", Language::JavaScript, 20, || Ok(()));
    check(&attempt, 21, "Bound");
}

// B06, SYNTHETIC streams
#[test]
fn utf8_is_strict_counts_bytes_and_follows_the_overflow_decision() {
    let invalid = read_stream(
        script(&[0xff, 0xfe, b'a']),
        "a.js",
        Language::JavaScript,
        1000,
        || Ok(()),
    );
    check(&invalid, 3, "Skipped(NotUtf8)");
    let text = "var s = \"héllo wörld ✓\";\n";
    assert_ne!(
        text.len(),
        text.chars().count(),
        "inadmissible precondition: the text must be multibyte"
    );
    let multibyte = read_stream(
        script(text.as_bytes()),
        "a.js",
        Language::JavaScript,
        1000,
        || Ok(()),
    );
    check(&multibyte, text.len() as u64, "Loaded(a.js)");
    // Overflow is decided first: a truncated sentinel buffer is never decoded or parsed.
    let bound = read_stream(
        std::io::repeat(0xff).take(11),
        "a.js",
        Language::JavaScript,
        10,
        || Ok(()),
    );
    check(&bound, 11, "Bound");
    let long = std::io::repeat(0xff).take(M + 1);
    let skip = read_stream(long, "a.js", Language::JavaScript, M + 1, || Ok(()));
    check(&skip, M + 1, &too_large(M + 1));
}

// B07
#[test]
fn parse_gate_rejects_failures_and_severe_rates_but_admits_exactly_point_three() {
    let attempt = finish("a.js", JS.to_string(), Ok(clean()), 10);
    check(&attempt, 10, "Loaded(a.js)");
    let file = loaded(attempt);
    assert!(file.parse_quality.is_none() && file.parse_warnings.is_empty());
    assert_eq!(file.source, JS);

    // SYNTHETIC: a controlled parser failure.
    let failed = finish(
        "a.js",
        JS.to_string(),
        Err(anyhow::anyhow!("scripted failure")),
        7,
    );
    assert_eq!(failed.attempted_read_bytes, 7);
    match &failed.outcome {
        ContextReadOutcome::Skipped(ContextSkip::ParseFailed { message }) => {
            assert!(message.contains("scripted failure"), "{message}")
        }
        _ => panic!("expected ParseFailed, got {}", outcome(&failed)),
    }

    // SYNTHETIC counters.
    assert_eq!(
        with_counts(300, 1000).error_rate(),
        SEVERE_PARSE_ERROR_RATE,
        "inadmissible precondition: 300 of 1000 must be exactly the threshold"
    );
    for (errors, admitted) in [(299, true), (300, true), (301, false)] {
        let attempt = finish("a.js", JS.to_string(), Ok(with_counts(errors, 1000)), 5);
        assert_eq!(attempt.attempted_read_bytes, 5);
        let want = if admitted {
            "Loaded(a.js)".to_string()
        } else {
            format!("Skipped(SevereParse {{ error_count: {errors}, node_count: 1000 }})")
        };
        assert_eq!(outcome(&attempt), want, "{errors} of 1000");
    }
}

// B08, SYNTHETIC counters
#[test]
fn quality_and_warnings_follow_the_existing_grader_and_severe_parses_stay_out() {
    for (errors, quality, warned) in [
        (5, None, false),
        (50, Some("degraded"), false),
        (150, Some("poor"), true),
        (300, Some("poor"), true),
    ] {
        let file = loaded(finish(
            "a.js",
            JS.to_string(),
            Ok(with_counts(errors, 1000)),
            1,
        ));
        let graded = file.parse_quality.as_ref();
        assert_eq!(graded.map(|q| q.quality.as_str()), quality, "{errors}");
        if let Some(graded) = graded {
            assert_eq!((graded.error_count, graded.node_count), (errors, 1000));
        }
        assert_eq!(!file.parse_warnings.is_empty(), warned, "{errors}");
        if warned {
            let want = format!("File a.js has {errors} parse errors");
            assert!(
                file.parse_warnings[0].contains(&want),
                "{:?}",
                file.parse_warnings
            );
        }
    }
    // The grader alone would only warn about a severe parse; the reader keeps it out.
    let files = std::collections::BTreeMap::from([("a.js".to_string(), with_counts(301, 1000))]);
    let (warnings, quality) = crate::ast::parse_quality::check_parse_quality(&files);
    assert_eq!(quality["a.js"].quality, "unparseable");
    assert!(warnings[0].contains("severe parse errors"), "{warnings:?}");
    let severe = finish("a.js", JS.to_string(), Ok(with_counts(301, 1000)), 1);
    assert!(
        outcome(&severe).starts_with("Skipped(SevereParse"),
        "{}",
        outcome(&severe)
    );
}
