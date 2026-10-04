//! Positive package-entry proofs against the immutable lane-P snapshot.
//! TS 5.9.3: 44504 (conditions), 45745 (legacy entries), 45851-46164
//! (exports), 46298-46407 (ancestor lookup). Discovery alone never binds.
use crate::{
    js_paths_snapshot::JsPathsSnapshot,
    js_paths_syntax::{dir, norm},
};
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// Lexical classification only. Paths/ambient authority must be checked before
/// interpreting a scheme as external or proven unresolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecifierClass {
    Relative,
    Package,
    ExternalBuiltin,
    LoaderScheme,
    PackageImport,
}
pub fn classify(spec: &str) -> SpecifierClass {
    if spec.starts_with('.') || spec.starts_with('/') {
        SpecifierClass::Relative
    } else if spec.starts_with('#') {
        SpecifierClass::PackageImport
    } else if spec.starts_with("node:") {
        SpecifierClass::ExternalBuiltin
    } else if spec.contains(':') {
        SpecifierClass::LoaderScheme
    } else {
        SpecifierClass::Package
    }
}

/// Positive source proof, native absence, or an unmodeled native capability.
/// Only `ProvenUnresolved` may justify S2-O9 exclusion. Declaration/external
/// winners and every incomplete proof remain `Unsupported`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Resolution {
    Bound(String),
    ProvenUnresolved,
    Unsupported(String),
}
impl Resolution {
    pub(crate) fn bound(self) -> Option<String> {
        match self {
            Self::Bound(path) => Some(path),
            _ => None,
        }
    }
}

/// Resolve with the same captured writer ownership used by production. The
/// caller supplies the exact indexed source population, never workspace names.
pub fn resolve_import(
    snapshot: &JsPathsSnapshot,
    file: &str,
    spec: &str,
    indexed: &BTreeSet<String>,
) -> (Resolution, Option<String>) {
    let mut resolver = crate::js_paths::Resolver::new(snapshot);
    let owner = resolver.project(file);
    (resolver.resolution(file, spec, indexed), owner)
}

