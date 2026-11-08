use crate::access_path::AccessPath;
use crate::languages::Language;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser, Tree};

#[cfg(test)]
pub(crate) mod call_membership_tests;

pub(crate) mod binding_verdict_memo;
mod call_membership;
pub(crate) mod call_names;
mod go_calls;
mod go_compat_types;
mod go_header_defs;
pub(crate) mod go_receiver_walk;
pub(crate) mod member_reference_index;

mod js_binding;
mod js_binding_alias;
pub(crate) use js_binding::{JsBinding, JsBindingCache};
pub use js_binding_alias::JsFollow;
// Public because the public JsLocalBinding variants carry JsTerminal (private_interfaces).
pub use js_binding::JsTerminal;
mod js_anonymous;
mod js_binding_checks;
mod js_binding_decls;
#[cfg(test)]
mod js_binding_helper_tests;
mod js_binding_namespace;
mod js_binding_recovery;
pub(crate) use js_binding_root::JsRootScope;
mod js_binding_returned;
mod js_binding_root;
mod js_binding_site;
mod js_binding_values;
mod js_binding_walk;
mod js_binding_writes;
mod js_cjs_anonymous;
mod js_cjs_export_barriers;
mod js_cjs_member_writes;
mod js_cjs_module_calls;
mod js_cjs_terminal;
mod js_export_memo;
#[cfg(test)]
mod js_export_work_tests;
mod js_generator;
pub(crate) mod js_language_gate;
mod js_module_forwarding;
mod js_return_facts;
mod js_wrapped_export;
mod shadow_walk;
mod singleton_rvalue_spans;
mod statement_traversal;
pub(crate) use statement_traversal::is_c_cpp_statement_kind;
mod tree_access;
pub(crate) mod tree_helpers;

mod assignments;
mod call_args;
mod calls;
mod control_flow;
mod fptr_text;
pub(crate) mod go;
mod imports;
pub mod parse_quality;
pub mod js_ts;
mod lvalues;
mod parameters;
mod python;
mod receiver_bindings;
mod receiver_typing;
mod references;
mod rvalue_identifiers;
mod rvalues;
mod value_bindings;
#[cfg(test)]
pub(crate) use fptr_text::candidate_fields_on_line;
pub(crate) use fptr_text::line_field_targets;
pub use fptr_text::{
    find_assignment_eq, resolve_array_dispatch, resolve_fptr_assignment,
    resolve_struct_field_assignment,
};

/// A parameter binding and the byte span of its identifier token.
pub type ParameterOccurrence = (String, usize, usize);

/// Count ERROR and MISSING nodes in a parse tree.
///
/// Returns `(error_count, total_nodes)` so callers can compute an error rate.
/// A high error rate indicates tree-sitter could not parse the source cleanly —
/// common with macro-heavy C/C++ code.
pub fn count_error_nodes(tree: &Tree) -> (usize, usize) {
    let mut error_count = 0usize;
    let mut total_count = 0usize;
    count_nodes_recursive(tree.root_node(), &mut error_count, &mut total_count);
    (error_count, total_count)
}

fn count_nodes_recursive(node: Node<'_>, errors: &mut usize, total: &mut usize) {
    *total += 1;
    if node.is_error() || node.is_missing() {
        *errors += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count_nodes_recursive(child, errors, total);
    }
}

