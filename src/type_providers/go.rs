//! Go type provider — extracts struct/interface definitions and method sets
//! from tree-sitter ASTs for interface satisfaction and dispatch resolution.
//!
//! Go's type system is structural: a type satisfies an interface if it has all
//! the interface's methods, regardless of explicit `implements` declarations.
//! This provider computes satisfaction by comparing method sets.

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

/// A Go struct definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoStruct {
    /// Struct name.
    name: String,
    /// Fields: (name, type_string). Anonymous/embedded fields have name == type.
    fields: Vec<(String, String)>,
    /// Embedded type names (for promoted methods).
    embedded: Vec<String>,
    /// Source file path.
    file: String,
    /// Line number of the type declaration.
    line: usize,
}

/// A Go interface definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoInterface {
    /// Interface name.
    name: String,
    /// Method signatures: method_name → parameter/return signature string.
    methods: BTreeMap<String, String>,
    /// Embedded interfaces.
    embedded: Vec<String>,
    /// Source file path.
    file: String,
}

/// A method attached to a type via a receiver.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoMethod {
    /// Method name.
    name: String,
    /// Receiver type name (without pointer, e.g., "Server" not "*Server").
    receiver_type: String,
    /// Whether the receiver is a pointer receiver (*T).
    is_pointer_receiver: bool,
    /// Signature string for interface matching (parameter types → return types).
    signature: String,
    /// Source file.
    file: String,
    /// Start line.
    start_line: usize,
    /// End line.
    end_line: usize,
}

// ---------------------------------------------------------------------------
// GoTypeProvider
// ---------------------------------------------------------------------------

/// Inner data for GoTypeProvider, shared via Arc when registered as both
/// TypeProvider and DispatchProvider.
pub struct GoTypeData {
    /// Struct definitions by name.
    structs: BTreeMap<String, GoStruct>,
    /// Interface definitions by name.
    interfaces: BTreeMap<String, GoInterface>,
    /// Methods grouped by receiver type name.
    methods: BTreeMap<String, Vec<GoMethod>>,
    /// Type aliases: alias_name → canonical_name.
    aliases: BTreeMap<String, String>,
    /// Precomputed interface satisfaction: interface_name → set of concrete types.
    satisfaction: BTreeMap<String, BTreeSet<String>>,
}

/// Go type provider that extracts struct/interface definitions and method sets
/// from tree-sitter ASTs, then computes interface satisfaction for dispatch.
///
/// Uses `Arc<GoTypeData>` so the same extracted data can be shared when
/// registered as both `TypeProvider` and `DispatchProvider` in the registry.
#[derive(Clone)]
pub struct GoTypeProvider {
    pub data: Arc<GoTypeData>,
}

impl GoTypeProvider {
    /// Build a GoTypeProvider by scanning all Go parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = GoTypeData {
            structs: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            methods: BTreeMap::new(),
            aliases: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if parsed.language != Language::Go {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        Self::compute_satisfaction(&mut inner);
        GoTypeProvider {
            data: Arc::new(inner),
        }
    }

    // -----------------------------------------------------------------------
    // AST extraction (static methods operating on GoTypeData)
    // -----------------------------------------------------------------------

