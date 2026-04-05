//! Rust type provider — extracts struct, enum, trait, and impl definitions
//! from tree-sitter ASTs for trait-based dispatch resolution.
//!
//! Rust's trait system is nominal: a type satisfies a trait only via explicit
//! `impl Trait for Type` blocks. This provider extracts these relationships
//! and resolves dynamic dispatch (`dyn Trait`) to concrete implementations.
//!
//! **Limitation:** Local variable types are often inferred in Rust. This
//! provider only knows types that are explicitly annotated or defined as
//! struct/enum/trait items. `resolve_type` returns `None` for inferred locals.

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use crate::type_provider::{
    DispatchProvider, ResolvedType, ResolvedTypeKind, TypeFieldInfo, TypeProvider,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Extracted type information
// ---------------------------------------------------------------------------

/// A Rust struct definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RustStruct {
    /// Struct name.
    name: String,
    /// Fields: name → type string. Empty for unit/tuple structs.
    fields: BTreeMap<String, String>,
    /// Source file path.
    file: String,
    /// Line number of the struct declaration.
    line: usize,
}

/// A Rust enum definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RustEnum {
    /// Enum name.
    name: String,
    /// Variant names.
    variants: Vec<String>,
    /// Source file.
    file: String,
    /// Line number.
    line: usize,
}

/// A Rust trait definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RustTrait {
    /// Trait name.
    name: String,
    /// Method signatures: name → signature string.
    methods: BTreeMap<String, String>,
    /// Supertraits (e.g., `trait Foo: Bar + Baz`).
    supertraits: Vec<String>,
    /// Source file.
    file: String,
    /// Line number.
    line: usize,
}

/// A method from an impl block.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RustMethod {
    /// Method name.
    name: String,
    /// The type this method is implemented for.
    impl_type: String,
    /// The trait being implemented (None for inherent impls).
    impl_trait: Option<String>,
    /// Signature string.
    signature: String,
    /// Source file.
    file: String,
    /// Start line.
    start_line: usize,
    /// End line.
    end_line: usize,
}

// ---------------------------------------------------------------------------
// RustTypeProvider
// ---------------------------------------------------------------------------

/// Inner data shared via Arc.
pub struct RustTypeData {
    /// Struct definitions by name.
    structs: BTreeMap<String, RustStruct>,
    /// Enum definitions by name.
    enums: BTreeMap<String, RustEnum>,
    /// Trait definitions by name.
    traits: BTreeMap<String, RustTrait>,
    /// Type aliases: alias_name → target type string.
    aliases: BTreeMap<String, String>,
    /// Methods grouped by implementing type name.
    /// Includes both inherent methods and trait impl methods.
    methods: BTreeMap<String, Vec<RustMethod>>,
    /// Precomputed trait satisfaction: trait_name → set of concrete types.
    satisfaction: BTreeMap<String, BTreeSet<String>>,
}

/// Rust type provider extracting struct/enum/trait/impl definitions from
/// tree-sitter ASTs.
///
/// Uses `Arc<RustTypeData>` so the same data can be shared when registered
/// as both `TypeProvider` and `DispatchProvider` in the registry.
#[derive(Clone)]
pub struct RustTypeProvider {
    pub data: Arc<RustTypeData>,
}

impl RustTypeProvider {
    /// Build a RustTypeProvider by scanning all Rust parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = RustTypeData {
            structs: BTreeMap::new(),
            enums: BTreeMap::new(),
            traits: BTreeMap::new(),
            aliases: BTreeMap::new(),
            methods: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if parsed.language != Language::Rust {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        Self::compute_satisfaction(&mut inner);
        RustTypeProvider {
            data: Arc::new(inner),
        }
    }

    // -----------------------------------------------------------------------
    // AST extraction
    // -----------------------------------------------------------------------

