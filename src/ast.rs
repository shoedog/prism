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
mod go;
mod imports;
mod js_ts;
mod lvalues;
mod parameters;
mod python;
mod receiver_bindings;
mod receiver_typing;
mod references;
mod rvalue_identifiers;
mod rvalues;
mod value_bindings;

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
/// macro-argument calls — see `crate::rust_macro_args`) a way to override
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
    js_export_facts: std::sync::OnceLock<crate::js_exports::JsExportFacts>,
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

    pub(crate) fn call_args_index(&self) -> &CallArgsIndex {
        self.call_args.get_or_init(|| self.build_call_args_index())
    }

    fn build_call_args_index(&self) -> CallArgsIndex {
        let mut by_call = BTreeMap::new();
        self.index_call_args(self.tree.root_node(), &mut by_call);
        CallArgsIndex { by_call }
    }

    fn index_call_args(&self, node: Node<'_>, out: &mut BTreeMap<(usize, String), Vec<CallArg>>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let key = (node.start_byte(), self.node_text(&name_node).to_string());
                // First pre-order occurrence wins — mirrors the legacy first-match.
                out.entry(key)
                    .or_insert_with(|| self.named_arg_spans(&node));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.index_call_args(child, out);
        }
    }

    fn named_arg_spans(&self, call_node: &Node<'_>) -> Vec<CallArg> {
        let mut args = Vec::new();
        if let Some(args_node) = self.language.call_arguments(call_node) {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if self.is_positional_argument_node(child) {
                    args.push(CallArg {
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                    });
                }
            }
        }
        args
    }

    /// JS/TS grammars expose comment trivia as named argument-list children.
    /// Preserve expression slots (including spread), but never give trivia an
    /// argument index. Other languages retain their existing extraction contract.
    fn is_positional_argument_node(&self, node: Node<'_>) -> bool {
        node.is_named()
            && !(matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            ) && node.kind() == "comment")
    }

    /// Derive an argument's text from its span, byte-identically to the legacy
    /// `node_text(child).trim().trim_start_matches('&')`. Mirrors `node_text`'s
    /// `utf8_text(...).unwrap_or("")` (panic-safe on a malformed span — yields "").
    fn arg_text(&self, a: &CallArg) -> String {
        // `.get(..)` (not direct `[..]`) so an out-of-range span yields "" instead of
        // panicking — matches the stated panic-safe guarantee and `node_text`'s
        // `utf8_text(...).unwrap_or("")`.
        self.source
            .as_bytes()
            .get(a.start_byte..a.end_byte)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
            .trim()
            .trim_start_matches('&')
            .to_string()
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

    /// Find condition variables in control flow statements on the given lines.
    pub fn condition_variables_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        // Condition vars are a composition: find control flow nodes on diff lines,
        // extract the condition sub-node, then collect identifiers within it.
        // The control flow node matching stays manual (too many node types per language),
        // but the identifier extraction within the condition uses the Identifiers query
        // via collect_all_identifiers (which is called on the condition sub-node).
        let mut vars = Vec::new();
        self.collect_condition_vars(*func_node, lines, &mut vars);
        vars
    }

    fn collect_condition_vars(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_control_flow_node(node.kind()) {
            if let Some(condition) = self.language.control_flow_condition(&node) {
                self.collect_all_identifiers(condition, out);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_condition_vars(child, lines, out);
        }
    }

    /// Find all return statements within a function.
    pub fn return_statements(&self, func_node: &Node<'_>) -> Vec<usize> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Returns) {
            let ret_idx = query
                .capture_index_for_name("ret")
                .expect("Returns query must have @ret capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ret_idx {
                        lines.push(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = Vec::new();
        self.collect_returns_manual(*func_node, &mut lines);
        lines
    }

    /// Manual recursive return collection (pre-query fallback).
    fn collect_returns_manual(&self, node: Node<'_>, out: &mut Vec<usize>) {
        if self.language.is_return_node(node.kind()) {
            out.push(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_returns_manual(child, out);
        }
    }

    /// Collect return statements with their value expressions.
    ///
    /// For each return statement in the function, extracts the return value
    /// expression text and node kind. Also detects Rust trailing expressions
    /// (last expression in a block without semicolon).
    pub fn return_value_nodes(&self, func_node: &Node<'_>) -> Vec<ReturnInfo> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut returns = Vec::new();
        self.collect_return_infos(func_node, &func_node, &mut returns);

        // For Rust: check for trailing expressions (last expr in block without `;`)
        if self.language == Language::Rust {
            self.collect_trailing_returns(&func_node, &mut returns);
        }

        returns
    }

    /// Recursively collect ReturnInfo from return statement nodes.
    fn collect_return_infos(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let kind = node.kind();
        if kind == "return_statement" || kind == "return_expression" {
            let line = node.start_position().row + 1;

            // Extract the return value expression (first named child that
            // isn't the `return` keyword itself).
            let mut values = Vec::new();
            let mut value_text = None;
            let mut value_kind = None;

            if kind == "return_statement" && self.language == Language::Go {
                // Go: return may have an expression_list child
                if let Some(child) = node.named_child(0) {
                    let ck = child.kind();
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(ck.to_string());
                    if ck == "expression_list" {
                        let mut cursor = child.walk();
                        for (slot, value) in child.named_children(&mut cursor).enumerate() {
                            values.push(ReturnValueInfo {
                                slot,
                                line: value.start_position().row + 1,
                                text: self.node_text(&value).to_string(),
                                kind: value.kind().to_string(),
                                start_byte: value.start_byte(),
                                end_byte: value.end_byte(),
                            });
                        }
                    } else {
                        // Single expression (not expression_list)
                        values.push(ReturnValueInfo {
                            slot: 0,
                            line: child.start_position().row + 1,
                            text: self.node_text(&child).to_string(),
                            kind: ck.to_string(),
                            start_byte: child.start_byte(),
                            end_byte: child.end_byte(),
                        });
                    }
                }
            } else {
                // All other languages: first named child is the expression
                if let Some(child) = node.named_child(0) {
                    value_text = Some(self.node_text(&child).to_string());
                    value_kind = Some(child.kind().to_string());
                    values.push(ReturnValueInfo {
                        slot: 0,
                        line: child.start_position().row + 1,
                        text: self.node_text(&child).to_string(),
                        kind: child.kind().to_string(),
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                    });
                }
            }

            let is_conditional = self.is_inside_conditional(&node, func_node);

            out.push(ReturnInfo {
                line,
                value_text,
                value_kind,
                is_conditional,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                values,
            });
            return; // Don't recurse into return children
        }

        // Don't recurse into nested callable scopes. Fence on node IDENTITY,
        // not kind: a nested definition may have the SAME kind as the enclosing
        // function, while anonymous closures may not be included in
        // `function_node_types()` at all.
        if self.language.callable_boundary_node_types().contains(&kind)
            && node.id() != func_node.id()
        {
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_return_infos(child, func_node, out);
        }
    }

    /// Check if a node is inside a conditional branch (if/else/match) within the function.
    fn is_inside_conditional(&self, node: &Node<'_>, func_node: &Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.id() == func_node.id() {
                return false;
            }
            let pk = parent.kind();
            if matches!(
                pk,
                "if_statement"
                    | "if_expression"
                    | "if_let_expression"
                    | "else_clause"
                    | "elif_clause"
                    | "match_expression"
                    | "switch_statement"
                    | "conditional_expression"
                    | "ternary_expression"
            ) {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Collect Rust trailing expressions that act as implicit returns.
    ///
    /// In Rust, the last expression in a block (without semicolon) is the
    /// return value. This detects such expressions in the function body
    /// and in if/else/match branches.
    fn collect_trailing_returns(&self, func_node: &Node<'_>, out: &mut Vec<ReturnInfo>) {
        let body = func_node.child_by_field_name("body");
        if let Some(body_node) = body {
            self.collect_block_trailing_returns(&body_node, func_node, out);
        }
    }

    fn collect_block_trailing_returns(
        &self,
        block: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let child_count = block.named_child_count();
        if child_count == 0 {
            return;
        }
        let last_child = block.named_child(child_count - 1).unwrap();
        let kind = last_child.kind();

        // If it's an expression_statement, check if it wraps an if/match expression
        // (Rust trailing if/match are often wrapped in expression_statement).
        if kind == "expression_statement" {
            // Check if it ends with a semicolon — if so, not a trailing return
            let text = self.node_text(&last_child);
            if text.ends_with(';') {
                return;
            }
            // Check children for if/match expressions
            if let Some(child) = last_child.named_child(0) {
                let ck = child.kind();
                if ck == "if_expression" || ck == "if_let_expression" {
                    self.collect_if_trailing_returns(&child, func_node, out);
                    return;
                }
                if ck == "match_expression" {
                    self.collect_match_trailing_returns(&child, func_node, out);
                    return;
                }
                // Other expression — treat as trailing return
                let line = child.start_position().row + 1;
                if !out.iter().any(|r| r.line == line) {
                    out.push(ReturnInfo {
                        line,
                        value_text: Some(self.node_text(&child).to_string()),
                        value_kind: Some(ck.to_string()),
                        is_conditional: self.is_inside_conditional(&child, func_node),
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                        values: vec![ReturnValueInfo {
                            slot: 0,
                            line,
                            text: self.node_text(&child).to_string(),
                            kind: ck.to_string(),
                            start_byte: child.start_byte(),
                            end_byte: child.end_byte(),
                        }],
                    });
                }
                return;
            }
            return;
        }

        // If it's an if_expression or match_expression, recurse into branches
        if kind == "if_expression" || kind == "if_let_expression" {
            self.collect_if_trailing_returns(&last_child, func_node, out);
            return;
        }
        if kind == "match_expression" {
            self.collect_match_trailing_returns(&last_child, func_node, out);
            return;
        }

        // Skip return_expression — already handled by collect_return_infos
        if kind == "return_expression" {
            return;
        }

        // Skip statements that aren't expressions
        if kind.ends_with("_statement")
            || kind == "let_declaration"
            || kind == "macro_invocation"
            || kind == "empty_statement"
        {
            return;
        }

        // This is a trailing expression — implicit return
        let line = last_child.start_position().row + 1;
        // Check it's not already captured as a return
        if out.iter().any(|r| r.line == line) {
            return;
        }

        let text = self.node_text(&last_child).to_string();
        let is_conditional = self.is_inside_conditional(&last_child, func_node);
        out.push(ReturnInfo {
            line,
            value_text: Some(text.clone()),
            value_kind: Some(kind.to_string()),
            is_conditional,
            start_byte: last_child.start_byte(),
            end_byte: last_child.end_byte(),
            values: vec![ReturnValueInfo {
                slot: 0,
                line,
                text,
                kind: kind.to_string(),
                start_byte: last_child.start_byte(),
                end_byte: last_child.end_byte(),
            }],
        });
    }

    fn collect_if_trailing_returns(
        &self,
        if_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        // Recurse into consequence (then block) and alternative (else block)
        if let Some(consequence) = if_node.child_by_field_name("consequence") {
            self.collect_block_trailing_returns(&consequence, func_node, out);
        }
        if let Some(alternative) = if_node.child_by_field_name("alternative") {
            let ak = alternative.kind();
            if ak == "else_clause" {
                // The else clause's child is either a block or another if_expression
                let mut cursor = alternative.walk();
                for child in alternative.named_children(&mut cursor) {
                    if child.kind() == "block" {
                        self.collect_block_trailing_returns(&child, func_node, out);
                    } else if child.kind() == "if_expression" || child.kind() == "if_let_expression"
                    {
                        self.collect_if_trailing_returns(&child, func_node, out);
                    }
                }
            } else if ak == "block" {
                self.collect_block_trailing_returns(&alternative, func_node, out);
            } else if ak == "if_expression" || ak == "if_let_expression" {
                self.collect_if_trailing_returns(&alternative, func_node, out);
            }
        }
    }

    fn collect_match_trailing_returns(
        &self,
        match_node: &Node<'_>,
        func_node: &Node<'_>,
        out: &mut Vec<ReturnInfo>,
    ) {
        let mut cursor = match_node.walk();
        for child in match_node.named_children(&mut cursor) {
            if child.kind() == "match_arm" {
                // The value of a match arm is its last named child (after the `=>`)
                let arm_count = child.named_child_count();
                if arm_count > 0 {
                    let arm_value = child.named_child(arm_count - 1).unwrap();
                    let vk = arm_value.kind();
                    if vk == "block" {
                        self.collect_block_trailing_returns(&arm_value, func_node, out);
                    } else if vk != "match_pattern" {
                        let line = arm_value.start_position().row + 1;
                        if !out.iter().any(|r| r.line == line) {
                            out.push(ReturnInfo {
                                line,
                                value_text: Some(self.node_text(&arm_value).to_string()),
                                value_kind: Some(vk.to_string()),
                                is_conditional: true,
                                start_byte: arm_value.start_byte(),
                                end_byte: arm_value.end_byte(),
                                values: vec![ReturnValueInfo {
                                    slot: 0,
                                    line,
                                    text: self.node_text(&arm_value).to_string(),
                                    kind: vk.to_string(),
                                    start_byte: arm_value.start_byte(),
                                    end_byte: arm_value.end_byte(),
                                }],
                            });
                        }
                    }
                }
            }
        }
    }

    /// Find all goto statements in a function node.
    /// Returns `(target_label, goto_line)` pairs.
    pub fn goto_statements(&self, func_node: &Node<'_>) -> Vec<(String, usize)> {
        let mut gotos = Vec::new();
        self.collect_gotos(*func_node, &mut gotos);
        gotos
    }

    fn collect_gotos(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "goto_statement" {
            // tree-sitter-c: goto_statement has a "label" field with the target name
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_gotos(child, out);
        }
    }

    /// Find all label definitions in a function node.
    /// Returns `(label_name, label_line, section_end_line)` triples.
    /// `section_end_line` is the line of the next label or the function end,
    /// representing the code section "owned" by this label.
    pub fn label_sections(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize)> {
        let mut labels: Vec<(String, usize)> = Vec::new();
        self.collect_labels(*func_node, &mut labels);

        let func_end = func_node.end_position().row + 1;

        // Sort labels by line number to determine sections
        labels.sort_by_key(|(_, line)| *line);

        let mut sections = Vec::new();
        for i in 0..labels.len() {
            let (ref name, start) = labels[i];
            let end = if i + 1 < labels.len() {
                labels[i + 1].1.saturating_sub(1)
            } else {
                func_end
            };
            sections.push((name.clone(), start, end));
        }
        sections
    }

    fn collect_labels(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        if node.kind() == "labeled_statement" {
            // tree-sitter-c: labeled_statement has a "label" field
            if let Some(label_node) = node.child_by_field_name("label") {
                let label = self.node_text(&label_node).to_string();
                let line = node.start_position().row + 1;
                out.push((label, line));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_labels(child, out);
        }
    }

    /// Classify labels as cleanup-only (reachable only via goto) or
    /// flow-through (also reachable from sequential execution).
    ///
    /// A label is cleanup-only if the statement immediately preceding it
    /// is a flow-terminating statement (return, goto, break, continue).
    /// If the preceding statement is normal code, sequential execution
    /// falls through into the label — it's part of the normal path.
    ///
    /// Returns `(name, start, end, is_cleanup_only)` tuples.
    pub fn classify_labels(&self, func_node: &Node<'_>) -> Vec<(String, usize, usize, bool)> {
        let sections = self.label_sections(func_node);
        let (func_start, _func_end) = self.node_line_range(func_node);

        sections
            .into_iter()
            .map(|(name, start, end)| {
                let is_cleanup = if start <= func_start + 1 {
                    // Label at the very beginning of the function — flow-through
                    false
                } else {
                    // Check lines immediately before the label for a flow terminator.
                    // We scan backwards from label_start-1 to skip blank/brace lines.
                    let mut found_terminator = false;
                    for check_line in (func_start..start).rev() {
                        let row = check_line.saturating_sub(1);
                        if self.find_flow_terminator(self.tree.root_node(), row) {
                            found_terminator = true;
                            break;
                        }
                        // Check if this line has any real code (not just whitespace/braces)
                        if let Some(line_str) = self.source.lines().nth(row) {
                            let trimmed = line_str.trim();
                            if !trimmed.is_empty() && trimmed != "}" && trimmed != "{" {
                                // Found non-empty, non-brace code that isn't a terminator
                                break;
                            }
                        }
                    }
                    found_terminator
                };
                (name, start, end, is_cleanup)
            })
            .collect()
    }

    /// Check if a given row (0-indexed) contains a flow-terminating statement.
    fn find_flow_terminator(&self, node: Node<'_>, row: usize) -> bool {
        if node.start_position().row == row
            && matches!(
                node.kind(),
                "return_statement" | "goto_statement" | "break_statement" | "continue_statement"
            )
        {
            return true;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.start_position().row <= row && child.end_position().row >= row {
                if self.find_flow_terminator(child, row) {
                    return true;
                }
            }
        }
        false
    }

    /// Partition a function's lines into normal-path lines and cleanup label sections.
    ///
    /// Returns `(normal_lines, label_lines)` where:
    /// - `normal_lines` are lines NOT in any cleanup-only label section
    /// - `label_lines` maps each cleanup-only label name to its line range
    ///
    /// Labels reachable via fall-through (no preceding flow terminator) are
    /// considered part of the normal path. Only cleanup-only labels (preceded
    /// by return/goto/break/continue) are separated out.
    pub fn partition_by_labels(
        &self,
        func_node: &Node<'_>,
    ) -> (Vec<usize>, BTreeMap<String, Vec<usize>>) {
        let (func_start, func_end) = self.node_line_range(func_node);
        let classified = self.classify_labels(func_node);
        let cleanup_labels: Vec<_> = classified
            .iter()
            .filter(|(_, _, _, is_cleanup)| *is_cleanup)
            .collect();

        if cleanup_labels.is_empty() {
            let normal: Vec<usize> = (func_start..=func_end).collect();
            return (normal, BTreeMap::new());
        }

        // Build a set of all lines in cleanup-only label sections
        let mut cleanup_line_set = std::collections::BTreeSet::new();
        let mut label_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (name, start, end, _) in &cleanup_labels {
            let lines: Vec<usize> = (*start..=*end).collect();
            cleanup_line_set.extend(lines.iter().copied());
            label_map.insert(name.clone(), lines);
        }

        let normal: Vec<usize> = (func_start..=func_end)
            .filter(|l| !cleanup_line_set.contains(l))
            .collect();

        (normal, label_map)
    }

    /// Get all lines reachable from a `goto label`, including fall-through
    /// to subsequent labels (unless a `return` breaks the fall-through).
    ///
    /// In the cascading kernel cleanup pattern:
    /// ```c
    ///   err_dev:
    ///       kfree(dev);          // reachable from goto err_dev
    ///   err_buf:                 // falls through from err_dev (no return above)
    ///       kfree(buf);          // also reachable from goto err_dev
    ///       return -1;
    /// ```
    ///
    /// `goto err_dev` reaches both sections. `goto err_buf` reaches only err_buf.
    pub fn lines_reachable_from_goto(
        &self,
        func_node: &Node<'_>,
        target_label: &str,
    ) -> Vec<usize> {
        let label_secs = self.label_sections(func_node);
        let returns = self.return_statements(func_node);
        let return_set: std::collections::BTreeSet<usize> = returns.into_iter().collect();

        let mut reachable = Vec::new();
        let mut found_target = false;

        for (name, start, end) in &label_secs {
            if name == target_label {
                found_target = true;
            }
            if !found_target {
                continue;
            }

            reachable.extend(*start..=*end);

            // Check if this section contains a return — if so, fall-through stops
            if (*start..=*end).any(|l| return_set.contains(&l)) {
                break;
            }
        }

        reachable
    }

    /// Find the enclosing control flow block (if/for/while) for a given line,
    /// and return its start and end lines.
    pub fn enclosing_branch(&self, line: usize) -> Option<(usize, usize)> {
        let row = line.saturating_sub(1);
        self.find_enclosing_branch(self.tree.root_node(), row)
    }

    fn find_enclosing_branch(&self, node: Node<'_>, row: usize) -> Option<(usize, usize)> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_branch(child, row) {
                return Some(found);
            }
        }

        if self.language.is_control_flow_node(node.kind()) {
            Some((start + 1, end + 1))
        } else {
            None
        }
    }

    /// Find function calls on the given lines and return the called function names.
    pub fn function_calls_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx && membership.is_owned(capture.node) {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.call_function_name(&capture.node) {
                                let name = self.node_text(&name_node).to_string();
                                calls.push((name, line));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual(*func_node, lines, &mut calls);
        calls
    }

    /// Find function calls on the given lines and return the called function
    /// names plus the call node's byte range.
    ///
    /// Rust also mints value calls found INSIDE a transparent macro's
    /// arguments (`assert!(check(x))`) — see `crate::rust_macro_args`. Those
    /// entries carry `kind_override`/`origin_override` (`Call`/`MacroArg`);
    /// every grammar-parsed entry leaves both `None`, unchanged from before
    /// this struct existed. `macro_shadow` is the repo-wide macro-name shadow
    /// set (`crate::rust_macro_args::collect_macro_shadow_set`) — pass
    /// `&BTreeSet::new()` for non-Rust callers or callers with no shadow facts.
    pub fn function_calls_with_spans_on_lines<'a>(
        &'a self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        macro_shadow: &BTreeSet<String>,
    ) -> Vec<CallSiteMeta<'a>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index != call_idx {
                        continue;
                    }
                    if !membership.is_owned(capture.node) {
                        continue;
                    }
                    let line = capture.node.start_position().row + 1;
                    if !lines.contains(&line) {
                        continue;
                    }
                    if self.language == crate::languages::Language::Rust
                        && capture.node.kind() == "macro_invocation"
                    {
                        let (sites, _facts) =
                            crate::rust_macro_args::extract_calls(self, capture.node, macro_shadow);
                        calls.extend(sites);
                        continue;
                    }
                    if let Some(name_node) = self.call_function_name(&capture.node) {
                        let name = self.node_text(&name_node).to_string();
                        calls.push(CallSiteMeta {
                            callee_name: name,
                            line,
                            qualifier: None,
                            start_byte: capture.node.start_byte(),
                            end_byte: capture.node.end_byte(),
                            receiver_node: None,
                            arg_count: None,
                            arg_spread: false,
                            kind_override: None,
                            origin_override: None,
                        });
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_spans(*func_node, lines, &mut calls);
        calls
    }

    /// Like `function_calls_on_lines`, but also extracts the module/object qualifier.
    /// Returns `(callee_name, line, qualifier)` tuples.
    pub fn function_calls_on_lines_with_qualifier(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize, Option<String>)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx && membership.is_owned(capture.node) {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.call_function_name(&capture.node) {
                                let name = self.node_text(&name_node).to_string();
                                let qualifier = self
                                    .call_function_qualifier(&capture.node)
                                    .map(|q| self.node_text(&q).to_string());
                                calls.push((name, line, qualifier));
                            }
                        }
                    }
                }
            }
            return calls;
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_qualifier(*func_node, lines, &mut calls);
        calls
    }

    /// Like `function_calls_on_lines_with_qualifier`, but also returns the call
    /// node's byte range plus the qualifier/receiver node, wrapped in
    /// `CallSiteMeta`. `receiver_node` (the selector operand — e.g. the
    /// `type_assertion_expression` in `x.(Module).M()`) feeds the S3
    /// `ReceiverClassifier`. It is surfaced on the query path only; the
    /// manual fallback yields `None` (type-assertion recovery is Go-only, and
    /// Go uses the query path).
    ///
    /// Rust also mints value calls found INSIDE a transparent macro's
    /// arguments (`assert!(check(x))` / `assert!(v.contains(x))`) — see
    /// `crate::rust_macro_args`. The returned `MacroArgFacts` aggregates
    /// telemetry for every macro invocation encountered on `lines` in this
    /// one call (the caller sums these per file). `macro_shadow` is the
    /// repo-wide macro-name shadow set
    /// (`crate::rust_macro_args::collect_macro_shadow_set`) — pass
    /// `&BTreeSet::new()` for non-Rust callers or callers with no shadow facts.
    pub fn function_calls_with_qualifier_and_spans_on_lines<'a>(
        &'a self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        macro_shadow: &BTreeSet<String>,
    ) -> (Vec<CallSiteMeta<'a>>, crate::rust_macro_args::MacroArgFacts) {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let mut facts = crate::rust_macro_args::MacroArgFacts::default();

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let membership = call_membership::CallMembership::new(self, *func_node);
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index != call_idx {
                        continue;
                    }
                    if !membership.is_owned(capture.node) {
                        continue;
                    }
                    let line = capture.node.start_position().row + 1;
                    if !lines.contains(&line) {
                        continue;
                    }
                    if self.language == crate::languages::Language::Rust
                        && capture.node.kind() == "macro_invocation"
                    {
                        let (sites, site_facts) =
                            crate::rust_macro_args::extract_calls(self, capture.node, macro_shadow);
                        calls.extend(sites);
                        facts.calls_recorded += site_facts.calls_recorded;
                        facts.skipped_macros += site_facts.skipped_macros;
                        facts.ctor_skips += site_facts.ctor_skips;
                        continue;
                    }
                    if let Some(name_node) = self.call_function_name(&capture.node) {
                        let name = self.node_text(&name_node).to_string();
                        let qualifier_node = self.call_function_qualifier(&capture.node);
                        let qualifier = qualifier_node.map(|q| self.node_text(&q).to_string());
                        let (arg_count, arg_spread) = self
                            .language
                            .call_arguments(&capture.node)
                            .map(|args| {
                                let mut count = 0usize;
                                let mut spread = false;
                                let mut cursor2 = args.walk();
                                for child in args.children(&mut cursor2) {
                                    if !self.is_positional_argument_node(child) {
                                        continue; // punctuation and JS/TS comment trivia
                                    }
                                    if child.kind() == "variadic_argument" {
                                        spread = true;
                                    }
                                    count += 1;
                                }
                                (Some(count), spread)
                            })
                            .unwrap_or((None, false));
                        calls.push(CallSiteMeta {
                            callee_name: name,
                            line,
                            qualifier,
                            start_byte: capture.node.start_byte(),
                            end_byte: capture.node.end_byte(),
                            receiver_node: qualifier_node,
                            arg_count,
                            arg_spread,
                            kind_override: None,
                            origin_override: None,
                        });
                    }
                }
            }
            return (calls, facts);
        }

        let mut calls = Vec::new();
        self.collect_calls_manual_with_qualifier_and_spans(*func_node, lines, &mut calls);
        (calls, facts)
    }

    fn collect_calls_manual_with_qualifier_and_spans<'a>(
        &'a self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        self.collect_calls_manual_with_qualifier_and_spans_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_qualifier_and_spans_for_owner<'a>(
        &'a self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                // Manual fallback surfaces no receiver node (None); type-assertion
                // recovery is Go-only and Go uses the query path above.
                // arg_count/arg_spread left as None/false — arity filter treats None as keep.
                out.push(CallSiteMeta {
                    callee_name: name,
                    line,
                    qualifier,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    receiver_node: None,
                    arg_count: None,
                    arg_spread: false,
                    kind_override: None,
                    origin_override: None,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier_and_spans_for_owner(owner, child, lines, out);
        }
    }

    fn collect_calls_manual_with_qualifier(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        self.collect_calls_manual_with_qualifier_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_qualifier_for_owner(
        &self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                out.push((name, line, qualifier));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier_for_owner(owner, child, lines, out);
        }
    }

    fn collect_calls_manual_with_spans<'a>(
        &'a self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        self.collect_calls_manual_with_spans_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_with_spans_for_owner<'a>(
        &'a self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push(CallSiteMeta {
                    callee_name: name,
                    line,
                    qualifier: None,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    receiver_node: None,
                    arg_count: None,
                    arg_spread: false,
                    kind_override: None,
                    origin_override: None,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_spans_for_owner(owner, child, lines, out);
        }
    }

    /// Manual recursive call collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_calls_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        self.collect_calls_manual_for_owner(node, node, lines, out);
    }

    fn collect_calls_manual_for_owner(
        &self,
        owner: Node<'_>,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && self.language.is_call_node(node.kind())
            && self.call_capture_is_owned(&owner, node)
        {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_for_owner(owner, child, lines, out);
        }
    }

    /// Find a function definition by name.
    pub fn find_function_by_name(&self, name: &str) -> Option<Node<'_>> {
        self.find_function_by_name_inner(self.tree.root_node(), name)
    }

    fn find_function_by_name_inner<'a>(&self, node: Node<'a>, name: &str) -> Option<Node<'a>> {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            if let Some(name_node) = self.language.function_name(&node) {
                if self.node_text(&name_node) == name {
                    return Some(node);
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_function_by_name_inner(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Extract the Nth argument expression text from a call expression on a given line.
    ///
    /// Searches for call expressions on the specified line, then returns the text
    /// of the argument at `arg_index` (0-based).
    pub fn call_argument_text_at(
        &self,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        self.find_call_arg_at(self.tree.root_node(), line, callee_name, arg_index)
    }

    fn find_call_arg_at(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> Option<String> {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        // Count expression children, excluding JS/TS comment trivia.
                        let mut arg_idx = 0;
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                if arg_idx == arg_index {
                                    let text = self.node_text(&child).trim().to_string();
                                    // Strip address-of operator
                                    let text = text.trim_start_matches('&').to_string();
                                    return Some(text);
                                }
                                arg_idx += 1;
                            }
                        }
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(result) = self.find_call_arg_at(child, line, callee_name, arg_index) {
                return Some(result);
            }
        }
        None
    }

    /// Extract all argument texts from a call to `callee_name` on the given line.
    ///
    /// Returns a list of argument expressions as strings, preserving positional order.
    /// Used by interprocedural data flow to map arguments to parameters.
    pub fn call_argument_texts(&self, line: usize, callee_name: &str) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args(self.tree.root_node(), line, callee_name, &mut args);
        args
    }

    /// Like `call_argument_texts`, but selects the call expression whose start
    /// byte == `start_byte` (disambiguates multiple calls on one line).
    pub fn call_argument_texts_at(&self, start_byte: usize, callee_name: &str) -> Vec<String> {
        self.call_argument_texts_and_spans_at(start_byte, callee_name)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    /// Like `call_argument_texts_at`, while retaining each argument node's
    /// half-open source-byte span. The text-only API remains a wrapper because
    /// its consumers intentionally do not need occurrence identity.
    pub(crate) fn call_argument_texts_and_spans_at(
        &self,
        start_byte: usize,
        callee_name: &str,
    ) -> Vec<(String, std::ops::Range<usize>)> {
        self.call_args_index()
            .by_call
            .get(&(start_byte, callee_name.to_string()))
            .map(|spans| {
                spans
                    .iter()
                    .map(|a| (self.arg_text(a), a.start_byte..a.end_byte))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    fn call_argument_texts_at_reference(
        &self,
        start_byte: usize,
        callee_name: &str,
    ) -> Vec<String> {
        let mut args = Vec::new();
        self.collect_call_args_at_reference(
            self.tree.root_node(),
            start_byte,
            callee_name,
            &mut args,
        );
        args
    }

    fn collect_call_args(
        &self,
        node: Node<'_>,
        line: usize,
        callee_name: &str,
        out: &mut Vec<String>,
    ) {
        let node_line = node.start_position().row + 1;

        if node_line == line && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                let text = self.node_text(&child).trim().to_string();
                                let text = text.trim_start_matches('&').to_string();
                                out.push(text);
                            }
                        }
                    }
                    return; // Found the call, stop.
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !out.is_empty() {
                return;
            }
            self.collect_call_args(child, line, callee_name, out);
        }
    }

    #[cfg(test)]
    fn collect_call_args_at_reference(
        &self,
        node: Node<'_>,
        start_byte: usize,
        callee_name: &str,
        out: &mut Vec<String>,
    ) {
        if node.start_byte() == start_byte && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if self.is_positional_argument_node(child) {
                                let text = self.node_text(&child).trim().to_string();
                                let text = text.trim_start_matches('&').to_string();
                                out.push(text);
                            }
                        }
                    }
                    return; // Found the call, stop.
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !out.is_empty() {
                return;
            }
            self.collect_call_args_at_reference(child, start_byte, callee_name, out);
        }
    }

    /// Check if a function node has a variadic parameter (`...`).
    ///
    /// In C/C++, tree-sitter represents `...` as a `variadic_parameter` node
    /// inside the parameter list.
    pub fn is_variadic_function(&self, func_node: &Node<'_>) -> bool {
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "variadic_parameter" {
                    return true;
                }
            }
        }
        false
    }

    /// Find all call expression names inside a function body.
    /// Returns deduplicated list of callee names.
    pub fn callees_in_function(&self, func_node: &Node<'_>) -> Vec<String> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut names = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        if let Some(name_node) = self.call_function_name(&capture.node) {
                            names.insert(self.node_text(&name_node).to_string());
                        }
                    }
                }
            }
            return names.into_iter().collect();
        }

        let mut names = BTreeSet::new();
        self.collect_all_callees_manual(*func_node, &mut names);
        names.into_iter().collect()
    }

    fn collect_all_callees_manual(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.call_function_name(&node) {
                out.insert(self.node_text(&name_node).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_callees_manual(child, out);
        }
    }
}

