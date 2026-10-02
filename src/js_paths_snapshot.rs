//! Bounded repository closure and configuration/occupancy snapshot for P1.
#[path = "js_paths_boundary.rs"]
mod boundary;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct JsPathsSnapshot {
    pub(crate) configs: BTreeMap<String, Option<Vec<u8>>>,
    pub(crate) entries: BTreeMap<String, u8>, // 0 regular, 1 directory, 2 opaque
    pub(crate) complete: bool,
    opaque_directories: BTreeSet<String>,
    package_hashes: BTreeMap<String, String>,
    pub(crate) ambient: BTreeMap<String, Vec<String>>,
    pub(crate) references: BTreeMap<String, Vec<(String, String)>>,
    // The ambient scan, unlike the indexing walk, covers skipped directories.
    covered: BTreeMap<String, u8>, // 0 source read, 1 directory traversed
    root: String,
    type_redirects: BTreeMap<String, Option<Vec<String>>>,
    case_insensitive: bool,
    folded_entries: BTreeMap<String, Option<String>>,
    links: BTreeMap<String, Option<String>>,
    external_types: BTreeMap<String, u8>,
    module_dependencies: BTreeMap<String, Option<Vec<String>>>,
    root_spelling: String,
    #[serde(skip)]
    declined_configs: Arc<Mutex<BTreeSet<String>>>,
    #[serde(skip)]
    boundary_failure: Arc<std::sync::OnceLock<Option<String>>>,
    #[serde(skip)]
    boundary_inputs: Arc<Mutex<BTreeMap<String, bool>>>,
    #[serde(skip)]
    module_packages: BTreeMap<String, Option<Vec<u8>>>,
    #[serde(skip)]
    external_modules: BTreeMap<String, u8>,
    #[serde(skip)]
    type_metadata_bytes: u64,
    #[serde(skip)]
    pub(crate) scan_stats: ScanStats,
    #[serde(skip)]
    probes: Arc<Mutex<BTreeSet<String>>>,
    #[serde(skip)]
    first_pass_facts: Arc<Mutex<BTreeMap<String, Option<u8>>>>,
    #[serde(skip)]
    type_entries: BTreeMap<String, u8>,
    #[serde(skip)]
    absent_inputs: Arc<Mutex<BTreeSet<(String, String)>>>,
    #[serde(skip)]
    unsafe_inputs: Arc<Mutex<BTreeSet<(String, String)>>>,
}

enum TypeInput {
    Present(String),
    Absent,
    Unsafe,
}