fn is_js_ts_function_like(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "generator_function_declaration"
            | "generator_function"
            | "method_definition"
            | "method_signature"
            | "abstract_method_signature"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsTsReceiverBindingEvidence {
    ClassOwner,
    Materialized,
    ImportedProps {
        type_name: String,
        property: String,
        declaration_end_byte: usize,
    },
    Recovered {
        static_type: String,
        recovery: crate::resolution::ReceiverRecovery,
        declaration_end_byte: Option<usize>,
    },
}

/// Metadata for one extracted call site.
///
/// Threading this struct through the extraction paths that feed
/// `CallGraph::CallSite` construction (`function_calls_with_spans_on_lines`,
/// `function_calls_with_qualifier_and_spans_on_lines`) replaces the previous
/// unlabeled positional tuples and gives a non-grammar extraction path (Rust
/// macro-argument calls — see `crate::call_graph::rust::macro_args`) a way to override
/// `kind`/`origin` without going through `CallGraph::call_kind_at`'s ancestor
/// walk. That walk classifies ANY span nested under a `macro_invocation` as
/// `CallKind::MacroInvocation` — correct for the macro's own name/args as a
/// whole, but wrong for an ordinary value call minted from *inside* those
/// arguments (it must route through `NS_VALUE`, not `NS_MACRO`).
///
/// `kind_override`/`origin_override` are `None` for every grammar-parsed call
/// site: the caller derives `kind` via `call_kind_at` and leaves `origin` at
/// its `Source` default, exactly as before this struct existed.
#[derive(Debug, Clone)]
pub struct CallSiteMeta<'a> {
    pub callee_name: String,
    pub line: usize,
    pub qualifier: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    /// The selector operand (S3 `ReceiverClassifier` input). `None` for the
    /// simple (no-qualifier) extraction path and for manual-fallback languages.
    pub receiver_node: Option<Node<'a>>,
    pub arg_count: Option<usize>,
    pub arg_spread: bool,
    pub kind_override: Option<crate::call_graph::CallKind>,
    pub origin_override: Option<crate::call_graph::CallSiteOrigin>,
}

/// Information about a single return statement within a function.
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    /// Line number of the return statement (1-indexed).
    pub line: usize,
    /// The return value expression text, or None for bare `return`/`return;`.
    pub value_text: Option<String>,
    /// The tree-sitter node kind of the return value expression.
    pub value_kind: Option<String>,
    /// Whether this return is inside a conditional branch.
    pub is_conditional: bool,
    /// Stable identity of the return statement itself.
    pub start_byte: usize,
    pub end_byte: usize,
    /// Position-bearing return children. Go expression lists expose one entry
    /// per child; every other explicit/trailing return exposes one entry.
    pub values: Vec<ReturnValueInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnValueInfo {
    pub slot: usize,
    pub line: usize,
    pub text: String,
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// An assignment/declaration whose complete RHS value is one exact call node.
/// The LHS spans are bound to this AST parent, never recovered by line alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentCallInfo {
    pub start_byte: usize,
    pub end_byte: usize,
    pub rhs_start_byte: usize,
    pub rhs_end_byte: usize,
    pub lvalues: Vec<PathSpan>,
}

/// One function definition captured at parse time. Plain owned data: the
/// Sync-friendly seam for span-based function identity. `name == None` for
/// anonymous functions (JS/TS callback lambdas). Sequence preserves the
/// capture order of the dual-path collection (query when compiled, manual
/// walk otherwise).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: Option<String>,
    pub kind_id: u16,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize, // 1-indexed, inclusive
    pub end_line: usize,   // 1-indexed, inclusive
    /// Positional runtime slots when the signature can be represented exactly.
    /// `None` fails positional consumers closed.
    pub param_names: Option<Vec<String>>,
    /// S3: owning type for methods (bare key, generics stripped). None = free fn.
    pub owner: Option<String>,
    /// S3 (Go only): receiver variable name (`t` in `func (t *T) m()`).
    pub receiver_var: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AstNodeSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionAstSpans {
    pub name: Option<AstNodeSpan>,
    pub body: Option<AstNodeSpan>,
    pub symbol_indentation: Result<String, &'static str>,
    pub body_indentation: Result<String, &'static str>,
}

/// One argument of a call, as a source byte-span (the S2 byte-identity anchor).
/// Text is derived on demand (§3.4) so the index can later carry typed-arg info /
/// re-descend to the node without a rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CallArg {
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Per-file call-argument index, keyed by (call node start_byte, callee name).
/// Built once per file by a single pre-order walk; replaces the per-lookup
/// full-tree walk frozen as `collect_call_args_at_reference`.
#[derive(Clone, Debug, Default)]
pub(crate) struct CallArgsIndex {
    by_call: BTreeMap<(usize, String), Vec<CallArg>>,
}

/// A variable occurrence the parser located, with its real source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSpan {
    pub path: AccessPath,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Execution region used only by the rvalue-query family. Whole-file callers
