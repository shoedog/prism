//! Java type provider — extracts class, interface, and enum definitions from
//! tree-sitter ASTs for class hierarchy dispatch resolution.
//!
//! Java's type system is nominal: a class satisfies an interface only via
//! explicit `implements` declarations, and inheritance is via `extends`.
//! This provider builds the class hierarchy and resolves virtual dispatch
//! using RTA (Rapid Type Analysis) filtering.

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

/// A Java class definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaClass {
    /// Class name.
    name: String,
    /// Fields: name → type string.
    fields: BTreeMap<String, String>,
    /// Methods defined in this class.
    methods: Vec<JavaMethod>,
    /// Interfaces this class explicitly implements.
    implements: Vec<String>,
    /// Parent class (extends), if any.
    extends: Option<String>,
    /// Source file path.
    file: String,
    /// Line number of the class declaration.
    line: usize,
}

/// A Java interface definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaInterface {
    /// Interface name.
    name: String,
    /// Method signatures: name → signature string.
    methods: BTreeMap<String, String>,
    /// Extended interfaces.
    extends: Vec<String>,
    /// Source file path.
    file: String,
    /// Line number.
    line: usize,
}

/// A method within a Java class.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaMethod {
    /// Method name.
    name: String,
    /// Signature string (parameter types → return type).
    signature: String,
    /// Source file.
    file: String,
    /// Start line.
    start_line: usize,
    /// End line.
    end_line: usize,
}

/// A Java enum definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaEnum {
    /// Enum name.
    name: String,
    /// Enum constant names.
    members: Vec<String>,
    /// Methods declared in the enum body.
    methods: Vec<JavaMethod>,
    /// Interfaces this enum implements.
    implements: Vec<String>,
    /// Source file.
    file: String,
}

// ---------------------------------------------------------------------------
// JavaTypeProvider
// ---------------------------------------------------------------------------

/// Inner data shared via Arc.
pub struct JavaTypeData {
    /// Class definitions by name.
    classes: BTreeMap<String, JavaClass>,
    /// Interface definitions by name.
    interfaces: BTreeMap<String, JavaInterface>,
    /// Enum definitions by name.
    enums: BTreeMap<String, JavaEnum>,
    /// Precomputed: interface_name → set of class/enum names that implement it.
    satisfaction: BTreeMap<String, BTreeSet<String>>,
    /// Precomputed: class_name → all ancestor classes (transitive extends).
    ancestors: BTreeMap<String, Vec<String>>,
}

/// Java type provider extracting class/interface/enum definitions from
/// tree-sitter ASTs.
///
/// Uses `Arc<JavaTypeData>` so the same data can be shared when registered
/// as both `TypeProvider` and `DispatchProvider` in the registry.
#[derive(Clone)]
pub struct JavaTypeProvider {
    pub data: Arc<JavaTypeData>,
}

impl JavaTypeProvider {
    /// Build a JavaTypeProvider by scanning all Java parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = JavaTypeData {
            classes: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            enums: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
            ancestors: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if parsed.language != Language::Java {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        Self::compute_ancestors(&mut inner);
        Self::compute_satisfaction(&mut inner);
        JavaTypeProvider {
            data: Arc::new(inner),
        }
    }

    // -----------------------------------------------------------------------
    // AST extraction (static methods operating on JavaTypeData)
    // -----------------------------------------------------------------------

