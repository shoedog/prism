//! Tolerant repository ambient scan; direct external type inputs still decline.
use super::*;

// Read-only volume probe: compare a known entry with an alternate ASCII case.
// If neither spelling gives a discriminating observation, the caller assumes
// case-insensitivity. No probe file or Git mutation is necessary.
pub(super) fn case_insensitive(root: &Path) -> Option<bool> {
    // Probe an entry on the root's volume, rather than the root's name in its
    // parent: a mount point can have different case semantics from its parent.
    let entry = std::fs::read_dir(root).ok()?.find_map(|entry| {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_str()?;
        (entry.file_type().ok()?.is_file() && name.bytes().any(|b| b.is_ascii_alphabetic()))
            .then_some(entry)
    })?;
    let original = entry.metadata().ok()?;
    let name = entry.file_name();
    let name = name.to_str()?;
    let mut alternate = name.to_owned();
    let (index, byte) = name
        .bytes()
        .enumerate()
        .find(|(_, b)| b.is_ascii_alphabetic())?;
    alternate.replace_range(index..index + 1, &char::from(byte ^ 32).to_string());
    match std::fs::metadata(root.join(alternate)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(false),
        Ok(other) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                (original.dev() == other.dev() && original.ino() == other.ino()).then_some(true)
            }
            #[cfg(not(unix))]
            {
                let _ = (original, other);
                None
            }
        }
        _ => None,
    }
}

#[path = "js_paths_case_fold.rs"]
mod case_fold;
pub(super) fn fold(s: &str) -> String {
    s.chars().map(case_fold::simple).collect()
}

fn excluded(path: &Path, case_insensitive: bool) -> bool {
    let components: Vec<_> = path.components().map(|c| c.as_os_str()).collect();
    let equal = |name: &std::ffi::OsStr, expected: &str| {
        name == expected
            || (case_insensitive
                && name
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(expected)))
    };
    components.iter().any(|c| equal(c, ".git"))
        || components
            .windows(2)
            .any(|c| equal(c[0], "node_modules") && equal(c[1], ".bin"))
}

enum Event {
    Skip(&'static str),
    Entry,
    TypeEntry(String, u8),
    Link(String, Option<String>),
    Covered(String),
    Package {
        rel: String,
        path: std::path::PathBuf,
        is_file: bool,
    },
    Source(usize),
}

struct SourceJob {
    rel: String,
    path: std::path::PathBuf,
    len: u64,
}

struct Scanned {
    read: Option<Read>,
}
struct Read {
    len: u64,
    text: Option<Text>,
}
struct Text {
    declares: bool,
    candidate: bool,
    // Present exactly when the sequential scan hashed the file.
    digest: Option<String>,
    references: Vec<(String, String)>,
}
struct ScanResults {
    files: Vec<Scanned>,
    parsed: BTreeMap<String, (Vec<String>, usize)>,
}

// Candidates above this size parse one at a time; smaller trees may be live
// on every worker at once.
const INLINE_PARSE_BYTES: u64 = 256 * 1024;

// Read, prefilter, hash and parse every source job. Each result is a pure
// function of the file bytes; identical bytes share one parse by digest.
// Small candidates parse on any worker; large ones take the `large` lock so
// at most one large tree is live, which bounds the scan's memory high-water
// mark at the sequential scan's single-tree peak.
fn scan_sources(jobs: &[SourceJob]) -> ScanResults {
    use rayon::prelude::*;
    let parsed: Mutex<BTreeMap<String, (Vec<String>, usize)>> = Mutex::new(BTreeMap::new());
    let large = Mutex::new(());
    let files: Vec<Scanned> = jobs
        .par_iter()
        .map(|job| {
            use std::io::Read as _;
            let mut bytes = Vec::with_capacity(job.len.min(SCAN_BYTES + 1) as usize);
            let read = std::fs::File::open(&job.path)
                .and_then(|f| f.take(SCAN_BYTES + 1).read_to_end(&mut bytes));
            if read.is_err() {
                return Scanned { read: None };
            }
            let len = bytes.len() as u64;
            let Ok(source) = std::str::from_utf8(&bytes) else {
                return Scanned {
                    read: Some(Read { len, text: None }),
                };
            };
            let declares = bytes.windows(7).any(|w| w == b"declare");
            let candidate = declares && super::ambient_candidate(source);
            let references = super::triple_references(source);
            let digest = (declares || !references.is_empty())
                .then(|| format!("{:x}", Sha256::digest(&bytes)));
            if candidate {
                let digest = digest.as_ref().expect("candidates declare");
                let _one_large_tree = (len > INLINE_PARSE_BYTES).then(|| large.lock().unwrap());
                if !parsed.lock().unwrap().contains_key(digest) {
                    let result = super::ambient_patterns(source);
                    parsed
                        .lock()
                        .unwrap()
                        .entry(digest.clone())
                        .or_insert(result);
                }
            }
            Scanned {
                read: Some(Read {
                    len,
                    text: Some(Text {
                        declares,
                        candidate,
                        digest,
                        references,
                    }),
                }),
            }
        })
        .collect();
    ScanResults {
        files,
        parsed: parsed.into_inner().unwrap(),
    }
}

impl JsPathsSnapshot {
    fn skip(&mut self, reason: &str) {
        *self.scan_stats.skipped.entry(reason.into()).or_default() += 1;
    }
    pub(super) fn scan(
        &mut self,
        root: &Path,
        directory: &Path,
        _depth: usize,
    ) -> std::io::Result<()> {
        // The traversal records every bookkeeping effect in order and reads
        // nothing it would write. Source files are then read, prefiltered,
        // hashed and parsed off the traversal, and the journal is replayed in
        // traversal order, so every map, counter and the byte budget stop at
        // exactly the entry the sequential scan stopped at.
        let (events, jobs, traversal) = self.traverse(root, directory);
        let results = scan_sources(&jobs);
        self.replay(events, jobs, results)?;
        traversal
    }

