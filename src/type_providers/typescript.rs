//! TypeScript type provider — extracts interface, class, type alias, and enum
//! definitions from tree-sitter ASTs for dispatch resolution and structural
//! typing.
//!
//! TypeScript's type system is structural: a value is assignable to a type if
//! it has all the required properties, regardless of explicit `implements`.
//! This provider supports both nominal dispatch (class `implements` interface)
//! and structural compatibility (property-name comparison).

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use crate::type_provider::{
    Compatibility, DispatchProvider, ResolvedType, ResolvedTypeKind, StructuralTypingProvider,
    TypeFieldInfo, TypeProvider,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Extracted type information
// ---------------------------------------------------------------------------

/// A TypeScript interface definition.
///
/// Some fields (name, file, line) are reserved for Phase 7 (type-enriched
/// finding descriptions) and CompatibilitySlice.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TsInterface {
    /// Interface name.
    name: String,
    /// Properties: name → type string.
    properties: BTreeMap<String, String>,
    /// Method signatures: name → signature string.
    methods: BTreeMap<String, String>,
    /// Extended interfaces.
    extends: Vec<String>,
    /// Source file path.
    file: String,
    /// Line number.
    line: usize,
}

/// A TypeScript class definition.
///
/// Some fields (name, file, line) are reserved for Phase 7 (type-enriched
/// finding descriptions) and CompatibilitySlice.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TsClass {
    /// Class name.
    name: String,
    /// Properties: name → type string.
    properties: BTreeMap<String, String>,
    /// Methods: name → TsMethod.
    methods: BTreeMap<String, TsMethod>,
    /// Interfaces this class explicitly implements.
    implements: Vec<String>,
    /// Parent class (extends).
    extends: Option<String>,
    /// Source file path.
    file: String,
    /// Line number.
    line: usize,
}

/// A method within a TypeScript class.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TsMethod {
    /// Method name.
    name: String,
    /// Signature string.
    signature: String,
    /// Source file.
    file: String,
    /// Start line.
    start_line: usize,
    /// End line.
    end_line: usize,
}

/// A TypeScript enum definition.
///
/// Members field reserved for CompatibilitySlice.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TsEnum {
    /// Enum name.
    name: String,
    /// Member names.
    members: Vec<String>,
    /// Source file.
    file: String,
}

// ---------------------------------------------------------------------------
// TypeScriptTypeProvider
// ---------------------------------------------------------------------------

/// Inner data shared via Arc.
pub struct TsTypeData {
    /// Interface definitions by name.
    interfaces: BTreeMap<String, TsInterface>,
    /// Class definitions by name.
    classes: BTreeMap<String, TsClass>,
    /// Type aliases: alias_name → target type string.
    aliases: BTreeMap<String, String>,
    /// Enum definitions by name.
    enums: BTreeMap<String, TsEnum>,
    /// Precomputed: interface_name → set of class names that satisfy it.
    satisfaction: BTreeMap<String, BTreeSet<String>>,
}

/// TypeScript type provider extracting interface/class/type-alias/enum
/// definitions from tree-sitter ASTs.
///
/// Uses `Arc<TsTypeData>` so the same data can be shared when registered
/// as `TypeProvider`, `DispatchProvider`, and `StructuralTypingProvider`.
#[derive(Clone)]
pub struct TypeScriptTypeProvider {
    pub data: Arc<TsTypeData>,
}

impl TypeScriptTypeProvider {
    /// Build a provider by scanning all TypeScript/TSX parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = TsTypeData {
            interfaces: BTreeMap::new(),
            classes: BTreeMap::new(),
            aliases: BTreeMap::new(),
            enums: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if !matches!(parsed.language, Language::TypeScript | Language::Tsx) {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        Self::compute_satisfaction(&mut inner);
        TypeScriptTypeProvider {
            data: Arc::new(inner),
        }
    }

    // -----------------------------------------------------------------------
    // AST extraction (static methods operating on TsTypeData)
    // -----------------------------------------------------------------------

