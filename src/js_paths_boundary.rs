//! Repository closure. In-root links are scanned once; external inputs decline.
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

impl JsPathsSnapshot {
    pub(super) fn scan(
        &mut self,
        root: &Path,
        directory: &Path,
        _depth: usize,
    ) -> std::io::Result<()> {
        let mut pending = vec![directory.to_path_buf()];
        let mut seen = BTreeSet::new();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .map_err(std::io::Error::other)?;
        while let Some(mut path) = pending.pop() {
            let relative = path.strip_prefix(root).map_err(std::io::Error::other)?;
            if excluded(relative, self.case_insensitive) {
                continue;
            }
            let mut rel = relative
                .to_str()
                .ok_or_else(|| std::io::Error::other("P1 ambient non-UTF8 path"))?
                .to_owned();
            let mut metadata = std::fs::symlink_metadata(&path)?;
            if metadata.is_symlink() {
                self.type_entries.insert(rel.clone(), 2);
                let target = path.canonicalize().ok().filter(|p| p.starts_with(root));
                self.links.insert(
                    rel.clone(),
                    target
                        .as_ref()
                        .and_then(|p| p.strip_prefix(root).ok()?.to_str().map(str::to_owned)),
                );
                let Some(target) = target else {
                    continue;
                };
                if excluded(
                    target.strip_prefix(root).map_err(std::io::Error::other)?,
                    self.case_insensitive,
                ) {
                    continue;
                }
                path = target;
                rel = path
                    .strip_prefix(root)
                    .map_err(std::io::Error::other)?
                    .to_str()
                    .ok_or_else(|| std::io::Error::other("P1 ambient non-UTF8 target"))?
                    .to_owned();
                metadata = std::fs::symlink_metadata(&path)?;
            }
            // Physical paths are canonical after link resolution; repeated links
            // and directory cycles contribute neither duplicate reads nor bytes.
            if !seen.insert(path.clone()) {
                continue;
            }
            self.scan_stats.entries += 1;
            self.type_entries.insert(
                rel.clone(),
                if metadata.is_dir() {
                    1
                } else if metadata.is_file() {
                    0
                } else {
                    2
                },
            );
            if metadata.is_dir() {
                for entry in std::fs::read_dir(&path)? {
                    pending.push(entry?.path());
                }
                self.covered.insert(rel, 1);
                continue;
            }
            if path.file_name().is_some_and(|n| n == "package.json") {
                use std::io::Read;
                let bytes = if metadata.is_file() && self.type_metadata_bytes < SCAN_BYTES {
                    std::fs::File::open(&path).ok().and_then(|f| {
                        let mut bytes = Vec::new();
                        f.take(262_145).read_to_end(&mut bytes).ok()?;
                        (bytes.len() <= 262_144
                            && self.type_metadata_bytes + bytes.len() as u64 <= SCAN_BYTES)
                            .then_some(bytes)
                    })
                } else {
                    None
                };
                if let Some(bytes) = &bytes {
                    self.type_metadata_bytes += bytes.len() as u64;
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
                self.module_packages.insert(rel.clone(), bytes);
            }
            let lower = rel.to_ascii_lowercase();
            let ts = [".ts", ".tsx", ".mts", ".cts"]
                .iter()
                .any(|e| lower.ends_with(e));
            // Keep existing JavaScript triple-reference coverage, but declaration
            // parsing and the normative source byte budget concern TS-family files.
            let js = [".js", ".jsx", ".mjs", ".cjs"]
                .iter()
                .any(|e| lower.ends_with(e));
            if !ts && !js {
                continue;
            }
            if !metadata.is_file() {
                return Err(std::io::Error::other("P1 ambient scan non-regular source"));
            }
            let remaining = SCAN_BYTES.saturating_sub(self.scan_stats.bytes);
            if metadata.len() > remaining {
                return Err(std::io::Error::other("P1 ambient scan byte budget"));
            }
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(&path)?
                .take(remaining + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > remaining {
                return Err(std::io::Error::other("P1 ambient scan byte budget"));
            }
            self.scan_stats.files += 1;
            self.scan_stats.bytes += bytes.len() as u64;
            if rel.contains("node_modules/@types/") {
                self.scan_stats.types_files += 1;
                self.scan_stats.types_bytes += bytes.len() as u64;
            }
            let source = std::str::from_utf8(&bytes)
                .map_err(|_| std::io::Error::other("P1 ambient scan non-UTF8 source"))?;
            let declares = ts && bytes.windows(7).any(|w| w == b"declare");
            let patterns = if declares {
                super::ambient_patterns(source)
            } else {
                Vec::new()
            };
            let references = super::triple_references(source);
            let declaration = ts
                && (declares
                    || [".d.ts", ".d.mts", ".d.cts"]
                        .iter()
                        .any(|e| lower.ends_with(e)));
            let dependencies = declaration.then(|| module_dependencies(&mut parser, source));
            self.scan_stats.declaration_files += usize::from(declaration);
            self.covered.insert(rel.clone(), 0);
            self.package_hashes.insert(
                format!("scan:{rel}"),
                if declaration || !references.is_empty() {
                    format!("{:x}", Sha256::digest(&bytes))
                } else {
                    "regular".into()
                },
            );
            if !patterns.is_empty() {
                self.ambient.insert(rel.clone(), patterns);
            }
            if !references.is_empty() {
                self.references.insert(rel.clone(), references);
            }
            if let Some(dependencies) = dependencies {
                self.module_dependencies.insert(rel, dependencies);
            }
        }
        Ok(())
    }

    pub(super) fn finish_case_inventory(&mut self) {
        if !self.case_insensitive {
            return;
        }
        for path in self.entries.keys().chain(self.type_entries.keys()) {
            self.folded_entries
                .entry(fold(path))
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
            if self
                .folded_entries
                .get(&fold(&prefix))
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
    pub(crate) fn boundary_closed(&self, config: &str, explicit_types: bool) -> bool {
        let reason = if !explicit_types && self.external_types.values().any(|k| *k != 0) {
            Some("ancestor @types above repository root".to_owned())
        } else {
            self.boundary_failure
                .get_or_init(|| self.global_boundary_failure())
                .clone()
        };
        if let Some(reason) = reason {
            if self.declined_configs.lock().unwrap().insert(config.into()) {
                eprintln!("warning: P1 paths config declined: {config}: {reason}");
            }
            return false;
        }
        true
    }
    fn global_boundary_failure(&self) -> Option<String> {
        if let Some((link, _)) = self.links.iter().find(|(_, target)| target.is_none()) {
            Some(format!("outside-root symlink {link}"))
        } else {
            self.module_dependencies
                .iter()
                .find_map(|(file, dependencies)| {
                    let Some(dependencies) = dependencies else {
                        return Some(format!("unread module dependency in {file}"));
                    };
                    for name in dependencies {
                        if !self.module_dependency_inside(file, name) {
                            return Some(format!(
                                "outside-root module dependency {name:?} from {file}"
                            ));
                        }
                    }
                    // Declaration directives are boundaries regardless of includes.
                    if self.references.get(file).is_some_and(|refs| {
                        refs.iter().any(|(kind, name)| {
                            !self.reference_covered(
                                crate::js_paths_syntax::dir(file),
                                kind,
                                name,
                                None,
                            )
                        })
                    }) {
                        return Some(format!("outside-root declaration reference from {file}"));
                    }
                    None
                })
        }
    }
    fn module_dependency_inside(&self, file: &str, name: &str) -> bool {
        // A bare module can contain a colon (node:fs, bun:test). Only a drive
        // prefix or a rooted/backslash path denotes an external absolute input.
        let drive = name.as_bytes().get(1) == Some(&b':')
            && name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
        if name.starts_with('/') || name.contains('\\') || drive {
            return false;
        }
        if name.starts_with('.') {
            let Some(path) = self.input_path(crate::js_paths_syntax::dir(file), name) else {
                return false;
            };
            let Some(path) = self.link_target(&path) else {
                return false;
            };
            return self.boundary_input_covered(&path);
        }
        if self
            .ambient
            .values()
            .flatten()
            .any(|p| match p.split_once('*') {
                Some((pre, post)) => name.starts_with(pre) && name.ends_with(post),
                None => p == name,
            })
        {
            return true;
        }
        // An existing in-root package cannot leave the capture except through a
        // retained link or a package redirect, both checked explicitly here.
        let mut parts = name.split('/');
        let first = parts.next().unwrap_or_default();
        let package = if name.starts_with('@') {
            format!("{first}/{}", parts.next().unwrap_or_default())
        } else {
            first.into()
        };
        let mut directory = crate::js_paths_syntax::dir(file);
        loop {
            let base = if directory.is_empty() {
                "node_modules".into()
            } else {
                format!("{directory}/node_modules")
            };
            let path = format!("{base}/{package}");
            let Some(path) = self.link_target(&path) else {
                return false;
            };
            if self.type_entries.contains_key(&path) {
                return self.boundary_input_covered(&path);
            }
            if directory.is_empty() {
                break;
            }
            directory = crate::js_paths_syntax::dir(directory);
        }
        let mangled = name.strip_prefix('@').map(|s| s.replacen('/', "__", 1));
        self.external_type_absent(name, mangled.as_deref().unwrap_or(name))
    }
    fn boundary_input_covered(&self, path: &str) -> bool {
        let mut inputs = self.boundary_inputs.lock().unwrap();
        *inputs
            .entry(path.into())
            .or_insert_with(|| self.input_closure_covered(path, None, &mut BTreeSet::new(), false))
    }
}

// The TSX grammar tolerates declaration/source syntax; ambient matching itself
// remains the Unicode-aware over-approximation in the parent module. Comments
// and string contents are not module dependencies.
fn module_dependencies(parser: &mut tree_sitter::Parser, source: &str) -> Option<Vec<String>> {
    let tree = parser.parse(source, None)?;
    let mut pending = vec![tree.root_node()];
    let mut dependencies = BTreeSet::new();
    while let Some(node) = pending.pop() {
        let candidate = match node.kind() {
            "import_statement" | "export_statement" | "import_require_clause" => {
                node.child_by_field_name("source")
            }
            "call_expression" => {
                let name = node
                    .child_by_field_name("function")?
                    .utf8_text(source.as_bytes())
                    .ok()?;
                if ["import", "require"].contains(&name) {
                    let arguments = node.child_by_field_name("arguments")?;
                    let mut cursor = arguments.walk();
                    let argument = arguments
                        .named_children(&mut cursor)
                        .find(|c| c.kind() != "comment");
                    argument
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(candidate) = candidate {
            let text = candidate.utf8_text(source.as_bytes()).ok()?;
            dependencies.insert(super::literal(text)?.0);
        }
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor));
    }
    Some(dependencies.into_iter().collect())
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
    fn dependency_forms_and_literal_content() {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .unwrap();
        for source in [
            "import '/outside';",
            "export * from '/outside';",
            "export type T=import('/outside').T;",
            "import T = require('/outside');",
            "declare const t: typeof import('/outside');",
            "declare const t: any; require('/outside');",
            "import\u{feff}'/outside';",
            "export\u{feff}*\u{feff}from\u{feff}'/outside';",
        ] {
            assert_eq!(
                module_dependencies(&mut p, source),
                Some(vec!["/outside".into()]),
                "{source}"
            );
        }
        assert_eq!(
            module_dependencies(&mut p, "declare const t: any; import(variable);"),
            None
        );
        assert_eq!(
            module_dependencies(
                &mut p,
                "// import '/outside';\nconst text=\"require('/outside')\";"
            ),
            Some(vec![])
        );
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