    fn traverse(
        &self,
        root: &Path,
        directory: &Path,
    ) -> (Vec<Event>, Vec<SourceJob>, std::io::Result<()>) {
        let mut events = Vec::new();
        let mut jobs = Vec::new();
        let mut pending = vec![directory.to_path_buf()];
        let mut seen = BTreeSet::new();
        while let Some(mut path) = pending.pop() {
            let relative = match path.strip_prefix(root) {
                Ok(relative) => relative,
                Err(error) => return (events, jobs, Err(std::io::Error::other(error))),
            };
            if excluded(relative, self.case_insensitive) {
                events.push(Event::Skip("excluded"));
                continue;
            }
            let Some(relative) = relative.to_str() else {
                events.push(Event::Skip("non_utf8_path"));
                continue;
            };
            let mut rel = relative.to_owned();
            let Ok(mut metadata) = std::fs::symlink_metadata(&path) else {
                events.push(Event::Skip("unreadable"));
                continue;
            };
            if metadata.is_symlink() {
                events.push(Event::TypeEntry(rel.clone(), 2));
                let target = match path.canonicalize() {
                    Ok(target) if target.starts_with(root) => Some(target),
                    Ok(_) => {
                        events.push(Event::Skip("outside_root_symlink"));
                        None
                    }
                    Err(_) => {
                        events.push(Event::Skip("unreadable"));
                        None
                    }
                };
                events.push(Event::Link(
                    rel.clone(),
                    target
                        .as_ref()
                        .and_then(|p| p.strip_prefix(root).ok()?.to_str().map(str::to_owned)),
                ));
                let Some(target) = target else {
                    continue;
                };
                let target_relative = match target.strip_prefix(root) {
                    Ok(relative) => relative,
                    Err(error) => return (events, jobs, Err(std::io::Error::other(error))),
                };
                if excluded(target_relative, self.case_insensitive) {
                    events.push(Event::Skip("excluded"));
                    continue;
                }
                path = target;
                let Some(target_rel) = path.strip_prefix(root).ok().and_then(Path::to_str) else {
                    events.push(Event::Skip("non_utf8_path"));
                    continue;
                };
                rel = target_rel.to_owned();
                let Ok(target_metadata) = std::fs::symlink_metadata(&path) else {
                    events.push(Event::Skip("unreadable"));
                    continue;
                };
                metadata = target_metadata;
            }
            // Canonical paths dedupe links and cycles without duplicate reads.
            if !seen.insert(path.clone()) {
                continue;
            }
            events.push(Event::Entry);
            let kind = if metadata.is_dir() {
                1
            } else if metadata.is_file() {
                0
            } else {
                2
            };
            // The indexing inventory already owns most source/directory paths.
            if self.entries.get(&rel) != Some(&kind) {
                events.push(Event::TypeEntry(rel.clone(), kind));
            }
            if metadata.is_dir() {
                let Ok(entries) = std::fs::read_dir(&path) else {
                    events.push(Event::Skip("unreadable"));
                    continue;
                };
                for entry in entries {
                    match entry {
                        Ok(entry) => pending.push(entry.path()),
                        Err(_) => events.push(Event::Skip("unreadable")),
                    }
                }
                events.push(Event::Covered(rel));
                continue;
            }
            if path.file_name().is_some_and(|n| n == "package.json") {
                events.push(Event::Package {
                    rel: rel.clone(),
                    path: path.clone(),
                    is_file: metadata.is_file(),
                });
            }
            let lower = rel.to_ascii_lowercase();
            let source_file = [".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"]
                .iter()
                .any(|e| lower.ends_with(e));
            if !source_file {
                events.push(Event::Skip("non_source"));
                continue;
            }
            if !metadata.is_file() {
                events.push(Event::Skip("non_regular_source"));
                continue;
            }
            events.push(Event::Source(jobs.len()));
            jobs.push(SourceJob {
                rel,
                path,
                len: metadata.len(),
            });
        }
        (events, jobs, Ok(()))
    }