/// retain the historical inventory walk; supported JS/TS/TSX callables are
/// bounded to their exact body bytes. Unknown roots fail closed rather than
/// guessing ownership from a display name or line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RvalueQueryScope {
    LegacyInventory,
    CallableBody {
        owner_start_byte: usize,
        owner_end_byte: usize,
        body_start_byte: usize,
        body_end_byte: usize,
    },
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RvalueOwnerDecision {
    Owned,
    Nested,
    Signature,
    Outside,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CallOwnerDecision {
    Owned,
    Nested,
    Outside,
    Unsupported,
    LegacyInventory,
}

/// A statement the parser located, with its real source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementSpan {
    pub line: usize,
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// The lexical branch-arm tree of one function's CFG statement universe.
///
/// Arm `0` is the function body; every other arm is one *mutually exclusive*
/// lexical alternative (an `if` consequence, an `else` clause, a loop body, a
/// `match` arm, …). Ids are allocated in depth-first **preorder**, so an arm's
/// descendants occupy the contiguous id range `(id ..= subtree_last)` — which is
/// what makes containment an integer comparison rather than a tree walk. Ids are
/// only comparable within one function: the counter restarts at every
/// `ParsedFile::statement_arms_in_function` call, and `cfg::build_function_cfg`
/// never emits an edge that leaves its function.
///
/// Built by `ParsedFile::statement_arms_in_function`; consumed by
/// `cfg::build_cfg_edges_with_arms`.
#[derive(Debug, Clone, Default)]
pub(crate) struct LexicalArms {
    /// Innermost arm containing each statement line.
    of_line: BTreeMap<usize, u32>,
    /// `subtree_last[a]` is the largest arm id allocated inside arm `a`.
    subtree_last: Vec<u32>,
}

impl LexicalArms {
    /// Innermost arm holding `line`, paired with the last id in that arm's
    /// subtree. `None` when the line is not one of the function's
    /// `statements_in_function` lines.
    pub(crate) fn interval_of_line(&self, line: usize) -> Option<(u32, u32)> {
        let arm = *self.of_line.get(&line)?;
        Some((arm, self.subtree_last[arm as usize]))
    }

    /// Innermost arm holding `line`, if any. Production code wants the interval,
    /// not the bare id; this is the arm-tree assertion helper for `cfg`'s tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn arm_of_line(&self, line: usize) -> Option<u32> {
        self.of_line.get(&line).copied()
    }

    /// How many distinct arms actually hold a statement line. Test-only: it is
    /// how `cfg`'s tests prove the channel discriminates rather than tagging
    /// everything arm 0.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn distinct_arms(&self) -> usize {
        self.of_line.values().collect::<BTreeSet<_>>().len()
    }
}

/// P5 S2 (Go func-value callbacks): which syntactic form a raw registration
/// candidate matched. Raw/unresolved — `call_graph.rs` applies target
/// resolution, the shadow gate, and per-form owner-identity/field-typing
/// gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoRegistrationForm {
    /// `Command{Run: helper}` — `struct_type_text` is the composite literal's
    /// written type (`Command` or `pkg.Command`, unresolved); `field_name` is
    /// the keyed field.
    CompositeLiteralField {
        struct_type_text: String,
        field_name: String,
    },
    /// `x.Run = helper` — `operand_name`/`field_name` name the LHS selector;
    /// `assign_line`/`assign_start_byte` anchor the assignment statement
    /// itself (used to recover the operand's type via the same
    /// `receiver_type_in_fn` machinery recovered-receiver calls use).
    FieldAssignment {
        operand_name: String,
        field_name: String,
        assign_line: usize,
        assign_start_byte: usize,
    },
    /// `Register(helper)` — a bare identifier passed as a call argument.
    /// Carries no field key; a `CallArgument` registration is always nav-only
    /// (never feeds S3).
    CallArgument,
}

/// P5 S2: one raw registration candidate found by `go_registration_candidates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoRegistrationCandidate {
    pub form: GoRegistrationForm,
    /// The bare identifier being registered (the value referencing a
    /// function, unresolved).
    pub value_name: String,
    /// 1-indexed line of `value_name`'s occurrence.
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Byte-span identity check between two nodes, used instead of `Node`'s own
/// `PartialEq` (both are well-defined, but comparing the span explicitly
/// keeps the identity check obviously robust regardless of `tree_sitter`
/// internals — the same defensive style as `reconstruct_function_node`'s
/// span comparisons above).
fn byte_range_eq(a: &Node<'_>, b: &Node<'_>) -> bool {
    a.start_byte() == b.start_byte() && a.end_byte() == b.end_byte()
}

