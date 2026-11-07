//! JS/TS export facts: ESM export statements, CommonJS exports, default classes.

use crate::ast::{js_binding, js_export_memo, JsBinding, JsTsReceiverBindingEvidence, ParsedFile};
use crate::languages::Language;
use tree_sitter::Node;

impl ParsedFile {
    /// Extract JS/TS module-level export facts: exported name (what an
    /// importer writes — `"default"`, `"process"`, a rename target, ...) ->
    /// either a local declaration in this file or a re-export target in
    /// another module, plus `export * from` barrel module paths.
    ///
    /// Covers: `export default function name() {}` / `export default name;`,
    /// named export lists incl. renames (`export { a, b as c };`), exported
    /// const-arrow / function-expression declarations
    /// (`export const f = () => {};`), CommonJS `module.exports`/`exports`
    /// assignments, and re-export chains (`export { x } from './y'`,
    /// `export * from './y'`). Re-export chains are resolved whole-program,
    /// depth-bounded and cycle-safe, in
    /// `CallGraph::apply_js_export_resolution` (`js_exports::resolve_js_exports`)
    /// — this method only extracts the raw, per-file, un-resolved facts.
    ///
    /// Out of scope (skipped, counted in `skipped_expr_count` where the
    /// syntax is otherwise a recognized export/CJS-assignment shape but the
    /// value isn't a plain identifier): dynamic `require(expr)`/`import(expr)`,
    /// TS `export =` CJS interop, anonymous/aliased class exports, anonymous default
    /// function/arrow exports, spread in `module.exports = { ...x }`.
    pub fn extract_js_ts_export_facts(&self) -> super::exports::JsExportFacts {
        self.js_export_facts
            .get_or_init(|| self.extract_js_ts_export_facts_uncached())
            .clone()
    }

    fn extract_js_ts_export_facts_uncached(&self) -> super::exports::JsExportFacts {
        let mut facts = super::exports::JsExportFacts::default();
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return facts;
        }