    fn replay(
        &mut self,
        events: Vec<Event>,
        jobs: Vec<SourceJob>,
        results: ScanResults,
    ) -> std::io::Result<()> {
        for event in events {
            match event {
                Event::Skip(reason) => self.skip(reason),
                Event::Entry => self.scan_stats.entries += 1,
                Event::TypeEntry(rel, kind) => {
                    self.type_entries.insert(rel, kind);
                }
                Event::Link(rel, target) => {
                    self.links.insert(rel, target);
                }
                Event::Covered(rel) => {
                    self.covered.insert(rel, 1);
                }
                Event::Package { rel, path, is_file } => {
                    self.replay_package(rel, &path, is_file)?
                }
                Event::Source(index) => {
                    let job = &jobs[index];
                    let scanned = &results.files[index];
                    self.replay_source(job, scanned, &results.parsed)?;
                }
            }
        }
        Ok(())
    }

    fn replay_package(&mut self, rel: String, path: &Path, is_file: bool) -> std::io::Result<()> {
        use std::io::Read;
        let bytes = if is_file {
            std::fs::File::open(path).ok().and_then(|f| {
                let mut bytes = Vec::new();
                f.take(262_145).read_to_end(&mut bytes).ok()?;
                (bytes.len() <= 262_144).then_some(bytes)
            })
        } else {
            None
        };
        if let Some(bytes) = &bytes {
            self.type_metadata_bytes += bytes.len() as u64;
            if self.type_metadata_bytes > SCAN_BYTES {
                return Err(std::io::Error::other("P1 ambient scan byte budget"));
            }
        } else {
            self.skip("unreadable");
        }
        self.package_hashes.insert(
            format!("type-package:{rel}"),
            bytes
                .as_ref()
                .map(|b| format!("{:x}", Sha256::digest(b)))
                .unwrap_or_else(|| "opaque".into()),
        );
        self.type_redirects.insert(
            rel.clone(),
            bytes.as_ref().and_then(|b| super::type_redirects(b)),
        );
        self.module_packages.insert(rel, bytes);
        Ok(())
    }

