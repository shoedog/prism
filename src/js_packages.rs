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

/// Schemes retain a separate classification from unresolved package names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecifierClass {
    Relative,
    Package,
    ExternalBuiltin,
    LoaderScheme,
}
pub fn classify(spec: &str) -> SpecifierClass {
    if spec.starts_with('.') || spec.starts_with('/') {
        SpecifierClass::Relative
    } else if spec.starts_with("node:") {
        SpecifierClass::ExternalBuiltin
    } else if spec.contains(':') {
        SpecifierClass::LoaderScheme
    } else {
        SpecifierClass::Package
    }
}

// Retain source member order. Numeric/duplicate keys are refused rather than
// accidentally treating serde_json's sorted map order as Node condition order.
#[derive(Debug)]
enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    String(String),
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
                Ok(Json::Other)
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
    Blocked,
}
struct Search<'a> {
    snapshot: &'a JsPathsSnapshot,
    indexed: &'a BTreeSet<String>,
    modern: bool,
    esm: bool,
    conditions: BTreeSet<String>,
}
impl Search<'_> {
    fn metadata(&self, p: &str) -> Result<Option<Json>, ()> {
        self.snapshot
            .first_pass_package(&norm(p, "package.json").ok_or(())?)?
            .map(|b| serde_json::from_slice(b).map_err(|_| ()))
            .transpose()
    }
    fn file(&self, p: &str, exact: bool) -> Found {
        // Declaration winners are blockers, never implementation redirects.
        let suffix = [
            ".d.ts", ".d.mts", ".d.cts", ".tsx", ".ts", ".jsx", ".js", ".mts", ".mjs", ".cts",
            ".cjs",
        ]
        .iter()
        .find(|e| p.ends_with(**e));
        let candidates: Vec<String> = match suffix.copied() {
            Some(".js" | ".ts") => [".ts", ".tsx", ".d.ts"]
                .iter()
                .map(|e| format!("{}{e}", &p[..p.len() - 3]))
                .collect(),
            Some(".jsx" | ".tsx") => [".tsx", ".ts", ".d.ts"]
                .iter()
                .map(|e| format!("{}{e}", &p[..p.len() - 4]))
                .collect(),
            Some(".mjs" | ".mts" | ".cjs" | ".cts") => return Found::Blocked,
            Some(_) => vec![p.into()],
            None if exact => vec![p.into()],
            None => [".ts", ".tsx", ".d.ts"]
                .iter()
                .map(|e| format!("{p}{e}"))
                .collect(),
        };
        for q in candidates {
            if self.snapshot.first_pass_absent(&q) {
                continue;
            }
            return if self.snapshot.kind(&q) == Some(0)
                && self.snapshot.unblocked(&q)
                && self.indexed.contains(&q)
                && (q.ends_with(".ts") || q.ends_with(".tsx"))
                && !q.ends_with(".d.ts")
            {
                Found::Source(q)
            } else {
                Found::Blocked
            };
        }
        // JS secondary package lookup is intentionally not admitted by this
        // prototype: proving one missing local TS file is not a full Node10
        // priority-pass absence proof across all ancestors and @types.
        if [p.to_owned(), format!("{p}.js"), format!("{p}.jsx")]
            .iter()
            .any(|q| !self.snapshot.first_pass_absent(q))
        {
            Found::Blocked
        } else {
            Found::Absent
        }
    }
    fn legacy(&self, root: &str, sub: &str) -> Found {
        let p = match norm(root, sub) {
            Some(p) => p,
            None => return Found::Blocked,
        };
        if !sub.is_empty() {
            match self.file(&p, false) {
                Found::Absent => {}
                v => return v,
            }
        }
        let m = match self.metadata(&p) {
            Ok(m) => m,
            Err(()) => return Found::Blocked,
        };
        if m.as_ref().is_some_and(|m| m.get("typesVersions").is_some()) {
            return Found::Blocked;
        }
        if let Some(m) = &m {
            // Native selects typings/types, otherwise main; a missing types
            // target falls to index, not to main. `module` is not read by TS.
            if let Some(entry) = ["typings", "types", "main"]
                .iter()
                .find_map(|k| m.get(k).and_then(Json::string).filter(|s| !s.is_empty()))
            {
                let q = match norm(&p, entry) {
                    Some(q) => q,
                    None => return Found::Blocked,
                };
                match self.file(&q, false) {
                    Found::Absent => {}
                    v => return v,
                }
            }
        }
        if self.esm {
            return Found::Blocked;
        }
        self.file(&format!("{p}/index"), false)
    }
    fn target(&self, root: &str, value: &Json, capture: &str, depth: usize) -> Found {
        if depth > 16 {
            return Found::Blocked;
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
                    return Found::Blocked;
                }
                let target = target.replace('*', capture);
                match norm(root, &target) {
                    Some(p) => self.file(&p, true),
                    None => Found::Blocked,
                }
            }
            Json::Object(entries) => {
                for (condition, v) in entries {
                    if condition.starts_with("types@") || condition.starts_with('.') {
                        return Found::Blocked;
                    }
                    if condition == "default" || self.conditions.contains(condition) {
                        // TS toSearchResult(undefined) is undefined (46646),
                        // so a proved missing leaf tries the next condition.
                        // Declaration/opaque winners still block fallback.
                        match self.target(root, v, capture, depth + 1) {
                            Found::Absent => continue,
                            r => return r,
                        }
                    }
                }
                Found::Absent
            }
            // Arrays/null/invalid targets fail closed in this bounded slice.
            Json::Array(values) => {
                let _ = values;
                Found::Blocked
            }
            Json::Other => Found::Blocked,
        }
    }
    fn entry(&self, root: &str, sub: &str) -> Found {
        let m = match self.metadata(root) {
            Ok(Some(m)) => m,
            Ok(None) => return self.legacy(root, sub),
            Err(()) => return Found::Blocked,
        };
        let Some(exports) = m.get("exports").filter(|_| self.modern) else {
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
                    return Found::Blocked;
                }
                if let Some((_, v)) = entries.iter().find(|(k, _)| k == &key) {
                    return self.target(root, v, "", 0);
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
                return matches
                    .first()
                    .map_or(Found::Blocked, |(_, _, capture, v)| {
                        self.target(root, v, capture, 0)
                    });
            }
        }
        if !sub.is_empty() {
            return Found::Blocked;
        }
        self.target(root, exports, "", 0)
    }
}
pub(crate) fn resolve(
    snapshot: &JsPathsSnapshot,
    options: &BTreeMap<String, Value>,
    file: &str,
    spec: &str,
    indexed: &BTreeSet<String>,
) -> Option<String> {
    if !snapshot.complete
        || classify(spec) != SpecifierClass::Package
        || spec.contains(['\\', '%', '*'])
        || spec
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return None;
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
        return None;
    }
    if snapshot
        .ambient
        .values()
        .any(|v| v.iter().any(|p| crate::js_paths::ambient_matches(p, spec)))
    {
        return None;
    }
    let mode = options
        .get("moduleResolution")?
        .as_str()?
        .to_ascii_lowercase();
    let modern = match mode.as_str() {
        "node" | "node10" => false,
        "node16" | "nodenext" | "bundler" => true,
        _ => return None,
    };
    let parts = spec.split('/').collect::<Vec<_>>();
    let n = if spec.starts_with('@') {
        if parts.len() < 2 {
            return None;
        }
        2
    } else {
        1
    };
    let name = parts[..n].join("/");
    let sub = parts[n..].join("/");
    let mut enclosing = None;
    let mut d = dir(file);
    loop {
        match snapshot.first_pass_package(&norm(d, "package.json")?) {
            Ok(Some(b)) => {
                enclosing = Some((d.to_owned(), serde_json::from_slice::<Json>(b).ok()?));
                break;
            }
            Ok(None) => {}
            Err(()) => return None,
        }
        if d.is_empty() {
            break;
        }
        d = dir(d);
    }
    let esm = if mode == "bundler" {
        // A static import in CommonJS emit uses the require condition even
        // with bundler resolution (TS getEmitSyntaxForUsageLocationWorker).
        // Do not infer a mode for unsupported/default module configurations.
        match options
            .get("module")?
            .as_str()?
            .to_ascii_lowercase()
            .as_str()
        {
            "commonjs" => false,
            "es2015" | "es6" | "es2020" | "es2022" | "esnext" | "preserve" => true,
            _ => return None,
        }
    } else if modern {
        let module = options.get("module")?.as_str()?.to_ascii_lowercase();
        if !["node16", "nodenext"].contains(&module.as_str()) {
            return None;
        }
        enclosing
            .as_ref()
            .and_then(|(_, m)| m.get("type"))
            .and_then(Json::string)
            == Some("module")
    } else {
        false
    };
    let mut conditions = BTreeSet::from([
        if esm { "import" } else { "require" }.into(),
        "types".into(),
    ]);
    if mode != "bundler" {
        conditions.insert("node".into());
    }
    if let Some(custom) = options.get("customConditions") {
        for v in custom.as_array()? {
            conditions.insert(v.as_str()?.into());
        }
    }
    let exports_enabled = modern
        && options
            .get("resolvePackageJsonExports")
            .and_then(Value::as_bool)
            != Some(false);
    let search = Search {
        snapshot,
        indexed,
        modern: exports_enabled,
        esm,
        conditions,
    };
    if exports_enabled {
        if let Some((root, m)) = &enclosing {
            if m.get("name").and_then(Json::string) == Some(name.as_str())
                && m.get("exports").is_some()
            {
                return match search.entry(root, &sub) {
                    Found::Source(s) => Some(s),
                    _ => None,
                };
            }
        }
    }
    let mut d = dir(file);
    loop {
        if d.rsplit('/').next() != Some("node_modules") {
            let lexical = norm(d, &format!("node_modules/{name}"))?;
            // TS consults a root exports map before ordinary file/subpath
            // lookup. Resolve the captured link once, then probe canonical
            // targets; opaque lexical descendants cannot prove their absence.
            if exports_enabled {
                if let Some(root) = snapshot.package_root(&lexical).ok()? {
                    if search
                        .metadata(&root)
                        .ok()?
                        .is_some_and(|m| m.get("exports").is_some())
                    {
                        return match search.entry(&root, &sub) {
                            Found::Source(s) => Some(s),
                            _ => None,
                        };
                    }
                }
            }
            // A sibling module file precedes a package directory. Never
            // bypass external/unindexed/opaque first-pass candidates.
            let candidate = norm(d, &format!("node_modules/{spec}"))?;
            match search.file(&candidate, false) {
                Found::Absent => {}
                Found::Source(s) => return Some(s),
                Found::Blocked => {
                    // A known directory is not a JS file candidate.
                    snapshot.package_root(&lexical).ok().flatten()?;
                    for ext in [".ts", ".tsx", ".d.ts"] {
                        if !snapshot.first_pass_absent(&format!("{candidate}{ext}")) {
                            return None;
                        }
                    }
                }
            }
            match snapshot.package_root(&lexical) {
                Ok(Some(root)) => {
                    return match search.entry(&root, &sub) {
                        Found::Source(s) => Some(s),
                        _ => None,
                    }
                }
                Err(()) => return None,
                Ok(None) => {}
            }
            let mangled = if n == 2 {
                format!("{}__{}", &parts[0][1..], parts[1])
            } else {
                name.clone()
            };
            if !snapshot.first_pass_absent(&norm(d, &format!("node_modules/@types/{mangled}"))?) {
                return None;
            }
        }
        if d.is_empty() {
            break;
        }
        d = dir(d);
    }
    None
}
