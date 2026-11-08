//! Rule S and function-literal formals and named results (#694 slice B).
//!
//! Both now have producer defs, so a body use of one is reached by its own def and S may drop the
//! pair from an outer def of the same spelling (a different variable). S keeps a pair only where
//! the real value of the formal or result arrives through something prism does not connect, so
//! the name coincidence is the only edge that carries it (owner precedent from #709: keep main's
//! behaviour where the replacement path is not connected; the shapes are listed in CLAUDE.md):
//!
//! * a formal of a literal that is not a direct call argument (called in place, `go`/`defer`
//!   included, or assigned, stored, returned, spread): the call's arguments, however indirect
//!   (`arg := v + 1; func(v int){..}(arg)`), or a later call from any scope supply the value, and
//!   no argument is bound to the formal. Every def of the spelling that main carried stays; a def
//!   that is itself a formal or result of another declaration is a new def and drops.
//! * a formal of a literal passed directly as an argument whose callee only calls it during the
//!   call (`sort.Slice`, `t.Run`, `filepath.Walk`, `x.Visit` as statements): only a def whose
//!   variable is in scope where the literal is created can be the value it passes. Any other
//!   argument-list literal keeps every ordinary def: a conversion (`http.HandlerFunc(func..)`),
//!   `append(fs, func..)` or a wrapper whose result is used hands the literal back to the owner,
//!   and a callee that is a literal or a name bound in the owner may call it with its own values.
//!   A callee that stores the literal (`s.register(func..)`, then `s.cb(v)`) is a disclosed limit.
//! * a named result read inside a callable nested in the one that declares it: `return e` assigns
//!   the result, and a deferred literal reads it afterwards, so the def of `e` reaches the read by
//!   name only. A read in the declaring callable's own body is reached by the writes S sees.
//! * a def in a callable nested strictly inside the callable of the read, for a name that a
//!   formal or named result of the owner declares (`nested_in_use_callable`): main kept every such
//!   pair, so this adds no edge main lacks. It covers a value that leaves a literal by its call
//!   result, a deferred literal that sets the carrier's result and a literal bound inside another
//!   literal. Ordinary-name return flow out of a literal is not modelled (as on main).
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tree_sitter::Node;

enum Form {
    Formal {
        /// Start byte of the literal that declares the formal.
        literal: usize,
        /// The literal is passed directly as a call argument (and so is not called in place).
        direct: bool,
        /// A discarded callback whose creation-scope drop the owner gate lifted (`function_acceptors`).
        gated: bool,
    },
    Result {
        /// Start byte of the function or literal that declares the result.
        callable: usize,
    },
}

/// Per owner: the forms of its formal and result declarations, by declaration index.
pub(super) struct ParamFacts {
    by_declaration: BTreeMap<usize, Form>,
    callables: RefCell<HashMap<usize, Option<usize>>>,
}

const CALLABLES: [&str; 3] = ["func_literal", "function_declaration", "method_declaration"];

/// The formals and named results of one owner that slice B gives producer defs: the start byte of
/// each name's identifier, with the body span of the callable that declares it (its scope).
pub(super) struct SliceBDefs {
    scope: HashMap<usize, (usize, usize)>,
    names: BTreeSet<String>,
}

