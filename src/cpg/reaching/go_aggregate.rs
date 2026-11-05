//! S1 Go syntax certificates: genuine writes and maximal eager value reads.
use crate::access_path::{AccessPath, DEFAULT_K_LIMIT};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Debug)]
pub(super) struct Occurrence {
    pub path: AccessPath,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    /// The read is the receiver of a call `R.M(..)` (S2): the selector `M`.
    pub receiver: Option<String>,
    /// Index into `Facts::closures` when the read sits in a function literal.
    pub closure: Option<usize>,
    /// Start byte of the simple statement (or header clause) evaluating it.
    pub stmt: usize,
    /// Start byte of the `for` clause whose condition holds this read.
    pub cond_of: Option<usize>,
}

/// Evaluation-order facts of one genuine write.
#[derive(Clone, Copy, Debug)]
pub(super) struct WriteMeta {
    pub stmt: usize,
    /// Start byte of the `for` clause whose post statement holds this write.
    pub post_of: Option<usize>,
}

/// An outermost function literal of the owner and every path written
/// anywhere inside it (nested literals included).
#[derive(Debug)]
pub(super) struct Closure {
    pub line: usize,
    pub writes: Vec<AccessPath>,
}

#[derive(Default)]
pub(super) struct Facts {
    pub writes: BTreeMap<(AccessPath, usize), usize>,
    pub write_meta: BTreeMap<(AccessPath, usize), WriteMeta>,
    pub reads: Vec<Occurrence>,
    pub closures: Vec<Closure>,
    stmt: usize,
    depth: usize,
    clause: Option<(usize, bool)>,
    cur_closure: Option<usize>,
    shadow: Vec<BTreeSet<String>>,
    owner_names: BTreeSet<String>,
    scope: Option<super::go_recall_scope::FileScope>,
}

/// Parse the entire path, refusing indices and depth truncation. Dereference
/// normalizes storage exactly as the owner's existing DefSites do.
pub(super) fn path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    match node.kind() {
        "identifier" => Some(AccessPath::simple(parsed.node_text(&node))),
        "selector_expression" => {
            let mut p = path(parsed, node.child_by_field_name("operand")?)?;
            let field = node.child_by_field_name("field")?;
            if field.kind() != "field_identifier" || p.fields.len() == DEFAULT_K_LIMIT {
                return None;
            }
            p.fields.push(parsed.node_text(&field).to_owned());
            Some(p)
        }
        "parenthesized_expression" => path(parsed, node.named_child(0)?),
        "unary_expression" if parsed.node_text(&node.child_by_field_name("operator")?) == "*" => {
            path(parsed, node.child_by_field_name("operand")?)
        }
        _ => None,
    }
}

impl Facts {
    pub fn new(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut facts = Self::default();
        if !owner.has_error() {
            facts.owner_names = super::go_recall_scope::declared_names(parsed, owner);
            if let Some(body) = owner.child_by_field_name("body") {
                facts.walk(parsed, body);
            }
        }
        facts
    }

    fn write(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        if let Some(p) = path(parsed, node) {
            match self.cur_closure {
                Some(c) => self.closures[c].writes.push(p),
                None => {
                    self.write_meta.insert(
                        (p.clone(), node.start_byte()),
                        WriteMeta {
                            stmt: self.stmt,
                            post_of: self.clause.filter(|c| c.1).map(|c| c.0),
                        },
                    );
                    self.writes.insert((p, node.start_byte()), node.end_byte());
                }
            }
        }
        // An LHS map key is evaluated; the storage operand is not an aggregate read.
        if node.kind() == "index_expression" {
            if let Some(index) = node.child_by_field_name("index") {
                self.value(parsed, index);
            }
        }
    }

    /// A simple statement is one evaluation unit; nested ones (a call inside an
    /// assignment) share the outermost statement's key.
    fn enter(&mut self, node: Node<'_>) -> usize {
        let saved = self.stmt;
        if self.depth == 0 {
            self.stmt = node.start_byte();
        }
        self.depth += 1;
        saved
    }

    fn leave(&mut self, saved: usize) {
        self.depth -= 1;
        self.stmt = saved;
    }

