//! #761: a parameter-list default is read like the body statement placed at the parameter. A default
//! that is a bare identifier (`u = o`) is the body alias `const u = o`: it reads `o` bare (so the formal
//! is not member-only) and it establishes the alias `u -> o` the body alias machinery already follows.
//! Only the own parameter list is read here (nested callables have their own passes); the forms the body
//! machinery does not alias (`{ a: u } = { a: o }`, `[u] = [o]`, `u = o.x`, `u = g(o)`) are not aliases
//! here either, though a bare read in any of them still makes the formal mixed.

use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

fn is_js_ts(language: Language) -> bool {
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

/// `(binding, default value)` of one parameter-list entry that carries a default:
/// JS `assignment_pattern` and `object_assignment_pattern` (`left`/`right`), TS `required_parameter`/`optional_parameter`
/// (`pattern`/`value`).
fn default_entry<'a>(param: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    match param.kind() {
        "assignment_pattern" | "object_assignment_pattern" => Some((
            param.child_by_field_name("left")?,
            param.child_by_field_name("right")?,
        )),
        "required_parameter" | "optional_parameter" => Some((
            param.child_by_field_name("pattern")?,
            param.child_by_field_name("value")?,
        )),
        _ => None,
    }
}

impl ParsedFile {
    /// Whether the entry `entry` of `params` aliases a plain identifier: it binds a plain identifier or a
    /// destructuring pattern (`u = o`, `[u] = o`, `{ a: u } = o`) and its default is a plain identifier that no
    /// formal at or after the entry binds (a self or forward reference is a TDZ error: no value flows).
    /// Returns the binding and the target name.
    fn param_alias_of_entry<'a>(
        &self,
        params: Node<'_>,
        entry: Node<'a>,
    ) -> Option<(Node<'a>, String)> {
        let (binding, value) = default_entry(entry)?;
        if !matches!(
            binding.kind(),
            "identifier" | "object_pattern" | "array_pattern"
        ) || value.kind() != "identifier"
        {
            return None;
        }
        let target = self.node_text(&value);
        let mut cursor = params.walk();
        let later_binds = params
            .named_children(&mut cursor)
            .filter(|param| param.start_byte() >= entry.start_byte())
            .any(|param| self.js_ts_binding_pattern_may_bind(param, target));
        (!later_binds).then(|| (binding, target.to_string()))
    }

    /// The aliasing entries of the own parameter list, in source order (see `param_alias_of_entry`).
    fn js_ts_param_default_aliases<'a>(&self, func_node: &Node<'a>) -> Vec<(Node<'a>, String)> {
        if !is_js_ts(self.language) {
            return Vec::new();
        }
        let Some(params) = self.find_parameters_node(func_node) else {
            return Vec::new();
        };
        let mut cursor = params.walk();
        let found = params
            .named_children(&mut cursor)
            .filter_map(|entry| self.param_alias_of_entry(params, entry))
            .collect();
        found
    }

    /// The target of the parameter-list alias whose binding identifier is `ident`, if it is one.
    pub(crate) fn js_ts_param_default_alias_target(&self, ident: Node<'_>) -> Option<String> {
        if !is_js_ts(self.language) || ident.kind() != "identifier" {
            return None;
        }
        let entry = ident.parent()?;
        let params = entry.parent()?;
        if params.kind() != "formal_parameters" || default_entry(entry)?.0.id() != ident.id() {
            return None;
        }
        self.param_alias_of_entry(params, entry)
            .map(|(_, target)| target)
    }

    /// The target of the aliasing parameter default whose binding is the destructuring pattern `pattern`
    /// (`[u] = o`, `{ a: u } = o`), if it is one.
    pub(crate) fn js_ts_param_pattern_alias_target(&self, pattern: Node<'_>) -> Option<String> {
        if !is_js_ts(self.language) {
            return None;
        }
        let entry = pattern.parent()?;
        let params = entry.parent()?;
        if params.kind() != "formal_parameters" || default_entry(entry)?.0.id() != pattern.id() {
            return None;
        }
        self.param_alias_of_entry(params, entry)
            .map(|(_, target)| target)
    }

    /// `js_ts_param_default_alias_target` for the identifier occupying `[start, end)`.
    pub(crate) fn js_ts_param_default_alias_target_at(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<String> {
        let ident = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)?;
        self.js_ts_param_default_alias_target(ident)
    }

    /// Whether `[start, end)` is a name bound by an aliasing entry of the own parameter list (`u` of `u = o`,
    /// `u` of `{ a: u } = o`): a def this module mints, whose binding identifier is not a read of itself.
    pub(crate) fn js_ts_is_param_alias_binding_at(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> bool {
        if !is_js_ts(self.language) {
            return false;
        }
        let Some(mut node) = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
        else {
            return false;
        };
        let params = loop {
            let Some(parent) = node.parent() else {
                return false;
            };
            if parent.kind() == "formal_parameters" {
                break parent;
            }
            node = parent;
        };
        let entry = node;
        default_entry(entry).is_some_and(|(binding, _)| {
            binding.start_byte() <= start_byte
                && end_byte <= binding.end_byte()
                && self.param_alias_of_entry(params, entry).is_some()
        })
    }

    /// Alias triples `(alias, target, line)` for `collect_alias_assignments` (the named-owner alias map),
    /// from the body's own extractors: an identifier is the declarator `const u = o`, a pattern is
    /// `const [u] = o` / `const { a: u } = o`. Every def and twin of the list sits on the owner's first line
    /// with the identifier's bytes, where main puts every formal's def (a signature row other than the first
    /// maps to no CFG node, so a twin there would neither kill nor collapse with the formal's def).
    pub(crate) fn js_ts_param_default_alias_triples(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, String, usize)> {
        let start = self.node_line_range(func_node).0;
        let mut out = Vec::new();
        if !lines.contains(&start) {
            return out;
        }
        for (binding, target) in self.js_ts_param_default_aliases(func_node) {
            if binding.kind() == "identifier" {
                out.push((self.node_text(&binding).to_string(), target, start));
            } else {
                self.extract_pattern_aliases(&binding, &target, start, &mut out);
            }
        }
        out
    }

    /// The defs of the alias bindings, as the lvalue query reports a body declarator's names, on the owner's
    /// first line (see `js_ts_param_default_alias_triples`).
    pub(crate) fn js_ts_param_default_alias_spans(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let start = self.node_line_range(func_node).0;
        if !lines.contains(&start) {
            return;
        }
        for (binding, _) in self.js_ts_param_default_aliases(func_node) {
            let from = out.len();
            if binding.kind() == "identifier" {
                self.push_simple_lvalue_span(binding, out);
            } else {
                self.extract_destructuring_def_spans(&binding, out);
            }
            for span in &mut out[from..] {
                span.line = start;
            }
        }
    }

    /// Whether a default in the own parameter list reads the formal `name` bare (`u = o`, `u = g(o)`,
    /// `{ k } = { k: o }`, a computed pattern key), like the same read in the body would. Binding names,
    /// TS type annotations and member bases (`o.x`) are not bare reads. A read inside a nested callable
    /// counts, as the body's `find_bare_ref` counts it (the capture rule labels it).
    pub(crate) fn js_ts_parameter_list_reads_bare(&self, func_node: &Node<'_>, name: &str) -> bool {
        if !is_js_ts(self.language) {
            return false;
        }
        let Some(params) = self.find_parameters_node(func_node) else {
            return false;
        };
        self.param_defaults_read_bare(params, name)
    }

    fn param_defaults_read_bare(&self, node: Node<'_>, name: &str) -> bool {
        if node.kind() == "type_annotation" {
            return false;
        }
        let default = default_entry(node);
        if let Some((_, value)) = default {
            if self.default_expr_reads_bare(value, name) {
                return true;
            }
        }
        if node.kind() == "computed_property_name" && self.default_expr_reads_bare(node, name) {
            return true;
        }
        let value_id = default.map(|(_, value)| value.id());
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .filter(|child| Some(child.id()) != value_id)
            .any(|child| self.param_defaults_read_bare(child, name));
        found
    }

    /// `find_bare_ref` for a default expression, with one exception: `o[k] = v` (a subscript write through
    /// the formal) is not a read of `o`. A write through the own parameter list is `own-params`' domain
    /// (a refusal the formal's projection edges carry while it stays member-only), and the index `k` is
    /// still scanned.
    fn default_expr_reads_bare(&self, node: Node<'_>, name: &str) -> bool {
        match node.kind() {
            "subscript_expression" if self.member_is_write_target(node) => {
                let object = node.child_by_field_name("object");
                let index = node.child_by_field_name("index");
                object.is_some_and(|object| {
                    object.kind() != "identifier" && self.default_expr_reads_bare(object, name)
                }) || index.is_some_and(|index| self.default_expr_reads_bare(index, name))
            }
            "member_expression" => node.child_by_field_name("object").is_some_and(|object| {
                object.kind() != "identifier" && self.default_expr_reads_bare(object, name)
            }),
            kind if self.language.is_identifier_node(kind) => self.node_text(&node) == name,
            _ => {
                let mut cursor = node.walk();
                let found = node
                    .children(&mut cursor)
                    .any(|child| self.default_expr_reads_bare(child, name));
                found
            }
        }
    }
}