    /// Extract type information from a single Go file.
    fn extract_from_file(data: &mut GoTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "type_declaration" => {
                    Self::extract_type_declaration(data, &child, path, parsed);
                }
                "method_declaration" => {
                    Self::extract_method(data, &child, path, parsed);
                }
                _ => {}
            }
        }
    }

    /// Extract type specs from a type_declaration node.
    fn extract_type_declaration(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_spec" => Self::extract_type_spec(data, &child, path, parsed),
                "type_alias" => Self::extract_type_alias(data, &child, parsed),
                _ => {}
            }
        }
    }

    /// Extract a single type_spec: `name type_definition`.
    fn extract_type_spec(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();
        if name.is_empty() {
            return;
        }

        let type_node = match node.child_by_field_name("type") {
            Some(n) => n,
            None => return,
        };

        let line = node.start_position().row + 1;

        match type_node.kind() {
            "struct_type" => {
                let (fields, embedded) = Self::extract_struct_fields(&type_node, parsed);
                data.structs.insert(
                    name.clone(),
                    GoStruct {
                        name,
                        fields,
                        embedded,
                        file: path.to_string(),
                        line,
                    },
                );
            }
            "interface_type" => {
                let (methods, embedded) = Self::extract_interface_methods(&type_node, parsed);
                data.interfaces.insert(
                    name.clone(),
                    GoInterface {
                        name,
                        methods,
                        embedded,
                        file: path.to_string(),
                    },
                );
            }
            _ => {
                // Type definition: `type Name OtherType`
                let target = parsed.node_text(&type_node).trim().to_string();
                if !target.is_empty() {
                    data.aliases.insert(name, target);
                }
            }
        }
    }

    /// Extract a type alias: `type Name = OtherType`.
    fn extract_type_alias(data: &mut GoTypeData, node: &tree_sitter::Node, parsed: &ParsedFile) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let type_node = match node.child_by_field_name("type") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();
        let target = parsed.node_text(&type_node).trim().to_string();
        if !name.is_empty() && !target.is_empty() {
            data.aliases.insert(name, target);
        }
    }

    /// Extract fields from a struct_type node.
    fn extract_struct_fields(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let mut fields = Vec::new();
        let mut embedded = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "field_declaration_list" {
                let mut inner = child.walk();
                for field in child.children(&mut inner) {
                    if field.kind() == "field_declaration" {
                        Self::extract_one_field(&field, parsed, &mut fields, &mut embedded);
                    }
                }
            }
        }
        (fields, embedded)
    }

    /// Extract a single field_declaration.
    fn extract_one_field(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        fields: &mut Vec<(String, String)>,
        embedded: &mut Vec<String>,
    ) {
        let mut names: Vec<String> = Vec::new();
        let mut type_str = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "field_identifier" => {
                    names.push(parsed.node_text(&child).trim().to_string());
                }
                _ if type_str.is_empty()
                    && child.kind() != "field_identifier"
                    && child.kind() != "tag"
                    && child.kind() != "comment"
                    && child.kind() != "," =>
                {
                    if child.kind() != "field_identifier" {
                        type_str = parsed.node_text(&child).trim().to_string();
                    }
                }
                _ => {}
            }
        }

        if names.is_empty() {
            let embedded_name = strip_pointer(&type_str);
            if !embedded_name.is_empty() {
                embedded.push(embedded_name.to_string());
                fields.push((embedded_name.to_string(), type_str));
            }
        } else {
            for name in names {
                fields.push((name, type_str.clone()));
            }
        }
    }

    /// Extract method signatures from an interface_type node.
    fn extract_interface_methods(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (BTreeMap<String, String>, Vec<String>) {
        let mut methods = BTreeMap::new();
        let mut embedded = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_interface_body(&child, parsed, &mut methods, &mut embedded);
        }

        (methods, embedded)
    }

    fn walk_interface_body(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        methods: &mut BTreeMap<String, String>,
        embedded: &mut Vec<String>,
    ) {
        match node.kind() {
            "method_spec" | "method_elem" => {
                let mut name = String::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "field_identifier" && name.is_empty() {
                        name = parsed.node_text(&child).trim().to_string();
                    }
                }
                if !name.is_empty() {
                    let sig = Self::extract_method_signature(node, parsed);
                    methods.insert(name, sig);
                }
            }
            "type_identifier" | "qualified_type" => {
                let iface_name = parsed.node_text(node).trim().to_string();
                if !iface_name.is_empty() {
                    embedded.push(iface_name);
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::walk_interface_body(&child, parsed, methods, embedded);
                }
            }
        }
    }

    /// Extract a method's parameter+return type signature for comparison.
    fn extract_method_signature(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();
        let mut cursor = node.walk();
        let mut skip_name = true;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "field_identifier" if skip_name => {
                    skip_name = false;
                    continue;
                }
                "parameter_list" => {
                    parts.push(parsed.node_text(&child).trim().to_string());
                }
                "type_identifier" | "pointer_type" | "qualified_type" | "slice_type"
                | "map_type" | "channel_type" | "array_type" | "interface_type"
                | "function_type" => {
                    parts.push(parsed.node_text(&child).trim().to_string());
                }
                _ => {}
            }
        }
        parts.join(" -> ")
    }

    /// Extract a method_declaration (method with receiver).
    fn extract_method(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();

        let (receiver_type, is_pointer) = match Self::extract_receiver(node, parsed) {
            Some(r) => r,
            None => return,
        };

        let sig = Self::extract_func_signature(node, parsed);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        data.methods
            .entry(receiver_type.clone())
            .or_default()
            .push(GoMethod {
                name,
                receiver_type,
                is_pointer_receiver: is_pointer,
                signature: sig,
                file: path.to_string(),
                start_line,
                end_line,
            });
    }

    /// Extract receiver type from a method_declaration.
    fn extract_receiver(node: &tree_sitter::Node, parsed: &ParsedFile) -> Option<(String, bool)> {
        let receiver = node.child_by_field_name("receiver")?;
        let mut cursor = receiver.walk();
        for child in receiver.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                if let Some(type_node) = child.child_by_field_name("type") {
                    let type_text = parsed.node_text(&type_node).trim().to_string();
                    let is_pointer = type_text.starts_with('*');
                    let base_type = strip_pointer(&type_text).to_string();
                    if !base_type.is_empty() {
                        return Some((base_type, is_pointer));
                    }
                }
            }
        }
        None
    }

    /// Extract function signature (parameters + return) for interface matching.
    fn extract_func_signature(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(parsed.node_text(&params).trim().to_string());
        }

        if let Some(result) = node.child_by_field_name("result") {
            parts.push(parsed.node_text(&result).trim().to_string());
        }

        parts.join(" -> ")
    }

    // -----------------------------------------------------------------------
    // Interface satisfaction (static, operates on GoTypeData)
    // -----------------------------------------------------------------------

    /// Compute which concrete types satisfy which interfaces.
    fn compute_satisfaction(data: &mut GoTypeData) {
        let interface_methods = Self::resolve_all_interface_methods(data);
        let concrete_methods = Self::resolve_all_concrete_methods(data);

        for (iface_name, iface_methods) in &interface_methods {
            let mut satisfying = BTreeSet::new();
            for (type_name, type_methods) in &concrete_methods {
                if iface_methods
                    .keys()
                    .all(|method_name| type_methods.contains_key(method_name))
                {
                    satisfying.insert(type_name.clone());
                }
            }
            data.satisfaction.insert(iface_name.clone(), satisfying);
        }
    }

    /// Resolve all methods for each interface, flattening embedded interfaces.
    fn resolve_all_interface_methods(
        data: &GoTypeData,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        let mut result = BTreeMap::new();
        for iface_name in data.interfaces.keys() {
            let methods =
                Self::collect_interface_methods_from(data, iface_name, &mut BTreeSet::new());
            result.insert(iface_name.clone(), methods);
        }
        result
    }

    fn collect_interface_methods_from(
        data: &GoTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(name.to_string()) {
            return BTreeMap::new();
        }

        let mut methods = BTreeMap::new();
        if let Some(iface) = data.interfaces.get(name) {
            methods.extend(iface.methods.clone());
            for embedded in &iface.embedded {
                let embedded_methods =
                    Self::collect_interface_methods_from(data, embedded, visited);
                methods.extend(embedded_methods);
            }
        }
        methods
    }

    /// Resolve all methods for each concrete type, including promoted methods.
    fn resolve_all_concrete_methods(
        data: &GoTypeData,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        let mut result = BTreeMap::new();

        for (type_name, type_methods) in &data.methods {
            let mut method_map = BTreeMap::new();
            for m in type_methods {
                method_map.insert(m.name.clone(), m.signature.clone());
            }
            result.insert(type_name.clone(), method_map);
        }

        for (struct_name, go_struct) in &data.structs {
            let entry = result.entry(struct_name.clone()).or_default();
            Self::collect_promoted_methods_from(data, go_struct, entry, &mut BTreeSet::new());
        }

        result
    }

    fn collect_promoted_methods_from(
        data: &GoTypeData,
        go_struct: &GoStruct,
        methods: &mut BTreeMap<String, String>,
        visited: &mut BTreeSet<String>,
    ) {
        if !visited.insert(go_struct.name.clone()) {
            return;
        }

        for embedded_name in &go_struct.embedded {
            if let Some(embedded_methods) = data.methods.get(embedded_name) {
                for m in embedded_methods {
                    methods.entry(m.name.clone()).or_insert(m.signature.clone());
                }
            }
            if let Some(inner_struct) = data.structs.get(embedded_name) {
                Self::collect_promoted_methods_from(data, inner_struct, methods, visited);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TypeProvider implementation
// ---------------------------------------------------------------------------

impl TypeProvider for GoTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        // Check if expr is a known type name.
        if let Some(_s) = self.data.structs.get(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Concrete,
                type_params: Vec::new(),
            });
        }
        if self.data.interfaces.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Interface,
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
        let go_struct = self.data.structs.get(type_name)?;
        Some(
            go_struct
                .fields
                .iter()
                .map(|(name, type_str)| TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                })
                .collect(),
        )
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // In Go, "subtypes" of an interface means concrete types that satisfy it.
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
        vec![Language::Go]
    }
}