    fn replay_source(
        &mut self,
        job: &SourceJob,
        scanned: &Scanned,
        parsed: &BTreeMap<String, (Vec<String>, usize)>,
    ) -> std::io::Result<()> {
        let rel = &job.rel;
        // File occupancy classifies direct inputs; scan hashes bind read
        // dependencies. Only traversed directories need coverage records.
        let remaining = SCAN_BYTES.saturating_sub(self.scan_stats.bytes);
        if job.len > remaining {
            return Err(std::io::Error::other("P1 ambient scan byte budget"));
        }
        let Some(read) = &scanned.read else {
            self.skip("unreadable");
            return Ok(());
        };
        if read.len > remaining {
            return Err(std::io::Error::other("P1 ambient scan byte budget"));
        }
        self.scan_stats.files += 1;
        self.scan_stats.bytes += read.len;
        if rel.contains("node_modules/@types/") {
            self.scan_stats.types_files += 1;
            self.scan_stats.types_bytes += read.len;
        }
        let Some(text) = &read.text else {
            self.skip("non_utf8_source");
            self.package_hashes
                .insert(format!("scan:{rel}"), "non_utf8".into());
            return Ok(());
        };
        let declares = text.declares;
        let (patterns, unparseable) = match &text.digest {
            Some(digest) if text.candidate => parsed
                .get(digest)
                .cloned()
                .expect("every candidate digest is parsed"),
            _ => (Vec::new(), 0),
        };
        if unparseable != 0 {
            *self
                .scan_stats
                .skipped
                .entry("unparseable_declare_module".into())
                .or_default() += unparseable;
        }
        let references = &text.references;
        self.scan_stats.declaration_files += usize::from(declares);
        // Indexed source occupancy already binds addition/removal; the
        // loader hashes its bytes. Keep extra read receipts for sources in
        // skipped directories and all declaration/reference content.
        if declares || !references.is_empty() || self.entries.get(rel) != Some(&0) {
            self.package_hashes.insert(
                format!("scan:{rel}"),
                match &text.digest {
                    Some(digest) => digest.clone(),
                    None => "regular".into(),
                },
            );
        }
        if !patterns.is_empty() {
            self.ambient.insert(rel.clone(), patterns);
        }
        if !references.is_empty() {
            self.references.insert(rel.clone(), references.clone());
        }
        Ok(())
    }