    fn extract_from_file(data: &mut TsTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            Self::extract_top_level(data, &child, path, parsed);
        }
    }

    fn extract_top_level(
        data: &mut TsTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        match node.kind() {
            "interface_declaration" => {
                Self::extract_interface(data, node, path, parsed);
            }
            "class_declaration" => {
                Self::extract_class(data, node, path, parsed);
            }
            "type_alias_declaration" => {
                Self::extract_type_alias(data, node, parsed);
            }
            "enum_declaration" => {
                Self::extract_enum(data, node, path, parsed);
            }
            "export_statement" => {
                // Recurse into `export interface ...`, `export class ...`, etc.
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_top_level(data, &child, path, parsed);
                }
            }
            _ => {}
        }
    }

    /// Extract an interface declaration.
    fn extract_interface(
        data: &mut TsTypeData,
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
        let mut extends = Vec::new();

        // Check for extends clause
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "extends_type_clause" {
                Self::collect_type_list(&child, parsed, &mut extends);
            }
        }

        let (properties, methods) = Self::extract_interface_body(node, parsed);

        data.interfaces.insert(
            name.clone(),
            TsInterface {
                name,
                properties,
                methods,
                extends,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract properties and method signatures from an interface body.
    fn extract_interface_body(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
        let mut properties = BTreeMap::new();
        let mut methods = BTreeMap::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return (properties, methods),
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "property_signature" => {
                    if let Some((name, type_str)) = Self::extract_property_sig(&child, parsed) {
                        properties.insert(name, type_str);
                    }
                }
                "method_signature" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = parsed.node_text(&name_node).trim().to_string();
                        let sig = Self::extract_signature_text(&child, parsed);
                        methods.insert(name, sig);
                    }
                }
                _ => {}
            }
        }

        (properties, methods)
    }

    /// Extract a property signature: `name: type` or `name?: type`.
    fn extract_property_sig(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> Option<(String, String)> {
        let name_node = node.child_by_field_name("name")?;
        let name = parsed.node_text(&name_node).trim().to_string();

        let type_str = node
            .child_by_field_name("type")
            .map(|t| {
                let text = parsed.node_text(&t).trim().to_string();
                // type_annotation nodes include the leading `: `, strip it
                text.strip_prefix(':').unwrap_or(&text).trim().to_string()
            })
            .unwrap_or_default();

        Some((name, type_str))
    }

    /// Extract a class declaration.
    fn extract_class(
        data: &mut TsTypeData,
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
        let mut implements = Vec::new();
        let mut extends = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "implements_clause" => {
                    Self::collect_type_list(&child, parsed, &mut implements);
                }
                "class_heritage" => {
                    // In some tree-sitter versions, extends/implements are
                    // children of class_heritage.
                    let mut inner = child.walk();
                    for hchild in child.children(&mut inner) {
                        match hchild.kind() {
                            "extends_clause" => {
                                if let Some(val) = hchild.child_by_field_name("value") {
                                    extends = Some(parsed.node_text(&val).trim().to_string());
                                } else {
                                    let mut hc = hchild.walk();
                                    for c in hchild.children(&mut hc) {
                                        if c.kind() == "type_identifier" || c.kind() == "identifier"
                                        {
                                            extends = Some(parsed.node_text(&c).trim().to_string());
                                            break;
                                        }
                                    }
                                }
                            }
                            "implements_clause" => {
                                Self::collect_type_list(&hchild, parsed, &mut implements);
                            }
                            _ => {}
                        }
                    }
                }
                "extends_clause" => {
                    if let Some(val) = child.child_by_field_name("value") {
                        extends = Some(parsed.node_text(&val).trim().to_string());
                    } else {
                        let mut ec = child.walk();
                        for c in child.children(&mut ec) {
                            if c.kind() == "type_identifier" || c.kind() == "identifier" {
                                extends = Some(parsed.node_text(&c).trim().to_string());
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let (properties, methods) = Self::extract_class_body(node, path, parsed);

        data.classes.insert(
            name.clone(),
            TsClass {
                name,
                properties,
                methods,
                implements,
                extends,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract properties and methods from a class body.
    fn extract_class_body(
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) -> (BTreeMap<String, String>, BTreeMap<String, TsMethod>) {
        let mut properties = BTreeMap::new();
        let mut methods = BTreeMap::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return (properties, methods),
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "public_field_definition" | "property_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = parsed.node_text(&name_node).trim().to_string();
                        let type_str = child
                            .child_by_field_name("type")
                            .map(|t| {
                                let text = parsed.node_text(&t).trim().to_string();
                                text.strip_prefix(':').unwrap_or(&text).trim().to_string()
                            })
                            .unwrap_or_default();
                        properties.insert(name, type_str);
                    }
                }
                "method_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = parsed.node_text(&name_node).trim().to_string();
                        // Skip constructor for interface matching purposes
                        if name == "constructor" {
                            continue;
                        }
                        let sig = Self::extract_signature_text(&child, parsed);
                        let start_line = child.start_position().row + 1;
                        let end_line = child.end_position().row + 1;
                        methods.insert(
                            name.clone(),
                            TsMethod {
                                name,
                                signature: sig,
                                file: path.to_string(),
                                start_line,
                                end_line,
                            },
                        );
                    }
                }
                _ => {}
            }
        }

        (properties, methods)
    }

    /// Extract a type alias declaration: `type Name = Type`.
    fn extract_type_alias(data: &mut TsTypeData, node: &tree_sitter::Node, parsed: &ParsedFile) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();

        let value_node = match node.child_by_field_name("value") {
            Some(n) => n,
            None => return,
        };
        let target = parsed.node_text(&value_node).trim().to_string();

        if !name.is_empty() && !target.is_empty() {
            data.aliases.insert(name, target);
        }
    }

    /// Extract an enum declaration.
    fn extract_enum(
        data: &mut TsTypeData,
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

        let mut members = Vec::new();
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => {
                data.enums.insert(
                    name.clone(),
                    TsEnum {
                        name,
                        members,
                        file: path.to_string(),
                    },
                );
                return;
            }
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_member" || child.kind() == "property_identifier" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    members.push(parsed.node_text(&name_node).trim().to_string());
                } else {
                    // Some tree-sitter versions use the child text directly.
                    let text = parsed.node_text(&child).trim().to_string();
                    if !text.is_empty() && !text.contains('{') && !text.contains('}') {
                        // Strip any initializer: `Foo = 1` → `Foo`
                        let member_name = text.split('=').next().unwrap_or("").trim().to_string();
                        if !member_name.is_empty() {
                            members.push(member_name);
                        }
                    }
                }
            }
        }

        data.enums.insert(
            name.clone(),
            TsEnum {
                name,
                members,
                file: path.to_string(),
            },
        );
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Collect type identifiers from a list clause (extends, implements).
    fn collect_type_list(node: &tree_sitter::Node, parsed: &ParsedFile, out: &mut Vec<String>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" | "identifier" => {
                    let text = parsed.node_text(&child).trim().to_string();
                    if !text.is_empty() {
                        out.push(text);
                    }
                }
                "generic_type" => {
                    // Extract the base name from `Foo<T>`.
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let text = parsed.node_text(&name_node).trim().to_string();
                        if !text.is_empty() {
                            out.push(text);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract a textual signature from a method/function node for comparison.
    fn extract_signature_text(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();
        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(parsed.node_text(&params).trim().to_string());
        }
        if let Some(ret) = node.child_by_field_name("return_type") {
            let text = parsed.node_text(&ret).trim().to_string();
            parts.push(text.strip_prefix(':').unwrap_or(&text).trim().to_string());
        }
        parts.join(" -> ")
    }

    // -----------------------------------------------------------------------
    // Interface satisfaction
    // -----------------------------------------------------------------------

    /// Compute which classes satisfy which interfaces.
    ///
    /// A class satisfies an interface if:
    /// 1. It explicitly `implements` the interface, OR
    /// 2. It has all the interface's required properties and methods (structural).
    fn compute_satisfaction(data: &mut TsTypeData) {
        // Resolve full interface shapes (flattening extends).
        let iface_shapes = Self::resolve_all_interface_shapes(data);

        for (iface_name, shape) in &iface_shapes {
            let mut satisfying = BTreeSet::new();

            for (class_name, class) in &data.classes {
                // Nominal: explicit `implements`.
                if class.implements.contains(iface_name) {
                    satisfying.insert(class_name.clone());
                    continue;
                }

                // Structural: check all properties and methods.
                let has_all_props = shape
                    .properties
                    .keys()
                    .all(|p| class.properties.contains_key(p) || class.methods.contains_key(p));
                let has_all_methods = shape.methods.keys().all(|m| class.methods.contains_key(m));

                if has_all_props && has_all_methods {
                    satisfying.insert(class_name.clone());
                }
            }

            data.satisfaction.insert(iface_name.clone(), satisfying);
        }
    }

    /// Resolve all interface shapes, flattening `extends` chains.
    fn resolve_all_interface_shapes(data: &TsTypeData) -> BTreeMap<String, InterfaceShape> {
        let mut result = BTreeMap::new();
        for iface_name in data.interfaces.keys() {
            let shape = Self::resolve_interface_shape(data, iface_name, &mut BTreeSet::new());
            result.insert(iface_name.clone(), shape);
        }
        result
    }

    fn resolve_interface_shape(
        data: &TsTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> InterfaceShape {
        if !visited.insert(name.to_string()) {
            return InterfaceShape::empty();
        }

        let mut shape = InterfaceShape::empty();
        if let Some(iface) = data.interfaces.get(name) {
            shape.properties.extend(iface.properties.clone());
            shape.methods.extend(iface.methods.clone());

            for parent in &iface.extends {
                let parent_shape = Self::resolve_interface_shape(data, parent, visited);
                // Parent properties/methods are inherited (don't overwrite child).
                for (k, v) in parent_shape.properties {
                    shape.properties.entry(k).or_insert(v);
                }
                for (k, v) in parent_shape.methods {
                    shape.methods.entry(k).or_insert(v);
                }
            }
        }
        shape
    }

    /// Get the full set of properties for a class, including inherited ones.
    fn resolve_class_properties(data: &TsTypeData, class_name: &str) -> BTreeMap<String, String> {
        Self::resolve_class_props_inner(data, class_name, &mut BTreeSet::new())
    }

    fn resolve_class_props_inner(
        data: &TsTypeData,
        class_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(class_name.to_string()) {
            return BTreeMap::new();
        }

        let mut props = BTreeMap::new();

        if let Some(class) = data.classes.get(class_name) {
            // Add parent class properties first.
            if let Some(parent) = &class.extends {
                let parent_props = Self::resolve_class_props_inner(data, parent, visited);
                props.extend(parent_props);
            }
            // Own properties override parent.
            props.extend(class.properties.clone());
            // Methods also count as properties in TS structural typing.
            for method_name in class.methods.keys() {
                props.entry(method_name.clone()).or_default();
            }
        }
        props
    }
}

/// Flattened interface shape for satisfaction checking.
struct InterfaceShape {
    properties: BTreeMap<String, String>,
    methods: BTreeMap<String, String>,
}

impl InterfaceShape {
    fn empty() -> Self {
        InterfaceShape {
            properties: BTreeMap::new(),
            methods: BTreeMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// TypeProvider implementation
// ---------------------------------------------------------------------------

impl TypeProvider for TypeScriptTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        if self.data.interfaces.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Interface,
                type_params: Vec::new(),
            });
        }
        if self.data.classes.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Concrete,
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
        if self.data.enums.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Enum,
                type_params: Vec::new(),
            });
        }
        None
    }

    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>> {
        // Interfaces: resolve full extends chain via resolve_interface_shape.
        if self.data.interfaces.contains_key(type_name) {
            let shape = TypeScriptTypeProvider::resolve_interface_shape(
                &self.data,
                type_name,
                &mut BTreeSet::new(),
            );
            let mut fields: Vec<TypeFieldInfo> = shape
                .properties
                .iter()
                .map(|(name, type_str)| TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                })
                .collect();
            for (name, sig) in &shape.methods {
                fields.push(TypeFieldInfo {
                    name: name.clone(),
                    type_str: sig.clone(),
                });
            }
            return Some(fields);
        }

        // Classes: resolve full extends chain via resolve_class_properties.
        if self.data.classes.contains_key(type_name) {
            let props = TypeScriptTypeProvider::resolve_class_properties(&self.data, type_name);
            let fields: Vec<TypeFieldInfo> = props
                .into_iter()
                .map(|(name, type_str)| TypeFieldInfo { name, type_str })
                .collect();
            return Some(fields);
        }

        None
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
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
        vec![Language::TypeScript, Language::Tsx]
    }
}

