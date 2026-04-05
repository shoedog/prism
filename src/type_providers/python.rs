//! Python type provider — extracts class, function, and type alias definitions
//! from tree-sitter ASTs using PEP 484 type annotations.
//!
//! Python's type system is structural (duck typing). This provider implements
//! `TypeProvider` only — no `DispatchProvider`, since Python method dispatch
//! depends on runtime duck typing, not declared interfaces.
//!
//! **Limitation:** Coverage depends on annotation density in the repository.
//! Unannotated parameters and variables return `None` from `resolve_type`.

use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::type_provider::{ResolvedType, ResolvedTypeKind, TypeFieldInfo, TypeProvider};
use std::collections::BTreeMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Extracted type information
// ---------------------------------------------------------------------------

/// A Python class definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PythonClass {
    /// Class name.
    name: String,
    /// Base classes (from `class Foo(Base1, Base2):`).
    bases: Vec<String>,
    /// Annotated class attributes: name → type string.
    attributes: BTreeMap<String, String>,
    /// Method names → signature string.
    methods: BTreeMap<String, String>,
    /// Whether the class has a `@dataclass` decorator.
    is_dataclass: bool,
    /// Source file path.
    file: String,
    /// Line number of the class declaration.
    line: usize,
}

/// A top-level function definition with type annotations.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PythonFunction {
    /// Function name.
    name: String,
    /// Parameters: name → type annotation string. Unannotated params omitted.
    params: BTreeMap<String, String>,
    /// Return type annotation, if present.
    return_type: Option<String>,
    /// Source file path.
    file: String,
    /// Line number.
    line: usize,
}

// ---------------------------------------------------------------------------
// PythonTypeProvider
// ---------------------------------------------------------------------------

/// Inner data shared via Arc.
pub struct PythonTypeData {
    /// Class definitions by name.
    classes: BTreeMap<String, PythonClass>,
    /// Top-level function definitions by name.
    functions: BTreeMap<String, PythonFunction>,
    /// Type aliases: alias_name → target type string.
    aliases: BTreeMap<String, String>,
}

/// Python type provider extracting class, function, and type alias definitions
/// from tree-sitter ASTs using PEP 484 type annotations.
///
/// Uses `Arc<PythonTypeData>` for efficient cloning when registered in the
/// TypeRegistry.
#[derive(Clone)]
pub struct PythonTypeProvider {
    pub data: Arc<PythonTypeData>,
}

impl PythonTypeProvider {
    /// Build a PythonTypeProvider by scanning all Python parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = PythonTypeData {
            classes: BTreeMap::new(),
            functions: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if parsed.language != Language::Python {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        PythonTypeProvider {
            data: Arc::new(inner),
        }
    }

    // -----------------------------------------------------------------------
    // AST extraction
    // -----------------------------------------------------------------------