    fn extract_from_file(data: &mut RustTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            Self::extract_top_level(data, &child, path, parsed);
        }
    }

    /// Extract top-level items from the AST.
    ///
    /// TODO: nested items (impl blocks inside functions, modules with inline
    /// items) are not extracted. This covers the common case of top-level
    /// definitions.
    fn extract_top_level(
        data: &mut RustTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        match node.kind() {
            "struct_item" => {
                Self::extract_struct(data, node, path, parsed);
            }
            "enum_item" => {
                Self::extract_enum(data, node, path, parsed);
            }
            "trait_item" => {
                Self::extract_trait(data, node, path, parsed);
            }
            "impl_item" => {
                Self::extract_impl(data, node, path, parsed);
            }
            "type_item" => {
                Self::extract_type_alias(data, node, parsed);
            }
            "mod_item" => {
                // Recurse into inline module bodies.
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        Self::extract_top_level(data, &child, path, parsed);
                    }
                }
            }
            _ => {}
        }
    }

    /// Extract a struct definition.
    fn extract_struct(
        data: &mut RustTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).trim().to_string(),
            None => return,
        };
        if name.is_empty() {
            return;
        }

        let line = node.start_position().row + 1;
        let fields = Self::extract_struct_fields(node, parsed);

        data.structs.insert(
            name.clone(),
            RustStruct {
                name,
                fields,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract fields from a struct body (field_declaration_list).
    /// Tuple structs (`struct Foo(i32, String)`) have unnamed positional fields
    /// in an `ordered_field_declaration_list` and are intentionally skipped —
    /// only named struct fields are tracked.
    fn extract_struct_fields(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return fields,
        };

        if body.kind() != "field_declaration_list" {
            return fields;
        }

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let field_name = child
                    .child_by_field_name("name")
                    .map(|n| parsed.node_text(&n).trim().to_string());
                let field_type = child
                    .child_by_field_name("type")
                    .map(|t| parsed.node_text(&t).trim().to_string());

                if let (Some(name), Some(type_str)) = (field_name, field_type) {
                    if !name.is_empty() {
                        fields.insert(name, type_str);
                    }
                }
            }
        }

        fields
    }

    /// Extract an enum definition.
    fn extract_enum(
        data: &mut RustTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).trim().to_string(),
            None => return,
        };
        if name.is_empty() {
            return;
        }

        let line = node.start_position().row + 1;
        let mut variants = Vec::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => {
                data.enums.insert(
                    name.clone(),
                    RustEnum {
                        name,
                        variants,
                        file: path.to_string(),
                        line,
                    },
                );
                return;
            }
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                if let Some(vname) = child.child_by_field_name("name") {
                    let variant_name = parsed.node_text(&vname).trim().to_string();
                    if !variant_name.is_empty() {
                        variants.push(variant_name);
                    }
                }
            }
        }

        data.enums.insert(
            name.clone(),
            RustEnum {
                name,
                variants,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract a trait definition.
    fn extract_trait(
        data: &mut RustTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).trim().to_string(),
            None => return,
        };
        if name.is_empty() {
            return;
        }

        let line = node.start_position().row + 1;
        let mut supertraits = Vec::new();

        // Extract supertraits from trait_bounds (e.g., `trait Foo: Bar + Baz`).
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "trait_bounds" {
                Self::collect_trait_bounds(&child, parsed, &mut supertraits);
            }
        }

        let methods = Self::extract_trait_methods(node, parsed);

        data.traits.insert(
            name.clone(),
            RustTrait {
                name,
                methods,
                supertraits,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract method signatures from a trait body.
    fn extract_trait_methods(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> BTreeMap<String, String> {
        let mut methods = BTreeMap::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return methods,
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" || child.kind() == "function_signature_item" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = parsed.node_text(&name_node).trim().to_string();
                    let sig = Self::build_fn_signature(&child, parsed);
                    if !name.is_empty() {
                        methods.insert(name, sig);
                    }
                }
            }
        }

        methods
    }

    /// Extract an impl block: `impl Type { ... }` or `impl Trait for Type { ... }`.
    fn extract_impl(
        data: &mut RustTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        // Determine the type being implemented and optional trait.
        let (impl_type, impl_trait) = Self::parse_impl_header(node, parsed);
        if impl_type.is_empty() {
            return;
        }

        // Extract methods from the impl body.
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return,
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = parsed.node_text(&name_node).trim().to_string();
                    if name.is_empty() {
                        continue;
                    }

                    let sig = Self::build_fn_signature(&child, parsed);
                    let start_line = child.start_position().row + 1;
                    let end_line = child.end_position().row + 1;

                    data.methods
                        .entry(impl_type.clone())
                        .or_default()
                        .push(RustMethod {
                            name,
                            impl_type: impl_type.clone(),
                            impl_trait: impl_trait.clone(),
                            signature: sig,
                            file: path.to_string(),
                            start_line,
                            end_line,
                        });
                }
            }
        }
    }

    /// Parse the impl header to get (impl_type, Option<trait_name>).
    ///
    /// `impl Foo { ... }` → ("Foo", None)
    /// `impl Bar for Foo { ... }` → ("Foo", Some("Bar"))
    fn parse_impl_header(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (String, Option<String>) {
        let mut impl_type = String::new();
        let mut impl_trait = None;

        // tree-sitter-rust impl_item fields:
        // - "type": the type being implemented
        // - "trait": the trait being implemented (if trait impl)
        if let Some(type_node) = node.child_by_field_name("type") {
            impl_type = parsed.node_text(&type_node).trim().to_string();
            // Strip generic params: Foo<T> → Foo
            if let Some(base) = impl_type.split('<').next() {
                impl_type = base.trim().to_string();
            }
        }

        if let Some(trait_node) = node.child_by_field_name("trait") {
            let trait_text = parsed.node_text(&trait_node).trim().to_string();
            let base = trait_text
                .split('<')
                .next()
                .unwrap_or(&trait_text)
                .trim()
                .to_string();
            if !base.is_empty() {
                impl_trait = Some(base);
            }
        }

        (impl_type, impl_trait)
    }

    /// Extract a type alias: `type Foo = Bar;`.
    fn extract_type_alias(data: &mut RustTypeData, node: &tree_sitter::Node, parsed: &ParsedFile) {
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).trim().to_string(),
            None => return,
        };
        let type_node = match node.child_by_field_name("type") {
            Some(t) => t,
            None => return,
        };
        let target = parsed.node_text(&type_node).trim().to_string();
        if !name.is_empty() && !target.is_empty() {
            data.aliases.insert(name, target);
        }
    }

    /// Build a function signature string from parameters and return type.
    fn build_fn_signature(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(parsed.node_text(&params).trim().to_string());
        }

        if let Some(ret) = node.child_by_field_name("return_type") {
            parts.push(parsed.node_text(&ret).trim().to_string());
        }

        parts.join(" ")
    }

    /// Collect trait names from a trait_bounds node.
    fn collect_trait_bounds(node: &tree_sitter::Node, parsed: &ParsedFile, out: &mut Vec<String>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" | "scoped_type_identifier" | "generic_type" => {
                    let text = parsed.node_text(&child).trim().to_string();
                    let base = text.split('<').next().unwrap_or(&text).trim();
                    if !base.is_empty() {
                        out.push(base.to_string());
                    }
                }
                // Recurse into nested structures.
                _ if child.named_child_count() > 0 => {
                    Self::collect_trait_bounds(&child, parsed, out);
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Trait satisfaction
    // -----------------------------------------------------------------------

    /// Compute which concrete types satisfy which traits via `impl Trait for Type`.
    fn compute_satisfaction(data: &mut RustTypeData) {
        // Collect explicit trait implementations from methods.
        for methods in data.methods.values() {
            for method in methods {
                if let Some(trait_name) = &method.impl_trait {
                    data.satisfaction
                        .entry(trait_name.clone())
                        .or_default()
                        .insert(method.impl_type.clone());
                }
            }
        }

        // Propagate supertrait satisfaction: if T: Foo and Foo: Bar,
        // then T also satisfies Bar.
        let trait_names: Vec<String> = data.traits.keys().cloned().collect();
        for trait_name in &trait_names {
            if let Some(implementors) = data.satisfaction.get(trait_name).cloned() {
                let supertraits =
                    Self::collect_all_supertraits(data, trait_name, &mut BTreeSet::new());
                for supertrait in supertraits {
                    let entry = data.satisfaction.entry(supertrait).or_default();
                    for implementor in &implementors {
                        entry.insert(implementor.clone());
                    }
                }
            }
        }
    }

    /// Collect all supertraits transitively.
    fn collect_all_supertraits(
        data: &RustTypeData,
        trait_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> Vec<String> {
        if !visited.insert(trait_name.to_string()) {
            return Vec::new();
        }

        let mut result = Vec::new();
        if let Some(t) = data.traits.get(trait_name) {
            for supertrait in &t.supertraits {
                result.push(supertrait.clone());
                result.extend(Self::collect_all_supertraits(data, supertrait, visited));
            }
        }
        result
    }

    /// Resolve all trait methods, flattening supertrait chains.
    fn resolve_trait_methods(
        data: &RustTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(name.to_string()) {
            return BTreeMap::new();
        }

        let mut methods = BTreeMap::new();
        if let Some(t) = data.traits.get(name) {
            methods.extend(t.methods.clone());
            for supertrait in &t.supertraits {
                let parent_methods = Self::resolve_trait_methods(data, supertrait, visited);
                for (k, v) in parent_methods {
                    methods.entry(k).or_insert(v);
                }
            }
        }
        methods
    }
}

// ---------------------------------------------------------------------------
// TypeProvider implementation
// ---------------------------------------------------------------------------

impl TypeProvider for RustTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        if self.data.structs.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Concrete,
                type_params: Vec::new(),
            });
        }
        if self.data.traits.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Interface,
                type_params: Vec::new(),
            });
        }
        if self.data.enums.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Enum,
                type_params: Vec::new(),
            });
        }
        if let Some(target) = self.data.aliases.get(expr) {
            return Some(ResolvedType {
                name: target.clone(),
                kind: ResolvedTypeKind::Alias,
                type_params: Vec::new(),
            });
        }
        None
    }

    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>> {
        // Struct fields + inherent methods.
        if let Some(s) = self.data.structs.get(type_name) {
            let mut fields: Vec<TypeFieldInfo> = s
                .fields
                .iter()
                .map(|(name, type_str)| TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                })
                .collect();

            // Add inherent methods (impl Type, no trait).
            if let Some(methods) = self.data.methods.get(type_name) {
                for m in methods {
                    if m.impl_trait.is_none() {
                        fields.push(TypeFieldInfo {
                            name: m.name.clone(),
                            type_str: m.signature.clone(),
                        });
                    }
                }
            }

            return Some(fields);
        }

        // Trait methods (flattened with supertraits).
        if self.data.traits.contains_key(type_name) {
            let all_methods =
                Self::resolve_trait_methods(&self.data, type_name, &mut BTreeSet::new());
            let fields: Vec<TypeFieldInfo> = all_methods
                .into_iter()
                .map(|(name, sig)| TypeFieldInfo {
                    name,
                    type_str: sig,
                })
                .collect();
            return Some(fields);
        }

        // Enum: inherent methods only.
        if self.data.enums.contains_key(type_name) {
            let mut fields = Vec::new();
            if let Some(methods) = self.data.methods.get(type_name) {
                for m in methods {
                    if m.impl_trait.is_none() {
                        fields.push(TypeFieldInfo {
                            name: m.name.clone(),
                            type_str: m.signature.clone(),
                        });
                    }
                }
            }
            return Some(fields);
        }

        None
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // For traits: return types that implement the trait.
        self.data
            .satisfaction
            .get(type_name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn resolve_alias(&self, type_name: &str) -> String {
        self.data
            .aliases
            .get(type_name)
            .cloned()
            .unwrap_or_else(|| type_name.to_string())
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Rust]
    }
}

