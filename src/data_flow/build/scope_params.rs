//! Rule S and function-literal formals and named results (#694 slice B).
//!
//! Both now have producer defs, so a body use of one is reached by its own def and S may drop the
//! pair from an outer def of the same spelling (a different variable). S keeps a pair only where
//! the real value of the formal or result arrives through something prism does not connect, so
//! the name coincidence is the only edge that carries it (owner precedent from #709: keep main's
//! behaviour where the replacement path is not connected; the shapes are listed in CLAUDE.md):
//!
//! * a formal of a literal that is not called in place (passed to a callee, assigned, returned):
//!   the callee or a later call supplies the value, and no argument is bound to the formal. Kept
//!   only for defs whose variable is in scope where the literal is created: a sibling literal's
//!   formal or any other out-of-scope def cannot be an argument of it.
//! * a formal of a literal called in place (`func(v T){..}(args)`, `go`/`defer` included) whose
//!   argument list reads the name: `func(v int){..}(v)` carries the outer value into the formal.
//!   An argument list that never mentions the name cannot carry the outer variable, so the pair
//!   drops.
//! * a named result read inside a callable nested in the one that declares it: `return e` assigns
//!   the result, and a deferred literal reads it afterwards, so the def of `e` reaches the read by
//!   name only. A read in the declaring callable's own body is reached by the writes S sees.
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

enum Form {
    Formal {
        /// Start byte of the literal that declares the formal.
        literal: usize,
        /// Identifier spellings in the argument list when the literal is called in place.
        in_place_args: Option<BTreeSet<String>>,
    },
    Result {
        /// Start byte of the function or literal that declares the result.
        callable: usize,
    },
}

/// Per owner: the forms of its formal and result declarations, by declaration index.
pub(super) struct ParamFacts {
    by_declaration: BTreeMap<usize, Form>,
}

const CALLABLES: [&str; 3] = ["func_literal", "function_declaration", "method_declaration"];

impl ParamFacts {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>, facts: &BindingFacts) -> Self {
        let mut by_declaration = BTreeMap::new();
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if CALLABLES.contains(&node.kind()) {
                let literal = node.kind() == "func_literal";
                let in_place_args = literal.then(|| in_place_arguments(parsed, node)).flatten();
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
                                        in_place_args: in_place_args.clone(),
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
        Self { by_declaration }
    }

    /// The pair (def `def_index` of `name`, a read at `use_start` that resolves to declaration
    /// `other`) is kept although the two are different declarations.
    pub(super) fn keeps(
        &self,
        parsed: &ParsedFile,
        facts: &BindingFacts,
        name: &str,
        def_index: usize,
        other: usize,
        use_start: usize,
    ) -> bool {
        match self.by_declaration.get(&other) {
            None => false,
            Some(Form::Result { callable }) => {
                enclosing_callable(parsed, use_start).is_some_and(|c| c != *callable)
            }
            Some(Form::Formal {
                literal,
                in_place_args,
            }) => {
                facts.def_scope_contains(def_index, *literal)
                    && in_place_args
                        .as_ref()
                        .is_none_or(|args| args.contains(name))
            }
        }
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

/// The identifier spellings of the argument list when `literal` is the callee of a call
/// (`func(..){..}(args)`, also under `go`/`defer`); `None` when it is not called in place.
fn in_place_arguments(parsed: &ParsedFile, literal: Node<'_>) -> Option<BTreeSet<String>> {
    let call = literal.parent().filter(|p| p.kind() == "call_expression")?;
    if call.child_by_field_name("function")?.id() != literal.id() {
        return None;
    }
    let args = call.child_by_field_name("arguments")?;
    let mut out = BTreeSet::new();
    let mut stack = vec![args];
    while let Some(node) = stack.pop() {
        if node.kind() == "identifier" {
            out.insert(parsed.node_text(&node).to_owned());
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    Some(out)
}