/// Scan the function-source prefix before a call for exactly one assignment of
/// the form `var_name = func_name`, where `func_name` is a known function.
/// Any second assignment or non-function RHS fails closed.
///
/// Used by call graph Level 1 to resolve local function pointer variables.
pub fn resolve_fptr_assignment(
    func_source: &str,
    var_name: &str,
    known_fns: &BTreeSet<String>,
) -> Option<String> {
    let mut resolved = None;
    let mut saw_assignment = false;
    for line in func_source.lines() {
        let trimmed = line.trim();
        // Match: var_name = identifier  (with optional trailing semicolon/comma)
        // Also match initialization: type (*var_name)(...) = identifier;
        //   and typedef:  callback_fn var_name = identifier;

        // Strategy: find `var_name` followed by `=` (but not `==`), then extract RHS identifier
        if let Some(eq_pos) = find_assignment_eq(trimmed) {
            let lhs = trimmed[..eq_pos].trim().trim_end_matches(':').trim();
            let rhs = trimmed[eq_pos + 1..].trim().trim_end_matches(';').trim();

            // Check if LHS contains var_name as the assigned variable
            // Handles: `var_name =`, `type var_name =`, `type (*var_name)(args) =`
            let lhs_has_var = lhs == var_name
                || lhs.ends_with(&format!(" {}", var_name))
                || lhs.ends_with(&format!("*{}", var_name))
                || lhs.contains(&format!("(*{})", var_name))
                || lhs.contains(&format!(" {} ", var_name));

            if !lhs_has_var {
                continue;
            }

            if saw_assignment {
                return None;
            }
            saw_assignment = true;

            // RHS should be a plain identifier (possibly with & prefix for address-of)
            let rhs_name = rhs.trim_start_matches('&');
            if !rhs_name.is_empty()
                && rhs_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && known_fns.contains(rhs_name)
            {
                resolved = Some(rhs_name.to_string());
            } else {
                return None;
            }
        }
    }
    resolved
}