// ---------------------------------------------------------------------------
// DispatchProvider implementation
// ---------------------------------------------------------------------------

impl DispatchProvider for RustTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        // If receiver_type is a concrete type (struct/enum), resolve directly.
        if self.data.structs.contains_key(receiver_type)
            || self.data.enums.contains_key(receiver_type)
        {
            if let Some(fid) = self.find_method_on_type(receiver_type, method) {
                return vec![fid];
            }
        }

        // If receiver_type is a trait (dyn Trait), find all implementing types.
        if let Some(satisfying) = self.data.satisfaction.get(receiver_type) {
            let candidates: BTreeSet<&String> = if live_types.is_empty() {
                satisfying.iter().collect()
            } else {
                satisfying.intersection(live_types).collect()
            };

            // RTA fallback: if filtering eliminates all targets, use full set.
            let targets = if candidates.is_empty() && !live_types.is_empty() {
                satisfying.iter().collect::<Vec<_>>()
            } else {
                candidates.into_iter().collect::<Vec<_>>()
            };

            let mut results = Vec::new();
            for type_name in targets {
                if let Some(fid) = self.find_method_on_type(type_name, method) {
                    results.push(fid);
                }
            }
            return results;
        }

        Vec::new()
    }
}

impl RustTypeProvider {
    /// Find a method on a concrete type (checking both inherent and trait impls).
    /// Prefers inherent methods over trait impls (Rust method resolution order).
    fn find_method_on_type(&self, type_name: &str, method: &str) -> Option<FunctionId> {
        if let Some(methods) = self.data.methods.get(type_name) {
            let inherent = methods
                .iter()
                .find(|m| m.name == method && m.impl_trait.is_none());
            let any = methods.iter().find(|m| m.name == method);
            if let Some(m) = inherent.or(any) {
                return Some(FunctionId {
                    name: method.to_string(),
                    file: m.file.clone(),
                    start_line: m.start_line,
                    end_line: m.end_line,
                });
            }
        }
        None
    }
}