impl SliceBDefs {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut out = Self {
            scope: HashMap::new(),
            names: BTreeSet::new(),
        };
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if CALLABLES.contains(&node.kind()) {
                if let Some(body) = node.child_by_field_name("body") {
                    let span = (body.start_byte(), body.end_byte());
                    let mut lists = vec![node.child_by_field_name("result")];
                    if node.kind() == "func_literal" {
                        lists.push(node.child_by_field_name("parameters"));
                    }
                    for list in lists.into_iter().flatten() {
                        if list.kind() != "parameter_list" {
                            continue;
                        }
                        let mut cursor = list.walk();
                        for param in list.named_children(&mut cursor) {
                            let mut names = param.walk();
                            for name in param.children_by_field_name("name", &mut names) {
                                out.scope.insert(name.start_byte(), span);
                                out.names.insert(parsed.node_text(&name).to_owned());
                            }
                        }
                    }
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        out
    }

    /// The scope (body span) of the formal or result declared at identifier byte `start`.
    pub(super) fn scope_of(&self, start: usize) -> Option<(usize, usize)> {
        self.scope.get(&start).copied()
    }

    pub(super) fn empty() -> Self {
        Self {
            scope: HashMap::new(),
            names: BTreeSet::new(),
        }
    }

    pub(super) fn names(&self) -> &BTreeSet<String> {
        &self.names
    }
}

impl ParamFacts {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>, facts: &BindingFacts) -> Self {
        let mut by_declaration = BTreeMap::new();
        let acceptors = function_acceptors(parsed, owner);
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if CALLABLES.contains(&node.kind()) {
                let literal = node.kind() == "func_literal";
                // Only another owner literal with a formal that can take a function value can hold
                // the callback and call it with a def the creation scope does not contain.
                let discarded = literal && discarded_callback(parsed, facts, node);
                let gated = discarded && acceptors.iter().any(|a| *a != node.start_byte());
                let direct = discarded && !gated;
                // A signature name is declared in the body's scope, which starts at the body.
                let Some(body_start) = node.child_by_field_name("body").map(|b| b.start_byte())
                else {
                    continue;
                };
                let mut insert = |list: Option<Node<'_>>, formal: bool| {
                    let Some(list) = list.filter(|l| l.kind() == "parameter_list") else {
                        return;
                    };
                    let mut cursor = list.walk();
                    for param in list.named_children(&mut cursor) {
                        let mut names = param.walk();
                        for name in param.children_by_field_name("name", &mut names) {
                            let text = parsed.node_text(&name);
                            if text == "_" {
                                continue;
                            }
                            if let BindingId::Declaration(d) = facts.binding_at(text, body_start) {
                                let form = if formal {
                                    Form::Formal {
                                        literal: node.start_byte(),
                                        direct,
                                        gated,
                                    }
                                } else {
                                    Form::Result {
                                        callable: node.start_byte(),
                                    }
                                };
                                by_declaration.insert(d, form);
                            }
                        }
                    }
                };
                if literal {
                    insert(node.child_by_field_name("parameters"), true);
                }
                insert(node.child_by_field_name("result"), false);
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        Self {
            by_declaration,
            callables: RefCell::new(HashMap::new()),
        }
    }

    /// The pair (def `def_index` of `name`, a read at `use_start` that resolves to declaration
    /// `other`) is kept although the two are different declarations. `declare_only`: the def is
    /// itself a formal or named result (a new def, never one main carried).
    pub(super) fn keeps(
        &self,
        parsed: &ParsedFile,
        facts: &BindingFacts,
        def_index: usize,
        declare_only: bool,
        other: usize,
        use_start: usize,
    ) -> bool {
        match self.by_declaration.get(&other) {
            None => false,
            Some(Form::Result { callable }) => self
                .enclosing_callable(parsed, use_start)
                .is_some_and(|c| c != *callable),
            Some(Form::Formal {
                literal,
                direct,
                gated,
            }) => {
                if *direct {
                    facts.def_scope_contains(def_index, *literal)
                } else {
                    // A gated callback may be called by another owner literal: a def whose
                    // scope holds the creation point still passes (the direct rule's keep).
                    !declare_only || (*gated && facts.def_scope_contains(def_index, *literal))
                }
            }
        }
    }