/// Scan a function (or file-scope) source for an array initializer of the form:
///   `type array_name[] = { func_a, func_b, func_c };`
/// Returns all known function names that appear in the initializer list.
///
/// Used by call graph Phase 3 to resolve dispatch table calls.
pub fn resolve_array_dispatch(
    source: &str,
    array_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = Vec::new();

    // Find the array initializer: look for `array_name` followed by `[` ... `]` ... `=` ... `{`
    // This is a text heuristic — we look for lines containing the array name and an initializer
    let lines: Vec<&str> = source.lines().collect();
    let mut in_initializer = false;
    let mut brace_depth = 0;

    for line in &lines {
        let trimmed = line.trim();

        if !in_initializer {
            // Look for: array_name ... [] = { or array_name ... [N] = {
            if trimmed.contains(array_name) && trimmed.contains('[') && trimmed.contains('=') {
                in_initializer = true;
                // Count braces on this line
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth <= 0 {
                                in_initializer = false;
                            }
                        }
                        _ => {}
                    }
                }
                // Extract identifiers from this line's initializer portion
                if let Some(brace_start) = trimmed.find('{') {
                    extract_fn_names_from_init(&trimmed[brace_start..], known_fns, &mut targets);
                }
                continue;
            }
        }

        if in_initializer {
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth <= 0 {
                            in_initializer = false;
                        }
                    }
                    _ => {}
                }
            }
            extract_fn_names_from_init(trimmed, known_fns, &mut targets);
        }
    }

    targets
}