    pub(super) fn finish_case_inventory(&mut self) {
        if !self.case_insensitive {
            return;
        }
        for path in self.entries.keys().chain(self.type_entries.keys()) {
            let folded = fold(path);
            if folded == *path {
                continue;
            }
            self.folded_entries
                .entry(folded)
                .and_modify(|prior| {
                    if prior.as_deref() != Some(path) {
                        *prior = None;
                    }
                })
                .or_insert_with(|| Some(path.clone()));
        }
    }
    pub(super) fn case_collision(&self, path: &str) -> bool {
        if !self.case_insensitive {
            return false;
        }
        // A differently cased parent also makes a candidate opaque.
        let mut prefix = String::new();
        for part in path.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let folded = fold(&prefix);
            if (folded != prefix
                && (self.entries.contains_key(&folded) || self.type_entries.contains_key(&folded)))
                || self
                    .folded_entries
                    .get(&folded)
                    .is_some_and(|p| p.as_deref() != Some(prefix.as_str()))
            {
                return true;
            }
        }
        false
    }
    pub(super) fn link_target(&self, path: &str) -> Option<String> {
        for (link, target) in &self.links {
            if path == link {
                return target.clone();
            }
            if let Some(rest) = path.strip_prefix(&(link.clone() + "/")) {
                return Some(format!("{}/{rest}", target.as_ref()?));
            }
        }
        Some(path.into())
    }
    pub(super) fn external_type_absent(&self, name: &str, mangled: &str) -> bool {
        self.external_modules.iter().all(|(modules, kind)| {
            *kind == 0
                || [format!("@types/{mangled}"), name.into()]
                    .iter()
                    .all(|name| {
                        [
                            "", ".ts", ".tsx", ".d.ts", ".mts", ".cts", ".d.mts", ".d.cts", ".js",
                            ".jsx",
                        ]
                        .iter()
                        .all(|suffix| {
                            let path = Path::new(modules).join(format!("{name}{suffix}"));
                            let fact = match std::fs::symlink_metadata(&path) {
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                                _ => Some(2),
                            };
                            let path = path.to_string_lossy().into_owned();
                            self.probes.lock().unwrap().insert(path.clone());
                            self.first_pass_facts.lock().unwrap().insert(path, fact);
                            fact.is_none()
                        })
                    })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_probe_and_unknown_fallback() {
        let d = tempfile::TempDir::new().unwrap();
        assert_eq!(case_insensitive(d.path()), None);
        assert!(JsPathsSnapshot::capture(d.path()).case_insensitive);
        std::fs::write(d.path().join("probe.ts"), "").unwrap();
        assert_eq!(
            case_insensitive(d.path()),
            Some(d.path().join("PROBE.ts").exists())
        );
    }
    #[test]
    fn unicode_candidate_parent_and_sensitive_occupancy() {
        let mut s = JsPathsSnapshot {
            case_insensitive: true,
            ..JsPathsSnapshot::default()
        };
        s.entries.insert("src".into(), 1);
        s.entries.insert("src/Σ.ts".into(), 0);
        s.entries.insert("src/ſ.ts".into(), 0);
        s.finish_case_inventory();
        for p in ["src/ς.ts", "src/s.ts", "SRC/absent.ts"] {
            assert_eq!(s.kind(p), Some(2));
            assert!(!s.first_pass_absent(p));
        }
        assert_eq!(s.kind("src/absent.ts"), None);
        s.case_insensitive = false;
        assert_eq!(s.kind("src/ς.ts"), None);
    }
    #[test]
    fn compact_inventory_reuses_lowercase_paths_and_preserves_collisions() {
        let d = tempfile::TempDir::new().unwrap();
        std::fs::write(d.path().join("tsconfig.json"), "{}").unwrap();
        std::fs::write(d.path().join("lower.ts"), "export {};").unwrap();
        let s = JsPathsSnapshot::capture(d.path());
        assert!(!s.type_entries.contains_key("lower.ts"));
        assert!(!s.covered.contains_key("lower.ts"));
        assert!(!s.package_hashes.contains_key("scan:lower.ts"));
        let mut s = JsPathsSnapshot {
            case_insensitive: true,
            ..JsPathsSnapshot::default()
        };
        s.entries.insert("src".into(), 1);
        s.entries.insert("src/lower.ts".into(), 0);
        s.type_entries.insert("node_modules".into(), 1);
        s.type_entries.insert("node_modules/lower.ts".into(), 0);
        s.finish_case_inventory();
        assert!(s.folded_entries.is_empty());
        for path in ["src/LOWER.ts", "SRC/absent.ts", "node_modules/LOWER.ts"] {
            assert!(s.case_collision(path), "{path}");
        }
        assert!(!s.case_collision("src/lower.ts"));
        assert!(!s.case_collision("src/absent.ts"));
        s.entries.insert("src/Lower.ts".into(), 0);
        s.finish_case_inventory();
        assert!(s.case_collision("src/lower.ts"));
        assert!(s.case_collision("src/Lower.ts"));
    }
    #[test]
    fn only_bin_and_git_are_excluded() {
        for p in [".git/a.ts", "nested/.git/a.ts", "node_modules/.bin/a.ts"] {
            assert!(excluded(Path::new(p), false));
        }
        assert!(excluded(Path::new("NODE_MODULES/.BIN/a.ts"), true));
        for p in [
            ".GIT/a.ts",
            "node_modules/bin/a.ts",
            "vendor/a.ts",
            ".hidden/a.ts",
        ] {
            assert!(!excluded(Path::new(p), false));
        }
    }
    #[test]
    fn source_entry_count_does_not_bound_scan() {
        let d = tempfile::TempDir::new().unwrap();
        std::fs::write(d.path().join("a.ts"), "declare module 'test' {}").unwrap();
        let mut s = JsPathsSnapshot::default();
        s.scan_stats.entries = 200_001;
        s.scan(d.path(), d.path(), 0).unwrap();
        assert!(s.scan_stats.entries > 200_001);
        assert_eq!(s.ambient["a.ts"], ["test"]);
    }
    #[test]
    #[cfg(unix)]
    fn linked_source_is_scanned_and_counted_once() {
        let d = tempfile::TempDir::new().unwrap();
        std::fs::write(d.path().join("tsconfig.json"), "{}").unwrap();
        let source = "declare module 'linked' {}";
        std::fs::write(d.path().join("actual.d.ts"), source).unwrap();
        for name in ["one.d.ts", "two.d.ts"] {
            std::os::unix::fs::symlink("actual.d.ts", d.path().join(name)).unwrap();
        }
        let s = JsPathsSnapshot::capture(d.path());
        assert!(s.complete);
        assert_eq!(s.scan_stats.files, 1);
        assert_eq!(s.scan_stats.bytes, source.len() as u64);
        assert_eq!(s.ambient.len(), 1);
        assert_eq!(s.links.len(), 2);
    }
    #[test]
    fn tolerant_scan_counts_each_skip_arm_and_retains_valid_patterns() {
        let d = tempfile::TempDir::new().unwrap();
        let root = d.path().join("project");
        std::fs::create_dir_all(root.join("node_modules/.bin")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("tsconfig.json"), "{}").unwrap();
        std::fs::write(root.join("tool"), "declare module 'ignored' {}").unwrap();
        std::fs::write(root.join("constants.json"), "{}").unwrap();
        std::fs::write(root.join("bad.d.ts"), [0xff]).unwrap();
        std::fs::write(
            root.join("malformed.d.ts"),
            "declare module '\\xZZ' {}\ndeclare module 'kept' {}",
        )
        .unwrap();
        std::fs::write(d.path().join("outside.d.ts"), "declare module 'ignored' {}").unwrap();
        std::os::unix::fs::symlink(d.path().join("outside.d.ts"), root.join("outside.d.ts"))
            .unwrap();
        std::os::unix::fs::symlink("missing", root.join("missing.d.ts")).unwrap();
        let s = JsPathsSnapshot::capture(&root);
        assert!(s.complete);
        assert_eq!(
            s.scan_stats.skipped,
            BTreeMap::from([
                ("excluded".into(), 2),
                ("non_source".into(), 3),
                ("outside_root_symlink".into(), 1),
                ("unreadable".into(), 1),
                ("non_utf8_source".into(), 1),
                ("unparseable_declare_module".into(), 1),
            ])
        );
        assert_eq!(s.ambient["malformed.d.ts"], ["kept"]);
        let mut missing = JsPathsSnapshot::default();
        missing.scan(&root, &root.join("no-file.ts"), 0).unwrap();
        assert_eq!(missing.scan_stats.skipped["unreadable"], 1);
    }

    #[test]
    fn unreadable_source_and_directory_skip() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::TempDir::new().unwrap();
        let file = d.path().join("unread.ts");
        let dir = d.path().join("unread");
        std::fs::write(&file, "declare module 'ignored' {}").unwrap();
        std::fs::create_dir(&dir).unwrap();
        for path in [&file, &dir] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o0)).unwrap();
        }
        let mut s = JsPathsSnapshot::default();
        let result = s.scan(d.path(), d.path(), 0);
        let walk_result = s.walk(d.path(), d.path());
        for path in [&file, &dir] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        result.unwrap();
        walk_result.unwrap();
        assert_eq!(s.scan_stats.skipped["unreadable"], 2);
        assert!(s.ambient.is_empty());
    }
    #[test]
    fn non_regular_and_non_utf8_paths_skip() {
        use std::os::unix::ffi::OsStringExt;
        let d = tempfile::TempDir::new().unwrap();
        // A FIFO exercises non-regular source metadata without binding a socket.
        let output = std::process::Command::new("mkfifo")
            .arg(d.path().join("pipe.ts"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let non_utf8 = d.path().join(std::ffi::OsString::from_vec(vec![0xff]));
        let mut s = JsPathsSnapshot::default();
        s.scan(d.path(), d.path(), 0).unwrap();
        assert_eq!(s.scan_stats.skipped["non_regular_source"], 1);
        // Reject the spelling before metadata; no filesystem creation is needed.
        s.scan(d.path(), &non_utf8, 0).unwrap();
        assert_eq!(s.scan_stats.skipped["non_utf8_path"], 1);
        assert!(s.ambient.is_empty());
        // The indexing inventory is tolerant too, before the ambient scan.
        s.walk(d.path(), d.path()).unwrap();
    }

    #[test]
    fn references_are_leading_line_comments_only() {
        let reference = "///\u{feff}<reference path='/outside.d.ts' />";
        for source in [
            reference.to_owned(),
            format!("\u{feff}/* header */\u{2028}{reference}\nexport {{}};"),
            format!("#!/usr/bin/env node\n{reference}"),
        ] {
            assert_eq!(
                super::super::triple_references(&source),
                [("path".into(), "/outside.d.ts".into())]
            );
        }
        for source in [
            format!("/* {reference} */"),
            format!("const text={reference:?};"),
            format!("const text=`\n{reference}`;"),
            format!("export {{}};\n{reference}"),
            "// <reference path='/outside.d.ts' />".into(),
        ] {
            assert!(
                super::super::triple_references(&source).is_empty(),
                "{source}"
            );
        }
    }
}