// Bound traversal, retained dependencies, and bytes, including node_modules/@types.
const SCAN_BYTES: u64 = 1024 * 1024 * 1024;
#[derive(Debug, Clone, Default, serde::Serialize)]
pub(crate) struct ScanStats {
    entries: usize,
    files: usize,
    bytes: u64,
    types_files: usize,
    types_bytes: u64,
    declaration_files: usize,
}
impl JsPathsSnapshot {
    pub(crate) fn capture(root: &Path) -> Self {
        if root.as_os_str().is_empty() {
            return Self::default();
        }
        let mut s = Self {
            complete: true,
            root_spelling: std::path::absolute(root)
                .ok()
                .and_then(|p| p.to_str().map(str::to_owned))
                .unwrap_or_default(),
            root: root
                .canonicalize()
                .ok()
                .and_then(|p| p.to_str().map(str::to_owned))
                .unwrap_or_default(),
            ..Self::default()
        };
        let root = Path::new(&s.root).to_path_buf();
        s.case_insensitive = boundary::case_insensitive(&root).unwrap_or(true);
        if let Err(error) = s.walk(&root, &root) {
            s.complete = false;
            eprintln!(
                "warning: P1 paths disabled: {error} ({} entries)",
                s.entries.len()
            );
        }
        // Indexing may skip directories; declaration absence never may. Avoid
        // this additional scan only when no config can authorize P1 at all.
        if s.complete && !s.configs.is_empty() {
            if let Err(error) = s.scan(&root, &root, 0) {
                s.complete = false;
                eprintln!(
                    "warning: P1 paths disabled: {error} ({} scan entries, {} bytes)",
                    s.scan_stats.entries, s.scan_stats.bytes
                );
            }
        }
        if !s.configs.is_empty() {
            // Retain r1's outside-capture boundary: missing ancestor module
            // directories prove absence; existing/unread ones are opaque.
            for ancestor in Path::new(&s.root).ancestors().skip(1) {
                let path = ancestor.join("node_modules");
                let kind = match std::fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
                    _ => 2,
                };
                let types = path.join("@types");
                let type_kind = match std::fs::symlink_metadata(&types) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
                    _ => 2,
                };
                s.external_types
                    .insert(types.to_string_lossy().into(), type_kind);
                s.external_modules
                    .insert(path.to_string_lossy().into(), kind);
            }
        }
        s.finish_case_inventory();
        s
    }
    fn walk(&mut self, root: &Path, dir: &Path) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if self.entries.len() >= 200_000 {
                return Err(std::io::Error::other("P1 snapshot budget"));
            }
            let p = entry.path();
            let rel = p.strip_prefix(root).map_err(std::io::Error::other)?;
            let Some(rel) = rel.to_str() else {
                return Err(std::io::Error::other("P1 non-UTF8 path"));
            };
            let ft = entry.file_type()?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| std::io::Error::other("P1 path"))?;
            let blocked = ft.is_symlink()
                || name.starts_with('.')
                || ["target", "node_modules", "vendor", "dist", "build"].contains(&name);
            if blocked && (ft.is_dir() || ft.is_symlink()) {
                self.opaque_directories.insert(rel.to_owned());
            }
            if name == "package.json" {
                let digest = if !blocked && ft.is_file() && entry.metadata()?.len() <= 262_144 {
                    std::fs::read(&p)
                        .ok()
                        .map(|b| format!("{:x}", Sha256::digest(b)))
                } else {
                    None
                };
                self.package_hashes
                    .insert(rel.to_owned(), digest.unwrap_or_else(|| "opaque".into()));
            }
            self.entries.insert(
                rel.to_owned(),
                if blocked {
                    2
                } else if ft.is_dir() {
                    1
                } else if ft.is_file() {
                    0
                } else {
                    2
                },
            );
            let config_name = name.to_ascii_lowercase();
            if config_name == "jsconfig.json"
                || config_name == "tsconfig.json"
                || (config_name.starts_with("tsconfig.") && config_name.ends_with(".json"))
            {
                let bytes = if name == config_name
                    && !blocked
                    && ft.is_file()
                    && entry.metadata()?.len() <= 262_144
                {
                    std::fs::read(&p).ok()
                } else {
                    None
                };
                let key = format!("{}{}", &rel[..rel.len() - name.len()], config_name);
                self.record_config(key, bytes);
            }
            if ft.is_dir() && !blocked {
                self.walk(root, &p)?;
            }
        }
        Ok(())
    }
    fn record_config(&mut self, key: String, bytes: Option<Vec<u8>>) {
        self.configs
            .entry(key)
            .and_modify(|prior| *prior = None)
            .or_insert(bytes);
    }
    pub(crate) fn topology(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        if self.configs.is_empty() {
            return out;
        }
        for (p, b) in &self.configs {
            out.insert(
                format!("js_paths_config:{p}"),
                b.as_ref()
                    .map(|b| format!("{:x}", Sha256::digest(b)))
                    .unwrap_or_else(|| "opaque".into()),
            );
        }
        // Additions, removals, skipped candidates, symlink and directory replacement all invalidate.
        let occupancy: BTreeMap<_, _> = self
            .entries
            .iter()
            .filter(|(p, kind)| {
                let name = p.rsplit('/').next().unwrap_or(p);
                self.opaque_directories.contains(*p)
                    || (**kind != 1
                        && (name == "package.json"
                            || (!name.starts_with('.') && !name.contains('.'))
                            || [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]
                                .iter()
                                .any(|e| name.ends_with(e))))
            })
            .collect();
        let dependencies: BTreeMap<_, _> = self
            .probes
            .lock()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p.clone(),
                    self.first_pass_facts
                        .lock()
                        .unwrap()
                        .get(p)
                        .copied()
                        .unwrap_or_else(|| {
                            self.type_entries
                                .get(p)
                                .or_else(|| self.entries.get(p))
                                .copied()
                        }),
                )
            })
            .collect();
        let bytes = bincode::serialize(&(
            "repository-ambient-closure-v3-main-unicode16",
            &occupancy,
            &dependencies,
            &self.package_hashes,
            &self.ambient,
            &self.references,
            &self.covered,
            &self.type_redirects,
            &self.external_modules,
            &self.external_types,
            &self.links,
            &self.module_dependencies,
            self.case_insensitive,
            self.complete,
        ))
        .expect("snapshot serializes");
        let absent = self.absent_inputs.lock().unwrap().len();
        if absent != 0 {
            eprintln!("P1 paths: absent type inputs={absent} (distinct origin/input pairs)");
        }
        let declined = self.declined_configs.lock().unwrap().len();
        if declined != 0 {
            eprintln!("P1 paths: boundary-declined configs={declined}");
        }
        out.insert(
            "js_paths_occupancy".into(),
            format!("{:x}", Sha256::digest(bytes)),
        );
        out
    }
    pub(crate) fn kind(&self, p: &str) -> Option<u8> {
        self.probes.lock().unwrap().insert(p.into());
        if self.case_collision(p) {
            self.first_pass_facts
                .lock()
                .unwrap()
                .insert(p.into(), Some(2));
            return Some(2);
        }
        self.entries.get(p).copied()
    }
    // Candidate occupancy comes from the full scan, including directories the
    // indexing walk skips. Every probe, even an absent one, joins topology.
    pub(crate) fn first_pass_absent(&self, p: &str) -> bool {
        self.probes.lock().unwrap().insert(p.into());
        self.complete && self.first_pass_occupancy(p).is_none()
    }
    fn first_pass_occupancy(&self, p: &str) -> Option<u8> {
        if let Some(fact) = self.first_pass_facts.lock().unwrap().get(p).copied() {
            return fact;
        }
        let inspect = || {
            if self.case_collision(p) {
                return Some(2);
            }
            // Literal snapshot keys alone cannot prove absence on filesystems
            // with case or Unicode aliases. Never follow an unknown parent:
            // captured, traversed directories are the only readable chain.
            if self.root.is_empty()
                || p.is_empty()
                || p.split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || p.contains(['\\', ':'])
            {
                return Some(2);
            }
            let mut prefix = String::new();
            let mut parts = p.split('/').peekable();
            while let Some(part) = parts.next() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                let kind = self
                    .type_entries
                    .get(&prefix)
                    .or_else(|| self.entries.get(&prefix));
                if parts.peek().is_none() {
                    if let Some(kind) = kind {
                        return Some(*kind);
                    }
                } else if kind.is_some() {
                    if self.covered.get(&prefix) == Some(&1) {
                        continue;
                    }
                    return if kind == Some(&0) { None } else { Some(2) };
                }
                // All prior parents were captured readable directories. An
                // alias/new entry is opaque; a missing parent proves absence
                // without descending through an uninstalled node_modules.
                return match std::fs::symlink_metadata(Path::new(&self.root).join(&prefix)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    _ => Some(2),
                };
            }
            Some(2)
        };
        let fact = inspect();
        self.first_pass_facts.lock().unwrap().insert(p.into(), fact);
        fact
    }
    pub(crate) fn first_pass_root_file_absent(&self, ts: bool) -> bool {
        // TypeScript appends extensions to the absolute root name, producing
        // sibling files outside this capture. Record each opaque candidate.
        let extensions: &[&str] = if ts {
            &[".ts", ".tsx", ".d.ts"]
        } else {
            &[".d.ts"]
        };
        extensions.iter().fold(true, |absent, extension| {
            self.first_pass_absent(&format!("{}{extension}", self.root)) & absent
        })
    }
    pub(crate) fn first_pass_package(&self, p: &str) -> Result<Option<&[u8]>, ()> {
        if self.first_pass_absent(p) {
            return Ok(None);
        }
        self.module_packages
            .get(p)
            .and_then(Option::as_deref)
            .map(Some)
            .ok_or(())
    }
    pub(crate) fn external_first_pass_absent(&self) -> bool {
        self.external_modules.values().all(|kind| *kind == 0)
    }
    pub(crate) fn input_path(&self, base: &str, path: &str) -> Option<String> {
        use crate::js_paths_syntax::norm;
        let p = if path.starts_with('/') {
            // Normalize before checking containment: root/../outside is outside.
            let absolute = norm("", path.trim_start_matches('/'))?;
            [self.root.as_str(), self.root_spelling.as_str()]
                .iter()
                .find_map(|root| {
                    let root = norm("", root.trim_start_matches('/'))?;
                    if absolute == root {
                        Some(String::new())
                    } else {
                        absolute.strip_prefix(&(root + "/")).map(str::to_owned)
                    }
                })?
        } else {
            norm(base, path)?
        };
        Some(p)
    }
    fn classify_path(&self, base: &str, path: &str) -> TypeInput {
        let Some(p) = self
            .input_path(base, path)
            .filter(|_| !path.is_empty() && self.complete)
        else {
            return TypeInput::Unsafe;
        };
        let Some(p) = self.link_target(&p) else {
            return TypeInput::Unsafe;
        };
        self.probes.lock().unwrap().insert(p.clone());
        if self.case_collision(&p) {
            return TypeInput::Unsafe;
        }
        if p.is_empty() || self.covered.contains_key(&p) {
            TypeInput::Present(p)
        } else if self.first_pass_occupancy(&p).is_some() {
            TypeInput::Unsafe
        } else {
            TypeInput::Absent
        }
    }
    fn record_input(&self, base: &str, name: &str, input: &TypeInput) {
        match input {
            TypeInput::Absent => {
                self.absent_inputs
                    .lock()
                    .unwrap()
                    .insert((base.into(), name.into()));
            }
            TypeInput::Unsafe => {
                if self
                    .unsafe_inputs
                    .lock()
                    .unwrap()
                    .insert((base.into(), name.into()))
                {
                    eprintln!("warning: P1 paths config declined: outside-root or unread type input {name:?} from {base:?}");
                }
            }
            TypeInput::Present(_) => {}
        }
    }
    pub(crate) fn covered_root(&self, base: &str, path: &str) -> Option<String> {
        let input = self.classify_path(base, path);
        self.record_input(base, path, &input);
        match input {
            TypeInput::Present(p) => Some(p),
            TypeInput::Absent => self.input_path(base, path),
            TypeInput::Unsafe => None,
        }
    }
    #[cfg(test)]
    pub(crate) fn was_probed(&self, path: &str) -> bool {
        self.probes.lock().unwrap().contains(path)
    }
    fn covered_redirect(&self, base: &str, target: &str) -> TypeInput {
        if !target.contains('*') {
            return self.classify_path(base, target);
        }
        // typesVersions may select any file under this scanned directory.
        let (prefix, suffix) = target.split_once('*').unwrap();
        if suffix.contains("..") || suffix.contains(':') || suffix.contains('\\') {
            return TypeInput::Unsafe;
        }
        let directory = crate::js_paths_syntax::dir(prefix);
        self.classify_path(base, if directory.is_empty() { "." } else { directory })
    }
    pub(crate) fn covered_type(&self, base: &str, name: &str, roots: Option<&[String]>) -> bool {
        let input = self.resolve_type(base, name, roots);
        self.record_input(base, name, &input);
        match input {
            TypeInput::Present(p) => self.input_references_covered(&p, roots, &mut BTreeSet::new()),
            TypeInput::Absent => true,
            TypeInput::Unsafe => false,
        }
    }
    fn resolve_type(&self, base: &str, name: &str, roots: Option<&[String]>) -> TypeInput {
        if name.is_empty() {
            return TypeInput::Unsafe;
        }
        if name.starts_with('.') || name.starts_with('/') {
            return self.classify_path(base, name);
        }
        let resolve = |directory: &str, name: &str| {
            ["", ".d.ts", ".d.mts", ".d.cts"]
                .iter()
                .map(|suffix| self.classify_path(directory, &format!("{name}{suffix}")))
                .find(|input| !matches!(input, TypeInput::Absent))
                .unwrap_or(TypeInput::Absent)
        };
        let mangled = name.strip_prefix('@').map(|s| s.replacen('/', "__", 1));
        let typed_name = mangled.as_deref().unwrap_or(name);
        if let Some(roots) = roots {
            if let Some(input) = roots
                .iter()
                .map(|root| resolve(root, typed_name))
                .find(|input| !matches!(input, TypeInput::Absent))
            {
                return input;
            }
        }
        // Node10's secondary lookup still searches node_modules after custom
        // typeRoots miss, including when the caller supplied an empty array.
        let mut directory = base;
        loop {
            for candidate in [
                format!("node_modules/@types/{typed_name}"),
                format!("node_modules/{name}"),
            ] {
                let input = resolve(directory, &candidate);
                if !matches!(input, TypeInput::Absent) {
                    return input;
                }
            }
            if directory.is_empty() {
                return if self.external_type_absent(name, typed_name) {
                    TypeInput::Absent
                } else {
                    TypeInput::Unsafe
                };
            }
            directory = crate::js_paths_syntax::dir(directory);
        }
    }
    pub(crate) fn reference_covered(
        &self,
        base: &str,
        kind: &str,
        name: &str,
        roots: Option<&[String]>,
    ) -> bool {
        let input = if kind == "path" {
            self.classify_path(base, name)
        } else {
            self.resolve_type(base, name, roots)
        };
        self.record_input(base, name, &input);
        match input {
            TypeInput::Present(p) => self.input_references_covered(&p, roots, &mut BTreeSet::new()),
            TypeInput::Absent => true,
            TypeInput::Unsafe => false,
        }
    }
    pub(crate) fn input_references_covered(
        &self,
        input: &str,
        roots: Option<&[String]>,
        seen: &mut BTreeSet<String>,
    ) -> bool {
        self.input_closure_covered(input, roots, seen, true)
    }
    fn input_closure_covered(
        &self,
        input: &str,
        roots: Option<&[String]>,
        seen: &mut BTreeSet<String>,
        references: bool,
    ) -> bool {
        // Every queued path belongs to the bounded scan inventory. Iteration
        // handles long reference chains and safe cycles without a second cap.
        let mut pending = vec![input.to_owned()];
        while let Some(input) = pending.pop() {
            if !seen.insert(input.clone()) {
                continue;
            }
            // Ordinary module resolution also consults package main. Type
            // directives do not. Probe the selected directory's literal metadata
            // name so case aliases/opaque metadata are cache-bound refusals.
            if !references {
                if self.type_entries.get(&input) == Some(&1)
                    && self
                        .first_pass_package(format!("{input}/package.json").trim_start_matches('/'))
                        .is_err()
                {
                    return false;
                }
                for (file, bytes) in self
                    .module_packages
                    .iter()
                    .filter(|(file, _)| input.is_empty() || file.starts_with(&format!("{input}/")))
                {
                    let Some(value) = bytes
                        .as_ref()
                        .and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok())
                    else {
                        return false;
                    };
                    if let Some(target) = value
                        .get("main")
                        .and_then(serde_json::Value::as_str)
                        .filter(|s| !s.is_empty())
                    {
                        let base = crate::js_paths_syntax::dir(file);
                        let target_input = self.covered_redirect(base, target);
                        self.record_input(base, target, &target_input);
                        match target_input {
                            TypeInput::Present(p) => pending.push(p),
                            TypeInput::Absent => {}
                            TypeInput::Unsafe => return false,
                        }
                    }
                }
            }
            for (file, redirects) in self
                .type_redirects
                .iter()
                .filter(|(file, _)| input.is_empty() || file.starts_with(&format!("{input}/")))
            {
                let Some(redirects) = redirects else {
                    return false;
                };
                for target in redirects {
                    let base = crate::js_paths_syntax::dir(file);
                    let input = self.covered_redirect(base, target);
                    self.record_input(base, target, &input);
                    match input {
                        TypeInput::Present(p) => pending.push(p),
                        TypeInput::Absent => {}
                        TypeInput::Unsafe => return false,
                    }
                }
            }
            // The repository module boundary checks every scanned declaration's
            // directives separately. A literal .js import can resolve to .d.ts;
            // do not promote strings in the unused JS implementation to inputs.
            if !references {
                continue;
            }
            for (file, references) in self.references.iter().filter(|(file, _)| {
                file.as_str() == input || input.is_empty() || file.starts_with(&format!("{input}/"))
            }) {
                for (kind, name) in references {
                    let base = crate::js_paths_syntax::dir(file);
                    let input = if kind == "path" {
                        self.classify_path(base, name)
                    } else {
                        self.resolve_type(base, name, roots)
                    };
                    self.record_input(base, name, &input);
                    match input {
                        TypeInput::Present(p) => pending.push(p),
                        TypeInput::Absent => {}
                        TypeInput::Unsafe => return false,
                    }
                }
            }
        }
        true
    }
    pub(crate) fn unblocked(&self, p: &str) -> bool {
        let mut prefix = String::new();
        for part in p.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if self.kind(&prefix) == Some(2) {
                return false;
            }
        }
        true
    }
}

