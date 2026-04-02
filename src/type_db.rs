//! Type Database — optional C/C++ type enrichment from `compile_commands.json` + clang.
//!
//! Extracts struct/class/union definitions, field types, typedefs, and class
//! hierarchy from clang's JSON AST dump. This information annotates CPG Variable
//! nodes to enable:
//!
//! - **Precise whole-struct detection:** `memcpy(&dev, ...)` with size matching struct size
//! - **Complete field enumeration:** all fields from struct definition, not just accessed ones
//! - **Typedef resolution:** `my_handle_t` → `struct device *`
//! - **Virtual dispatch:** class hierarchy analysis for C++ polymorphic calls
//! - **Union field overlap:** fields in a union alias each other
//!
//! This is an optional enrichment pass (CPG Phase 5). The CPG builds correctly
//! without it. Type information is only available for C/C++ files that appear in
//! `compile_commands.json`.
//!
//! See `docs/cpg-architecture.md` §Layer 3 for the full design.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Type representation
// ---------------------------------------------------------------------------

/// A field within a struct, class, or union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    /// Field name.
    pub name: String,
    /// Type as a string (e.g., "int", "struct device *", "std::string").
    pub type_str: String,
    /// Byte offset within the struct (if known).
    pub offset: Option<usize>,
}

/// A struct, class, or union definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordInfo {
    /// Fully qualified name (e.g., "device", "ns::MyClass").
    pub name: String,
    /// Kind: struct, class, or union.
    pub kind: RecordKind,
    /// Ordered list of fields.
    pub fields: Vec<FieldInfo>,
    /// Base classes (C++ inheritance). Each entry is a base class name.
    pub bases: Vec<String>,
    /// Virtual methods declared in this record (name → return type).
    pub virtual_methods: BTreeMap<String, String>,
    /// Total size in bytes (if known from clang).
    pub size: Option<usize>,
    /// The file where this record is defined.
    pub file: String,
}

/// The kind of record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Struct,
    Class,
    Union,
}

/// A typedef or type alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedefInfo {
    /// The alias name (e.g., "my_handle_t").
    pub name: String,
    /// The underlying type (e.g., "struct device *").
    pub underlying: String,
}

/// Resolved type information for a variable or expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    /// The canonical type name (typedefs resolved).
    pub canonical: String,
    /// If this is a record type, its definition.
    pub record: Option<String>,
    /// Whether this is a pointer to the record type.
    pub is_pointer: bool,
}

// ---------------------------------------------------------------------------
// compile_commands.json parsing
// ---------------------------------------------------------------------------

