//! S1b (SPEC §3.1): what a JS/TS name statically denotes. The nearest scope that declares
//! the name decides; the binding denotes one callable only when that scope holds exactly one
//! declaration whose value is a function (or an admitted React wrapper of one), the scope
//! parses cleanly or its errors are sealed (B1), and every named node kind in it is
//! classified (B0). A written binding, or a declarator whose value is any other call with a
//! function argument, is the may-call class and keeps base behavior (owner E5). Runtime
//! mutation is out of model. S1b-2a evaluates the core at module scope only.
#![allow(dead_code)] // S1b-2b and S1b-3 wire these; remove the allow there
use super::{is_js_ts_function_like, ParsedFile};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use tree_sitter::Node;

/// A proven callable: its registered name (`Language::function_name`), its line span, and
/// whether it is a wrapped React render function (bindable from JSX element sites only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsTerminal {
    pub(crate) local: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) wrapped: bool,
}

/// What a name denotes at a site (SPEC §3.1 classification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsBinding {
    Callable(JsTerminal),
    /// Owner E5 (M1, M2): keep base behavior.
    MayCall,
    Refused(&'static str),
    /// Owner OQ1 = (a), Option K: unproven, so base behavior, counted by reason.
    Unchecked(&'static str),
}

/// Strictness of the code holding a scope, for Annex B.3.2 (SPEC §3.1 D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Strictness {
    Strict,
    Sloppy,
    Unknown,
}

/// Declared names and their declaring nodes (markers included).
pub(super) type Index<'a> = BTreeMap<String, Vec<Node<'a>>>;

/// Per-file memo: scope declaration indexes (with the unknown-strictness Annex-B names),
/// B0 results, both keyed by scope node id, and the file-level B1 brace condition.
#[derive(Default)]
pub(crate) struct JsBindingCache<'a> {
    pub(super) decls: BTreeMap<usize, (Index<'a>, BTreeSet<String>)>,
    pub(super) clean: BTreeMap<usize, Result<(), &'static str>>,
    pub(super) braces: Option<bool>,
}

/// B0's allowlist: every named node kind, leaf or not, of the pinned JavaScript, TypeScript
/// and TSX grammars, each classified by SPEC §3.1 (`probes/grammar_closure.py --kinds-only`
/// derives it from `node-types.json` and fails on a missing or extra kind).
const CLASSIFIED: &str = "\
abstract_class_declaration abstract_method_signature accessibility_modifier \
adding_type_annotation ambient_declaration arguments array array_pattern array_type \
arrow_function as_expression asserts asserts_annotation assignment_expression \
assignment_pattern augmented_assignment_expression await_expression binary_expression \
break_statement call_expression call_signature catch_clause class class_body \
class_declaration class_heritage class_static_block comment computed_property_name \
conditional_type constraint construct_signature constructor_type continue_statement \
debugger_statement decorator default_type do_statement else_clause empty_statement \
enum_assignment enum_body enum_declaration escape_sequence existential_type export_clause \
export_specifier export_statement expression_statement extends_clause extends_type_clause \
false field_definition finally_clause flow_maybe_type for_in_statement for_statement \
formal_parameters function_declaration function_expression function_signature function_type \
generator_function generator_function_declaration generic_type hash_bang_line \
html_character_reference html_comment identifier if_statement implements_clause import \
import_alias import_attribute import_clause import_require_clause import_specifier \
import_statement index_signature index_type_query infer_type instantiation_expression \
interface_body interface_declaration internal_module intersection_type jsx_attribute \
jsx_closing_element jsx_element jsx_expression jsx_namespace_name jsx_opening_element \
jsx_self_closing_element jsx_text labeled_statement lexical_declaration literal_type \
lookup_type mapped_type_clause member_expression meta_property method_definition \
method_signature module named_imports namespace_export namespace_import nested_identifier \
nested_type_identifier new_expression non_null_expression null number object \
object_assignment_pattern object_pattern object_type omitting_type_annotation \
opting_type_annotation optional_chain optional_parameter optional_type override_modifier \
pair pair_pattern parenthesized_expression parenthesized_type predefined_type \
private_property_identifier program property_identifier property_signature \
public_field_definition readonly_type regex regex_flags regex_pattern required_parameter \
rest_pattern rest_type return_statement satisfies_expression sequence_expression \
shorthand_property_identifier shorthand_property_identifier_pattern spread_element \
statement_block statement_identifier string string_fragment subscript_expression super \
switch_body switch_case switch_default switch_statement template_literal_type \
template_string template_substitution template_type ternary_expression this this_type \
throw_statement true try_statement tuple_type type_alias_declaration type_annotation \
type_arguments type_assertion type_identifier type_parameter type_parameters type_predicate \
type_predicate_annotation type_query unary_expression undefined union_type \
update_expression variable_declaration variable_declarator while_statement with_statement \
yield_expression";

