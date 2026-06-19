use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, FunctionId, MethodFacts, MethodKind, RecvMode};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{FileId, ScopeId};
use crate::resolution_identity::{resolve_type_path_to_type_scope, TypeKey};
use std::collections::BTreeMap;

type RustTypeAliases = BTreeMap<(ScopeId, String), Vec<(Option<String>, String)>>;
type RustFieldTypes = BTreeMap<(ScopeId, String), Vec<(Option<String>, TypeKey)>>;
type RustReturnTypes = BTreeMap<FunctionId, Vec<(Option<String>, TypeKey)>>;

impl CallGraph {
    pub(crate) fn rust_type_aliases_by_module(
        graph: &ScopeGraph,
        files: &BTreeMap<String, ParsedFile>,
    ) -> RustTypeAliases {
        let mut aliases: RustTypeAliases = BTreeMap::new();
        for (file_path, parsed) in files {
            if !matches!(parsed.language, crate::languages::Language::Rust) {
                continue;
            }
            let Some(file_id) = graph.file_paths.get(file_path).copied() else {
                continue;
            };
            Self::visit_rust_nodes(parsed.tree.root_node(), &mut |node| {
                if node.kind() != "type_item" || Self::has_rust_item_ancestor(&node) {
                    return;
                }
                let Some(module_scope) =
                    Self::module_scope_for_byte(graph, file_id, node.start_byte())
                else {
                    return;
                };
                let (Some(name), Some(target)) = (
                    node.child_by_field_name("name"),
                    node.child_by_field_name("type"),
                ) else {
                    return;
                };
                aliases
                    .entry((module_scope, parsed.node_text(&name).trim().to_string()))
                    .or_default()
                    .push((
                        Self::raw_cfg_attr(parsed, &node),
                        parsed.node_text(&target).trim().to_string(),
                    ));
            });
        }
        aliases
    }

