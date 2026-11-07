//! JS/TS import and require extraction, import bindings and type-only imports.

use crate::ast::{is_plain_ident, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

impl ParsedFile {
    pub(in crate::ast) fn collect_js_imports(
        &self,
        node: Node<'_>,
        out: &mut BTreeMap<String, String>,
    ) {
        match node.kind() {
            // ES6: `import x from './mod'` or `import { func } from './mod'`
            "import_statement" => {
                let source = node.child_by_field_name("source").map(|n| {
                    let text = self.node_text(&n);
                    text.trim_matches(|c| c == '\'' || c == '"').to_string()
                });
                if let Some(module_path) = source {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "import_clause" => {
                                self.collect_js_import_clause(&child, &module_path, out);
                            }
                            "identifier" => {
                                // `import x from './mod'`
                                let name = self.node_text(&child).to_string();
                                out.insert(name, module_path.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
            // CommonJS: `const x = require('./mod')`
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        self.collect_require_binding(&child, out);
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_imports(child, out);
                }
            }
        }
    }

    fn collect_js_import_clause(
        &self,
        node: &Node<'_>,
        module_path: &str,
        out: &mut BTreeMap<String, String>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    // Default import: `import utils from './mod'`
                    out.insert(self.node_text(&child).to_string(), module_path.to_string());
                }
                "named_imports" => {
                    // `import { func, other as alias } from './mod'`
                    let mut inner = child.walk();
                    for spec in child.children(&mut inner) {
                        if spec.kind() == "import_specifier" {
                            let name = spec
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = spec
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let Some(local) = alias.or(name) {
                                out.insert(local, module_path.to_string());
                            }
                        }
                    }
                }
                "namespace_import" => {
                    // `import * as utils from './mod'`
                    if let Some(id) = child.child_by_field_name("name") {
                        out.insert(self.node_text(&id).to_string(), module_path.to_string());
                    }
                    // Fallback: find identifier child
                    let mut inner = child.walk();
                    for c in child.children(&mut inner) {
                        if c.kind() == "identifier" {
                            out.insert(self.node_text(&c).to_string(), module_path.to_string());
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_require_binding(&self, node: &Node<'_>, out: &mut BTreeMap<String, String>) {
        // `const x = require('./mod')` or `const { a, b } = require('./mod')`
        let value = node.child_by_field_name("value");
        let name = node.child_by_field_name("name");

        if let Some(val) = value {
            if let (Some(n), Some((module_path, _member))) =
                (name, self.js_require_member_binding(&val))
            {
                if n.kind() == "identifier" {
                    out.insert(self.node_text(&n).to_string(), module_path);
                    return;
                }
            }
            // CommonJS factory calls: `const app = require('fastify')()`.
            // Treat the bound local as importing the required module so framework
            // detection and import-aware sink logic see the canonical binding.
            if self.language.is_call_node(val.kind()) {
                if let Some(func) = val.child_by_field_name("function") {
                    if let Some(path) = self.js_require_call_module_path(&func) {
                        if let Some(n) = &name {
                            if n.kind() == "object_pattern" {
                                self.collect_js_require_pattern_bindings(n, &path, out);
                            } else {
                                out.insert(self.node_text(n).to_string(), path);
                            }
                        }
                        return;
                    }
                }
            }
            // Check if value is a require() call
            if self.language.is_call_node(val.kind()) {
                if let Some(func_name) = self.call_function_name(&val) {
                    if self.node_text(&func_name) == "require"
                        && self.js_ts_unshadowed_ambient(func_name)
                    {
                        if let Some(args) = self.language.call_arguments(&val) {
                            // Extract the module path from first argument
                            let mut cursor = args.walk();
                            for child in args.children(&mut cursor) {
                                if self.is_positional_argument_node(child) {
                                    let text = self.node_text(&child);
                                    let path =
                                        text.trim_matches(|c| c == '\'' || c == '"').to_string();
                                    // Bind the name(s) to this module
                                    if let Some(n) = &name {
                                        if n.kind() == "object_pattern" {
                                            // Destructuring: `const { a, b } = require('./mod')`
                                            self.collect_js_require_pattern_bindings(n, &path, out);
                                        } else {
                                            out.insert(self.node_text(n).to_string(), path.clone());
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn js_require_member_binding(&self, node: &Node<'_>) -> Option<(String, String)> {
        if node.kind() != "member_expression" {
            return None;
        }
        let object = node.child_by_field_name("object")?;
        let property = node.child_by_field_name("property")?;
        let module_path = self.js_require_call_module_path(&object)?;
        let member = self
            .node_text(&property)
            .trim_matches(|c| c == '\'' || c == '"' || c == '`')
            .to_string();
        Some((module_path, member))
    }

    fn js_require_call_module_path(&self, node: &Node<'_>) -> Option<String> {
        if !self.language.is_call_node(node.kind()) {
            return None;
        }
        let func_name = self.call_function_name(node)?;
        if self.node_text(&func_name) != "require" || !self.js_ts_unshadowed_ambient(func_name) {
            return None;
        }
        let args = self.language.call_arguments(node)?;
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if self.is_positional_argument_node(child) {
                let text = self.node_text(&child);
                return Some(text.trim_matches(|c| c == '\'' || c == '"').to_string());
            }
        }
        None
    }

    fn collect_js_require_pattern_bindings(
        &self,
        pattern: &Node<'_>,
        module_path: &str,
        out: &mut BTreeMap<String, String>,
    ) {
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" | "identifier" => {
                    out.insert(self.node_text(&child).to_string(), module_path.to_string());
                }
                "pair_pattern" => {
                    if let Some(value) = child.child_by_field_name("value") {
                        if value.kind() == "identifier" {
                            out.insert(self.node_text(&value).to_string(), module_path.to_string());
                        } else if value.kind() == "object_pattern"
                            || value.kind() == "array_pattern"
                        {
                            self.collect_js_require_pattern_bindings(&value, module_path, out);
                        }
                    }
                }
                "rest_pattern" => {
                    let mut inner = child.walk();
                    for inner_child in child.children(&mut inner) {
                        if inner_child.kind() == "identifier" {
                            out.insert(
                                self.node_text(&inner_child).to_string(),
                                module_path.to_string(),
                            );
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.collect_js_require_pattern_bindings(&child, module_path, out);
                }
                _ => {}
            }
        }
    }

    /// Collect import binding data from a single JS/TS `import_statement` node.
    /// Called only for direct children of the root `program` node.
    pub(in crate::ast) fn collect_js_import_bindings_node(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        match node.kind() {
            "import_statement" => self.collect_js_import_statement_bindings(node, out),
            // CommonJS destructured `require`: `const { a, b: c } = require('./y')`.
            // Whole-module requires (`const g = require('./y')`) stay out of
            // scope here — no member name to bind against R4c.
            //
            // F1 (review-fix wave, codex BLOCKER 1): `const` only. A `let`/`var`
            // destructured require can be reassigned after the binding
            // (`let { f } = require('./m'); f = localFn;`), and this layer does
            // not track reassignment -- extracting it anyway risks R4c minting
            // a false Exact edge to the require target even after a rebind.
            // ADJUDICATION: fail closed by never extracting `let`/`var`
            // destructured requires at all (not just when later reassigned) --
            // deliberately not building assignment-rebind tracking this slice.
            "lexical_declaration" if self.js_ts_lexical_declaration_is_const(node) => {
                self.collect_js_require_import_bindings(node, out)
            }
            "lexical_declaration" | "variable_declaration" => {}
            _ => {}
        }
    }

    /// Whether a `lexical_declaration` node (`const x = ...` / `let x = ...`)
    /// is the `const` form. `variable_declaration` (`var`) has no `kind`
    /// field and is handled by its own match arm.
    fn js_ts_lexical_declaration_is_const(&self, node: Node<'_>) -> bool {
        node.child_by_field_name("kind")
            .is_some_and(|k| k.kind() == "const")
    }

    pub(in crate::ast) fn collect_js_import_statement_bindings(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        use crate::call_graph::{ImportBinding, ImportBindingKind};
        if self.js_ts_import_statement_is_type_only(node) {
            return;
        }
        let source = node.child_by_field_name("source").map(|n| {
            let text = self.node_text(&n);
            text.trim_matches(|c| c == '\'' || c == '"').to_string()
        });
        if let Some(module_path) = source {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "import_clause" => {
                        self.collect_js_import_clause_bindings(&child, &module_path, out);
                    }
                    "identifier" => {
                        let name = self.node_text(&child).to_string();
                        out.push(ImportBinding {
                            local: name,
                            module_path: module_path.clone(),
                            member: Some("default".to_string()),
                            kind: ImportBindingKind::MemberImport,
                            eligible: false, // eligibility set later
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    /// `const { a, b: c } = require('./y')` — a destructured require binds
    /// each destructured property as a `MemberImport`, mirroring
    /// `import { a, b as c } from './y'`. Only object-pattern destructuring is
    /// in scope; `const g = require('./y')` (whole module) is a `ModuleImport`
    /// handled separately (framework/sink detection), not R4c member
    /// resolution.
    fn collect_js_require_import_bindings(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        let mut cursor = node.walk();
        for decl in node.children(&mut cursor) {
            if decl.kind() != "variable_declarator" {
                continue;
            }
            let (Some(name_node), Some(value)) = (
                decl.child_by_field_name("name"),
                decl.child_by_field_name("value"),
            ) else {
                continue;
            };
            if name_node.kind() != "object_pattern" {
                continue;
            }
            let Some(module_path) = self.js_require_call_module_path(&value) else {
                continue;
            };
            self.collect_js_require_pattern_import_bindings(&name_node, &module_path, out);
        }
    }

    fn collect_js_require_pattern_import_bindings(
        &self,
        pattern: &Node<'_>,
        module_path: &str,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        use crate::call_graph::{ImportBinding, ImportBindingKind};
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" => {
                    let name = self.node_text(&child).to_string();
                    out.push(ImportBinding {
                        local: name.clone(),
                        module_path: module_path.to_string(),
                        member: Some(name),
                        kind: ImportBindingKind::MemberImport,
                        eligible: false, // eligibility set later
                    });
                }
                "pair_pattern" => {
                    let key = child
                        .child_by_field_name("key")
                        .map(|k| self.node_text(&k).to_string());
                    let value = child.child_by_field_name("value");
                    if let (Some(key), Some(value)) = (key, value) {
                        if value.kind() == "identifier" {
                            out.push(ImportBinding {
                                local: self.node_text(&value).to_string(),
                                module_path: module_path.to_string(),
                                member: Some(key),
                                kind: ImportBindingKind::MemberImport,
                                eligible: false, // eligibility set later
                            });
                        }
                        // Nested destructuring patterns are out of scope.
                    }
                }
                // Rest patterns (`...rest`) are out of scope: no single
                // member name to bind.
                _ => {}
            }
        }
    }

    fn collect_js_import_clause_bindings(
        &self,
        node: &Node<'_>,
        module_path: &str,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        use crate::call_graph::{ImportBinding, ImportBindingKind};
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    // `import X from './y'` — a default import. Eligible for
                    // R4c member-import consultation like a named import; it
                    // only actually resolves when the target module has a
                    // `"default"` export fact (js_ts_resolved_exports), so an
                    // ordinary named-export-only module still falls through.
                    let name = self.node_text(&child).to_string();
                    out.push(ImportBinding {
                        local: name.clone(),
                        module_path: module_path.to_string(),
                        member: Some("default".to_string()),
                        kind: ImportBindingKind::MemberImport,
                        eligible: false, // eligibility set later
                    });
                }
                "named_imports" => {
                    let mut inner = child.walk();
                    for spec in child.children(&mut inner) {
                        if spec.kind() == "import_specifier" {
                            if self.js_ts_import_specifier_is_type_only(spec) {
                                continue;
                            }
                            // `import { "g" as h }` names export `g` (StringValue, S1b-2b); an
                            // unmatchable name (an unpaired surrogate, or any name in a list with a
                            // parse error: recovery puts the ERROR beside the specifier) keeps the
                            // binding with no member, which R4c refuses.
                            let name = spec
                                .child_by_field_name("name")
                                .and_then(|n| self.js_ts_module_export_name(n));
                            let alias = spec
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            let local = alias.clone().or_else(|| name.clone());
                            if let Some(local) = local {
                                out.push(ImportBinding {
                                    local,
                                    module_path: module_path.to_string(),
                                    member: name.filter(|_| !child.has_error()),
                                    kind: ImportBindingKind::MemberImport,
                                    eligible: true,
                                });
                            }
                        }
                    }
                }
                "namespace_import" => {
                    // `import * as utils from './mod'` — not a wildcard poison;
                    // it's a namespace binding (module import).
                    let ident = child.child_by_field_name("name");
                    let ident = if ident.is_some() {
                        ident
                    } else {
                        let mut inner = child.walk();
                        let found = child
                            .children(&mut inner)
                            .find(|c| c.kind() == "identifier");
                        found
                    };
                    if let Some(id) = ident {
                        out.push(ImportBinding {
                            local: self.node_text(&id).to_string(),
                            module_path: module_path.to_string(),
                            member: None,
                            kind: ImportBindingKind::ModuleImport,
                            eligible: false,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    pub(in crate::ast) fn js_ts_import_statement_is_type_only(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| child.kind() == "type");
        found
    }

    pub(in crate::ast) fn js_ts_import_specifier_is_type_only(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| child.kind() == "type");
        found
    }

    pub(in crate::ast) fn js_ts_type_only_imports(
        &self,
    ) -> BTreeMap<String, Option<(String, String)>> {
        let mut imports = BTreeMap::new();
        if !matches!(self.language, Language::TypeScript | Language::Tsx)
            || self.tree.root_node().has_error()
        {
            return imports;
        }
        fn insert(
            out: &mut BTreeMap<String, Option<(String, String)>>,
            local: String,
            target: Option<(String, String)>,
        ) {
            use std::collections::btree_map::Entry;
            match out.entry(local) {
                Entry::Vacant(e) => {
                    e.insert(target);
                }
                Entry::Occupied(mut e) => {
                    e.insert(None);
                }
            }
        }
        fn walk(
            parsed: &ParsedFile,
            node: Node<'_>,
            module: &str,
            statement_type_only: bool,
            out: &mut BTreeMap<String, Option<(String, String)>>,
        ) {
            if node.kind() == "import_specifier" {
                if statement_type_only || parsed.js_ts_import_specifier_is_type_only(node) {
                    if let Some(name) = node.child_by_field_name("name") {
                        let imported = parsed.node_text(&name);
                        let local = node.child_by_field_name("alias").unwrap_or(name);
                        let target = is_plain_ident(imported)
                            .then(|| (module.to_string(), imported.to_string()));
                        insert(out, parsed.node_text(&local).to_string(), target);
                    }
                }
                return;
            }
            if statement_type_only
                && node.kind() == "identifier"
                && node
                    .parent()
                    .is_some_and(|p| matches!(p.kind(), "import_clause" | "namespace_import"))
            {
                let target = node
                    .parent()
                    .filter(|p| p.kind() == "import_clause")
                    .map(|_| (module.to_string(), "default".to_string()));
                insert(out, parsed.node_text(&node).to_string(), target);
                return;
            }
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk(parsed, child, module, statement_type_only, out);
            }
        }
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for statement in root
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "import_statement")
        {
            if let Some(source) = statement.child_by_field_name("source") {
                let module = self.node_text(&source).trim_matches(['\'', '"']);
                walk(
                    self,
                    statement,
                    module,
                    self.js_ts_import_statement_is_type_only(statement),
                    &mut imports,
                );
            }
        }
        fn poison_module_types(
            parsed: &ParsedFile,
            node: Node<'_>,
            imports: &mut BTreeMap<String, Option<(String, String)>>,
        ) {
            if matches!(
                node.kind(),
                "class_declaration"
                    | "abstract_class_declaration"
                    | "interface_declaration"
                    | "type_alias_declaration"
                    | "enum_declaration"
                    | "internal_module"
                    | "module"
                    | "import_alias"
            ) {
                if let Some(name) = node.child_by_field_name("name") {
                    // A dotted namespace introduces its first component in the
                    // enclosing type namespace, not the entire dotted spelling.
                    let local = parsed
                        .node_text(&name)
                        .split('.')
                        .next()
                        .unwrap_or_default();
                    if let Some(target) = imports.get_mut(local) {
                        *target = None;
                    }
                }
            } else if matches!(
                node.kind(),
                "program" | "export_statement" | "ambient_declaration" | "expression_statement"
            ) {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    poison_module_types(parsed, child, imports);
                }
            }
        }
        poison_module_types(self, root, &mut imports);
        imports
    }
}
