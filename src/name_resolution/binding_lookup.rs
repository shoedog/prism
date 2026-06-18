//! Direct local-binding lookup for build-time receiver typing.
//!
//! This intentionally returns the original [`Binding`] rather than the engine's
//! [`Candidate`](crate::name_resolution::types::Candidate), because the
//! Task-2.3 receiver typer needs the binding span as a def-byte identity.

use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{EK_GLOB, NS_VALUE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::{BindTarget, Binding, FileId, ScopeKind, SourceLoc, Target};
use serde::{Deserialize, Serialize};

/// Syntactic kind of a Rust local binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BindingKind {
    Param,
    Let,
    Pattern,
}

/// Syntactic initializer shape for a Rust local binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum InitExpr {
    /// `T::new()` / `T::default()` / `T { ... }`.
    Ctor(String),
    /// `e.f`.
    Field(String),
    /// `g(...)`.
    Call(String),
    Other,
}

/// Syntactic local-binding fact keyed by `(FileId, def_byte)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalFact {
    pub kind: BindingKind,
    pub annotation: Option<String>,
    pub init: Option<InitExpr>,
}

/// Return the nearest visible value binding named `name` at `(file, at_byte)`.
pub fn lookup_visible_binding<'a>(
    graph: &'a ScopeGraph,
    file: FileId,
    at_byte: usize,
    name: &str,
) -> Option<&'a Binding> {
    let at = SourceLoc {
        file,
        byte: at_byte,
    };
    let mut cur = enclosing_scope(graph, file, at_byte);
    while let Some(scope_id) = cur {
        let mut matches = graph.bindings.iter().filter(|binding| {
            binding.scope == scope_id
                && binding.name == name
                && binding.ns == NS_VALUE
                && matches!(&binding.target, BindTarget::Resolved(Target::Local(_)))
                && vis_extent_covers(binding, &at)
        });
        let first = matches.next();
        if matches.next().is_some() {
            return None;
        }
        if first.is_some() {
            return first;
        }
        if macro_wildcard_poisons(graph, scope_id, &at) {
            return None;
        }
        let kind = match graph.scope(scope_id) {
            Some(scope) => &scope.kind,
            None => return None,
        };
        if !matches!(
            kind,
            ScopeKind::Block | ScopeKind::Callable | ScopeKind::Type
        ) {
            break;
        }
        cur = graph.parent_of(scope_id);
    }
    None
}

fn vis_extent_covers(binding: &Binding, at: &SourceLoc) -> bool {
    if binding.vis_extents.is_empty() {
        return true;
    }
    binding
        .vis_extents
        .iter()
        .any(|span| span.lo.file == at.file && at.byte >= span.lo.byte && at.byte < span.hi.byte)
}

fn macro_wildcard_poisons(
    graph: &ScopeGraph,
    scope: crate::name_resolution::types::ScopeId,
    at: &SourceLoc,
) -> bool {
    graph.macro_wildcards.iter().any(|wildcard| {
        wildcard.scope == scope
            && wildcard.ns == NS_VALUE
            && wildcard.range.lo.file == at.file
            && at.byte >= wildcard.range.lo.byte
            && at.byte < wildcard.range.hi.byte
    }) || graph.edges.iter().any(|edge| {
        edge.from == scope
            && edge.kind == EK_GLOB
            && matches!(&edge.to, BindTarget::Pending(_, _))
            && edge.vis_range.as_ref().is_none_or(|span| {
                span.lo.file == at.file && at.byte >= span.lo.byte && at.byte < span.hi.byte
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::call_graph::{CallGraph, ScopeGraphBuildInputs};
    use crate::languages::Language::Rust;
    use crate::name_resolution::rust_populator::RustCrateConfig;

    #[test]
    fn lookup_visible_binding_returns_binding_by_name_and_byte() {
        let src = "struct X;\nimpl X { fn new() -> Self { X } fn m(&self) {} }\nfn f(){ let b = X::new(); { let b = X::new(); b.m(); } }\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.rs".to_string(),
            ParsedFile::parse("a.rs", src, Rust).unwrap(),
        );
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
        let graph = cg.scope_graph.as_ref().expect("scope graph");
        let file = graph.file_paths["a.rs"];
        let outer_let_b_def_byte = src.find("b = X::new").unwrap();
        let inner_let_b_def_byte = src.rfind("b = X::new").unwrap();
        let call_byte = src.rfind("b.m()").unwrap();

        let binding = lookup_visible_binding(graph, file, call_byte, "b").expect("binding");

        let binding_def_byte = binding.vis_extents.first().unwrap().lo.byte;
        assert_eq!(binding_def_byte, inner_let_b_def_byte);
        assert_ne!(binding_def_byte, outer_let_b_def_byte);
        assert!(matches!(
            graph
                .local_facts
                .get(&(file, inner_let_b_def_byte))
                .and_then(|fact| fact.init.as_ref()),
            Some(InitExpr::Ctor(s)) if s == "X::new()"
        ));
    }
}