/// P7 S1: which `@property`-family decorator classifies a Python getter —
/// `cached_property` is tracked separately so S3 call-stats can report a
/// dedicated "cached_property recorded" count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PythonPropertyKind {
    Property,
    CachedProperty,
}

/// P7 S2: one candidate `@property`/`@cached_property` LOAD access
/// (`recv.attr`) found by `python_attribute_load_candidates`. Raw/unresolved
/// — `call_graph.rs` applies the receiver-narrowing tiers (self/same-class,
/// same-file single base, or capped unknown-receiver fanout) and the S1
/// index membership check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PythonAttributeLoadCandidate {
    pub attr_name: String,
    /// `Some(text)` when the receiver ("object") is a plain identifier —
    /// lets the caller test for an exact `self` (the only narrowing
    /// receiver) vs. everything else, `cls` included (spec-review MAJOR:
    /// `cls.attr` must NOT get the same-class narrowing `self.attr` gets —
    /// class access returns the descriptor, not the getter — so it is
    /// deliberately treated the same as any other unrecognized receiver).
    pub receiver_identifier: Option<String>,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// P7 (F5): the result of `python_attribute_load_candidates` — the raw LOAD
/// candidates plus a count of store/delete-context attribute accesses whose
/// name is S1-indexed that were skipped (never a load, so never a
/// candidate): `assignment`/`augmented_assignment` LHS, `del` targets,
/// `for`/comprehension targets, and `with ... as` alias targets (F4).
/// `call_graph.rs` isn't in scope inside `ast.rs`, so this struct is how the
/// count reaches `CallGraph::property_access_store_skips`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PythonAttributeLoadScan {
    pub candidates: Vec<PythonAttributeLoadCandidate>,
    pub store_skips: usize,
}

/// Wraps a tree-sitter parse tree with helpers for slicing analysis.
#[derive(Clone)]
pub struct ParsedFile {
    pub path: String,
    pub source: String,
    pub tree: Tree,
    pub language: Language,
    /// Number of ERROR or MISSING nodes in the parse tree.
    pub parse_error_count: usize,
    /// Total number of nodes in the parse tree.
    pub parse_node_count: usize,
    /// Byte offset of each line start (0-indexed by line number).
    /// `line_offsets[i]` is the byte offset where line `i+1` begins (1-indexed lines).
    line_offsets: Vec<usize>,
    /// Lazy framework detection, populated on first call to `framework()`.
    pub framework: std::sync::OnceLock<Option<&'static crate::frameworks::FrameworkSpec>>,
    functions: Vec<FunctionInfo>,
    go_compat: go_compat_types::Cache,
    /// Lazily-built call-argument index (Task S1.5). Lazy so warm nav-cache loads,
    /// AST-only consumers, and callers with no resolved-call args pay nothing; the
    /// cold CPG/nav build that runs Step 5b builds it on demand.
    call_args: std::sync::OnceLock<CallArgsIndex>,
    /// Lazily-extracted JS/TS export facts: a pure function of the parsed
    /// file, shared by the P1 loader priming and the call-graph build.
    js_export_facts: std::sync::OnceLock<js_ts::exports::JsExportFacts>,
}

/// How a formal parameter is used in its callable body (PR-C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterUse {
    Bare,
    MemberOnly,
    Unused,
}

impl ParsedFile {
    /// Parse source code in the given language.
    pub fn parse(path: &str, source: &str, language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        let ts_language = language.tree_sitter_language();
        parser
            .set_language(&ts_language)
            .context("Failed to set language")?;
        let tree = parser
            .parse(source, None)
            .context("Failed to parse source")?;
        let (parse_error_count, parse_node_count) = count_error_nodes(&tree);
        // Precompute line→byte offset table for O(1) lookup in line_has_code_text.
        let mut line_offsets = vec![0usize]; // Line 1 starts at byte 0.
        for (i, &b) in source.as_bytes().iter().enumerate() {
            if b == b'\n' {
                line_offsets.push(i + 1);
            }
        }
        let mut pf = Self {
            path: path.to_string(),
            source: source.to_string(),
            tree,
            language,
            parse_error_count,
            parse_node_count,
            line_offsets,
            framework: std::sync::OnceLock::new(),
            functions: Vec::new(),
            go_compat: go_compat_types::Cache::default(),
            call_args: std::sync::OnceLock::new(),
            js_export_facts: std::sync::OnceLock::new(),
        };
        pf.functions = pf.build_function_table();
        Ok(pf)
    }