    pub(crate) fn extract_rust_field_types(
        graph: &ScopeGraph,
        aliases: &RustTypeAliases,
        file_id: FileId,
        parsed: &ParsedFile,
        field_types: &mut RustFieldTypes,
    ) {
        Self::visit_rust_nodes(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "struct_item" || Self::has_rust_item_ancestor(&node) {
                return;
            }
            let Some(module_scope) = Self::module_scope_for_byte(graph, file_id, node.start_byte())
            else {
                return;
            };
            let Some(name_node) = node.child_by_field_name("name") else {
                return;
            };
            let struct_name = parsed.node_text(&name_node).trim();
            let Some(TypeKey::InRepo(owner_scope)) =
                resolve_type_path_to_type_scope(graph, module_scope, struct_name)
            else {
                return;
            };
            let Some(body) = node.child_by_field_name("body") else {
                return;
            };
            match body.kind() {
                "field_declaration_list" => {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() != "field_declaration" {
                            continue;
                        }
                        let (Some(name), Some(type_node)) = (
                            child.child_by_field_name("name"),
                            child.child_by_field_name("type"),
                        ) else {
                            continue;
                        };
                        let field_name = parsed.node_text(&name).trim().to_string();
                        let type_syntax = parsed.node_text(&type_node);
                        if crate::resolution_receiver::std_wrapper_was_peeled(type_syntax) {
                            continue;
                        }
                        let item_cfg = Self::raw_cfg_attr(parsed, &child);
                        for (type_cfg, key) in
                            Self::resolve_index_types(graph, aliases, module_scope, type_syntax)
                        {
                            field_types
                                .entry((owner_scope, field_name.clone()))
                                .or_default()
                                .push((Self::combine_cfg_attrs(item_cfg.clone(), type_cfg), key));
                        }
                    }
                }
                "ordered_field_declaration_list" => {
                    let mut position = 0usize;
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if !child.is_named()
                            || matches!(child.kind(), "attribute_item" | "visibility_modifier")
                        {
                            continue;
                        }
                        let type_syntax = parsed.node_text(&child);
                        if crate::resolution_receiver::std_wrapper_was_peeled(type_syntax) {
                            position += 1;
                            continue;
                        }
                        let item_cfg = Self::raw_cfg_attr(parsed, &child);
                        for (type_cfg, key) in
                            Self::resolve_index_types(graph, aliases, module_scope, type_syntax)
                        {
                            field_types
                                .entry((owner_scope, position.to_string()))
                                .or_default()
                                .push((Self::combine_cfg_attrs(item_cfg.clone(), type_cfg), key));
                        }
                        position += 1;
                    }
                }
                _ => {}
            }
        });
    }

    pub(crate) fn extract_rust_return_types(
        graph: &ScopeGraph,
        aliases: &RustTypeAliases,
        file_path: &str,
        file_id: FileId,
        parsed: &ParsedFile,
        return_types: &mut RustReturnTypes,
    ) {
        for func_node in parsed.all_functions() {
            if parsed
                .language
                .rust_enclosing_method_item(&func_node)
                .is_none()
                && Self::has_rust_function_ancestor(&func_node)
            {
                continue;
            }
            let Some(return_type) = func_node.child_by_field_name("return_type") else {
                continue;
            };
            let Some(name_node) = parsed.language.function_name(&func_node) else {
                continue;
            };
            let Some(module_scope) =
                Self::module_scope_for_byte(graph, file_id, func_node.start_byte())
            else {
                continue;
            };
            let return_syntax = parsed.node_text(&return_type);
            if crate::resolution_receiver::std_wrapper_was_peeled(return_syntax) {
                continue;
            }
            let keys = if crate::resolution::peel_type(return_syntax) == "Self" {
                Self::resolve_impl_self_return(graph, module_scope, parsed, &func_node)
                    .map(|key| vec![(None, key)])
                    .unwrap_or_default()
            } else if matches!(
                parsed
                    .language
                    .rust_enclosing_method_item(&func_node)
                    .map(|n| n.kind()),
                Some("trait_item")
            ) {
                Vec::new()
            } else {
                Self::resolve_index_types(graph, aliases, module_scope, return_syntax)
            };
            if keys.is_empty() {
                continue;
            }
            let (start, end) = parsed.node_line_range(&func_node);
            let fid = FunctionId {
                file: file_path.to_string(),
                name: parsed.node_text(&name_node).to_string(),
                start_line: start,
                end_line: end,
            };
            let item_cfg = Self::raw_cfg_attr(parsed, &func_node);
            let entries = return_types.entry(fid).or_default();
            for (type_cfg, key) in keys {
                entries.push((Self::combine_cfg_attrs(item_cfg.clone(), type_cfg), key));
            }
        }
    }

    fn resolve_impl_self_return(
        graph: &ScopeGraph,
        module_scope: ScopeId,
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
    ) -> Option<TypeKey> {
        let enclosing = parsed.language.rust_enclosing_method_item(func_node)?;
        if enclosing.kind() != "impl_item" {
            return None;
        }
        let owner = enclosing.child_by_field_name("type")?;
        match resolve_type_path_to_type_scope(graph, module_scope, parsed.node_text(&owner)) {
            Some(TypeKey::InRepo(scope)) => Some(TypeKey::InRepo(scope)),
            _ => None,
        }
    }

    fn resolve_index_types(
        graph: &ScopeGraph,
        aliases: &RustTypeAliases,
        module_scope: ScopeId,
        type_syntax: &str,
    ) -> Vec<(Option<String>, TypeKey)> {
        Self::resolve_index_type_inner(graph, aliases, module_scope, type_syntax, 0)
    }

    fn resolve_index_type_inner(
        graph: &ScopeGraph,
        aliases: &RustTypeAliases,
        module_scope: ScopeId,
        type_syntax: &str,
        depth: usize,
    ) -> Vec<(Option<String>, TypeKey)> {
        if depth > 8 {
            return Vec::new();
        }
        // A std wrapper (Arc/Box/Rc/Pin) at THIS level — including an alias target
        // reached by recursion (e.g. `type BoxedFoo = Box<Foo>`) — is not a sound
        // receiver type; fail closed so wrapper-owned methods (e.g. clone) are never
        // dispatched on the inner type. (Reference-only `&mut Self`/`&Foo` peels are
        // NOT std-wrapper peels, so the builder-chain buy is preserved.)
        if crate::resolution_receiver::std_wrapper_was_peeled(type_syntax) {
            return Vec::new();
        }
        let peeled = crate::resolution::peel_type(type_syntax);
        if peeled.is_empty() || peeled == "Self" {
            return Vec::new();
        }
        if !peeled.contains("::") {
            if let Some(targets) = aliases.get(&(module_scope, peeled.clone())) {
                let mut entries = Vec::new();
                for (alias_cfg, target) in targets {
                    for (target_cfg, key) in Self::resolve_index_type_inner(
                        graph,
                        aliases,
                        module_scope,
                        target,
                        depth + 1,
                    ) {
                        entries.push((Self::combine_cfg_attrs(alias_cfg.clone(), target_cfg), key));
                    }
                }
                let entries = Self::dedupe_cfg_type_entries(entries);
                if Self::has_unconditioned_conflicting_targets(&entries) {
                    return Vec::new();
                }
                return entries;
            }
        }
        match resolve_type_path_to_type_scope(graph, module_scope, &peeled) {
            Some(TypeKey::InRepo(scope)) => vec![(None, TypeKey::InRepo(scope))],
            _ => Vec::new(),
        }
    }

    fn combine_cfg_attrs(item_cfg: Option<String>, alias_cfg: Option<String>) -> Option<String> {
        match (item_cfg, alias_cfg) {
            (None, None) => None,
            (Some(cfg), None) | (None, Some(cfg)) => Some(cfg),
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(left), Some(right)) => Some(format!("{left} && {right}")),
        }
    }

    fn dedupe_cfg_type_entries(
        entries: Vec<(Option<String>, TypeKey)>,
    ) -> Vec<(Option<String>, TypeKey)> {
        let mut deduped = Vec::new();
        for entry in entries {
            if !deduped.contains(&entry) {
                deduped.push(entry);
            }
        }
        deduped
    }

    fn has_unconditioned_conflicting_targets(entries: &[(Option<String>, TypeKey)]) -> bool {
        entries.iter().any(|(cfg, _)| cfg.is_none())
            && entries
                .iter()
                .any(|(_, left)| entries.iter().any(|(_, right)| left != right))
    }

    fn visit_rust_nodes<'a>(
        node: tree_sitter::Node<'a>,
        f: &mut impl FnMut(tree_sitter::Node<'a>),
    ) {
        f(node);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::visit_rust_nodes(child, f);
        }
    }

    fn has_rust_item_ancestor(node: &tree_sitter::Node<'_>) -> bool {
        let mut parent = node.parent();
        while let Some(p) = parent {
            if matches!(p.kind(), "function_item" | "impl_item" | "trait_item") {
                return true;
            }
            parent = p.parent();
        }
        false
    }

    fn has_rust_function_ancestor(node: &tree_sitter::Node<'_>) -> bool {
        let mut parent = node.parent();
        while let Some(p) = parent {
            if p.kind() == "function_item" {
                return true;
            }
            parent = p.parent();
        }
        false
    }

    pub(crate) fn insert_method_by_scope(
        methods_by_scope: &mut BTreeMap<(ScopeId, String), Vec<FunctionId>>,
        scope: ScopeId,
        name: String,
        fid: FunctionId,
    ) {
        let fids = methods_by_scope.entry((scope, name)).or_default();
        if !fids.contains(&fid) {
            fids.push(fid);
        }
    }

    pub(crate) fn insert_extension_method(
        extension_methods: &mut BTreeMap<(String, String), Vec<FunctionId>>,
        canon: String,
        name: String,
        fid: FunctionId,
    ) {
        let fids = extension_methods.entry((canon, name)).or_default();
        if !fids.contains(&fid) {
            fids.push(fid);
        }
    }

    pub(crate) fn method_facts(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
    ) -> Option<MethodFacts> {
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            return None;
        }
        let enclosing = parsed.language.rust_enclosing_method_item(func_node)?;
        let kind = match enclosing.kind() {
            "trait_item" => {
                let trait_name = enclosing.child_by_field_name("name")?;
                MethodKind::Trait(crate::resolution::owner_key(parsed.node_text(&trait_name)))
            }
            "impl_item" => parsed
                .language
                .rust_impl_trait(func_node)
                .map(|n| MethodKind::Trait(crate::resolution::owner_key(parsed.node_text(&n))))
                .unwrap_or(MethodKind::Inherent),
            _ => return None,
        };
        let params = parsed.language.method_params(func_node);
        let self_param = parsed.language.self_param(func_node);
        let has_self = self_param.is_some();
        let recv_mode = self_param
            .map(|n| {
                if n.kind() == "self_parameter" {
                    let text = parsed.node_text(&n);
                    if text.contains('&') && text.contains("mut") {
                        RecvMode::SelfRefMut
                    } else if text.contains('&') {
                        RecvMode::SelfRef
                    } else {
                        RecvMode::SelfBy
                    }
                } else {
                    match n.child_by_field_name("type") {
                        Some(ty) if ty.kind() == "reference_type" => {
                            let mut c = ty.walk();
                            if ty
                                .children(&mut c)
                                .any(|ch| ch.kind() == "mutable_specifier")
                            {
                                RecvMode::SelfRefMut
                            } else {
                                RecvMode::SelfRef
                            }
                        }
                        _ => RecvMode::SelfBy,
                    }
                }
            })
            .unwrap_or(RecvMode::None);
        let arity_excl_self = params.len().saturating_sub(usize::from(has_self));
        Some(MethodFacts {
            kind,
            has_self,
            recv_mode,
            arity_excl_self,
            cfg: Self::raw_cfg_attr(parsed, func_node),
        })
    }

    pub(crate) fn raw_cfg_attr(
        parsed: &ParsedFile,
        item: &tree_sitter::Node<'_>,
    ) -> Option<String> {
        let mut prev = item.prev_sibling();
        while let Some(sib) = prev {
            if sib.kind() != "attribute_item" {
                break;
            }
            let text = parsed.node_text(&sib).trim();
            if text.starts_with("#[cfg(") {
                return Some(text.to_string());
            }
            prev = sib.prev_sibling();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::ScopeGraphBuildInputs;

    fn build_rust_call_graph(source: &str) -> CallGraph {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        use crate::name_resolution::rust_populator::RustCrateConfig;

        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.rs".into(),
            ParsedFile::parse("a.rs", source, Rust).unwrap(),
        );
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    }

    fn in_repo_type_scope(cg: &CallGraph, path: &str, name: &str) -> ScopeId {
        let graph = cg.scope_graph.as_ref().expect("scope graph");
        let file = graph.file_paths.get(path).copied().expect("file id");
        let scope = CallGraph::module_scope_for_byte(graph, file, 0).expect("module scope");
        match resolve_type_path_to_type_scope(graph, scope, name) {
            Some(TypeKey::InRepo(scope)) => scope,
            other => panic!("expected in-repo type scope for {name}, got {other:?}"),
        }
    }

    #[test]
    fn method_facts_distinguish_inherent_trait_self_arity() {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.rs".to_string(),
            ParsedFile::parse(
                "a.rs",
                "struct S;\nimpl S { fn inh(&self, x: u8) {} fn assoc() {} fn by(self) {} fn r(&self) {} fn rm(&mut self) {} fn boxed(self: Box<Self>, a: u8, b: u8) {} fn pinned(self: Pin<&mut Self>) {} fn tyref(self: &Self) {} fn tymut(self: &mut Self) {} }\n\
         trait T { fn td(&self) {} }\nimpl T for S { fn tm(&self) {} }\n",
                Rust,
            )
            .unwrap(),
        );
        let cg = CallGraph::build(&files);
        let fid = |n: &str| {
            cg.functions
                .values()
                .flatten()
                .find(|f| f.name == n)
                .cloned()
                .unwrap()
        };
        let inh = cg.method_facts.get(&fid("inh")).unwrap();
        assert_eq!(inh.kind, MethodKind::Inherent);
        assert!(inh.has_self && inh.arity_excl_self == 1);
        assert!(!cg.method_facts.get(&fid("assoc")).unwrap().has_self);
        let by = cg.method_facts.get(&fid("by")).unwrap();
        assert!(by.has_self);
        assert_eq!(by.recv_mode, RecvMode::SelfBy);
        assert_eq!(by.arity_excl_self, 0);
        let r = cg.method_facts.get(&fid("r")).unwrap();
        assert!(r.has_self);
        assert_eq!(r.recv_mode, RecvMode::SelfRef);
        let rm = cg.method_facts.get(&fid("rm")).unwrap();
        assert!(rm.has_self);
        assert_eq!(rm.recv_mode, RecvMode::SelfRefMut);
        let boxed = cg.method_facts.get(&fid("boxed")).unwrap();
        assert!(boxed.has_self);
        assert_eq!(boxed.recv_mode, RecvMode::SelfBy);
        assert_eq!(boxed.arity_excl_self, 2);
        let pinned = cg.method_facts.get(&fid("pinned")).unwrap();
        assert!(pinned.has_self);
        assert_eq!(pinned.recv_mode, RecvMode::SelfBy);
        assert_eq!(
            cg.method_facts.get(&fid("tyref")).unwrap().recv_mode,
            RecvMode::SelfRef
        );
        assert_eq!(
            cg.method_facts.get(&fid("tymut")).unwrap().recv_mode,
            RecvMode::SelfRefMut
        );
        assert!(matches!(
            cg.method_facts.get(&fid("td")).unwrap().kind,
            MethodKind::Trait(_)
        ));
        assert!(matches!(
            cg.method_facts.get(&fid("tm")).unwrap().kind,
            MethodKind::Trait(_)
        ));
    }

    #[test]
    fn methods_by_scope_distinct_per_module_and_marks_complete() {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        use crate::name_resolution::rust_populator::RustCrateConfig;
        let mut files = std::collections::BTreeMap::new();
        for path in ["a.rs", "b.rs"] {
            files.insert(
                path.into(),
                ParsedFile::parse(path, "pub struct Foo; impl Foo { fn m(&self){} }\n", Rust)
                    .unwrap(),
            );
        }
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
        let a_foo = in_repo_type_scope(&cg, "a.rs", "Foo");
        let b_foo = in_repo_type_scope(&cg, "b.rs", "Foo");
        assert_ne!(a_foo, b_foo);
        assert_eq!(
            cg.methods_by_scope
                .keys()
                .filter(|(_, name)| name == "m")
                .count(),
            2
        );
        assert!(cg
            .identity_complete
            .contains(&("Foo".to_string(), "m".to_string())));
    }

    #[test]
    fn external_trait_bucket_is_not_identity_complete() {
        let cg = build_rust_call_graph(
            "pub struct Foo;\nimpl std::fmt::Debug for Foo { fn fmt(&self){} }\n",
        );

        assert!(cg
            .methods
            .contains_key(&("Debug".to_string(), "fmt".to_string())));
        assert!(cg.methods_by_scope.keys().any(|(_, name)| name == "fmt"));
        assert!(!cg
            .identity_complete
            .contains(&("Debug".to_string(), "fmt".to_string())));
    }

    #[test]
    fn in_repo_trait_dual_key_buckets_are_identity_complete() {
        let cg = build_rust_call_graph(
            "pub trait Tr { fn t(&self); }\npub struct Foo;\nimpl Tr for Foo { fn t(&self){} }\n",
        );

        assert!(cg
            .identity_complete
            .contains(&("Foo".to_string(), "t".to_string())));
        assert!(cg
            .identity_complete
            .contains(&("Tr".to_string(), "t".to_string())));
    }

    #[test]
    fn unresolved_concrete_owner_bucket_is_not_identity_complete() {
        let cg = build_rust_call_graph("impl Missing { fn ghost(&self){} }\n");

        assert!(cg
            .methods
            .contains_key(&("Missing".to_string(), "ghost".to_string())));
        assert!(!cg
            .identity_complete
            .contains(&("Missing".to_string(), "ghost".to_string())));
    }
}