pub(super) fn classified(kind: &str) -> bool {
    static SET: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| CLASSIFIED.split_whitespace().collect())
        .contains(kind)
}

pub(super) fn is_class(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration" | "class" | "abstract_class_declaration"
    )
}

/// A node whose extent parse errors cannot move (SPEC §3.1 B1): a class, a static block, or
/// a function with a braced body.
pub(super) fn sealing(node: Node<'_>) -> bool {
    is_class(node.kind())
        || node.kind() == "class_static_block"
        || (is_js_ts_function_like(node.kind())
            && node
                .child_by_field_name("body")
                .is_some_and(|b| b.kind() == "statement_block"))
}

impl ParsedFile {
    /// The module terminal (SPEC §3.1 at module scope, §3.2 step 3): what `name`, used at
    /// `site`, denotes when the site's walk reaches the program scope.
    pub(crate) fn js_ts_module_binding<'a>(
        &'a self,
        name: &str,
        site: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        let root = self.tree.root_node();
        if !cache.decls.contains_key(&root.id()) {
            let (mut index, mut annex) = (Index::new(), BTreeSet::new());
            let mode = (true, true, self.js_ts_strictness(root));
            self.js_ts_declare_walk(root, mode, &BTreeSet::new(), &mut index, &mut annex);
            cache.decls.insert(root.id(), (index, annex));
        }
        let (index, annex) = &cache.decls[&root.id()];
        if annex.contains(name) {
            return JsBinding::Unchecked("annex_b_strictness");
        }
        let Some(decls) = index.get(name).cloned() else {
            return JsBinding::Refused("unbound");
        };
        self.js_ts_classify(root, &decls, name, site, cache)
    }

    /// Classification of a binding, in the SPEC §3.1 order: B0, B1, `with`, B2, M2, B3, M1,
    /// then `not_callable` and `unindexed`.
    fn js_ts_classify<'a>(
        &'a self,
        scope: Node<'a>,
        decls: &[Node<'a>],
        name: &str,
        site: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        if let Err(reason) = self.js_ts_scope_clean(scope, cache) {
            return JsBinding::Refused(reason);
        }
        if !self.js_ts_recovery_sealed(scope, site, cache) {
            return JsBinding::Refused("parse_recovery");
        }
        if scope.kind() == "with_statement" {
            return JsBinding::Refused("with");
        }
        let [decl] = decls else {
            return JsBinding::Refused("duplicate_declaration");
        };
        let fe_self = matches!(decl.kind(), "function_expression" | "generator_function")
            && decl
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name);
        let using = decl.kind() == "assignment_expression";
        let declarator = (decl.kind() == "variable_declarator" || using)
            && decl
                .child_by_field_name(if using { "left" } else { "name" })
                .is_some_and(|n| n.kind() == "identifier");
        let function = matches!(
            decl.kind(),
            "function_declaration" | "generator_function_declaration"
        );
        // M2. Module scope uses the base write scan after F1–F3 (SPEC §3.2, identical to the
        // scoped scan on the corpora, Q28); S1b-3 adds the scoped scan for nested scopes.
        if (fe_self || declarator || function) && self.js_ts_module_value_written(name) {
            return JsBinding::MayCall;
        }
        let callable = if function || fe_self {
            *decl
        } else if declarator {
            let field = if using { "right" } else { "value" };
            let Some(mut value) = decl.child_by_field_name(field) else {
                return JsBinding::Refused("not_callable");
            };
            // B3 unwraps parentheses, TS assertions and assignment chains (`(M.f = function(){})`).
            while let Some(inner) = match value.kind() {
                "parenthesized_expression"
                | "as_expression"
                | "satisfies_expression"
                | "non_null_expression" => value.named_child(0),
                "type_assertion" => value.named_child(value.named_child_count().saturating_sub(1)),
                "assignment_expression" => value.child_by_field_name("right"),
                _ => None,
            } {
                value = inner;
            }
            match value.kind() {
                "arrow_function" | "function_expression" => value,
                "call_expression" => return self.js_ts_classify_call(*decl, value, using),
                _ => return JsBinding::Refused("not_callable"),
            }
        } else {
            return JsBinding::Refused("not_callable");
        };
        let Some(local) = self.language.function_name(&callable) else {
            return JsBinding::Refused("unindexed");
        };
        let (start_line, end_line) = self.node_line_range(&callable);
        JsBinding::Callable(JsTerminal {
            local: self.node_text(&local).to_string(),
            start_line,
            end_line,
            wrapped: false,
        })
    }

    /// B3's admitted React wrapper (S1's predicate, wrapped) and M1: any other call with a
    /// direct function argument is may-call; a call without one holds no callable.
    fn js_ts_classify_call(&self, decl: Node<'_>, call: Node<'_>, using: bool) -> JsBinding {
        let admitted = match decl.parent() {
            Some(list) if !using => self.js_ts_wrapped_export(list, decl).ok(),
            _ => None,
        };
        if let Some((local, start_line, end_line)) = admitted {
            return JsBinding::Callable(JsTerminal {
                local,
                start_line,
                end_line,
                wrapped: true,
            });
        }
        let Some(args) = call.child_by_field_name("arguments") else {
            return JsBinding::Refused("not_callable");
        };
        let mut cursor = args.walk();
        let function_argument = args
            .named_children(&mut cursor)
            .any(|a| matches!(a.kind(), "arrow_function" | "function_expression"));
        if function_argument {
            JsBinding::MayCall
        } else {
            JsBinding::Refused("not_callable")
        }
    }

    /// The file is an ES module: a top-level `import`/`export` statement, or `.mjs`/`.mts`.
    pub(super) fn js_ts_is_module(&self) -> bool {
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        let found = root
            .named_children(&mut cursor)
            .any(|n| matches!(n.kind(), "import_statement" | "export_statement"));
        found || self.path.ends_with(".mjs") || self.path.ends_with(".mts")
    }

    /// Annex B.3.2 applies only to non-strict code (SPEC §3.1 D1). Strict: a module, a Use
    /// Strict Directive in the program's or an enclosing function's prologue, class code.
    /// Sloppy: a `.cjs` script. Unknown: any other script (package `type` or TS
    /// `alwaysStrict` decides, which prism does not read).
    pub(super) fn js_ts_strictness(&self, scope: Node<'_>) -> Strictness {
        if self.js_ts_is_module() {
            return Strictness::Strict;
        }
        let mut up = Some(scope);
        while let Some(n) = up {
            let prologue = match n.kind() {
                "program" => Some(n),
                k if is_class(k) => return Strictness::Strict,
                k if is_js_ts_function_like(k) => n.child_by_field_name("body"),
                _ => None,
            };
            if prologue.is_some_and(|p| self.js_ts_use_strict(p)) {
                return Strictness::Strict;
            }
            up = n.parent();
        }
        if self.path.ends_with(".cjs") {
            Strictness::Sloppy
        } else {
            Strictness::Unknown
        }
    }

    /// A Use Strict Directive in `body`'s directive prologue: the leading string-literal
    /// expression statements (comments are not statements), spelled without escapes.
    fn js_ts_use_strict(&self, body: Node<'_>) -> bool {
        let mut cursor = body.walk();
        for st in body.named_children(&mut cursor) {
            if matches!(st.kind(), "comment" | "hash_bang_line") {
                continue;
            }
            let Some(lit) = st
                .named_child(0)
                .filter(|l| st.kind() == "expression_statement" && l.kind() == "string")
            else {
                return false;
            };
            if matches!(self.node_text(&lit), "'use strict'" | "\"use strict\"") {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "js_binding_tests.rs"]
mod tests;
