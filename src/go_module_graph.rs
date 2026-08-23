use crate::go_mod::{tokenize, valid_module_path, PathKind, Token};
use crate::manifest_snapshot::{ManifestSnapshot, ManifestSnapshotEntry};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

mod paths;
mod replacements;
use paths::{normalize_repo_dir, path_to_repo_string};
mod identity;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub(crate) enum GoImportPathReason {
    InactiveModule,
    ReplaceUnproven,
    WorkspaceInvalid,
    NoGoMod,
    Malformed,
    Symlink,
}

impl GoImportPathReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InactiveModule => "inactive_module",
            Self::ReplaceUnproven => "replace_unproven",
            Self::WorkspaceInvalid => "workspace_invalid",
            Self::NoGoMod => "no_go_mod",
            Self::Malformed => "malformed",
            Self::Symlink => "symlink",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct GoModuleGraphTelemetry {
    pub(crate) modules: usize,
    pub(crate) active: usize,
    pub(crate) replaces_parsed: usize,
    pub(crate) replaces_applied: usize,
    pub(crate) workspace_invalid: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GoImportPathResolution {
    pub(crate) paths: BTreeMap<String, String>,
    pub(crate) proven_files: usize,
    pub(crate) unproven_files: usize,
    pub(crate) reasons: BTreeMap<String, usize>,
    pub(crate) graph: GoModuleGraphTelemetry,
}

pub(crate) struct GoModuleGraph {
    telemetry: GoModuleGraphTelemetry,
    active: BTreeSet<String>,
    boundaries: BTreeMap<String, ModuleBoundary>,
    providers: BTreeMap<String, String>,
    replace_unproven: BTreeSet<String>,
    replace_unproven_dirs: BTreeSet<String>,
    memo: BTreeMap<String, Result<String, GoImportPathReason>>,
    manifest_parse_counts: BTreeMap<String, usize>,
}

impl GoModuleGraph {
    pub(crate) fn new(repo_root: &Path, snapshot: &ManifestSnapshot) -> Self {
        let mut boundaries = BTreeMap::new();
        let mut manifest_parse_counts = BTreeMap::new();
        let active_selection =
            load_active_module_selection(repo_root, snapshot, &mut manifest_parse_counts);
        let main_module_dirs = active_selection
            .as_ref()
            .ok()
            .map(|selection| &selection.dirs);
        for (path, entry) in snapshot.entries() {
            let Some(dir) = go_mod_dir(path) else {
                continue;
            };
            if manifest_is_excluded(path) {
                continue;
            }
            let boundary = match entry {
                ManifestSnapshotEntry::Regular { bytes, .. } => {
                    *manifest_parse_counts.entry(path.clone()).or_default() += 1;
                    let path_kind = if main_module_dirs.is_some_and(|dirs| dirs.contains(&dir)) {
                        PathKind::MainModule
                    } else {
                        PathKind::Dependency
                    };
                    std::str::from_utf8(bytes)
                        .ok()
                        .and_then(|source| parse_go_mod(source, path_kind))
                        .map(ModuleBoundary::Valid)
                        .unwrap_or(ModuleBoundary::Malformed)
                }
                ManifestSnapshotEntry::SymlinkRefused => ModuleBoundary::Symlink,
            };
            boundaries.insert(dir, boundary);
        }

        let mut graph = Self {
            telemetry: GoModuleGraphTelemetry {
                modules: boundaries
                    .values()
                    .filter(|boundary| matches!(boundary, ModuleBoundary::Valid(_)))
                    .count(),
                ..GoModuleGraphTelemetry::default()
            },
            active: BTreeSet::new(),
            boundaries,
            providers: BTreeMap::new(),
            replace_unproven: BTreeSet::new(),
            replace_unproven_dirs: BTreeSet::new(),
            memo: BTreeMap::new(),
            manifest_parse_counts,
        };
        debug_assert!(
            graph
                .manifest_parse_counts
                .values()
                .all(|count| *count == 1),
            "each manifest must be parsed at most once from the loader snapshot"
        );
        let work = graph.select_active_modules(active_selection);
        if !graph.telemetry.workspace_invalid {
            replacements::apply(&mut graph, repo_root, work.as_ref());
        }
        graph
    }

    fn select_active_modules(
        &mut self,
        selection: Result<ActiveModuleSelection, ()>,
    ) -> Option<ParsedGoWork> {
        let Ok(ActiveModuleSelection { dirs, work }) = selection else {
            self.invalidate_workspace();
            return None;
        };
        for dir in &dirs {
            if !matches!(
                self.boundaries.get(dir),
                Some(ModuleBoundary::Valid(module))
                    if module.path_kind == PathKind::MainModule
            ) {
                self.invalidate_workspace();
                return None;
            }
        }
        self.active = dirs;

        let mut path_owners = BTreeMap::new();
        for dir in &self.active {
            let ModuleBoundary::Valid(module) = &self.boundaries[dir] else {
                unreachable!("active modules were validated above")
            };
            if path_owners
                .insert(module.path.clone(), dir.clone())
                .is_some()
            {
                self.invalidate_workspace();
                return None;
            }
            self.providers.insert(dir.clone(), module.path.clone());
        }
        self.telemetry.active = self.active.len();
        work
    }

    fn invalidate_workspace(&mut self) {
        self.telemetry.workspace_invalid = true;
        self.telemetry.active = 0;
        self.active.clear();
        self.providers.clear();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn telemetry(&self) -> &GoModuleGraphTelemetry {
        &self.telemetry
    }

    #[cfg(test)]
    fn active_dirs(&self) -> &BTreeSet<String> {
        &self.active
    }

    #[cfg(test)]
    fn memo_len(&self) -> usize {
        self.memo.len()
    }

    #[cfg(test)]
    fn provider_path(&self, dir: &str) -> Option<&str> {
        self.providers.get(dir).map(String::as_str)
    }

    #[cfg(test)]
    fn module_path_kind(&self, dir: &str) -> Option<PathKind> {
        let ModuleBoundary::Valid(module) = self.boundaries.get(dir)? else {
            return None;
        };
        Some(module.path_kind)
    }

    #[cfg(test)]
    fn replacement_is_unproven(&self, path: &str) -> bool {
        self.replace_unproven.contains(path)
    }

    #[cfg(test)]
    fn replacement_dir_is_unproven(&self, dir: &str) -> bool {
        self.replace_unproven_dirs.contains(dir)
    }

    #[cfg(test)]
    fn manifest_parse_counts(&self) -> &BTreeMap<String, usize> {
        &self.manifest_parse_counts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModuleBoundary {
    Valid(ParsedGoMod),
    Malformed,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGoMod {
    path: String,
    path_kind: PathKind,
    requires: BTreeSet<String>,
    replaces: Vec<Replacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGoWork {
    uses: Vec<String>,
    replaces: Vec<Replacement>,
}

struct ActiveModuleSelection {
    dirs: BTreeSet<String>,
    work: Option<ParsedGoWork>,
}

fn load_active_module_selection(
    repo_root: &Path,
    snapshot: &ManifestSnapshot,
    manifest_parse_counts: &mut BTreeMap<String, usize>,
) -> Result<ActiveModuleSelection, ()> {
    match snapshot.get("go.work") {
        Some(ManifestSnapshotEntry::Regular { bytes, .. }) => {
            *manifest_parse_counts
                .entry("go.work".to_string())
                .or_default() += 1;
            let work = std::str::from_utf8(bytes)
                .ok()
                .and_then(parse_go_work)
                .ok_or(())?;
            let mut dirs = BTreeSet::new();
            for use_path in &work.uses {
                let dir = normalize_repo_dir(repo_root, "", use_path).ok_or(())?;
                if !dirs.insert(dir) {
                    return Err(());
                }
            }
            Ok(ActiveModuleSelection {
                dirs,
                work: Some(work),
            })
        }
        Some(ManifestSnapshotEntry::SymlinkRefused) => Err(()),
        None => {
            let dirs = if snapshot.get("go.mod").is_some() {
                BTreeSet::from([String::new()])
            } else {
                BTreeSet::new()
            };
            Ok(ActiveModuleSelection { dirs, work: None })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Replacement {
    lhs_path: String,
    lhs_version: Option<String>,
    rhs: ReplacementRhs,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ReplacementRhs {
    Local(String),
    Module { path: String, version: String },
}

#[derive(Debug)]
struct Directive {
    name: String,
    args: Vec<String>,
}

fn parse_go_mod(source: &str, path_kind: PathKind) -> Option<ParsedGoMod> {
    let directives = parse_directives(source)?;
    let mut module_path = None;
    let mut singleton_directives = BTreeSet::new();
    let mut requires = BTreeSet::new();
    let mut replace_keys = BTreeSet::new();
    let mut replaces = Vec::new();

    for directive in directives {
        match directive.name.as_str() {
            "module" => {
                validate_args(&directive.args, 1)?;
                if module_path.is_some() || !valid_module_path(&directive.args[0], path_kind) {
                    return None;
                }
                module_path = Some(directive.args[0].clone());
            }
            "go" => validate_singleton(&directive, &mut singleton_directives, valid_go_version)?,
            "toolchain" => validate_singleton(&directive, &mut singleton_directives, |value| {
                value.starts_with("go") && valid_go_version(&value[2..])
            })?,
            "godebug" => validate_args(&directive.args, 1)?,
            "tool" => {
                validate_args(&directive.args, 1)?;
                if !valid_module_path(&directive.args[0], PathKind::Dependency) {
                    return None;
                }
            }
            "require" => {
                validate_args(&directive.args, 2)?;
                if !valid_module_path(&directive.args[0], PathKind::Dependency)
                    || !valid_module_version(&directive.args[1])
                {
                    return None;
                }
                if !requires.insert(directive.args[0].clone()) {
                    return None;
                }
            }
            "exclude" => {
                validate_args(&directive.args, 2)?;
                if !valid_module_path(&directive.args[0], PathKind::Dependency)
                    || !valid_module_version(&directive.args[1])
                {
                    return None;
                }
            }
            "replace" => {
                let replacement = parse_replace(&directive.args)?;
                if !replace_keys.insert((
                    replacement.lhs_path.clone(),
                    replacement.lhs_version.clone(),
                )) {
                    return None;
                }
                replaces.push(replacement);
            }
            "retract" => {
                if directive.args.is_empty() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(ParsedGoMod {
        path: module_path?,
        path_kind,
        requires,
        replaces,
    })
}

fn parse_go_work(source: &str) -> Option<ParsedGoWork> {
    let directives = parse_directives(source)?;
    let mut singleton_directives = BTreeSet::new();
    let mut replace_keys = BTreeSet::new();
    let mut uses = Vec::new();
    let mut replaces = Vec::new();
    for directive in directives {
        match directive.name.as_str() {
            "go" => validate_singleton(&directive, &mut singleton_directives, valid_go_version)?,
            "toolchain" => validate_singleton(&directive, &mut singleton_directives, |value| {
                value.starts_with("go") && valid_go_version(&value[2..])
            })?,
            "godebug" => validate_args(&directive.args, 1)?,
            "use" => {
                validate_args(&directive.args, 1)?;
                uses.push(directive.args[0].clone());
            }
            "replace" => {
                let replacement = parse_replace(&directive.args)?;
                if !replace_keys.insert((
                    replacement.lhs_path.clone(),
                    replacement.lhs_version.clone(),
                )) {
                    return None;
                }
                replaces.push(replacement);
            }
            _ => return None,
        }
    }
    singleton_directives
        .contains("go")
        .then_some(ParsedGoWork { uses, replaces })
}

fn parse_directives(source: &str) -> Option<Vec<Directive>> {
    let tokens = tokenize(source)?;
    let mut directives = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        skip_newlines(&tokens, &mut index);
        if index == tokens.len() {
            break;
        }
        let Token::Word(name) = tokens.get(index)? else {
            return None;
        };
        let name = name.clone();
        index += 1;
        if matches!(tokens.get(index), Some(Token::LeftParen)) {
            index += 1;
            if !matches!(tokens.get(index), Some(Token::Newline)) {
                return None;
            }
            index += 1;
            loop {
                skip_newlines(&tokens, &mut index);
                if matches!(tokens.get(index), Some(Token::RightParen)) {
                    index += 1;
                    if matches!(tokens.get(index), Some(Token::Newline)) {
                        index += 1;
                    } else if index != tokens.len() {
                        return None;
                    }
                    break;
                }
                let args = take_line_words(&tokens, &mut index)?;
                if args.is_empty() {
                    return None;
                }
                directives.push(Directive {
                    name: name.clone(),
                    args,
                });
            }
        } else {
            let args = take_line_words(&tokens, &mut index)?;
            directives.push(Directive { name, args });
        }
    }
    Some(directives)
}

fn skip_newlines(tokens: &[Token], index: &mut usize) {
    while matches!(tokens.get(*index), Some(Token::Newline)) {
        *index += 1;
    }
}

fn take_line_words(tokens: &[Token], index: &mut usize) -> Option<Vec<String>> {
    let mut words = Vec::new();
    while let Some(token) = tokens.get(*index) {
        match token {
            Token::Word(word) => words.push(word.clone()),
            Token::Newline => {
                *index += 1;
                return Some(words);
            }
            Token::LeftParen | Token::RightParen => return None,
        }
        *index += 1;
    }
    Some(words)
}

fn validate_singleton(
    directive: &Directive,
    seen: &mut BTreeSet<String>,
    validate: impl FnOnce(&str) -> bool,
) -> Option<()> {
    validate_args(&directive.args, 1)?;
    if !seen.insert(directive.name.clone()) || !validate(&directive.args[0]) {
        return None;
    }
    Some(())
}

fn validate_args(args: &[String], count: usize) -> Option<()> {
    (args.len() == count).then_some(())
}

fn valid_go_version(version: &str) -> bool {
    let mut pieces = version.split('.');
    matches!(pieces.next(), Some(part) if !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        && matches!(pieces.next(), Some(part) if !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        && pieces.all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_module_version(version: &str) -> bool {
    version.starts_with('v') && version.len() > 1 && version.is_ascii()
}

fn parse_replace(args: &[String]) -> Option<Replacement> {
    let arrow = args.iter().position(|arg| arg == "=>")?;
    if args.iter().skip(arrow + 1).any(|arg| arg == "=>") {
        return None;
    }
    let (lhs, rhs_with_arrow) = args.split_at(arrow);
    let rhs = &rhs_with_arrow[1..];
    if !(lhs.len() == 1 || lhs.len() == 2) || !(rhs.len() == 1 || rhs.len() == 2) {
        return None;
    }
    if !valid_module_path(&lhs[0], PathKind::Dependency) {
        return None;
    }
    let lhs_version = lhs.get(1).cloned();
    if lhs_version
        .as_deref()
        .is_some_and(|version| !valid_module_version(version))
    {
        return None;
    }
    let rhs = if rhs.len() == 1 && is_local_path(&rhs[0]) {
        ReplacementRhs::Local(rhs[0].clone())
    } else if rhs.len() == 2
        && valid_module_path(&rhs[0], PathKind::Dependency)
        && valid_module_version(&rhs[1])
    {
        ReplacementRhs::Module {
            path: rhs[0].clone(),
            version: rhs[1].clone(),
        }
    } else {
        return None;
    };
    Some(Replacement {
        lhs_path: lhs[0].clone(),
        lhs_version,
        rhs,
    })
}

fn is_local_path(path: &str) -> bool {
    Path::new(path).is_absolute()
        || path == "."
        || path == ".."
        || path.starts_with("./")
        || path.starts_with("../")
}

fn go_mod_dir(path: &str) -> Option<String> {
    if path == "go.mod" {
        Some(String::new())
    } else {
        path.strip_suffix("/go.mod").map(str::to_string)
    }
}

fn manifest_is_excluded(path: &str) -> bool {
    Path::new(path).components().any(
        |component| matches!(component, Component::Normal(name) if name == "vendor" || name == "testdata"),
    )
}

#[cfg(test)]
#[path = "go_module_graph_tests.rs"]
mod tests;
