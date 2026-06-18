use crate::name_resolution::engine::{resolve, resolve_path};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{RustPolicy, NS_TYPE};
use crate::name_resolution::types::{
    Anchor, AnchorKind, Candidate, CfgCtx, PolicyQueryCtx, RawPath, ResStatus, ResolveQuery,
    ScopeId, SourceLoc, Target,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TypeKey {
    InRepo(ScopeId),
    External(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReceiverTypeKey {
    InRepo(ScopeId),
    External(String),
    Bare(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReceiverOutcome {
    pub key: ReceiverTypeKey,
    pub bare: String,
    pub recovery: crate::resolution::ReceiverRecovery,
}

pub fn resolve_type_path_to_type_scope(
    graph: &ScopeGraph,
    from: ScopeId,
    type_syntax: &str,
) -> Option<TypeKey> {
    let peeled = crate::resolution::peel_type(type_syntax);
    if peeled.is_empty() {
        return None;
    }

    let target = if peeled.contains("::") {
        resolve_via_path(graph, from, &peeled)
    } else {
        resolve_via_lexical(graph, from, &peeled)
    };
    if let Some(t) = target.and_then(|t| type_scope_of_target(graph, &t)) {
        return Some(TypeKey::InRepo(t));
    }

    if is_confidently_external(&peeled) {
        return Some(TypeKey::External(canonical_external(&peeled)));
    }
    None
}

pub fn canonical_external(name: &str) -> String {
    let segs: Vec<&str> = name.split("::").filter(|s| !s.is_empty()).collect();
    segs.last().copied().unwrap_or(name).trim().to_string()
}

fn resolve_via_lexical(graph: &ScopeGraph, from: ScopeId, name: &str) -> Option<Target> {
    let at = scope_loc(graph, from)?;
    let q = ResolveQuery {
        name: name.to_string(),
        ns: NS_TYPE,
        from,
        at,
        cfg: CfgCtx::default(),
        ctx: PolicyQueryCtx::default(),
    };
    let policy = RustPolicy::new(graph, graph.edition);
    single_resolved_target(resolve(graph, &q, &policy))
}

fn resolve_via_path(graph: &ScopeGraph, from: ScopeId, path: &str) -> Option<Target> {
    let (anchor, raw) = type_path_anchor(path)?;
    if raw.0.is_empty() {
        return None;
    }
    let at = scope_loc(graph, from)?;
    let policy = RustPolicy::new(graph, graph.edition);
    single_resolved_target(resolve_path(
        graph, &raw, NS_TYPE, &anchor, from, NS_TYPE, &at, &policy,
    ))
}

fn single_resolved_target(res: crate::name_resolution::types::Resolution) -> Option<Target> {
    match (res.status, res.candidates.as_slice()) {
        (ResStatus::Resolved, [Candidate { target, .. }]) => Some(target.clone()),
        _ => None,
    }
}

fn type_path_anchor(raw: &str) -> Option<(Anchor, RawPath)> {
    let mut segs: Vec<String> = raw.split("::").map(str::to_string).collect();
    if segs.is_empty() {
        return None;
    }
    let anchor = match segs.first().map(String::as_str) {
        Some("") => {
            segs.remove(0);
            Anchor {
                kind: AnchorKind::LeadingColon,
                prelude: None,
            }
        }
        Some("crate") => {
            segs.remove(0);
            Anchor::crate_root()
        }
        Some("self") => {
            segs.remove(0);
            Anchor::self_mod()
        }
        Some("super") => {
            let mut n = 0u32;
            while matches!(segs.first().map(String::as_str), Some("super")) {
                segs.remove(0);
                n += 1;
            }
            Anchor::super_n(n)
        }
        Some(_) => Anchor::bare(),
        None => return None,
    };
    Some((anchor, RawPath(segs)))
}

fn type_scope_of_target(_graph: &ScopeGraph, target: &Target) -> Option<ScopeId> {
    match target {
        Target::Item { owns: Some(s), .. } => Some(*s),
        Target::Scope(s) => Some(*s),
        _ => None,
    }
}

fn is_confidently_external(name: &str) -> bool {
    let segs: Vec<&str> = name.split("::").filter(|s| !s.is_empty()).collect();
    matches!(segs.first().copied(), Some("std" | "core" | "alloc"))
        || matches!(segs.as_slice(), [bare] if is_known_std_bare_type(bare))
}

fn is_known_std_bare_type(name: &str) -> bool {
    !name.contains("::")
        && matches!(
            name,
            "String"
                | "Vec"
                | "VecDeque"
                | "BTreeMap"
                | "BTreeSet"
                | "HashMap"
                | "HashSet"
                | "Box"
                | "Arc"
                | "Rc"
                | "Option"
                | "Result"
                | "str"
                | "bool"
                | "char"
                | "usize"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "f32"
                | "f64"
        )
}

fn scope_loc(graph: &ScopeGraph, scope: ScopeId) -> Option<SourceLoc> {
    graph
        .scope(scope)?
        .extents
        .first()
        .map(|e| e.range.lo.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::call_graph::ScopeGraphBuildInputs;
    use crate::languages::Language::Rust;
    use crate::name_resolution::rust_populator::RustCrateConfig;
    use crate::name_resolution::types::ScopeKind;

    fn graph_of(srcs: &[(&str, &str)]) -> crate::name_resolution::graph::ScopeGraph {
        let mut files = std::collections::BTreeMap::new();
        for (p, s) in srcs {
            files.insert(p.to_string(), ParsedFile::parse(p, s, Rust).unwrap());
        }
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        crate::call_graph::CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
            .scope_graph
            .expect("scope graph")
    }

    fn module_scope(graph: &crate::name_resolution::graph::ScopeGraph, path: &str) -> ScopeId {
        let file = graph
            .file_paths
            .get(path)
            .copied()
            .unwrap_or_else(|| panic!("missing file path {path}"));
        graph
            .scopes
            .iter()
            .find_map(|(id, scope)| {
                let top_level = matches!(scope.kind, ScopeKind::Root | ScopeKind::Module);
                let in_file = scope.extents.iter().any(|ext| ext.file == file);
                (top_level && in_file).then_some(*id)
            })
            .unwrap_or_else(|| panic!("missing module scope for {path}"))
    }

    #[test]
    fn in_repo_first_then_external_and_unresolved_is_none() {
        let g = graph_of(&[("a.rs", "pub struct Foo;"), ("b.rs", "pub struct Foo;")]);
        let a = module_scope(&g, "a.rs");
        let b = module_scope(&g, "b.rs");
        let ka = resolve_type_path_to_type_scope(&g, a, "Foo");
        let kb = resolve_type_path_to_type_scope(&g, b, "Foo");
        assert!(
            matches!((&ka, &kb), (Some(TypeKey::InRepo(x)), Some(TypeKey::InRepo(y))) if x != y)
        );
        assert_eq!(
            resolve_type_path_to_type_scope(&g, a, "String"),
            Some(TypeKey::External("String".into()))
        );
        assert_eq!(
            resolve_type_path_to_type_scope(&g, a, "::std::string::String"),
            Some(TypeKey::External("String".into()))
        );
        assert_eq!(
            resolve_type_path_to_type_scope(&g, a, "::core::option::Option"),
            Some(TypeKey::External("Option".into()))
        );
        assert_eq!(resolve_type_path_to_type_scope(&g, a, "NoSuch"), None);
    }

    #[test]
    fn in_repo_struct_named_string_is_not_external() {
        let g = graph_of(&[("a.rs", "pub struct String;")]);
        let a = module_scope(&g, "a.rs");
        assert!(matches!(
            resolve_type_path_to_type_scope(&g, a, "String"),
            Some(TypeKey::InRepo(_))
        ));
    }

    #[test]
    fn external_canonicalization_unifies_std_paths() {
        assert_eq!(
            canonical_external("String"),
            canonical_external("std::string::String")
        );
        assert_eq!(
            canonical_external("std::string::String"),
            canonical_external("alloc::string::String")
        );
        assert_eq!(
            canonical_external("::std::string::String"),
            canonical_external("String")
        );
    }
}