fn type_redirects(bytes: &[u8]) -> Option<Vec<String>> {
    fn strings(value: &serde_json::Value, out: &mut Vec<String>) -> Option<()> {
        match value {
            serde_json::Value::String(s) if !s.is_empty() => out.push(s.clone()),
            serde_json::Value::Array(a) => {
                for v in a {
                    strings(v, out)?;
                }
            }
            serde_json::Value::Object(o) => {
                for v in o.values() {
                    strings(v, out)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;
    let mut out = Vec::new();
    for key in ["types", "typings", "typesVersions"] {
        if let Some(v) = object.get(key) {
            strings(v, &mut out)?;
        }
    }
    // P1 supports Node10, whose type-directive lookup ignores exports.
    Some(out)
}

fn whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}
fn line_end(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{2028}' | '\u{2029}')
}
fn trivia(mut s: &str) -> &str {
    loop {
        s = s.trim_start_matches(whitespace);
        if let Some(rest) = s.strip_prefix("/*") {
            s = rest.split_once("*/").map_or("", |(_, tail)| tail);
        } else if let Some(rest) = s.strip_prefix("//") {
            s = rest.find(line_end).map_or("", |end| &rest[end..]);
        } else if let Some(rest) = s.strip_prefix('\\').filter(|s| s.starts_with(line_end)) {
            s = rest.trim_start_matches(line_end);
        } else {
            return s;
        }
    }
}
fn identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}
fn keyword<'a>(s: &'a str, word: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(word)?;
    (!rest.starts_with(identifier)).then_some(rest)
}
// Return decoded name and remaining input. An unsupported or malformed name
// barriers the repository rather than establishing declaration absence.
pub(super) fn literal(s: &str) -> Option<(String, &str)> {
    let quote = s.chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let mut rest = &s[1..];
    let mut name = String::new();
    while let Some(c) = rest.chars().next() {
        rest = &rest[c.len_utf8()..];
        if c == quote {
            return Some((name, rest));
        }
        if line_end(c) {
            return None;
        }
        if c != '\\' {
            name.push(c);
            continue;
        }
        let escaped = rest.chars().next()?;
        rest = &rest[escaped.len_utf8()..];
        if line_end(escaped) {
            if escaped == '\r' {
                rest = rest.strip_prefix('\n').unwrap_or(rest);
            }
            continue;
        }
        let decoded = match escaped {
            'u' | 'x' => {
                let value = if escaped == 'u' && rest.starts_with('{') {
                    let end = rest.find('}')?;
                    let digits = &rest[1..end];
                    if digits.is_empty()
                        || digits.len() > 6
                        || !digits.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return None;
                    }
                    let v = u32::from_str_radix(digits, 16).ok()?;
                    rest = &rest[end + 1..];
                    v
                } else {
                    let n = if escaped == 'u' { 4 } else { 2 };
                    let digits = rest.get(..n)?;
                    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return None;
                    }
                    let mut v = u32::from_str_radix(digits, 16).ok()?;
                    rest = &rest[n..];
                    if (0xd800..=0xdbff).contains(&v) {
                        let low = rest.strip_prefix("\\u")?.get(..4)?;
                        if !low.bytes().all(|b| b.is_ascii_hexdigit()) {
                            return None;
                        }
                        let low = u32::from_str_radix(low, 16).ok()?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return None;
                        }
                        v = 0x10000 + ((v - 0xd800) << 10) + low - 0xdc00;
                        rest = &rest[6..];
                    }
                    v
                };
                char::from_u32(value)?
            }
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'v' => '\u{b}',
            '0' if !rest.starts_with(|c: char| c.is_ascii_digit()) => '\0',
            '0'..='9' => return None,
            c => c,
        };
        name.push(decoded);
    }
    None
}
// Tolerant syntax separates declaration candidates from comments and literals.
// The Unicode-aware lexical fallback still fences malformed real candidates.
fn ambient_patterns(source: &str) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
        .is_err()
    {
        return vec!["*".into()];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec!["*".into()];
    };
    let mut pending = vec![tree.root_node()];
    let mut literals = Vec::new();
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            "comment" | "string" | "template_string" | "regex"
        ) {
            literals.push(node.byte_range());
        } else {
            let mut cursor = node.walk();
            pending.extend(node.named_children(&mut cursor));
        }
    }
    let mut patterns = Vec::new();
    for (start, _) in source.match_indices("declare") {
        if literals.iter().any(|range| range.contains(&start)) {
            continue;
        }
        if start > 0 && source[..start].chars().next_back().is_some_and(identifier) {
            continue;
        }
        let Some(rest) = keyword(&source[start..], "declare") else {
            continue;
        };
        let Some(rest) = keyword(trivia(rest), "module") else {
            continue;
        };
        match literal(trivia(rest)) {
            Some((name, _)) => patterns.push(if name.matches('*').count() > 1 {
                "*".into()
            } else {
                name
            }),
            None => return vec!["*".into()],
        }
    }
    patterns
}
fn triple_references(source: &str) -> Vec<(String, String)> {
    let mut references = Vec::new();
    // TypeScript preprocesses only leading comment trivia. Directive-shaped
    // text in strings, templates, block comments or after code is not an input.
    let mut source = source.trim_start_matches(whitespace);
    if source.starts_with("#!") {
        source = source.find(line_end).map_or("", |end| &source[end..]);
    }
    loop {
        source = source.trim_start_matches(whitespace);
        if let Some(block) = source.strip_prefix("/*") {
            let Some((_, tail)) = block.split_once("*/") else {
                break;
            };
            source = tail;
            continue;
        }
        let Some(line) = source.strip_prefix("//") else {
            break;
        };
        let end = line.find(line_end).unwrap_or(line.len());
        source = &line[end..];
        let Some(line) = line[..end].strip_prefix('/') else {
            continue;
        };
        let rest = line.trim_start_matches(whitespace);
        let Some(mut rest) = rest.strip_prefix("<reference") else {
            continue;
        };
        if !rest.starts_with(whitespace) {
            continue;
        }
        loop {
            rest = rest.trim_start_matches(whitespace);
            if rest.starts_with('/') || rest.starts_with('>') {
                break;
            }
            let end = rest
                .find(|c: char| !identifier(c) && c != '-')
                .unwrap_or(rest.len());
            let key = &rest[..end];
            let tail = rest[end..].trim_start_matches(whitespace);
            let value = tail
                .strip_prefix('=')
                .map(|s| s.trim_start_matches(whitespace));
            let parsed = value.and_then(|s| {
                let quote = s.chars().next()?;
                if !matches!(quote, '\'' | '"') {
                    return None;
                }
                let end = s[1..].find(quote)? + 1;
                Some((s[1..end].to_owned(), &s[end + 1..]))
            });
            let Some((name, tail)) = parsed else {
                // Malformed apparent directives cannot prove input coverage.
                references.push(("path".into(), String::new()));
                break;
            };
            if matches!(key, "path" | "types") {
                references.push((key.into(), name));
            }
            rest = tail;
        }
    }
    references
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambient_literals_and_fail_closed_candidates() {
        for (source, expected) in [
            (r#"declare module '\u0040\x6cib' {}"#, "@lib"),
            (r#"declare module '\u{1f600}' {}"#, "😀"),
            (r#"declare module '\ud83d\ude00' {}"#, "😀"),
            (r#"declare module 'a\'b' {}"#, "a'b"),
            (r#"declare module '\u{110000}' {}"#, "*"),
            (r#"declare module '\ud800' {}"#, "*"),
            (r#"declare module '\xZZ' {}"#, "*"),
            (r#"declare module '\01' {}"#, "*"),
            (r#"declare module '*a*' {}"#, "*"),
            ("declare module /* unterminated", "*"),
            ("declare module 'unterminated", "*"),
        ] {
            assert_eq!(ambient_patterns(source), [expected], "{source}");
        }
        assert!(ambient_patterns("const pattern = /[\"']/; const text = `it's multiline`; undeclare module '@lib'; declare moduleFoo '@lib';").is_empty());
        assert_eq!(
            ambient_patterns("declare //separator\u{2028}module '@lib' {}"),
            ["@lib"]
        );
    }
    #[test]
    fn scan_budgets_disable_repository() {
        let d = tempfile::TempDir::new().unwrap();
        std::fs::write(d.path().join("small.d.ts"), "declare module '@lib' {}").unwrap();
        let mut s = JsPathsSnapshot::default();
        s.scan_stats.bytes = SCAN_BYTES;
        assert!(s
            .scan(d.path(), d.path(), 0)
            .unwrap_err()
            .to_string()
            .contains("byte budget"));
        s.scan_stats.bytes = SCAN_BYTES
            - std::fs::metadata(d.path().join("small.d.ts"))
                .unwrap()
                .len();
        assert!(s.scan(d.path(), d.path(), 0).is_ok());
        assert_eq!(s.scan_stats.bytes, SCAN_BYTES);
    }
    #[test]
    fn unreadable_declaration_scan_disables_repository() {
        let d = tempfile::TempDir::new().unwrap();
        std::fs::write(d.path().join("tsconfig.json"), "{}").unwrap();
        std::fs::write(d.path().join("bad.d.ts"), [0xff]).unwrap();
        assert!(!JsPathsSnapshot::capture(d.path()).complete);
    }
    #[test]
    fn config_case_collision_is_order_independent_barrier() {
        for variants in [[Some(b"{}".to_vec()), None], [None, Some(b"{}".to_vec())]] {
            let mut snapshot = JsPathsSnapshot::default();
            for bytes in variants {
                snapshot.record_config("tsconfig.json".into(), bytes);
            }
            assert_eq!(snapshot.configs["tsconfig.json"], None);
        }
    }
}
