//! Absence proof for TypeScript 5.9.3's Node10 priority pass.
//! Source: lib/typescript.js SHA256
//! 3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675.
//! 45240-45243: priority pass; 45287-45336: optional paths/baseUrl,
//! node_modules, then custom typeRoots; 45423-45503: file extensions;
//! 45745-45813: package typings/types/main, typesVersions and index;
//! 46298-46445: ancestor files/directories, @types and paths substitutions;
//! 44284-44286,46573-46596: custom-root declaration lookup and mangling.
use crate::{
    js_paths_snapshot::JsPathsSnapshot,
    js_paths_syntax::{dir, norm},
};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{fmt, marker::PhantomData};

// JSON member order is significant for TypeScript's first matching version
// range and equal-prefix patterns. serde_json's default map sorts keys.
struct Ordered<T>(Vec<(String, T)>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Ordered<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Object<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Object<T> {
            type Value = Ordered<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Self::Value, M::Error> {
                let mut entries: Vec<(String, T)> = Vec::new();
                while let Some((key, value)) = m.next_entry::<String, T>()? {
                    if let Some((_, prior)) = entries.iter_mut().find(|(name, _)| name == &key) {
                        *prior = value;
                    } else {
                        entries.push((key, value));
                    }
                }
                // ECMAScript enumerates canonical array-index keys first,
                // numerically, then other keys in insertion order. A duplicate
                // replaces its value without changing its original position.
                entries.sort_by_key(|(key, _)| {
                    key.parse::<u32>()
                        .ok()
                        .filter(|n| *n != u32::MAX && n.to_string() == *key)
                        .map_or((1, 0), |n| (0, n))
                });
                Ok(Ordered(entries))
            }
        }
        d.deserialize_map(Object(PhantomData))
    }
}
#[derive(Deserialize)]
struct Package {
    #[serde(default)]
    typings: Value,
    #[serde(default)]
    types: Value,
    #[serde(default)]
    main: Value,
    #[serde(default, rename = "typesVersions")]
    versions: Option<Ordered<Ordered<Value>>>,
}
struct Pass<'a> {
    snapshot: &'a JsPathsSnapshot,
}
impl Pass<'_> {
    fn probe(&self, p: &str) -> bool {
        self.snapshot.first_pass_absent(p)
    }
    fn extensions(&self, stem: &str, original: &str, ts: bool) -> bool {
        let suffixes: &[&str] = match original {
            ".mjs" | ".mts" | ".d.mts" => {
                if ts {
                    &[".mts", ".d.mts"]
                } else {
                    &[".d.mts"]
                }
            }
            ".cjs" | ".cts" | ".d.cts" => {
                if ts {
                    &[".cts", ".d.cts"]
                } else {
                    &[".d.cts"]
                }
            }
            ".json" => &[".d.json.ts"],
            ".tsx" | ".jsx" => {
                if ts {
                    &[".tsx", ".ts", ".d.ts"]
                } else {
                    &[".d.ts"]
                }
            }
            ".ts" | ".d.ts" | ".js" | "" => {
                if ts {
                    &[".ts", ".tsx", ".d.ts"]
                } else {
                    &[".d.ts"]
                }
            }
            _ => return self.probe(&format!("{stem}.d{original}.ts")),
        };
        suffixes.iter().fold(true, |absent, ext| {
            self.probe(&format!("{stem}{ext}")) & absent
        })
    }
    fn file(&self, p: &str, ts: bool) -> bool {
        if p.is_empty() {
            return self.snapshot.first_pass_root_file_absent(ts);
        }
        let mut absent = true;
        if let Some((stem, extension)) = split_extension(p) {
            absent &= self.extensions(stem, extension, ts);
        }
        // loadModuleFromFile also appends implicit extensions to the full name.
        absent & self.extensions(p, "", ts)
    }
    fn paths(
        &self,
        name: &str,
        base: &str,
        paths: &Ordered<Value>,
        ts: bool,
        depth: usize,
        package: Option<(&str, &Package)>,
    ) -> Result<Option<bool>, ()> {
        let selected = paths
            .0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(key, value)| (key, value, ""))
            .or_else(|| {
                let mut best = None;
                for (key, value) in &paths.0 {
                    let Some((pre, post)) = key.split_once('*') else {
                        continue;
                    };
                    if post.contains('*')
                        || !name.starts_with(pre)
                        || !name.ends_with(post)
                        || name.len() < pre.len() + post.len()
                    {
                        continue;
                    }
                    if best
                        .as_ref()
                        .is_none_or(|(old, _, _): &(&String, &Value, &str)| {
                            old.split_once('*').unwrap().0.len() < pre.len()
                        })
                    {
                        best = Some((key, value, &name[pre.len()..name.len() - post.len()]));
                    }
                }
                best
            });
        let Some((key, values, capture)) = selected else {
            return Ok(None);
        };
        // An empty exact key is falsy in tryLoadModuleUsingPaths; a wildcard
        // pattern with an empty capture is still a truthy pattern object.
        if key.is_empty() {
            return Ok(None);
        }
        let mut absent = true;
        for target in values.as_array().ok_or(())? {
            let target = target.as_str().ok_or(())?;
            let target = if capture.is_empty() {
                target.to_owned()
            } else {
                target.replacen('*', capture, 1)
            };
            let p = self.snapshot.input_path(base, &target).ok_or(())?;
            // tryLoadModuleUsingPaths first tries the literal known-extension
            // substitution even in the priority pass (46430-46438).
            if known_extension(&p) {
                absent &= self.probe(&p);
            }
            absent &= if let Some(package) = package {
                self.file(&p, ts) & self.directory_worker(&p, ts, depth + 1, Some(package))?
            } else {
                // Package-field loader: no nested package.json, and expand
                // Declaration to TypeScript for the file/folder fallback.
                self.file(&p, true) & self.directory_worker(&p, ts, depth + 1, None)?
            };
        }
        Ok(Some(absent))
    }
    fn package(&self, p: &str) -> Result<Option<Package>, ()> {
        self.snapshot
            .first_pass_package(&format!("{p}/package.json"))?
            .map(|bytes| serde_json::from_slice(bytes).map_err(|_| ()))
            .transpose()
    }
    fn directory(&self, p: &str, ts: bool, depth: usize) -> Result<bool, ()> {
        let package = self.package(p)?;
        self.directory_worker(p, ts, depth, package.as_ref().map(|package| (p, package)))
    }
    fn directory_worker(
        &self,
        p: &str,
        ts: bool,
        depth: usize,
        package: Option<(&str, &Package)>,
    ) -> Result<bool, ()> {
        if depth >= 32 {
            return Err(());
        }
        let index = norm(p, "index").ok_or(())?;
        if let Some((root, package)) = package {
            let field = (root == p)
                .then(|| {
                    [&package.typings, &package.types, &package.main]
                        .into_iter()
                        .filter_map(Value::as_str)
                        .find(|s| !s.is_empty())
                })
                .flatten();
            let entry = field
                .map(|s| self.snapshot.input_path(p, s).ok_or(()))
                .transpose()?;
            if let Some(versions) = &package.versions {
                let selected = versions
                    .0
                    .iter()
                    .find(|(range, _)| range_matches(range) == Some(true));
                if let Some((_, paths)) = selected {
                    let entry = entry.as_deref().unwrap_or(&index);
                    // 45789: only contained entries are remapped. A readable
                    // entry elsewhere in the captured root uses the field loader.
                    let relative = if entry == p {
                        Some("")
                    } else {
                        entry.strip_prefix(&format!("{p}/"))
                    };
                    if let Some(relative) = relative {
                        if let Some(absent) = self.paths(relative, p, paths, ts, depth, None)? {
                            return Ok(absent);
                        }
                    }
                }
            }
            if let Some(entry) = entry {
                // Declaration-only package fields expand to TS implementations
                // after their direct filename probe (45757-45777).
                let direct = if known_ts_extension(&entry) {
                    self.probe(&entry)
                } else {
                    split_extension(&entry).is_none_or(|(stem, ext)| self.extensions(stem, ext, ts))
                };
                return Ok(direct
                    & self.file(&entry, true)
                    & self.directory_worker(&entry, ts, depth + 1, None)?
                    & self.file(&index, ts));
            }
        }
        Ok(self.file(&index, ts))
    }
    fn relative(&self, p: &str, ts: bool, depth: usize) -> Result<bool, ()> {
        Ok(self.file(p, ts) & self.directory(p, ts, depth)?)
    }
    fn module(&self, base: &str, spec: &str, ts: bool) -> Result<bool, ()> {
        let p = norm(base, spec).ok_or(())?;
        // A subpath package has precedence over root typesVersions.
        if spec_rest(spec).is_some() && self.package(&p)?.is_none() {
            let (name, rest) = spec_rest(spec).unwrap();
            let root = norm(base, name).ok_or(())?;
            if let Some(package) = self.package(&root)? {
                if let Some(versions) = &package.versions {
                    if let Some((_, paths)) = versions
                        .0
                        .iter()
                        .find(|(range, _)| range_matches(range) == Some(true))
                    {
                        if let Some(absent) =
                            self.paths(rest, &root, paths, ts, 0, Some((&root, &package)))?
                        {
                            return Ok(absent);
                        }
                    }
                }
            }
        }
        self.relative(&p, ts, 0)
    }
}
fn spec_rest(spec: &str) -> Option<(&str, &str)> {
    let offset = if spec.starts_with('@') {
        spec.find('/')? + 1
    } else {
        0
    };
    let slash = offset + spec[offset..].find('/')?;
    Some((&spec[..slash], &spec[slash + 1..]))
}
fn split_extension(p: &str) -> Option<(&str, &str)> {
    for ext in [".d.mts", ".d.cts", ".d.ts"] {
        if let Some(stem) = p.strip_suffix(ext) {
            return Some((stem, ext));
        }
    }
    let index = p.rfind('.')?;
    (!p[index..].contains('/')).then_some((&p[..index], &p[index..]))
}
fn known_ts_extension(p: &str) -> bool {
    [".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .any(|e| p.ends_with(e))
}
fn known_extension(p: &str) -> bool {
    known_ts_extension(p)
        || [".js", ".jsx", ".mjs", ".cjs", ".json"]
            .iter()
            .any(|e| p.ends_with(e))
}
fn mangle(spec: &str) -> String {
    spec.strip_prefix('@')
        .filter(|s| s.contains('/'))
        .map_or_else(|| spec.into(), |s| s.replacen('/', "__", 1))
}
pub(crate) fn absent(
    snapshot: &JsPathsSnapshot,
    file: &str,
    spec: &str,
    target: &str,
    roots: Option<&[String]>,
) -> bool {
    let pass = Pass { snapshot };
    let proof = || -> Result<bool, ()> {
        let mut absent = pass.relative(target, true, 0)?;
        // A matched paths pattern returns a search result even on a miss;
        // optional settings do not fall through to baseUrl (44966-44978).
        let mut directory = dir(file);
        loop {
            if directory.rsplit('/').next() != Some("node_modules") {
                let modules = norm(directory, "node_modules").ok_or(())?;
                absent &= pass.module(&modules, spec, true)?;
                absent &= pass.module(&format!("{modules}/@types"), &mangle(spec), false)?;
            }
            if directory.is_empty() {
                break;
            }
            directory = dir(directory);
        }
        for root in roots.unwrap_or_default() {
            let name = if root == "node_modules/@types" || root.ends_with("/node_modules/@types") {
                mangle(spec)
            } else {
                spec.into()
            };
            absent &= pass.relative(&norm(root, &name).ok_or(())?, false, 0)?;
        }
        Ok(absent & snapshot.external_first_pass_absent())
    };
    proof().unwrap_or(false)
}

// Fixed-version port of TS's VersionRange parser, typescript.js:4935-5037.
// Build metadata does not affect comparisons. 5.9.3 is above its prereleases.
fn partial(text: &str) -> Option<([f64; 3], usize, bool)> {
    let (text, build) = text
        .split_once('+')
        .map_or((text, None), |(a, b)| (a, Some(b)));
    let (text, pre) = text
        .split_once('-')
        .map_or((text, None), |(a, b)| (a, Some(b)));
    for (part, prerelease) in [(build, false), (pre, true)] {
        if let Some(part) = part {
            if part.is_empty()
                || part.split('.').any(|id| {
                    id.is_empty()
                        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                        || (prerelease
                            && id.len() > 1
                            && id.starts_with('0')
                            && id.bytes().all(|c| c.is_ascii_digit()))
                })
            {
                return None;
            }
        }
    }
    let parts: Vec<_> = text.split('.').collect();
    if parts.len() > 3 || (parts.len() != 3 && (pre.is_some() || build.is_some())) {
        return None;
    }
    let mut v = [0.0; 3];
    let mut wildcard = 3;
    for (i, part) in parts.iter().enumerate() {
        if ["*", "x", "X"].contains(part) {
            wildcard = wildcard.min(i);
        } else {
            if part.is_empty()
                || !part.bytes().all(|c| c.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                return None;
            }
            let n = part.parse().ok()?;
            if i < wildcard {
                v[i] = n;
            }
        }
    }
    wildcard = wildcard.min(parts.len());
    Some((v, wildcard, pre.is_some()))
}
// ECMAScript trim/\s (4937-4946): includes BOM, excludes Unicode NEL.
fn version_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
fn range_matches(text: &str) -> Option<bool> {
    let current = [5.0, 9.0, 3.0];
    let mut any = false;
    let mut alternatives = 0;
    let compare = |v: [f64; 3], pre: bool, op: &str| {
        let cmp = current
            .partial_cmp(&v)
            .expect("decimal versions cannot be NaN");
        let cmp = if cmp.is_eq() && pre {
            std::cmp::Ordering::Greater
        } else {
            cmp
        };
        match op {
            "<" => cmp.is_lt(),
            "<=" => !cmp.is_gt(),
            ">" => cmp.is_gt(),
            ">=" => !cmp.is_lt(),
            _ => cmp.is_eq(),
        }
    };
    let increment = |mut v: [f64; 3], index: usize| -> Option<[f64; 3]> {
        v[index] += 1.0;
        for item in &mut v[index + 1..] {
            *item = 0.0;
        }
        Some(v)
    };
    for alternative in text
        .trim_matches(version_whitespace)
        .split("||")
        .filter(|s| !s.is_empty())
    {
        alternatives += 1;
        let mut matches = true;
        let tokens: Vec<_> = alternative
            .split(version_whitespace)
            .filter(|s| !s.is_empty())
            .collect();
        // TS skips an empty alternative before trim, but a nonempty all-space
        // alternative reaches rangeRegExp and invalidates the entire range.
        if tokens.is_empty() {
            return None;
        }
        if tokens.len() == 3 && tokens[1] == "-" {
            let (left, w, pre) = partial(tokens[0])?;
            if w > 0 {
                matches &= compare(left, pre, ">=");
            }
            let (right, w, pre) = partial(tokens[2])?;
            if w > 0 {
                matches &= if w < 3 {
                    compare(increment(right, w - 1)?, false, "<")
                } else {
                    compare(right, pre, "<=")
                };
            }
        } else {
            for token in tokens {
                let n = token.bytes().take_while(|c| b"~^<>= ".contains(c)).count();
                let (op, value) = token.split_at(n);
                if !["", "=", "~", "^", "<", ">", "<=", ">="].contains(&op) {
                    return None;
                }
                let (v, w, pre) = partial(value)?;
                if w == 0 {
                    matches &= !["<", ">"].contains(&op);
                    continue;
                }
                matches &= match op {
                    "~" | "^" => {
                        let i = if op == "~" {
                            if w == 1 {
                                0
                            } else {
                                1
                            }
                        } else if v[0] > 0.0 || w == 1 {
                            0
                        } else if v[1] > 0.0 || w == 2 {
                            1
                        } else {
                            2
                        };
                        compare(v, pre, ">=") && compare(increment(v, i)?, false, "<")
                    }
                    "<" | ">=" => compare(v, pre || w < 3, op),
                    "<=" | ">" if w < 3 => compare(
                        increment(v, w - 1)?,
                        true,
                        if op == "<=" { "<" } else { ">=" },
                    ),
                    "<=" | ">" => compare(v, pre, op),
                    _ if w < 3 => {
                        compare(v, true, ">=") && compare(increment(v, w - 1)?, true, "<")
                    }
                    _ => compare(v, pre, "="),
                };
            }
        }
        any |= matches;
    }
    Some(any || alternatives == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn write(root: &std::path::Path, name: &str, text: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    #[test]
    fn native_version_range_parity() {
        let cases: Vec<(String, Option<bool>)> = serde_json::from_str(include_str!(
            "../tests/integration/fixtures/js_paths_first_pass_ranges.json"
        ))
        .unwrap();
        for (range, expected) in cases {
            assert_eq!(range_matches(&range), expected, "{range}");
        }
    }
    #[test]
    fn native_whitespace_population() {
        let expected: Vec<u32> = serde_json::from_str(include_str!(
            "../tests/integration/fixtures/js_paths_version_whitespace.json"
        ))
        .unwrap();
        for code in 0..=0x10ffff {
            if let Some(c) = char::from_u32(code) {
                assert_eq!(
                    version_whitespace(c),
                    expected.contains(&code),
                    "U+{code:04X}"
                );
            }
        }
    }
    #[test]
    fn every_location_is_an_occupancy_dependency() {
        let d = TempDir::new().unwrap();
        write(d.path(), "tsconfig.json", "{}");
        write(
            d.path(),
            "node_modules/utils/package.json",
            r#"{"types":"decls/entry"}"#,
        );
        write(
            d.path(),
            "node_modules/@types/utils/package.json",
            r#"{"typings":"decls/types"}"#,
        );
        write(
            d.path(),
            "types/utils/package.json",
            r#"{"typesVersions":{"*":{"*":["decls/*"]}}}"#,
        );
        let s = JsPathsSnapshot::capture(d.path());
        assert!(absent(
            &s,
            "src/app.tsx",
            "utils",
            "lib/real",
            Some(&["types".into()])
        ));
        for path in [
            "lib/real.ts",
            "lib/real/index.d.ts",
            "src/node_modules/utils.d.ts",
            "node_modules/utils.d.ts",
            "node_modules/utils/index.d.ts",
            "node_modules/utils/decls/entry.d.ts",
            "node_modules/@types/utils/index.d.ts",
            "node_modules/@types/utils/decls/types.d.ts",
            "types/utils.d.ts",
            "types/utils/decls/index.d.ts",
        ] {
            assert!(s.was_probed(path), "missing dependency: {path}");
        }
    }

    #[test]
    fn physical_alias_occupancy_enters_cache_dependencies() {
        let d = TempDir::new().unwrap();
        write(d.path(), "tsconfig.json", "{}");
        std::fs::create_dir(d.path().join("node_modules")).unwrap();
        let before = JsPathsSnapshot::capture(d.path());
        assert!(before.first_pass_absent("node_modules/utils.d.ts"));
        write(d.path(), "node_modules/UTILS.D.TS", "export {};");
        let after = JsPathsSnapshot::capture(d.path());
        let alias = d.path().join("node_modules/utils.d.ts").exists();
        assert_eq!(after.first_pass_absent("node_modules/utils.d.ts"), !alias);
        if alias {
            // The uppercase extension is absent from the legacy occupancy
            // filter and source scan. The requested-path fact must invalidate.
            assert_ne!(before.topology(), after.topology());
        }
        for path in [
            "",
            "/outside.d.ts",
            "../outside.d.ts",
            "node_modules/../outside.d.ts",
        ] {
            assert!(!after.first_pass_absent(path));
        }
    }
}