// Retain source member order. Numeric/duplicate keys are refused rather than
// accidentally treating serde_json's sorted map order as Node condition order.
#[derive(Debug)]
enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    String(String),
    Null,
    Other,
}
impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Read;
        impl<'de> Visitor<'de> for Read {
            type Value = Json;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("package JSON")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Json, M::Error> {
                let mut values = Vec::new();
                while let Some((k, v)) = m.next_entry::<String, Json>()? {
                    if k.parse::<u32>().is_ok() || values.iter().any(|(key, _)| key == &k) {
                        return Err(serde::de::Error::custom("ambiguous package key"));
                    }
                    values.push((k, v));
                }
                Ok(Json::Object(values))
            }
            fn visit_seq<S: SeqAccess<'de>>(self, mut s: S) -> Result<Json, S::Error> {
                let mut values = Vec::new();
                while let Some(v) = s.next_element()? {
                    values.push(v);
                }
                Ok(Json::Array(values))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Json, E> {
                Ok(Json::String(v.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Json, E> {
                Ok(Json::Other)
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Json, E> {
                Ok(Json::Other)
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Json, E> {
                Ok(Json::Other)
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Json, E> {
                Ok(Json::Other)
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }
        }
        d.deserialize_any(Read)
    }
}
impl Json {
    fn get(&self, k: &str) -> Option<&Self> {
        match self {
            Self::Object(v) => v.iter().find(|(key, _)| key == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}
#[derive(Debug)]
enum Found {
    Absent,
    Source(String),
    Blocked(&'static str),
}
struct Search<'a> {
    snapshot: &'a JsPathsSnapshot,
    indexed: &'a BTreeSet<String>,
    modern: bool,
    node_esm: bool,
    jsx: bool,
    conditions: BTreeSet<String>,
}
impl Search<'_> {
    fn metadata(&self, p: &str) -> Result<Option<Json>, ()> {
        self.snapshot
            .first_pass_package(&norm(p, "package.json").ok_or(())?)?
            .map(|b| serde_json::from_slice(b).map_err(|_| ()))
            .transpose()
    }
    fn present(&self, q: String) -> Found {
        if self.snapshot.first_pass_absent(&q) {
            return Found::Absent;
        }
        if q.ends_with(".tsx") && !self.jsx {
            return Found::Blocked("JSX compiler option");
        }
        if self.snapshot.kind(&q) == Some(1) {
            return Found::Absent; // tryFile does not admit directories.
        }
        if self.snapshot.kind(&q) == Some(0)
            && self.snapshot.unblocked(&q)
            && self.indexed.contains(&q)
            && (q.ends_with(".ts") || q.ends_with(".tsx"))
            && !q.ends_with(".d.ts")
        {
            Found::Source(q)
        } else {
            Found::Blocked("declaration, unindexed, or opaque winner")
        }
    }
    fn file(&self, p: &str, exact: bool, implicit: bool) -> Found {
        // TS 5.9.3 loadFileNameFromPackageJsonField / tryAddingExtensions.
        let suffix = [
            ".d.ts", ".d.mts", ".d.cts", ".tsx", ".ts", ".jsx", ".js", ".mts", ".mjs", ".cts",
            ".cjs", ".json",
        ]
        .into_iter()
        .find(|e| p.ends_with(e));
        let ts_suffix = suffix.is_some_and(|e| {
            [".ts", ".tsx", ".d.ts", ".mts", ".cts", ".d.mts", ".d.cts"].contains(&e)
        });
        if exact && ts_suffix {
            return self.present(p.into());
        }
        let (stem, extensions): (&str, &[&str]) = match suffix {
            Some(".js" | ".ts" | ".d.ts") => (
                &p[..p.len() - suffix.unwrap().len()],
                &[".ts", ".tsx", ".d.ts"],
            ),
            Some(".jsx" | ".tsx") => (&p[..p.len() - 4], &[".tsx", ".ts", ".d.ts"]),
            Some(".mjs" | ".mts" | ".d.mts") => {
                (&p[..p.len() - suffix.unwrap().len()], &[".mts", ".d.mts"])
            }
            Some(".cjs" | ".cts" | ".d.cts") => {
                (&p[..p.len() - suffix.unwrap().len()], &[".cts", ".d.cts"])
            }
            Some(".json") => (&p[..p.len() - 5], &[".d.json.ts"]),
            _ => (p, &[]),
        };
        for e in extensions {
            match self.present(format!("{stem}{e}")) {
                Found::Absent => {}
                r => return r,
            }
        }
        if suffix.is_none() {
            if let Some((stem, ext)) = p.rsplit_once('.') {
                if !ext.contains('/') {
                    match self.present(format!("{stem}.d.{ext}.ts")) {
                        Found::Absent => {}
                        r => return r,
                    }
                }
            }
        }
        if !exact && implicit {
            for e in [".ts", ".tsx", ".d.ts"] {
                match self.present(format!("{p}{e}")) {
                    Found::Absent => {}
                    r => return r,
                }
            }
        }
        // A missing TS pass does not prove a missing full JS priority pass.
        let js = match suffix {
            Some(".mjs" | ".mts" | ".d.mts") => vec![format!("{stem}.mjs")],
            Some(".cjs" | ".cts" | ".d.cts") => vec![format!("{stem}.cjs")],
            Some(".jsx" | ".tsx") => vec![format!("{stem}.jsx"), format!("{stem}.js")],
            Some(".js" | ".ts" | ".d.ts") => vec![format!("{stem}.js"), format!("{stem}.jsx")],
            _ if !exact && implicit => vec![format!("{p}.js"), format!("{p}.jsx")],
            _ => vec![],
        };
        if js.iter().any(|q| !self.snapshot.first_pass_absent(q)) {
            Found::Blocked("JS secondary priority pass")
        } else {
            Found::Absent
        }
    }
    fn legacy(&self, root: &str, sub: &str) -> Found {
        let Some(p) = norm(root, sub) else {
            return Found::Blocked("legacy path syntax");
        };
        if !sub.is_empty() {
            match self.file(&p, false, !self.node_esm) {
                Found::Absent => {}
                r => return r,
            }
            if self.node_esm {
                return Found::Absent; // ESM subpaths never guess a directory.
            }
        }
        let m = match self.metadata(&p) {
            Ok(m) => m,
            Err(()) => return Found::Blocked("package metadata"),
        };
        if m.as_ref().is_some_and(|m| m.get("typesVersions").is_some()) {
            return Found::Blocked("typesVersions");
        }
        if let Some(m) = &m {
            if let Some(entry) = ["typings", "types", "main"]
                .iter()
                .find_map(|k| m.get(k).and_then(Json::string).filter(|s| !s.is_empty()))
            {
                let Some(q) = norm(&p, entry) else {
                    return Found::Blocked("legacy path syntax");
                };
                // Package fields try the literal TS/declaration first, then
                // ordinary replacement. Only non-module dependencies clear ESM.
                match self.file(&q, true, false) {
                    Found::Absent => {}
                    r => return r,
                }
                let implicit =
                    !self.node_esm || m.get("type").and_then(Json::string) != Some("module");
                match self.file(&q, false, implicit) {
                    Found::Absent => {}
                    r => return r,
                }
                if implicit && self.snapshot.kind(&q) == Some(1) {
                    return Found::Blocked("directory-valued package field");
                }
            }
        }
        if self.node_esm {
            // Native root fallback only with package metadata, no active exports.
            if sub.is_empty() && m.is_some() {
                self.file(&format!("{p}/index.js"), false, false)
            } else {
                Found::Absent
            }
        } else {
            self.file(&format!("{p}/index"), false, true)
        }
    }
    fn target(
        &self,
        root: &str,
        value: &Json,
        capture: &str,
        pattern: bool,
        depth: usize,
    ) -> Found {
        if depth > 16 {
            return Found::Blocked("exports syntax or depth");
        }
        match value {
            Json::String(target) => {
                if !target.starts_with("./")
                    || target.contains(['\\', '%', ':'])
                    || target[2..]
                        .split('/')
                        .any(|p| [".", "..", "node_modules"].contains(&p))
                    || capture
                        .split('/')
                        .any(|p| [".", "..", "node_modules"].contains(&p))
                {
                    return Found::Blocked("exports syntax or depth");
                }
                let target = if pattern {
                    target.replace('*', capture)
                } else {
                    target.clone()
                };
                match norm(root, &target) {
                    Some(p) => self.file(&p, true, false),
                    None => Found::Blocked("exports target path"),
                }
            }
            Json::Object(entries) => {
                for (condition, v) in entries {
                    if condition.starts_with("types@") || condition.starts_with('.') {
                        return Found::Blocked("exports syntax or depth");
                    }
                    if condition == "default" || self.conditions.contains(condition) {
                        // TS toSearchResult(undefined) is undefined (46646),
                        // so a proved missing leaf tries the next condition.
                        // Declaration/opaque winners still block fallback.
                        match self.target(root, v, capture, pattern, depth + 1) {
                            Found::Absent => continue,
                            r => return r,
                        }
                    }
                }
                Found::Absent
            }
            // Arrays and invalid targets remain unsupported; null is a known no-result.
            Json::Array(values) => {
                let _ = values;
                Found::Blocked("export arrays")
            }
            Json::Null => Found::Absent,
            Json::Other => Found::Blocked("invalid export value"),
        }
    }
    fn entry(&self, root: &str, sub: &str) -> Found {
        let m = match self.metadata(root) {
            Ok(Some(m)) => m,
            Ok(None) => return self.legacy(root, sub),
            Err(()) => return Found::Blocked("package metadata"),
        };
        let Some(exports) = m
            .get("exports")
            .filter(|v| self.modern && !matches!(v, Json::Null))
        else {
            return self.legacy(root, sub);
        };
        let key = if sub.is_empty() {
            ".".into()
        } else {
            format!("./{sub}")
        };
        if let Json::Object(entries) = exports {
            let dotted = entries.iter().any(|(k, _)| k.starts_with('.'));
            if dotted {
                if entries.iter().any(|(k, _)| !k.starts_with('.')) {
                    return Found::Blocked("exports syntax or depth");
                }
                if let Some((_, v)) = entries.iter().find(|(k, _)| k == &key) {
                    return self.target(root, v, "", false, 0);
                }
                let mut matches = entries
                    .iter()
                    .filter_map(|(k, v)| {
                        let (pre, post) = k.split_once('*')?;
                        if post.contains('*')
                            || !key.starts_with(pre)
                            || !key.ends_with(post)
                            || key.len() < pre.len() + post.len()
                        {
                            return None;
                        }
                        Some((
                            pre.len(),
                            k.len(),
                            &key[pre.len()..key.len() - post.len()],
                            v,
                        ))
                    })
                    .collect::<Vec<_>>();
                matches.sort_by_key(|(pre, len, _, _)| std::cmp::Reverse((*pre, *len)));
                return matches.first().map_or(Found::Absent, |(_, _, capture, v)| {
                    self.target(root, v, capture, true, 0)
                });
            }
        }
        if !sub.is_empty() {
            return Found::Absent;
        }
        self.target(root, exports, "", false, 0)
    }
}
/// Static-import condition mode and Node ESM implicit-extension policy are
/// separate: bundler may select import without enforcing Node ESM file rules.
pub(crate) fn usage_mode(
    snapshot: &JsPathsSnapshot,
    options: &BTreeMap<String, Value>,
    file: &str,
) -> Result<(bool, bool, bool), &'static str> {
    let mode = options
        .get("moduleResolution")
        .and_then(Value::as_str)
        .ok_or("default moduleResolution")?
        .to_ascii_lowercase();
    let modern = match mode.as_str() {
        "node" | "node10" => false,
        "node16" | "nodenext" | "bundler" => true,
        _ => return Err("moduleResolution"),
    };
    if !modern {
        return Ok((false, false, false));
    }
    let module = options
        .get("module")
        .and_then(Value::as_str)
        .ok_or("default module")?
        .to_ascii_lowercase();
    let default_esm = if mode == "bundler" {
        match module.as_str() {
            "commonjs" => false,
            "es2015" | "es6" | "es2020" | "es2022" | "esnext" | "preserve" => true,
            _ => return Err("module emit mode"),
        }
    } else {
        if !["node16", "nodenext"].contains(&module.as_str()) {
            return Err("module emit mode");
        }
        false
    };
    let esm = if file.ends_with(".mjs") || file.ends_with(".mts") {
        true
    } else if file.ends_with(".cjs") || file.ends_with(".cts") {
        false
    } else if mode == "bundler" {
        default_esm
    } else {
        let package = enclosing(snapshot, file)?.map(|(_, m)| m);
        let outer;
        let m = if let Some(m) = &package {
            Some(m)
        } else {
            outer = snapshot
                .outer_package()
                .map_err(|()| "outer package scope")?
                .map(serde_json::from_slice::<Json>)
                .transpose()
                .map_err(|_| "outer package scope")?;
            outer.as_ref()
        };
        m.and_then(|m| m.get("type")).and_then(Json::string) == Some("module")
    };
    Ok((modern, esm, esm && mode != "bundler"))
}
fn enclosing(
    snapshot: &JsPathsSnapshot,
    file: &str,
) -> Result<Option<(String, Json)>, &'static str> {
    let mut d = dir(file);
    loop {
        let p = norm(d, "package.json").ok_or("package scope syntax")?;
        match snapshot.first_pass_package(&p) {
            Ok(Some(b)) => {
                return Ok(Some((
                    d.into(),
                    serde_json::from_slice(b).map_err(|_| "package scope metadata")?,
                )))
            }
            Ok(None) => {}
            Err(()) => return Err("package scope metadata"),
        }
        if d.is_empty() {
            return Ok(None);
        }
        d = dir(d);
    }
}
fn result(found: Found) -> Resolution {
    match found {
        Found::Source(s) => Resolution::Bound(s),
        Found::Absent => Resolution::ProvenUnresolved,
        Found::Blocked(reason) => Resolution::Unsupported(reason.into()),
    }
}
pub(crate) fn resolve(
    snapshot: &JsPathsSnapshot,
    options: &BTreeMap<String, Value>,
    file: &str,
    spec: &str,
    indexed: &BTreeSet<String>,
) -> Resolution {
    let unsupported = |s: &str| Resolution::Unsupported(s.into());
    if !snapshot.complete {
        return unsupported("snapshot incomplete");
    }
    if [
        "baseUrl",
        "rootDirs",
        "moduleSuffixes",
        "noResolve",
        "preserveSymlinks",
        "typeRoots",
        "types",
        "noDtsResolution",
        "rootDir",
        "outDir",
        "declarationDir",
    ]
    .iter()
    .any(|k| options.contains_key(*k))
    {
        return unsupported("compiler option authority");
    }
    if snapshot
        .ambient
        .values()
        .any(|v| v.iter().any(|p| crate::js_paths::ambient_matches(p, spec)))
    {
        return unsupported("ambient module authority");
    }
    match classify(spec) {
        SpecifierClass::PackageImport => return unsupported("package imports"),
        SpecifierClass::ExternalBuiltin | SpecifierClass::LoaderScheme => {}
        SpecifierClass::Relative => return unsupported("relative package request"),
        SpecifierClass::Package => {}
    }
    if spec.contains(['\\', '%', '*'])
        || spec
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return unsupported("specifier syntax");
    }
    let (modern, esm, node_esm) = match usage_mode(snapshot, options, file) {
        Ok(v) => v,
        Err(r) => return unsupported(r),
    };
    let parts = spec.split('/').collect::<Vec<_>>();
    let n = if spec.starts_with('@') {
        if parts.len() < 2 {
            return unsupported("package name");
        }
        2
    } else {
        1
    };
    let name = parts[..n].join("/");
    let sub = parts[n..].join("/");
    let mut conditions = BTreeSet::from([
        if esm { "import" } else { "require" }.into(),
        "types".into(),
    ]);
    if options
        .get("moduleResolution")
        .and_then(Value::as_str)
        .is_some_and(|m| !m.eq_ignore_ascii_case("bundler"))
    {
        conditions.insert("node".into());
    }
    if let Some(custom) = options.get("customConditions") {
        let Some(values) = custom.as_array() else {
            return unsupported("customConditions");
        };
        for v in values {
            let Some(v) = v.as_str() else {
                return unsupported("customConditions");
            };
            conditions.insert(v.into());
        }
    }
    let exports_enabled = modern
        && options
            .get("resolvePackageJsonExports")
            .and_then(Value::as_bool)
            != Some(false);
    let mut search = Search {
        snapshot,
        indexed,
        modern: exports_enabled,
        node_esm,
        jsx: options.contains_key("jsx"),
        conditions,
    };
    // SelfName is a separate feature; disabling ordinary exports does not disable it.
    if modern {
        match enclosing(snapshot, file) {
            Ok(Some((root, m)))
                if m.get("name").and_then(Json::string) == Some(name.as_str())
                    && m.get("exports").is_some_and(|v| !matches!(v, Json::Null)) =>
            {
                search.modern = true;
                let found = search.entry(&root, &sub);
                search.modern = exports_enabled;
                match found {
                    Found::Absent => {}
                    v => return result(v),
                }
            }
            Err(r) => return unsupported(r),
            Ok(None) => {
                // TS self-reference precedes URI classification and may live
                // outside the captured repository. Do not bypass that scope.
                let outer = match snapshot.outer_package() {
                    Ok(b) => b,
                    Err(()) => return unsupported("outer package scope"),
                };
                if let Some(b) = outer {
                    let m: Json = match serde_json::from_slice(b) {
                        Ok(m) => m,
                        Err(_) => return unsupported("outer package scope"),
                    };
                    if m.get("name").and_then(Json::string) == Some(name.as_str())
                        && m.get("exports").is_some_and(|v| !matches!(v, Json::Null))
                    {
                        return unsupported("outside-root self-reference");
                    }
                }
            }
            _ => {}
        }
    }
    if spec.contains(':') {
        return Resolution::ProvenUnresolved;
    }
    let mut d = dir(file);
    loop {
        if d.rsplit('/').next() != Some("node_modules") {
            let Some(lexical) = norm(d, &format!("node_modules/{name}")) else {
                return unsupported("package path");
            };
            let root = match snapshot.package_root(&lexical) {
                Ok(root) => root,
                Err(()) => return unsupported("package link or external authority"),
            };
            let has_exports = if let Some(root) = &root {
                match search.metadata(root) {
                    Ok(m) => {
                        let active_exports = exports_enabled
                            && m.as_ref().is_some_and(|m| {
                                m.get("exports").is_some_and(|v| !matches!(v, Json::Null))
                            });
                        // Root version paths precede canonical legacy subpath
                        // file probes. entry() is already too late to fence them.
                        if !active_exports
                            && m.as_ref().is_some_and(|m| m.get("typesVersions").is_some())
                        {
                            return unsupported("typesVersions");
                        }
                        active_exports
                    }
                    Err(()) => return unsupported("package metadata"),
                }
            } else {
                false
            };
            let found = if has_exports {
                search.entry(root.as_ref().unwrap(), &sub)
            } else {
                // Root sibling files precede legacy directories. For linked
                // subpaths, prove root siblings then probe canonical descendants.
                let sibling = if sub.is_empty() {
                    lexical.clone()
                } else if let Some(root) = &root {
                    match search.file(&lexical, false, !node_esm) {
                        Found::Absent => {}
                        v => return result(v),
                    }
                    match norm(root, &sub) {
                        Some(p) => p,
                        None => return unsupported("package subpath"),
                    }
                } else {
                    match norm(d, &format!("node_modules/{spec}")) {
                        Some(p) => p,
                        None => return unsupported("package subpath"),
                    }
                };
                match search.file(&sibling, false, !node_esm) {
                    Found::Absent => {}
                    v => return result(v),
                }
                root.map_or(Found::Absent, |root| search.entry(&root, &sub))
            };
            match found {
                Found::Absent => {}
                v => return result(v),
            }
            let mangled = if n == 2 {
                format!("{}__{}", &parts[0][1..], parts[1])
            } else {
                name.clone()
            };
            let Some(types) = norm(d, &format!("node_modules/@types/{mangled}")) else {
                return unsupported("types package path");
            };
            if !snapshot.first_pass_absent(&types) {
                return unsupported("@types authority");
            }
        }
        if d.is_empty() {
            break;
        }
        d = dir(d);
    }
    if snapshot.external_first_pass_absent() {
        Resolution::ProvenUnresolved
    } else {
        unsupported("external node_modules ancestry")
    }
}

/// Node ESM export-hop file substitution, without implicit extensions/index.
pub(crate) fn relative_esm(
    snapshot: &JsPathsSnapshot,
    from: &str,
    spec: &str,
    indexed: &BTreeSet<String>,
) -> Option<String> {
    let p = norm(dir(from), spec)?;
    let search = Search {
        snapshot,
        indexed,
        modern: true,
        node_esm: true,
        jsx: true,
        conditions: BTreeSet::new(),
    };
    result(search.file(&p, false, false)).bound()
}