        let _session = js_export_memo::Session::new(self);
        let root = self.tree.root_node();
        let mut cjs = super::exports::JsExportFacts::default();
        // A local export of an imported binding is not an in-file callable.
        // Forwarding requires separate proof; do not let a nested same-name
        // declaration satisfy the existing Local(file, name) route.
        let types = self.js_ts_type_only_imports();
        let imports = self.extract_import_bindings().into_iter().map(|b| b.local);
        let mut d4 = (
            imports.chain(types.keys().cloned()).collect(),
            Default::default(),
        );
        let mut has_esm_exports = false;
        let mut unsupported_export = root.has_error();
        facts.namespace_proof_complete = true;
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "export_statement" => {
                    has_esm_exports = true;
                    let mut export_cursor = child.walk();
                    unsupported_export |= child
                        .children(&mut export_cursor)
                        .any(|n| matches!(n.kind(), "=" | "import_alias"));
                    self.collect_js_ts_export_statement(child, &mut facts, &mut d4)
                }
                "expression_statement" => self.collect_js_ts_cjs_export_statement(child, &mut cjs),
                _ => {}
            }
        }
        // Preserve ESM provenance: an unsafe CJS use cannot erase an
        // independently extracted ESM export. Safe cross-form duplicates still
        // pass through the normal conflict-aware insertion path.
        let cjs_safe = self.js_ts_cjs_export_object_safe();
        facts.namespace_proof_complete &= has_esm_exports
            && !unsupported_export
            && cjs.is_empty()
            && cjs_safe
            && facts.skipped_expr_count == 0
            && facts.local_export_refusals.is_empty();
        facts.skipped_expr_count += cjs.skipped_expr_count;
        // #664: in a file with an ESM export `exports`/`module` do not exist, so its CommonJS
        // writes are no authoritative export claim (#598). A name an ESM export also claims still
        // meets it in the conflict-aware insertion (cross-form duplicates stay conflicted); every
        // other CJS name takes the refusal path below, as an unsafe producer's does.
        let cjs_trusted = cjs.conflicted.is_empty() && cjs_safe;
        if cjs_trusted && !has_esm_exports {
            for (name, target) in cjs.named {
                facts.insert_named(name, target);
            }
            facts.cjs_callables = cjs.cjs_callables;
            facts.cjs_module_callable = cjs.cjs_module_callable;
        } else {
            if cjs_trusted {
                for (name, target) in &cjs.named {
                    if facts.named.contains_key(name) || facts.conflicted.contains(name) {
                        facts.insert_named(name.clone(), target.clone());
                    }
                }
            }
            // Unsafe producers revoke the whole CJS set, including disjoint
            // siblings. Retain already-enumerated names as refusal claims so a
            // barrel cannot mistake them for absence. Independent ESM wins.
            // No claim is invented for unknown/computed-only export names.
            for name in cjs.named.keys().chain(cjs.conflicted.iter()) {
                if !facts.named.contains_key(name) && !facts.conflicted.contains(name) {
                    facts.insert_named(
                        name.clone(),
                        super::exports::JsExportTarget::UnprovenLocal(name.clone()),
                    );
                }
            }
        }
        self.js_ts_export_imports_and_functions(&mut facts);
        facts.type_only_imports = types;
        facts.module_value_bindings = self.js_ts_function_local_bindings(&root);
        for (exported, target) in &facts.named {
            if let super::exports::JsExportTarget::Local(local) = target {
                if d4.0.contains(local) {
                    facts.conflicted.insert(exported.clone());
                }
            }
        }
        for local in &facts.module_value_bindings {
            // The landed core's wrapper provenance check is structural and cannot
            // re-enter classification through a self/mutual initializer cycle.
            if let JsBinding::Callable(t) = self.js_ts_module_binding(local, root, &mut d4.1) {
                facts
                    .namespace_callable_locals
                    .entry(t.local.clone())
                    .or_default()
                    .push(super::exports::ResolvedJsExport {
                        file: self.path.clone(),
                        local_name: t.local,
                        span: Some((t.start_line, t.end_line)),
                        wrapped: t.wrapped,
                        // Local callable terminals have traversed no star. The
                        // namespace resolver rejects any unresolved branch.
                        via_unresolved_star: false,
                    });
            }
        }
        facts
    }

    fn js_ts_export_statement_is_default(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| child.kind() == "default");
        found
    }

    /// The bounded class-export lane accepts only an undecorated named declaration.
    pub(crate) fn js_ts_named_exported_class<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        if node.kind() != "export_statement" || node.has_error() {
            return None;
        }
        let declaration = node.child_by_field_name("declaration")?;
        let mut cursor = node.walk();
        let decorated = node
            .named_children(&mut cursor)
            .any(|n| n.kind() == "decorator");
        let mut cursor = declaration.walk();
        let decorated = decorated
            || declaration
                .named_children(&mut cursor)
                .any(|n| n.kind() == "decorator");
        (declaration.kind() == "class_declaration" && !decorated).then_some(declaration)
    }

    /// None means this is not a local declared class. Some(false) is terminal
    /// class poison, not permission to reinterpret the export as a function.
    fn js_ts_local_default_class(&self, value: &Node<'_>, name: &str) -> Option<bool> {
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        let classes: Vec<_> = root
            .named_children(&mut cursor)
            .filter_map(|node| {
                let declaration = if node.kind() == "export_statement" {
                    node.child_by_field_name("declaration")?
                } else {
                    node
                };
                (matches!(
                    declaration.kind(),
                    "class_declaration" | "abstract_class_declaration"
                ) && declaration
                    .child_by_field_name("name")
                    .is_some_and(|n| self.node_text(&n) == name))
                .then_some((node, declaration))
            })
            .collect();
        if classes.is_empty() {
            return None;
        }
        let (wrapper, declaration) = classes[0];
        let mut cursor = wrapper.walk();
        let decorated = wrapper
            .named_children(&mut cursor)
            .any(|n| n.kind() == "decorator");
        let mut cursor = declaration.walk();
        let decorated = decorated
            || declaration
                .named_children(&mut cursor)
                .any(|n| n.kind() == "decorator");
        Some(
            !root.has_error()
                && classes.len() == 1
                && declaration.kind() == "class_declaration"
                && !decorated
                && declaration.end_byte() < value.start_byte()
                && matches!(
                    self.js_ts_scope_receiver_binding_evidence(root, value, name, false, false),
                    Some(JsTsReceiverBindingEvidence::ClassOwner)
                )
                && !self.js_ts_type_only_imports().contains_key(name)
                && !self
                    .extract_import_bindings()
                    .iter()
                    .any(|binding| binding.local == name)
                && !self.js_ts_module_value_written(name),
        )
    }

    /// Handle a single top-level `export_statement` node: default exports,
    /// named export lists (local or re-export), direct declaration exports
    /// (`export function`/`export const`), and `export * [from]`.
    fn collect_js_ts_export_statement<'a>(
        &'a self,
        node: Node<'a>,
        facts: &mut super::exports::JsExportFacts,
        d4: &mut js_binding::ExportScope<'a>,
    ) {
        use super::exports::JsExportTarget;

        // The grammar uses the same immediate `type` token for type-only
        // import/export statements and specifiers; names named `type` remain
        // identifier nodes and are not rejected by this predicate.
        if self.js_ts_import_statement_is_type_only(node) {
            return;
        }

        let source = node.child_by_field_name("source").map(|n| {
            self.node_text(&n)
                .trim_matches(|c| c == '\'' || c == '"')
                .to_string()
        });

        let mut export_clause = None;
        let mut has_namespace_export = false;
        let mut ec = node.walk();
        for child in node.children(&mut ec) {
            match child.kind() {
                "export_clause" => export_clause = Some(child),
                "namespace_export" => {
                    has_namespace_export = true;
                    facts.namespace_proof_complete = false;
                }
                _ => {}
            }
        }

        // `export * from './y'` (bare barrel re-export-all). `export * as ns
        // from './y'` creates a namespace export object instead — out of scope.
        if let Some(module_path) = &source {
            if export_clause.is_none() && !has_namespace_export {
                facts.star_reexports.insert(module_path.clone());
                return;
            }
        }

        // `export { a, b as c };` (local) or `export { a, b as c } from './y'`
        // (re-export list).
        if let Some(clause) = export_clause {
            let mut cc = clause.walk();
            for spec in clause.children(&mut cc) {
                if spec.kind() != "export_specifier"
                    || self.js_ts_import_specifier_is_type_only(spec)
                    || clause.has_error()
                {
                    continue;
                }
                // An errorful list (above: recovery puts the ERROR beside the specifier) or an
                // unmatchable name or alias (an unpaired surrogate) records no claim.
                let Some(name) = spec
                    .child_by_field_name("name")
                    .and_then(|n| self.js_ts_module_export_name(n))
                else {
                    facts.namespace_proof_complete = false;
                    continue;
                };
                let alias = spec
                    .child_by_field_name("alias")
                    .map(|n| self.js_ts_module_export_name(n));
                let Some(exported_as) = alias.unwrap_or_else(|| Some(name.clone())) else {
                    facts.namespace_proof_complete = false;
                    continue;
                };
                let target = match &source {
                    Some(module_path) => JsExportTarget::ReExport {
                        module_path: module_path.clone(),
                        imported: name,
                    },
                    None => self
                        .js_ts_forwarded_import(&name)
                        .unwrap_or_else(|| self.js_ts_local_export_target(name, spec, facts, d4)),
                };
                facts.insert_named(exported_as, target);
            }
            return;
        }

        // `export default <expr>;` (no `declaration` field — a bare value).
        if let Some(value) = node.child_by_field_name("value") {
            if value.kind() == "identifier" {
                let name = self.node_text(&value).to_string();
                if let Some(proven) = self.js_ts_local_default_class(&value, &name) {
                    facts.insert_named("default".to_string(), JsExportTarget::Class(name));
                    if !proven {
                        // A rejected class must not retry the callable Local lane.
                        facts.conflicted.insert("default".to_string());
                    }
                    return;
                }
                let target = self
                    .js_ts_forwarded_import(&name)
                    .unwrap_or_else(|| self.js_ts_local_export_target(name, value, facts, d4));
                facts.insert_named("default".to_string(), target);
            } else {
                facts.skipped_expr_count += 1;
            }
            return;
        }

        // `export function name() {}` / `export default function name() {}` /
        // `export const f = () => {};` / directly named `export class Foo {}`.
        if let Some(decl) = node.child_by_field_name("declaration") {
            let is_default = self.js_ts_export_statement_is_default(node);
            match decl.kind() {
                "class_declaration" if self.js_ts_named_exported_class(node).is_some() => {
                    if let Some(name) = decl.child_by_field_name("name") {
                        let name = self.node_text(&name).to_string();
                        let exported = if is_default {
                            "default".to_string()
                        } else {
                            name.clone()
                        };
                        facts.insert_named(exported, JsExportTarget::Class(name));
                    }
                }
                "function_declaration" | "generator_function_declaration" => {
                    if let Some(name) = self.language.function_name(&decl) {
                        let name_text = self.node_text(&name).to_string();
                        let exported_as = if is_default {
                            "default".to_string()
                        } else {
                            name_text.clone()
                        };
                        let target = self.js_ts_local_export_target(name_text, decl, facts, d4);
                        facts.insert_named(exported_as, target);
                    }
                }
                "lexical_declaration" | "variable_declaration" => {
                    let mut vc = decl.walk();
                    for d in decl.children(&mut vc) {
                        if d.kind() != "variable_declarator" {
                            continue;
                        }
                        let Some(name_node) = d.child_by_field_name("name") else {
                            continue;
                        };
                        if name_node.kind() != "identifier" {
                            facts.namespace_proof_complete = false;
                            continue;
                        }
                        let name_text = self.node_text(&name_node).to_string();
                        // F4 (review-fix wave, codex MAJOR 2): only the
                        // spec's 1c forms (arrow function / function
                        // expression initializer) are in scope. Any other
                        // initializer (identifier, ternary, call, literal,
                        // ...) is skipped + counted -- recording it would
                        // let R4c bind the export to an unrelated same-named
                        // declaration elsewhere in the file (e.g. a nested
                        // function), since the "local" target here is just
                        // the declarator's own name, not a verified function.
                        //
                        // S1 (SPEC §3.1): a call initializer is admitted only as a
                        // span-verified React wrapper; every other skip names its reason.
                        let admitted = match d.child_by_field_name("value").map(|v| v.kind()) {
                            Some("arrow_function") | Some("function_expression") => {
                                let target =
                                    self.js_ts_local_export_target(name_text.clone(), d, facts, d4);
                                facts.insert_named(name_text, target);
                                continue;
                            }
                            Some("call_expression") => self.js_ts_wrapped_export(decl, d, false),
                            _ => Err("non_call_initializer"),
                        };
                        match admitted {
                            Ok((local, start_line, end_line)) => {
                                facts.spanned_admitted += 1;
                                let target = JsExportTarget::SpannedLocal {
                                    local,
                                    start_line,
                                    end_line,
                                };
                                facts.insert_named(name_text, target);
                            }
                            Err(reason) => {
                                facts.skipped_expr_count += 1;
                                *facts
                                    .skipped_decl_reasons
                                    .entry(reason.to_string())
                                    .or_default() += 1;
                            }
                        }
                    }
                }
                // Unrecorded value exports cannot establish a unique namespace identity.
                _ => facts.namespace_proof_complete = false,
            }
        } else {
            facts.namespace_proof_complete = false;
        }
    }

    /// Handle a single top-level `expression_statement`, looking for CommonJS
    /// `module.exports = ...` / `module.exports.f = ...` / `exports.f = ...`
    /// assignments.
    fn collect_js_ts_cjs_export_statement(
        &self,
        node: Node<'_>,
        facts: &mut super::exports::JsExportFacts,
    ) {
        let mut cursor = node.walk();
        let Some(assign) = node
            .children(&mut cursor)
            .find(|c| c.kind() == "assignment_expression")
        else {
            return;
        };
        let (Some(left), Some(right)) = (
            assign.child_by_field_name("left"),
            assign.child_by_field_name("right"),
        ) else {
            return;
        };
        if left.kind() != "member_expression" {
            return;
        }
        let (Some(object), Some(property)) = (
            left.child_by_field_name("object"),
            left.child_by_field_name("property"),
        ) else {
            return;
        };
        let property_name = self.node_text(&property).to_string();

        if object.kind() == "identifier"
            && self.node_text(&object) == "module"
            && property_name == "exports"
        {
            if !self.js_ts_unshadowed_ambient(object) {
                return;
            }
            // `module.exports = <rhs>;`
            self.collect_js_ts_cjs_module_exports_rhs(right, facts);
            return;
        }

        let is_module_exports_member = object.kind() == "member_expression"
            && object.child_by_field_name("object").is_some_and(|o| {
                o.kind() == "identifier"
                    && self.node_text(&o) == "module"
                    && self.js_ts_unshadowed_ambient(o)
            })
            && object
                .child_by_field_name("property")
                .is_some_and(|p| self.node_text(&p) == "exports");
        let is_exports_member = object.kind() == "identifier"
            && self.node_text(&object) == "exports"
            && self.js_ts_unshadowed_ambient(object)
            && !self.js_ts_module_exports_replaced(self.tree.root_node());

        if is_module_exports_member || is_exports_member {
            self.collect_js_ts_cjs_member_value(property_name, right, facts);
        }
    }

    /// `module.exports = <rhs>;` — either a single identifier (the module's
    /// whole export value, consumed like a default export) or an object
    /// literal of named members (`{ a, x: b }`).
    fn collect_js_ts_cjs_module_exports_rhs(
        &self,
        rhs: Node<'_>,
        facts: &mut super::exports::JsExportFacts,
    ) {
        match rhs.kind() {
            "identifier" => {
                self.insert_js_ts_cjs_local(facts, None, rhs);
            }
            "function_expression" => match self.js_ts_cjs_named_expr_target(rhs) {
                Some(target) => self.insert_js_ts_cjs_verified(facts, None, target),
                None => facts.skipped_expr_count += 1,
            },
            "object" => {
                // F2 (review-fix wave, codex BLOCKER 2): a spread can shadow
                // any named member with a value prism cannot see (`{ f,
                // ...override }` -- `override` may itself define `f`). Fail
                // closed for the WHOLE object literal when ANY spread is
                // present, rather than only the members after it: record
                // zero facts from this literal, counted once via the
                // existing skip counter. (Possible future refinement: only
                // poison names after the last spread -- not done this slice.)
                let mut probe = rhs.walk();
                if rhs
                    .children(&mut probe)
                    .any(|c| c.kind() == "spread_element")
                {
                    facts.skipped_expr_count += 1;
                    return;
                }
                let mut cursor = rhs.walk();
                for prop in rhs.children(&mut cursor) {
                    match prop.kind() {
                        "shorthand_property_identifier" => {
                            let name = self.node_text(&prop).to_string();
                            self.insert_js_ts_cjs_local(facts, Some(name), prop);
                        }
                        "pair" => {
                            let key = prop.child_by_field_name("key").map(|k| {
                                self.node_text(&k)
                                    .trim_matches(|c| c == '\'' || c == '"')
                                    .to_string()
                            });
                            let value = prop.child_by_field_name("value");
                            match (key, value) {
                                (Some(key), Some(value)) if value.kind() == "identifier" => {
                                    self.insert_js_ts_cjs_local(facts, Some(key), value);
                                }
                                (Some(key), Some(value)) => {
                                    match self.js_ts_cjs_named_expr_target(value) {
                                        Some(t) => {
                                            self.insert_js_ts_cjs_verified(facts, Some(key), t)
                                        }
                                        None => facts.skipped_expr_count += 1,
                                    }
                                }
                                _ => {}
                            }
                        }
                        "method_definition" => {
                            facts.skipped_expr_count += 1;
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                facts.skipped_expr_count += 1;
            }
        }
    }
}