    /// Extract type information from a single Java file.
    fn extract_from_file(data: &mut JavaTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            Self::extract_top_level(data, &child, path, parsed);
        }
    }

    /// Dispatch on a top-level AST node.
    ///
    /// TODO: inner/nested classes (class Outer { class Inner {} }) and
    /// anonymous classes are not extracted. These are common in Java
    /// (Builder pattern, event handlers, test fixtures). Supporting them
    /// requires recursing into class_body children.
    fn extract_top_level(
        data: &mut JavaTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        match node.kind() {
            "class_declaration" => {
                Self::extract_class(data, node, path, parsed);
            }
            "interface_declaration" => {
                Self::extract_interface(data, node, path, parsed);
            }
            "enum_declaration" => {
                Self::extract_enum(data, node, path, parsed);
            }
            "program" => {
                // Recurse into program children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_top_level(data, &child, path, parsed);
                }
            }
            _ => {}
        }
    }

    /// Extract a class declaration.
    fn extract_class(
        data: &mut JavaTypeData,
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
        let mut extends = None;
        let mut implements = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "superclass" => {
                    // superclass: "extends" keyword + type_identifier
                    let mut sc = child.walk();
                    for sc_child in child.children(&mut sc) {
                        if sc_child.kind() == "type_identifier"
                            || sc_child.kind() == "generic_type"
                            || sc_child.kind() == "scoped_type_identifier"
                        {
                            let text = parsed.node_text(&sc_child).trim().to_string();
                            let base = text.split('<').next().unwrap_or(&text).trim();
                            if !base.is_empty() {
                                extends = Some(base.to_string());
                            }
                            break;
                        }
                    }
                }
                "super_interfaces" => {
                    Self::collect_type_list(&child, parsed, &mut implements);
                }
                _ => {}
            }
        }

        let (fields, methods) = Self::extract_class_body(node, path, parsed);

        data.classes.insert(
            name.clone(),
            JavaClass {
                name,
                fields,
                methods,
                implements,
                extends,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract fields and methods from a class body.
    fn extract_class_body(
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) -> (BTreeMap<String, String>, Vec<JavaMethod>) {
        let mut fields = BTreeMap::new();
        let mut methods = Vec::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return (fields, methods),
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "field_declaration" => {
                    Self::extract_field(&child, parsed, &mut fields);
                }
                "method_declaration" => {
                    if let Some(m) = Self::extract_method(&child, path, parsed) {
                        methods.push(m);
                    }
                }
                "constructor_declaration" => {
                    if let Some(m) = Self::extract_constructor(&child, path, parsed) {
                        methods.push(m);
                    }
                }
                _ => {}
            }
        }

        (fields, methods)
    }

    /// Extract a field declaration: `type name = value;` or `type name;`.
    fn extract_field(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        fields: &mut BTreeMap<String, String>,
    ) {
        // field_declaration → type + variable_declarator(s)
        let type_str = node
            .child_by_field_name("type")
            .map(|t| parsed.node_text(&t).trim().to_string())
            .unwrap_or_default();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = parsed.node_text(&name_node).trim().to_string();
                    if !name.is_empty() {
                        fields.insert(name, type_str.clone());
                    }
                }
            }
        }
    }

    /// Extract a method declaration.
    fn extract_method(
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) -> Option<JavaMethod> {
        let name_node = node.child_by_field_name("name")?;
        let name = parsed.node_text(&name_node).trim().to_string();
        if name.is_empty() {
            return None;
        }

        let sig = Self::build_method_signature(node, parsed);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        Some(JavaMethod {
            name,
            signature: sig,
            file: path.to_string(),
            start_line,
            end_line,
        })
    }

    /// Extract a constructor declaration (treated as a method named `<init>`).
    fn extract_constructor(
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) -> Option<JavaMethod> {
        let name_node = node.child_by_field_name("name")?;
        let name = parsed.node_text(&name_node).trim().to_string();
        if name.is_empty() {
            return None;
        }

        let sig = Self::build_method_signature(node, parsed);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        Some(JavaMethod {
            name,
            signature: sig,
            file: path.to_string(),
            start_line,
            end_line,
        })
    }

    /// Build a signature string from method parameters and return type.
    fn build_method_signature(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();

        // Return type (for methods, not constructors).
        if let Some(type_node) = node.child_by_field_name("type") {
            parts.push(parsed.node_text(&type_node).trim().to_string());
        }

        // Parameters.
        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(parsed.node_text(&params).trim().to_string());
        }

        parts.join(" ")
    }

    /// Extract an interface declaration.
    fn extract_interface(
        data: &mut JavaTypeData,
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

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "extends_interfaces" {
                Self::collect_type_list(&child, parsed, &mut extends);
            }
        }

        let methods = Self::extract_interface_methods(node, parsed);

        data.interfaces.insert(
            name.clone(),
            JavaInterface {
                name,
                methods,
                extends,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract method signatures from an interface body.
    fn extract_interface_methods(
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
            if child.kind() == "method_declaration" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = parsed.node_text(&name_node).trim().to_string();
                    let sig = Self::build_method_signature(&child, parsed);
                    if !name.is_empty() {
                        methods.insert(name, sig);
                    }
                }
            }
        }

        methods
    }

    /// Extract an enum declaration.
    fn extract_enum(
        data: &mut JavaTypeData,
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
        let mut methods = Vec::new();
        let mut implements = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "super_interfaces" => {
                    Self::collect_type_list(&child, parsed, &mut implements);
                }
                "enum_body" => {
                    let mut inner = child.walk();
                    for member in child.children(&mut inner) {
                        match member.kind() {
                            "enum_constant" => {
                                if let Some(name_node) = member.child_by_field_name("name") {
                                    let member_name =
                                        parsed.node_text(&name_node).trim().to_string();
                                    if !member_name.is_empty() {
                                        members.push(member_name);
                                    }
                                }
                            }
                            // Methods in enums live inside enum_body_declarations.
                            "enum_body_declarations" => {
                                let mut decl_cursor = member.walk();
                                for decl in member.children(&mut decl_cursor) {
                                    match decl.kind() {
                                        "method_declaration" => {
                                            if let Some(m) =
                                                Self::extract_method(&decl, path, parsed)
                                            {
                                                methods.push(m);
                                            }
                                        }
                                        "constructor_declaration" => {
                                            if let Some(m) =
                                                Self::extract_constructor(&decl, path, parsed)
                                            {
                                                methods.push(m);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        data.enums.insert(
            name.clone(),
            JavaEnum {
                name,
                members,
                methods,
                implements,
                file: path.to_string(),
            },
        );
    }

    /// Collect type names from a type_list node (used by super_interfaces,
    /// extends_interfaces).
    fn collect_type_list(node: &tree_sitter::Node, parsed: &ParsedFile, out: &mut Vec<String>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_list" {
                // Recurse into type_list
                Self::collect_type_list(&child, parsed, out);
            } else if child.kind() == "type_identifier"
                || child.kind() == "generic_type"
                || child.kind() == "scoped_type_identifier"
            {
                let text = parsed.node_text(&child).trim().to_string();
                // Strip generic params for matching: Map<K,V> → Map
                let base = text.split('<').next().unwrap_or(&text).trim();
                if !base.is_empty() {
                    out.push(base.to_string());
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Hierarchy computation
    // -----------------------------------------------------------------------

    /// Compute transitive ancestor chains for all classes.
    fn compute_ancestors(data: &mut JavaTypeData) {
        let class_names: Vec<String> = data.classes.keys().cloned().collect();
        for name in &class_names {
            let ancestors = Self::collect_ancestors(data, name, &mut BTreeSet::new());
            data.ancestors.insert(name.clone(), ancestors);
        }
    }

    fn collect_ancestors(
        data: &JavaTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> Vec<String> {
        if !visited.insert(name.to_string()) {
            return Vec::new();
        }

        let mut result = Vec::new();
        if let Some(class) = data.classes.get(name) {
            if let Some(parent) = &class.extends {
                result.push(parent.clone());
                result.extend(Self::collect_ancestors(data, parent, visited));
            }
        }
        result
    }

    /// Compute which concrete types satisfy which interfaces.
    ///
    /// Java uses nominal typing: a class satisfies an interface only via
    /// explicit `implements` (directly or through inheritance chain).
    fn compute_satisfaction(data: &mut JavaTypeData) {
        // Resolve full interface method sets (flattening extends).
        let all_interfaces: Vec<String> = data.interfaces.keys().cloned().collect();

        for iface_name in &all_interfaces {
            let mut satisfying = BTreeSet::new();

            // Check classes.
            for (class_name, class) in &data.classes {
                if Self::class_implements(data, class, iface_name, &mut BTreeSet::new()) {
                    satisfying.insert(class_name.clone());
                }
            }

            // Check enums that implement the interface.
            for (enum_name, java_enum) in &data.enums {
                if java_enum.implements.contains(iface_name) {
                    satisfying.insert(enum_name.clone());
                }
            }

            data.satisfaction.insert(iface_name.clone(), satisfying);
        }

        // Also map parent classes: if B extends A and A implements I,
        // then B also satisfies I (already handled via class_implements).
    }

    /// Check if a class implements an interface (directly or via inheritance).
    ///
    /// `visited` prevents infinite recursion on malformed cyclic `extends`
    /// chains (invalid Java, but tree-sitter parses them).
    fn class_implements(
        data: &JavaTypeData,
        class: &JavaClass,
        iface_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if !visited.insert(class.name.clone()) {
            return false;
        }

        // Direct implementation.
        if class.implements.contains(&iface_name.to_string()) {
            return true;
        }

        // Through parent class.
        if let Some(parent_name) = &class.extends {
            if let Some(parent) = data.classes.get(parent_name) {
                if Self::class_implements(data, parent, iface_name, visited) {
                    return true;
                }
            }
        }

        // Check if any interface we implement extends the target interface.
        for impl_iface in &class.implements {
            if Self::interface_extends(data, impl_iface, iface_name, &mut BTreeSet::new()) {
                return true;
            }
        }

        false
    }

    /// Check if interface `child` extends `target` (directly or transitively).
    fn interface_extends(
        data: &JavaTypeData,
        child: &str,
        target: &str,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if child == target {
            return true;
        }
        if !visited.insert(child.to_string()) {
            return false;
        }
        if let Some(iface) = data.interfaces.get(child) {
            for parent in &iface.extends {
                if Self::interface_extends(data, parent, target, visited) {
                    return true;
                }
            }
        }
        false
    }

    /// Collect all methods for a class including inherited methods.
    #[allow(dead_code)]
    fn collect_all_methods<'a>(
        data: &'a JavaTypeData,
        class_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> Vec<&'a JavaMethod> {
        if !visited.insert(class_name.to_string()) {
            return Vec::new();
        }

        let mut methods = Vec::new();
        if let Some(class) = data.classes.get(class_name) {
            methods.extend(class.methods.iter());

            // Inherited methods from parent class (don't include overridden ones).
            if let Some(parent) = &class.extends {
                let parent_methods = Self::collect_all_methods(data, parent, visited);
                let own_names: BTreeSet<&str> =
                    class.methods.iter().map(|m| m.name.as_str()).collect();
                for pm in parent_methods {
                    if !own_names.contains(pm.name.as_str()) {
                        methods.push(pm);
                    }
                }
            }
        }
        methods
    }

    /// Resolve all interface methods, flattening `extends` chains.
    fn resolve_interface_methods(
        data: &JavaTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(name.to_string()) {
            return BTreeMap::new();
        }

        let mut methods = BTreeMap::new();
        if let Some(iface) = data.interfaces.get(name) {
            methods.extend(iface.methods.clone());
            for parent in &iface.extends {
                let parent_methods = Self::resolve_interface_methods(data, parent, visited);
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

impl TypeProvider for JavaTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        if self.data.classes.contains_key(expr) {
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
        self.field_layout_inner(type_name, &mut BTreeSet::new())
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // For interfaces: return implementing classes (transitive — includes
        // classes that inherit an implements declaration from a parent).
        if let Some(satisfying) = self.data.satisfaction.get(type_name) {
            return satisfying.iter().cloned().collect();
        }

        // For classes: return direct subclasses only.
        // TODO: consider making this transitive using the precomputed ancestors
        // map for consistency with the interface path. Currently resolve_dispatch
        // compensates by walking ancestors for class receivers, but the public
        // subtypes_of API returns asymmetric results for classes vs interfaces.
        let mut subtypes = Vec::new();
        for (class_name, class) in &self.data.classes {
            if let Some(parent) = &class.extends {
                if parent == type_name {
                    subtypes.push(class_name.clone());
                }
            }
        }
        subtypes
    }

    fn resolve_alias(&self, type_name: &str) -> String {
        // Java doesn't have type aliases.
        type_name.to_string()
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Java]
    }
}

// ---------------------------------------------------------------------------
// DispatchProvider implementation
// ---------------------------------------------------------------------------

impl DispatchProvider for JavaTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        // If receiver_type is a concrete class, resolve directly (may walk up
        // the hierarchy to find inherited methods).
        // TODO: when the resolved method is abstract (detectable by checking
        // tree-sitter modifiers for "abstract"), skip this path and fall
        // through to the subclass dispatch below so callers get concrete
        // override targets instead of the abstract declaration.
        if self.data.classes.contains_key(receiver_type)
            && !self.data.interfaces.contains_key(receiver_type)
        {
            if let Some(fid) = self.find_method_in_hierarchy(receiver_type, method) {
                return vec![fid];
            }
        }

        // If receiver_type is an interface, find all implementing classes
        // that have this method.
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
                if let Some(fid) = self.find_method_in_hierarchy(type_name, method) {
                    results.push(fid);
                }
            }
            return results;
        }

        // Also handle abstract class dispatch: if receiver_type is a class,
        // check subclasses.
        if self.data.classes.contains_key(receiver_type) {
            let mut results = Vec::new();
            // Find all subclasses that override this method.
            for (class_name, _) in &self.data.classes {
                if let Some(ancestors) = self.data.ancestors.get(class_name) {
                    if ancestors.contains(&receiver_type.to_string()) {
                        let should_include = if live_types.is_empty() {
                            true
                        } else {
                            live_types.contains(class_name)
                        };
                        if should_include {
                            if let Some(fid) = self.find_method_in_hierarchy(class_name, method) {
                                results.push(fid);
                            }
                        }
                    }
                }
            }
            if !results.is_empty() {
                return results;
            }
        }

        Vec::new()
    }
}

impl JavaTypeProvider {
    /// Find a method in a class (or enum) or its parent hierarchy.
    ///
    /// Uses `visited` to guard against cyclic `extends` chains in
    /// malformed files (invalid Java, but tree-sitter parses them).
    fn find_method_in_hierarchy(&self, type_name: &str, method: &str) -> Option<FunctionId> {
        self.find_method_in_hierarchy_inner(type_name, method, &mut BTreeSet::new())
    }

    fn find_method_in_hierarchy_inner(
        &self,
        type_name: &str,
        method: &str,
        visited: &mut BTreeSet<String>,
    ) -> Option<FunctionId> {
        if !visited.insert(type_name.to_string()) {
            return None;
        }

        // Check class methods.
        if let Some(class) = self.data.classes.get(type_name) {
            for m in &class.methods {
                if m.name == method {
                    return Some(FunctionId {
                        name: method.to_string(),
                        file: m.file.clone(),
                        start_line: m.start_line,
                        end_line: m.end_line,
                    });
                }
            }

            // Walk up the inheritance chain.
            if let Some(parent) = &class.extends {
                return self.find_method_in_hierarchy_inner(parent, method, visited);
            }
        }

        // Check enum methods.
        if let Some(java_enum) = self.data.enums.get(type_name) {
            for m in &java_enum.methods {
                if m.name == method {
                    return Some(FunctionId {
                        name: method.to_string(),
                        file: m.file.clone(),
                        start_line: m.start_line,
                        end_line: m.end_line,
                    });
                }
            }
        }

        None
    }

    /// `field_layout` with cycle protection for malformed extends chains.
    fn field_layout_inner(
        &self,
        type_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> Option<Vec<TypeFieldInfo>> {
        if let Some(class) = self.data.classes.get(type_name) {
            if !visited.insert(type_name.to_string()) {
                return Some(Vec::new());
            }

            let mut fields: Vec<TypeFieldInfo> = class
                .fields
                .iter()
                .map(|(name, type_str)| TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                })
                .collect();

            // Include methods as fields (same pattern as TS provider).
            for m in &class.methods {
                fields.push(TypeFieldInfo {
                    name: m.name.clone(),
                    type_str: m.signature.clone(),
                });
            }

            // Include inherited fields from parent class.
            if let Some(parent_name) = &class.extends {
                if let Some(parent_fields) = self.field_layout_inner(parent_name, visited) {
                    let own_names: BTreeSet<String> =
                        fields.iter().map(|f| f.name.clone()).collect();
                    for pf in parent_fields {
                        if !own_names.contains(&pf.name) {
                            fields.push(pf);
                        }
                    }
                }
            }

            return Some(fields);
        }

        if self.data.interfaces.contains_key(type_name) {
            // resolve_interface_methods already flattens the extends chain
            // with its own cycle protection, so we use it directly.
            let all_methods =
                Self::resolve_interface_methods(&self.data, type_name, &mut BTreeSet::new());
            let fields: Vec<TypeFieldInfo> = all_methods
                .into_iter()
                .map(|(name, sig)| TypeFieldInfo {
                    name,
                    type_str: sig,
                })
                .collect();
            return Some(fields);
        }

        // Enum field layout: methods declared in the enum body.
        if let Some(java_enum) = self.data.enums.get(type_name) {
            let fields: Vec<TypeFieldInfo> = java_enum
                .methods
                .iter()
                .map(|m| TypeFieldInfo {
                    name: m.name.clone(),
                    type_str: m.signature.clone(),
                })
                .collect();
            return Some(fields);
        }

        None
    }
}
