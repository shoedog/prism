//! S1b (SPEC §3.1): what a JS/TS name statically denotes. The nearest scope that declares
//! the name decides; the binding denotes one callable only when that scope holds exactly one
//! declaration whose value is a function (or an admitted React wrapper of one), the scope
//! parses cleanly or its errors are sealed (B1), and every named node kind in it is
//! classified (B0). A written binding, or a declarator whose value is any other call with a
//! function argument, is the may-call class and keeps base behavior (owner E5). Runtime
//! mutation is out of model. S1b-2a evaluates the core at module scope only.
use super::{is_js_ts_function_like, ParsedFile};
use crate::js_exports::{JsExportFacts, JsExportTarget};
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

/// D4's per-file context: the imported locals (base `Local`, poisoned later: E9) and the memo.
pub(super) type ExportScope<'a> = (BTreeSet<String>, JsBindingCache<'a>);

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
        let (index, annex) = cache.decls.entry(root.id()).or_insert_with(|| {
            let (mut index, mut annex) = (Index::new(), BTreeSet::new());
            let mode = (true, true, self.js_ts_strictness(root));
            self.js_ts_declare_walk(root, mode, &BTreeSet::new(), &mut index, &mut annex);
            (index, annex)
        });
        let (decls, annex_b) = (index.get(name).cloned(), annex.contains(name));
        if decls.is_none() && !annex_b {
            return JsBinding::Refused("unbound");
        }
        // B0 and B1 precede the Annex-B uncertainty, which precedes B2 (sol r1 W2).
        if let Err(reason) = self.js_ts_scope_clean(root, cache) {
            return JsBinding::Refused(reason);
        }
        let Some(sealers) = self.js_ts_recovery_sealed(root, site, cache) else {
            return JsBinding::Refused("parse_recovery");
        };
        match decls {
            Some(decls) if !annex_b => self.js_ts_classify(root, &decls, name, &sealers),
            _ => JsBinding::Unchecked("annex_b_strictness"),
        }
    }

    /// Classification of a binding that passed B0 and B1 (`sealers` are B1's sealing nodes),
    /// in the SPEC §3.1 order: `with`, B2, M2, B3, M1, then `not_callable` and `unindexed`.
    fn js_ts_classify<'a>(
        &'a self,
        scope: Node<'a>,
        decls: &[Node<'a>],
        name: &str,
        sealers: &[Node<'a>],
    ) -> JsBinding {
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
        // M2 holds for any declarator or `using` (Opus r1 W3); B3 needs an identifier.
        let binding = decl.kind() == "variable_declarator" || using;
        let declarator = binding
            && decl
                .child_by_field_name(if using { "left" } else { "name" })
                .is_some_and(|n| n.kind() == "identifier");
        let function = matches!(
            decl.kind(),
            "function_declaration" | "generator_function_declaration"
        );
        // M2. Module scope uses the base write scan after F1–F3 (SPEC §3.2, identical to the
        // scoped scan on the corpora, Q28), with `using` declarations not writes (S1b-2b) and
        // widened by the writes it cannot see; S1b-3 adds the scoped scan for nested scopes.
        if (fe_self || binding || function)
            && (self.js_ts_module_written(name, true) || self.js_ts_written_unseen(name, sealers))
        {
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

    /// Writes of `name` the base scan's closer-binding test cannot see (impl r1 fold, Opus W1/W2,
    /// sol W1); counting them only moves answers toward M2, which is base behavior. (a) A write
    /// target under a position a function-like evaluates before entering its own environments
    /// (SPEC §3.1a: J1 `formal_parameters`, J2 `decorator`, J3 `computed_property_name`; its
    /// other positions hold a binding name or types, never a W1 form), which the base test
    /// shadows by the function's parameters and body `var`s. (b) An identifier spelled `name`
    /// in a B1-sealed node, whose error the base test reads as a closer binding and which may
    /// have mangled a write.
    fn js_ts_written_unseen(&self, name: &str, sealers: &[Node<'_>]) -> bool {
        let spelled = |n: Node<'_>| {
            matches!(
                n.kind(),
                "identifier"
                    | "shorthand_property_identifier"
                    | "shorthand_property_identifier_pattern"
            ) && self.node_text(&n) == name
        };
        if sealers.iter().any(|s| any_node(*s, spelled)) {
            return true;
        }
        any_node(self.tree.root_node(), |n| {
            let Some(target) = self.js_ts_write_target(n) else {
                return false;
            };
            let mut names = BTreeSet::new();
            self.collect_js_ts_binding_pattern_names(target, &mut names);
            let mut up = n.parent();
            while let Some(a) = up.filter(|a| {
                !matches!(
                    a.kind(),
                    "formal_parameters" | "decorator" | "computed_property_name"
                )
            }) {
                up = a.parent();
            }
            names.contains(name) && up.is_some()
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

impl ParsedFile {
    /// D4 (SPEC §3.2): the target an ESM local export occurrence of `name` at `site` records.
    /// Proven callables carry their span (`wrapped` ones as S1's `SpannedLocal`); may-call
    /// keeps base `Local` (E5, counted); a refusal is `UnprovenLocal`, counted by reason.
    pub(super) fn js_ts_local_export_target<'a>(
        &'a self,
        name: String,
        site: Node<'a>,
        facts: &mut JsExportFacts,
        (imported, cache): &mut ExportScope<'a>,
    ) -> JsExportTarget {
        if imported.contains(&name) {
            return JsExportTarget::Local(name);
        }
        match self.js_ts_module_binding(&name, site, cache) {
            JsBinding::Callable(t) if t.wrapped => JsExportTarget::SpannedLocal {
                local: t.local,
                start_line: t.start_line,
                end_line: t.end_line,
            },
            JsBinding::Callable(t) => JsExportTarget::VerifiedLocal {
                local: t.local,
                start_line: t.start_line,
                end_line: t.end_line,
            },
            JsBinding::MayCall => {
                facts.local_export_may_call += 1;
                JsExportTarget::Local(name)
            }
            // Unreachable at D4: an export statement makes the file a strict module.
            JsBinding::Unchecked(_) => JsExportTarget::Local(name),
            JsBinding::Refused(reason) => {
                *facts
                    .local_export_refusals
                    .entry(reason.to_string())
                    .or_default() += 1;
                JsExportTarget::UnprovenLocal(name)
            }
        }
    }

    /// ECMAScript `ModuleExportName`: an identifier's text, or a string literal's StringValue
    /// (either quote form, escape sequences decoded).
    pub(crate) fn js_ts_module_export_name(&self, node: Node<'_>) -> String {
        if node.kind() != "string" {
            return self.node_text(&node).to_string();
        }
        let mut out = String::new();
        let mut cursor = node.walk();
        for part in node.named_children(&mut cursor) {
            let text = self.node_text(&part);
            let Some(esc) = text
                .strip_prefix('\\')
                .filter(|_| part.kind() == "escape_sequence")
            else {
                out.push_str(text);
                continue;
            };
            let hex = esc[1..].trim_start_matches('{').trim_end_matches('}');
            out.extend(match esc.chars().next() {
                Some('u' | 'x') => u32::from_str_radix(hex, 16).ok().and_then(char::from_u32),
                Some('n') => Some('\n'),
                Some('t') => Some('\t'),
                Some('r') => Some('\r'),
                Some('b') => Some('\u{8}'),
                Some('f') => Some('\u{c}'),
                Some('v') => Some('\u{b}'),
                Some('0') => Some('\0'),
                // A line continuation contributes nothing.
                Some('\n' | '\r' | '\u{2028}' | '\u{2029}') => None,
                other => other,
            });
        }
        out
    }
}

/// Whether `pred` holds for `node` or any named descendant.
fn any_node<'a>(node: Node<'a>, mut pred: impl FnMut(Node<'a>) -> bool) -> bool {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if pred(n) {
            return true;
        }
        let mut cursor = n.walk();
        stack.extend(n.named_children(&mut cursor));
    }
    false
}

#[cfg(test)]
#[path = "js_binding_tests.rs"]
mod tests;