    fn extract_from_file(data: &mut PythonTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            Self::extract_top_level(data, &child, path, parsed);
        }
    }

    fn extract_top_level(
        data: &mut PythonTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        match node.kind() {
            "class_definition" => {
                Self::extract_class(data, node, path, parsed, false);
            }
            "function_definition" => {
                Self::extract_function(data, node, path, parsed);
            }
            "decorated_definition" => {
                Self::extract_decorated(data, node, path, parsed);
            }
            "expression_statement" => {
                // Type aliases: `MyType: TypeAlias = int` or `MyType = int`
                Self::try_extract_type_alias(data, node, parsed);
            }
            _ => {}
        }
    }

    /// Extract a decorated definition — unwraps decorators and delegates to
    /// class or function extraction.
    fn extract_decorated(
        data: &mut PythonTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let is_dataclass = Self::has_decorator(node, parsed, "dataclass");

        // The actual definition is the last named child (class or function).
        if let Some(definition) = node.child_by_field_name("definition") {
            match definition.kind() {
                "class_definition" => {
                    Self::extract_class(data, &definition, path, parsed, is_dataclass);
                }
                "function_definition" => {
                    Self::extract_function(data, &definition, path, parsed);
                }
                _ => {}
            }
        }
    }

    /// Check if a decorated_definition has a specific decorator name.
    fn has_decorator(node: &tree_sitter::Node, parsed: &ParsedFile, name: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                let text = parsed.node_text(&child).trim().to_string();
                // @dataclass or @dataclass(...) or @dataclasses.dataclass
                if text.starts_with('@') {
                    let decorator_name = text[1..]
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .split('.')
                        .last()
                        .unwrap_or("");
                    if decorator_name == name {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Extract a class definition.
    fn extract_class(
        data: &mut PythonTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
        is_dataclass: bool,
    ) {
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).trim().to_string(),
            None => return,
        };
        if name.is_empty() {
            return;
        }

        let line = node.start_position().row + 1;
        let bases = Self::extract_bases(node, parsed);
        let (attributes, methods) = Self::extract_class_body(node, parsed);

        data.classes.insert(
            name.clone(),
            PythonClass {
                name,
                bases,
                attributes,
                methods,
                is_dataclass,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract base classes from `class Foo(Base1, Base2):`.
    fn extract_bases(node: &tree_sitter::Node, parsed: &ParsedFile) -> Vec<String> {
        let mut bases = Vec::new();

        let superclasses = match node.child_by_field_name("superclasses") {
            Some(s) => s,
            None => return bases,
        };

        // superclasses is an argument_list node containing identifiers.
        let mut cursor = superclasses.walk();
        for child in superclasses.children(&mut cursor) {
            match child.kind() {
                "identifier" | "attribute" => {
                    let text = parsed.node_text(&child).trim().to_string();
                    if !text.is_empty() {
                        bases.push(text);
                    }
                }
                // Handle generic bases like Protocol[T] — extract the base name.
                "subscript" => {
                    if let Some(value) = child.child_by_field_name("value") {
                        let text = parsed.node_text(&value).trim().to_string();
                        if !text.is_empty() {
                            bases.push(text);
                        }
                    }
                }
                _ => {}
            }
        }

        bases
    }

    /// Extract annotated attributes and methods from a class body.
    fn extract_class_body(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
        let mut attributes = BTreeMap::new();
        let mut methods = BTreeMap::new();

        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return (attributes, methods),
        };

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "expression_statement" => {
                    // Annotated attribute: `name: str`
                    Self::try_extract_annotation(&child, parsed, &mut attributes);
                }
                "function_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let method_name = parsed.node_text(&name_node).trim().to_string();
                        if !method_name.is_empty() {
                            let sig = Self::build_fn_signature(&child, parsed);
                            methods.insert(method_name, sig);
                        }
                    }
                }
                "decorated_definition" => {
                    // Decorated method (e.g., @staticmethod, @classmethod, @property).
                    if let Some(definition) = child.child_by_field_name("definition") {
                        if definition.kind() == "function_definition" {
                            if let Some(name_node) = definition.child_by_field_name("name") {
                                let method_name = parsed.node_text(&name_node).trim().to_string();
                                if !method_name.is_empty() {
                                    let sig = Self::build_fn_signature(&definition, parsed);
                                    methods.insert(method_name, sig);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        (attributes, methods)
    }

    /// Try to extract a type annotation from an expression_statement.
    /// Handles: `name: Type` and `name: Type = value`.
    fn try_extract_annotation(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        attrs: &mut BTreeMap<String, String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type" {
                // This is a standalone type annotation: `name: Type`
                // The previous sibling should be the identifier.
                // tree-sitter-python structures this as:
                //   expression_statement -> (type (identifier) (type))
                // Actually, it's more nuanced. Let's handle both patterns.
                continue;
            }
            if child.kind() == "assignment" {
                // `name: Type = value` — the assignment node has a `type` field
                // Actually this is handled below.
            }
        }

        // Look for direct annotation pattern in the expression_statement.
        // tree-sitter-python: expression_statement containing an `assignment`
        // with type annotation, or a bare `type` annotation.
        let mut cursor2 = node.walk();
        for child in node.children(&mut cursor2) {
            if child.kind() == "assignment" {
                // `x: int = 5` — assignment with optional type
                if let Some(type_node) = child.child_by_field_name("type") {
                    if let Some(left) = child.child_by_field_name("left") {
                        let name = parsed.node_text(&left).trim().to_string();
                        let type_str = parsed.node_text(&type_node).trim().to_string();
                        if !name.is_empty() && !type_str.is_empty() {
                            attrs.insert(name, type_str);
                        }
                    }
                }
            }
        }
    }

    /// Try to extract a type alias from a top-level expression_statement.
    /// Handles: `MyType = int` or `MyType: TypeAlias = int`.
    fn try_extract_type_alias(
        data: &mut PythonTypeData,
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "assignment" {
                // Check if this looks like a type alias:
                // - `MyType: TypeAlias = SomeType`
                // - `MyType = SomeType` where SomeType starts with uppercase
                let left = match child.child_by_field_name("left") {
                    Some(l) => l,
                    None => continue,
                };
                let right = match child.child_by_field_name("right") {
                    Some(r) => r,
                    None => continue,
                };

                let name = parsed.node_text(&left).trim().to_string();
                let target = parsed.node_text(&right).trim().to_string();

                if name.is_empty() || target.is_empty() {
                    continue;
                }

                // Check for explicit TypeAlias annotation.
                if let Some(type_node) = child.child_by_field_name("type") {
                    let annotation = parsed.node_text(&type_node).trim().to_string();
                    if annotation == "TypeAlias" {
                        data.aliases.insert(name, target);
                        return;
                    }
                }

                // Heuristic: `Name = OtherName` where both start uppercase
                // and the RHS is a simple type expression (identifier, subscript,
                // or attribute).
                if name.starts_with(|c: char| c.is_ascii_uppercase())
                    && matches!(right.kind(), "identifier" | "subscript" | "attribute")
                {
                    let first_char_upper = target.starts_with(|c: char| c.is_ascii_uppercase());
                    if first_char_upper {
                        data.aliases.insert(name, target);
                    }
                }
            }
        }
    }

    /// Build a function signature string from parameters and return type.
    fn build_fn_signature(node: &tree_sitter::Node, parsed: &ParsedFile) -> String {
        let mut parts = Vec::new();

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(parsed.node_text(&params).trim().to_string());
        }

        if let Some(ret) = node.child_by_field_name("return_type") {
            let ret_text = parsed.node_text(&ret).trim().to_string();
            parts.push(format!("-> {}", ret_text));
        }

        parts.join(" ")
    }

    /// Extract a top-level function definition.
    fn extract_function(
        data: &mut PythonTypeData,
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
        let params = Self::extract_typed_params(node, parsed);
        let return_type = node
            .child_by_field_name("return_type")
            .map(|rt| parsed.node_text(&rt).trim().to_string())
            .filter(|s| !s.is_empty());

        data.functions.insert(
            name.clone(),
            PythonFunction {
                name,
                params,
                return_type,
                file: path.to_string(),
                line,
            },
        );
    }

    /// Extract typed parameters from a function's parameter list.
    fn extract_typed_params(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();

        let parameters = match node.child_by_field_name("parameters") {
            Some(p) => p,
            None => return params,
        };

        let mut cursor = parameters.walk();
        for child in parameters.children(&mut cursor) {
            match child.kind() {
                "typed_parameter" | "typed_default_parameter" => {
                    let param_name = child
                        .child_by_field_name("name")
                        .map(|n| parsed.node_text(&n).trim().to_string());
                    let param_type = child
                        .child_by_field_name("type")
                        .map(|t| parsed.node_text(&t).trim().to_string());

                    if let (Some(name), Some(type_str)) = (param_name, param_type) {
                        if !name.is_empty()
                            && !type_str.is_empty()
                            && name != "self"
                            && name != "cls"
                        {
                            params.insert(name, type_str);
                        }
                    }
                }
                _ => {}
            }
        }

        params
    }

    /// Collect all methods from a class and its base classes (MRO approximation).
    /// Uses visited set for cycle protection.
    fn collect_methods_with_bases(
        data: &PythonTypeData,
        class_name: &str,
        visited: &mut std::collections::BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(class_name.to_string()) {
            return BTreeMap::new();
        }

        let mut methods = BTreeMap::new();

        if let Some(cls) = data.classes.get(class_name) {
            // Base class methods first (overridden by child).
            for base in &cls.bases {
                let base_methods = Self::collect_methods_with_bases(data, base, visited);
                methods.extend(base_methods);
            }
            // Child methods override base.
            methods.extend(cls.methods.clone());
        }

        methods
    }

    /// Collect all attributes from a class and its base classes.
    fn collect_attrs_with_bases(
        data: &PythonTypeData,
        class_name: &str,
        visited: &mut std::collections::BTreeSet<String>,
    ) -> BTreeMap<String, String> {
        if !visited.insert(class_name.to_string()) {
            return BTreeMap::new();
        }

        let mut attrs = BTreeMap::new();

        if let Some(cls) = data.classes.get(class_name) {
            for base in &cls.bases {
                let base_attrs = Self::collect_attrs_with_bases(data, base, visited);
                attrs.extend(base_attrs);
            }
            attrs.extend(cls.attributes.clone());
        }

        attrs
    }
}

// ---------------------------------------------------------------------------
// TypeProvider implementation
// ---------------------------------------------------------------------------

impl TypeProvider for PythonTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
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
        None
    }

    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>> {
        if let Some(_cls) = self.data.classes.get(type_name) {
            let mut fields = Vec::new();

            // Annotated attributes (including inherited).
            let all_attrs = Self::collect_attrs_with_bases(
                &self.data,
                type_name,
                &mut std::collections::BTreeSet::new(),
            );
            for (name, type_str) in &all_attrs {
                fields.push(TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                });
            }

            // Methods (including inherited), excluding dunder methods.
            let all_methods = Self::collect_methods_with_bases(
                &self.data,
                type_name,
                &mut std::collections::BTreeSet::new(),
            );
            for (name, sig) in &all_methods {
                if !name.starts_with("__") {
                    fields.push(TypeFieldInfo {
                        name: name.clone(),
                        type_str: sig.clone(),
                    });
                }
            }

            return Some(fields);
        }

        None
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // Find all classes that list type_name as a direct base.
        self.data
            .classes
            .iter()
            .filter(|(_, cls)| cls.bases.contains(&type_name.to_string()))
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn resolve_alias(&self, type_name: &str) -> String {
        self.data
            .aliases
            .get(type_name)
            .cloned()
            .unwrap_or_else(|| type_name.to_string())
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Python]
    }
}
