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
//! * a formal of a literal passed directly as a call argument (`sort.Slice`, `t.Run`): only the
//!   callee calls it, so only a def whose variable is in scope where the literal is created can be
//!   the value it passes.
//! * a named result read inside a callable nested in the one that declares it: `return e` assigns
//!   the result, and a deferred literal reads it afterwards, so the def of `e` reaches the read by
//!   name only. A read in the declaring callable's own body is reached by the writes S sees.
//! * a def inside a literal called in place whose call is on the right of an assignment or
//!   declaration naming the same variable (`err := func() error { err := do(); return err }()`):
//!   the value comes out through the call result, which no edge connects.
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
    },
    Result {
        /// Start byte of the function or literal that declares the result.
        callable: usize,
    },
}

/// A literal called in place whose call is on the right of a statement that names variables.
struct ReturnSink {
    start: usize,
    end: usize,
    names: BTreeSet<String>,
}

/// Per owner: the forms of its formal and result declarations, by declaration index.
pub(super) struct ParamFacts {
    by_declaration: BTreeMap<usize, Form>,
    sinks: Vec<ReturnSink>,
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
        let mut sinks = Vec::new();
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if CALLABLES.contains(&node.kind()) {
                let literal = node.kind() == "func_literal";
                let direct = literal && node.parent().is_some_and(|p| p.kind() == "argument_list");
                if let Some(names) = literal.then(|| return_sink(parsed, node)).flatten() {
                    sinks.push(ReturnSink {
                        start: node.start_byte(),
                        end: node.end_byte(),
                        names,
                    });
                }
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
            sinks,
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
            Some(Form::Formal { literal, direct }) => {
                if *direct {
                    facts.def_scope_contains(def_index, *literal)
                } else {
                    !declare_only
                }
            }
        }
    }

    /// The def at `def_start` lies inside a literal called in place whose call is on the right of
    /// an assignment or declaration that names `name`, and the read at `use_start` lies outside it.
    pub(super) fn returned_out(&self, name: &str, def_start: usize, use_start: usize) -> bool {
        self.sinks.iter().any(|s| {
            s.start <= def_start
                && def_start < s.end
                && !(s.start <= use_start && use_start < s.end)
                && s.names.contains(name)
        })
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

/// The variables named on the left of the assignment or declaration whose right side holds the
/// call of `literal`, when `literal` is the callee of that call (`func(..){..}(args)`, also as
/// an operand of a larger expression or the init of an `if`/`switch`).
fn return_sink(parsed: &ParsedFile, literal: Node<'_>) -> Option<BTreeSet<String>> {
    let call = literal.parent().filter(|p| p.kind() == "call_expression")?;
    if call.child_by_field_name("function")?.id() != literal.id() {
        return None;
    }
    let mut child = call;
    while let Some(node) = child.parent() {
        let (left, right) = match node.kind() {
            "short_var_declaration" | "assignment_statement" => ("left", "right"),
            "var_spec" => ("name", "value"),
            k if k == "block"
                || k == "func_literal"
                || k.ends_with("_statement")
                || k.ends_with("_declaration") =>
            {
                return None;
            }
            _ => {
                child = node;
                continue;
            }
        };
        let side = node.child_by_field_name(right)?;
        if !(side.start_byte() <= child.start_byte() && child.end_byte() <= side.end_byte()) {
            return None;
        }
        let mut names = BTreeSet::new();
        let mut cursor = node.walk();
        let mut stack: Vec<Node<'_>> = node.children_by_field_name(left, &mut cursor).collect();
        while let Some(n) = stack.pop() {
            if n.kind() == "identifier" {
                names.insert(parsed.node_text(&n).to_owned());
            }
            let mut inner = n.walk();
            stack.extend(n.named_children(&mut inner));
        }
        return Some(names);
    }
    None
}