    /// The def at `def_start` sits in a callable nested strictly inside the callable that holds
    /// the read at `use_start` (the owner counts as a callable). With a name a formal or named
    /// result declares, main blanket-kept this pair (`address_taken_names`), so keeping it adds
    /// no edge main lacks (the caller excludes a declare-only def, which main never had).
    pub(super) fn nested_in_use_callable(
        &self,
        parsed: &ParsedFile,
        def_start: usize,
        use_start: usize,
    ) -> bool {
        let Some(def) = self.enclosing_callable(parsed, def_start) else {
            return false;
        };
        let Some(use_callable) = self.enclosing_callable(parsed, use_start) else {
            // The read is outside every callable of the owner: any callable encloses nothing of it.
            return true;
        };
        if def == use_callable {
            return false;
        }
        // The use's callable is a proper ancestor of the def's callable.
        let mut node = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(def, def)
            .and_then(|n| {
                let mut n = n;
                while !(CALLABLES.contains(&n.kind()) && n.start_byte() == def) {
                    n = n.parent()?;
                }
                n.parent()
            });
        while let Some(n) = node {
            if CALLABLES.contains(&n.kind()) && n.start_byte() == use_callable {
                return true;
            }
            node = n.parent();
        }
        false
    }

    /// Start byte of the innermost function or literal around `byte` (memoized).
    fn enclosing_callable(&self, parsed: &ParsedFile, byte: usize) -> Option<usize> {
        *self
            .callables
            .borrow_mut()
            .entry(byte)
            .or_insert_with(|| enclosing_callable(parsed, byte))
    }
}

/// Start byte of the innermost function or literal around `byte`.
fn enclosing_callable(parsed: &ParsedFile, byte: usize) -> Option<usize> {
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(byte, byte);
    while let Some(n) = node {
        if CALLABLES.contains(&n.kind()) {
            return Some(n.start_byte());
        }
        node = n.parent();
    }
    None
}

/// `literal` is an argument of a call whose result is discarded (a whole statement, or under
/// `go`/`defer`) and whose callee is neither a literal nor a name bound in the owner: the callee
/// only calls it during the call. Anything else may hand it back or call it with its own values.
fn discarded_callback(parsed: &ParsedFile, facts: &BindingFacts, literal: Node<'_>) -> bool {
    let Some(call) = literal
        .parent()
        .filter(|p| p.kind() == "argument_list")
        .and_then(|list| list.parent())
        .filter(|c| c.kind() == "call_expression")
    else {
        return false;
    };
    if !call.parent().is_some_and(|p| {
        matches!(
            p.kind(),
            "expression_statement" | "go_statement" | "defer_statement"
        )
    }) {
        return false;
    }
    let Some(mut callee) = call.child_by_field_name("function") else {
        return false;
    };
    while callee.kind() == "parenthesized_expression" {
        let Some(inner) = callee.named_child(0) else {
            return false;
        };
        callee = inner;
    }
    match callee.kind() {
        "func_literal" => false,
        "identifier" => !matches!(
            facts.binding_at(parsed.node_text(&callee), call.start_byte()),
            BindingId::Declaration(_)
        ),
        _ => true,
    }
}

/// Start bytes of the owner's literals with a formal able to accept a function value: a function,
/// interface (`any` included) or named type, a variadic of these. Pointer, slice, map, array, chan,
/// struct and predeclared basic types cannot take a literal.
fn function_acceptors(parsed: &ParsedFile, owner: Node<'_>) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![owner];
    while let Some(node) = stack.pop() {
        if node.kind() == "func_literal" {
            let accepts = node.child_by_field_name("parameters").is_some_and(|list| {
                let mut cursor = list.walk();
                let any = list.named_children(&mut cursor).any(|p| {
                    p.child_by_field_name("type")
                        .is_some_and(|t| accepts_function(parsed, t))
                });
                any
            });
            if accepts {
                out.push(node.start_byte());
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    out
}

fn accepts_function(parsed: &ParsedFile, mut ty: Node<'_>) -> bool {
    while ty.kind() == "parenthesized_type" {
        match ty.named_child(0) {
            Some(inner) => ty = inner,
            None => return true,
        }
    }
    match ty.kind() {
        "function_type" | "interface_type" | "qualified_type" | "generic_type" => true,
        // `error` and `any` are interfaces; every other predeclared name is a basic type.
        "type_identifier" => !matches!(
            parsed.node_text(&ty),
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uintptr"
                | "float32"
                | "float64"
                | "complex64"
                | "complex128"
                | "string"
                | "bool"
                | "byte"
                | "rune"
        ),
        _ => false,
    }
}
