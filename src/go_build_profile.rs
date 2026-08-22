//! Go package/build-profile partitioning for same-package resolution.
//!
//! These profiles are per-file facts: all Go files remain parsed and indexed,
//! and compatibility is consulted only when choosing among same-name
//! same-directory candidates.

use crate::ast::ParsedFile;
use crate::languages::Language;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoBuildProfile {
    pub package_clause: String,
    pub is_test_file: bool,
    pub goos: Option<String>,
    pub goarch: Option<String>,
    pub build_expr: Option<BuildExpr>,
    #[serde(default)]
    pub build_unparsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BuildExpr {
    Tag(String),
    Not(Box<BuildExpr>),
    And(Vec<BuildExpr>),
    Or(Vec<BuildExpr>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoBuildDiagnostics {
    pub unparsed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoBuildVisibility {
    pub visible: bool,
    pub certain: bool,
    pub namespace_decisive: bool,
    pub build_decisive: bool,
    pub diagnostics: GoBuildDiagnostics,
}

pub fn extract_go_file_profiles(
    files: &BTreeMap<String, ParsedFile>,
) -> (BTreeMap<String, GoBuildProfile>, usize) {
    let mut out = BTreeMap::new();
    let mut unparsed = 0usize;
    for (path, parsed) in files {
        if parsed.language != Language::Go {
            continue;
        }
        let (profile, count) = extract_go_file_profile(path, parsed);
        unparsed += count;
        out.insert(path.clone(), profile);
    }
    (out, unparsed)
}

pub fn extract_go_file_profile(path: &str, parsed: &ParsedFile) -> (GoBuildProfile, usize) {
    let package_clause = package_clause(parsed).unwrap_or_default();
    let is_test_file = path.ends_with("_test.go");
    let (goos, goarch) = filename_constraints(path);
    let (build_expr, unparsed) = parse_build_header(&parsed.source);
    (
        GoBuildProfile {
            package_clause,
            is_test_file,
            goos,
            goarch,
            build_expr,
            build_unparsed: unparsed > 0,
        },
        unparsed,
    )
}

pub fn unconstrained_profile() -> GoBuildProfile {
    GoBuildProfile {
        package_clause: String::new(),
        is_test_file: false,
        goos: None,
        goarch: None,
        build_expr: None,
        build_unparsed: false,
    }
}

pub fn profile_allows_exact(profile: Option<&GoBuildProfile>) -> bool {
    profile.is_some_and(|p| !p.build_unparsed && !p.package_clause.is_empty())
}

/// Can a bare Go call in `caller` legally bind to `candidate`?
pub fn go_same_package_visible(caller: &GoBuildProfile, candidate: &GoBuildProfile) -> bool {
    go_same_package_visible_detailed(caller, candidate).visible
}

pub fn go_same_package_visible_detailed(
    caller: &GoBuildProfile,
    candidate: &GoBuildProfile,
) -> GoBuildVisibility {
    if !caller.package_clause.is_empty()
        && !candidate.package_clause.is_empty()
        && caller.package_clause != candidate.package_clause
    {
        return GoBuildVisibility {
            visible: false,
            certain: true,
            namespace_decisive: true,
            build_decisive: false,
            diagnostics: GoBuildDiagnostics::default(),
        };
    }
    if candidate.is_test_file && !caller.is_test_file {
        return GoBuildVisibility {
            visible: false,
            certain: true,
            namespace_decisive: true,
            build_decisive: false,
            diagnostics: GoBuildDiagnostics::default(),
        };
    }
    let sat = build_sat(caller, candidate);
    GoBuildVisibility {
        visible: sat.compatible,
        certain: sat.certain,
        namespace_decisive: false,
        build_decisive: !sat.compatible,
        diagnostics: sat.diagnostics,
    }
}

pub fn visibility_allows_exact(
    profile: Option<&GoBuildProfile>,
    visibility: &GoBuildVisibility,
) -> bool {
    visibility.certain && profile_allows_exact(profile)
}

fn package_clause(parsed: &ParsedFile) -> Option<String> {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() != "package_clause" {
            continue;
        }
        let mut pcursor = child.walk();
        for n in child.children(&mut pcursor) {
            if n.kind() == "package_identifier" {
                let text = parsed.node_text(&n).trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

fn filename_constraints(path: &str) -> (Option<String>, Option<String>) {
    let file = path.rsplit('/').next().unwrap_or(path);
    let Some(mut stem) = file.strip_suffix(".go") else {
        return (None, None);
    };
    if let Some(stripped) = stem.strip_suffix("_test") {
        stem = stripped;
    }
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() >= 3 && !parts[..parts.len() - 2].join("_").is_empty() {
        let a = parts[parts.len() - 2];
        let b = parts[parts.len() - 1];
        if is_goos(a) && is_goarch(b) {
            return (Some(a.to_string()), Some(b.to_string()));
        }
    }
    if parts.len() >= 2 && !parts[..parts.len() - 1].join("_").is_empty() {
        let last = parts[parts.len() - 1];
        if is_goos(last) {
            return (Some(last.to_string()), None);
        }
        if is_goarch(last) {
            return (None, Some(last.to_string()));
        }
    }
    (None, None)
}

fn parse_build_header(source: &str) -> (Option<BuildExpr>, usize) {
    let mut go_build = Vec::new();
    let mut in_block = false;
    let mut ended = false;
    let mut header_end = 0usize;
    let mut offset = 0usize;

    'lines: for raw in source.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let mut trimmed = line.trim();
        let after_line = offset + raw.len();

        if trimmed.is_empty() && !ended {
            header_end = after_line;
            offset = after_line;
            continue 'lines;
        }
        if !trimmed.starts_with("//") {
            ended = true;
        }

        if !in_block {
            if let Some(expr) = split_go_build(trimmed) {
                go_build.push(expr);
            }
        }

        loop {
            if in_block {
                let Some(end) = trimmed.find("*/") else {
                    offset = after_line;
                    continue 'lines;
                };
                in_block = false;
                trimmed = trimmed[end + 2..].trim();
                continue;
            }
            if trimmed.is_empty() {
                offset = after_line;
                continue 'lines;
            }
            if trimmed.starts_with("//") {
                offset = after_line;
                continue 'lines;
            }
            if let Some(rest) = trimmed.strip_prefix("/*") {
                in_block = true;
                trimmed = rest.trim();
                continue;
            }
            break 'lines;
        }
    }
    if go_build.len() > 1 {
        return (None, 1);
    }
    if let Some(expr) = go_build.first() {
        return match Parser::new(expr).parse() {
            Some(e) => (Some(e), 0),
            None => (None, 1),
        };
    }

    let plus_build: Vec<String> = source[..header_end]
        .lines()
        .filter_map(|line| split_plus_build(line.trim()))
        .collect();
    if plus_build.is_empty() {
        return (None, 0);
    }
    let mut lines = Vec::new();
    for line in plus_build {
        let mut ors = Vec::new();
        for alt in line.split_whitespace() {
            let mut ands = Vec::new();
            for part in alt.split(',') {
                if part.is_empty() {
                    return (None, 1);
                }
                let (neg, tag) = part.strip_prefix('!').map_or((false, part), |s| (true, s));
                if !valid_tag(tag) {
                    return (None, 1);
                }
                let atom = BuildExpr::Tag(tag.to_string());
                ands.push(if neg {
                    BuildExpr::Not(Box::new(atom))
                } else {
                    atom
                });
            }
            ors.push(fold_and(ands));
        }
        if ors.is_empty() {
            return (None, 1);
        }
        lines.push(fold_or(ors));
    }
    (Some(fold_and(lines)), 0)
}

fn split_go_build(line: &str) -> Option<String> {
    let rest = line.strip_prefix("//go:build")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim().to_string())
}

fn split_plus_build(line: &str) -> Option<String> {
    let rest = line.strip_prefix("//")?.trim_start();
    let rest = rest.strip_prefix("+build")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim().to_string())
}

struct SatResult {
    compatible: bool,
    certain: bool,
    diagnostics: GoBuildDiagnostics,
}

fn build_sat(a: &GoBuildProfile, b: &GoBuildProfile) -> SatResult {
    let mut tags = BTreeSet::new();
    collect_profile_tags(a, &mut tags);
    collect_profile_tags(b, &mut tags);
    let free: Vec<String> = tags
        .iter()
        .filter(|t| !is_goos(t) && !is_goarch(t) && t.as_str() != "unix")
        .cloned()
        .collect();
    if free.len() > 8 {
        return SatResult {
            compatible: true,
            certain: false,
            diagnostics: GoBuildDiagnostics { unparsed: 1 },
        };
    }
    let free_count = 1usize << free.len();
    for goos in GOOS {
        for goarch in GOARCH {
            for mask in 0..free_count {
                if profile_satisfied(a, goos, goarch, &free, mask)
                    && profile_satisfied(b, goos, goarch, &free, mask)
                {
                    return SatResult {
                        compatible: true,
                        certain: true,
                        diagnostics: GoBuildDiagnostics::default(),
                    };
                }
            }
        }
    }
    SatResult {
        compatible: false,
        certain: true,
        diagnostics: GoBuildDiagnostics::default(),
    }
}

fn profile_satisfied(
    p: &GoBuildProfile,
    goos: &str,
    goarch: &str,
    free: &[String],
    mask: usize,
) -> bool {
    if p.goos
        .as_ref()
        .is_some_and(|tag| !match_tag(tag, goos, goarch))
    {
        return false;
    }
    if p.goarch
        .as_ref()
        .is_some_and(|tag| !match_tag(tag, goos, goarch))
    {
        return false;
    }
    p.build_expr
        .as_ref()
        .map(|e| eval_expr(e, goos, goarch, free, mask))
        .unwrap_or(true)
}

fn eval_expr(expr: &BuildExpr, goos: &str, goarch: &str, free: &[String], mask: usize) -> bool {
    match expr {
        BuildExpr::Tag(t) => {
            match_tag(t, goos, goarch)
                || free
                    .iter()
                    .position(|f| f == t)
                    .is_some_and(|i| (mask & (1usize << i)) != 0)
        }
        BuildExpr::Not(e) => !eval_expr(e, goos, goarch, free, mask),
        BuildExpr::And(es) => es.iter().all(|e| eval_expr(e, goos, goarch, free, mask)),
        BuildExpr::Or(es) => es.iter().any(|e| eval_expr(e, goos, goarch, free, mask)),
    }
}

fn match_tag(tag: &str, goos: &str, goarch: &str) -> bool {
    tag == goos
        || tag == goarch
        || (tag == "linux" && goos == "android")
        || (tag == "solaris" && goos == "illumos")
        || (tag == "darwin" && goos == "ios")
        || (tag == "unix" && is_unix_goos(goos))
}

fn collect_profile_tags(p: &GoBuildProfile, out: &mut BTreeSet<String>) {
    if let Some(goos) = &p.goos {
        out.insert(goos.clone());
    }
    if let Some(goarch) = &p.goarch {
        out.insert(goarch.clone());
    }
    if let Some(expr) = &p.build_expr {
        collect_expr_tags(expr, out);
    }
}

fn collect_expr_tags(expr: &BuildExpr, out: &mut BTreeSet<String>) {
    match expr {
        BuildExpr::Tag(t) => {
            out.insert(t.clone());
        }
        BuildExpr::Not(e) => collect_expr_tags(e, out),
        BuildExpr::And(es) | BuildExpr::Or(es) => {
            for e in es {
                collect_expr_tags(e, out);
            }
        }
    }
}

fn fold_and(mut exprs: Vec<BuildExpr>) -> BuildExpr {
    if exprs.len() == 1 {
        exprs.remove(0)
    } else {
        BuildExpr::And(exprs)
    }
}

fn fold_or(mut exprs: Vec<BuildExpr>) -> BuildExpr {
    if exprs.len() == 1 {
        exprs.remove(0)
    } else {
        BuildExpr::Or(exprs)
    }
}

struct Parser {
    tokens: Vec<String>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        let spaced = input
            .replace("&&", " && ")
            .replace("||", " || ")
            .replace('!', " ! ")
            .replace('(', " ( ")
            .replace(')', " ) ");
        Parser {
            tokens: spaced.split_whitespace().map(|s| s.to_string()).collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Option<BuildExpr> {
        let expr = self.parse_or()?;
        (self.pos == self.tokens.len()).then_some(expr)
    }

    fn parse_or(&mut self) -> Option<BuildExpr> {
        let mut exprs = vec![self.parse_and()?];
        while self.peek() == Some("||") {
            self.pos += 1;
            exprs.push(self.parse_and()?);
        }
        Some(fold_or(exprs))
    }

    fn parse_and(&mut self) -> Option<BuildExpr> {
        let mut exprs = vec![self.parse_unary()?];
        while self.peek() == Some("&&") {
            self.pos += 1;
            exprs.push(self.parse_unary()?);
        }
        Some(fold_and(exprs))
    }

    fn parse_unary(&mut self) -> Option<BuildExpr> {
        if self.peek() == Some("!") {
            self.pos += 1;
            return Some(BuildExpr::Not(Box::new(self.parse_unary()?)));
        }
        if self.peek() == Some("(") {
            self.pos += 1;
            let expr = self.parse_or()?;
            if self.peek() != Some(")") {
                return None;
            }
            self.pos += 1;
            return Some(expr);
        }
        let tok = self.peek()?.to_string();
        if !valid_tag(&tok) {
            return None;
        }
        self.pos += 1;
        Some(BuildExpr::Tag(tok))
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|s| s.as_str())
    }
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|c| c == '_' || c == '.' || c.is_ascii_alphanumeric())
}

fn is_goos(s: &str) -> bool {
    GOOS.contains(&s)
}

fn is_goarch(s: &str) -> bool {
    GOARCH.contains(&s)
}

fn is_unix_goos(s: &str) -> bool {
    UNIX_GOOS.contains(&s)
}

// Mirrors Go 1.26.2 $GOROOT/src/internal/syslist/syslist.go KnownOS/KnownArch.
#[rustfmt::skip]
const GOOS: &[&str] = &["aix", "android", "darwin", "dragonfly", "freebsd", "hurd", "illumos", "ios", "js", "linux", "nacl", "netbsd", "openbsd", "plan9", "solaris", "wasip1", "windows", "zos"];
#[rustfmt::skip]
const UNIX_GOOS: &[&str] = &["aix", "android", "darwin", "dragonfly", "freebsd", "hurd", "illumos", "ios", "linux", "netbsd", "openbsd", "solaris"];
#[rustfmt::skip]
const GOARCH: &[&str] = &["386", "amd64", "amd64p32", "arm", "armbe", "arm64", "arm64be", "loong64", "mips", "mipsle", "mips64", "mips64le", "mips64p32", "mips64p32le", "ppc", "ppc64", "ppc64le", "riscv", "riscv64", "s390", "s390x", "sparc", "sparc64", "wasm"];
