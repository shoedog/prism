//! S1b (SPEC §3.1): what a JS/TS name statically denotes. The nearest scope that declares
//! the name decides; the binding denotes one callable only when that scope holds exactly one
//! declaration whose value is a function (or an admitted React wrapper of one), the scope
//! parses cleanly or its errors are sealed (B1), and every named node kind in it is
//! classified (B0). A written binding, or a declarator whose value is any other call with a
//! function argument, is the may-call class and keeps base behavior (owner E5). A declarator
//! whose value may hold a function by value flow (anything but a function, or a literal that
//! provably holds none) is an alias, which also keeps base behavior (owner 2026-09-29).
//! Runtime mutation is out of model. S1b-3 evaluates the core at every scope (SPEC §3.1a,
//! §3.8): the site walk lives in `js_binding_walk.rs`, the per-scope index and the call-site
//! entry point in `js_binding_site.rs`, the scoped write resolver in `js_binding_writes.rs`,
//! the recovered-import predicate in `js_binding_recovery.rs`, and call-value/NoFn
//! classification in `js_binding_values.rs`.
use super::js_binding_values::{js_ts_has_default, js_ts_holds_no_function};
use super::{is_js_ts_function_like, ParsedFile};
use crate::js_exports::{JsExportFacts, JsExportTarget};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use tree_sitter::Node;

/// A proven callable: its registered name (`Language::function_name`), its line span, and
/// whether it is a wrapped React render function (bindable from JSX element sites only).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JsTerminal {
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
    /// A well-formed import binding (SPEC §3.8 (5)): the import rungs decide, uncounted.
    Import,
    /// S1b-3 (owner 2026-09-29, SPEC §3.8 (11)): a declarator whose value may hold a function
    /// by value flow (anything but a function, or a literal that provably holds none): keeps
    /// base behavior, counted `alias`.
    Alias,
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
pub(super) type NamespaceImports<'a> = BTreeMap<String, Vec<(Node<'a>, Option<String>)>>;

/// Per-file memo: scope declaration indexes (with the unknown-strictness Annex-B names), B0
/// results, both keyed by scope node id, the file-level B1 brace condition, the leave
/// predicate's namespace/enum partner counts, and M2's write-target index and `(scope, name)`
/// answers (SPEC §3.8 (2)(3), C-19, C-37).
#[derive(Default)]
pub(crate) struct JsBindingCache<'a> {
    pub(super) decls: BTreeMap<usize, (Index<'a>, BTreeSet<String>)>,
    pub(super) clean: BTreeMap<usize, Result<(), &'static str>>,
    pub(super) braces: Option<bool>,
    pub(super) partners: Option<BTreeMap<String, usize>>,
    pub(super) write_targets: Option<BTreeMap<String, Vec<Node<'a>>>>,
    pub(super) written: BTreeMap<(usize, String), bool>,
    pub(super) namespace_imports: Option<NamespaceImports<'a>>,
}