// ---------------------------------------------------------------------------
// DispatchProvider implementation
// ---------------------------------------------------------------------------

impl DispatchProvider for TypeScriptTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        // Direct class method lookup.
        if let Some(class) = self.data.classes.get(receiver_type) {
            if let Some(m) = class.methods.get(method) {
                return vec![FunctionId {
                    name: method.to_string(),
                    file: m.file.clone(),
                    start_line: m.start_line,
                    end_line: m.end_line,
                }];
            }
        }

        // Interface dispatch: find all satisfying classes with this method.
        if let Some(satisfying) = self.data.satisfaction.get(receiver_type) {
            let candidates: BTreeSet<&String> = if live_types.is_empty() {
                satisfying.iter().collect()
            } else {
                satisfying.intersection(live_types).collect()
            };

            // Fall back to full set if RTA eliminates all targets.
            let targets = if candidates.is_empty() && !live_types.is_empty() {
                satisfying.iter().collect::<Vec<_>>()
            } else {
                candidates.into_iter().collect::<Vec<_>>()
            };

            let mut results = Vec::new();
            for class_name in targets {
                if let Some(class) = self.data.classes.get(class_name) {
                    if let Some(m) = class.methods.get(method) {
                        results.push(FunctionId {
                            name: method.to_string(),
                            file: m.file.clone(),
                            start_line: m.start_line,
                            end_line: m.end_line,
                        });
                    }
                }
            }
            return results;
        }

        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// StructuralTypingProvider implementation
