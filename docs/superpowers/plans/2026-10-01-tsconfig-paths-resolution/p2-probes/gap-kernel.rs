// Included in an isolated copy of js_paths.rs, never in the production crate.
// The explainer must reproduce the unmodified kernel's Option on every request.
impl Resolver<'_> {
    pub(crate) fn explain(
        &mut self, file: &str, spec: &str, indexed: &BTreeSet<String>,
        allow: bool, entry: bool,
    ) -> serde_json::Value {
        let actual = if entry { self.resolve(file, spec, indexed) }
            else { self.relative(file, spec, indexed, allow) };
        let explained = self.explain_inner(file, spec, indexed, allow, entry);
        assert_eq!(actual, explained.as_ref().ok().cloned(), "kernel replay disagreement");
        serde_json::json!({"target":actual,"gate":explained.err().unwrap_or("PASS")})
    }
    fn explain_inner(
        &mut self, file: &str, spec: &str, indexed: &BTreeSet<String>,
        allow: bool, entry: bool,
    ) -> Result<String, &'static str> {
        if !self.snapshot.complete { return Err("SNAPSHOT_INCOMPLETE"); }
        if directory_target(spec) { return Err("DIRECTORY_LITERAL"); }
        if entry {
            if spec.starts_with('.') || spec.contains(['\\', ':']) || spec.starts_with('/') {
                return Err("ENTRY_SPECIFIER_SYNTAX");
            }
            let c = self.select(file).ok_or("ENTRY_CONFIG_SELECTION")?;
            let mode = c.options.get("moduleResolution").and_then(Value::as_str)
                .ok_or("ENTRY_MODE_OR_OPTIONS")?.to_ascii_lowercase();
            if !["node", "node10"].contains(&mode.as_str()) ||
                ["rootDirs", "moduleSuffixes", "noResolve"].iter().any(|k|c.options.contains_key(*k)) {
                return Err("ENTRY_MODE_OR_OPTIONS");
            }
            let (origin, paths) = c.paths.as_ref().ok_or("ENTRY_NO_PATHS")?;
            let (key, capture) = if paths.contains_key(spec) { (spec.to_owned(), String::new()) }
            else {
                let mut best: Option<(usize, String, String)> = None;
                let mut tied = false;
                for k in paths.keys() {
                    let Some((pre, post)) = k.split_once('*') else {continue};
                    if !spec.starts_with(pre) || !spec.ends_with(post) || spec.len()<pre.len()+post.len() {continue}
                    let n=pre.len(); let capture=spec[n..spec.len()-post.len()].to_owned();
                    match &best {
                        Some((m,_,_)) if *m==n => tied=true,
                        Some((m,_,_)) if *m>n => {},
                        _ => {best=Some((n,k.clone(),capture));tied=false;}
                    }
                }
                let (_,k,s)=best.ok_or("ENTRY_NO_MATCHING_PATH")?;
                if tied {return Err("ENTRY_TIED_PATTERN")}
                if s.is_empty() {return Err("ENTRY_EMPTY_CAPTURE")}
                (k,s)
            };
            if self.snapshot.ambient.values().any(|ps|ps.iter().any(|p|ambient_matches(p,spec))) {
                return Err("ENTRY_AMBIENT_COMPETITION");
            }
            let raw=paths.get(&key).and_then(|v|v.first()).ok_or("ENTRY_PATHS_SHAPE")?;
            if !key.contains('*') && raw.contains('*') {return Err("ENTRY_UNMATCHED_TARGET_STAR")}
            let target=raw.replacen('*',&capture,1);
            if directory_target(&target) {return Err("DIRECTORY_LITERAL")}
            let p=norm(c.base.as_deref().unwrap_or(origin),&target).ok_or("PATH_OUTSIDE_OR_SYNTAX")?;
            let q=self.explain_path(&p,indexed)?;
            if js_family(&q) {
                if c.options.get("allowJs").and_then(Value::as_bool)!=Some(true) {return Err("ALLOW_JS_OFF")}
                if !crate::js_paths_first_pass::absent(self.snapshot,file,spec,&p,c.type_roots.as_deref()) {
                    return Err("ENTRY_JS_FIRST_PASS");
                }
            }
            Ok(q)
        } else {
            if !(spec.starts_with("./") || spec.starts_with("../")) {return Err("NONRELATIVE_HOP")}
            let p=norm(dir(file),spec).ok_or("PATH_OUTSIDE_OR_SYNTAX")?;
            if let Some(stem)=p.strip_suffix(".tsx").or_else(||p.strip_suffix(".ts")) {
                for ext in [".ts",".tsx",".d.ts",".js",".jsx"] {
                    let candidate=format!("{stem}{ext}");
                    if candidate!=p && self.snapshot.kind(&candidate).is_some() {return Err("EXPLICIT_TS_COMPETITION")}
                }
            }
            let q=if js_family(&p) {
                if !self.snapshot.unblocked(&p) {return Err("PATH_OPAQUE")}
                if self.snapshot.kind(&p)!=Some(0) {return Err("EXPLICIT_JS_LITERAL_UNAVAILABLE")}
                if !indexed.contains(&p) {return Err("SOURCE_NOT_INDEXED")}
                p.clone()
            } else { self.explain_path(&p,indexed)? };
            if js_family(&q) {
                if !allow {return Err("ALLOW_JS_OFF")}
                if !crate::js_paths_first_pass::relative_absent(self.snapshot,&p) {return Err("HOP_JS_FIRST_PASS")}
            }
            Ok(q)
        }
    }
    fn explain_path(&self,p:&str,indexed:&BTreeSet<String>) -> Result<String,&'static str> {
        if !self.snapshot.unblocked(p) {return Err("PATH_OPAQUE")}
        if p.ends_with(".ts") || p.ends_with(".tsx") {
            if self.snapshot.kind(p)!=Some(0) {return Err("EXPLICIT_TS_LITERAL_UNAVAILABLE")}
            if !indexed.contains(p) {return Err("SOURCE_NOT_INDEXED")}
            if p.ends_with(".d.ts") {return Err("DECLARATION_WINNER")}
            return Ok(p.into());
        }
        if [".js",".jsx",".mjs",".cjs",".mts",".cts",".json"].iter().any(|e|p.ends_with(e)) {
            return Err("EXPLICIT_EXTENSION_PATH");
        }
        if self.snapshot.kind(&format!("{p}/package.json")).is_some() {return Err("PACKAGE_DIRECTORY_REDIRECT")}
        if let Some((stem,suffix))=p.rsplit_once('.') {
            if !suffix.contains('/') && self.snapshot.kind(&format!("{stem}.d.{suffix}.ts")).is_some() {
                return Err("UNKNOWN_SUFFIX_DECLARATION_COMPETITION");
            }
        }
        let mut present=Vec::new();
        for ext in [".ts",".tsx",".d.ts",".js",".jsx"] {
            for q in [format!("{p}{ext}"),format!("{p}/index{ext}")] {
                if self.snapshot.kind(&q).is_some() {present.push(q)}
            }
        }
        if self.snapshot.kind(p)==Some(0) {return Err("EXTENSIONLESS_LITERAL_FILE")}
        if present.is_empty() {return Err("CANDIDATE_ABSENT")}
        if present.len()!=1 {return Err("CANDIDATE_COMPETITION")}
        let q=present.pop().unwrap();
        if !self.snapshot.unblocked(&q) || self.snapshot.kind(&q)!=Some(0) {return Err("PATH_OPAQUE")}
        if !indexed.contains(&q) {return Err("SOURCE_NOT_INDEXED")}
        if q.ends_with(".d.ts") {return Err("DECLARATION_WINNER")}
        Ok(q)
    }
}