/// A single entry from `compile_commands.json`.
#[derive(Debug, Deserialize)]
struct CompileCommand {
    /// The working directory for the compilation.
    directory: String,
    /// The source file being compiled.
    file: String,
    /// The compile command as a single string (optional).
    command: Option<String>,
    /// The compile command as an argument array (optional).
    arguments: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// clang JSON AST node types (subset we care about)
// ---------------------------------------------------------------------------

/// A node in clang's JSON AST dump.
#[derive(Debug, Deserialize)]
struct ClangAstNode {
    /// Node kind: "RecordDecl", "FieldDecl", "TypedefDecl", "CXXMethodDecl", etc.
    kind: String,
    /// Node name (for named declarations).
    name: Option<String>,
    /// Inner nodes (children).
    inner: Option<Vec<ClangAstNode>>,
    /// For RecordDecl: "struct", "class", "union".
    #[serde(rename = "tagUsed")]
    tag_used: Option<String>,
    /// Type information.
    #[serde(rename = "type")]
    type_info: Option<ClangTypeInfo>,
    /// For CXXMethodDecl: whether it's virtual.
    #[serde(rename = "virtual")]
    is_virtual: Option<bool>,
    /// For CXXRecordDecl: base classes.
    bases: Option<Vec<ClangBase>>,
    /// Whether this is a definition (vs just a declaration).
    #[serde(rename = "completeDefinition")]
    complete_definition: Option<bool>,
    /// Source location (reserved for future use).
    #[allow(dead_code)]
    loc: Option<ClangLoc>,
    /// Source range (reserved for future use).
    #[allow(dead_code)]
    range: Option<ClangRange>,
}

#[derive(Debug, Deserialize)]
struct ClangTypeInfo {
    #[serde(rename = "qualType")]
    qual_type: Option<String>,
    #[serde(rename = "desugaredQualType")]
    desugared_qual_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClangBase {
    /// The base class type.
    #[serde(rename = "type")]
    type_info: Option<ClangTypeInfo>,
    /// Whether it's virtual inheritance (reserved for future use).
    #[serde(rename = "isVirtual")]
    #[allow(dead_code)]
    is_virtual: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClangLoc {
    file: Option<String>,
    line: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClangRange {
    begin: Option<ClangLoc>,
}

// ---------------------------------------------------------------------------
// TypeDatabase
// ---------------------------------------------------------------------------

/// Database of C/C++ type information extracted from clang.
///
/// Built from `compile_commands.json` by running clang on each translation unit
/// and parsing the JSON AST dump.
#[derive(Debug, Default, Clone)]
pub struct TypeDatabase {
    /// Record (struct/class/union) definitions by name.
    pub records: BTreeMap<String, RecordInfo>,
    /// Typedef/alias mappings: alias name → TypedefInfo.
    pub typedefs: BTreeMap<String, TypedefInfo>,
    /// Class hierarchy: class name → list of direct base class names.
    pub class_hierarchy: BTreeMap<String, Vec<String>>,
}

impl TypeDatabase {
    /// Build a TypeDatabase from a `compile_commands.json` file.
    ///
    /// For each translation unit in the compile commands:
    /// 1. Run `clang -fsyntax-only -Xclang -ast-dump=json` with the same flags
    /// 2. Parse the JSON AST dump
    /// 3. Extract struct/class/union definitions, typedefs, and class hierarchy
    ///
    /// Files not in `filter_files` (if provided) are skipped for efficiency.
    pub fn from_compile_commands(
        compile_commands_path: &Path,
        filter_files: Option<&[&str]>,
    ) -> Result<Self> {
        let content = std::fs::read_to_string(compile_commands_path)
            .with_context(|| format!("reading {}", compile_commands_path.display()))?;

        let commands: Vec<CompileCommand> =
            serde_json::from_str(&content).with_context(|| "parsing compile_commands.json")?;

        let mut db = TypeDatabase::default();

        for cmd in &commands {
            // Skip files not in our filter set
            if let Some(filter) = filter_files {
                let file_name = Path::new(&cmd.file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let matches = filter
                    .iter()
                    .any(|f| cmd.file.ends_with(f) || file_name == *f);
                if !matches {
                    continue;
                }
            }

            // Run clang to get JSON AST
            match run_clang_ast_dump(cmd) {
                Ok(ast_json) => {
                    if let Err(e) = db.extract_from_ast(&ast_json, &cmd.file) {
                        eprintln!("warning: type extraction failed for {}: {}", cmd.file, e);
                    }
                }
                Err(e) => {
                    eprintln!("warning: clang AST dump failed for {}: {}", cmd.file, e);
                }
            }
        }

        // Build class hierarchy from base class info
        for record in db.records.values() {
            if !record.bases.is_empty() {
                db.class_hierarchy
                    .insert(record.name.clone(), record.bases.clone());
            }
        }

        Ok(db)
    }

    /// Extract type information from a clang JSON AST dump.
    pub fn extract_from_ast(&mut self, json: &str, file: &str) -> Result<()> {
        let root: ClangAstNode =
            serde_json::from_str(json).with_context(|| "parsing clang AST JSON")?;

        self.visit_node(&root, file);
        Ok(())
    }

    /// Recursively visit AST nodes to extract type definitions.
    fn visit_node(&mut self, node: &ClangAstNode, file: &str) {
        match node.kind.as_str() {
            "RecordDecl" | "CXXRecordDecl" => {
                // Only process complete definitions (not forward declarations)
                if node.complete_definition.unwrap_or(false) {
                    if let Some(name) = &node.name {
                        if !name.is_empty() {
                            self.extract_record(node, name, file);
                        }
                    }
                }
            }
            "TypedefDecl" | "TypeAliasDecl" => {
                if let Some(name) = &node.name {
                    if let Some(type_info) = &node.type_info {
                        if let Some(qual_type) = &type_info.qual_type {
                            let underlying = type_info
                                .desugared_qual_type
                                .as_deref()
                                .unwrap_or(qual_type)
                                .to_string();
                            self.typedefs.insert(
                                name.clone(),
                                TypedefInfo {
                                    name: name.clone(),
                                    underlying,
                                },
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        // Recurse into children
        if let Some(inner) = &node.inner {
            for child in inner {
                self.visit_node(child, file);
            }
        }
    }

    /// Extract a record (struct/class/union) definition from a clang AST node.
    fn extract_record(&mut self, node: &ClangAstNode, name: &str, file: &str) {
        let kind = match node.tag_used.as_deref() {
            Some("struct") => RecordKind::Struct,
            Some("class") => RecordKind::Class,
            Some("union") => RecordKind::Union,
            _ => {
                // CXXRecordDecl without tagUsed defaults to class
                if node.kind == "CXXRecordDecl" {
                    RecordKind::Class
                } else {
                    RecordKind::Struct
                }
            }
        };

        let mut fields = Vec::new();
        let mut virtual_methods = BTreeMap::new();
        let mut bases = Vec::new();

        // Extract base classes
        if let Some(base_list) = &node.bases {
            for base in base_list {
                if let Some(type_info) = &base.type_info {
                    if let Some(qual_type) = &type_info.qual_type {
                        // Clean up "class Foo" or "struct Bar" prefix
                        let base_name = qual_type
                            .trim_start_matches("class ")
                            .trim_start_matches("struct ")
                            .to_string();
                        bases.push(base_name);
                    }
                }
            }
        }

        // Extract fields and virtual methods from inner nodes
        if let Some(inner) = &node.inner {
            for child in inner {
                match child.kind.as_str() {
                    "FieldDecl" => {
                        if let Some(field_name) = &child.name {
                            let type_str = child
                                .type_info
                                .as_ref()
                                .and_then(|t| t.qual_type.as_deref())
                                .unwrap_or("unknown")
                                .to_string();
                            fields.push(FieldInfo {
                                name: field_name.clone(),
                                type_str,
                                offset: None,
                            });
                        }
                    }
                    "CXXMethodDecl" => {
                        if child.is_virtual.unwrap_or(false) {
                            if let Some(method_name) = &child.name {
                                let return_type = child
                                    .type_info
                                    .as_ref()
                                    .and_then(|t| t.qual_type.as_deref())
                                    .unwrap_or("void")
                                    .to_string();
                                virtual_methods.insert(method_name.clone(), return_type);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let record = RecordInfo {
            name: name.to_string(),
            kind,
            fields,
            bases,
            virtual_methods,
            size: None,
            file: file.to_string(),
        };

        self.records.insert(name.to_string(), record);
    }

    // -----------------------------------------------------------------------
    // Query methods
    // -----------------------------------------------------------------------

    /// Resolve a typedef chain to its canonical underlying type.
    ///
    /// Follows typedef chains up to a depth limit to prevent infinite loops.
    pub fn resolve_typedef(&self, name: &str) -> String {
        let mut current = name.to_string();
        for _ in 0..10 {
            match self.typedefs.get(&current) {
                Some(td) => current = td.underlying.clone(),
                None => break,
            }
        }
        current
    }

    /// Get the record (struct/class/union) for a type name, resolving typedefs.
    pub fn resolve_record(&self, type_name: &str) -> Option<&RecordInfo> {
        // Try direct lookup first
        if let Some(record) = self.records.get(type_name) {
            return Some(record);
        }

        // Resolve typedefs
        let resolved = self.resolve_typedef(type_name);

        // Strip pointer/reference qualifiers: "struct device *" → "device"
        let stripped = strip_type_qualifiers(&resolved);

        self.records.get(stripped)
    }

    /// Get all fields of a record type (struct/class/union), including
    /// inherited fields from base classes.
    pub fn all_fields(&self, record_name: &str) -> Vec<FieldInfo> {
        let mut fields = Vec::new();
        let mut visited = std::collections::BTreeSet::new();
        self.collect_fields(record_name, &mut fields, &mut visited);
        fields
    }

    fn collect_fields(
        &self,
        name: &str,
        fields: &mut Vec<FieldInfo>,
        visited: &mut std::collections::BTreeSet<String>,
    ) {
        if !visited.insert(name.to_string()) {
            return;
        }

        if let Some(record) = self.records.get(name) {
            // First add inherited fields from base classes
            for base in &record.bases {
                self.collect_fields(base, fields, visited);
            }
            // Then add own fields
            fields.extend(record.fields.iter().cloned());
        }
    }

    /// Get all virtual dispatch targets for a method call on a given class.
    ///
    /// Returns all classes in the hierarchy that override this method,
    /// implementing Class Hierarchy Analysis (CHA).
    pub fn virtual_dispatch_targets(&self, class_name: &str, method: &str) -> Vec<String> {
        let mut targets = Vec::new();

        // Check the class itself
        if let Some(record) = self.records.get(class_name) {
            if record.virtual_methods.contains_key(method) {
                targets.push(class_name.to_string());
            }
        }

        // Check all subclasses (classes that have `class_name` as a base)
        for (name, record) in &self.records {
            if self.is_subclass_of(name, class_name) {
                if record.virtual_methods.contains_key(method) {
                    targets.push(name.clone());
                }
            }
        }

        targets
    }

    /// Rapid Type Analysis (RTA) refinement of virtual dispatch.
    ///
    /// Like `virtual_dispatch_targets` but filters results to only include
    /// classes that are actually instantiated in the codebase. If no live
    /// classes are provided (empty set), falls back to full CHA.
    pub fn virtual_dispatch_targets_rta(
        &self,
        class_name: &str,
        method: &str,
        live_classes: &std::collections::BTreeSet<String>,
    ) -> Vec<String> {
        let cha_targets = self.virtual_dispatch_targets(class_name, method);

        // If no live class info, fall back to CHA
        if live_classes.is_empty() {
            return cha_targets;
        }

        // Filter to only instantiated classes
        let filtered: Vec<String> = cha_targets
            .into_iter()
            .filter(|t| live_classes.contains(t))
            .collect();

        // If filtering removes all targets, fall back to CHA to avoid
        // false negatives (the instantiation might be in code we didn't scan)
        if filtered.is_empty() {
            return self.virtual_dispatch_targets(class_name, method);
        }

        filtered
    }

    /// Scan parsed files for class instantiation expressions.
    ///
    /// Detects:
    /// - `new ClassName(...)` — heap allocation
    /// - `make_unique<ClassName>(...)` / `make_shared<ClassName>(...)` — smart pointers
    /// - `ClassName varname;` or `ClassName varname(...)` — stack allocation
    ///
    /// Returns a set of class names that are known to be instantiated.
    pub fn collect_live_classes(
        files: &BTreeMap<String, crate::ast::ParsedFile>,
    ) -> std::collections::BTreeSet<String> {
        let mut live = std::collections::BTreeSet::new();

        for parsed in files.values() {
            if !matches!(parsed.language, crate::languages::Language::Cpp) {
                continue;
            }
            Self::scan_instantiations(parsed.tree.root_node(), parsed, &mut live);
        }

        live
    }

    /// Recursively scan a tree-sitter node for instantiation expressions.
    fn scan_instantiations(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
        live: &mut std::collections::BTreeSet<String>,
    ) {
        match node.kind() {
            // new ClassName(...) — new_expression
            "new_expression" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    let type_name = parsed.node_text(&type_node).to_string().trim().to_string();
                    if !type_name.is_empty() {
                        live.insert(type_name);
                    }
                }
            }
            // make_unique<ClassName>(...), make_shared<ClassName>(...)
            "call_expression" => {
                if let Some(func) = node.child_by_field_name("function") {
                    if func.kind() == "template_function" {
                        let func_text = parsed.node_text(&func).to_string();
                        if func_text.starts_with("make_unique")
                            || func_text.starts_with("make_shared")
                            || func_text.starts_with("std::make_unique")
                            || func_text.starts_with("std::make_shared")
                        {
                            // Extract template argument: make_unique<Circle>(...)
                            let mut cursor = func.walk();
                            for child in func.children(&mut cursor) {
                                if child.kind() == "template_argument_list" {
                                    let mut arg_cursor = child.walk();
                                    for arg in child.children(&mut arg_cursor) {
                                        if arg.kind() == "type_descriptor"
                                            || arg.kind() == "type_identifier"
                                        {
                                            let name = parsed
                                                .node_text(&arg)
                                                .to_string()
                                                .trim()
                                                .to_string();
                                            if !name.is_empty() {
                                                live.insert(name);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Stack allocation: ClassName varname; or ClassName varname(...);
            // This is a declaration with a type_identifier matching a known class
            "declaration" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    if type_node.kind() == "type_identifier"
                        || type_node.kind() == "qualified_identifier"
                    {
                        let type_name = parsed.node_text(&type_node).to_string().trim().to_string();
                        if !type_name.is_empty() {
                            live.insert(type_name);
                        }
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::scan_instantiations(child, parsed, live);
        }
    }

    /// Check if `derived` is a (transitive) subclass of `base`.
    pub fn is_subclass_of(&self, derived: &str, base: &str) -> bool {
        if derived == base {
            return false;
        }
        let mut visited = std::collections::BTreeSet::new();
        self.is_subclass_of_inner(derived, base, &mut visited)
    }

    fn is_subclass_of_inner(
        &self,
        current: &str,
        target: &str,
        visited: &mut std::collections::BTreeSet<String>,
    ) -> bool {
        if !visited.insert(current.to_string()) {
            return false;
        }

        if let Some(bases) = self.class_hierarchy.get(current) {
            for b in bases {
                if b == target || self.is_subclass_of_inner(b, target, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a record type is a union (fields alias each other).
    pub fn is_union(&self, type_name: &str) -> bool {
        self.resolve_record(type_name)
            .is_some_and(|r| r.kind == RecordKind::Union)
    }

    /// Resolve a variable's type given its name and the function context.
    ///
    /// This is a convenience method that looks up the type of a field access.
    /// For example, given a record "device" and field "config", returns the
    /// type of the "config" field.
    pub fn field_type(&self, record_name: &str, field_name: &str) -> Option<String> {
        let record = self.resolve_record(record_name)?;
        record
            .fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| f.type_str.clone())
    }

    /// Check whether a type is a pointer type.
    pub fn is_pointer_type(type_str: &str) -> bool {
        type_str.trim().ends_with('*')
    }

    // -----------------------------------------------------------------------
    // Tree-sitter fallback extraction
    // -----------------------------------------------------------------------

    /// Build a TypeDatabase from parsed files using tree-sitter ASTs.
    ///
    /// This is a zero-dependency fallback for when `--compile-commands` is not
    /// provided. Extracts struct/class/union definitions, fields, typedefs,
    /// base classes, and virtual methods directly from tree-sitter parse trees.
    ///
    /// Only processes C and C++ files. Returns an empty database for other languages.
    ///
    /// Compared to clang-based extraction, this provides:
    /// - Field names (yes), field types (approximate from declaration text)
    /// - Base classes (yes), virtual methods (yes)
    /// - No typedef resolution across `#include` boundaries
    /// - No macro expansion — heavily macro'd code may have ERROR nodes
    pub fn from_parsed_files(files: &BTreeMap<String, crate::ast::ParsedFile>) -> Self {
        let mut db = TypeDatabase::default();

        for (file_path, parsed) in files {
            // Only extract types from C/C++ files
            if !matches!(
                parsed.language,
                crate::languages::Language::C | crate::languages::Language::Cpp
            ) {
                continue;
            }

            db.extract_from_tree_sitter(parsed, file_path);
        }

        // Build class hierarchy from base class info
        for record in db.records.values() {
            if !record.bases.is_empty() {
                db.class_hierarchy
                    .insert(record.name.clone(), record.bases.clone());
            }
        }

        db
    }

    /// Extract type definitions from a single parsed file's tree-sitter AST.
    fn extract_from_tree_sitter(&mut self, parsed: &crate::ast::ParsedFile, file_path: &str) {
        let root = parsed.tree.root_node();
        self.visit_ts_node(root, parsed, file_path);
    }

    /// Recursively visit tree-sitter nodes to find type definitions.
    fn visit_ts_node(
        &mut self,
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
        file_path: &str,
    ) {
        match node.kind() {
            // C struct/union and C++ struct/class with body
            "struct_specifier" | "union_specifier" | "class_specifier" => {
                self.extract_ts_record(node, parsed, file_path);
            }
            // C/C++ typedef: `typedef struct device dev_t;`
            "type_definition" => {
                self.extract_ts_typedef(node, parsed);
            }
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_ts_node(child, parsed, file_path);
        }
    }

    /// Extract a struct/class/union definition from a tree-sitter node.
    fn extract_ts_record(
        &mut self,
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
        file_path: &str,
    ) {
        // Must have a body (field_declaration_list) — skip forward declarations
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return,
        };

        // Get the record name
        let name = match node.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).to_string(),
            None => return, // Anonymous struct — skip
        };

        if name.is_empty() {
            return;
        }

        let kind = match node.kind() {
            "struct_specifier" => RecordKind::Struct,
            "class_specifier" => RecordKind::Class,
            "union_specifier" => RecordKind::Union,
            _ => RecordKind::Struct,
        };

        let mut fields = Vec::new();
        let mut virtual_methods = BTreeMap::new();
        let mut bases = Vec::new();

        // Extract base classes (C++ only): base_class_clause
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "base_class_clause" {
                Self::extract_ts_bases(child, parsed, &mut bases);
            }
        }

        // Extract fields and virtual methods from body
        let mut body_cursor = body.walk();
        for child in body.children(&mut body_cursor) {
            match child.kind() {
                "field_declaration" => {
                    // C++ virtual methods appear as field_declaration with `virtual` child
                    if Self::has_virtual_specifier(child, parsed) {
                        if let Some(method_name) =
                            Self::extract_ts_virtual_method_name(child, parsed)
                        {
                            let type_text = parsed.node_text(&child).to_string();
                            virtual_methods.insert(method_name, type_text);
                        }
                    } else if let Some(field) = Self::extract_ts_field(child, parsed) {
                        fields.push(field);
                    }
                }
                // C++ access specifiers
                "access_specifier" => {}
                // C++ method declarations/definitions outside field_declaration
                "declaration" | "function_definition" => {
                    if Self::has_virtual_specifier(child, parsed) {
                        if let Some(method_name) = Self::extract_ts_function_name(child, parsed) {
                            let type_text = parsed.node_text(&child).to_string();
                            virtual_methods.insert(method_name, type_text);
                        }
                    }
                }
                _ => {}
            }
        }

        let record = RecordInfo {
            name: name.clone(),
            kind,
            fields,
            bases,
            virtual_methods,
            size: None,
            file: file_path.to_string(),
        };

        self.records.insert(name, record);
    }

    /// Extract a field from a field_declaration node.
    fn extract_ts_field(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
    ) -> Option<FieldInfo> {
        // field_declaration has a type and a declarator
        // Get the declarator name — could be nested in pointer_declarator etc.
        let declarator = node.child_by_field_name("declarator")?;
        let name = Self::find_declarator_name(declarator, parsed)?;

        // Get the type — everything before the declarator
        let type_node = node.child_by_field_name("type")?;
        let type_str = parsed.node_text(&type_node).to_string().trim().to_string();

        // Check if declarator is a pointer
        let decl_text = parsed.node_text(&declarator).to_string();
        let full_type = if decl_text.starts_with('*') {
            format!("{} *", type_str)
        } else {
            type_str
        };

        Some(FieldInfo {
            name,
            type_str: full_type,
            offset: None,
        })
    }

    /// Find the identifier name in a (possibly nested) declarator.
    fn find_declarator_name(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
    ) -> Option<String> {
        match node.kind() {
            "identifier" | "field_identifier" | "type_identifier" => {
                Some(parsed.node_text(&node).to_string())
            }
            "pointer_declarator" | "array_declarator" | "parenthesized_declarator" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(name) = Self::find_declarator_name(child, parsed) {
                        return Some(name);
                    }
                }
                None
            }
            "function_declarator" => {
                // Function pointer field: void (*callback)(void *)
                if let Some(inner) = node.child_by_field_name("declarator") {
                    return Self::find_declarator_name(inner, parsed);
                }
                None
            }
            _ => {
                // Try declarator field
                if let Some(inner) = node.child_by_field_name("declarator") {
                    return Self::find_declarator_name(inner, parsed);
                }
                None
            }
        }
    }

    /// Extract base class names from a base_class_clause.
    fn extract_ts_bases(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
        bases: &mut Vec<String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Each base specifier has a type name
            if child.kind() == "type_identifier" || child.kind() == "qualified_identifier" {
                let base_name = parsed.node_text(&child).to_string();
                if !base_name.is_empty() {
                    bases.push(base_name);
                }
            }
            // Could also be nested in access specifier like "public Shape"
            if child.kind() == "access_specifier" || child.kind() == "base_specifier" {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "type_identifier" || inner.kind() == "qualified_identifier" {
                        let base_name = parsed.node_text(&inner).to_string();
                        if !base_name.is_empty() {
                            bases.push(base_name);
                        }
                    }
                }
            }
        }
    }

    /// Check if a declaration/function_definition has a `virtual` specifier.
    fn has_virtual_specifier(node: tree_sitter::Node<'_>, parsed: &crate::ast::ParsedFile) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "virtual" {
                return true;
            }
            // Also check for "virtual" as text in specifiers
            if child.kind() == "virtual_function_specifier" || child.kind() == "virtual_specifier" {
                return true;
            }
            // Check if child text is "virtual"
            if parsed.node_text(&child) == "virtual" {
                return true;
            }
        }
        false
    }

    /// Extract virtual method name from a field_declaration with `virtual`.
    ///
    /// In tree-sitter C++, `virtual void draw() = 0;` is a `field_declaration`
    /// with a `function_declarator` child containing `field_identifier "draw"`.
    fn extract_ts_virtual_method_name(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
    ) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_declarator" {
                // function_declarator → field_identifier
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "field_identifier" || inner.kind() == "identifier" {
                        return Some(parsed.node_text(&inner).to_string());
                    }
                }
                // Also try declarator field
                if let Some(decl) = child.child_by_field_name("declarator") {
                    return Self::find_declarator_name(decl, parsed);
                }
            }
        }
        None
    }

    /// Extract function/method name from a declaration or function_definition.
    fn extract_ts_function_name(
        node: tree_sitter::Node<'_>,
        parsed: &crate::ast::ParsedFile,
    ) -> Option<String> {
        // Try declarator field first
        if let Some(declarator) = node.child_by_field_name("declarator") {
            return Self::find_declarator_name(declarator, parsed);
        }
        None
    }

    /// Extract a typedef from a type_definition node.
    fn extract_ts_typedef(&mut self, node: tree_sitter::Node<'_>, parsed: &crate::ast::ParsedFile) {
        // type_definition: `typedef <type> <declarator>;`
        // Get the declarator (alias name)
        let declarator = match node.child_by_field_name("declarator") {
            Some(d) => d,
            None => return,
        };

        let alias_name = match Self::find_declarator_name(declarator, parsed) {
            Some(n) => n,
            None => return,
        };

        // Get the type (underlying type)
        let type_node = match node.child_by_field_name("type") {
            Some(t) => t,
            None => return,
        };

        let underlying = parsed.node_text(&type_node).to_string().trim().to_string();

        if !alias_name.is_empty() && !underlying.is_empty() {
            self.typedefs.insert(
                alias_name.clone(),
                TypedefInfo {
                    name: alias_name,
                    underlying,
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Clang invocation
// ---------------------------------------------------------------------------

/// Run clang with `-Xclang -ast-dump=json` on a compilation unit.
fn run_clang_ast_dump(cmd: &CompileCommand) -> Result<String> {
    let (compiler, mut args) = parse_compile_command(cmd)?;

    // Replace the compiler with clang/clang++ (use the original if already clang)
    let clang_cmd = if compiler.contains("++") || compiler.contains("cpp") {
        "clang++"
    } else {
        "clang"
    };

    // Remove output flags (-o <file>) and the source file from args
    let source_file = &cmd.file;
    remove_output_flags(&mut args, source_file);

    // Add AST dump flags
    args.push("-fsyntax-only".to_string());
    args.push("-Xclang".to_string());
    args.push("-ast-dump=json".to_string());
    args.push(source_file.clone());

    let output = Command::new(clang_cmd)
        .args(&args)
        .current_dir(&cmd.directory)
        .output()
        .map_err(|e| anyhow!("failed to run {}: {}", clang_cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // clang may succeed partially — check if we got JSON output
        let stdout = String::from_utf8(output.stdout)
            .map_err(|_| anyhow!("clang output is not valid UTF-8"))?;
        if stdout.starts_with('{') {
            // Partial success — warnings but valid AST
            return Ok(stdout);
        }
        return Err(anyhow!(
            "clang exited with {}: {}",
            output.status,
            stderr.lines().take(5).collect::<Vec<_>>().join("\n")
        ));
    }

    String::from_utf8(output.stdout).map_err(|_| anyhow!("clang output is not valid UTF-8"))
}

/// Parse a compile command into (compiler, arguments).
fn parse_compile_command(cmd: &CompileCommand) -> Result<(String, Vec<String>)> {
    if let Some(arguments) = &cmd.arguments {
        if arguments.is_empty() {
            return Err(anyhow!("empty arguments array in compile_commands.json"));
        }
        Ok((arguments[0].clone(), arguments[1..].to_vec()))
    } else if let Some(command) = &cmd.command {
        let parts: Vec<String> = shell_split(command);
        if parts.is_empty() {
            return Err(anyhow!("empty command in compile_commands.json"));
        }
        Ok((parts[0].clone(), parts[1..].to_vec()))
    } else {
        Err(anyhow!(
            "compile_commands.json entry has neither 'command' nor 'arguments'"
        ))
    }
}

/// Simple shell-like splitting (handles basic quoting).
fn shell_split(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quote = None;
    let mut escape = false;

    for c in s.chars() {
        if escape {
            current.push(c);
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        match in_quote {
            Some(q) if c == q => {
                in_quote = None;
            }
            Some(_) => {
                current.push(c);
            }
            None => {
                if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c.is_whitespace() {
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                } else {
                    current.push(c);
                }
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Remove `-o <file>` flags and the source file from compiler arguments.
fn remove_output_flags(args: &mut Vec<String>, source_file: &str) {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-o" {
            // Remove -o and its argument
            args.remove(i);
            if i < args.len() {
                args.remove(i);
            }
            continue;
        }
        if args[i] == "-c" {
            args.remove(i);
            continue;
        }
        // Remove the source file argument
        if args[i] == source_file
            || Path::new(&args[i]).file_name() == Path::new(source_file).file_name()
        {
            args.remove(i);
            continue;
        }
        i += 1;
    }
}

/// Strip pointer, reference, const, volatile qualifiers from a type string
/// to get the base record name.
///
/// "const struct device *" → "device"
/// "class MyClass &" → "MyClass"
/// "volatile int" → "int"
fn strip_type_qualifiers(type_str: &str) -> &str {
    let s = type_str.trim();
    // Strip trailing * and &
    let s = s.trim_end_matches('*').trim_end_matches('&').trim();
    // Strip const/volatile
    let s = s
        .trim_start_matches("const ")
        .trim_start_matches("volatile ")
        .trim_start_matches("struct ")
        .trim_start_matches("class ")
        .trim_start_matches("union ")
        .trim_start_matches("enum ");
    s.trim()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_type_qualifiers() {
        assert_eq!(strip_type_qualifiers("int"), "int");
        assert_eq!(strip_type_qualifiers("struct device *"), "device");
        assert_eq!(strip_type_qualifiers("const struct device *"), "device");
        assert_eq!(strip_type_qualifiers("class MyClass &"), "MyClass");
        assert_eq!(strip_type_qualifiers("volatile int"), "int");
        assert_eq!(strip_type_qualifiers("struct device"), "device");
    }

    #[test]
    fn test_shell_split() {
        let parts = shell_split(r#"gcc -I/usr/include -DFOO="bar baz" -c file.c -o file.o"#);
        assert_eq!(
            parts,
            vec![
                "gcc",
                "-I/usr/include",
                "-DFOO=bar baz",
                "-c",
                "file.c",
                "-o",
                "file.o"
            ]
        );
    }

    #[test]
    fn test_remove_output_flags() {
        let mut args = vec![
            "-I/usr/include".to_string(),
            "-c".to_string(),
            "file.c".to_string(),
            "-o".to_string(),
            "file.o".to_string(),
        ];
        remove_output_flags(&mut args, "file.c");
        assert_eq!(args, vec!["-I/usr/include"]);
    }

    #[test]
    fn test_typedef_resolution() {
        let mut db = TypeDatabase::default();
        db.typedefs.insert(
            "handle_t".to_string(),
            TypedefInfo {
                name: "handle_t".to_string(),
                underlying: "device_handle".to_string(),
            },
        );
        db.typedefs.insert(
            "device_handle".to_string(),
            TypedefInfo {
                name: "device_handle".to_string(),
                underlying: "struct device *".to_string(),
            },
        );

        assert_eq!(db.resolve_typedef("handle_t"), "struct device *");
        assert_eq!(db.resolve_typedef("int"), "int"); // no typedef
    }

    #[test]
    fn test_record_lookup_with_typedef() {
        let mut db = TypeDatabase::default();
        db.records.insert(
            "device".to_string(),
            RecordInfo {
                name: "device".to_string(),
                kind: RecordKind::Struct,
                fields: vec![
                    FieldInfo {
                        name: "name".to_string(),
                        type_str: "char *".to_string(),
                        offset: None,
                    },
                    FieldInfo {
                        name: "id".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                ],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: "device.h".to_string(),
            },
        );
        db.typedefs.insert(
            "dev_t".to_string(),
            TypedefInfo {
                name: "dev_t".to_string(),
                underlying: "struct device *".to_string(),
            },
        );

        // Direct lookup
        let record = db.resolve_record("device").unwrap();
        assert_eq!(record.fields.len(), 2);

        // Via typedef
        let record = db.resolve_record("dev_t").unwrap();
        assert_eq!(record.name, "device");

        // Field type query
        assert_eq!(db.field_type("device", "name"), Some("char *".to_string()));
        assert_eq!(db.field_type("device", "id"), Some("int".to_string()));
        assert_eq!(db.field_type("device", "nonexistent"), None);
    }

    #[test]
    fn test_class_hierarchy() {
        let mut db = TypeDatabase::default();
        db.records.insert(
            "Shape".to_string(),
            RecordInfo {
                name: "Shape".to_string(),
                kind: RecordKind::Class,
                fields: vec![],
                bases: vec![],
                virtual_methods: BTreeMap::from([("draw".to_string(), "void ()".to_string())]),
                size: None,
                file: "shape.h".to_string(),
            },
        );
        db.records.insert(
            "Circle".to_string(),
            RecordInfo {
                name: "Circle".to_string(),
                kind: RecordKind::Class,
                fields: vec![FieldInfo {
                    name: "radius".to_string(),
                    type_str: "double".to_string(),
                    offset: None,
                }],
                bases: vec!["Shape".to_string()],
                virtual_methods: BTreeMap::from([("draw".to_string(), "void ()".to_string())]),
                size: None,
                file: "circle.h".to_string(),
            },
        );
        db.records.insert(
            "Rect".to_string(),
            RecordInfo {
                name: "Rect".to_string(),
                kind: RecordKind::Class,
                fields: vec![],
                bases: vec!["Shape".to_string()],
                virtual_methods: BTreeMap::from([("draw".to_string(), "void ()".to_string())]),
                size: None,
                file: "rect.h".to_string(),
            },
        );
        db.class_hierarchy
            .insert("Circle".to_string(), vec!["Shape".to_string()]);
        db.class_hierarchy
            .insert("Rect".to_string(), vec!["Shape".to_string()]);

        // Hierarchy queries
        assert!(db.is_subclass_of("Circle", "Shape"));
        assert!(db.is_subclass_of("Rect", "Shape"));
        assert!(!db.is_subclass_of("Shape", "Circle"));
        assert!(!db.is_subclass_of("Circle", "Rect"));

        // Virtual dispatch
        let mut targets = db.virtual_dispatch_targets("Shape", "draw");
        targets.sort();
        assert_eq!(targets, vec!["Circle", "Rect", "Shape"]);
    }

    #[test]
    fn test_all_fields_with_inheritance() {
        let mut db = TypeDatabase::default();
        db.records.insert(
            "Base".to_string(),
            RecordInfo {
                name: "Base".to_string(),
                kind: RecordKind::Class,
                fields: vec![FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                }],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: "base.h".to_string(),
            },
        );
        db.records.insert(
            "Derived".to_string(),
            RecordInfo {
                name: "Derived".to_string(),
                kind: RecordKind::Class,
                fields: vec![FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: None,
                }],
                bases: vec!["Base".to_string()],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: "derived.h".to_string(),
            },
        );

        let fields = db.all_fields("Derived");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "id"); // inherited
        assert_eq!(fields[1].name, "name"); // own
    }

    #[test]
    fn test_union_detection() {
        let mut db = TypeDatabase::default();
        db.records.insert(
            "data_value".to_string(),
            RecordInfo {
                name: "data_value".to_string(),
                kind: RecordKind::Union,
                fields: vec![
                    FieldInfo {
                        name: "i".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                    FieldInfo {
                        name: "f".to_string(),
                        type_str: "float".to_string(),
                        offset: None,
                    },
                ],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: "value.h".to_string(),
            },
        );

        assert!(db.is_union("data_value"));
        assert!(!db.is_union("nonexistent"));
    }

    #[test]
    fn test_pointer_type_detection() {
        assert!(TypeDatabase::is_pointer_type("struct device *"));
        assert!(TypeDatabase::is_pointer_type("int *"));
        assert!(!TypeDatabase::is_pointer_type("int"));
        assert!(!TypeDatabase::is_pointer_type("struct device"));
    }

    #[test]
    fn test_extract_from_ast_json() {
        // Minimal clang AST JSON for a struct definition
        let json = r#"{
            "kind": "TranslationUnitDecl",
            "inner": [
                {
                    "kind": "RecordDecl",
                    "name": "point",
                    "tagUsed": "struct",
                    "completeDefinition": true,
                    "inner": [
                        {
                            "kind": "FieldDecl",
                            "name": "x",
                            "type": { "qualType": "int" }
                        },
                        {
                            "kind": "FieldDecl",
                            "name": "y",
                            "type": { "qualType": "int" }
                        }
                    ]
                },
                {
                    "kind": "TypedefDecl",
                    "name": "point_t",
                    "type": { "qualType": "struct point", "desugaredQualType": "struct point" }
                }
            ]
        }"#;

        let mut db = TypeDatabase::default();
        db.extract_from_ast(json, "test.c").unwrap();

        assert_eq!(db.records.len(), 1);
        let record = db.records.get("point").unwrap();
        assert_eq!(record.kind, RecordKind::Struct);
        assert_eq!(record.fields.len(), 2);
        assert_eq!(record.fields[0].name, "x");
        assert_eq!(record.fields[1].name, "y");

        assert_eq!(db.typedefs.len(), 1);
        let td = db.typedefs.get("point_t").unwrap();
        assert_eq!(td.underlying, "struct point");
    }
}