impl<'a> JsBindingCache<'a> {
    /// S1b-3b: one cache per file, shared across every call-site binding lookup in it.
    pub(crate) fn for_sites() -> Self {
        Self::default()
    }
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
    /// The module terminal (SPEC §3.1 at module scope, §3.2 step 3, §3.8 (2)): what `name`,
    /// used at `site`, denotes when the site's walk reaches the program scope. The program
    /// scope's case of `js_ts_scope_binding`.
    pub(crate) fn js_ts_module_binding<'a>(
        &'a self,
        name: &str,
        site: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        // D4 never admits `useCallback` (SPEC §3.7, §3.8 (9)): `local_route` is false.
        self.js_ts_scope_binding(self.tree.root_node(), None, name, site, cache, false)
    }

    /// What `name` denotes in `scope`, the scope the site walk stopped at (`explicit`: the
    /// `with` or dotted-namespace node that binds it), used at `site` (SPEC §3.8 (2)).
    /// `local_route` is true only for a call-site binding lookup (S1b-3b): on that route, B3's
    /// admitted-wrapper check also accepts `useCallback` as a plain callable (E4).
    pub(super) fn js_ts_scope_binding<'a>(
        &'a self,
        scope: Node<'a>,
        explicit: Option<Node<'a>>,
        name: &str,
        site: Node<'a>,
        cache: &mut JsBindingCache<'a>,
        local_route: bool,
    ) -> JsBinding {
        let (decls, annex_b) = match explicit {
            Some(d) => (Some(vec![d]), false),
            None => self.js_ts_scope_lookup(scope, name, cache),
        };
        if decls.is_none() && !annex_b {
            return JsBinding::Refused("unbound");
        }
        // A recovered-import marker (its declaring node is the `ERROR` itself, §3.8 (7)).
        if decls.iter().flatten().any(|d| d.is_error()) {
            return JsBinding::Refused("import_parse_recovery");
        }
        // B0 and B1 precede the Annex-B uncertainty, which precedes B2 (sol r1 W2).
        if let Err(reason) = self.js_ts_scope_clean(scope, cache) {
            return JsBinding::Refused(reason);
        }
        if !self.js_ts_recovery_sealed(scope, site, cache) {
            return JsBinding::Refused("parse_recovery");
        }
        match decls {
            Some(decls) if !annex_b => self.js_ts_classify(scope, &decls, name, cache, local_route),
            _ => JsBinding::Unchecked("annex_b_strictness"),
        }
    }

    /// Classification of a binding that passed B0 and B1, in the SPEC §3.1 order (as amended
    /// by §3.8 (3)(4)(5)(11)): `with`, B2, `Import`, M2 (any declaration kind), B3, M1, NoFn,
    /// then `unindexed` and `unbound`.
    fn js_ts_classify<'a>(
        &'a self,
        scope: Node<'a>,
        decls: &[Node<'a>],
        name: &str,
        cache: &mut JsBindingCache<'a>,
        local_route: bool,
    ) -> JsBinding {
        if scope.kind() == "with_statement" {
            return JsBinding::Refused("with");
        }
        let [decl] = decls else {
            return JsBinding::Refused("duplicate_declaration");
        };
        if matches!(decl.kind(), "import_statement" | "import_alias") {
            // Fold r1 (sol WRONG 2, owner M2 ruling): a written import binding keeps base on
            // the call-site route, whatever its kind — an identifier write
            // (`f = g`) is W1 syntax, not module-object mutation. D4/R4c are unaffected: an
            // exported name already filtered as an import never reaches this arm there, but
            // gate on `local_route` explicitly so a namespace/`import =` export that does
            // reach it keeps the landed `Import` answer (pinned by `d4_written_import_pin`).
            return if local_route && self.js_ts_scoped_written(scope, name, cache) {
                JsBinding::MayCall
            } else {
                JsBinding::Import
            };
        }
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
        // M2 (owner 2026-09-27, SPEC §3.8 (3)): any written binding, whatever its kind, keeps
        // base behavior. The scoped resolver replaces the module write scan and 2a's widening
        // predicates (`js_ts_written_unseen`, deleted).
        if self.js_ts_scoped_written(scope, name, cache) {
            return JsBinding::MayCall;
        }
        let callable = if function || fe_self {
            *decl
        } else if binding {
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
            // Owner 2026-09-29 (spec r2 sol W1, the conservative cut, SPEC §3.8 (11)): a
            // destructuring pattern holding a default anywhere may bind the default's value.
            let pattern = decl.child_by_field_name(if using { "left" } else { "name" });
            if !declarator && pattern.is_some_and(js_ts_has_default) {
                return JsBinding::Alias;
            }
            match value.kind() {
                "arrow_function" | "function_expression" if declarator => value,
                "call_expression" if declarator => {
                    return self.js_ts_classify_call(*decl, value, using, local_route, cache);
                }
                // M1 for a destructuring declarator (S1b-3 OQ11 a, SPEC §3.8 (4)): its names
                // may hold what a call with a function argument returns.
                "call_expression" => return super::js_binding_values::js_ts_call_may_call(value),
                _ if js_ts_holds_no_function(value) => return JsBinding::Refused("not_callable"),
                _ => return JsBinding::Alias,
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
    /// Proven callables carry their span (`wrapped` ones as S1's `SpannedLocal`); may-call and
    /// alias values keep base `Local` (E5, counted); a refusal is `UnprovenLocal`, counted by
    /// reason (an alias counts as `not_callable`, the landed 2b answer, §3.8 (11)).
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
            // D4 keeps the landed S1b-2b answer for a value-flow alias (SPEC §3.8 (11): "D4 is
            // unchanged") and, likewise, for an `Import` (fold A, Opus r1 W1): the D4 `imported`
            // set only poisons the value-import forms it recognizes (`extract_import_bindings`
            // plus type-only), so an `import f = M.g` / `import f = require(...)` alias export
            // is not filtered above and would otherwise leak an unpoisoned `Local`, letting R4c
            // bind a same-file decoy Exact — re-opening the edge S1b-2b closed. Both count
            // `not_callable`.
            JsBinding::Alias | JsBinding::Import => {
                *facts
                    .local_export_refusals
                    .entry("not_callable".into())
                    .or_default() += 1;
                JsExportTarget::UnprovenLocal(name)
            }
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
    /// (either quote form; escapes decoded to UTF-16 code units, so a surrogate pair spelled
    /// `😀` or `\u{D83D}\u{DE00}` is one scalar). `None` when the value is not
    /// well-formed Unicode (an unpaired surrogate) or an escape does not decode: the name is
    /// not matchable and callers record no claim for it (fail closed). Callers also treat a
    /// specifier with a parse error as unmatchable (a recovered `"\xGG"` is identifier `GG`).
    pub(crate) fn js_ts_module_export_name(&self, node: Node<'_>) -> Option<String> {
        if node.kind() != "string" {
            return Some(self.node_text(&node).to_string());
        }
        let is_digit = |c: char| c.is_ascii_digit();
        let mut units: Vec<u16> = Vec::new();
        let mut cursor = node.walk();
        for part in node.named_children(&mut cursor) {
            let text = self.node_text(&part);
            let Some(esc) = text
                .strip_prefix('\\')
                .filter(|_| part.kind() == "escape_sequence")
            else {
                units.extend(text.encode_utf16());
                continue;
            };
            let unit = match esc.chars().next()? {
                'u' | 'x' => u32::from_str_radix(esc[1..].trim_matches(['{', '}']), 16).ok()?,
                'n' => 0xA,
                't' => 0x9,
                'r' => 0xD,
                'b' => 0x8,
                'f' => 0xC,
                'v' => 0xB,
                // Legacy octal (`\01`, `\1`, `\0` before a digit) and `\8`/`\9` are module
                // SyntaxErrors (impl r2 sol W1): unmatchable, never NUL.
                '0' if esc.len() == 1 && !self.source[part.end_byte()..].starts_with(is_digit) => 0,
                '0'..='9' => return None,
                // A line continuation contributes nothing.
                '\n' | '\r' | '\u{2028}' | '\u{2029}' => continue,
                other => other as u32,
            };
            match u16::try_from(unit) {
                Ok(unit) => units.push(unit),
                Err(_) => units.extend(char::from_u32(unit)?.encode_utf16(&mut [0; 2]).iter()),
            }
        }
        String::from_utf16(&units).ok()
    }
}

#[cfg(test)]
#[path = "js_binding_tests.rs"]
mod tests;