    /// Returns the active framework for this file, detected lazily on first call.
    /// First-match wins per `crate::frameworks::ALL_FRAMEWORKS` ordering.
    /// `None` means no framework matched (quiet mode default).
    pub fn framework(&self) -> Option<&'static crate::frameworks::FrameworkSpec> {
        *self
            .framework
            .get_or_init(|| crate::frameworks::detect_for(self))
    }

    /// Fraction of AST nodes that are ERROR or MISSING (0.0–1.0).
    pub fn error_rate(&self) -> f64 {
        if self.parse_node_count == 0 {
            return 0.0;
        }
        self.parse_error_count as f64 / self.parse_node_count as f64
    }

    /// Get text for a node.
    pub fn node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Manual recursive function collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_functions_manual<'a>(&self, node: Node<'a>, out: &mut Vec<Node<'a>>) {
        if self.is_inventoried_function(&node) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_functions_manual(child, out);
        }
    }

    /// Get the line range (1-indexed, inclusive) of a node.
    pub fn node_line_range(&self, node: &Node) -> (usize, usize) {
        (node.start_position().row + 1, node.end_position().row + 1)
    }

    /// Byte offset where 1-indexed `line` begins. Saturates to source length for
    /// out-of-range / parse-degraded lines. Best-effort anchor for line-collapsed
    /// occurrences (S2 §3).
    pub fn line_start_byte(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        self.line_offsets
            .get(line - 1)
            .copied()
            .unwrap_or(self.source.len())
    }

    /// 1-indexed line containing `byte`.
    pub fn line_for_byte(&self, byte: usize) -> usize {
        let byte = byte.min(self.source.len());
        match self.line_offsets.binary_search(&byte) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}

fn is_plain_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        && s.chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
}

/// Collect line numbers containing ERROR or MISSING nodes (up to `max` lines).
pub fn collect_error_lines(tree: &Tree, max: usize) -> Vec<usize> {
    let mut lines = BTreeSet::new();
    collect_error_lines_recursive(tree.root_node(), &mut lines, max);
    lines.into_iter().collect()
}

fn collect_error_lines_recursive(node: Node<'_>, lines: &mut BTreeSet<usize>, max: usize) {
    if lines.len() >= max {
        return;
    }
    if node.is_error() || node.is_missing() {
        lines.insert(node.start_position().row + 1);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_lines_recursive(child, lines, max);
    }
}

#[cfg(test)]
#[path = "ast_erased_rvalue_tests.rs"]
mod erased_rvalue_tests;

#[path = "ast_asserted_member.rs"]
mod asserted_member;

#[path = "ast_callback_identity.rs"]
mod callback_identity;
#[path = "ast_member_projection.rs"]
mod member_projection;
pub use crate::anon_owner::is_anonymous_owner_name as is_synthetic_owner;
pub(crate) use member_projection::FormalMemberFacts;

#[cfg(test)]
#[path = "ast_asserted_member_tests.rs"]
mod asserted_member_tests;

#[cfg(test)]
#[path = "ast_required_parameter_tests.rs"]
mod required_parameter_tests;

#[cfg(test)]
#[path = "ast_js_param_defs_tests.rs"]
mod js_param_defs_tests;

#[cfg(test)]
#[path = "ast_inert_default_parameter_tests.rs"]
mod inert_default_parameter_tests;

#[cfg(test)]
#[path = "ast_loop_header_tests.rs"]
mod loop_header_tests;

#[cfg(test)]
#[path = "ast_contained_rvalue_tests.rs"]
mod contained_rvalue_tests;

#[cfg(test)]
#[path = "ast_nested_execution_owner_tests.rs"]
mod nested_execution_owner_tests;

#[cfg(test)]
#[path = "ast_call_execution_owner_tests.rs"]
mod call_execution_owner_tests;

#[cfg(test)]
#[path = "ast_identifiers_on_line_tests.rs"]
mod identifier_row_tests;

#[cfg(test)]
mod tests;