    /// A function literal: its reads are capture reads of the owner, its
    /// writes are recorded against the outermost literal only.
    fn closure(&mut self, parsed: &ParsedFile, literal: Node<'_>) {
        let saved = self.cur_closure;
        let index = saved.unwrap_or_else(|| {
            self.closures.push(Closure {
                line: literal.start_position().row + 1,
                writes: Vec::new(),
            });
            self.closures.len() - 1
        });
        self.cur_closure = Some(index);
        self.shadow
            .push(super::go_recall_scope::declared_names(parsed, literal));
        let (stmt, depth, clause) = (self.stmt, self.depth, self.clause.take());
        self.depth = 0;
        if let Some(body) = literal.child_by_field_name("body") {
            self.walk(parsed, body);
        }
        (self.stmt, self.depth, self.clause) = (stmt, depth, clause);
        self.shadow.pop();
        self.cur_closure = saved;
    }

    fn walk(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        match node.kind() {
            "func_literal" => {
                self.closure(parsed, node);
                return;
            }
            "function_declaration" | "method_declaration" => return,
            "for_clause" => {
                let id = node.start_byte();
                if let Some(init) = node.child_by_field_name("initializer") {
                    self.walk(parsed, init);
                }
                if let Some(cond) = node.child_by_field_name("condition") {
                    self.clause = Some((id, false));
                    self.walk(parsed, cond);
                }
                if let Some(update) = node.child_by_field_name("update") {
                    self.clause = Some((id, true));
                    self.walk(parsed, update);
                }
                self.clause = None;
                return;
            }
            "assignment_statement" | "short_var_declaration" => {
                let saved = self.enter(node);
                if let Some(left) = node.child_by_field_name("left") {
                    let mut cursor = left.walk();
                    for child in left.named_children(&mut cursor) {
                        self.write(parsed, child);
                    }
                }
                if let Some(right) = node.child_by_field_name("right") {
                    self.value(parsed, right);
                }
                self.leave(saved);
                return;
            }
            "var_spec" => {
                let saved = self.enter(node);
                let mut cursor = node.walk();
                for name in node.children_by_field_name("name", &mut cursor) {
                    self.write(parsed, name);
                }
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                self.leave(saved);
                return;
            }
            "inc_statement" | "dec_statement" => {
                let saved = self.enter(node);
                if let Some(value) = node.named_child(0) {
                    self.write(parsed, value);
                }
                self.leave(saved);
                return;
            }
            "return_statement" => {
                let saved = self.enter(node);
                if let Some(value) = node.named_child(0) {
                    self.value(parsed, value);
                }
                self.leave(saved);
                return;
            }
            "send_statement" => {
                let saved = self.enter(node);
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                self.leave(saved);
                return;
            }
            "call_expression" => {
                let saved = self.enter(node);
                self.value(parsed, node);
                self.leave(saved);
                return;
            }
            // Type-only identifiers are never consumers.
            "type_declaration" | "type_spec" => return,
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk(parsed, child);
        }
    }