/// Extract known function names from an initializer fragment like `{func_a, func_b, .field = func_c}`.
fn extract_fn_names_from_init(text: &str, known_fns: &BTreeSet<String>, out: &mut Vec<String>) {
    // Split on commas and braces, then check each token
    for token in text.split(|c: char| c == ',' || c == '{' || c == '}' || c == '(' || c == ')') {
        let token = token.trim();
        // Handle designated initializers: `.field = func_name`
        let ident = if let Some(eq_pos) = token.find('=') {
            let rhs = token[eq_pos + 1..].trim();
            // Skip ==
            if token.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                continue;
            }
            rhs
        } else {
            token
        };
        let ident = ident.trim_start_matches('&');
        if !ident.is_empty()
            && ident.chars().all(|c| c.is_alphanumeric() || c == '_')
            && known_fns.contains(ident)
            && !out.contains(&ident.to_string())
        {
            out.push(ident.to_string());
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

/// Find the position of an assignment `=` in a trimmed line, skipping `==`, `!=`, `<=`, `>=`.
/// Returns `None` if no plain assignment is found.
pub fn find_assignment_eq(trimmed: &str) -> Option<usize> {
    if let Some(eq_pos) = trimmed.find('=') {
        // Skip ==
        if eq_pos + 1 < trimmed.len() {
            if trimmed.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                return None;
            }
        }
        // Skip !=, <=, >=
        if eq_pos > 0 {
            let before = trimmed.as_bytes().get(eq_pos - 1);
            if before == Some(&b'!') || before == Some(&b'<') || before == Some(&b'>') {
                return None;
            }
        }
        Some(eq_pos)
    } else {
        None
    }
}

/// The legacy per-line, per-field matcher — extracted verbatim from
/// resolve_struct_field_assignment so the Level-4 index (call_graph) and the
/// legacy oracle share ONE core. Quirks (arrow-anywhere priority,
/// prefix-consumption, single-= anchor, RHS stop set, &-strip) are pinned by
/// the level4_legacy_* tests and are CONTRACT until retired by a separate
/// measured change.
pub(crate) fn line_field_targets(
    trimmed: &str,
    field_name: &str,
    known_fns: &BTreeSet<String>,
    targets: &mut BTreeSet<String>,
) {
    let arrow_pattern = format!("->{}", field_name);
    let dot_pattern = format!(".{}", field_name);
    let mut search_from = 0usize;
    while search_from < trimmed.len() {
        let field_pos = trimmed[search_from..]
            .find(&arrow_pattern)
            .map(|p| (p + search_from, arrow_pattern.len()))
            .or_else(|| {
                trimmed[search_from..]
                    .find(&dot_pattern)
                    .map(|p| (p + search_from, dot_pattern.len()))
            });
        let (pos, pat_len) = match field_pos {
            Some(v) => v,
            None => break,
        };
        let after_field = pos + pat_len;
        search_from = after_field;
        let rest = trimmed[after_field..].trim_start();
        if !rest.starts_with('=') || rest.starts_with("==") {
            continue;
        }
        let rhs = rest[1..].trim();
        let rhs_end = rhs
            .find(|c: char| c == ';' || c == ',' || c == '}' || c == ')' || c.is_whitespace())
            .unwrap_or(rhs.len());
        let rhs_token = rhs[..rhs_end].trim().trim_start_matches('&');
        if !rhs_token.is_empty()
            && rhs_token.chars().all(|c| c.is_alphanumeric() || c == '_')
            && known_fns.contains(rhs_token)
        {
            targets.insert(rhs_token.to_string());
        }
    }
}

/// Every maximal identifier immediately preceded by `->` or `.` on the line,
/// under the PINNED predicate: char::is_alphanumeric(c) || c == '_' — the same
/// Unicode-aware class the Level-4 call-site filter applies to callee names —
/// scanned over RAW source, including comments and strings (legacy semantics).
///
/// Completeness: a field can only produce a target when the accessor occurrence
/// is followed (after optional whitespace) by `=`, which terminates the
/// identifier run. Prefix occurrences (`->cbx` when querying `cb`) produce no
/// targets; the legacy core retains their consumption side-effects.
#[cfg(test)]
pub(crate) fn candidate_fields_on_line(trimmed: &str) -> BTreeSet<String> {
    fn ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    let mut out = BTreeSet::new();
    let mut rest = trimmed;
    loop {
        let arrow = rest.find("->");
        let dot = rest.find('.');
        let (pos, len) = match (arrow, dot) {
            (Some(a), Some(d)) if a <= d => (a, 2),
            (Some(a), None) => (a, 2),
            (_, Some(d)) => (d, 1),
            (None, None) => break,
        };
        let after = &rest[pos + len..];
        let end = after.find(|c: char| !ident_char(c)).unwrap_or(after.len());
        if end > 0 {
            out.insert(after[..end].to_string());
            rest = &after[end..];
        } else {
            rest = after;
        }
    }
    out
}

/// Scan source text for struct field assignments that assign a known function
/// to a field with the given name.
/// Post-S1 this is the differential-test oracle; production Level-4 resolution
/// uses the inverted index in call_graph.rs.
///
/// Matches patterns:
///   `anything->field_name = known_func;`
///   `anything.field_name = known_func;`
///   `.field_name = known_func` (designated initializer)
///
/// Returns all unique known function names found assigned to this field.
pub fn resolve_struct_field_assignment(
    source: &str,
    field_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = BTreeSet::new();
    let arrow_pattern = format!("->{}", field_name);
    let dot_pattern = format!(".{}", field_name);
    for line in source.lines() {
        let trimmed = line.trim();
        if !(trimmed.contains(&arrow_pattern) || trimmed.contains(&dot_pattern)) {
            continue;
        }
        line_field_targets(trimmed, field_name, known_fns, &mut targets);
    }
    targets.into_iter().collect()
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
mod tests {
    use super::*;

    fn fns(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn descendant_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = descendant_of_kind(child, kind) {
                return Some(found);
            }
        }
        None
    }

    fn assert_statement_markers(path: &str, source: &str, language: Language, markers: &[&str]) {
        let parsed = ParsedFile::parse(path, source, language).unwrap();
        let func = parsed.all_functions()[0];
        let lines: Vec<usize> = parsed
            .statements_in_function(&func)
            .into_iter()
            .map(|(line, _)| line)
            .collect();
        let span_lines: Vec<usize> = parsed
            .statement_spans_in_function(&func)
            .into_iter()
            .map(|span| span.line)
            .collect();
        assert_eq!(
            span_lines, lines,
            "{path}: byte-span and line-only statement walkers must stay identical"
        );
        let arms = parsed.statement_arms_in_function(&func);
        let untagged: Vec<usize> = lines
            .iter()
            .copied()
            .filter(|line| arms.arm_of_line(*line).is_none())
            .collect();
        assert!(
            untagged.is_empty(),
            "{path}: statement lines missing lexical arms: {untagged:?}; got {lines:?}"
        );
        let missing: Vec<(&str, usize)> = markers
            .iter()
            .map(|marker| {
                let line = source
                    .lines()
                    .position(|source_line| source_line.contains(marker))
                    .map(|line| line + 1)
                    .unwrap_or_else(|| panic!("marker `{marker}` must occur in the fixture"));
                (*marker, line)
            })
            .filter(|(_, line)| !lines.contains(line))
            .collect();
        assert!(
            missing.is_empty(),
            "{path}: nested statement markers missing from CFG universe: {missing:?}; got {lines:?}"
        );
    }

    #[test]
    fn statements_in_function_descends_rust_expression_statement_wrappers() {
        let source = "fn h(x: i32) {\n    let a = x + 1;\n    if a > 0 {\n        sink_rust_if(a);\n    }\n    match a {\n        _ => { sink_rust_match(a); }\n    }\n    loop {\n        sink_rust_loop(a);\n        break;\n    }\n    while a > 0 {\n        sink_rust_while(a);\n        break;\n    }\n    for v in 0..a {\n        sink_rust_for(v);\n    }\n    unsafe {\n        sink_rust_unsafe(a);\n    }\n    async {\n        sink_rust_async(a);\n    };\n    const {\n        sink_rust_const(a);\n    };\n    try {\n        sink_rust_try(a);\n    };\n    gen {\n        sink_rust_gen(a);\n    };\n    sink(a);\n}\n";
        assert_statement_markers(
            "m.rs",
            source,
            Language::Rust,
            &[
                "sink_rust_if",
                "sink_rust_match",
                "sink_rust_loop",
                "sink_rust_while",
                "sink_rust_for",
                "sink_rust_unsafe",
                "sink_rust_async",
                "sink_rust_const",
                "sink_rust_try",
                "sink_rust_gen",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_python_statement_wrappers() {
        let source = "def h(x, lock):\n    with lock:\n        sink_py_with(x)\n    match x:\n        case 1:\n            sink_py_case(x)\n    try:\n        sink_py_try(x)\n    except ValueError:\n        sink_py_except(x)\n    finally:\n        sink_py_finally(x)\n    try:\n        sink_py_group_try(x)\n    except* TypeError:\n        sink_py_except_group(x)\n    return x\n";
        assert_statement_markers(
            "m.py",
            source,
            Language::Python,
            &[
                "sink_py_with",
                "sink_py_case",
                "sink_py_except(x)",
                "sink_py_finally",
                "sink_py_except_group",
            ],
        );
    }

    const JS_WRAPPER_SOURCE: &str = "function h(x, obj) {\n  let y = x;\nlabel:\n  while (y < 2) {\n    sink_js_label(y);\n    break;\n  }\n  switch (y) {\n    case 1:\n      sink_js_case(y);\n      break;\n    default:\n      sink_js_default(y);\n  }\n  try {\n    sink_js_try(y);\n  } catch (e) {\n    sink_js_catch(y);\n  } finally {\n    sink_js_finally(y);\n  }\n  with (obj) {\n    sink_js_with(y);\n  }\n  return y;\n}\n";

    #[test]
    fn statements_in_function_descends_javascript_statement_wrappers() {
        assert_statement_markers(
            "m.js",
            JS_WRAPPER_SOURCE,
            Language::JavaScript,
            &[
                "sink_js_label",
                "sink_js_case",
                "sink_js_default",
                "sink_js_catch",
                "sink_js_finally",
                "sink_js_with",
            ],
        );
    }

    const TS_WRAPPER_SOURCE: &str = "function h(x: number): number {\n  let y = x;\nlabel:\n  while (y < 2) {\n    sink_ts_label(y);\n    break;\n  }\n  switch (y) {\n    case 1:\n      sink_ts_case(y);\n      break;\n    default:\n      sink_ts_default(y);\n  }\n  try {\n    sink_ts_try(y);\n  } catch (e) {\n    sink_ts_catch(y);\n  } finally {\n    sink_ts_finally(y);\n  }\n  return y;\n}\n";

    #[test]
    fn statements_in_function_descends_typescript_statement_wrappers() {
        assert_statement_markers(
            "m.ts",
            TS_WRAPPER_SOURCE,
            Language::TypeScript,
            &[
                "sink_ts_label",
                "sink_ts_case",
                "sink_ts_default",
                "sink_ts_catch",
                "sink_ts_finally",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_tsx_statement_wrappers() {
        assert_statement_markers(
            "m.tsx",
            TS_WRAPPER_SOURCE,
            Language::Tsx,
            &[
                "sink_ts_label",
                "sink_ts_case",
                "sink_ts_default",
                "sink_ts_catch",
                "sink_ts_finally",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_go_statement_wrappers() {
        let source = "package p\nfunc h(ch chan any, x any) any {\n    y := x\nlabel:\n    for y == nil {\n        sink_go_label(y)\n        break\n    }\n    select {\n    case v := <-ch:\n        sink_go_select(v)\n    default:\n        sink_go_select_default(y)\n    }\n    switch y {\n    case 1:\n        sink_go_switch(y)\n    default:\n        sink_go_switch_default(y)\n    }\n    switch v := y.(type) {\n    case int:\n        sink_go_type(v)\n    default:\n        sink_go_type_default(y)\n    }\n    return y\n}\n";
        assert_statement_markers(
            "m.go",
            source,
            Language::Go,
            &[
                "sink_go_label",
                "sink_go_select(v)",
                "sink_go_select_default",
                "sink_go_switch(y)",
                "sink_go_switch_default",
                "sink_go_type(v)",
                "sink_go_type_default",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_c_labeled_statement_wrapper() {
        let source = "int h(int x) {\n    int y = x;\nlabel:\n    while (y < 2) {\n        sink_c_label(y);\n        y++;\n    }\n    [[likely]] while (y < 3) {\n        sink_c_attribute(y);\n        break;\n    }\n    return y;\n}\n";
        assert_statement_markers(
            "m.c",
            source,
            Language::C,
            &["sink_c_label", "sink_c_attribute"],
        );
    }

    #[test]
    fn statements_in_function_descends_cpp_statement_wrappers() {
        let source = "int h(int x) {\n    int y = x;\nlabel:\n    while (y < 2) {\n        sink_cpp_label(y);\n        y++;\n    }\n    [[likely]] while (y < 3) {\n        sink_cpp_attribute(y);\n        break;\n    }\n    try {\n        sink_cpp_try(y);\n    } catch (...) {\n        sink_cpp_catch(y);\n    }\n    return y;\n}\n";
        assert_statement_markers(
            "m.cpp",
            source,
            Language::Cpp,
            &["sink_cpp_label", "sink_cpp_attribute", "sink_cpp_catch"],
        );
    }

    #[test]
    fn statements_in_function_descends_java_statement_wrappers() {
        let source = "class M {\n  static int h(int x, Object lock) {\n    int y = x;\nlabel:\n    while (y < 0) {\n      sink_java_label(y);\n      break;\n    }\n    synchronized (lock) {\n      sink_java_sync(y);\n    }\n    switch (y) {\n      case 1:\n        sink_java_case(y);\n        break;\n      default:\n        sink_java_default(y);\n    }\n    switch (y) {\n      case 2 -> sink_java_rule(y);\n      default -> sink_java_rule_default(y);\n    }\n    try (Resource r = open()) {\n      sink_java_try(y);\n    } catch (RuntimeException e) {\n      sink_java_catch(y);\n    } finally {\n      sink_java_finally(y);\n    }\n    return y;\n  }\n}\n";
        assert_statement_markers(
            "M.java",
            source,
            Language::Java,
            &[
                "sink_java_label",
                "sink_java_sync",
                "sink_java_case",
                "sink_java_default",
                "sink_java_rule(y)",
                "sink_java_rule_default",
                "sink_java_try",
                "sink_java_catch",
                "sink_java_finally",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_bash_statement_wrappers() {
        let source = "h() {\n  local x=0\n  for i in 1; do\n    sink_bash_do \"$x\"\n  done\n  case \"$x\" in\n    1) sink_bash_case \"$x\" ;;\n  esac\n  (\n    sink_bash_subshell \"$x\"\n  )\n  {\n    sink_bash_redirect \"$x\"\n  } > /dev/null\n}\n";
        assert_statement_markers(
            "m.sh",
            source,
            Language::Bash,
            &[
                "sink_bash_do",
                "sink_bash_case",
                "sink_bash_subshell",
                "sink_bash_redirect",
            ],
        );
    }

    #[test]
    fn statements_in_function_descends_lua_statement_wrappers() {
        let source = "function h(x)\n  if x == 1 then\n    sink_lua_first(x)\n  elseif x == 2 then\n    sink_lua_middle(x)\n  else\n    sink_lua_last(x)\n  end\n  return x\nend\n";
        assert_statement_markers(
            "m.lua",
            source,
            Language::Lua,
            &["sink_lua_middle", "sink_lua_last"],
        );
    }

    #[test]
    fn parsed_file_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ParsedFile>();
    }

    #[test]
    fn loaded_repo_is_send_and_sync() {
        // Informational companion: a !Sync field elsewhere in LoadedRepo must NOT
        // gate C2 — if only this one fails, that field is investigated separately.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<crate::repo_loader::LoadedRepo>();
    }

    #[test]
    fn decorated_function_field_readers_unwrap() {
        let p = ParsedFile::parse(
            "a.py",
            "@deco\ndef f(x, y):\n    z = x\n    return z\n",
            Language::Python,
        )
        .unwrap();
        let deco =
            descendant_of_kind(p.tree.root_node(), "decorated_definition").expect("deco node");

        assert!(p.find_parameters_node(&deco).is_some());
        assert!(p.function_body_node(&deco).is_some());
        assert!(!p.statements_in_function(&deco).is_empty());
        assert!(!p.statement_spans_in_function(&deco).is_empty());
        assert!(!p.return_value_nodes(&deco).is_empty());
    }

    #[test]
    fn return_value_nodes_excludes_nested_function_returns() {
        // A nested `def` has the SAME kind as the outer, so the recursion fence must
        // be by node identity, not kind — else the nested return is mis-collected as
        // the outer's (corrupts contract postconditions; reachable for decorated fns
        // via the unwrap). Here outer returns 1; the nested `inner` returns 99.
        let p = ParsedFile::parse(
            "a.py",
            "@deco\ndef outer():\n    def inner():\n        return 99\n    return 1\n",
            Language::Python,
        )
        .unwrap();
        let deco =
            descendant_of_kind(p.tree.root_node(), "decorated_definition").expect("deco node");
        let returns = p.return_value_nodes(&deco);
        assert_eq!(
            returns.len(),
            1,
            "only the outer's return, not the nested fn's"
        );
        assert_eq!(returns[0].value_text.as_deref(), Some("1"));
    }

    #[test]
    fn return_value_nodes_preserves_go_legacy_expression_list_and_adds_slots() {
        let p = ParsedFile::parse(
            "a.go",
            "package p\nfunc pair(a string) (string, error) { return a, nil }\n",
            Language::Go,
        )
        .unwrap();
        let func = p.all_functions().into_iter().next().expect("pair");
        let returns = p.return_value_nodes(&func);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].value_text.as_deref(), Some("a, nil"));
        assert_eq!(returns[0].value_kind.as_deref(), Some("expression_list"));
        assert_eq!(
            returns[0]
                .values
                .iter()
                .map(|value| value.text.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "nil"]
        );
    }

    #[test]
    fn decorated_function_canonical_single_node() {
        let p =
            ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
        let fs: Vec<_> = p
            .all_functions()
            .into_iter()
            .filter(|n| p.language.function_name(n).map(|nm| p.node_text(&nm)) == Some("f".into()))
            .collect();

        assert_eq!(fs.len(), 1, "one canonical record for f");
        assert_eq!(
            fs[0].kind(),
            "decorated_definition",
            "the wrapper is canonical"
        );
    }

    #[test]
    fn decorated_function_canonical_single_node_on_reconstruction_fallback() {
        let mut p =
            ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
        p.functions[0].kind_id = u16::MAX;

        let (nodes, used_fallback) = p.all_functions_inner();
        let fs: Vec<_> = nodes
            .into_iter()
            .filter(|n| p.language.function_name(n).map(|nm| p.node_text(&nm)) == Some("f".into()))
            .collect();

        assert!(used_fallback, "synthetic corruption must force fallback");
        assert_eq!(fs.len(), 1, "fallback must preserve wrapper-canonical f");
        assert_eq!(
            fs[0].kind(),
            "decorated_definition",
            "the wrapper is canonical"
        );
    }

    fn all_call_sites(pf: &ParsedFile) -> Vec<(usize, String)> {
        // Every call node's (start_byte, callee_name), pre-order — the keys the index must serve.
        fn walk(pf: &ParsedFile, node: tree_sitter::Node<'_>, out: &mut Vec<(usize, String)>) {
            if pf.language.is_call_node(node.kind()) {
                if let Some(name_node) = pf.call_function_name(&node) {
                    out.push((node.start_byte(), pf.node_text(&name_node).to_string()));
                }
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk(pf, child, out);
            }
        }
        let mut out = Vec::new();
        walk(pf, pf.tree.root_node(), &mut out);
        out
    }

    fn index_texts(pf: &ParsedFile, sb: usize, name: &str) -> Vec<String> {
        pf.call_args_index()
            .by_call
            .get(&(sb, name.to_string()))
            .map(|spans| spans.iter().map(|a| pf.arg_text(a)).collect())
            .unwrap_or_default()
    }

    #[test]
    fn call_args_index_matches_reference_all_languages() {
        // One fixture per Language::all() variant; each contains calls with varied arg shapes.
        let cases: Vec<(&str, Language, &str)> = vec![
            (
                "fn f(){ g(a, &b); o.m(x); a.b().c(d); h(); }",
                Language::Rust,
                "t.rs",
            ),
            ("func f(){ g(a, b); o.M(x); h() }", Language::Go, "t.go"),
            (
                "def f():\n    g(a, b)\n    o.m(x)\n    h()\n",
                Language::Python,
                "t.py",
            ),
            (
                "function f(){ g(a, b); o.m(x); h(); }",
                Language::JavaScript,
                "t.js",
            ),
            (
                "function f(x: number){ g(a, b); o.m(x); }",
                Language::TypeScript,
                "t.ts",
            ),
            (
                "const f = () => { g(a, b); o.m(x); };",
                Language::Tsx,
                "t.tsx",
            ),
            (
                "class C { void f(){ g(a, b); o.m(x); h(); } }",
                Language::Java,
                "T.java",
            ),
            ("void f(){ g(a, b); o->m(x); h(); }", Language::C, "t.c"),
            ("void f(){ g(a, b); o.m(x); h(); }", Language::Cpp, "t.cpp"),
            ("function f() g(a, b) o.m(x) end", Language::Lua, "t.lua"),
            ("locals { x = max(1, 2) }\n", Language::Terraform, "t.tf"),
            ("f() { cmd arg1 arg2; g; }\n", Language::Bash, "t.sh"),
        ];
        for (src, lang, path) in cases {
            let pf = ParsedFile::parse(path, src, lang).unwrap();
            let sites = all_call_sites(&pf);
            assert!(!sites.is_empty(), "no call sites parsed for {path}");
            for (sb, name) in sites {
                assert_eq!(
                    index_texts(&pf, sb, &name),
                    pf.call_argument_texts_at_reference(sb, &name),
                    "mismatch in {path} at byte {sb} call `{name}`"
                );
            }
        }
    }

    #[test]
    fn call_args_index_edge_cases() {
        // For each fixture, every call site: index == reference walk.
        let fixtures: &[(&str, Language, &str)] = &[
            ("fn f(){ g(&x, &&y); }", Language::Rust, "ref.rs"), // leading-& strip (repeated)
            ("fn f(){ a(b(c), d(e)); }", Language::Rust, "nest.rs"), // nested calls
            ("fn f(){ p(1); q(2); }", Language::Rust, "twoline.rs"), // two calls, one line
            ("fn f(){ z(); }", Language::Rust, "zero.rs"),       // zero-arg
            ("fn f(){ a.b().c(d); }", Language::Rust, "chain.rs"), // same start_byte, distinct names
            ("fn f(){ a.b().b(d); }", Language::Rust, "samechain.rs"), // same start_byte AND name -> outer wins
            (
                "fn f(){\n  obj\n    .m(\n      arg,\n    );\n}",
                Language::Rust,
                "multi.rs",
            ), // multi-line
        ];
        for (src, lang, path) in fixtures {
            let pf = ParsedFile::parse(path, src, *lang).unwrap();
            for (sb, name) in all_call_sites(&pf) {
                assert_eq!(
                    index_texts(&pf, sb, &name),
                    pf.call_argument_texts_at_reference(sb, &name),
                    "{path} byte {sb} `{name}`"
                );
            }
        }
    }

    #[test]
    fn call_argument_texts_at_production_matches_reference() {
        let cases: &[(&str, Language, &str)] = &[
            (
                "fn f(){ g(a, &b); o.m(x); a.b().c(d); a.b().b(z); }",
                Language::Rust,
                "p.rs",
            ),
            ("func f(){ g(a, b); o.M(x) }", Language::Go, "p.go"),
            (
                "def f():\n    g(a, b)\n    o.m(x)\n",
                Language::Python,
                "p.py",
            ),
        ];
        for (src, lang, path) in cases {
            let pf = ParsedFile::parse(path, src, *lang).unwrap();
            for (sb, name) in all_call_sites(&pf) {
                assert_eq!(
                    pf.call_argument_texts_at(sb, &name),
                    pf.call_argument_texts_at_reference(sb, &name),
                    "{path} byte {sb} `{name}`"
                );
            }
        }
    }

    #[test]
    fn call_args_index_name_mismatch_is_empty() {
        // Querying a real call's start_byte with the WRONG name returns empty (legacy parity).
        let pf = ParsedFile::parse("m.rs", "fn f(){ g(a, b); }", Language::Rust).unwrap();
        let (sb, name) = all_call_sites(&pf)
            .into_iter()
            .find(|(_, n)| n == "g")
            .unwrap();
        assert_eq!(index_texts(&pf, sb, "not_g"), Vec::<String>::new());
        assert_eq!(
            index_texts(&pf, sb, &name),
            pf.call_argument_texts_at_reference(sb, &name)
        );
    }

    #[test]
    fn call_args_index_same_start_same_name_outer_wins() {
        // a.b().b(d): outer and inner call both start at byte(a) and are named `b`.
        // First-write-wins (pre-order = outer) must match the reference walk.
        let pf =
            ParsedFile::parse("s.rs", "fn f(){ a.b().b(outer_arg); }", Language::Rust).unwrap();
        // The shared start_byte is the byte of `a` inside the body.
        let a_byte = pf.source.find("a.b()").unwrap();
        let got = index_texts(&pf, a_byte, "b");
        assert_eq!(got, pf.call_argument_texts_at_reference(a_byte, "b"));
    }

    #[test]
    fn candidate_fields_are_maximal_post_accessor_identifiers() {
        let got = candidate_fields_on_line("s.cb = f; t->cbx = g; obj.data->next = h; x = 3.14;");
        let want: BTreeSet<String> = ["cb", "cbx", "data", "next", "14"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(got, want); // "14" is harmless noise: digits are identifier chars,
                               // and no callee filter ever queries it
    }

    #[test]
    fn candidate_fields_use_the_pinned_unicode_predicate() {
        // same predicate as the Level-4 call-site filter (char::is_alphanumeric || '_'),
        // Unicode-aware — `café` is ONE identifier.
        let got = candidate_fields_on_line("obj.café = handler; p->x_1 = g;");
        let want: BTreeSet<String> = ["café", "x_1"].iter().map(|s| s.to_string()).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn level4_legacy_quirk_arrow_anywhere_priority() {
        // ->field ANYWHERE in the line beats a closer .field (quirk 1):
        // the .cb assignment to `f` is dropped because ->cb is found first.
        let src = "s.cb = f; t->cb = g;\n";
        let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
        assert_eq!(got, vec!["g".to_string()]);
    }

    #[test]
    fn level4_legacy_quirk_prefix_consumption() {
        // ->cbx matches find("->cb"); scan position advances past it, and the
        // REAL earlier .cb assignment is never revisited (quirk 2).
        let src = "s.cb = f; t->cbx = g;\n";
        let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
        assert_eq!(got, Vec::<String>::new());
    }

    #[test]
    fn level4_legacy_rhs_rules() {
        // single-= anchor (== rejected), &-strip, stop at ; , } ) and whitespace,
        // known_fns filter, BTreeSet dedup+sort.
        let src =
            "a.cb == f;\nb.cb = &handler;\nc.cb = handler, x;\nd.cb = unknown_fn;\ne.cb = handler;\n";
        let got = resolve_struct_field_assignment(src, "cb", &fns(&["handler", "f"]));
        assert_eq!(got, vec!["handler".to_string()]); // deduped, == rejected, unknown filtered
    }

    #[test]
    fn level4_legacy_designated_initializer_multi_field() {
        let src = "static struct ops o = { .open = do_open, .close = do_close };\n";
        assert_eq!(
            resolve_struct_field_assignment(src, "open", &fns(&["do_open", "do_close"])),
            vec!["do_open".to_string()]
        );
        assert_eq!(
            resolve_struct_field_assignment(src, "close", &fns(&["do_open", "do_close"])),
            vec!["do_close".to_string()]
        );
    }

    #[test]
    fn function_table_captures_named_and_anonymous_in_query_order() {
        // JS: named fn + anonymous callback lambda (function_name() returns None for the latter)
        let src = "function alpha(a, b) { return a; }\nitems.forEach((x) => { use(x); });\n";
        let pf = ParsedFile::parse("t.js", src, Language::JavaScript).unwrap();
        let table = pf.functions();
        // Full captured sequence preserved, query order, including unnamed entries
        let direct = pf.all_functions();
        assert_eq!(table.len(), direct.len());
        for (info, node) in table.iter().zip(direct.iter()) {
            assert_eq!(info.start_byte, node.start_byte());
            assert_eq!(info.end_byte, node.end_byte());
            assert_eq!(info.kind_id, node.kind_id());
        }
        assert_eq!(table[0].name.as_deref(), Some("alpha"));
        assert_eq!(
            table[0].param_names,
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert!(table.iter().any(|f| f.name.is_none())); // the arrow callback
    }

    #[test]
    fn function_table_rust_and_same_named_functions() {
        let src = "fn f(x: u32) -> u32 { x }\nmod a { pub fn f(y: u32) -> u32 { y } }\n";
        let pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
        let named: Vec<_> = pf
            .functions()
            .iter()
            .filter(|f| f.name.as_deref() == Some("f"))
            .collect();
        assert_eq!(named.len(), 2); // both kept, query order — no dedup/last-writer-wins
        assert_eq!(named[0].param_names, Some(vec!["x".to_string()]));
    }

    #[test]
    fn all_functions_reconstructed_matches_direct_query_per_language() {
        // (source, language, path) for all supported languages; each has >=2 functions.
        let cases: Vec<(&str, Language, &str)> = vec![
            (
                "fn a() {}\nfn b(x: u32) { let _ = x; }",
                Language::Rust,
                "t.rs",
            ),
            ("func A() {}\nfunc B(x int) { _ = x }", Language::Go, "t.go"),
            (
                "def a():\n    pass\n\ndef b(x):\n    return x\n",
                Language::Python,
                "t.py",
            ),
            (
                "function a() {}\nitems.map((x) => x + 1);",
                Language::JavaScript,
                "t.js",
            ),
            (
                "function a(): void {}\nconst b = (x: number) => x + 1;",
                Language::TypeScript,
                "t.ts",
            ),
            (
                "function A(): JSX.Element { return <div/>; }\nconst B = () => <span/>;",
                Language::Tsx,
                "t.tsx",
            ),
            (
                "class K { void a() {} int b(int x) { return x; } }",
                Language::Java,
                "t.java",
            ),
            (
                "void a(void) {}\nint b(int x) { return x; }",
                Language::C,
                "t.c",
            ),
            (
                "class K { void a() {} };\nint b() { return 0; }",
                Language::Cpp,
                "t.cpp",
            ),
            (
                "function a() end\nlocal function b(x) return x end",
                Language::Lua,
                "t.lua",
            ),
            (
                "resource \"aws_s3_bucket\" \"a\" {}\nresource \"aws_s3_bucket\" \"b\" {}",
                Language::Terraform,
                "t.tf",
            ),
            (
                "a() { echo hi; }\nb() { echo bye; }",
                Language::Bash,
                "t.sh",
            ),
        ];
        for (src, lang, path) in cases {
            let pf = ParsedFile::parse(path, src, lang).unwrap();
            let direct = pf.all_functions_via_tree();
            let (reconstructed, used_fallback) = pf.all_functions_inner();
            // Anti-vacuous guard: a grammar bump that stops capturing functions for a
            // language would otherwise make this drift detector pass on empty sets.
            assert!(
                direct.len() >= 2,
                "{path}: fixture must parse to >=2 functions (got {})",
                direct.len()
            );
            assert!(!used_fallback, "{path}: reconstruction must not fall back");
            assert_eq!(direct.len(), reconstructed.len(), "{path}");
            for (d, r) in direct.iter().zip(reconstructed.iter()) {
                assert_eq!(
                    (d.kind_id(), d.start_byte(), d.end_byte()),
                    (r.kind_id(), r.start_byte(), r.end_byte()),
                    "{path}"
                );
            }
        }
    }

    #[test]
    fn all_functions_falls_back_to_direct_query_on_reconstruction_miss() {
        let src = "fn a() {}\nfn b() {}\n";
        let mut pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
        pf.functions[0].kind_id = u16::MAX; // synthetic corruption: no node can match
        let (nodes, used_fallback) = pf.all_functions_inner();
        assert!(used_fallback); // the drift detector
        assert_eq!(nodes.len(), 2); // full sequence via fallback — never silently skipped
    }

    #[test]
    fn go_dot_import_marker_pins_single_grouped_and_named_shapes() {
        for source in [
            "package p\nimport . \"example.com/a\"\n",
            "package p\nimport (\n . \"example.com/a\"\n x \"example.com/x\"\n)\n",
        ] {
            let parsed = ParsedFile::parse("p.go", source, Language::Go).unwrap();
            assert!(parsed.go_has_dot_import(), "{source}");
            assert!(!parsed.extract_imports().contains_key("."), "{source}");
        }
        let named = ParsedFile::parse(
            "p.go",
            "package p\nimport x \"example.com/x\"\n",
            Language::Go,
        )
        .unwrap();
        assert!(!named.go_has_dot_import());
    }

    #[test]
    fn go_type_parameter_binder_pins_function_and_method_receiver_shapes() {
        let parsed = ParsedFile::parse(
            "p.go",
            "package p\nfunc free[T any](v T) {}\ntype Store[U any] struct{}\nfunc (s *Store[T]) method(v T) {}\nfunc plain(v T) {}\nfunc nested(v T) { type Local[T any] struct{}; _ = Local[int]{} }\n",
            Language::Go,
        )
        .unwrap();
        let function = |name: &str| {
            parsed
                .all_functions()
                .into_iter()
                .find(|node| {
                    parsed
                        .language
                        .function_name(node)
                        .is_some_and(|n| parsed.node_text(&n) == name)
                })
                .unwrap_or_else(|| panic!("missing {name}"))
        };
        assert!(parsed.go_type_parameter_binds_receiver(&function("free"), "T"));
        assert!(parsed.go_type_parameter_binds_receiver(&function("method"), "T"));
        assert!(!parsed.go_type_parameter_binds_receiver(&function("plain"), "T"));
        assert!(!parsed.go_type_parameter_binds_receiver(&function("nested"), "T"));
        assert!(!parsed.go_type_parameter_binds_receiver(&function("free"), "pkg.T"));
        assert!(!parsed.go_type_parameter_binds_receiver(&function("free"), "Store[T]"));
    }
}
