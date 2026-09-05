use crate::access_path::AccessPath;
use crate::languages::Language;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser, Tree};

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

/// A statement the parser located, with its real source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementSpan {
    pub line: usize,
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
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

/// A `keyed_element`'s `key`/`value` field is a `literal_element`, which wraps
/// a single `_expression` (or `literal_value`) child. Returns that child when
/// it is a plain `identifier` — the shape form (a) needs for both the field
/// name and the registered value (tree-sitter-go does not use
/// `field_identifier` for composite-literal keys, since the parser can't
/// distinguish a struct field key from a map key syntactically).
fn literal_element_identifier<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() != "literal_element" {
        return None;
    }
    let inner = node.named_child(0)?;
    (inner.kind() == "identifier").then_some(inner)
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
    /// Lazily-built call-argument index (Task S1.5). Lazy so warm nav-cache loads,
    /// AST-only consumers, and callers with no resolved-call args pay nothing; the
    /// cold CPG/nav build that runs Step 5b builds it on demand.
    call_args: std::sync::OnceLock<CallArgsIndex>,
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
            call_args: std::sync::OnceLock::new(),
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
            if let Some(name_node) = self.language.call_function_name(&node) {
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
                if child.is_named() {
                    args.push(CallArg {
                        start_byte: child.start_byte(),
                        end_byte: child.end_byte(),
                    });
                }
            }
        }
        args
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

    /// Find the smallest function/method node containing the given line (1-indexed).
    pub fn enclosing_function(&self, line: usize) -> Option<Node<'_>> {
        let row = line.saturating_sub(1); // tree-sitter uses 0-indexed rows
        self.find_enclosing_node(
            self.tree.root_node(),
            row,
            &self.language.function_node_types(),
        )
    }

    fn find_enclosing_node<'a>(
        &self,
        node: Node<'a>,
        row: usize,
        types: &[&str],
    ) -> Option<Node<'a>> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        // Check children first (prefer smallest/deepest match)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_node(child, row, types) {
                return Some(found);
            }
        }

        if types.contains(&node.kind()) {
            Some(node)
        } else {
            None
        }
    }

    /// Function nodes, reconstructed from the eager table. On any reconstruction
    /// miss, falls back to the direct dual-path collection for the WHOLE file —
    /// never a partial sequence. The bool is the fallback-fire flag: in-module
    /// tests assert it is false for all 12 supported languages so grammar drift
    /// cannot silently route a language to the slow path.
    pub fn all_functions(&self) -> Vec<Node<'_>> {
        self.all_functions_inner().0
    }

    fn unwrap_decorated<'a>(&self, node: Node<'a>) -> Node<'a> {
        if self.language == Language::Python && node.kind() == "decorated_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    return child;
                }
            }
        }
        node
    }

    pub(crate) fn all_functions_inner(&self) -> (Vec<Node<'_>>, bool) {
        let mut out = Vec::with_capacity(self.functions.len());
        for info in &self.functions {
            match self.reconstruct_function_node(info) {
                Some(node) => out.push(node),
                None => return (self.all_functions_via_tree(), true),
            }
        }
        (out, false)
    }

    fn reconstruct_function_node(&self, info: &FunctionInfo) -> Option<Node<'_>> {
        let mut node = self
            .tree
            .root_node()
            .descendant_for_byte_range(info.start_byte, info.end_byte)?;
        // descendant_for_byte_range returns the DEEPEST node spanning the range,
        // so recovery walks UP through same-span ancestors — a walk-down can
        // never reach a same-span ancestor.
        loop {
            if node.start_byte() == info.start_byte
                && node.end_byte() == info.end_byte
                && node.kind_id() == info.kind_id
            {
                return Some(node);
            }
            match node.parent() {
                Some(p) if p.start_byte() == info.start_byte && p.end_byte() == info.end_byte => {
                    node = p
                }
                _ => return None,
            }
        }
    }

    /// Reconstruct one callable by its exact eager-table byte identity and project
    /// edit coordinates without returning or copying source text.
    pub(crate) fn function_ast_spans(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<FunctionAstSpans> {
        const MAX_INDENTATION_BYTES: usize = 256;

        fn span(node: Node<'_>) -> AstNodeSpan {
            AstNodeSpan {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
            }
        }

        fn indentation_before(
            source: &str,
            byte: usize,
            non_whitespace_reason: &'static str,
        ) -> Result<String, &'static str> {
            let line_start = source.as_bytes()[..byte]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |newline| newline + 1);
            let prefix = &source[line_start..byte];
            if prefix.len() > MAX_INDENTATION_BYTES {
                return Err("line indentation exceeds 256-byte limit");
            }
            if prefix.bytes().all(|b| matches!(b, b' ' | b'\t')) {
                Ok(prefix.to_string())
            } else {
                Err(non_whitespace_reason)
            }
        }

        let info = self
            .functions
            .iter()
            .find(|info| info.start_byte == start_byte && info.end_byte == end_byte)?;
        let outer = self.reconstruct_function_node(info)?;
        let inner = self.unwrap_decorated(outer);
        let name = self.language.function_name(&inner).map(span);
        let body_node = inner.child_by_field_name("body");
        let body = body_node.map(span);
        let symbol_indentation = indentation_before(
            &self.source,
            outer.start_byte(),
            "outer callable is not preceded only by line indentation",
        );
        let body_indentation = match body_node {
            None => Err("body span unavailable"),
            Some(body) => match body.named_child(0) {
                None => Err("body has no named child"),
                Some(child) => indentation_before(
                    &self.source,
                    child.start_byte(),
                    "first named body child is not preceded only by line indentation",
                ),
            },
        };

        Some(FunctionAstSpans {
            name,
            body,
            symbol_indentation,
            body_indentation,
        })
    }

    fn all_functions_via_tree(&self) -> Vec<Node<'_>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let mut functions = Vec::new();

        // Use compiled tree-sitter query when available (faster: skips irrelevant subtrees).
        if let Some(query) = get_query(self.language, QueryKind::Functions) {
            let func_idx = query
                .capture_index_for_name("func")
                .expect("Functions query must have @func capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == func_idx {
                        functions.push(capture.node);
                    }
                }
            }
        } else {
            // Fallback: manual recursive walk.
            self.collect_functions_manual(self.tree.root_node(), &mut functions);
        }

        functions.retain(|node| !self.is_python_decorated_inner_function(node));
        functions
    }

    fn is_python_decorated_inner_function(&self, node: &Node<'_>) -> bool {
        self.language == Language::Python
            && node.kind() == "function_definition"
            && node
                .parent()
                .is_some_and(|parent| parent.kind() == "decorated_definition")
    }

    /// Build the eager function table via the existing dual-path collection.
    fn build_function_table(&self) -> Vec<FunctionInfo> {
        self.all_functions_via_tree()
            .into_iter()
            .map(|node| FunctionInfo {
                name: self
                    .language
                    .function_name(&node)
                    .map(|n| self.node_text(&n).to_string()),
                kind_id: node.kind_id(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                param_names: self.function_parameter_slots(&node),
                owner: self
                    .language
                    .method_owner(&node)
                    .map(|n| crate::resolution::owner_key(self.node_text(&n))),
                receiver_var: self
                    .language
                    .go_receiver_var(&node)
                    .map(|n| self.node_text(&n).to_string()),
            })
            .collect()
    }

    pub fn functions(&self) -> &[FunctionInfo] {
        &self.functions
    }

    /// Find the smallest function node spanning the given 1-indexed line.
    pub fn function_node_spanning(&self, line: usize) -> Option<Node<'_>> {
        self.all_functions()
            .into_iter()
            .filter(|node| {
                let (start, end) = self.node_line_range(node);
                start <= line && line <= end
            })
            .min_by_key(|node| node.end_byte().saturating_sub(node.start_byte()))
    }

    /// S3 P6-lite: syntactically-provable receiver type for `receiver` at a call
    /// on `call_line`. Typed params + constructor locals; when `recover_var` is true
    /// also recovers `var r T` declarations. Only bindings at or before `call_line`
    /// count; >1 binding before the call means shadow bail. Rust + Go +
    /// guarded Python.
    /// Returns `(type_found, binding_count)`: the raw, unpeeled type text +
    /// which fact recovered it, plus how many local bindings of `receiver`
    /// were seen (so the caller can distinguish "no bindings" from "bindings
    /// present but type unrecoverable").
    pub fn receiver_type_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
    ) -> (Option<(String, crate::resolution::ReceiverRecovery)>, usize) {
        let (found, bindings, _) = self.receiver_type_evidence_in_fn(
            func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
        );
        (found, bindings)
    }

    /// P17 fix wave 2: the ordinary scan result plus the first recoverable
    /// binding. Go on-demand receiver proof uses the first fact only to retain
    /// the legacy ladder input when a later lexical rebinding forces R3.
    pub(crate) fn receiver_type_evidence_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
    ) -> (
        Option<(String, crate::resolution::ReceiverRecovery)>,
        usize,
        Option<(String, crate::resolution::ReceiverRecovery)>,
    ) {
        self.receiver_type_evidence_in_fn_mode(
            func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
            false,
        )
    }

    /// Wave-3 exception used only by the post-merge direct-route proof. The
    /// ordinary classifier above deliberately retains f16663e's projection.
    pub(crate) fn go_same_scope_reuse_receiver_type_evidence_in_fn(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
    ) -> (
        Option<(String, crate::resolution::ReceiverRecovery)>,
        usize,
        Option<(String, crate::resolution::ReceiverRecovery)>,
    ) {
        self.receiver_type_evidence_in_fn_mode(
            func_node,
            receiver,
            call_line,
            call_start_byte,
            recover_var,
            true,
        )
    }

    /// P17 wave 5 provenance: true when the call is inside a Go function
    /// literal whose parameter list binds `receiver`. Main deliberately treated
    /// this parameter as unrecoverable; P17 wave 2 began recovering its declared
    /// type, so callers use this fact to keep that new path out of the legacy
    /// bare-interface ladder when the qualified type is external.
    pub(crate) fn go_func_literal_parameter_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }

        fn walk(
            parsed: &ParsedFile,
            node: Node<'_>,
            receiver: &str,
            call_start_byte: usize,
        ) -> bool {
            if call_start_byte < node.start_byte() || call_start_byte >= node.end_byte() {
                return false;
            }
            if node.kind() == "func_literal"
                && parsed.find_parameters_node(&node).is_some_and(|params| {
                    let mut cursor = params.walk();
                    let found = params.children(&mut cursor).any(|param| {
                        param
                            .child_by_field_name("type")
                            .is_some_and(|ty| parsed.go_parameter_binds_name(param, ty, receiver))
                    });
                    found
                })
            {
                return true;
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(parsed, child, receiver, call_start_byte));
            found
        }

        walk(self, *func_node, receiver, call_start_byte)
    }

    fn receiver_type_evidence_in_fn_mode(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        recover_var: bool,
        enable_go_same_scope_reuse: bool,
    ) -> (
        Option<(String, crate::resolution::ReceiverRecovery)>,
        usize,
        Option<(String, crate::resolution::ReceiverRecovery)>,
    ) {
        use crate::languages::Language;
        use crate::resolution::ReceiverRecovery;

        if !matches!(
            self.language,
            Language::Rust | Language::Go | Language::Python
        ) {
            return (None, 0, None);
        }

        let mut found: Option<(String, ReceiverRecovery)> = None;
        let mut first_found: Option<(String, ReceiverRecovery)> = None;
        let mut bindings = 0usize;
        let mut go_lexical_rebinding = false;

        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for param in params.children(&mut cursor) {
                match self.language {
                    Language::Rust if param.kind() == "parameter" => {
                        let (Some(pattern), Some(ty)) = (
                            param.child_by_field_name("pattern"),
                            param.child_by_field_name("type"),
                        ) else {
                            continue;
                        };
                        if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        } else if self.node_binds_name(pattern, receiver) {
                            bindings += 1;
                            found = None;
                        }
                    }
                    Language::Go if param.kind() == "parameter_declaration" => {
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.go_parameter_binds_name(param, ty, receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        }
                    }
                    Language::Python
                        if matches!(
                            param.kind(),
                            "typed_parameter" | "typed_default_parameter"
                        ) =>
                    {
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.parameter_binds_name_before_type(param, ty, receiver) {
                            found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                            bindings += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        if bindings == 1 {
            first_found = found.clone();
        }

        // Python: count bare (untyped) parameters as bindings — `def run(Foo):`
        // shadows an import/class of the same name. Typed params were already
        // counted above, so only increment when `bindings` is still 0 for this
        // receiver to avoid double-counting.
        if matches!(self.language, Language::Python) && bindings == 0 {
            if let Some(params) = self.find_parameters_node(func_node) {
                let mut pcursor = params.walk();
                for param in params.children(&mut pcursor) {
                    match param.kind() {
                        "identifier"
                        | "default_parameter"
                        | "list_splat_pattern"
                        | "dictionary_splat_pattern" => {
                            let name_node = if param.kind() == "identifier" {
                                Some(param)
                            } else if matches!(
                                param.kind(),
                                "list_splat_pattern" | "dictionary_splat_pattern"
                            ) {
                                // Splat patterns have no `name` field; find the identifier child.
                                let mut sc = param.walk();
                                let found =
                                    param.children(&mut sc).find(|c| c.kind() == "identifier");
                                found
                            } else {
                                param.child_by_field_name("name")
                            };
                            if let Some(n) = name_node {
                                if self.simple_binding_text(&n).as_deref() == Some(receiver) {
                                    bindings += 1;
                                    // No type recovery possible from a bare param.
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Go declarations shadow only within their lexical binding scope. Keep
        // this visibility rule independent of the narrow same-scope `:=` reuse
        // exception: ordinary classification needs scope filtering too, but
        // must not silently enable reuse recovery.
        let go_filter_declarations_to_call_scope = matches!(self.language, Language::Go);
        self.walk_receiver_bindings(
            *func_node,
            true,
            receiver,
            call_line,
            call_start_byte,
            go_filter_declarations_to_call_scope,
            enable_go_same_scope_reuse,
            &mut found,
            &mut first_found,
            &mut bindings,
            &mut go_lexical_rebinding,
            recover_var,
        );
        if self.language == Language::Python && found.is_some() && bindings == 1 {
            let mut ancestor =
                func_node.descendant_for_byte_range(call_start_byte, call_start_byte);
            while let Some(scope) = ancestor {
                if scope.id() == func_node.id() {
                    break;
                }
                if matches!(scope.kind(), "for_statement" | "while_statement") {
                    let mut prefix_bindings = 0;
                    self.walk_receiver_bindings(
                        scope,
                        true,
                        receiver,
                        call_line,
                        call_start_byte,
                        false,
                        false,
                        &mut None,
                        &mut None,
                        &mut prefix_bindings,
                        &mut false,
                        recover_var,
                    );
                    // With one proven binding overall, a binding in this loop's
                    // prefix is the origin and resets it before each call. An
                    // origin outside the loop must also survive its back edge.
                    if prefix_bindings == 0
                        && self
                            .function_local_value_bindings(&scope)
                            .is_none_or(|names| names.contains(receiver))
                    {
                        found = None;
                        bindings += 1;
                        break;
                    }
                }
                ancestor = scope.parent();
            }
        }
        if bindings > 1 {
            return (
                None,
                bindings,
                go_lexical_rebinding.then_some(first_found).flatten(),
            );
        }
        (found, bindings, first_found)
    }

    /// Whether this Go file contains a dot import. Dot imports stay separate
    /// from the named-import map because `.` is not a resolvable qualifier.
    pub(crate) fn go_has_dot_import(&self) -> bool {
        if self.language != Language::Go {
            return false;
        }

        fn walk(node: Node<'_>) -> bool {
            if node.kind() == "import_spec" {
                let mut cursor = node.walk();
                return node
                    .children(&mut cursor)
                    .any(|child| child.kind() == "dot");
            }
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).any(walk);
            found
        }

        walk(self.tree.root_node())
    }

    /// True when the recovered bare receiver type is bound by the enclosing
    /// Go declaration's own generic parameters. Free functions declare them in
    /// `type_parameters`; methods declare receiver-specific names in the
    /// receiver type's `type_arguments` (`Store[T]`).
    pub(crate) fn go_type_parameter_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver_type: &str,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }
        let receiver_type = receiver_type
            .trim()
            .trim_start_matches('&')
            .trim_start_matches('*')
            .trim();
        if receiver_type.is_empty()
            || receiver_type.contains('.')
            || receiver_type.contains('[')
            || !receiver_type
                .chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_')
        {
            return false;
        }

        if let Some(params) = func_node.child_by_field_name("type_parameters") {
            let mut cursor = params.walk();
            if params.children(&mut cursor).any(|declaration| {
                if declaration.kind() != "type_parameter_declaration" {
                    return false;
                }
                let mut names = declaration.walk();
                let found = declaration.children(&mut names).any(|name| {
                    name.kind() == "identifier" && self.node_text(&name) == receiver_type
                });
                found
            }) {
                return true;
            }
        }

        if func_node.kind() != "method_declaration" {
            return false;
        }
        let Some(receiver) = func_node.child_by_field_name("receiver") else {
            return false;
        };

        fn receiver_args_bind(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
            if node.kind() == "type_arguments" {
                let mut cursor = node.walk();
                return node.named_children(&mut cursor).any(|arg| {
                    if matches!(arg.kind(), "type_identifier" | "identifier") {
                        return parsed.node_text(&arg).trim() == name;
                    }
                    if arg.kind() != "type_elem" {
                        return false;
                    }
                    let mut elem_cursor = arg.walk();
                    let found = arg.named_children(&mut elem_cursor).any(|elem| {
                        matches!(elem.kind(), "type_identifier" | "identifier")
                            && parsed.node_text(&elem).trim() == name
                    });
                    found
                });
            }
            let mut cursor = node.walk();
            let found = node
                .named_children(&mut cursor)
                .any(|child| receiver_args_bind(parsed, child, name));
            found
        }

        receiver_args_bind(self, receiver, receiver_type)
    }

    /// Whether a same-named function-local Go type declaration is lexically
    /// visible at this call. Package-qualified types cannot be shadowed by a
    /// local declaration and are rejected up front.
    pub(crate) fn go_local_type_shadows(
        &self,
        func_node: &Node<'_>,
        receiver_type: &str,
        call_start_byte: usize,
    ) -> bool {
        if self.language != Language::Go {
            return false;
        }
        let type_name = receiver_type
            .trim()
            .trim_start_matches('&')
            .trim_start_matches('*')
            .trim();
        if type_name.is_empty() || type_name.contains('.') || type_name.contains('[') {
            return false;
        }

        fn scope_contains_call(node: Node<'_>, call_start_byte: usize) -> bool {
            let mut parent = node.parent();
            while let Some(scope) = parent {
                if matches!(
                    scope.kind(),
                    "block" | "expression_case" | "type_case" | "communication_case"
                ) {
                    return scope.start_byte() <= call_start_byte
                        && call_start_byte < scope.end_byte();
                }
                parent = scope.parent();
            }
            false
        }

        fn walk(
            parsed: &ParsedFile,
            node: Node<'_>,
            type_name: &str,
            call_start_byte: usize,
        ) -> bool {
            if node.start_byte() >= call_start_byte {
                return false;
            }
            if matches!(node.kind(), "type_spec" | "type_alias")
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| parsed.node_text(&name).trim() == type_name)
                && scope_contains_call(node, call_start_byte)
            {
                return true;
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(parsed, child, type_name, call_start_byte));
            found
        }

        walk(self, *func_node, type_name, call_start_byte)
    }

    /// P11 S1: locate the (at most one, per `receiver_type_in_fn`'s own
    /// `bindings <= 1` shadow-safety gate) Go `short_var_declaration`
    /// statement that binds `receiver` at the FIRST LHS position with a
    /// single-call RHS, and return that call's raw callee text (`newDemux`,
    /// `pkg.New` — resolution to a static type is the caller's job,
    /// `go_receiver_index::resolve_go_return_type_call`). Returns `None` for
    /// every other shape: `receiver` at a non-first LHS position (`_, err :=
    /// f()` when probing `err`), a multi-expression RHS (`a, b := f(), g()`),
    /// or a non-call RHS.
    ///
    /// Callers MUST already have confirmed there is exactly one unshadowed
    /// receiver binding for this `(receiver, call_line, call_start_byte)`.
    /// This helper does not itself re-derive that count; it only relocates the
    /// single qualifying statement's RHS shape. The binding may be untyped or
    /// provisionally typed by Go's name-based `NewX` constructor heuristic.
    pub(crate) fn go_short_var_call_rhs(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
    ) -> Option<String> {
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            receiver: &str,
            call_line: usize,
            call_start_byte: usize,
        ) -> Option<String> {
            if node.start_position().row + 1 > call_line {
                return None;
            }
            if node.start_byte() >= call_start_byte {
                return None;
            }
            if !is_root && this.language.function_node_types().contains(&node.kind()) {
                return None; // same convention as walk_receiver_bindings.
            }
            // B1 (codex impl-review BLOCKER): `Language::function_node_types()`
            // omits Go function literals (closures), so the check above never
            // fences a closure body. Without this, a sibling closure's OWN
            // `x := newT()` binding — already closed by the time an unrelated
            // later call executes — would still be found here and misreported
            // as this call's qualifying statement. Same lexical-scope fence as
            // `walk_receiver_bindings`'s `func_literal` arm: skip this
            // closure's entire subtree unless the call site actually lies
            // within its span.
            if !is_root && node.kind() == "func_literal" {
                let call_inside =
                    call_start_byte >= node.start_byte() && call_start_byte < node.end_byte();
                if !call_inside {
                    return None;
                }
            }
            if node.kind() == "short_var_declaration" {
                if let (Some(left), Some(right)) = (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ) {
                    let mut lcur = left.walk();
                    let names: Vec<Node> = left.named_children(&mut lcur).collect();
                    if names
                        .first()
                        .map(|n| this.node_text(n).trim() == receiver)
                        .unwrap_or(false)
                    {
                        let mut rcur = right.walk();
                        let rhs: Vec<Node> = right.named_children(&mut rcur).collect();
                        if rhs.len() == 1 && rhs[0].kind() == "call_expression" {
                            if let Some(func) = rhs[0].child_by_field_name("function") {
                                if matches!(func.kind(), "identifier" | "selector_expression") {
                                    return Some(this.node_text(&func).trim().to_string());
                                }
                            }
                        }
                        return None; // receiver IS first, but RHS doesn't qualify.
                    }
                }
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(r) = walk(this, child, false, receiver, call_line, call_start_byte) {
                    return Some(r);
                }
            }
            None
        }
        walk(self, *func_node, true, receiver, call_line, call_start_byte)
    }

    /// P17 fix wave 3: collect every same-scope multi-name `:=` that REUSES
    /// `receiver` before the call. Go requires at least one other LHS name to be
    /// new in that block. Each reuse must assign the receiver from the first
    /// result of one call so the post-merge return-type index can prove that the
    /// static type did not change. `Err(())` is fail-closed evidence: a reuse was
    /// visible, but its RHS could not be compared.
    pub(crate) fn go_same_scope_short_var_reuse_calls(
        &self,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> Result<Vec<String>, ()> {
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            func_node: &Node<'_>,
            receiver: &str,
            call_start_byte: usize,
            out: &mut Vec<String>,
        ) -> Result<(), ()> {
            if node.start_byte() >= call_start_byte {
                return Ok(());
            }
            if !is_root && this.language.function_node_types().contains(&node.kind()) {
                return Ok(());
            }
            if !is_root && node.kind() == "func_literal" {
                let call_inside =
                    node.start_byte() <= call_start_byte && call_start_byte < node.end_byte();
                if !call_inside {
                    return Ok(());
                }
            }
            if node.kind() == "short_var_declaration"
                && this.go_short_decl_reuses_in_scope(node, func_node, receiver, call_start_byte)
            {
                let left = node.child_by_field_name("left").ok_or(())?;
                let names = this.go_short_decl_names(left);
                if names.first().map(String::as_str) != Some(receiver) {
                    return Err(());
                }
                let right = node.child_by_field_name("right").ok_or(())?;
                let mut rcur = right.walk();
                let rhs: Vec<Node> = right.named_children(&mut rcur).collect();
                if rhs.len() != 1 || rhs[0].kind() != "call_expression" {
                    return Err(());
                }
                let func = rhs[0].child_by_field_name("function").ok_or(())?;
                if !matches!(func.kind(), "identifier" | "selector_expression") {
                    return Err(());
                }
                out.push(this.node_text(&func).trim().to_string());
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk(
                    this,
                    child,
                    false,
                    func_node,
                    receiver,
                    call_start_byte,
                    out,
                )?;
            }
            Ok(())
        }

        let mut calls = Vec::new();
        walk(
            self,
            *func_node,
            true,
            func_node,
            receiver,
            call_start_byte,
            &mut calls,
        )?;
        Ok(calls)
    }

    /// Manual recursive function collection (pre-query fallback).
    /// `pub(crate)` for dual-path consistency testing in `queries::tests`.
    pub(crate) fn collect_functions_manual<'a>(&self, node: Node<'a>, out: &mut Vec<Node<'a>>) {
        let types = self.language.function_node_types();
        if types.contains(&node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_functions_manual(child, out);
        }
    }

    /// Find all identifiers (variable references) on a given line (1-indexed).
    ///
    /// Uses the manual recursive walk because it needs to match the broad set
    /// of node types in `Language::is_identifier_node()` (property_identifier,
    /// field_identifier, etc.), not just the core `identifier` type.
    pub fn identifiers_on_line(&self, line: usize) -> Vec<Node<'_>> {
        let row = line.saturating_sub(1);
        let mut result = Vec::new();
        self.collect_identifiers_at_row(self.tree.root_node(), row, &mut result);
        result
    }

    /// Manual recursive identifier-at-row collection (pre-query fallback).
    fn collect_identifiers_at_row<'a>(&self, node: Node<'a>, row: usize, out: &mut Vec<Node<'a>>) {
        if node.start_position().row == row && self.language.is_identifier_node(node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_at_row(child, row, out);
        }
    }

    /// Check whether `text` appears on `line` (1-indexed) in actual code,
    /// i.e. NOT inside a comment or string literal AST node.
    ///
    /// For each occurrence of `text` on the line, we ask tree-sitter for the
    /// smallest node covering that byte range. If every occurrence lands in a
    /// comment or string, returns `false`.
    pub fn line_has_code_text(&self, line: usize, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let row = line.saturating_sub(1);
        let source = self.source.as_bytes();

        // O(1) line→byte offset lookup via precomputed table.
        let line_start = self.line_offsets.get(row).copied().unwrap_or(source.len());
        let line_end = self
            .line_offsets
            .get(row + 1)
            .map(|&off| off.saturating_sub(1)) // exclude the newline
            .unwrap_or(source.len());
        let line_bytes = &source[line_start..line_end];
        let line_str = std::str::from_utf8(line_bytes).unwrap_or("");

        // For each occurrence of `text` in this line, check the AST node.
        let text_bytes = text.as_bytes();
        let mut search_start = 0;
        while let Some(pos) = line_str[search_start..].find(text) {
            let abs_start = line_start + search_start + pos;
            let abs_end = abs_start + text_bytes.len();

            // Find the smallest AST node covering this byte range.
            let node = self
                .tree
                .root_node()
                .descendant_for_byte_range(abs_start, abs_end);
            if let Some(n) = node {
                // Walk up to check if any ancestor (up to the line boundary) is
                // a comment or string. The immediate node might be an identifier
                // inside a string interpolation, so we check ancestors too.
                if !self.is_inside_comment_or_string(n) {
                    return true; // At least one occurrence is in real code.
                }
            } else {
                // No AST node found — conservative: treat as code.
                return true;
            }
            search_start += pos + text_bytes.len();
        }
        false
    }

    /// Walk up from `node` to check if it (or any ancestor) is a comment or
    /// string literal node.
    fn is_inside_comment_or_string(&self, node: Node<'_>) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if self.language.is_comment_or_string_node(n.kind()) {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// Extract import bindings from the file.
    ///
    /// Returns a map of `alias → module_path` where:
    /// - Python: `import utils` → `("utils", "utils")`, `from utils import func` → `("func", "utils")`
    /// - JS/TS: `import x from './mod'` → `("x", "./mod")`, `const x = require('./mod')` → `("x", "./mod")`
    /// - Go: `import "pkg"` → `("pkg", "pkg")`, `import alias "pkg"` → `("alias", "pkg")`
    ///
    /// Module paths are returned as-is (not resolved to filesystem paths).
    pub fn extract_imports(&self) -> BTreeMap<String, String> {
        let mut imports = BTreeMap::new();
        self.collect_imports(self.tree.root_node(), &mut imports);
        imports
    }

    fn collect_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match self.language {
            Language::Python => self.collect_python_imports(node, out),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                self.collect_js_imports(node, out)
            }
            Language::Go => self.collect_go_imports(node, out),
            _ => {} // C/C++/Rust/Java/Lua/Terraform/Bash: no module-qualified calls
        }
    }

    fn collect_python_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match node.kind() {
            // `import utils` or `import utils as u`
            "import_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "dotted_name" => {
                            let name = self.node_text(&child).to_string();
                            // Use last component as alias: `import os.path` → alias "path"
                            let alias = name.rsplit('.').next().unwrap_or(&name).to_string();
                            out.insert(alias, name);
                        }
                        "aliased_import" => {
                            let module = child
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = child
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let (Some(module), Some(alias)) = (module, alias) {
                                out.insert(alias, module);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // `from utils import func` or `from utils import func as f`
            "import_from_statement" => {
                let module = node
                    .child_by_field_name("module_name")
                    .map(|n| self.node_text(&n).to_string());
                let module = module.or_else(|| {
                    // tree-sitter-python uses different field names across versions
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "dotted_name" || child.kind() == "relative_import" {
                            return Some(self.node_text(&child).to_string());
                        }
                    }
                    None
                });
                if let Some(module) = module {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "dotted_name" | "identifier" => {
                                let name = self.node_text(&child).to_string();
                                // Skip the module name itself (first dotted_name)
                                if name != module {
                                    out.insert(name, module.clone());
                                }
                            }
                            "aliased_import" => {
                                let alias = child
                                    .child_by_field_name("alias")
                                    .map(|n| self.node_text(&n).to_string());
                                if let Some(alias) = alias {
                                    out.insert(alias, module.clone());
                                }
                            }
                            "wildcard_import" => {
                                out.insert("*".to_string(), "*".to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_python_imports(child, out);
                }
            }
        }
    }

    fn collect_js_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
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
                if let Some(func_name) = self.language.call_function_name(&val) {
                    if self.node_text(&func_name) == "require" {
                        if let Some(args) = self.language.call_arguments(&val) {
                            // Extract the module path from first argument
                            let mut cursor = args.walk();
                            for child in args.children(&mut cursor) {
                                if child.is_named() {
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
        let func_name = self.language.call_function_name(node)?;
        if self.node_text(&func_name) != "require" {
            return None;
        }
        let args = self.language.call_arguments(node)?;
        let mut cursor = args.walk();
        for child in args.children(&mut cursor) {
            if child.is_named() {
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

    // -------------------------------------------------------------------
    // R4c: structured import-binding extraction (parallel to extract_imports)
    // -------------------------------------------------------------------

    /// Extract structured import bindings from Python/JS/TS files.
    /// Returns one `ImportBinding` per import clause.
    ///
    /// Only collects module-scope imports: direct children of the module root,
    /// plus imports inside module-scope compound statements (if/try/for/while/with)
    /// which ARE module-scope in Python. Function-local and class-nested imports
    /// are excluded — they don't create file-wide bindings.
    pub fn extract_import_bindings(&self) -> Vec<crate::call_graph::ImportBinding> {
        let mut out = Vec::new();
        match self.language {
            Language::Python => {
                let root = self.tree.root_node();
                let mut cursor = root.walk();
                for child in root.children(&mut cursor) {
                    self.collect_python_module_scope_imports(child, &mut out);
                }
            }
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                // ES module imports are syntactically top-level only; walk
                // direct children of the root `program` node.
                let root = self.tree.root_node();
                let mut cursor = root.walk();
                for child in root.children(&mut cursor) {
                    self.collect_js_import_bindings_node(child, &mut out);
                }
            }
            _ => {}
        }
        out
    }

    /// Collect imports from a single module-scope Python node.
    ///
    /// Called for each direct child of the root. If the node is a compound
    /// statement (`if`/`try`/`for`/`while`/`with`), we walk one level into
    /// their block/clause children for imports — those ARE module-scope in
    /// Python. We do NOT recurse into `function_definition` or
    /// `class_definition` bodies (their imports are function-/class-local).
    fn collect_python_module_scope_imports(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        match node.kind() {
            "import_statement" | "import_from_statement" => {
                self.collect_python_import_node(node, out);
            }
            // Module-scope compound statements can contain imports that are
            // still module-scope (e.g. `if TYPE_CHECKING: from x import y`).
            "if_statement" | "try_statement" | "for_statement" | "while_statement"
            | "with_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "block" | "except_clause" | "finally_clause" | "else_clause" => {
                            let mut bc = child.walk();
                            for stmt in child.children(&mut bc) {
                                if stmt.kind() == "import_statement"
                                    || stmt.kind() == "import_from_statement"
                                {
                                    self.collect_python_import_node(stmt, out);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Do NOT walk into function_definition, class_definition,
            // decorated_definition — their imports are not module-scope.
            _ => {}
        }
    }

    /// Extract binding data from a single Python import/import_from node.
    fn collect_python_import_node(
        &self,
        node: Node<'_>,
        out: &mut Vec<crate::call_graph::ImportBinding>,
    ) {
        use crate::call_graph::{ImportBinding, ImportBindingKind};
        match node.kind() {
            "import_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "dotted_name" => {
                            let name = self.node_text(&child).to_string();
                            // Python binds the root for an unaliased dotted import:
                            // `import pkg.models` creates local `pkg`, not `models`.
                            let alias = name.split('.').next().unwrap_or(&name).to_string();
                            out.push(ImportBinding {
                                local: alias,
                                module_path: name,
                                member: None,
                                kind: ImportBindingKind::ModuleImport,
                                eligible: false, // module imports don't resolve unqualified calls
                            });
                        }
                        "aliased_import" => {
                            let module = child
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = child
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            if let (Some(module), Some(alias)) = (module, alias) {
                                out.push(ImportBinding {
                                    local: alias,
                                    module_path: module,
                                    member: None,
                                    kind: ImportBindingKind::AliasedModuleImport,
                                    eligible: false,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            "import_from_statement" => {
                let module = node
                    .child_by_field_name("module_name")
                    .map(|n| self.node_text(&n).to_string())
                    .or_else(|| {
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            if child.kind() == "dotted_name" || child.kind() == "relative_import" {
                                return Some(self.node_text(&child).to_string());
                            }
                        }
                        None
                    });
                if let Some(module) = module {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "dotted_name" | "identifier" => {
                                let name = self.node_text(&child).to_string();
                                if name != module {
                                    out.push(ImportBinding {
                                        local: name.clone(),
                                        module_path: module.clone(),
                                        member: Some(name),
                                        kind: ImportBindingKind::MemberImport,
                                        eligible: true, // eligibility set later
                                    });
                                }
                            }
                            "aliased_import" => {
                                let original = child
                                    .child_by_field_name("name")
                                    .map(|n| self.node_text(&n).to_string());
                                let alias = child
                                    .child_by_field_name("alias")
                                    .map(|n| self.node_text(&n).to_string());
                                if let Some(alias) = alias {
                                    out.push(ImportBinding {
                                        local: alias,
                                        module_path: module.clone(),
                                        member: original,
                                        kind: ImportBindingKind::MemberImport,
                                        eligible: true,
                                    });
                                }
                            }
                            "wildcard_import" => {
                                out.push(ImportBinding {
                                    local: "*".to_string(),
                                    module_path: module.clone(),
                                    member: None,
                                    kind: ImportBindingKind::WildcardImport,
                                    eligible: false,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect import binding data from a single JS/TS `import_statement` node.
    /// Called only for direct children of the root `program` node.
    fn collect_js_import_bindings_node(
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

    fn collect_js_import_statement_bindings(
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
                            let name = spec
                                .child_by_field_name("name")
                                .map(|n| self.node_text(&n).to_string());
                            let alias = spec
                                .child_by_field_name("alias")
                                .map(|n| self.node_text(&n).to_string());
                            let local = alias.clone().or_else(|| name.clone());
                            if let Some(local) = local {
                                out.push(ImportBinding {
                                    local,
                                    module_path: module_path.to_string(),
                                    member: name,
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

    fn js_ts_import_statement_is_type_only(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| child.kind() == "type");
        found
    }

    fn js_ts_import_specifier_is_type_only(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| child.kind() == "type");
        found
    }

    // -------------------------------------------------------------------
    // R4c: module-scope binding extraction (occurrence-clean eligibility)
    // -------------------------------------------------------------------

    /// Extract module-scope binding kinds from Python/JS/TS files.
    /// Walks direct children of the module root AND descends into Python
    /// compound statements (`if`/`try`/`for`/`while`/`with`) whose bodies
    /// are still module-scope.
    pub fn extract_module_bindings(
        &self,
    ) -> BTreeMap<String, crate::call_graph::ModuleBindingKind> {
        let mut out = BTreeMap::new();
        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            Self::extract_module_bindings_from_stmt(child, &self.language, &self.source, &mut out);
        }
        // Walk compound statement bodies (still module-scope in Python).
        let mut cursor2 = root.walk();
        for child in root.children(&mut cursor2) {
            Self::descend_compound_for_bindings(child, &self.language, &self.source, &mut out);
        }
        out
    }

    /// An inert initializer cannot install a competing package attribute or
    /// execute an import hook. Deliberately narrower than a binding-name scan.
    pub(crate) fn python_inert_initializer(&self) -> bool {
        if self.language != Language::Python {
            return false;
        }
        let root = self.tree.root_node();
        if root.has_error() {
            return false;
        }
        let mut cursor = root.walk();
        let inert = root.named_children(&mut cursor).all(|node| {
            if matches!(node.kind(), "comment" | "pass_statement") {
                return true;
            }
            if node.kind() != "expression_statement" || node.named_child_count() != 1 {
                return false;
            }
            let Some(value) = node.named_child(0) else {
                return false;
            };
            if value.kind() != "string" {
                return false;
            }
            let text = self.node_text(&value);
            let prefix = text
                .split(['\'', '"'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(prefix.as_str(), "" | "r" | "u") {
                return false;
            }
            let mut cursor = value.walk();
            let plain = value
                .named_children(&mut cursor)
                .all(|n| n.kind() != "interpolation");
            plain
        });
        inert
    }

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
    pub fn extract_js_ts_export_facts(&self) -> crate::js_exports::JsExportFacts {
        let mut facts = crate::js_exports::JsExportFacts::default();
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return facts;
        }

        let root = self.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "export_statement" => self.collect_js_ts_export_statement(child, &mut facts),
                "expression_statement" => {
                    self.collect_js_ts_cjs_export_statement(child, &mut facts)
                }
                _ => {}
            }
        }
        facts.esm_named_imports = self.js_ts_esm_named_imports(None);
        facts.type_only_imports = self.js_ts_type_only_imports();
        facts
    }

    fn js_ts_type_only_imports(&self) -> BTreeMap<String, Option<(String, String)>> {
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

    fn js_ts_esm_named_imports(&self, only: Option<&str>) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let root = self.tree.root_node();
        if root.has_error() {
            return names;
        }
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            if node.kind() != "import_statement" || self.js_ts_import_statement_is_type_only(node) {
                continue;
            }
            let mut imports = Vec::new();
            self.collect_js_import_statement_bindings(node, &mut imports);
            names.extend(
                imports
                    .into_iter()
                    .filter(|b| {
                        b.kind == crate::call_graph::ImportBindingKind::MemberImport
                            && only.is_none_or(|name| b.local == name)
                    })
                    .map(|b| b.local),
            );
        }
        names.retain(|name| !self.js_ts_module_value_written(name));
        names
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
    fn collect_js_ts_export_statement(
        &self,
        node: Node<'_>,
        facts: &mut crate::js_exports::JsExportFacts,
    ) {
        use crate::js_exports::JsExportTarget;

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
                "namespace_export" => has_namespace_export = true,
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
                if spec.kind() != "export_specifier" {
                    continue;
                }
                let Some(name) = spec
                    .child_by_field_name("name")
                    .map(|n| self.node_text(&n).to_string())
                else {
                    continue;
                };
                let alias = spec
                    .child_by_field_name("alias")
                    .map(|n| self.node_text(&n).to_string());
                let exported_as = alias.unwrap_or_else(|| name.clone());
                let target = match &source {
                    Some(module_path) => JsExportTarget::ReExport {
                        module_path: module_path.clone(),
                        imported: name,
                    },
                    None => JsExportTarget::Local(name),
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
                facts.insert_named("default".to_string(), JsExportTarget::Local(name));
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
                        facts.insert_named(exported_as, JsExportTarget::Local(name_text));
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
                        match d.child_by_field_name("value").map(|v| v.kind()) {
                            Some("arrow_function") | Some("function_expression") => {
                                facts.insert_named(
                                    name_text.clone(),
                                    JsExportTarget::Local(name_text),
                                );
                            }
                            _ => facts.skipped_expr_count += 1,
                        }
                    }
                }
                // Class exports are deliberately out of scope.
                _ => {}
            }
        }
    }

    /// Handle a single top-level `expression_statement`, looking for CommonJS
    /// `module.exports = ...` / `module.exports.f = ...` / `exports.f = ...`
    /// assignments.
    fn collect_js_ts_cjs_export_statement(
        &self,
        node: Node<'_>,
        facts: &mut crate::js_exports::JsExportFacts,
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
            // `module.exports = <rhs>;`
            self.collect_js_ts_cjs_module_exports_rhs(right, facts);
            return;
        }

        let is_module_exports_member = object.kind() == "member_expression"
            && object
                .child_by_field_name("object")
                .is_some_and(|o| o.kind() == "identifier" && self.node_text(&o) == "module")
            && object
                .child_by_field_name("property")
                .is_some_and(|p| self.node_text(&p) == "exports");
        let is_exports_member =
            object.kind() == "identifier" && self.node_text(&object) == "exports";

        if is_module_exports_member || is_exports_member {
            // `module.exports.f = f;` or `exports.f = f;`
            if right.kind() == "identifier" {
                facts.insert_named(
                    property_name,
                    crate::js_exports::JsExportTarget::Local(self.node_text(&right).to_string()),
                );
            } else {
                facts.skipped_expr_count += 1;
            }
        }
    }

    /// `module.exports = <rhs>;` — either a single identifier (the module's
    /// whole export value, consumed like a default export) or an object
    /// literal of named members (`{ a, x: b }`).
    fn collect_js_ts_cjs_module_exports_rhs(
        &self,
        rhs: Node<'_>,
        facts: &mut crate::js_exports::JsExportFacts,
    ) {
        use crate::js_exports::JsExportTarget;
        match rhs.kind() {
            "identifier" => {
                facts.insert_named(
                    "default".to_string(),
                    JsExportTarget::Local(self.node_text(&rhs).to_string()),
                );
            }
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
                            facts.insert_named(name.clone(), JsExportTarget::Local(name));
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
                                    facts.insert_named(
                                        key,
                                        JsExportTarget::Local(self.node_text(&value).to_string()),
                                    );
                                }
                                (Some(_), Some(_)) => facts.skipped_expr_count += 1,
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

    /// Conservative JS/TS function-scope bindings used to avoid false exact
    /// import-member edges when an imported local is shadowed.
    pub fn js_ts_function_local_bindings(&self, func_node: &Node<'_>) -> BTreeSet<String> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return BTreeSet::new();
        }

        let mut out: BTreeSet<String> = BTreeSet::new();
        self.collect_js_ts_parameter_bindings(*func_node, &mut out);
        let root_key = (
            func_node.start_byte(),
            func_node.end_byte(),
            func_node.kind_id(),
        );
        self.collect_js_ts_local_bindings(*func_node, root_key, &mut out);
        out
    }

    /// Whether a simple JS/TS receiver identifier is bound in the lexical scope
    /// that contains this call. This is deliberately a binding fact, not type
    /// recovery: later receiver classification may consume it, while import-
    /// qualified resolution uses it only as a fail-closed shadow guard.
    pub fn js_ts_receiver_lexically_bound_at_call(
        &self,
        func_node: &Node<'_>,
        receiver_node: Option<Node<'_>>,
    ) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let Some(receiver) = receiver_node else {
            return false;
        };
        if receiver.kind() != "identifier" {
            return false;
        }
        let receiver_name = self.node_text(&receiver);
        if !is_plain_ident(receiver_name) {
            return false;
        }

        let mut seen_functions = BTreeSet::new();
        let mut current = Some(receiver);
        while let Some(node) = current {
            if is_js_ts_function_like(node.kind()) && seen_functions.insert(node.id()) {
                if self.js_ts_function_scope_binds_receiver(&node, &receiver, receiver_name) {
                    return true;
                }
            }
            current = node.parent();
        }

        if seen_functions.insert(func_node.id())
            && self.js_ts_function_scope_binds_receiver(func_node, &receiver, receiver_name)
        {
            return true;
        }

        let root = self.tree.root_node();
        self.js_ts_receiver_binding_reaches_call(root, root.id(), &receiver, receiver_name)
    }

    /// Call-position-aware JS/TS value-binding evidence for receiver recovery.
    /// The nearest reaching lexical scope wins. Constructor origins are limited
    /// to the call's innermost function; typed parameters may be captured by a
    /// nested callable because their static type remains declared by the outer
    /// signature.
    pub(crate) fn js_ts_receiver_binding_evidence_at_call(
        &self,
        func_node: &Node<'_>,
        receiver_node: Option<Node<'_>>,
    ) -> Option<JsTsReceiverBindingEvidence> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return None;
        }
        let receiver = receiver_node?;
        if receiver.kind() == "member_expression"
            && receiver
                .child_by_field_name("object")
                .is_some_and(|n| n.kind() == "this")
        {
            return Some(self.js_ts_constructor_field_type(receiver).map_or(
                JsTsReceiverBindingEvidence::Materialized,
                |static_type| JsTsReceiverBindingEvidence::Recovered {
                    static_type,
                    recovery: crate::resolution::ReceiverRecovery::FieldTyped,
                    declaration_end_byte: None,
                },
            ));
        }
        if receiver.kind() != "identifier" {
            return None;
        }
        let receiver_name = self.node_text(&receiver);
        if !is_plain_ident(receiver_name) {
            return None;
        }

        let mut seen_functions = BTreeSet::new();
        let mut current = Some(receiver);
        let mut function_rank = 0usize;
        while let Some(node) = current {
            if is_js_ts_function_like(node.kind()) && seen_functions.insert(node.id()) {
                if let Some(evidence) = self.js_ts_scope_receiver_binding_evidence(
                    node,
                    &receiver,
                    receiver_name,
                    function_rank == 0,
                    true,
                ) {
                    return Some(evidence);
                }
                function_rank += 1;
            }
            current = node.parent();
        }

        if seen_functions.insert(func_node.id()) {
            if let Some(evidence) = self.js_ts_scope_receiver_binding_evidence(
                *func_node,
                &receiver,
                receiver_name,
                false,
                true,
            ) {
                return Some(evidence);
            }
        }

        let root = self.tree.root_node();
        self.js_ts_scope_receiver_binding_evidence(root, &receiver, receiver_name, false, false)
            .or_else(|| {
                self.js_ts_receiver_lexically_bound_at_call(func_node, Some(receiver))
                    .then_some(JsTsReceiverBindingEvidence::Materialized)
            })
    }

    /// Own constructor assignment establishes a bounded instance-field invariant.
    /// Dynamic-this callables and field initializer evaluation are not instances.
    fn js_ts_constructor_field_type(&self, receiver: Node<'_>) -> Option<String> {
        let field = receiver.child_by_field_name("property")?;
        if field.kind() != "property_identifier" {
            return None;
        }
        let field = self.node_text(&field);
        let mut current = receiver.parent();
        let member = loop {
            let node = current?;
            if node.kind() == "method_definition" {
                if node.parent()?.kind() != "class_body" {
                    return None;
                }
                break node;
            }
            if node.kind() == "arrow_function" {
                let parent = node.parent()?;
                if matches!(
                    parent.kind(),
                    "field_definition" | "public_field_definition"
                ) && parent.parent()?.kind() == "class_body"
                    && parent.child_by_field_name("value") == Some(node)
                {
                    break parent;
                }
            } else if is_js_ts_function_like(node.kind()) || node.kind() == "class_body" {
                return None;
            }
            current = node.parent();
        };
        if self.js_ts_method_is_static(&member) {
            return None;
        }
        if let Some(params) = member.child_by_field_name("parameters") {
            let mut cursor = params.walk();
            if params.named_children(&mut cursor).any(|p| {
                p.child_by_field_name("pattern")
                    .or_else(|| p.child_by_field_name("name"))
                    .is_some_and(|n| self.node_text(&n) == "this")
            }) {
                return None;
            }
        }
        let body = member.parent()?;
        let class = body.parent()?;
        let mut cursor = class.walk();
        if class.kind() != "class_declaration"
            || body.has_error()
            || class.children(&mut cursor).any(|n| n.kind() == "decorator")
        {
            return None;
        }
        let mut constructor = None;
        let mut slots = 0;
        let mut cursor = body.walk();
        for slot in body.named_children(&mut cursor) {
            if self.js_ts_method_is_static(&slot) {
                continue;
            }
            let key = slot
                .child_by_field_name("name")
                .or_else(|| slot.child_by_field_name("property"));
            if slot.kind() == "index_signature" {
                return None;
            }
            let Some(key) = key else {
                continue;
            };
            if key.kind() == "computed_property_name" {
                return None;
            }
            if self.node_text(&key) == "constructor" {
                if slot.kind() != "method_definition" || constructor.replace(slot).is_some() {
                    return None;
                }
            }
            if self.node_text(&key).trim_matches(['\'', '"']) == field {
                slots += 1;
                let mut cursor = slot.walk();
                if slots > 1
                    || key.kind() != "property_identifier"
                    || !matches!(slot.kind(), "field_definition" | "public_field_definition")
                    || slot.child_by_field_name("value").is_some()
                    || slot.children(&mut cursor).any(|n| {
                        n.kind() == "decorator"
                            || n.kind() == "?"
                            || matches!(
                                self.node_text(&n),
                                "private" | "protected" | "declare" | "abstract"
                            )
                    })
                {
                    return None;
                }
            }
        }
        // Without an own field, a base-class accessor can intercept assignment.
        let mut cursor = class.walk();
        if slots == 0
            && class
                .named_children(&mut cursor)
                .any(|n| n.kind() == "class_heritage")
        {
            return None;
        }
        let constructor = constructor?;
        let ctor_body = constructor.child_by_field_name("body")?;
        fn returns(node: Node<'_>) -> bool {
            if is_js_ts_function_like(node.kind()) {
                return false;
            }
            if node.kind() == "return_statement" {
                return true;
            }
            let mut cursor = node.walk();
            let found = node.named_children(&mut cursor).any(returns);
            found
        }
        if returns(ctor_body) {
            return None;
        }
        let mut initialization = None;
        let mut cursor = ctor_body.walk();
        for statement in ctor_body.named_children(&mut cursor) {
            if statement.kind() != "expression_statement" {
                continue;
            }
            let Some(assignment) = statement
                .named_child(0)
                .filter(|n| n.kind() == "assignment_expression")
            else {
                continue;
            };
            let left = assignment.child_by_field_name("left")?;
            if left.kind() == "member_expression"
                && left
                    .child_by_field_name("object")
                    .is_some_and(|n| n.kind() == "this")
                && left.child_by_field_name("property").is_some_and(|n| {
                    n.kind() == "property_identifier" && self.node_text(&n) == field
                })
                && initialization.replace(assignment).is_some()
            {
                return None;
            }
        }
        let initialization = initialization?;
        if member == constructor && receiver.start_byte() < initialization.end_byte() {
            return None;
        }
        if self.js_ts_constructor_field_written(body, field, initialization.id()) {
            return None;
        }
        self.js_ts_direct_new_constructor(initialization.child_by_field_name("right")?, constructor)
    }

    /// Include nested member writes and destructuring without text-based object
    /// comparison, so whitespace/parentheses cannot hide a field mutation.
    fn js_ts_target_writes_this_field(&self, target: Node<'_>, field: &str) -> bool {
        if self.js_ts_target_writes_member(target, "this", Some(field)) {
            return true;
        }
        if matches!(target.kind(), "member_expression" | "subscript_expression") {
            return target
                .child_by_field_name("object")
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field));
        }
        if matches!(
            target.kind(),
            "pair_pattern" | "assignment_pattern" | "object_assignment_pattern"
        ) {
            return target
                .child_by_field_name(if target.kind() == "pair_pattern" {
                    "value"
                } else {
                    "left"
                })
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field));
        }
        if matches!(
            target.kind(),
            "object_pattern" | "array_pattern" | "rest_pattern" | "parenthesized_expression"
        ) {
            let mut cursor = target.walk();
            return target
                .named_children(&mut cursor)
                .any(|n| self.js_ts_target_writes_this_field(n, field));
        }
        false
    }

    fn js_ts_constructor_field_written(
        &self,
        node: Node<'_>,
        field: &str,
        initialization: usize,
    ) -> bool {
        if node.id() != initialization
            && self
                .js_ts_write_target(node)
                .is_some_and(|n| self.js_ts_target_writes_this_field(n, field))
        {
            return true;
        }
        // Explicit reflective mutators are barriers; this is not alias analysis.
        if node.kind() == "call_expression" {
            if let Some(callee) = node
                .child_by_field_name("function")
                .filter(|n| matches!(n.kind(), "member_expression" | "subscript_expression"))
            {
                let object = callee
                    .child_by_field_name("object")
                    .map(|n| self.node_text(&n));
                let method = callee
                    .child_by_field_name("property")
                    .or_else(|| callee.child_by_field_name("index"))
                    .map(|n| self.node_text(&n).trim_matches(['\'', '"']));
                if matches!(object, Some("Object" | "Reflect"))
                    && matches!(
                        method,
                        Some(
                            "assign"
                                | "defineProperty"
                                | "defineProperties"
                                | "set"
                                | "deleteProperty"
                                | "setPrototypeOf"
                        )
                    )
                    && node
                        .child_by_field_name("arguments")
                        .and_then(|n| n.named_child(0))
                        .is_some_and(|mut n| {
                            while n.kind() == "parenthesized_expression" {
                                let Some(inner) = n.named_child(0) else {
                                    return true;
                                };
                                n = inner;
                            }
                            n.kind() == "this" || self.js_ts_target_writes_this_field(n, field)
                        })
                {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let written = node
            .named_children(&mut cursor)
            .any(|n| self.js_ts_constructor_field_written(n, field, initialization));
        written
    }

    fn js_ts_scope_receiver_binding_evidence(
        &self,
        root_scope: Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
        allow_constructor: bool,
        include_parameters: bool,
    ) -> Option<JsTsReceiverBindingEvidence> {
        let mut candidates = Vec::new();
        let mut parse_recovery = false;
        self.collect_js_ts_receiver_binding_candidates(
            root_scope,
            true,
            root_scope,
            receiver,
            receiver_name,
            allow_constructor,
            &mut candidates,
            &mut parse_recovery,
        );

        if include_parameters {
            if matches!(
                root_scope.kind(),
                "function_declaration"
                    | "function_expression"
                    | "generator_function_declaration"
                    | "generator_function"
            ) && self
                .language
                .function_name(&root_scope)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(distance) = self.js_ts_scope_distance(receiver, root_scope.id()) {
                    candidates.push((
                        distance,
                        root_scope.id(),
                        JsTsReceiverBindingEvidence::Materialized,
                    ));
                }
            }
            if let Some(evidence) = self.js_ts_parameter_receiver_binding(root_scope, receiver_name)
            {
                if let Some(distance) = self.js_ts_scope_distance(receiver, root_scope.id()) {
                    candidates.push((distance, root_scope.id(), evidence));
                }
            }
        }

        let nearest = candidates.iter().map(|(distance, _, _)| *distance).min()?;
        let nearest_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|(distance, _, _)| *distance == nearest)
            .map(|(_, scope_id, evidence)| (scope_id, evidence))
            .collect();
        if parse_recovery {
            return Some(JsTsReceiverBindingEvidence::Materialized);
        }
        if nearest_candidates.len() != 1 {
            return Some(
                if nearest_candidates.iter().all(|(_, evidence)| {
                    matches!(evidence, JsTsReceiverBindingEvidence::ClassOwner)
                }) {
                    JsTsReceiverBindingEvidence::ClassOwner
                } else {
                    JsTsReceiverBindingEvidence::Materialized
                },
            );
        }
        let (binding_scope_id, evidence) = nearest_candidates.into_iter().next()?;
        if let JsTsReceiverBindingEvidence::Recovered {
            declaration_end_byte: Some(declaration_end_byte),
            ..
        } = &evidence
        {
            if self.js_ts_receiver_written_between(
                root_scope,
                binding_scope_id,
                receiver,
                receiver_name,
                *declaration_end_byte,
            ) {
                return Some(JsTsReceiverBindingEvidence::Materialized);
            }
        }
        Some(evidence)
    }

    fn js_ts_parameter_receiver_binding(
        &self,
        func_node: Node<'_>,
        receiver_name: &str,
    ) -> Option<JsTsReceiverBindingEvidence> {
        let params = self.find_parameters_node(&func_node)?;
        let mut matches = Vec::new();
        let mut cursor = params.walk();
        for parameter in params.named_children(&mut cursor) {
            let mut names = BTreeSet::new();
            self.collect_js_ts_parameter_binding_names(parameter, &mut names);
            if !names.contains(receiver_name) {
                continue;
            }
            let binding = parameter
                .child_by_field_name("pattern")
                .or_else(|| parameter.child_by_field_name("name"))
                .or_else(|| parameter.child_by_field_name("left"))
                .or_else(|| (parameter.kind() == "identifier").then_some(parameter));
            let simple_binding = binding.is_some_and(|binding| {
                binding.kind() == "identifier" && self.node_text(&binding) == receiver_name
            });
            let recovered_type = matches!(self.language, Language::TypeScript | Language::Tsx)
                .then(|| {
                    if simple_binding {
                        self.js_ts_simple_type_annotation(parameter.child_by_field_name("type")?)
                    } else {
                        // An explicit annotation is terminal even when unsupported.
                        let ty = match parameter.child_by_field_name("type") {
                            Some(annotation) => annotation.named_child(0)?,
                            None => self.js_ts_contextual_parameter_type(func_node)?,
                        };
                        self.js_ts_inline_prop_receiver_type(parameter, binding?, ty, receiver_name)
                    }
                })
                .flatten();
            matches.push(recovered_type.map_or(
                JsTsReceiverBindingEvidence::Materialized,
                |static_type| JsTsReceiverBindingEvidence::Recovered {
                    static_type,
                    recovery: crate::resolution::ReceiverRecovery::TypedParam,
                    declaration_end_byte: Some(parameter.end_byte()),
                },
            ));
        }
        match matches.len() {
            0 => None,
            1 => matches.pop(),
            _ => Some(JsTsReceiverBindingEvidence::Materialized),
        }
    }

    /// Keep declaration nodes for ordinary signatures and use-site argument nodes
    /// for bounded F<P> = (p: P) instantiation; never substitute name strings.
    fn js_ts_contextual_parameter_type<'a>(&'a self, func: Node<'a>) -> Option<Node<'a>> {
        fn single_child(params: Node<'_>) -> Option<Node<'_>> {
            let mut cursor = params.walk();
            let mut children = params
                .named_children(&mut cursor)
                .filter(|n| n.kind() != "comment");
            let child = children.next()?;
            children.next().is_none().then_some(child)
        }
        fn single_parameter(params: Node<'_>) -> Option<Node<'_>> {
            single_child(params).filter(|n| n.kind() == "required_parameter")
        }
        if !matches!(func.kind(), "arrow_function" | "function_expression")
            || func.child_by_field_name("type_parameters").is_some()
        {
            return None;
        }
        let implementation = single_parameter(func.child_by_field_name("parameters")?)?;
        if implementation.child_by_field_name("type").is_some() {
            return None;
        }
        let declaration = func.parent()?;
        if declaration.kind() != "variable_declarator"
            || declaration.has_error()
            || declaration.child_by_field_name("value")? != func
            || declaration.child_by_field_name("name")?.kind() != "identifier"
        {
            return None;
        }
        let reference = declaration.child_by_field_name("type")?.named_child(0)?;
        let (signature, substitution) = if reference.kind() == "generic_type" {
            let alias = self.js_ts_local_type_alias(reference.child_by_field_name("name")?)?;
            let binder = single_child(alias.child_by_field_name("type_parameters")?)?;
            let name = binder.child_by_field_name("name")?;
            let mut cursor = binder.walk();
            if binder.kind() != "type_parameter"
                || name.kind() != "type_identifier"
                || !is_plain_ident(self.node_text(&name))
                || binder
                    .children(&mut cursor)
                    .any(|n| n != name && n.kind() != "comment")
            {
                return None;
            }
            let argument = single_child(reference.child_by_field_name("type_arguments")?)?;
            (alias.child_by_field_name("value")?, Some((name, argument)))
        } else {
            (
                self.js_ts_local_type_shape(reference, "function_type")?,
                None,
            )
        };
        if signature.kind() != "function_type"
            || signature.child_by_field_name("type_parameters").is_some()
        {
            return None;
        }
        let contextual = single_parameter(signature.child_by_field_name("parameters")?)?;
        if contextual.child_by_field_name("pattern")?.kind() != "identifier" {
            return None;
        }
        let mut cursor = contextual.walk();
        if contextual
            .children(&mut cursor)
            .any(|n| matches!(n.kind(), "=" | "?"))
        {
            return None;
        }
        let ty = contextual.child_by_field_name("type")?.named_child(0)?;
        if let Some((binder, argument)) = substitution {
            // Only the entire parameter type can be substituted, not a nested
            // member/union/alias. The signature has no nearer generic binder.
            (ty.kind() == "type_identifier" && self.node_text(&ty) == self.node_text(&binder))
                .then_some(argument)
        } else {
            Some(ty)
        }
    }

    /// Preserve the original shape node: its class names belong to the
    /// declaration scope, while receiver writes belong to the implementation.
    /// One module-local hop only; recursive/generic/ambient authority is excluded.
    fn js_ts_local_type_shape<'a>(
        &'a self,
        reference: Node<'a>,
        expected_kind: &str,
    ) -> Option<Node<'a>> {
        if reference.kind() == expected_kind {
            return Some(reference);
        }
        let alias = self.js_ts_local_type_alias(reference)?;
        if alias.child_by_field_name("type_parameters").is_some() {
            return None;
        }
        let shape = alias.child_by_field_name("value")?;
        (shape.kind() == expected_kind).then_some(shape)
    }

    /// Declaration identity only. Each consumer separately proves its supported
    /// binder count and RHS shape; this helper never follows another alias.
    fn js_ts_local_type_alias<'a>(&'a self, reference: Node<'a>) -> Option<Node<'a>> {
        let root = self.tree.root_node();
        let name = self.node_text(&reference);
        if reference.kind() != "type_identifier"
            || !is_plain_ident(name)
            || root.has_error()
            || self.js_ts_type_name_shadowed(root, &reference, name)
            || self.js_ts_type_only_imports().contains_key(name)
            || self
                .extract_import_bindings()
                .iter()
                .any(|b| b.local == name)
        {
            return None;
        }
        let mut alias = None;
        let mut cursor = root.walk();
        for mut declaration in root.named_children(&mut cursor) {
            let mut ambient = false;
            while matches!(
                declaration.kind(),
                "export_statement" | "ambient_declaration"
            ) {
                ambient |= declaration.kind() == "ambient_declaration";
                declaration = declaration
                    .child_by_field_name("declaration")
                    .or_else(|| declaration.named_child(0))?;
            }
            if !matches!(
                declaration.kind(),
                "type_alias_declaration"
                    | "interface_declaration"
                    | "class_declaration"
                    | "abstract_class_declaration"
                    | "enum_declaration"
                    | "internal_module"
                    | "module"
                    | "import_alias"
            ) {
                continue;
            }
            let declared = declaration.child_by_field_name("name").or_else(|| {
                (declaration.kind() == "import_alias")
                    .then(|| declaration.named_child(0))
                    .flatten()
            });
            if !declared.is_some_and(|n| self.node_text(&n).split('.').next() == Some(name)) {
                continue;
            }
            if ambient
                || declaration.kind() != "type_alias_declaration"
                || alias.replace(declaration).is_some()
            {
                return None;
            }
        }
        alias
    }

    /// Required properties in a direct or local-alias object shape; declaration
    /// type names and implementation bindings retain distinct source positions.
    fn js_ts_inline_prop_receiver_type(
        &self,
        parameter: Node<'_>,
        pattern: Node<'_>,
        ty: Node<'_>,
        receiver: &str,
    ) -> Option<String> {
        if pattern.kind() != "object_pattern" || parameter.has_error() {
            return None;
        }
        let mut cursor = parameter.walk();
        if parameter
            .children(&mut cursor)
            .any(|n| matches!(n.kind(), "=" | "?"))
        {
            return None;
        }
        let ty = self.js_ts_local_type_shape(ty, "object_type")?;
        let mut properties = BTreeSet::new();
        let mut locals = BTreeSet::new();
        let mut selected = None;
        let mut cursor = pattern.walk();
        for child in pattern.named_children(&mut cursor) {
            if child.kind() == "comment" {
                continue;
            }
            let (property, local) = match child.kind() {
                "shorthand_property_identifier_pattern" => (child, child),
                "pair_pattern" => {
                    let property = child.child_by_field_name("key")?;
                    let local = child.child_by_field_name("value")?;
                    if property.kind() != "property_identifier" || local.kind() != "identifier" {
                        return None;
                    }
                    (property, local)
                }
                _ => return None,
            };
            let property = self.node_text(&property);
            let local = self.node_text(&local);
            if !properties.insert(property) || !locals.insert(local) {
                return None;
            }
            if local == receiver {
                selected = Some(property);
            }
        }
        let selected = selected?;
        let mut names = BTreeSet::new();
        let mut result = None;
        let mut cursor = ty.walk();
        for property in ty.named_children(&mut cursor) {
            if property.kind() == "comment" {
                continue;
            }
            if property.kind() != "property_signature" {
                return None;
            }
            let name = property.child_by_field_name("name")?;
            if name.kind() != "property_identifier" {
                return None;
            }
            let name = self.node_text(&name);
            if !names.insert(name) {
                return None;
            }
            if name == selected {
                let mut cursor = property.walk();
                if property.children(&mut cursor).any(|n| n.kind() == "?") {
                    return None;
                }
                result = self.js_ts_simple_type_annotation(property.child_by_field_name("type")?);
            }
        }
        result
    }

    fn js_ts_simple_type_annotation(&self, annotation: Node<'_>) -> Option<String> {
        let ty = annotation.named_child(0)?;
        if ty.kind() != "type_identifier" {
            return None;
        }
        let text = self.node_text(&ty);
        (is_plain_ident(text) && !self.js_ts_type_name_shadowed(self.tree.root_node(), &ty, text))
            .then(|| text.to_string())
    }

    /// Value-binding absence does not prove a TypeScript type name. Only the
    /// module class is supported; nearer type declarations and generics fence it.
    fn js_ts_type_name_shadowed(&self, node: Node<'_>, at: &Node<'_>, name: &str) -> bool {
        if node.kind() == "class"
            && node
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name)
            && node.start_byte() <= at.start_byte()
            && at.end_byte() <= node.end_byte()
        {
            return true;
        }
        if matches!(node.kind(), "type_parameter")
            && node
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name)
            && node.parent().and_then(|n| n.parent()).is_some_and(|scope| {
                scope.start_byte() <= at.start_byte() && at.end_byte() <= scope.end_byte()
            })
        {
            return true;
        }
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
        ) && node
            .child_by_field_name("name")
            .is_some_and(|n| self.node_text(&n) == name)
        {
            if let Some(scope) =
                self.js_ts_nearest_lexical_scope_id(&node, self.tree.root_node().id())
            {
                // Module declarations are checked by clean_class_spans at
                // resolution; retain their existing unsupported-type metadata.
                if scope != self.tree.root_node().id()
                    && self.js_ts_scope_distance(at, scope).is_some()
                {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_type_name_shadowed(child, at, name));
        found
    }

    /// A declaration must finish in a statement list that reaches the call.
    /// Crossing a conditional, loop, exception handler, or callable is not proof
    /// of execution. Ordinary nested blocks are transparent.
    fn receiver_declaration_reaches_call(&self, declaration: Node<'_>, call_byte: usize) -> bool {
        if declaration.end_byte() > call_byte {
            return false;
        }
        let mut current = declaration.parent();
        while let Some(node) = current {
            let statement_list = matches!(
                node.kind(),
                "block" | "statement_block" | "module" | "program"
            );
            if node.start_byte() <= call_byte && call_byte < node.end_byte() {
                return statement_list;
            }
            if !statement_list
                && !matches!(
                    node.kind(),
                    "expression_statement" | "lexical_declaration" | "variable_declaration"
                )
            {
                return false;
            }
            current = node.parent();
        }
        false
    }

    /// Python constructor/local-annotation lookup uses enclosing function scopes.
    /// A signature annotation is evaluated outside its own function body.
    pub(crate) fn python_receiver_owner_shadowed(
        &self,
        function: Node<'_>,
        name: &str,
        include_current: bool,
    ) -> bool {
        let mut current = if include_current {
            Some(function)
        } else {
            function.parent()
        };
        while let Some(node) = current {
            if matches!(node.kind(), "function_definition" | "lambda")
                && self
                    .function_local_value_bindings(&node)
                    .is_none_or(|names| names.contains(name))
            {
                return true;
            }
            current = node.parent();
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_js_ts_receiver_binding_candidates(
        &self,
        node: Node<'_>,
        is_root: bool,
        root_scope: Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
        allow_constructor: bool,
        out: &mut Vec<(usize, usize, JsTsReceiverBindingEvidence)>,
        parse_recovery: &mut bool,
    ) {
        if node.is_error() || node.is_missing() {
            *parse_recovery = true;
            return;
        }

        let mut push = |scope_id: usize, evidence: JsTsReceiverBindingEvidence| {
            if let Some(distance) = self.js_ts_scope_distance(receiver, scope_id) {
                out.push((distance, scope_id, evidence));
            }
        };

        if !is_root && is_js_ts_function_like(node.kind()) {
            if matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && self
                .language
                .function_name(&node)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "interface_declaration" | "type_alias_declaration"
            )
        {
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "class_declaration" | "abstract_class_declaration"
            )
        {
            if node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
            {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::ClassOwner);
                }
            }
            return;
        }

        if !is_root
            && matches!(
                node.kind(),
                "enum_declaration" | "function_signature" | "import_alias"
            )
        {
            let name = node.child_by_field_name("name").or_else(|| {
                (node.kind() == "import_alias")
                    .then(|| node.named_child(0))
                    .flatten()
            });
            if name.is_some_and(|name| self.node_text(&name) == receiver_name) {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        if !is_root && matches!(node.kind(), "internal_module" | "module") {
            if node.child_by_field_name("name").is_some_and(|name| {
                name.kind() == "identifier" && self.node_text(&name) == receiver_name
            }) {
                if let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                {
                    push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                }
            }
            return;
        }

        match node.kind() {
            "variable_declarator" => {
                let mut names = BTreeSet::new();
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_js_ts_binding_pattern_names(name, &mut names);
                }
                if names.contains(receiver_name) {
                    let parent_is_var = node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "variable_declaration");
                    let scope_id = if parent_is_var {
                        Some(root_scope.id())
                    } else {
                        self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                    };
                    if let Some(scope_id) = scope_id {
                        let simple_name = node.child_by_field_name("name").is_some_and(|name| {
                            name.kind() == "identifier" && self.node_text(&name) == receiver_name
                        });
                        let evidence = if allow_constructor
                            && simple_name
                            && self.receiver_declaration_reaches_call(node, receiver.start_byte())
                        {
                            node.child_by_field_name("value")
                                .and_then(|value| {
                                    self.js_ts_direct_new_constructor(value, root_scope)
                                })
                                .map_or(JsTsReceiverBindingEvidence::Materialized, |static_type| {
                                    JsTsReceiverBindingEvidence::Recovered {
                                        static_type,
                                        recovery:
                                            crate::resolution::ReceiverRecovery::ConstructorLocal,
                                        declaration_end_byte: Some(node.end_byte()),
                                    }
                                })
                        } else {
                            JsTsReceiverBindingEvidence::Materialized
                        };
                        push(scope_id, evidence);
                    }
                }
            }
            "for_in_statement" | "for_of_statement" | "for_await_statement" => {
                let mut declared = false;
                let mut is_var = false;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "var" => {
                            declared = true;
                            is_var = true;
                        }
                        "let" | "const" => declared = true,
                        _ => {}
                    }
                }
                if declared {
                    let mut names = BTreeSet::new();
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_js_ts_binding_pattern_names(left, &mut names);
                    }
                    if names.contains(receiver_name) {
                        let scope_id = if is_var {
                            Some(root_scope.id())
                        } else {
                            self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                        };
                        if let Some(scope_id) = scope_id {
                            push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                        }
                    }
                }
            }
            "catch_clause" => {
                let mut names = BTreeSet::new();
                self.collect_js_ts_catch_binding_names(node, &mut names);
                if names.contains(receiver_name) {
                    if let Some(scope_id) =
                        self.js_ts_nearest_lexical_scope_id(&node, root_scope.id())
                    {
                        push(scope_id, JsTsReceiverBindingEvidence::Materialized);
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_js_ts_receiver_binding_candidates(
                child,
                false,
                root_scope,
                receiver,
                receiver_name,
                allow_constructor,
                out,
                parse_recovery,
            );
        }
    }

    fn js_ts_scope_distance(&self, receiver: &Node<'_>, scope_id: usize) -> Option<usize> {
        let mut current = Some(*receiver);
        let mut distance = 0usize;
        while let Some(node) = current {
            if node.id() == scope_id {
                return Some(distance);
            }
            current = node.parent();
            distance += 1;
        }
        None
    }

    /// Normalize a field callable to the member carrying its name and modifiers.
    fn js_ts_callable_member<'a>(&self, method: &Node<'a>) -> Node<'a> {
        method
            .parent()
            .filter(|n| matches!(n.kind(), "field_definition" | "public_field_definition"))
            .unwrap_or(*method)
    }

    /// Direct methods and bounded arrow fields share instance-slot authority.
    pub(crate) fn js_ts_method_slot_unproven(&self, method: &Node<'_>) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let slot = self.js_ts_callable_member(method);
        let Some(body) = slot.parent().filter(|n| n.kind() == "class_body") else {
            return false;
        };
        let Some(name) = slot
            .child_by_field_name("name")
            .or_else(|| slot.child_by_field_name("property"))
        else {
            return true;
        };
        let field = slot.id() != method.id();
        if field {
            let mut cursor = slot.walk();
            let modifiers = slot.children(&mut cursor).any(|n| {
                n.kind() == "decorator"
                    || matches!(
                        self.node_text(&n),
                        "private" | "protected" | "declare" | "abstract"
                    )
            });
            let class = body.parent().unwrap();
            let mut cursor = class.walk();
            let decorated_class = class.children(&mut cursor).any(|n| n.kind() == "decorator");
            if method.kind() != "arrow_function"
                || name.kind() != "property_identifier"
                || slot.child_by_field_name("value") != Some(*method)
                || body.has_error()
                || modifiers
                || decorated_class
            {
                return true;
            }
        }
        let name = self.node_text(&name);
        let mut matches = 0;
        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            if self.js_ts_method_is_static(&member) {
                continue;
            }
            let key = member
                .child_by_field_name("name")
                .or_else(|| member.child_by_field_name("property"));
            if let Some(key) = key {
                if key.kind() == "computed_property_name" {
                    return true;
                }
                if self.node_text(&key).trim_matches(['\'', '"']) == name {
                    matches += 1;
                    let mut cursor = member.walk();
                    let accessor = member
                        .children(&mut cursor)
                        .any(|n| matches!(n.kind(), "get" | "set"));
                    if accessor
                        || (member.kind() != "method_definition"
                            && !(field && member.id() == slot.id()))
                    {
                        return true;
                    }
                }
            }
        }
        if matches != 1 {
            return true;
        }
        fn writes_this(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
            if parsed
                .js_ts_write_target(node)
                .is_some_and(|target| parsed.js_ts_target_writes_member(target, "this", Some(name)))
            {
                return true;
            }
            let mut cursor = node.walk();
            let written = node
                .named_children(&mut cursor)
                .any(|n| writes_this(parsed, n, name));
            written
        }
        writes_this(self, body, name)
    }

    pub(crate) fn js_ts_method_is_static(&self, method: &Node<'_>) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        let slot = self.js_ts_callable_member(method);
        let mut cursor = slot.walk();
        let found = slot
            .children(&mut cursor)
            .any(|child| child.kind() == "static");
        found
    }

    fn js_ts_write_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let writes = matches!(
            node.kind(),
            "assignment_expression"
                | "augmented_assignment_expression"
                | "update_expression"
                | "for_in_statement"
                | "for_of_statement"
                | "for_await_statement"
        ) || (node.kind() == "unary_expression"
            && node
                .child_by_field_name("operator")
                .is_some_and(|n| self.node_text(&n) == "delete"));
        writes
            .then(|| {
                node.child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("argument"))
                    .or_else(|| node.named_child(0))
            })
            .flatten()
    }

    /// Inspect assignment targets, not RHS reads/default initializers.
    fn js_ts_target_writes_member(
        &self,
        target: Node<'_>,
        receiver: &str,
        member: Option<&str>,
    ) -> bool {
        if matches!(target.kind(), "member_expression" | "subscript_expression") {
            return target.child_by_field_name("object").is_some_and(|mut n| {
                while n.kind() == "parenthesized_expression" && n.named_child_count() == 1 {
                    n = n.named_child(0).unwrap();
                }
                self.node_text(&n) == receiver
            }) && (target.kind() == "subscript_expression"
                || member.is_none()
                || target
                    .child_by_field_name("property")
                    .is_some_and(|n| Some(self.node_text(&n)) == member));
        }
        if matches!(
            target.kind(),
            "pair_pattern" | "assignment_pattern" | "object_assignment_pattern"
        ) {
            return target
                .child_by_field_name(if target.kind() == "pair_pattern" {
                    "value"
                } else {
                    "left"
                })
                .is_some_and(|n| self.js_ts_target_writes_member(n, receiver, member));
        }
        if matches!(
            target.kind(),
            "object_pattern" | "array_pattern" | "rest_pattern" | "parenthesized_expression"
        ) {
            let mut cursor = target.walk();
            return target
                .named_children(&mut cursor)
                .any(|n| self.js_ts_target_writes_member(n, receiver, member));
        }
        false
    }

    fn js_ts_direct_new_constructor(
        &self,
        value: Node<'_>,
        function_scope: Node<'_>,
    ) -> Option<String> {
        if value.kind() != "new_expression" {
            return None;
        }
        let constructor = value.child_by_field_name("constructor")?;
        if constructor.kind() != "identifier" {
            return None;
        }
        let text = self.node_text(&constructor);
        if !is_plain_ident(text) {
            return None;
        }
        let mut scope = Some(function_scope);
        while let Some(node) = scope {
            if node.kind() == "class"
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| self.node_text(&name) == text)
            {
                return None;
            }
            if is_js_ts_function_like(node.kind())
                && self
                    .js_ts_scope_receiver_binding_evidence(node, &constructor, text, false, true)
                    .is_some()
            {
                return None;
            }
            scope = node.parent();
        }
        let module_evidence = self.js_ts_scope_receiver_binding_evidence(
            self.tree.root_node(),
            &constructor,
            text,
            false,
            false,
        );
        (matches!(
            module_evidence,
            Some(JsTsReceiverBindingEvidence::ClassOwner)
        ) || (module_evidence.is_none()
            && self.js_ts_esm_named_imports(Some(text)).contains(text)))
        .then(|| text.to_string())
    }

    fn js_ts_receiver_written_between(
        &self,
        node: Node<'_>,
        binding_scope_id: usize,
        receiver: &Node<'_>,
        receiver_name: &str,
        after_byte: usize,
    ) -> bool {
        let mut before_byte = receiver.start_byte();
        let mut ancestor = receiver.parent();
        while let Some(scope) = ancestor {
            if is_js_ts_function_like(scope.kind()) {
                break;
            }
            if matches!(
                scope.kind(),
                "for_statement"
                    | "for_in_statement"
                    | "for_of_statement"
                    | "for_await_statement"
                    | "while_statement"
                    | "do_statement"
            ) && after_byte <= scope.start_byte()
            {
                before_byte = before_byte.max(scope.end_byte());
            }
            ancestor = scope.parent();
        }
        if node.start_byte() >= before_byte || node.end_byte() <= after_byte {
            return false;
        }
        if let Some(target) = self.js_ts_write_target(node) {
            let mut written_names = BTreeSet::new();
            self.collect_js_ts_binding_pattern_names(target, &mut written_names);
            if (written_names.contains(receiver_name)
                || self.js_ts_target_writes_member(target, receiver_name, None))
                && !self.js_ts_receiver_has_closer_binding(&target, receiver_name, binding_scope_id)
            {
                return true;
            }
        }
        let mut cursor = node.walk();
        let found = node.named_children(&mut cursor).any(|child| {
            self.js_ts_receiver_written_between(
                child,
                binding_scope_id,
                receiver,
                receiver_name,
                after_byte,
            )
        });
        found
    }

    /// A module class/import is a live binding. Any visible write, including in
    /// an escaping callable, revokes the whole-file owner proof. Shadow writes
    /// are separated by the same lexical predicate used for receiver mutations.
    pub(crate) fn js_ts_module_value_written(&self, name: &str) -> bool {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }
        fn walk(parsed: &ParsedFile, node: Node<'_>, root_id: usize, name: &str) -> bool {
            if matches!(
                node.kind(),
                "assignment_expression"
                    | "augmented_assignment_expression"
                    | "update_expression"
                    | "for_in_statement"
                    | "for_of_statement"
                    | "for_await_statement"
            ) {
                if let Some(target) = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("argument"))
                    .or_else(|| node.named_child(0))
                {
                    let mut names = BTreeSet::new();
                    parsed.collect_js_ts_binding_pattern_names(target, &mut names);
                    if names.contains(name)
                        && !parsed.js_ts_receiver_has_closer_binding(&target, name, root_id)
                    {
                        return true;
                    }
                }
            }
            let mut cursor = node.walk();
            let written = node
                .named_children(&mut cursor)
                .any(|child| walk(parsed, child, root_id, name));
            written
        }
        let root = self.tree.root_node();
        walk(self, root, root.id(), name)
    }

    fn js_ts_receiver_has_closer_binding(
        &self,
        target: &Node<'_>,
        receiver_name: &str,
        binding_scope_id: usize,
    ) -> bool {
        let mut current = target.parent();
        while let Some(scope) = current {
            if scope.id() == binding_scope_id {
                return false;
            }
            if is_js_ts_function_like(scope.kind()) {
                if self.js_ts_function_scope_binds_receiver(&scope, target, receiver_name) {
                    return true;
                }
            } else if self.language.is_scope_block(scope.kind())
                || matches!(
                    scope.kind(),
                    "for_statement"
                        | "for_in_statement"
                        | "for_of_statement"
                        | "for_await_statement"
                        | "switch_statement"
                        | "switch_body"
                        | "catch_clause"
                )
            {
                if self.js_ts_receiver_binding_reaches_call(
                    scope,
                    scope.id(),
                    target,
                    receiver_name,
                ) {
                    return true;
                }
            }
            current = scope.parent();
        }
        true
    }

    /// Conservative function-local value bindings for callback value-reference
    /// resolution. `None` means this language or syntax is not modeled strongly
    /// enough to prove that a plain identifier is free.
    pub fn function_local_value_bindings(&self, func_node: &Node<'_>) -> Option<BTreeSet<String>> {
        fn contains_recovery(node: Node<'_>) -> bool {
            if node.is_error() || node.is_missing() {
                return true;
            }
            let mut cursor = node.walk();
            let found = node.children(&mut cursor).any(contains_recovery);
            found
        }

        if contains_recovery(*func_node) {
            return None;
        }
        if matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return Some(self.js_ts_function_local_bindings(func_node));
        }
        if !matches!(self.language, Language::Python | Language::Go) {
            return None;
        }

        let mut out: BTreeSet<String> = self
            .function_parameter_occurrences(func_node)
            .into_iter()
            .map(|(name, _, _)| name)
            .collect();
        let root_key = (
            func_node.start_byte(),
            func_node.end_byte(),
            func_node.kind_id(),
        );
        self.collect_python_go_local_value_bindings(*func_node, root_key, &mut out);
        Some(out)
    }

    fn collect_python_go_local_value_bindings(
        &self,
        node: Node<'_>,
        root_key: (usize, usize, u16),
        out: &mut BTreeSet<String>,
    ) {
        let is_root = (node.start_byte(), node.end_byte(), node.kind_id()) == root_key;
        if !is_root
            && matches!(
                node.kind(),
                "function_definition"
                    | "lambda"
                    | "function_declaration"
                    | "method_declaration"
                    | "func_literal"
            )
        {
            if node.kind() == "function_definition" {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            return;
        }

        match (self.language, node.kind()) {
            (Language::Python, "assignment" | "augmented_assignment") => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Python, "named_expression") => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_local_binding_pattern(name, out);
                }
            }
            (Language::Python, "for_statement" | "for_in_clause") => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Python, "as_pattern") => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    self.collect_local_binding_pattern(alias, out);
                }
            }
            (Language::Python, "except_clause" | "except_group_clause") => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    self.collect_local_binding_pattern(alias, out);
                } else {
                    // tree-sitter-python exposes the regular `except` alias as
                    // a field, but the `except*` shape has no named fields.
                    let mut cursor = node.walk();
                    for child in node.named_children(&mut cursor) {
                        if child.prev_sibling().is_some_and(|prev| prev.kind() == "as") {
                            self.collect_local_binding_pattern(child, out);
                        }
                    }
                }
            }
            (Language::Python, "delete_statement") => {
                let mut cursor = node.walk();
                for target in node.named_children(&mut cursor) {
                    self.collect_local_binding_pattern(target, out);
                }
            }
            (Language::Python, "type_alias_statement") => {
                if let Some(left) = node.child_by_field_name("left") {
                    // A generic alias may wrap the bound name with its type
                    // parameters. Over-collecting those identifiers is the
                    // fail-closed choice for imported-root authority.
                    self.collect_identifier_names(left, out);
                }
            }
            (Language::Python, "import_statement" | "import_from_statement") => {
                // Python imports inside a function bind names in that function's
                // local namespace. Collecting every identifier is deliberately
                // conservative: an extra local is safer than an omitted binding.
                self.collect_identifier_names(node, out);
            }
            (Language::Python, "case_pattern") => {
                // Capture patterns bind locals, while value/class patterns can
                // also contain identifiers. Over-collecting the latter is the
                // fail-closed choice for callback identity.
                self.collect_identifier_names(node, out);
            }
            (Language::Python, "class_definition") if !is_root => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
                return;
            }
            (Language::Go, "short_var_declaration") => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_local_binding_pattern(left, out);
                }
            }
            (Language::Go, "parameter_declaration" | "variadic_parameter_declaration") => {
                let type_start = node
                    .child_by_field_name("type")
                    .map_or(node.end_byte(), |ty| ty.start_byte());
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.start_byte() >= type_start {
                        break;
                    }
                    if child.kind() == "identifier" {
                        self.collect_local_binding_pattern(child, out);
                    }
                }
            }
            (Language::Go, "var_spec") => {
                let value_start = node
                    .child_by_field_name("value")
                    .map_or(node.end_byte(), |value| value.start_byte());
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.start_byte() >= value_start {
                        break;
                    }
                    if child.kind() == "identifier" {
                        self.collect_local_binding_pattern(child, out);
                    }
                }
            }
            (Language::Go, "range_clause") => {
                let text = self.node_text(&node);
                if text.contains(":=") {
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_local_binding_pattern(left, out);
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_go_local_value_bindings(child, root_key, out);
        }
    }

    fn collect_local_binding_pattern(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        match node.kind() {
            "identifier" => {
                let name = self.node_text(&node);
                if name != "_" {
                    out.insert(name.to_string());
                }
            }
            "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "list_splat_pattern"
            | "as_pattern_target"
            | "expression_list"
            | "parenthesized_expression" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.collect_local_binding_pattern(child, out);
                }
            }
            _ => {}
        }
    }

    fn collect_js_ts_parameter_bindings(&self, func_node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(params) = self.find_parameters_node(&func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                self.collect_js_ts_parameter_binding_names(child, out);
            }
        }
    }

    fn collect_js_ts_parameter_binding_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(pattern) = node.child_by_field_name("pattern") {
            self.collect_js_ts_binding_pattern_names(pattern, out);
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            self.collect_js_ts_binding_pattern_names(name, out);
            return;
        }
        if let Some(left) = node.child_by_field_name("left") {
            self.collect_js_ts_binding_pattern_names(left, out);
            return;
        }

        match node.kind() {
            "object_pattern" | "array_pattern" | "identifier" | "rest_pattern" => {
                self.collect_js_ts_binding_pattern_names(node, out);
            }
            kind if kind.contains("parameter") => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if matches!(child.kind(), "type_annotation" | "return_type") {
                        continue;
                    }
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            _ => {}
        }
    }

    fn collect_js_ts_binding_pattern_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" => {
                let name = self.node_text(&node);
                if is_plain_ident(name) {
                    out.insert(name.to_string());
                }
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.collect_js_ts_binding_pattern_names(value, out);
                }
            }
            "rest_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            "assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.collect_js_ts_binding_pattern_names(left, out);
                }
            }
            "object_pattern"
            | "array_pattern"
            | "parenthesized_expression"
            | "parenthesized_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_js_ts_binding_pattern_names(child, out);
                }
            }
            _ => {}
        }
    }

    fn collect_js_ts_local_bindings(
        &self,
        node: Node<'_>,
        root_key: (usize, usize, u16),
        out: &mut BTreeSet<String>,
    ) {
        let is_root = (node.start_byte(), node.end_byte(), node.kind_id()) == root_key;

        if !is_root && is_js_ts_function_like(node.kind()) {
            if node.kind() == "function_declaration" {
                if let Some(name) = self.language.function_name(&node) {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            return;
        }

        match node.kind() {
            "variable_declarator" => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_identifier_names(name, out);
                }
            }
            "catch_clause" => {
                self.collect_js_ts_catch_binding_names(node, out);
            }
            "class_declaration" if !is_root => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(self.node_text(&name).to_string());
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_js_ts_local_bindings(child, root_key, out);
        }
    }

    fn js_ts_receiver_binding_reaches_call(
        &self,
        node: Node<'_>,
        root_scope_id: usize,
        receiver: &Node<'_>,
        receiver_name: &str,
    ) -> bool {
        let is_root = node.id() == root_scope_id;

        if !is_root && is_js_ts_function_like(node.kind()) {
            return matches!(
                node.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && self
                .language
                .function_name(&node)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "class_declaration" {
            return node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "class" {
            let self_name_matches = node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name);
            let mut current = Some(*receiver);
            let mut receiver_is_inside = false;
            while let Some(ancestor) = current {
                if ancestor.id() == node.id() {
                    receiver_is_inside = true;
                    break;
                }
                current = ancestor.parent();
            }
            return self_name_matches && receiver_is_inside;
        }

        if !is_root
            && matches!(
                node.kind(),
                "interface_declaration" | "type_alias_declaration"
            )
        {
            return false;
        }

        if !is_root
            && matches!(
                node.kind(),
                "abstract_class_declaration" | "enum_declaration" | "function_signature"
            )
        {
            return node
                .child_by_field_name("name")
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && node.kind() == "import_alias" {
            return node
                .named_child(0)
                .is_some_and(|name| self.node_text(&name) == receiver_name)
                && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
        }

        if !is_root && matches!(node.kind(), "internal_module" | "module") {
            let name_binds =
                node.child_by_field_name("name").is_some_and(|name| {
                    name.kind() == "identifier" && self.node_text(&name) == receiver_name
                }) && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id);
            if name_binds {
                return true;
            }

            let mut current = Some(*receiver);
            let mut receiver_is_inside = false;
            while let Some(ancestor) = current {
                if ancestor.id() == node.id() {
                    receiver_is_inside = true;
                    break;
                }
                current = ancestor.parent();
            }
            if !receiver_is_inside {
                return false;
            }
        }

        if node.is_error() || node.is_missing() {
            return true;
        }

        match node.kind() {
            "for_in_statement" | "for_of_statement" | "for_await_statement" => {
                let mut is_var = false;
                let mut is_lexical = false;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "var" => is_var = true,
                        "let" | "const" => is_lexical = true,
                        _ => {}
                    }
                }
                if is_var || is_lexical {
                    let mut names = BTreeSet::new();
                    if let Some(left) = node.child_by_field_name("left") {
                        self.collect_js_ts_binding_pattern_names(left, &mut names);
                    }
                    if names.contains(receiver_name)
                        && (is_var
                            || self.js_ts_lexical_scope_reaches_receiver(
                                &node,
                                receiver,
                                root_scope_id,
                            ))
                    {
                        return true;
                    }
                }
            }
            "variable_declarator" => {
                let mut names = BTreeSet::new();
                if let Some(name) = node.child_by_field_name("name") {
                    self.collect_js_ts_binding_pattern_names(name, &mut names);
                }
                if names.contains(receiver_name) {
                    let is_function_scoped_var = node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "variable_declaration");
                    if is_function_scoped_var
                        || self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id)
                    {
                        return true;
                    }
                }
            }
            "catch_clause" => {
                let mut names = BTreeSet::new();
                self.collect_js_ts_catch_binding_names(node, &mut names);
                if names.contains(receiver_name)
                    && self.js_ts_lexical_scope_reaches_receiver(&node, receiver, root_scope_id)
                {
                    return true;
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        let found = node.named_children(&mut cursor).any(|child| {
            self.js_ts_receiver_binding_reaches_call(child, root_scope_id, receiver, receiver_name)
        });
        found
    }

    fn js_ts_function_scope_binds_receiver(
        &self,
        func_node: &Node<'_>,
        receiver: &Node<'_>,
        receiver_name: &str,
    ) -> bool {
        let mut parameters = BTreeSet::new();
        self.collect_js_ts_parameter_bindings(*func_node, &mut parameters);
        if parameters.contains(receiver_name) {
            return true;
        }

        if matches!(
            func_node.kind(),
            "function_declaration"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
        ) && self
            .language
            .function_name(func_node)
            .is_some_and(|name| self.node_text(&name) == receiver_name)
        {
            return true;
        }

        self.js_ts_receiver_binding_reaches_call(
            *func_node,
            func_node.id(),
            receiver,
            receiver_name,
        )
    }

    fn js_ts_lexical_scope_reaches_receiver(
        &self,
        binding: &Node<'_>,
        receiver: &Node<'_>,
        root_scope_id: usize,
    ) -> bool {
        let Some(scope_id) = self.js_ts_nearest_lexical_scope_id(binding, root_scope_id) else {
            return false;
        };
        let mut current = Some(*receiver);
        while let Some(node) = current {
            if node.id() == scope_id {
                return true;
            }
            if node.id() == root_scope_id {
                return false;
            }
            current = node.parent();
        }
        false
    }

    fn js_ts_nearest_lexical_scope_id(
        &self,
        binding: &Node<'_>,
        root_scope_id: usize,
    ) -> Option<usize> {
        let mut current = Some(*binding);
        while let Some(node) = current {
            if node.id() == root_scope_id {
                return Some(node.id());
            }
            if self.language.is_scope_block(node.kind())
                || matches!(
                    node.kind(),
                    "for_statement"
                        | "for_in_statement"
                        | "for_of_statement"
                        | "for_await_statement"
                        | "switch_statement"
                        | "switch_body"
                        | "catch_clause"
                )
            {
                return Some(node.id());
            }
            current = node.parent();
        }
        None
    }

    fn collect_js_ts_catch_binding_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if let Some(param) = node.child_by_field_name("parameter") {
            self.collect_js_ts_binding_pattern_names(param, out);
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "statement_block" {
                continue;
            }
            self.collect_js_ts_binding_pattern_names(child, out);
        }
    }

    fn collect_identifier_names(&self, node: Node<'_>, out: &mut BTreeSet<String>) {
        if node.kind() == "identifier" {
            out.insert(self.node_text(&node).to_string());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_names(child, out);
        }
    }

    /// Process a single statement node for module-scope bindings.
    fn extract_module_bindings_from_stmt(
        node: tree_sitter::Node<'_>,
        language: &crate::languages::Language,
        source: &str,
        out: &mut BTreeMap<String, crate::call_graph::ModuleBindingKind>,
    ) {
        use crate::call_graph::ModuleBindingKind;
        let text = |n: &tree_sitter::Node<'_>| -> String {
            n.utf8_text(source.as_bytes()).unwrap_or("").to_string()
        };
        match node.kind() {
            // Python
            "import_statement" | "import_from_statement" => {
                let mut ic = node.walk();
                for c in node.children(&mut ic) {
                    match c.kind() {
                        "dotted_name" => {
                            let name = text(&c);
                            // Keep occurrence-clean eligibility keyed by the name
                            // Python actually binds for `import pkg.models`.
                            let alias = name.split('.').next().unwrap_or(&name).to_string();
                            out.entry(alias).or_insert(ModuleBindingKind::Import);
                        }
                        "aliased_import" => {
                            if let Some(a) = c.child_by_field_name("alias") {
                                out.entry(text(&a)).or_insert(ModuleBindingKind::Import);
                            }
                        }
                        "identifier" => {
                            let name = text(&c);
                            if node.kind() == "import_from_statement" {
                                if let Some(mod_node) = node.child_by_field_name("module_name") {
                                    if name == text(&mod_node) {
                                        return;
                                    }
                                }
                            }
                            out.entry(name).or_insert(ModuleBindingKind::Import);
                        }
                        _ => {}
                    }
                }
            }
            "class_definition" | "class_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(text(&name), ModuleBindingKind::ClassDef);
                }
            }
            "function_definition" | "function_declaration" => {
                if let Some(name) = language.function_name(&node) {
                    out.insert(text(&name), ModuleBindingKind::FunctionDef);
                }
            }
            "decorated_definition" => {
                let mut dc = node.walk();
                for inner in node.children(&mut dc) {
                    match inner.kind() {
                        "class_definition" | "class_declaration" => {
                            if let Some(name) = inner.child_by_field_name("name") {
                                out.insert(text(&name), ModuleBindingKind::ClassDef);
                            }
                        }
                        "function_definition" | "function_declaration" => {
                            if let Some(name) = language.function_name(&inner) {
                                out.insert(text(&name), ModuleBindingKind::FunctionDef);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "expression_statement" => {
                let mut ec = node.walk();
                for inner in node.children(&mut ec) {
                    if inner.kind() == "assignment" {
                        if let Some(left) = inner.child_by_field_name("left") {
                            if left.kind() == "identifier" {
                                out.insert(text(&left), ModuleBindingKind::Assignment);
                            }
                        }
                    }
                }
            }
            // JS/TS: variable declarations, exported functions/classes
            "lexical_declaration" | "variable_declaration" => {
                let mut vc = node.walk();
                for decl in node.children(&mut vc) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(name) = decl.child_by_field_name("name") {
                            if name.kind() == "identifier" {
                                out.insert(text(&name), ModuleBindingKind::Assignment);
                            }
                        }
                    }
                }
            }
            "export_statement" => {
                let mut ec = node.walk();
                for inner in node.children(&mut ec) {
                    match inner.kind() {
                        "class_declaration" => {
                            if let Some(name) = inner.child_by_field_name("name") {
                                out.insert(text(&name), ModuleBindingKind::ClassDef);
                            }
                        }
                        "function_declaration" => {
                            if let Some(name) = language.function_name(&inner) {
                                out.insert(text(&name), ModuleBindingKind::FunctionDef);
                            }
                        }
                        "lexical_declaration" | "variable_declaration" => {
                            let mut vc = inner.walk();
                            for decl in inner.children(&mut vc) {
                                if decl.kind() == "variable_declarator" {
                                    if let Some(name) = decl.child_by_field_name("name") {
                                        if name.kind() == "identifier" {
                                            out.insert(text(&name), ModuleBindingKind::Assignment);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// Descend into Python compound statement bodies (`if`/`try`/`for`/`while`/`with`)
    /// at module scope to find bindings that are still effectively module-scope.
    fn descend_compound_for_bindings(
        node: tree_sitter::Node<'_>,
        language: &crate::languages::Language,
        source: &str,
        out: &mut BTreeMap<String, crate::call_graph::ModuleBindingKind>,
    ) {
        match node.kind() {
            "if_statement" | "try_statement" | "for_statement" | "while_statement"
            | "with_statement" => {
                let mut bcursor = node.walk();
                for block_child in node.children(&mut bcursor) {
                    match block_child.kind() {
                        "block" => {
                            let mut ic = block_child.walk();
                            for stmt in block_child.children(&mut ic) {
                                Self::extract_module_bindings_from_stmt(
                                    stmt, language, source, out,
                                );
                                // Recurse into nested compound statements.
                                Self::descend_compound_for_bindings(stmt, language, source, out);
                            }
                        }
                        "else_clause" | "elif_clause" | "except_clause" | "finally_clause" => {
                            let mut cc = block_child.walk();
                            for clause_child in block_child.children(&mut cc) {
                                if clause_child.kind() == "block" {
                                    let mut ic = clause_child.walk();
                                    for stmt in clause_child.children(&mut ic) {
                                        Self::extract_module_bindings_from_stmt(
                                            stmt, language, source, out,
                                        );
                                        Self::descend_compound_for_bindings(
                                            stmt, language, source, out,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_go_imports(&self, node: Node<'_>, out: &mut BTreeMap<String, String>) {
        match node.kind() {
            "import_declaration" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "import_spec_list" {
                        let mut inner = child.walk();
                        for spec in child.children(&mut inner) {
                            if spec.kind() == "import_spec" {
                                self.extract_go_import_spec(&spec, out);
                            }
                        }
                    } else if child.kind() == "import_spec" {
                        self.extract_go_import_spec(&child, out);
                    } else if child.kind() == "interpreted_string_literal" {
                        // `import "pkg"` — single import without parens
                        let text = self.node_text(&child);
                        let path = text.trim_matches('"').to_string();
                        let alias = path.rsplit('/').next().unwrap_or(&path).to_string();
                        out.insert(alias, path);
                    }
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_go_imports(child, out);
                }
            }
        }
    }

    fn extract_go_import_spec(&self, node: &Node<'_>, out: &mut BTreeMap<String, String>) {
        let mut path_str = None;
        let mut alias = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "interpreted_string_literal" => {
                    let text = self.node_text(&child);
                    path_str = Some(text.trim_matches('"').to_string());
                }
                "package_identifier" | "blank_identifier" | "dot" => {
                    alias = Some(self.node_text(&child).to_string());
                }
                _ => {}
            }
        }
        if let Some(path) = path_str {
            let local =
                alias.unwrap_or_else(|| path.rsplit('/').next().unwrap_or(&path).to_string());
            if local != "_" && local != "." {
                out.insert(local, path);
            }
        }
    }

    /// P5 S2 (Go func-value callbacks): find registration candidates within
    /// `func_node`, restricted to `lines` (the same `_on_lines` gating idiom
    /// `function_calls_with_qualifier_and_spans_on_lines` uses). Raw
    /// extraction only — target resolution, the shadow check, and per-form
    /// owner recovery/field-typing gates all live in `call_graph.rs`
    /// (`CallGraph::apply_go_registration_candidate`), since they need
    /// whole-program CallGraph state (`functions`, `imports`,
    /// `go_func_typed_fields`) this AST layer doesn't have.
    ///
    /// Go has no anonymous-function node in `function_node_types()`
    /// (`languages/mod.rs`: only `function_declaration`/`method_declaration`),
    /// so `all_functions()` never visits a nested closure separately — a
    /// registration inside one is naturally attributed to the enclosing named
    /// function by this recursive walk, with no double-counting to guard
    /// against.
    pub(crate) fn go_registration_candidates(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<GoRegistrationCandidate> {
        let mut out = Vec::new();
        self.collect_go_registration_candidates(*func_node, lines, &mut out);
        out
    }

    fn collect_go_registration_candidates(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<GoRegistrationCandidate>,
    ) {
        match node.kind() {
            // Form (a): `Command{Run: helper}` — a keyed field in a composite
            // literal whose value is a bare identifier.
            "composite_literal" => {
                if let (Some(type_node), Some(body)) = (
                    node.child_by_field_name("type"),
                    node.child_by_field_name("body"),
                ) {
                    let struct_type_text = self.node_text(&type_node).trim().to_string();
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() != "keyed_element" {
                            continue;
                        }
                        let (Some(key), Some(value)) = (
                            child.child_by_field_name("key"),
                            child.child_by_field_name("value"),
                        ) else {
                            continue;
                        };
                        let (Some(key_ident), Some(value_ident)) = (
                            literal_element_identifier(&key),
                            literal_element_identifier(&value),
                        ) else {
                            continue;
                        };
                        let line = value_ident.start_position().row + 1;
                        if !lines.contains(&line) {
                            continue;
                        }
                        out.push(GoRegistrationCandidate {
                            form: GoRegistrationForm::CompositeLiteralField {
                                struct_type_text: struct_type_text.clone(),
                                field_name: self.node_text(&key_ident).trim().to_string(),
                            },
                            value_name: self.node_text(&value_ident).trim().to_string(),
                            line,
                            start_byte: value_ident.start_byte(),
                            end_byte: value_ident.end_byte(),
                        });
                    }
                }
            }
            // Form (b): `x.Run = helper` — assignment to a selector whose
            // value is a bare identifier. Only the single-target,
            // single-value shape (`expression_list` of length 1 on both
            // sides) is recognized; multi-assignment (`a, b = x, y`) is out
            // of scope.
            "assignment_statement" => {
                if let (Some(left), Some(right)) = (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ) {
                    let mut lcursor = left.walk();
                    let left_items: Vec<_> = left.named_children(&mut lcursor).collect();
                    let mut rcursor = right.walk();
                    let right_items: Vec<_> = right.named_children(&mut rcursor).collect();
                    if left_items.len() == 1 && right_items.len() == 1 {
                        let sel = left_items[0];
                        let value_ident = right_items[0];
                        if sel.kind() == "selector_expression" && value_ident.kind() == "identifier"
                        {
                            if let (Some(operand), Some(field)) = (
                                sel.child_by_field_name("operand"),
                                sel.child_by_field_name("field"),
                            ) {
                                if operand.kind() == "identifier" {
                                    let line = value_ident.start_position().row + 1;
                                    if lines.contains(&line) {
                                        out.push(GoRegistrationCandidate {
                                            form: GoRegistrationForm::FieldAssignment {
                                                operand_name: self
                                                    .node_text(&operand)
                                                    .trim()
                                                    .to_string(),
                                                field_name: self
                                                    .node_text(&field)
                                                    .trim()
                                                    .to_string(),
                                                assign_line: node.start_position().row + 1,
                                                assign_start_byte: node.start_byte(),
                                            },
                                            value_name: self
                                                .node_text(&value_ident)
                                                .trim()
                                                .to_string(),
                                            line,
                                            start_byte: value_ident.start_byte(),
                                            end_byte: value_ident.end_byte(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Form (c): `Register(helper)` — a bare identifier passed
            // directly as a call argument (nested expressions, e.g.
            // `Register(wrap(helper))`'s outer argument, are excluded here;
            // the recursive walk below still visits `wrap(helper)` as its own
            // call_expression, so `helper` is still found as an argument of
            // THAT call).
            "call_expression" => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    let mut cursor = args.walk();
                    for arg in args.named_children(&mut cursor) {
                        if arg.kind() != "identifier" {
                            continue;
                        }
                        let line = arg.start_position().row + 1;
                        if !lines.contains(&line) {
                            continue;
                        }
                        out.push(GoRegistrationCandidate {
                            form: GoRegistrationForm::CallArgument,
                            value_name: self.node_text(&arg).trim().to_string(),
                            line,
                            start_byte: arg.start_byte(),
                            end_byte: arg.end_byte(),
                        });
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_go_registration_candidates(child, lines, out);
        }
    }

    /// P7 S1: which `@property`-family decorator (if any, exact-match only)
    /// decorates a Python function node from `all_functions()`. Reads
    /// decorator expressions on the raw `decorated_definition` wrapper BEFORE
    /// `unwrap_decorated` strips it (mirrors the existing
    /// `type_providers::python::has_decorator` / flask decorator-reading
    /// convention: iterate `decorated_definition`'s `decorator` children and
    /// read each one's source text directly — no compiled query exists for
    /// `decorator` nodes).
    ///
    /// Exact full-expression match only: `property`, `cached_property`, or
    /// `functools.cached_property`. This is deliberately not a last-segment
    /// match (unlike `has_decorator`'s dataclass check) — `@x.setter` /
    /// `@x.deleter` (whatever `x` is) and any other decorator never match,
    /// so the setter/deleter exclusion falls out of the exact-match rule
    /// itself rather than needing a separate reject list.
    pub(crate) fn python_property_kind(&self, func_node: &Node<'_>) -> Option<PythonPropertyKind> {
        if self.language != Language::Python || func_node.kind() != "decorated_definition" {
            return None;
        }
        let mut cursor = func_node.walk();
        for child in func_node.children(&mut cursor) {
            if child.kind() != "decorator" {
                continue;
            }
            let text = self.node_text(&child).trim();
            let Some(expr) = text.strip_prefix('@') else {
                continue;
            };
            match expr.trim() {
                "property" => return Some(PythonPropertyKind::Property),
                "cached_property" | "functools.cached_property" => {
                    return Some(PythonPropertyKind::CachedProperty)
                }
                _ => {}
            }
        }
        None
    }

    /// P7 (F2, codex MAJOR 2; re-fixed per codex re-review): whether
    /// `func_node` (as returned by `all_functions()`) is a genuine Python
    /// instance method whose first POSITIONAL parameter is literally named
    /// `self` — gates tier-1 same-class property narrowing for `self.attr`
    /// in `call_graph.rs::self_property_owner_getters`. (Tier-1 eligibility
    /// ALSO requires `method_owners.contains_key(&caller_id)`, checked in
    /// `call_graph.rs` before this gate is even consulted — a `self`
    /// receiver in a non-method has no owner and must route straight to
    /// tier-3, never tier-1-then-drop.)
    ///
    /// `method_owner` (`languages/mod.rs`) marks ANY class-contained
    /// function as owned by its class, including `@staticmethod`/
    /// `@classmethod`-decorated ones — a `@staticmethod def f(self)` has
    /// `self` as an ordinary parameter of unknown type, not a receiver, so
    /// it must not get same-class narrowing. This check is scoped to the P7
    /// property-access path only; it does NOT change `method_owner`'s
    /// semantics or method-CALL resolution's existing behavior.
    ///
    /// The enclosing function MAY legitimately carry other decorators,
    /// including `@property` itself: a getter reading `self.other_prop` is
    /// still a genuine instance method.
    pub(crate) fn python_is_self_instance_method(&self, func_node: &Node<'_>) -> bool {
        if self.language != Language::Python {
            return false;
        }
        if func_node.kind() == "decorated_definition" {
            let mut cursor = func_node.walk();
            for child in func_node.children(&mut cursor) {
                if child.kind() != "decorator" {
                    continue;
                }
                let text = self.node_text(&child).trim();
                let Some(expr) = text.strip_prefix('@') else {
                    continue;
                };
                if matches!(expr.trim(), "staticmethod" | "classmethod") {
                    return false;
                }
            }
        }
        self.python_first_positional_param_is_self(func_node)
    }

    /// P7 (F2 re-fix, codex MAJOR re-review): whether the FIRST POSITIONAL
    /// parameter of a Python function node is literally named `self`.
    ///
    /// Unlike the generic `function_parameter_names` extractor (which
    /// flattens every parameter into a bare name list with no
    /// separator/splat context — the exact gap the re-review flagged),
    /// this walks the `parameters` node's named children directly, in
    /// declaration order:
    /// - A bare `*` (`keyword_separator`), `*args` (`list_splat_pattern`),
    ///   or `**kwargs` (`dictionary_splat_pattern`) encountered before any
    ///   plain parameter FAILS the gate outright — there is no positional
    ///   receiver at all (a keyword-only or splat-only signature).
    /// - A bare `/` (`positional_separator`) is transparent: a `self`
    ///   preceding it is still the first positional parameter
    ///   (`def m(self, /)`).
    /// - The first plain parameter encountered — `identifier`,
    ///   `typed_parameter` (`self: T`), `default_parameter`/
    ///   `typed_default_parameter` (`self=...`) — must be named `self` to
    ///   pass; anything else (including a `tuple_pattern`, which can never
    ///   resolve to a plain name) fails.
    ///
    /// Scoped to the P7 property-access path only via
    /// `python_is_self_instance_method` — never consulted by method-CALL
    /// resolution or `method_owner`.
    fn python_first_positional_param_is_self(&self, func_node: &Node<'_>) -> bool {
        let Some(params) = self.find_parameters_node(func_node) else {
            return false;
        };
        let mut cursor = params.walk();
        for child in params.named_children(&mut cursor) {
            match child.kind() {
                "keyword_separator" | "list_splat_pattern" | "dictionary_splat_pattern" => {
                    return false;
                }
                "positional_separator" => continue,
                _ => {
                    return self.extract_param_name(&child).as_deref() == Some("self");
                }
            }
        }
        false
    }

    /// P7 S2: candidate `@property`/`@cached_property` LOAD access sites
    /// (`recv.attr`) within a function body, restricted to `attr_names`
    /// (S1's indexed property names — the "zero cost for normal attribute
    /// traffic" gate: an empty/non-matching `attr_names` short-circuits
    /// before any receiver/store analysis). Excludes:
    /// - a call's function child (`x.attr(...)` — `x.attr` is being CALLED,
    ///   not read; call_graph.rs documents why this must not double-record
    ///   against the ordinary call-resolution path for the same site).
    /// - any attribute node that is (a descendant of) the `left` field of an
    ///   `assignment` or `augmented_assignment` (`x.attr = v` / `x.attr += v`
    ///   — a STORE, never a load; both grammars put the target under `left`).
    /// - (F4) `del`-statement targets, `for`/comprehension targets, and
    ///   `with ... as` alias targets — see `collect_python_attribute_loads`.
    /// - (F3, codex MAJOR 3) anything inside a nested `function_definition`/
    ///   `decorated_definition`/`class_definition`: the walk fences at those
    ///   boundaries so a nested def's body is scanned exactly once, by its
    ///   OWN entry in `all_functions()` — never misattributed to the
    ///   enclosing function. Nested `lambda` bodies are fenced too, but
    ///   (unlike nested defs) nothing else ever scans them: lambdas never
    ///   get their own `FunctionId` from `all_functions()`, so any
    ///   `self.attr`/property access inside a lambda body is an accepted,
    ///   deliberate recall gap (never recorded against anything).
    ///
    /// (F5) Also reports a `store_skips` count: every store/delete-context
    /// attribute access above whose name is S1-indexed, even though it
    /// never becomes a candidate — telemetry for
    /// `CallGraph::property_access_store_skips`.
    pub(crate) fn python_attribute_load_candidates(
        &self,
        func_node: &Node<'_>,
        attr_names: &BTreeSet<String>,
    ) -> PythonAttributeLoadScan {
        if self.language != Language::Python || attr_names.is_empty() {
            return PythonAttributeLoadScan::default();
        }
        let def_node = if func_node.kind() == "decorated_definition" {
            func_node
                .child_by_field_name("definition")
                .unwrap_or(*func_node)
        } else {
            *func_node
        };
        let Some(body) = def_node.child_by_field_name("body") else {
            return PythonAttributeLoadScan::default();
        };
        let mut candidates = Vec::new();
        let mut store_skips = 0usize;
        self.collect_python_attribute_loads(
            body,
            attr_names,
            false,
            &mut candidates,
            &mut store_skips,
        );
        PythonAttributeLoadScan {
            candidates,
            store_skips,
        }
    }

    fn collect_python_attribute_loads(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        in_store_target: bool,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        // F3 (codex MAJOR 3): fence nested callable/class scopes. The walk
        // starts at a function's OWN body (never at the function node
        // itself — see `python_attribute_load_candidates`), so any of these
        // kinds encountered here is necessarily a NESTED one. The nested
        // def/class gets its own separate scan via its own `all_functions()`
        // entry; recursing into it here would misattribute its accesses to
        // the outer function AND double-scan it. Lambda bodies are fenced
        // too but are never scanned by anything else (lambdas have no
        // `FunctionId`) — an accepted, deliberate recall gap.
        if matches!(
            node.kind(),
            "function_definition" | "decorated_definition" | "lambda" | "class_definition"
        ) {
            return;
        }

        // (F4) `assignment`/`augmented_assignment` LHS and `for`/
        // `for_in_clause` (incl. comprehensions) loop targets share the same
        // shape: the `left` field is a store, every other child keeps the
        // ambient load/store context (the right-hand/iterable side is a
        // LOAD — e.g. `for x in r.text:` — `r.text` IS a load).
        if matches!(
            node.kind(),
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause"
        ) {
            self.collect_left_field_as_store(node, attr_names, in_store_target, out, store_skips);
            return;
        }

        // (F4) `del` targets and `with ... as TARGET:` alias targets are
        // always store/delete context, never a load, regardless of the
        // ambient context or internal shape (single attribute, tuple,
        // expression_list, ...). `as_pattern_target` also covers
        // except/match `as` bindings, which is harmless here (same rule).
        if matches!(node.kind(), "delete_statement" | "as_pattern_target") {
            self.collect_all_children_as_store(node, attr_names, out, store_skips);
            return;
        }

        if node.kind() == "attribute" {
            if let Some(attr_field) = node.child_by_field_name("attribute") {
                let name = self.node_text(&attr_field);
                if attr_names.contains(name) {
                    if in_store_target {
                        // F5: a store/delete-context access of an indexed
                        // property name is never a candidate, but IS counted.
                        *store_skips += 1;
                    } else {
                        let is_call_function = node
                            .parent()
                            .filter(|p| p.kind() == "call")
                            .and_then(|p| p.child_by_field_name("function"))
                            .is_some_and(|f| byte_range_eq(&f, &node));
                        if !is_call_function {
                            let receiver_identifier = node
                                .child_by_field_name("object")
                                .filter(|o| o.kind() == "identifier")
                                .map(|o| self.node_text(&o).to_string());
                            out.push(PythonAttributeLoadCandidate {
                                attr_name: name.to_string(),
                                receiver_identifier,
                                line: node.start_position().row + 1,
                                start_byte: node.start_byte(),
                                end_byte: node.end_byte(),
                            });
                        }
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_attribute_loads(
                child,
                attr_names,
                in_store_target,
                out,
                store_skips,
            );
        }
    }

    /// (F4) Shared shape for `assignment`/`augmented_assignment` and
    /// `for`/`for_in_clause`: recurse into `node`'s `left` field as a store
    /// target, and every other child with the ambient `in_store_target`
    /// flag unchanged (so the right-hand/iterable side stays LOAD context
    /// unless it was already nested inside an outer store target).
    fn collect_left_field_as_store(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        in_store_target: bool,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        if let Some(left) = node.child_by_field_name("left") {
            self.collect_python_attribute_loads(left, attr_names, true, out, store_skips);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let is_left = node
                .child_by_field_name("left")
                .is_some_and(|l| byte_range_eq(&l, &child));
            if is_left {
                continue; // already walked above with in_store_target = true.
            }
            self.collect_python_attribute_loads(
                child,
                attr_names,
                in_store_target,
                out,
                store_skips,
            );
        }
    }

    /// (F4) Shared shape for `delete_statement`/`as_pattern_target`: every
    /// child is unconditionally a store/delete target, regardless of the
    /// ambient context.
    fn collect_all_children_as_store(
        &self,
        node: Node<'_>,
        attr_names: &BTreeSet<String>,
        out: &mut Vec<PythonAttributeLoadCandidate>,
        store_skips: &mut usize,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_python_attribute_loads(child, attr_names, true, out, store_skips);
        }
    }

    /// Find all assignment targets (L-values) on diff lines within a function scope.
    pub fn assignment_lvalues_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lvalues = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalues(&capture.node, line, &mut lvalues);
                        }
                    }
                }
            }
            return lvalues;
        }

        let mut lvalues = Vec::new();
        self.collect_assignments_manual(*func_node, lines, &mut lvalues);
        lvalues
    }

    /// Extract L-value names from a matched assignment/declaration node.
    fn extract_assignment_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }
    }

    /// Manual recursive assignment collection (pre-query fallback).
    pub(crate) fn collect_assignments_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignments_manual(child, lines, out);
        }
    }

    /// Like `assignment_lvalues_on_lines`, but returns structured `AccessPath`s
    /// instead of plain variable name strings. Used by the DFG for field-sensitive tracking.
    pub fn assignment_lvalue_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut paths = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalue_paths(&capture.node, line, &mut paths);
                        }
                    }
                }
            }
            // The query may miss for_in_statement and as_pattern since they aren't
            // assignment/declaration nodes. Run the gap handlers on the full tree.
            self.collect_assignment_paths_gaps(*func_node, lines, &mut paths);
            return paths;
        }

        let mut paths = Vec::new();
        self.collect_assignment_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Byte-bearing sibling of `assignment_lvalue_paths_on_lines`.
    pub fn assignment_lvalue_spans_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<PathSpan> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut spans = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalue_spans(&capture.node, &mut spans);
                        }
                    }
                }
            }
            self.collect_assignment_span_gaps(*func_node, lines, &mut spans);
            spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
            return spans;
        }

        let mut spans = Vec::new();
        self.collect_assignment_spans_manual(*func_node, lines, &mut spans);
        spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
        spans
    }

    /// Recover the exact AST parent for a call that is itself an assignment or
    /// declaration RHS. Nested calls are rejected: only the RHS slot whose
    /// byte range exactly equals the supplied call range is admitted.
    pub fn assignment_for_exact_call(
        &self,
        call_start_byte: usize,
        call_end_byte: usize,
    ) -> Option<AssignmentCallInfo> {
        let leaf = self
            .tree
            .root_node()
            .descendant_for_byte_range(call_start_byte, call_end_byte)?;
        let mut current = Some(leaf);
        while let Some(node) = current {
            if self.language.is_assignment_node(node.kind())
                || self.language.is_declaration_node(node.kind())
            {
                let rhs = self
                    .language
                    .assignment_value(&node)
                    .or_else(|| self.language.declaration_value(&node))?;
                let rhs_slots: Vec<Node<'_>> =
                    if matches!(rhs.kind(), "expression_list" | "variable_list") {
                        let mut cursor = rhs.walk();
                        rhs.named_children(&mut cursor).collect()
                    } else {
                        vec![rhs]
                    };
                let rhs_slot = rhs_slots.iter().position(|slot| {
                    slot.start_byte() == call_start_byte && slot.end_byte() == call_end_byte
                })?;

                let lhs = self
                    .language
                    .assignment_target(&node)
                    .or_else(|| self.language.declaration_name(&node))?;
                let mut lvalues = Vec::new();
                self.extract_lvalue_spans_from_node(lhs, &mut lvalues);
                lvalues.sort_by_key(|span| (span.start_byte, span.end_byte));
                if rhs_slots.len() > 1 {
                    if rhs_slots.len() != lvalues.len() {
                        return None;
                    }
                    lvalues = vec![lvalues[rhs_slot].clone()];
                }
                return Some(AssignmentCallInfo {
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    rhs_start_byte: rhs.start_byte(),
                    rhs_end_byte: rhs.end_byte(),
                    lvalues,
                });
            }
            current = node.parent();
        }
        None
    }

    /// True when a byte-bearing DFG Use is a semantic value/receiver use, not
    /// a call's callee label or a Python keyword-argument label.
    pub fn is_semantic_value_use(&self, start_byte: usize, end_byte: usize) -> bool {
        let Some(leaf) = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
        else {
            return false;
        };
        let mut current = Some(leaf);
        while let Some(node) = current {
            if node.kind() == "keyword_argument" {
                if node.child_by_field_name("name").is_some_and(|name| {
                    name.start_byte() <= start_byte && end_byte <= name.end_byte()
                }) {
                    return false;
                }
            }
            if self.language.is_call_node(node.kind()) {
                if self.language.call_function_name(&node).is_some_and(|name| {
                    name.start_byte() <= start_byte && end_byte <= name.end_byte()
                }) {
                    return false;
                }
            }
            current = node.parent();
        }
        true
    }

    /// Extract L-value AccessPaths from a matched assignment/declaration node.
    fn extract_assignment_lvalue_paths(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&lhs, line, out);
                } else {
                    let lhs_text = self.node_text(&lhs).to_string();
                    for path in extract_lvalue_paths(&lhs_text) {
                        out.push((path, line));
                    }
                }
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }
    }

    fn extract_assignment_lvalue_spans(&self, node: &Node<'_>, out: &mut Vec<PathSpan>) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                self.extract_lvalue_spans_from_node(lhs, out);
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = node
                .child_by_field_name("pattern")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("declarator"))
                .or_else(|| self.language.declaration_name(node))
            {
                self.extract_lvalue_spans_from_node(name_node, out);
            }
        }
    }

    /// Handle gap patterns not covered by the Assignments query (for_in, as_pattern).
    fn collect_assignment_paths_gaps(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_paths_gaps(child, lines, out);
        }
    }

    /// Byte-bearing gap patterns not covered by the Assignments query.
    fn collect_assignment_span_gaps(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalue_spans(&node, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    self.push_simple_lvalue_span(child, out);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_span_gaps(child, lines, out);
        }
    }

    /// Manual recursive assignment path collection (pre-query fallback).
    fn collect_assignment_paths_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&lhs, line, out);
                } else {
                    let lhs_text = self.node_text(&lhs).to_string();
                    for path in extract_lvalue_paths(&lhs_text) {
                        out.push((path, line));
                    }
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_paths_manual(child, lines, out);
        }
    }

    /// Manual recursive assignment span collection (pre-query fallback).
    fn collect_assignment_spans_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                self.extract_lvalue_spans_from_node(lhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = node
                .child_by_field_name("pattern")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("declarator"))
                .or_else(|| self.language.declaration_name(&node))
            {
                self.extract_lvalue_spans_from_node(name_node, out);
            }
        }

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalue_spans(&node, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    self.push_simple_lvalue_span(child, out);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_spans_manual(child, lines, out);
        }
    }

    /// Extract individual L-value paths from a multi-target node (pattern_list, expression_list).
    ///
    /// Handles:
    /// - Go: `val, err := getData()` — expression_list with identifier children
    /// - Python: `name, age = get_user()` — pattern_list with identifier children
    /// - Python: `first, *rest = items` — pattern_list with identifier + list_splat_pattern
    /// - Go: `for key, value := range m` — expression_list in range clause
    fn extract_multi_target_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = self.node_text(&child).to_string();
                    out.push((AccessPath::simple(name), line));
                }
                // Python star unpack: *rest
                "list_splat_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            let name = self.node_text(&inner).to_string();
                            out.push((AccessPath::simple(name), line));
                        }
                    }
                }
                // Field access as L-value: obj.field, dev->field in multi-target
                "field_expression"
                | "member_expression"
                | "selector_expression"
                | "attribute"
                | "field_access"
                | "dot_index_expression"
                | "method_index_expression" => {
                    let text = self.node_text(&child).to_string();
                    for path in extract_lvalue_paths(&text) {
                        out.push((path, line));
                    }
                }
                // Nested tuple: (a, b), c = func()
                "pattern_list" | "tuple_pattern" | "parenthesized_expression" => {
                    self.extract_multi_target_lvalues(&child, line, out);
                }
                // Skip punctuation (commas, etc.)
                _ => {}
            }
        }
    }

    fn extract_lvalue_spans_from_node(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let kind = node.kind();

        if Self::is_field_access_node(kind) || Self::is_index_access_node(kind) {
            let text = self.node_text(&node).to_string();
            for path in extract_lvalue_paths(&text) {
                out.push(PathSpan {
                    path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
            }
            return;
        }

        if self.is_binding_identifier_node(kind) {
            self.push_simple_lvalue_span(node, out);
            return;
        }

        match kind {
            "object_pattern" | "array_pattern" => {
                self.extract_destructuring_def_spans(&node, out);
                return;
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.extract_lvalue_spans_from_node(value, out);
                    return;
                }
            }
            "rest_pattern"
            | "list_splat_pattern"
            | "dictionary_splat_pattern"
            | "spread_element"
            | "mutable_pattern"
            | "mut_pattern"
            | "reference_pattern"
            | "ref_pattern"
            | "tuple_pattern"
            | "pattern_list"
            | "expression_list"
            | "parenthesized_expression"
            | "parenthesized_pattern" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_lvalue_spans_from_node(child, out);
                }
                return;
            }
            _ => {}
        }

        let text = self.node_text(&node).to_string();
        let paths = extract_lvalue_paths(&text);
        if !paths.is_empty() {
            for path in paths {
                out.push(PathSpan {
                    path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_lvalue_spans_from_node(child, out);
        }
    }

    fn is_binding_identifier_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "identifier"
                | "shorthand_property_identifier_pattern"
                | "property_identifier"
                | "field_identifier"
        )
    }

    fn is_index_access_node(kind: &str) -> bool {
        matches!(
            kind,
            "subscript_expression" | "index_expression" | "slice_expression"
        )
    }

    fn push_simple_lvalue_span(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let name = self.node_text(&node).to_string();
        if is_plain_ident(&name) {
            out.push(PathSpan {
                path: AccessPath::simple(name),
                line: node.start_position().row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
        }
    }

    /// Extract L-value defs from a JS/TS `for_in_statement` loop header.
    ///
    /// Handles:
    /// - `for (const key in obj)` → def for "key"
    /// - `for (const { name, id } of items)` → defs for "name", "id"
    /// - `for (const [a, b] of pairs)` → defs for "a", "b"
    fn extract_for_in_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) && name != "const" && name != "let" && name != "var" {
                        out.push((AccessPath::simple(name), line));
                    }
                }
                // Destructuring: { name, id } or [a, b]
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_defs(&child, line, out);
                }
                _ => {}
            }
        }
    }

    fn extract_for_in_lvalue_spans(&self, node: &Node<'_>, out: &mut Vec<PathSpan>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) && name != "const" && name != "let" && name != "var" {
                        self.push_simple_lvalue_span(child, out);
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_def_spans(&child, out);
                }
                _ => {}
            }
        }
    }

    /// Extract defs from a destructuring pattern (object_pattern or array_pattern).
    fn extract_destructuring_defs(
        &self,
        pattern: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" | "identifier" => {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
                "pair_pattern" => {
                    // { name: alias } — the value side is the bound variable
                    if let Some(val) = child.child_by_field_name("value") {
                        if val.kind() == "identifier" {
                            let name = self.node_text(&val).to_string();
                            if is_plain_ident(&name) {
                                out.push((AccessPath::simple(name), line));
                            }
                        } else if val.kind() == "object_pattern" || val.kind() == "array_pattern" {
                            self.extract_destructuring_defs(&val, line, out);
                        }
                    }
                }
                "rest_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            let name = self.node_text(&inner).to_string();
                            if is_plain_ident(&name) {
                                out.push((AccessPath::simple(name), line));
                            }
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_defs(&child, line, out);
                }
                _ => {}
            }
        }
    }

    fn extract_destructuring_def_spans(&self, pattern: &Node<'_>, out: &mut Vec<PathSpan>) {
        let mut cursor = pattern.walk();
        for child in pattern.children(&mut cursor) {
            match child.kind() {
                "shorthand_property_identifier_pattern" | "identifier" => {
                    self.push_simple_lvalue_span(child, out);
                }
                "pair_pattern" => {
                    if let Some(val) = child.child_by_field_name("value") {
                        self.extract_lvalue_spans_from_node(val, out);
                    }
                }
                "rest_pattern" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            self.push_simple_lvalue_span(inner, out);
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    self.extract_destructuring_def_spans(&child, out);
                }
                _ => {}
            }
        }
    }

    /// Collect simple alias assignments within a function: `ptr = dev` where both
    /// sides are plain identifiers. Returns (alias, target, line) triples sorted by line.
    ///
    /// Used by Phase 3 must-alias tracking to resolve `ptr->field` to `dev->field`.
    pub fn collect_alias_assignments(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, String, usize)> {
        let mut aliases = Vec::new();
        self.collect_aliases_inner(*func_node, lines, &mut aliases);
        aliases.sort_by_key(|(_a, _t, line)| *line);
        aliases
    }

    fn collect_aliases_inner(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) {
            // Check assignments: ptr = dev
            if self.language.is_assignment_node(node.kind()) {
                if let Some(lhs) = self.language.assignment_target(&node) {
                    if let Some(rhs) = self.language.assignment_value(&node) {
                        let lhs_text = self.node_text(&lhs).to_string();
                        let rhs_text = self.node_text(&rhs).to_string().trim().to_string();
                        if is_plain_ident(&lhs_text) && is_plain_ident(&rhs_text) {
                            out.push((lhs_text, rhs_text, line));
                        }
                    }
                }
            }

            // Check declarations with initializers: type *ptr = dev, let ptr = dev
            if self.language.is_declaration_node(node.kind()) {
                // JS/TS destructuring: const { name, id } = obj → name aliases obj.name
                if self.extract_destructuring_aliases(&node, line, out) {
                    // Destructuring was handled, skip normal declaration path
                } else if let Some(name_node) = self.language.declaration_name(&node) {
                    if let Some(val_node) = self.language.declaration_value(&node) {
                        let name = self.node_text(&name_node).to_string();
                        let val = self.node_text(&val_node).to_string().trim().to_string();
                        if is_plain_ident(&name) && is_plain_ident(&val) {
                            out.push((name, val, line));
                        }
                    }
                }
            }

            // Gap 3: JS/TS for-of/for-in with destructuring patterns
            // `for (const { name, id } of items)` → name aliases items.name
            if node.kind() == "for_in_statement"
                && matches!(
                    self.language,
                    Language::JavaScript | Language::TypeScript | Language::Tsx
                )
            {
                self.extract_for_in_aliases(&node, line, out);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_aliases_inner(child, lines, out);
        }
    }

    /// Extract aliases from JS/TS destructuring declarations.
    ///
    /// Handles:
    /// - `const { name, id } = obj` → name aliases obj.name, id aliases obj.id
    /// - `const { name: userName } = obj` → userName aliases obj.name
    /// - `const [first, second] = arr` → first aliases arr, second aliases arr
    /// - Nested: `const { config: { host } } = obj` → host aliases obj.config.host
    ///
    /// Returns true if a destructuring pattern was found (even if no aliases emitted).
    fn extract_destructuring_aliases(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) -> bool {
        // Only JS/TS have destructuring patterns
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return false;
        }

        // Find variable_declarator children with object_pattern or array_pattern
        let declarator = if node.kind() == "variable_declarator" {
            *node
        } else {
            // lexical_declaration/variable_declaration → variable_declarator
            let mut found = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    found = Some(child);
                    break;
                }
            }
            match found {
                Some(d) => d,
                None => return false,
            }
        };

        let name_node = match declarator.child_by_field_name("name") {
            Some(n) => n,
            None => return false,
        };

        if name_node.kind() != "object_pattern" && name_node.kind() != "array_pattern" {
            return false;
        }

        let val_node = match declarator.child_by_field_name("value") {
            Some(v) => v,
            None => return false,
        };

        let rhs = self.node_text(&val_node).to_string().trim().to_string();
        if !is_plain_ident(&rhs) {
            return true; // It's destructuring, but RHS is complex — skip
        }

        self.extract_pattern_aliases(&name_node, &rhs, line, out);
        true
    }

    /// Extract aliases from a JS/TS for-in/for-of statement with destructuring.
    ///
    /// `for (const { name, id } of items)` → name aliases items.name, id aliases items.id
    /// `for (const key in obj)` → key aliases obj (no destructuring, but simple binding)
    fn extract_for_in_aliases(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) {
        // Find the iterable (right side): the "right" field of for_in_statement
        let rhs = match node.child_by_field_name("right") {
            Some(r) => r,
            None => return,
        };
        let rhs_text = self.node_text(&rhs).to_string().trim().to_string();
        if !is_plain_ident(&rhs_text) {
            return;
        }

        // Find the pattern or identifier (left side)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "object_pattern" => {
                    self.extract_pattern_aliases(&child, &rhs_text, line, out);
                }
                "array_pattern" => {
                    self.extract_pattern_aliases(&child, &rhs_text, line, out);
                }
                _ => {}
            }
        }
    }

    /// Recursively extract aliases from a destructuring pattern node.
    fn extract_pattern_aliases(
        &self,
        pattern: &Node<'_>,
        rhs_base: &str,
        line: usize,
        out: &mut Vec<(String, String, usize)>,
    ) {
        if pattern.kind() == "object_pattern" {
            let mut cursor = pattern.walk();
            for child in pattern.children(&mut cursor) {
                match child.kind() {
                    // { name } — shorthand, variable name matches property name
                    "shorthand_property_identifier_pattern" => {
                        let field = self.node_text(&child).to_string();
                        if is_plain_ident(&field) {
                            out.push((field.clone(), format!("{}.{}", rhs_base, field), line));
                        }
                    }
                    // { name: userName } — renamed, or { config: { host } } — nested
                    "pair_pattern" => {
                        if let Some(key_node) = child.child_by_field_name("key") {
                            let key = self.node_text(&key_node).to_string();
                            if let Some(val_node) = child.child_by_field_name("value") {
                                if val_node.kind() == "object_pattern"
                                    || val_node.kind() == "array_pattern"
                                {
                                    // Nested destructuring: { config: { host } }
                                    let nested_base = format!("{}.{}", rhs_base, key);
                                    self.extract_pattern_aliases(
                                        &val_node,
                                        &nested_base,
                                        line,
                                        out,
                                    );
                                } else {
                                    // Renamed: { name: userName }
                                    let alias = self.node_text(&val_node).to_string();
                                    if is_plain_ident(&alias) && is_plain_ident(&key) {
                                        out.push((alias, format!("{}.{}", rhs_base, key), line));
                                    }
                                }
                            }
                        }
                    }
                    // { ...rest } — rest element, aliases the whole object
                    "rest_pattern" => {
                        let mut inner_cursor = child.walk();
                        for inner in child.children(&mut inner_cursor) {
                            if inner.kind() == "identifier" {
                                let name = self.node_text(&inner).to_string();
                                if is_plain_ident(&name) {
                                    out.push((name, rhs_base.to_string(), line));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if pattern.kind() == "array_pattern" {
            // Array destructuring: [first, second] = arr
            // We can't track indices, so alias each to the base array
            let mut cursor = pattern.walk();
            for child in pattern.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        let name = self.node_text(&child).to_string();
                        if is_plain_ident(&name) {
                            out.push((name, rhs_base.to_string(), line));
                        }
                    }
                    "object_pattern" | "array_pattern" => {
                        // Nested pattern inside array — alias to base
                        self.extract_pattern_aliases(&child, rhs_base, line, out);
                    }
                    // Rest element: [...rest] = arr → rest aliases arr
                    "rest_pattern" => {
                        let mut inner_cursor = child.walk();
                        for inner in child.children(&mut inner_cursor) {
                            if inner.kind() == "identifier" {
                                let name = self.node_text(&inner).to_string();
                                if is_plain_ident(&name) {
                                    out.push((name, rhs_base.to_string(), line));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Like `rvalue_identifiers_on_lines`, but returns structured `AccessPath`s.
    /// Used by the DFG for field-sensitive tracking.
    pub fn rvalue_identifier_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        // Use the Assignments query to find assignment nodes, then extract RHS.
        // Also use the Calls query for call arguments on diff lines.
        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut paths = Vec::new();

            // Collect R-values from assignments
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                        {
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_identifier_paths(rhs, &mut paths);
                            }
                        }
                    }
                }
            }

            // Collect R-values from call arguments using Calls query
            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line) {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_identifier_paths(args, &mut paths);
                                }
                                if let Some(func_name_node) =
                                    self.language.call_function_name(&capture.node)
                                {
                                    let name = self.node_text(&func_name_node).to_string();
                                    paths.push((AccessPath::simple(name), line));
                                }
                            }
                        }
                    }
                }
            }

            return paths;
        }

        let mut paths = Vec::new();
        self.collect_rvalue_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Byte-bearing sibling of `rvalue_identifier_paths_on_lines`.
    pub fn rvalue_identifier_spans_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<PathSpan> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut spans = Vec::new();

            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                        {
                            if self.is_augmented_assignment(&capture.node) {
                                if let Some(lhs) = self.language.assignment_target(&capture.node) {
                                    self.collect_identifier_path_spans(lhs, &mut spans);
                                }
                            }
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_identifier_path_spans(rhs, &mut spans);
                            }
                        }
                    }
                }
            }

            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line) {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_identifier_path_spans(args, &mut spans);
                                }
                                if let Some(func_name_node) =
                                    self.language.call_function_name(&capture.node)
                                {
                                    self.push_identifier_path_span(func_name_node, &mut spans);
                                }
                            }
                        }
                    }
                }
            }

            self.collect_return_value_identifier_spans(func_node, lines, &mut spans);
            spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
            return spans;
        }

        let mut spans = Vec::new();
        self.collect_rvalue_spans_manual(*func_node, lines, &mut spans);
        spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
        spans
    }

    /// Manual recursive R-value path collection (pre-query fallback).
    fn collect_rvalue_paths_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_identifier_paths(rhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_identifier_paths(args, out);
            }
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((AccessPath::simple(name), line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalue_paths_manual(child, lines, out);
        }
    }

    /// Manual recursive R-value span collection (pre-query fallback).
    fn collect_rvalue_spans_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if self.is_augmented_assignment(&node) {
                if let Some(lhs) = self.language.assignment_target(&node) {
                    self.collect_identifier_path_spans(lhs, out);
                }
            }
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_identifier_path_spans(rhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_identifier_path_spans(args, out);
            }
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                self.push_identifier_path_span(func_name_node, out);
            }
        }

        if lines.contains(&line) && self.language.is_return_node(node.kind()) {
            self.collect_return_node_value_spans(node, out);
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalue_spans_manual(child, lines, out);
        }
    }

    /// Check if a node kind represents a field/member access expression.
    /// Each language has a different tree-sitter node kind for this:
    /// - C/C++, Rust: field_expression
    /// - JS/TS: member_expression
    /// - Go: selector_expression
    /// - Python: attribute
    /// - Java: field_access
    /// - Lua: dot_index_expression, method_index_expression (colon syntax)
    fn is_field_access_node(kind: &str) -> bool {
        matches!(
            kind,
            "field_expression"
                | "member_expression"
                | "selector_expression"
                | "attribute"
                | "field_access"
                | "dot_index_expression"
                | "method_index_expression"
        )
    }

    fn is_augmented_assignment(&self, node: &Node<'_>) -> bool {
        if matches!(
            node.kind(),
            "augmented_assignment"
                | "augmented_assignment_expression"
                | "compound_assignment_expr"
                | "update_expression"
        ) {
            return true;
        }

        const AUGMENTED_ASSIGN_OPS: &[&str] = &[
            "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "**=", "//=",
        ];
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).any(|child| {
            if child.is_named() {
                return false;
            }
            let kind = child.kind();
            let text = self.node_text(&child);
            AUGMENTED_ASSIGN_OPS.contains(&kind) || AUGMENTED_ASSIGN_OPS.contains(&text.trim())
        });
        found
    }

    fn collect_identifier_paths<'a>(&self, node: Node<'a>, out: &mut Vec<(AccessPath, usize)>) {
        // Check for field/member access expressions — emit the full qualified
        // path instead of individual identifiers.
        if Self::is_field_access_node(node.kind()) {
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::from_expr(&text), line));
            // Also emit the base identifier for field-insensitive fallback.
            // Different tree-sitter grammars use different field names:
            //   C/C++/Rust: "argument", Go: "operand", JS/TS/Python/Java: "object"
            //   Lua: first named child (no standard field name)
            let base_node = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0));
            if let Some(base) = base_node {
                if self.language.is_identifier_node(base.kind()) {
                    let base_name = self.node_text(&base).to_string();
                    out.push((AccessPath::simple(base_name), line));
                }
            }
            return; // Don't recurse into children — we've handled them
        }

        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::simple(name), line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_paths(child, out);
        }
    }

    fn collect_identifier_path_spans(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        if Self::is_field_access_node(node.kind()) {
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push(PathSpan {
                path: AccessPath::from_expr(&text),
                line,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });

            if let Some(base) = self.leftmost_receiver_identifier(node) {
                self.push_identifier_path_span(base, out);
            }
            return;
        }

        if self.language.is_identifier_node(node.kind()) {
            self.push_identifier_path_span(node, out);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_path_spans(child, out);
        }
    }

    fn leftmost_receiver_identifier<'a>(&self, mut node: Node<'a>) -> Option<Node<'a>> {
        loop {
            let receiver = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0))?;
            if Self::is_field_access_node(receiver.kind()) {
                node = receiver;
                continue;
            }
            return self
                .language
                .is_identifier_node(receiver.kind())
                .then_some(receiver);
        }
    }

    fn push_identifier_path_span(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let name = self.node_text(&node).to_string();
        if is_plain_ident(&name) {
            out.push(PathSpan {
                path: AccessPath::simple(name),
                line: node.start_position().row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
        }
    }

    fn collect_return_value_identifier_spans(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Returns) {
            let ret_idx = query
                .capture_index_for_name("ret")
                .expect("Returns query must have @ret capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ret_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.collect_return_node_value_spans(capture.node, out);
                        }
                    }
                }
            }
        }
    }

    fn collect_return_node_value_spans(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        if self.language == Language::Go {
            if let Some(child) = node.named_child(0) {
                self.collect_identifier_path_spans(child, out);
            }
            return;
        }

        if let Some(child) = node.named_child(0) {
            self.collect_identifier_path_spans(child, out);
        }
    }

    /// Find all R-value identifiers on diff lines within a function (excluding L-values).
    pub fn rvalue_identifiers_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut rvalues = Vec::new();

            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                        {
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_all_identifiers(rhs, &mut rvalues);
                            }
                        }
                    }
                }
            }

            // Also collect from call arguments
            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line) {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_all_identifiers(args, &mut rvalues);
                                }
                                if let Some(func_name_node) =
                                    self.language.call_function_name(&capture.node)
                                {
                                    let name = self.node_text(&func_name_node).to_string();
                                    rvalues.push((name, line));
                                }
                            }
                        }
                    }
                }
            }

            return rvalues;
        }

        let mut rvalues = Vec::new();
        self.collect_rvalues_manual(*func_node, lines, &mut rvalues);
        rvalues
    }

    /// Manual recursive R-value collection (pre-query fallback).
    fn collect_rvalues_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_all_identifiers(rhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_all_identifiers(args, out);
            }
            if let Some(func_name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalues_manual(child, lines, out);
        }
    }

    fn collect_all_identifiers<'a>(&self, node: Node<'a>, out: &mut Vec<(String, usize)>) {
        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((name, line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_all_identifiers(child, out);
        }
    }

    /// Find all lines in a function scope where a variable name is referenced.
    pub fn find_variable_references(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
    ) -> BTreeSet<usize> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Identifiers) {
            let ident_idx = query
                .capture_index_for_name("ident")
                .expect("Identifiers query must have @ident capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ident_idx && self.node_text(&capture.node) == var_name {
                        lines.insert(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = BTreeSet::new();
        self.collect_variable_refs_manual(*func_node, var_name, &mut lines);
        lines
    }

    /// Manual recursive variable ref collection (pre-query fallback).
    fn collect_variable_refs_manual(
        &self,
        node: Node<'_>,
        var_name: &str,
        out: &mut BTreeSet<usize>,
    ) {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs_manual(child, var_name, out);
        }
    }

    /// Find variable references with basic scope awareness.
    ///
    /// Like `find_variable_references`, but filters out references that lie inside
    /// an inner scope block which re-declares the same variable name — i.e., the
    /// reference would be bound to the inner declaration, not the one at `def_line`.
    pub fn find_variable_references_scoped(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> BTreeSet<usize> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Identifiers) {
            let ident_idx = query
                .capture_index_for_name("ident")
                .expect("Identifiers query must have @ident capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ident_idx && self.node_text(&capture.node) == var_name {
                        // Bottom-up scope check: walk from the capture node up to the
                        // function boundary. If any enclosing scope block re-declares
                        // var_name and doesn't contain def_line, skip this reference.
                        if !self.is_shadowed_at(&capture.node, func_node, var_name, def_line) {
                            lines.insert(capture.node.start_position().row + 1);
                        }
                    }
                }
            }
            return lines;
        }

        let mut lines = BTreeSet::new();
        self.collect_variable_refs_scoped_manual(*func_node, var_name, def_line, &mut lines);
        lines
    }

    /// Check whether a reference node is shadowed by a re-declaration in an inner scope.
    /// Walks bottom-up from the identifier node to the function boundary.
    fn is_shadowed_at(
        &self,
        node: &Node<'_>,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> bool {
        let func_id = func_node.id();
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.id() == func_id {
                break;
            }
            if self.language.is_scope_block(parent.kind()) {
                let scope_start = parent.start_position().row + 1;
                let scope_end = parent.end_position().row + 1;
                let def_in_scope = def_line >= scope_start && def_line <= scope_end;
                if !def_in_scope && self.scope_has_declaration(parent, var_name) {
                    return true;
                }
            }
            current = parent.parent();
        }
        false
    }

    /// Manual recursive scoped variable ref collection (pre-query fallback).
    fn collect_variable_refs_scoped_manual(
        &self,
        node: Node<'_>,
        var_name: &str,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let node_start = node.start_position().row + 1;
        let node_end = node.end_position().row + 1;

        if self.language.is_scope_block(node.kind()) {
            let def_in_scope = def_line >= node_start && def_line <= node_end;
            if !def_in_scope && self.scope_has_declaration(node, var_name) {
                return;
            }
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs_scoped_manual(child, var_name, def_line, out);
        }
    }

    /// Check whether a scope block directly declares a variable with the given name.
    /// Does not recurse into nested scope blocks (those have their own scope).
    fn scope_has_declaration(&self, node: Node<'_>, var_name: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.language.is_declaration_node(child.kind()) {
                if let Some(name_node) = self.language.declaration_name(&child) {
                    if self.node_text(&name_node) == var_name {
                        return true;
                    }
                }
            }
            // Recurse into non-scope children (e.g., for-loop init, expression statements)
            // but stop at nested scope blocks to avoid false positives.
            if !self.language.is_scope_block(child.kind())
                && self.scope_has_declaration(child, var_name)
            {
                return true;
            }
        }
        false
    }

    /// Check if a variable has any bare (non-field-access) references in a function
    /// body (excluding the parameter list itself).
    ///
    /// Returns true if the variable is used as a standalone identifier, not just as
    /// the base of a field/member access (e.g., `data` in `use(data)` counts, but
    /// `dev` in `dev.name` does not). Used to decide whether to register a parameter
    /// Def for interprocedural data flow.
    pub fn has_bare_references(&self, func_node: &Node<'_>, var_name: &str) -> bool {
        if let Some(body) = self.function_body_node(func_node) {
            return self.find_bare_ref(body, var_name);
        }
        self.find_bare_ref_excluding_parameters(*func_node, func_node, var_name)
    }

    fn function_body_node<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        let func_node = self.unwrap_decorated(*func_node);
        func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"))
    }

    fn find_bare_ref_excluding_parameters(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        var_name: &str,
    ) -> bool {
        if let Some(params) = self.find_parameters_node(func_node) {
            if node.start_byte() >= params.start_byte() && node.end_byte() <= params.end_byte() {
                return false;
            }
        }
        if Self::is_field_access_node(node.kind()) {
            return false;
        }
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            if let Some(parent) = node.parent() {
                if Self::is_field_access_node(parent.kind()) {
                    return false;
                }
            }
            return true;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.find_bare_ref_excluding_parameters(child, func_node, var_name) {
                return true;
            }
        }
        false
    }

    fn find_bare_ref(&self, node: Node<'_>, var_name: &str) -> bool {
        // If this is a field access (dev.name, dev->name), skip it — the `dev`
        // identifier inside is not a bare reference.
        if Self::is_field_access_node(node.kind()) {
            return false;
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            // Check parent: if parent is a field access, this isn't a bare ref.
            if let Some(parent) = node.parent() {
                if Self::is_field_access_node(parent.kind()) {
                    return false;
                }
            }
            return true;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.find_bare_ref(child, var_name) {
                return true;
            }
        }
        false
    }

    /// Find all references to an AccessPath within a function scope.
    ///
    /// For simple paths (no fields), delegates to `find_variable_references_scoped`.
    /// For field-qualified paths (`dev->name`), searches for matching field_expression
    /// nodes as well as bare identifier references to the base.
    pub fn find_path_references_scoped(
        &self,
        func_node: &Node<'_>,
        path: &AccessPath,
        def_line: usize,
    ) -> BTreeSet<usize> {
        if path.is_simple() {
            return self.find_variable_references_scoped(func_node, &path.base, def_line);
        }
        // For field-qualified paths, find matching field expressions.
        // NOTE: this filters to references AFTER def_line (`line > def_line` in `collect_path_refs`),
        // which does NOT match `find_variable_references_scoped` for simple paths — that returns ALL
        // non-shadowed references, including earlier lines (so a loop-carried `def@N → use@M (M<N)`
        // edge exists for simple paths but not field paths). `cpg/trace.rs::taint_neighbors` has a
        // field-path recovery arm that re-supplies these dropped edges for the reasoning layer; this
        // production DFG path is left as-is for byte-stability (Option C). See planA-followups Round 9.
        let mut lines = BTreeSet::new();
        self.collect_path_refs(*func_node, path, def_line, &mut lines);
        lines
    }

    fn collect_path_refs(
        &self,
        node: Node<'_>,
        path: &AccessPath,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let line = node.start_position().row + 1;

        // Check field/member access expressions (all languages)
        if Self::is_field_access_node(node.kind()) {
            let text = self.node_text(&node).to_string();
            let node_path = AccessPath::from_expr(&text);
            if node_path == *path && line > def_line {
                out.insert(line);
                return; // Don't recurse into matched field expression
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_path_refs(child, path, def_line, out);
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

    /// Collect all statement-level nodes within a function for CFG construction.
    ///
    /// Returns `(line, node_kind)` pairs in source order. Only direct children of
    /// the function body and top-level children of compound statements are included
    /// — nested expressions within a statement are not separate CFG nodes.
    pub fn statements_in_function(&self, func_node: &Node<'_>) -> Vec<(usize, String)> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut stmts = Vec::new();
        // Find the function body (compound_statement, block, etc.)
        let body = func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"));
        if let Some(body_node) = body {
            self.collect_statements(body_node, &mut stmts);
        }
        stmts.sort_by_key(|(line, _)| *line);
        stmts.dedup_by_key(|(line, _)| *line);
        stmts
    }

    /// Byte-bearing sibling of `statements_in_function`.
    pub fn statement_spans_in_function(&self, func_node: &Node<'_>) -> Vec<StatementSpan> {
        let func_node = self.unwrap_decorated(*func_node);
        let mut stmts = Vec::new();
        if let Some(body_node) = self.function_body_node(&func_node) {
            self.collect_statement_spans(body_node, &mut stmts);
        }
        stmts.sort_by_key(|stmt| stmt.line);
        stmts.dedup_by_key(|stmt| stmt.line);
        stmts
    }

    fn collect_statements(&self, node: Node<'_>, out: &mut Vec<(usize, String)>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                out.push((line, kind.to_string()));

                // For control flow nodes, also recurse into their bodies
                // to find nested statements (then-branch, else-branch, loop body)
                if self.language.is_control_flow_node(kind) {
                    self.collect_nested_statements(child, out);
                }
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                // Recurse into blocks
                self.collect_statements(child, out);
            }
        }
    }

    fn collect_statement_spans(&self, node: Node<'_>, out: &mut Vec<StatementSpan>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            let line = child.start_position().row + 1;

            if self.language.is_statement_node(kind) {
                out.push(StatementSpan {
                    line,
                    kind: kind.to_string(),
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                });

                if self.language.is_control_flow_node(kind) {
                    self.collect_nested_statement_spans(child, out);
                }
            } else if kind == "compound_statement" || kind == "block" || kind == "statement_block" {
                self.collect_statement_spans(child, out);
            }
        }
    }

    fn collect_nested_statements(&self, node: Node<'_>, out: &mut Vec<(usize, String)>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
            {
                self.collect_statements(child, out);
                self.collect_nested_statements(child, out);
            } else if self.language.is_control_flow_node(kind) {
                // Nested control flow (if inside if, etc.)
                let line = child.start_position().row + 1;
                out.push((line, kind.to_string()));
                self.collect_nested_statements(child, out);
            }
        }
    }

    fn collect_nested_statement_spans(&self, node: Node<'_>, out: &mut Vec<StatementSpan>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "compound_statement"
                || kind == "block"
                || kind == "statement_block"
                || kind == "else_clause"
                || kind == "elif_clause"
                || kind == "else_if_clause"
                || kind == "switch_body"
                || kind == "case_statement"
                || kind == "default_statement"
                || kind == "match_block"
                || kind == "match_arm"
            {
                self.collect_statement_spans(child, out);
                self.collect_nested_statement_spans(child, out);
            } else if self.language.is_control_flow_node(kind) {
                out.push(StatementSpan {
                    line: child.start_position().row + 1,
                    kind: kind.to_string(),
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                });
                self.collect_nested_statement_spans(child, out);
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
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
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
                    if let Some(name_node) = self.language.call_function_name(&capture.node) {
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
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut calls = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                let qualifier = self
                                    .language
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
                    if let Some(name_node) = self.language.call_function_name(&capture.node) {
                        let name = self.node_text(&name_node).to_string();
                        let qualifier_node = self.language.call_function_qualifier(&capture.node);
                        let qualifier = qualifier_node.map(|q| self.node_text(&q).to_string());
                        let (arg_count, arg_spread) = self
                            .language
                            .call_arguments(&capture.node)
                            .map(|args| {
                                let mut count = 0usize;
                                let mut spread = false;
                                let mut cursor2 = args.walk();
                                for child in args.children(&mut cursor2) {
                                    if !child.is_named() {
                                        continue; // skip punctuation (, )
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
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .language
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
            self.collect_calls_manual_with_qualifier_and_spans(child, lines, out);
        }
    }

    fn collect_calls_manual_with_qualifier(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize, Option<String>)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                let qualifier = self
                    .language
                    .call_function_qualifier(&node)
                    .map(|q| self.node_text(&q).to_string());
                out.push((name, line, qualifier));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual_with_qualifier(child, lines, out);
        }
    }

    fn collect_calls_manual_with_spans<'a>(
        &'a self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<CallSiteMeta<'a>>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
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
            self.collect_calls_manual_with_spans(child, lines, out);
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
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls_manual(child, lines, out);
        }
    }

    /// Collect all function call names on specific lines (1-indexed).
    /// Returns a map from line number to list of called function names found on that line.
    /// Only matches actual AST call nodes — ignores calls inside comments or string literals.
    pub fn call_names_on_lines(&self, lines: &[usize]) -> BTreeMap<usize, Vec<String>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let line_set: BTreeSet<usize> = lines.iter().copied().collect();

        if let Some(query) = get_query(self.language, QueryKind::Calls) {
            let call_idx = query
                .capture_index_for_name("call")
                .expect("Calls query must have @call capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == call_idx {
                        let line = capture.node.start_position().row + 1;
                        if line_set.contains(&line) {
                            if let Some(name_node) = self.language.call_function_name(&capture.node)
                            {
                                let name = self.node_text(&name_node).to_string();
                                result.entry(line).or_default().push(name);
                            }
                        }
                    }
                }
            }
            return result;
        }

        let mut result: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        self.collect_call_names_at_lines_manual(self.tree.root_node(), &line_set, &mut result);
        result
    }

    fn collect_call_names_at_lines_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut BTreeMap<usize, Vec<String>>,
    ) {
        let line = node.start_position().row + 1;
        if self.language.is_call_node(node.kind()) && lines.contains(&line) {
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.entry(line).or_default().push(name);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_call_names_at_lines_manual(child, lines, out);
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

    /// Extract binding names in declaration order for non-positional consumers.
    ///
    /// Go grouped declarations intentionally expand (`a, b string` yields both
    /// bindings), so DFG and reasoning receive one definition seed per name.
    pub fn function_parameter_names(&self, func_node: &Node<'_>) -> Vec<String> {
        if self.language == Language::Go {
            return self
                .function_parameter_occurrences(func_node)
                .into_iter()
                .map(|(name, _, _)| name)
                .collect();
        }

        let mut names = Vec::new();
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if let Some(name) = self.extract_param_name(&child) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// Byte-bearing sibling of `function_parameter_names`.
    ///
    /// The tuple is `(name, start_byte, end_byte)` for the actual parameter
    /// binding token. The DFG still anchors parameter defs to the function start
    /// line for call-boundary compatibility.
    pub fn function_parameter_occurrences(&self, func_node: &Node<'_>) -> Vec<ParameterOccurrence> {
        let mut params_out = Vec::new();
        if let Some(params) = self.find_parameters_node(func_node) {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                for name_node in self.parameter_binding_name_nodes(child) {
                    params_out.push((
                        self.node_text(&name_node).to_string(),
                        name_node.start_byte(),
                        name_node.end_byte(),
                    ));
                }
            }
        }
        params_out
    }

    /// Exact runtime parameter slots in declaration order. A deterministic
    /// prefix is returned through the first variadic, keyword-only, or
    /// unrepresentable parameter; duplicate bindings and parse recovery return
    /// `None` because they cannot support an exact positional mapping.
    pub fn function_parameter_slots(&self, func_node: &Node<'_>) -> Option<Vec<String>> {
        crate::parameter_slots::slots(self, func_node)
            .map(|occurrences| occurrences.into_iter().map(|(name, _, _)| name).collect())
    }

    /// Byte-bearing twin of [`Self::function_parameter_slots`].
    pub fn function_parameter_slot_occurrences(
        &self,
        func_node: &Node<'_>,
    ) -> Option<Vec<ParameterOccurrence>> {
        crate::parameter_slots::slots(self, func_node)
    }

    fn parameter_binding_name_nodes<'a>(&self, node: Node<'a>) -> Vec<Node<'a>> {
        if self.language == Language::Go
            && matches!(
                node.kind(),
                "parameter_declaration" | "variadic_parameter_declaration"
            )
        {
            let mut cursor = node.walk();
            return node
                .children(&mut cursor)
                .filter(|child| child.kind() == "identifier" && self.node_text(child) != "_")
                .collect();
        }
        self.extract_param_name_node(&node).into_iter().collect()
    }

    /// Find the parameters node within a function definition.
    /// Handles the C/C++ declarator chain (function_definition → declarator → function_declarator → parameters).
    pub(crate) fn find_parameters_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        let node = self.unwrap_decorated(*node);
        // Direct "parameters" field (Go, Rust, Python, JS/TS, Java, Lua)
        if let Some(params) = node.child_by_field_name("parameters") {
            return Some(params);
        }
        // C/C++: navigate declarator chain to find function_declarator with parameters
        if let Some(declarator) = node.child_by_field_name("declarator") {
            return self.find_params_in_declarator(&declarator);
        }
        None
    }

    /// Recursively search a C/C++ declarator chain for a parameters node.
    fn find_params_in_declarator<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        if let Some(params) = node.child_by_field_name("parameters") {
            return Some(params);
        }
        // Navigate: pointer_declarator → function_declarator, etc.
        if let Some(decl) = node.child_by_field_name("declarator") {
            return self.find_params_in_declarator(&decl);
        }
        // Walk children for other declarator wrappers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind().contains("declarator") {
                if let Some(params) = self.find_params_in_declarator(&child) {
                    return Some(params);
                }
            }
        }
        None
    }

    fn go_short_decl_names(&self, left: Node<'_>) -> Vec<String> {
        if let Some(name) = self.simple_binding_text(&left) {
            return vec![name];
        }
        let mut cursor = left.walk();
        left.named_children(&mut cursor)
            .filter_map(|child| self.simple_binding_text(&child))
            .collect()
    }

    fn go_enclosing_binding_scope<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let mut parent = node.parent();
        while let Some(scope) = parent {
            if matches!(
                scope.kind(),
                "block"
                    | "expression_case"
                    | "type_case"
                    | "communication_case"
                    | "if_statement"
                    | "for_statement"
                    | "expression_switch_statement"
                    | "type_switch_statement"
            ) {
                return Some(scope);
            }
            parent = scope.parent();
        }
        None
    }

    fn go_scope_declares_name_before(
        &self,
        func_node: &Node<'_>,
        scope: Node<'_>,
        name: &str,
        before_byte: usize,
    ) -> bool {
        fn walk(
            this: &ParsedFile,
            node: Node<'_>,
            is_root: bool,
            name: &str,
            before_byte: usize,
        ) -> bool {
            if node.start_byte() >= before_byte {
                return false;
            }
            if !is_root
                && (this.language.function_node_types().contains(&node.kind())
                    || matches!(
                        node.kind(),
                        "block"
                            | "expression_case"
                            | "type_case"
                            | "communication_case"
                            | "func_literal"
                            | "if_statement"
                            | "for_statement"
                            | "expression_switch_statement"
                            | "type_switch_statement"
                            | "select_statement"
                    ))
            {
                return false;
            }
            if node.kind() == "short_var_declaration" {
                if let Some(left) = node.child_by_field_name("left") {
                    return this.go_short_decl_names(left).iter().any(|n| n == name);
                }
            }
            if node.kind() == "var_spec" {
                let mut cursor = node.walk();
                if node
                    .children_by_field_name("name", &mut cursor)
                    .any(|child| this.node_text(&child).trim() == name)
                {
                    return true;
                }
            }
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .any(|child| walk(this, child, false, name, before_byte));
            found
        }

        let parameter_owner = scope.parent().filter(|parent| {
            matches!(
                parent.kind(),
                "function_declaration" | "method_declaration" | "func_literal"
            )
        });
        let parameter_owner = parameter_owner.or_else(|| {
            func_node
                .child_by_field_name("body")
                .filter(|body| body.start_byte() == scope.start_byte())
                .map(|_| *func_node)
        });
        if parameter_owner.is_some_and(|owner| {
            self.find_parameters_node(&owner).is_some_and(|params| {
                let mut cursor = params.walk();
                let found = params.children(&mut cursor).any(|param| {
                    param
                        .child_by_field_name("type")
                        .is_some_and(|ty| self.go_parameter_binds_name(param, ty, name))
                });
                found
            })
        }) {
            return true;
        }

        walk(self, scope, true, name, before_byte)
    }

    fn go_short_decl_reuses_in_scope(
        &self,
        node: Node<'_>,
        func_node: &Node<'_>,
        receiver: &str,
        call_start_byte: usize,
    ) -> bool {
        let Some(scope) = self.go_enclosing_binding_scope(node) else {
            return false;
        };
        if !(scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()) {
            return false;
        }
        let Some(left) = node.child_by_field_name("left") else {
            return false;
        };
        let names = self.go_short_decl_names(left);
        if names.len() < 2
            || !names.iter().any(|name| name == receiver)
            || !self.go_scope_declares_name_before(func_node, scope, receiver, node.start_byte())
        {
            return false;
        }
        names.iter().any(|name| {
            name != receiver
                && name != "_"
                && !self.go_scope_declares_name_before(func_node, scope, name, node.start_byte())
        })
    }

    fn walk_receiver_bindings(
        &self,
        node: Node<'_>,
        is_root: bool,
        receiver: &str,
        call_line: usize,
        call_start_byte: usize,
        go_filter_declarations_to_call_scope: bool,
        enable_go_same_scope_reuse: bool,
        found: &mut Option<(String, crate::resolution::ReceiverRecovery)>,
        first_found: &mut Option<(String, crate::resolution::ReceiverRecovery)>,
        bindings: &mut usize,
        go_lexical_rebinding: &mut bool,
        recover_var: bool,
    ) {
        use crate::languages::Language;
        use crate::resolution::ReceiverRecovery;

        if node.start_position().row + 1 > call_line {
            return;
        }
        if node.start_byte() >= call_start_byte {
            return;
        }
        if !is_root && self.language.function_node_types().contains(&node.kind()) {
            // Python: nested function name IS a binding in the enclosing scope.
            if matches!(self.language, Language::Python) {
                if let Some(name_node) = self.language.function_name(&node) {
                    if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            return;
        }
        if !is_root
            && matches!(self.language, Language::Python)
            && matches!(
                node.kind(),
                "class_definition" | "class_declaration" | "class"
            )
        {
            // The class name IS a binding in the enclosing scope.
            if let Some(name_node) = node.child_by_field_name("name") {
                if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                    *bindings += 1;
                    *found = None;
                }
            }
            return;
        }
        // Python 3 comprehensions have their own scope — bindings inside them
        // do NOT shadow names in the enclosing function scope.
        if !is_root
            && matches!(self.language, Language::Python)
            && matches!(
                node.kind(),
                "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
                    | "generator_expression"
            )
        {
            return;
        }

        match (self.language, node.kind()) {
            (Language::Rust, "let_declaration") => {
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                        *bindings += 1;
                        if let Some(ty) = node.child_by_field_name("type") {
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::ConstructorLocal,
                            ));
                        } else if let Some(value) = node.child_by_field_name("value") {
                            *found = self
                                .constructor_type(&value)
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal));
                        } else {
                            *found = None;
                        }
                    } else if self.node_binds_name(pattern, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Go, "short_var_declaration") => {
                let left = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("name"));
                let right = node
                    .child_by_field_name("right")
                    .or_else(|| node.child_by_field_name("value"));
                let visible_scope = !go_filter_declarations_to_call_scope
                    || self.go_enclosing_binding_scope(node).is_some_and(|scope| {
                        scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()
                    });
                if let Some(left) = left.filter(|_| visible_scope) {
                    if enable_go_same_scope_reuse
                        && self.go_short_decl_reuses_in_scope(
                            node,
                            &node
                                .parent()
                                .and_then(|mut parent| {
                                    while !self
                                        .language
                                        .function_node_types()
                                        .contains(&parent.kind())
                                        && parent.kind() != "func_literal"
                                    {
                                        parent = parent.parent()?;
                                    }
                                    Some(parent)
                                })
                                .unwrap_or(node),
                            receiver,
                            call_start_byte,
                        )
                    {
                        // Go reuses an existing binding for the already-declared
                        // LHS name. The post-merge pass proves the call's first
                        // return type still matches before trusting this fact.
                    } else if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *go_lexical_rebinding |= *bindings > 0;
                        *bindings += 1;
                        *found = right.and_then(|r| {
                            self.constructor_type(&r)
                                .or_else(|| self.first_constructor_type_child(&r))
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal))
                        });
                    } else if self.node_binds_name(left, receiver) {
                        *go_lexical_rebinding |= *bindings > 0;
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Go, "type_switch_statement") => {
                let mut cursor = node.walk();
                let call_in_case = node.named_children(&mut cursor).any(|case| {
                    case.kind() == "type_case"
                        && case.start_byte() <= call_start_byte
                        && call_start_byte < case.end_byte()
                });
                if call_in_case {
                    if let Some(alias) = node.child_by_field_name("alias") {
                        if self.simple_binding_text(&alias).as_deref() == Some(receiver)
                            || self.node_binds_name(alias, receiver)
                        {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = None;
                        }
                    }
                }
            }
            (Language::Go, "range_clause") => {
                let call_in_body = node
                    .parent()
                    .filter(|parent| parent.kind() == "for_statement")
                    .and_then(|parent| parent.child_by_field_name("body"))
                    .is_some_and(|body| {
                        body.start_byte() <= call_start_byte && call_start_byte < body.end_byte()
                    });
                if call_in_body && self.node_text(&node).contains(":=") {
                    if let Some(left) = node.child_by_field_name("left") {
                        if self.simple_binding_text(&left).as_deref() == Some(receiver)
                            || self.node_binds_name(left, receiver)
                        {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = None;
                        }
                    }
                }
            }
            (Language::Go, "var_spec") if recover_var => {
                // var_spec.name is multiple:true; match only the bound name(s), never names in
                // the declared type or initializer (that would be a false recovery).
                let visible_scope = !go_filter_declarations_to_call_scope
                    || self.go_enclosing_binding_scope(node).is_some_and(|scope| {
                        scope.start_byte() <= call_start_byte && call_start_byte < scope.end_byte()
                    });
                let mut cur = node.walk();
                let names: Vec<_> = node.children_by_field_name("name", &mut cur).collect();
                let matched = visible_scope
                    && names
                        .iter()
                        .any(|n| self.simple_binding_text(n).as_deref() == Some(receiver));
                if matched {
                    *go_lexical_rebinding |= *bindings > 0;
                    *bindings += 1;
                    if let Some(ty) = node.child_by_field_name("type") {
                        // `var r T` / `var a, b T` — the declared type applies to every name.
                        *found = Some((self.node_text(&ty).to_string(), ReceiverRecovery::VarDecl));
                    } else if names.len() == 1 {
                        // `var r = X{}` — single name ↔ single value; safe to recover the type.
                        *found = node
                            .child_by_field_name("value")
                            .and_then(|value| {
                                self.constructor_type(&value)
                                    .or_else(|| self.first_constructor_type_child(&value))
                            })
                            .map(|ty| (ty, ReceiverRecovery::VarDecl));
                    } else {
                        // multi-name untyped (`var a, b = X{}, Y{}`) — name↔value alignment is
                        // ambiguous; bail rather than emit a wrong edge (whole-branch review #2).
                        *found = None;
                    }
                }
            }
            (Language::Python, "assignment") => {
                let left = node.child_by_field_name("left");
                if let Some(left) = left {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        if !self.receiver_declaration_reaches_call(node, call_start_byte) {
                            *found = None;
                        } else if let Some(ty) = node.child_by_field_name("type") {
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::ConstructorLocal,
                            ));
                        } else if let Some(right) = node.child_by_field_name("right") {
                            *found = self
                                .constructor_type(&right)
                                .map(|ty| (ty, ReceiverRecovery::ConstructorLocal));
                        } else {
                            *found = None;
                        }
                    } else if self.node_binds_name(left, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "augmented_assignment") => {
                // `Foo += x` — augmented assignment rebinds the name; type unrecoverable.
                if let Some(left) = node.child_by_field_name("left") {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "for_statement") => {
                // `for x in items:` — iteration variable; type unrecoverable.
                if let Some(left) = node.child_by_field_name("left") {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(left, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            // NOTE: comprehension `for_in_clause` targets are NOT counted here.
            // Python 3 comprehensions have their own scope; the early-return
            // above prevents recursion into them, so they cannot over-suppress
            // names in the enclosing function scope.
            (Language::Python, "named_expression") => {
                // Walrus: `(x := compute())` — type unrecoverable.
                if let Some(name_node) = node.child_by_field_name("name") {
                    if self.simple_binding_text(&name_node).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "as_pattern") => {
                // `with ... as x:` / `except ... as x:` / `case ... as x:`
                // tree-sitter-python: as_pattern field `alias` = as_pattern_target
                // wrapping an identifier.  Also handle destructuring targets
                // like `with cm() as (Foo, other):` where simple_binding_text
                // returns None but node_binds_name finds the name inside a tuple.
                if let Some(alias) = node.child_by_field_name("alias") {
                    if self.simple_binding_text(&alias).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(alias, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            (Language::Python, "case_clause") => {
                // Match/case capture patterns bind names in the enclosing scope.
                // `case Foo:` captures and binds `Foo` (if not dotted).
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if self.simple_binding_text(&pattern).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    } else if self.node_binds_name(pattern, receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            // P11 (S2 fail-closed requirement, the P9 lesson in Go form):
            // `Language::function_node_types()` omits Go function literals
            // (closures), so without a lexical-scope fence the walk scans
            // straight through EVERY closure body in the enclosing function
            // as if it were still that function's own scope. Two distinct
            // hazards follow from that, both fixed here (B1, codex
            // impl-review BLOCKER):
            //   1. a closure PARAMETER of the same name would silently
            //      shadow-and-vanish (invisible to `bindings`), letting a
            //      stale outer recovery leak into the closure's own call
            //      sites — the call is INSIDE this closure;
            //   2. a closure-LOCAL `:=`/`var` binding (e.g. `x := newT()`)
            //      would be visible to a call OUTSIDE this closure entirely
            //      (a sibling closure, already closed by the time an
            //      unrelated later statement runs) and could mint a false
            //      Exact edge for that unrelated call — the call is OUTSIDE
            //      this closure.
            //
            // Fence by lexical scope: when the call site does not lie
            // within this literal's own byte span, this closure's ENTIRE
            // subtree is out of scope for `receiver` — `return` immediately,
            // skipping the generic recursion below (never `continue`/fall
            // through, or a sibling-closure local would still leak via that
            // trailing walk). Only when the call IS inside do we record the
            // closure parameter's declared type. It is recoverable when this
            // is the only same-name binding; if an outer binding was already
            // seen, P17's first-fact evidence marks the receiver proof
            // shadowed and routes through R3. The generic recursion then scans
            // ONLY this literal's body for later local rebindings.
            (Language::Go, "func_literal") if !is_root => {
                let call_inside =
                    call_start_byte >= node.start_byte() && call_start_byte < node.end_byte();
                if !call_inside {
                    return;
                }
                if let Some(params) = self.find_parameters_node(&node) {
                    let mut pcursor = params.walk();
                    for param in params.children(&mut pcursor) {
                        if param.kind() != "parameter_declaration" {
                            continue;
                        }
                        let Some(ty) = param.child_by_field_name("type") else {
                            continue;
                        };
                        if self.go_parameter_binds_name(param, ty, receiver) {
                            *go_lexical_rebinding |= *bindings > 0;
                            *bindings += 1;
                            *found = Some((
                                self.node_text(&ty).to_string(),
                                ReceiverRecovery::TypedParam,
                            ));
                        }
                    }
                }
            }
            (Language::Go, "assignment_statement") | (Language::Rust, "assignment_expression") => {
                let left = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("left_operand"));
                if let Some(left) = left {
                    if self.simple_binding_text(&left).as_deref() == Some(receiver) {
                        *bindings += 1;
                        *found = None;
                    }
                }
            }
            _ => {}
        }
        if *bindings == 1 && first_found.is_none() {
            *first_found = found.clone();
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_receiver_bindings(
                child,
                false,
                receiver,
                call_line,
                call_start_byte,
                go_filter_declarations_to_call_scope,
                enable_go_same_scope_reuse,
                found,
                first_found,
                bindings,
                go_lexical_rebinding,
                recover_var,
            );
        }
    }

    fn constructor_type(&self, node: &Node<'_>) -> Option<String> {
        use crate::languages::Language;

        fn python_constructor_name(text: &str) -> bool {
            let owner = crate::call_graph::python_qualified_receiver_parts(text)
                .map(|(_, owner)| owner)
                .unwrap_or(text);
            owner.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && (owner != text || text.chars().all(|c| c.is_alphanumeric() || c == '_'))
        }

        match node.kind() {
            "call_expression" => {
                let function = node
                    .child_by_field_name("function")
                    .or_else(|| node.child_by_field_name("name"))?;
                let text = self.node_text(&function).trim();
                if let (Language::Rust, Some((ty, constructor))) =
                    (self.language, text.rsplit_once("::"))
                {
                    if !matches!(constructor, "new" | "default") {
                        return None;
                    }
                    return Some(ty.rsplit("::").next().unwrap_or(ty).to_string());
                }
                if self.language == Language::Go {
                    return text
                        .strip_prefix("New")
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
                if self.language == Language::Python && python_constructor_name(text) {
                    return Some(text.to_string());
                }
                None
            }
            "call" if self.language == Language::Python => {
                let function = node
                    .child_by_field_name("function")
                    .or_else(|| node.child_by_field_name("name"))?;
                let text = self.node_text(&function).trim();
                if python_constructor_name(text) {
                    return Some(text.to_string());
                }
                None
            }
            "struct_expression" | "composite_literal" => {
                let ty = node
                    .child_by_field_name("name")
                    .or_else(|| node.child_by_field_name("type"))?;
                Some(self.node_text(&ty).to_string())
            }
            _ => None,
        }
    }

    fn first_constructor_type_child(&self, node: &Node<'_>) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(ty) = self.constructor_type(&child) {
                return Some(ty);
            }
        }
        None
    }

    fn node_binds_name(&self, node: Node<'_>, receiver: &str) -> bool {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == receiver {
            return true;
        }
        // Attribute/subscript accesses reference but don't bind the name.
        if matches!(
            node.kind(),
            "attribute" | "subscript" | "member_expression" | "subscript_expression"
        ) {
            return false;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.node_binds_name(child, receiver) {
                return true;
            }
        }
        false
    }

    fn simple_binding_text(&self, node: &Node<'_>) -> Option<String> {
        let text = self.node_text(node).trim();
        if text.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some(text.to_string());
        }
        text.strip_prefix("mut ")
            .map(str::trim)
            .filter(|s| s.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .map(str::to_string)
    }

    fn go_parameter_binds_name(&self, param: Node<'_>, ty: Node<'_>, receiver: &str) -> bool {
        let mut cursor = param.walk();
        for child in param.children(&mut cursor) {
            if child.start_byte() >= ty.start_byte() {
                continue;
            }
            if child.kind() == "identifier" && self.node_text(&child) == receiver {
                return true;
            }
        }
        false
    }

    fn parameter_binds_name_before_type(
        &self,
        param: Node<'_>,
        ty: Node<'_>,
        receiver: &str,
    ) -> bool {
        if let Some(pattern) = param
            .child_by_field_name("pattern")
            .or_else(|| param.child_by_field_name("name"))
        {
            return self.simple_binding_text(&pattern).as_deref() == Some(receiver);
        }
        let mut cursor = param.walk();
        for child in param.children(&mut cursor) {
            if child.start_byte() >= ty.start_byte() {
                continue;
            }
            if self.simple_binding_text(&child).as_deref() == Some(receiver) {
                return true;
            }
        }
        false
    }

    /// Extract the parameter name from a parameter declaration node.
    fn extract_param_name(&self, node: &Node<'_>) -> Option<String> {
        match node.kind() {
            "parameter_declaration" | "optional_parameter_declaration" => {
                // C/C++/Java: has a declarator field containing the identifier
                if let Some(decl) = node.child_by_field_name("declarator") {
                    return Some(self.innermost_identifier(&decl));
                }
                // Fallback: find any identifier child
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            "parameter" => {
                // Rust/Go: name field or pattern field
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(self.node_text(&name).to_string());
                }
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    return Some(self.node_text(&pattern).to_string());
                }
                // Go: last identifier before the type
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            "identifier" => {
                // Python/Lua: parameters may be direct identifiers
                Some(self.node_text(node).to_string())
            }
            "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => {
                // Python: typed / default / splat parameter forms.
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(self.node_text(&name).to_string());
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(self.node_text(&child).to_string());
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn extract_param_name_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match node.kind() {
            "parameter_declaration" | "optional_parameter_declaration" => {
                if let Some(decl) = node.child_by_field_name("declarator") {
                    return self.innermost_identifier_node(&decl);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            "parameter" => {
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(name);
                }
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    return self.first_binding_identifier(pattern);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            "identifier" => Some(*node),
            "typed_parameter"
            | "typed_default_parameter"
            | "default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => {
                if let Some(name) = node.child_by_field_name("name") {
                    return Some(name);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn first_binding_identifier<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        if self.is_binding_identifier_node(node.kind()) {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.first_binding_identifier(child) {
                return Some(found);
            }
        }
        None
    }

    /// Find the innermost identifier in a C/C++ declarator chain.
    fn innermost_identifier(&self, node: &Node<'_>) -> String {
        if node.kind() == "identifier" || node.kind() == "field_identifier" {
            return self.node_text(node).to_string();
        }
        // Check declarator field first
        if let Some(decl) = node.child_by_field_name("declarator") {
            return self.innermost_identifier(&decl);
        }
        // Walk children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return self.node_text(&child).to_string();
            }
            if child.kind().contains("declarator") {
                return self.innermost_identifier(&child);
            }
        }
        self.node_text(node).to_string()
    }

    fn innermost_identifier_node<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        if node.kind() == "identifier" || node.kind() == "field_identifier" {
            return Some(*node);
        }
        if let Some(decl) = node.child_by_field_name("declarator") {
            if let Some(found) = self.innermost_identifier_node(&decl) {
                return Some(found);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "field_identifier" {
                return Some(child);
            }
            if child.kind().contains("declarator") {
                if let Some(found) = self.innermost_identifier_node(&child) {
                    return Some(found);
                }
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
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        // Count non-punctuation children to find the Nth argument
                        let mut arg_idx = 0;
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            // Skip punctuation: ( ) , and whitespace
                            if child.is_named() {
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
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if child.is_named() {
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
            if let Some(name_node) = self.language.call_function_name(&node) {
                let name = self.node_text(&name_node);
                if name == callee_name {
                    if let Some(args_node) = self.language.call_arguments(&node) {
                        let mut cursor = args_node.walk();
                        for child in args_node.children(&mut cursor) {
                            if child.is_named() {
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
                        if let Some(name_node) = self.language.call_function_name(&capture.node) {
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
            if let Some(name_node) = self.language.call_function_name(&node) {
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

/// Extract the variable names that are logically written by an lvalue expression.
///
/// Handles three C/C++ indirection patterns:
///
/// - `*p`       → `["p"]`          pointer dereference — the pointer itself is mutated through
/// - `dev->f`   → `["f"]`          field via arrow — track only the qualified field path
/// - `buf[i]`   → `["buf"]`        array subscript — track the base array
/// - `x`        → `["x"]`          simple identifier — unchanged behaviour
///
/// For anything that doesn't match a known pattern (e.g. complex nested expressions)
/// the function returns an empty vec so the caller silently skips it rather than
/// storing an unusable composite name like `"*p"` or `"buf[0]"`.
/// Extract structured AccessPaths from an L-value expression.
///
/// For field access expressions (dev->field, obj.field), returns only the
/// fully qualified path: AccessPath { base: "dev", fields: ["field"] }.
///
/// Phase 2 field-sensitive matching: field assignments no longer emit a
/// base-only def, so `dev->name = x` creates a def for `dev.name` only —
/// taint on `dev.name` does NOT leak to `dev.id` through the base.
fn extract_lvalue_paths(lhs_text: &str) -> Vec<AccessPath> {
    let lhs = lhs_text.trim();

    // Pointer dereference: *p, **p
    if lhs.starts_with('*') {
        let inner = lhs.trim_start_matches('*').trim();
        let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
        if !inner.is_empty() && is_plain_ident(inner) {
            return vec![AccessPath::simple(inner)];
        }
        return vec![];
    }

    // Field via arrow: dev->field or dev->config->timeout
    if lhs.contains("->") {
        let full = AccessPath::from_expr(lhs);
        if full.has_fields() {
            return vec![full];
        }
        let base = AccessPath::simple(full.base.clone());
        return vec![base];
    }

    // Dot access: obj.field
    if lhs.contains('.') {
        let full = AccessPath::from_expr(lhs);
        if full.has_fields() {
            return vec![full];
        }
        let base = AccessPath::simple(full.base.clone());
        return vec![base];
    }

    // Array subscript: buf[i]
    if let Some(bracket) = lhs.find('[') {
        let base_str = lhs[..bracket].trim();
        if !base_str.is_empty() && is_plain_ident(base_str) {
            return vec![AccessPath::from_expr(lhs)];
        }
        return vec![];
    }

    // Simple identifier
    if !lhs.is_empty() && is_plain_ident(lhs) {
        return vec![AccessPath::simple(lhs)];
    }

    vec![]
}

fn extract_lvalue_names(lhs_text: &str) -> Vec<String> {
    let lhs = lhs_text.trim();

    // Pointer dereference: *p, **p
    if lhs.starts_with('*') {
        let inner = lhs.trim_start_matches('*').trim();
        // Strip surrounding parens: (*p)
        let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
        if !inner.is_empty() && is_plain_ident(inner) {
            return vec![inner.to_string()];
        }
        return vec![];
    }

    // Field via arrow: dev->field
    if let Some(arrow) = lhs.find("->") {
        let base = lhs[..arrow].trim();
        let field = lhs[arrow + 2..].trim();
        let mut names = Vec::new();
        if !field.is_empty() && is_plain_ident(field) {
            names.push(field.to_string());
        }
        if !base.is_empty() && is_plain_ident(base) {
            names.push(base.to_string());
        }
        return names;
    }

    // Array subscript: buf[i]  — only track the base name
    if let Some(bracket) = lhs.find('[') {
        let base = lhs[..bracket].trim();
        if !base.is_empty() && is_plain_ident(base) {
            return vec![base.to_string()];
        }
        return vec![];
    }

    // Simple identifier (also covers `obj.field` by treating `obj` as the def).
    // We intentionally ignore dot access for non-pointer structs here; the base
    // identifier appears as a separate rvalue and will be tracked via rvalue edges.
    if !lhs.is_empty() && is_plain_ident(lhs) {
        return vec![lhs.to_string()];
    }

    vec![]
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
/// scanned over RAW source lines including comments and string literals
/// (reproduces legacy text-scan semantics).
///
/// Completeness: a field can only produce a target when the accessor occurrence
/// is followed (after optional whitespace) by `=`, which terminates the
/// identifier run — so every PRODUCTIVE field is a maximal run here. Prefix
/// occurrences (`->cbx` while querying `cb`) never produce targets; their
/// consumption side-effects are reproduced by running the legacy core.
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
                if let Some(name_node) = pf.language.call_function_name(&node) {
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