// ---------------------------------------------------------------------------

impl StructuralTypingProvider for TypeScriptTypeProvider {
    fn is_assignable_to(
        &self,
        value_type: &ResolvedType,
        target_type: &ResolvedType,
    ) -> Compatibility {
        // Same type is always compatible.
        if value_type.name == target_type.name {
            return Compatibility::Compatible;
        }

        // Get the target's required property/method names.
        let target_props = match self.field_layout(&target_type.name) {
            Some(fields) => fields,
            None => return Compatibility::Unknown,
        };

        // Get the value's available property/method names.
        let value_props = match self.field_layout(&value_type.name) {
            Some(fields) => fields,
            None => return Compatibility::Unknown,
        };

        let value_names: BTreeSet<&str> = value_props.iter().map(|f| f.name.as_str()).collect();

        // Check that all target properties exist on the value.
        let mut missing = Vec::new();
        for field in &target_props {
            if !value_names.contains(field.name.as_str()) {
                missing.push(field.name.clone());
            }
        }

        if missing.is_empty() {
            Compatibility::Compatible
        } else {
            Compatibility::Incompatible {
                reason: format!(
                    "{} is missing properties: {}",
                    value_type.name,
                    missing.join(", ")
                ),
            }
        }
    }

    fn resolve_generic(&self, _base_type: &str, _type_args: &[String]) -> Option<ResolvedType> {
        // Phase 3: simple implementation returns None (no generic support).
        None
    }
}