// ---------------------------------------------------------------------------
// DispatchProvider implementation
// ---------------------------------------------------------------------------

impl DispatchProvider for GoTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        // If receiver_type is a concrete type, resolve directly.
        if let Some(type_methods) = self.data.methods.get(receiver_type) {
            for m in type_methods {
                if m.name == method {
                    return vec![FunctionId {
                        name: method.to_string(),
                        file: m.file.clone(),
                        start_line: m.start_line,
                        end_line: m.end_line,
                    }];
                }
            }
        }

        // If receiver_type is an interface, find all satisfying types that have this method.
        if let Some(satisfying) = self.data.satisfaction.get(receiver_type) {
            let candidates: BTreeSet<&String> = if live_types.is_empty() {
                satisfying.iter().collect()
            } else {
                satisfying.intersection(live_types).collect()
            };

            // If RTA filtering eliminates all targets, fall back to full set.
            let targets = if candidates.is_empty() && !live_types.is_empty() {
                satisfying.iter().collect::<Vec<_>>()
            } else {
                candidates.into_iter().collect::<Vec<_>>()
            };

            let mut results = Vec::new();
            for type_name in targets {
                if let Some(type_methods) = self.data.methods.get(type_name) {
                    for m in type_methods {
                        if m.name == method {
                            results.push(FunctionId {
                                name: method.to_string(),
                                file: m.file.clone(),
                                start_line: m.start_line,
                                end_line: m.end_line,
                            });
                        }
                    }
                }
            }
            return results;
        }

        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip leading `*` from a pointer type: `*Server` → `Server`.
fn strip_pointer(s: &str) -> &str {
    s.strip_prefix('*').unwrap_or(s).trim()
}