    fn value(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        if let Some(p) = path(parsed, node) {
            // Keep the actual identifier/selector span inside &/*/parentheses,
            // so an existing Use can be reused without changing its identity.
            let mut actual = node;
            while matches!(
                actual.kind(),
                "parenthesized_expression" | "unary_expression"
            ) {
                actual = actual
                    .child_by_field_name("operand")
                    .or_else(|| actual.named_child(0))
                    .unwrap();
            }
            self.read(p, actual, None);
            return;
        }
        match node.kind() {
            "func_literal" => self.closure(parsed, node),
            "call_expression" => {
                self.receiver(parsed, node);
                if let Some(f) = node
                    .child_by_field_name("function")
                    .filter(|f| f.kind() == "func_literal")
                {
                    self.closure(parsed, f);
                }
                // S1 excludes the callable/receiver, but arguments are eager
                // even in defer/go statements and nested calls.
                if let Some(args) = node.child_by_field_name("arguments") {
                    self.value(parsed, args);
                }
            }
            "composite_literal" => {
                if let Some(body) = node.child_by_field_name("body") {
                    self.value(parsed, body);
                }
            }
            "unary_expression" => {
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|op| matches!(parsed.node_text(&op), "&" | "*"))
                {
                    if let Some(operand) = node.child_by_field_name("operand") {
                        self.value(parsed, operand);
                    }
                } else {
                    self.walk_children(parsed, node);
                }
            }
            "expression_list"
            | "argument_list"
            | "literal_value"
            | "literal_element"
            | "parenthesized_expression" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.value(parsed, child);
                }
            }
            "keyed_element" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                // Bare keys of struct literals are field names. Only a
                // syntactically proven map literal admits its key as a value.
                let mut parent = node.parent();
                while let Some(p) = parent {
                    // An elided literal (`{Name: "v"}` as a map element) has no
                    // type of its own here: its keys are never proven values.
                    if p.kind() == "literal_value"
                        && p.parent().is_some_and(|q| q.kind() == "literal_element")
                    {
                        break;
                    }
                    if p.kind() == "composite_literal" {
                        if p.child_by_field_name("type")
                            .is_some_and(|t| t.kind() == "map_type")
                        {
                            if let Some(key) = node.child_by_field_name("key") {
                                self.value(parsed, key);
                            }
                        }
                        break;
                    }
                    parent = p.parent();
                }
            }
            "index_expression" => {
                if let Some(index) = node.child_by_field_name("index") {
                    self.value(parsed, index);
                }
            }
            // Binary identity tests, selector prefixes and type operands do
            // not consume aggregates. Still discover calls nested within them.
            _ => self.walk_children(parsed, node),
        }
    }

    /// Record a read; inside a function literal it is a capture read, unless
    /// the literal (or one enclosing it) declares the root name itself.
    fn read(&mut self, path: AccessPath, actual: Node<'_>, receiver: Option<String>) {
        if self.cur_closure.is_some() && self.shadow.iter().any(|s| s.contains(&path.base)) {
            return;
        }
        self.reads.push(Occurrence {
            path,
            line: actual.start_position().row + 1,
            start: actual.start_byte(),
            end: actual.end_byte(),
            receiver,
            closure: self.cur_closure,
            stmt: self.stmt,
            cond_of: self.clause.filter(|c| !c.1).map(|c| c.0),
        });
    }

    /// `R.M(..)` with a static path `R`: a receiver read of `R`. Refused when
    /// `R` is rooted at an import qualifier that the owner does not shadow.
    /// The selector travels with the edge: a func-typed field named `M`
    /// anywhere in the package is refused later, over the whole repository
    /// (`cpg::go_package_fields`). Method values are never calls.
    fn receiver(&mut self, parsed: &ParsedFile, call: Node<'_>) {
        // In `R.M().N()` only `M`'s receiver counts; walk down the chain.
        let mut call = call;
        loop {
            self.receiver_of(parsed, call);
            match call
                .child_by_field_name("function")
                .filter(|f| f.kind() == "selector_expression")
                .and_then(|f| f.child_by_field_name("operand"))
                .filter(|o| o.kind() == "call_expression")
            {
                Some(inner) => call = inner,
                None => break,
            }
        }
    }

    fn receiver_of(&mut self, parsed: &ParsedFile, call: Node<'_>) {
        let Some(function) = call
            .child_by_field_name("function")
            .filter(|f| f.kind() == "selector_expression")
        else {
            return;
        };
        let (Some(operand), Some(field)) = (
            function.child_by_field_name("operand"),
            function.child_by_field_name("field"),
        ) else {
            return;
        };
        let Some(p) = path(parsed, operand) else {
            return;
        };
        let scope = self
            .scope
            .get_or_insert_with(|| super::go_recall_scope::file_scope(parsed));
        if scope.imports.contains(&p.base)
            && !self.owner_names.contains(&p.base)
            && !self.shadow.iter().any(|s| s.contains(&p.base))
        {
            return;
        }
        let mut actual = operand;
        while matches!(
            actual.kind(),
            "parenthesized_expression" | "unary_expression"
        ) {
            actual = actual
                .child_by_field_name("operand")
                .or_else(|| actual.named_child(0))
                .unwrap();
        }
        self.read(p, actual, Some(parsed.node_text(&field).to_owned()));
    }

    fn walk_children(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk(parsed, child);
        }
    }
}
