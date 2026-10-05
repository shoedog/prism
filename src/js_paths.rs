//! P1: conservative paths-only module proof for named/default ESM import members.
use crate::js_paths_snapshot::JsPathsSnapshot;
use crate::js_paths_syntax::{dir, exclude_matches, jsonc, norm, pattern_matches};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
#[derive(Clone, Default)]
struct Config {
    options: BTreeMap<String, Value>,
    config_path: String,
    paths: Option<(String, BTreeMap<String, Vec<String>>)>,
    base: Option<String>,
    files: Option<Vec<String>>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    type_roots: Option<Vec<String>>,
    types_origin: String,
}
fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array()?
        .iter()
        .map(|x| x.as_str().map(String::from))
        .collect()
}
pub(crate) struct Resolver<'a> {
    snapshot: &'a JsPathsSnapshot,
    configs: BTreeMap<String, Option<Rc<Config>>>,
    selected: BTreeMap<String, Option<Rc<Config>>>,
    qualifier_mode: bool,
    refusal_modules: BTreeMap<(String, String, String), crate::js_import_qualifiers::RefusalModule>,
}
impl<'a> Resolver<'a> {
    pub(crate) fn new(snapshot: &'a JsPathsSnapshot) -> Self {
        Self {
            snapshot,
            configs: BTreeMap::new(),
            selected: BTreeMap::new(),
            qualifier_mode: false,
            refusal_modules: BTreeMap::new(),
        }
    }
    pub(crate) fn for_qualifiers(snapshot: &'a JsPathsSnapshot) -> Self {
        let mut resolver = Self::new(snapshot);
        resolver.qualifier_mode = true;
        resolver
    }
    fn config(&mut self, p: &str, seen: &mut BTreeSet<String>) -> Option<Rc<Config>> {
        if let Some(c) = self.configs.get(p) {
            return c.clone();
        }
        if seen.len() >= 16 || !seen.insert(p.into()) {
            return None;
        }
        let result = self.config_inner(p, seen).map(Rc::new);
        seen.remove(p);
        self.configs.insert(p.into(), result.clone());
        result
    }
    fn config_inner(&mut self, p: &str, seen: &mut BTreeSet<String>) -> Option<Config> {
        let value = jsonc(self.snapshot.configs.get(p)?.as_ref()?)?;
        let object = value.as_object()?;
        if (object.contains_key("references")
            && !(self.qualifier_mode
                && object
                    .get("references")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)))
            || object
                .get("files")
                .is_some_and(|v| v.as_array().is_some_and(Vec::is_empty))
        {
            return None;
        }
        let mut c = if let Some(ext) = object.get("extends") {
            let ext = ext.as_str()?;
            if !ext.starts_with('.') {
                return None;
            }
            let mut parent = norm(dir(p), ext)?;
            if !parent.ends_with(".json") {
                parent.push_str(".json");
            }
            self.config(&parent, seen)?.as_ref().clone()
        } else {
            Config::default()
        };
        for (key, slot) in [
            ("files", &mut c.files),
            ("include", &mut c.include),
            ("exclude", &mut c.exclude),
        ] {
            if let Some(v) = object.get(key) {
                let list = strings(v)?;
                if list.iter().any(|s| !membership_pattern(s, key == "files")) {
                    return None;
                }
                *slot = Some(
                    list.into_iter()
                        .map(|s| norm(dir(p), &s))
                        .collect::<Option<Vec<_>>>()?,
                );
            }
        }
        if let Some(v) = object.get("compilerOptions") {
            let opts = v.as_object()?;
            if let Some(roots) = opts.get("typeRoots") {
                c.type_roots = Some(
                    strings(roots)?
                        .iter()
                        .map(|root| self.snapshot.covered_root(dir(p), root))
                        .collect::<Option<Vec<_>>>()?,
                );
                if c.type_roots.as_ref()?.iter().any(|root| {
                    !self.snapshot.input_references_covered(
                        root,
                        c.type_roots.as_deref(),
                        &mut BTreeSet::new(),
                    )
                }) {
                    return None;
                }
            }
            if let Some(types) = opts.get("types") {
                c.types_origin = dir(p).into();
                if strings(types)?.iter().any(|name| {
                    !self
                        .snapshot
                        .covered_type(dir(p), name, c.type_roots.as_deref())
                }) {
                    return None;
                }
            }
            for (k, v) in opts {
                if k != "paths" {
                    c.options.insert(k.clone(), v.clone());
                }
            }
            if let Some(v) = opts.get("baseUrl") {
                c.base = Some(norm(dir(p), v.as_str()?)?);
            }
            if let Some(v) = opts.get("paths") {
                let mut paths = BTreeMap::new();
                for (k, v) in v.as_object()? {
                    if k.matches('*').count() > 1 {
                        return None;
                    }
                    let targets = strings(v)?;
                    if targets.len() != 1 || targets.iter().any(|t| t.matches('*').count() > 1) {
                        return None;
                    }
                    paths.insert(k.clone(), targets);
                }
                c.paths = Some((dir(p).into(), paths));
            }
        }
        if let Some(types) = c.options.get("types") {
            if strings(types)?.iter().any(|name| {
                !self
                    .snapshot
                    .covered_type(&c.types_origin, name, c.type_roots.as_deref())
            }) {
                return None;
            }
        }
        if c.exclude.is_none()
            && ["outDir", "declarationDir", "rootDir", "rootDirs"]
                .iter()
                .any(|k| c.options.contains_key(*k))
        {
            return None;
        }
        c.config_path = p.into();
        if self.snapshot.references.iter().any(|(file, references)| {
            self.includes(&c, file, &c.config_path) != Some(false)
                && references.iter().any(|(kind, name)| {
                    !self
                        .snapshot
                        .reference_covered(dir(file), kind, name, c.type_roots.as_deref())
                })
        }) {
            return None;
        }
        Some(c)
    }
    fn includes(&self, c: &Config, file: &str, config_path: &str) -> Option<bool> {
        if c.files
            .as_ref()
            .is_some_and(|f| f.iter().any(|x| x == file))
        {
            return Some(true);
        }
        if c.files
            .as_ref()
            .is_some_and(|f| f.iter().any(|x| x.eq_ignore_ascii_case(file)))
        {
            return None;
        }
        if matches!(file.rsplit('.').next(), Some("js" | "jsx" | "mjs" | "cjs"))
            && c.options.get("allowJs").and_then(Value::as_bool) != Some(true)
        {
            return Some(false);
        }
        // Wildcard extension-priority dedupe is deliberately a barrier, not an
        // ancestor fallback. Explicit files entries above remain authoritative.
        let basename = file.rsplit('/').next()?;
        if basename.starts_with('.') || basename.ends_with(".min.js") {
            return None;
        }
        // TypeScript 5.9.3 typescript.js:22530-22544 (allSupportedExtensions),
        // 43966-43997 (higher/lower priority dedupe). Match the longest suffix
        // so .d.ts/.d.cts/.d.mts are not mistaken for implementation extensions.
        let groups: &[&[&str]] = &[
            &[".ts", ".tsx", ".d.ts", ".js", ".jsx"],
            &[".cts", ".d.cts", ".cjs"],
            &[".mts", ".d.mts", ".mjs"],
        ];
        for group in groups {
            let Some((priority, suffix)) = group
                .iter()
                .enumerate()
                .filter(|(_, e)| file.ends_with(*e))
                .max_by_key(|(_, e)| e.len())
            else {
                continue;
            };
            let stem = file.strip_suffix(suffix)?;
            if group[..priority].iter().any(|e| {
                // Declaration files do not suppress JavaScript implementations.
                !(*e == ".d.ts" && [".js", ".jsx"].contains(suffix))
                    && self.snapshot.kind(&format!("{stem}{e}")).is_some()
            }) {
                return None;
            }
            // Retain P1's existing conservative cross-group MJS/CJS barrier.
            if [".mjs", ".cjs"].contains(suffix)
                && groups[0][..2]
                    .iter()
                    .any(|e| self.snapshot.kind(&format!("{stem}{e}")).is_some())
            {
                return None;
            }
        }
        let default;
        let inc = if let Some(i) = &c.include {
            i
        } else if c.files.is_some() {
            return Some(false);
        } else {
            default = vec![norm(dir(config_path), "**/*")?];
            &default
        };
        let mut included = false;
        for p in inc {
            if p.split('/').any(|segment| segment.starts_with('?')) {
                return None;
            }
            let exact = pattern_matches(p, file)?;
            if !exact && pattern_matches(&p.to_ascii_lowercase(), &file.to_ascii_lowercase())? {
                return None;
            }
            included |= exact;
        }
        if !included {
            return Some(false);
        }
        if let Some(ex) = &c.exclude {
            for p in ex {
                let exact = exclude_matches(p, file)?;
                if !exact && exclude_matches(&p.to_ascii_lowercase(), &file.to_ascii_lowercase())? {
                    return None;
                }
                if exact {
                    return Some(false);
                }
            }
        } else if let Some(out) = c.options.get("outDir").and_then(Value::as_str) {
            // outDir is origin-sensitive; P1 declines default-exclude inference here.
            if !out.is_empty() {
                return None;
            }
        }
        Some(true)
    }
    fn select(&mut self, file: &str) -> Option<Rc<Config>> {
        if let Some(c) = self.selected.get(file) {
            return c.clone();
        }
        let mut directory = dir(file);
        let result = loop {
            let p = if directory.is_empty() {
                "tsconfig.json".into()
            } else {
                format!("{directory}/tsconfig.json")
            };
            let jsconfig = if directory.is_empty() {
                "jsconfig.json".into()
            } else {
                format!("{directory}/jsconfig.json")
            };
            // tsserver searches tsconfig before jsconfig in each directory.
            // An admitted tsconfig wins; an excluding one may hand ownership
            // to its sibling jsconfig or stop solution search (Fix A).
            if !self.snapshot.configs.contains_key(&p)
                && self.snapshot.configs.contains_key(&jsconfig)
            {
                break None;
            }
            if self.snapshot.configs.contains_key(&p) {
                let Some(c) = self.config(&p, &mut BTreeSet::new()) else {
                    break None;
                };
                match self.includes(&c, file, &p) {
                    Some(true) => break Some(c),
                    Some(false) => {
                        if self.snapshot.configs.contains_key(&jsconfig)
                            || c.options
                                .get("disableSolutionSearching")
                                .and_then(Value::as_bool)
                                == Some(true)
                        {
                            break None;
                        }
                    }
                    None => break None,
                }
            }
            if directory.is_empty() {
                break None;
            }
            directory = dir(directory);
        };
        self.selected.insert(file.into(), result.clone());
        result
    }
    pub(crate) fn resolve(
        &mut self,
        file: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> Option<String> {
        self.resolution(file, spec, indexed).bound()
    }
    pub(crate) fn resolution(
        &mut self,
        file: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> crate::js_packages::Resolution {
        use crate::js_packages::Resolution;
        if !self.snapshot.complete
            || spec.starts_with('.')
            || spec.contains('\\')
            || spec.starts_with('/')
            || directory_target(spec)
        {
            return Resolution::Unsupported("request syntax or incomplete snapshot".into());
        }
        if file.ends_with(".mts") || file.ends_with(".cts") {
            return Resolution::Unsupported("unindexed writer extension".into());
        }
        let Some(c) = self.select(file) else {
            return Resolution::Unsupported("writer project ownership".into());
        };
        if let Some(module) = self.resolve_in(&c, file, spec, indexed) {
            return Resolution::Bound(module);
        }
        self.package_in(&c, file, spec, indexed)
    }
    fn package_in(
        &self,
        c: &Config,
        file: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> crate::js_packages::Resolution {
        // Matching paths retain Lane-P authority. Only the earlier positive
        // proof may bypass this barrier, including for colon specifiers.
        if c.paths.as_ref().is_some_and(|(_, paths)| {
            paths.keys().any(|k| {
                k == spec
                    || k.split_once('*')
                        .is_some_and(|(pre, post)| spec.starts_with(pre) && spec.ends_with(post))
            })
        }) {
            return crate::js_packages::Resolution::Unsupported("paths authority".into());
        }
        crate::js_packages::resolve(self.snapshot, &c.options, file, spec, indexed)
    }
    fn resolve_in(
        &self,
        c: &Config,
        file: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> Option<String> {
        if !self.snapshot.complete
            || spec.starts_with('.')
            || spec.contains('\\')
            || spec.starts_with('/')
            || directory_target(spec)
        {
            return None;
        }
        let mode = c
            .options
            .get("moduleResolution")
            .and_then(Value::as_str)?
            .to_ascii_lowercase();
        if !["node", "node10"].contains(&mode.as_str())
            && !(spec.contains(':') && ["node16", "nodenext", "bundler"].contains(&mode.as_str()))
            || ["rootDirs", "moduleSuffixes", "noResolve"]
                .iter()
                .any(|k| c.options.contains_key(*k))
        {
            return None;
        }
        let (origin, paths) = c.paths.as_ref()?;
        let (key, capture) = if paths.contains_key(spec) {
            (spec.to_owned(), String::new())
        } else {
            let mut best: Option<(usize, String, String)> = None;
            let mut tied = false;
            for k in paths.keys() {
                let Some((pre, post)) = k.split_once('*') else {
                    continue;
                };
                if !spec.starts_with(pre)
                    || !spec.ends_with(post)
                    || spec.len() < pre.len() + post.len()
                {
                    continue;
                }
                let n = pre.len();
                let capture = spec[n..spec.len() - post.len()].to_owned();
                match &best {
                    Some((m, _, _)) if *m == n => tied = true,
                    Some((m, _, _)) if *m > n => {}
                    _ => {
                        best = Some((n, k.clone(), capture));
                        tied = false;
                    }
                }
            }
            let (_, k, s) = best?;
            if tied || s.is_empty() {
                return None;
            }
            (k, s)
        };
        if self.snapshot.ambient.values().any(|patterns| {
            patterns
                .iter()
                .any(|pattern| ambient_matches(pattern, spec))
        }) {
            return None;
        }
        let raw_target = paths.get(&key)?.first()?;
        if !key.contains('*') && raw_target.contains('*') {
            return None;
        }
        let target = raw_target.replacen('*', &capture, 1);
        if directory_target(&target) {
            return None;
        }
        let p = norm(c.base.as_deref().unwrap_or(origin), &target)?;
        let q = self.prove_path(&p, indexed)?;
        if js_family(&q)
            && (c.options.get("allowJs").and_then(Value::as_bool) != Some(true)
                || !crate::js_paths_first_pass::absent(
                    self.snapshot,
                    file,
                    spec,
                    &p,
                    c.type_roots.as_deref(),
                ))
        {
            return None;
        }
        Some(q)
    }
    fn allow_js(&self, project: &str) -> bool {
        self.configs
            .get(project)
            .and_then(Option::as_ref)
            .is_some_and(|c| c.options.get("allowJs").and_then(Value::as_bool) == Some(true))
    }
    pub(crate) fn project(&mut self, file: &str) -> Option<String> {
        Some(self.select(file)?.config_path.clone())
    }
    /// Refusal-only reachability. Failed authority never widens a literal path
    /// or package name to identities in unrelated modules.
    pub(crate) fn refusal_module(
        &mut self,
        fallback: &str,
        from: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> crate::js_import_qualifiers::RefusalModule {
        let key = (fallback.to_owned(), from.to_owned(), spec.to_owned());
        if let Some(result) = self.refusal_modules.get(&key) {
            return result.clone();
        }
        let result = self.refusal_module_inner(fallback, from, spec, indexed);
        self.refusal_modules.insert(key, result.clone());
        result
    }

    fn refusal_module_inner(
        &mut self,
        fallback: &str,
        from: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> crate::js_import_qualifiers::RefusalModule {
        use crate::js_import_qualifiers::RefusalModule;
        let own = self.project(from);
        if let Some(target) = spec
            .starts_with('.')
            .then(|| {
                own.as_ref()
                    .and_then(|p| self.qualifier_hop(p, from, spec, indexed))
                    .or_else(|| self.qualifier_hop(fallback, from, spec, indexed))
                    .or_else(|| {
                        crate::call_graph::resolve_js_ts_relative_module(spec, from, indexed)
                    })
            })
            .flatten()
        {
            return RefusalModule::Resolved(target);
        }
        // Only the writer's captured native-resolution authority may certify
        // absence. A resolved forward remains in model in refusal_name.
        let unsupported = if !spec.starts_with('.') {
            match self.resolution(from, spec, indexed) {
                crate::js_packages::Resolution::Bound(target) => {
                    return RefusalModule::Resolved(target);
                }
                crate::js_packages::Resolution::ProvenUnresolved => {
                    return if spec.starts_with("node:") {
                        RefusalModule::ExternalBuiltin
                    } else {
                        RefusalModule::ProvenUnresolved
                    };
                }
                crate::js_packages::Resolution::Unsupported(reason) => {
                    if std::env::var_os("PRISM_S2_REFUSAL_DIAGNOSTICS").is_some() {
                        eprintln!("S2 unsupported writer import: writer={from} spec={spec} reason={reason}");
                    }
                    true
                }
            }
        } else {
            false
        };
        if spec.trim() != spec || spec.contains('\\') || spec.starts_with('/') {
            return RefusalModule::Opaque;
        }
        let mut paths = BTreeSet::new();
        let mut packages = BTreeSet::new();
        if spec.starts_with("./") || spec.starts_with("../") {
            if let Some(p) = norm(dir(from), spec) {
                paths.insert(p);
            }
        } else {
            // Paths substitutions remain possible even when a proof guard or
            // occupancy conflict prevents selecting a unique winner.
            for project in own.iter().map(String::as_str).chain([fallback]) {
                if let Some((origin, aliases)) = self
                    .configs
                    .get(project)
                    .and_then(Option::as_ref)
                    .and_then(|c| c.paths.as_ref())
                {
                    let c = self.configs.get(project).and_then(Option::as_ref).unwrap();
                    for (pattern, targets) in aliases {
                        let capture = match pattern.split_once('*') {
                            Some((pre, post)) => {
                                spec.strip_prefix(pre).and_then(|s| s.strip_suffix(post))
                            }
                            None => (pattern == spec).then_some(""),
                        };
                        if let Some(capture) = capture {
                            for target in targets {
                                if let Some(p) = norm(
                                    c.base.as_deref().unwrap_or(origin),
                                    &target.replace('*', capture),
                                ) {
                                    paths.insert(p);
                                }
                            }
                        }
                    }
                }
            }
            for (directory, name) in &self.snapshot.qualifier_packages {
                if name
                    .as_ref()
                    .is_none_or(|name| spec == name || spec.starts_with(&format!("{name}/")))
                {
                    packages.insert(directory.clone());
                }
            }
        }
        let mut candidates: BTreeSet<String> = indexed
            .iter()
            .filter(|file| {
                packages
                    .iter()
                    .any(|p| p.is_empty() || file.starts_with(&format!("{p}/")))
                    || paths.iter().any(|p| refusal_path_matches(p, file))
            })
            .cloned()
            .collect();
        if unsupported && candidates.is_empty() {
            // Unknown native scope cannot be an empty absence certificate.
            // The consumer still projects these files into its caller table.
            candidates.clone_from(indexed);
        }
        RefusalModule::Unavailable(candidates)
    }
    /// S2-only explicit emitted-JS spelling -> captured TS source. Ordinary
    /// relative and alias hops retain P2's occupancy/priority-pass guards.
    pub(crate) fn qualifier_hop(
        &self,
        project: &str,
        from: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> Option<String> {
        if spec.trim() != spec || spec.contains('\\') {
            return None;
        }
        let c = self.configs.get(project)?.as_ref()?;
        if ["rootDirs", "moduleSuffixes", "noResolve"]
            .iter()
            .any(|k| c.options.contains_key(*k))
        {
            return None;
        }
        if spec.starts_with("./") || spec.starts_with("../") {
            let p = norm(dir(from), spec)?;
            let suffixes: &[&str] = if p.ends_with(".js") {
                &[".ts", ".tsx", ".d.ts"]
            } else if p.ends_with(".jsx") {
                &[".tsx", ".ts", ".d.ts"]
            } else if p.ends_with(".mjs") {
                &[".mts", ".d.mts"]
            } else if p.ends_with(".cjs") {
                &[".cts", ".d.cts"]
            } else {
                &[]
            };
            if !suffixes.is_empty() {
                let stem = p.rsplit_once('.')?.0;
                for ext in suffixes {
                    let q = format!("{stem}{ext}");
                    if self.snapshot.kind(&q).is_some() {
                        return (self.snapshot.complete
                            && self.snapshot.unblocked(&q)
                            && self.snapshot.kind(&q) == Some(0)
                            && !ext.starts_with(".d.")
                            && indexed.contains(&q))
                        .then_some(q);
                    }
                }
            }
            self.relative(from, spec, indexed, self.allow_js(project))
        } else {
            self.hop(project, from, spec, indexed)
        }
    }
    /// TypeScript 5.9.3:127381-127394 passes program options for every file;
    /// 128816-128818 only substitutes referenced-project options (P1 refuses
    /// references). Never reselect the barrel's nearest owning config.
    pub(crate) fn hop(
        &self,
        project: &str,
        from: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
    ) -> Option<String> {
        let c = self.configs.get(project)?.as_ref()?;
        if spec.starts_with("./") || spec.starts_with("../") {
            let (_, _, node_esm) =
                crate::js_packages::usage_mode(self.snapshot, &c.options, from).ok()?;
            if node_esm {
                // ESM relative export hops never add extensions or guess index.
                // Keep the writer's project; only the barrel's usage format varies.
                crate::js_packages::relative_esm(self.snapshot, from, spec, indexed)
            } else {
                self.relative(from, spec, indexed, self.allow_js(project))
            }
        } else {
            self.resolve_in(c, from, spec, indexed)
                .or_else(|| self.package_in(c, from, spec, indexed).bound())
        }
    }
    pub(crate) fn relative(
        &self,
        file: &str,
        spec: &str,
        indexed: &BTreeSet<String>,
        allow_js: bool,
    ) -> Option<String> {
        if !self.snapshot.complete
            || !(spec.starts_with("./") || spec.starts_with("../"))
            || directory_target(spec)
        {
            return None;
        }
        let p = norm(dir(file), spec)?;
        if let Some(stem) = p.strip_suffix(".tsx").or_else(|| p.strip_suffix(".ts")) {
            for ext in [".ts", ".tsx", ".d.ts", ".js", ".jsx"] {
                let candidate = format!("{stem}{ext}");
                if candidate != p && self.snapshot.kind(&candidate).is_some() {
                    return None;
                }
            }
        }
        // Relative known JS spellings use suffix replacement, not the paths
        // substitution's literal shortcut (typescript.js:45423-45503 vs 46431).
        // An indexed literal JS source is the first secondary candidate for
        // its spelling; other secondary/package winners remain outside P2.
        let q = if js_family(&p) {
            (self.snapshot.unblocked(&p)
                && self.snapshot.kind(&p) == Some(0)
                && indexed.contains(&p))
            .then_some(p.clone())?
        } else {
            self.prove_path(&p, indexed)?
        };
        if js_family(&q)
            && (!allow_js || !crate::js_paths_first_pass::relative_absent(self.snapshot, &p))
        {
            return None;
        }
        Some(q)
    }
    fn prove_path(&self, p: &str, indexed: &BTreeSet<String>) -> Option<String> {
        if !self.snapshot.unblocked(p) {
            return None;
        }
        if p.ends_with(".ts") || p.ends_with(".tsx") {
            return (self.snapshot.kind(p) == Some(0)
                && indexed.contains(p)
                && !p.ends_with(".d.ts"))
            .then_some(p.to_owned());
        }
        // TS probes unknown suffixes (user.service.ts) on the full substitution.
        // Known non-TS extensions require substitution precedence, outside P1.
        if [".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts", ".json"]
            .iter()
            .any(|e| p.ends_with(e))
        {
            return None;
        }
        if self.snapshot.kind(&format!("{p}/package.json")).is_some() {
            return None;
        }
        // P1 leaves competing file/index/extension candidates at base. A declaration-only
        // winner or an unindexed/opaque source must never be bypassed for a lower candidate.
        if let Some((stem, suffix)) = p.rsplit_once('.') {
            if !suffix.contains('/')
                && self
                    .snapshot
                    .kind(&format!("{stem}.d.{suffix}.ts"))
                    .is_some()
            {
                return None;
            }
        }
        let mut present = Vec::new();
        for ext in [".ts", ".tsx", ".d.ts", ".js", ".jsx"] {
            for q in [format!("{p}{ext}"), format!("{p}/index{ext}")] {
                if self.snapshot.kind(&q).is_some() {
                    present.push(q);
                }
            }
        }
        if self.snapshot.kind(p) == Some(0) {
            return None;
        }
        if present.len() != 1 {
            return None;
        }
        let q = present.pop()?;
        (self.snapshot.unblocked(&q)
            && self.snapshot.kind(&q) == Some(0)
            && indexed.contains(&q)
            && !q.ends_with(".d.ts"))
        .then_some(q)
    }
}

fn refusal_path_matches(path: &str, file: &str) -> bool {
    if file == path {
        return true;
    }
    let stem = [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .find_map(|suffix| path.strip_suffix(suffix))
        .unwrap_or(path);
    [".ts", ".tsx", ".js", ".jsx", ".mts", ".cts", ".mjs", ".cjs"]
        .iter()
        .any(|ext| file == &format!("{stem}{ext}") || file == &format!("{path}/index{ext}"))
}

fn directory_target(p: &str) -> bool {
    p.ends_with('/') || p.ends_with("/.")
}
fn js_family(p: &str) -> bool {
    [".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .any(|e| p.ends_with(e))
}
pub(crate) fn ambient_matches(pattern: &str, spec: &str) -> bool {
    match pattern.split_once('*') {
        Some((pre, post)) => spec.starts_with(pre) && spec.ends_with(post),
        None => pattern == spec,
    }
}
// Validate raw strings before norm can erase unsupported syntax.
fn membership_pattern(p: &str, literal: bool) -> bool {
    let p = p.strip_prefix("./").unwrap_or(p);
    !p.is_empty()
        && p.len() <= 512
        && !p.starts_with('/')
        && !p.contains("..")
        && !p.contains(['?', '[', ']', '{', '}', '\\', ':'])
        && (literal || p.is_ascii())
        && (!literal || !p.contains('*'))
        && p != "**"
        && !p.ends_with("/**")
        && p.split('/')
            .all(|s| !s.is_empty() && s != "." && (!s.contains("**") || s == "**"))
}
