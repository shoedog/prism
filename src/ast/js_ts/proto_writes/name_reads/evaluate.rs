//! Evaluation order and the default transfer for unsupported syntax.
use super::*;

impl<'a> Analysis<'a> {
    pub(super) fn visit(&mut self, node: Node<'a>, mut context: Context<'a>, state: &mut State) {
        let outside = context;
        if !self.charge(context.owner, 1) {
            self.inventory(node, context, state);
            return;
        }
        if is_owner(node) && node != context.owner {
            for eager in super::super::eager_owner_inputs(node) {
                self.visit(eager, context, state);
            }
            self.defer_owner(node, context);
            return;
        }
        if is_scope(node) {
            context.scope = node;
            self.enter(node, context, state);
        }
        match node.kind() {
            "identifier" | "shorthand_property_identifier" | "this" | "super" => {
                let base = self.file.node_text(&node).to_string();
                let mut version = self.version(&base, context, state);
                version.mutated = false;
                version.incomplete |= state.dead;
                self.read_path(node, base.clone(), base, context, version, true);
            }
            "member_expression" | "subscript_expression" => {
                self.member(node, context, state);
            }
            "assignment_expression"
            | "augmented_assignment_expression"
            | "variable_declarator"
            | "update_expression" => {
                let declaration = node.kind() == "variable_declarator";
                let target = field(node, "left")
                    .or_else(|| field(node, "argument"))
                    .or_else(|| field(node, "name"));
                let right = field(node, "right").or_else(|| field(node, "value"));
                let assignment = declaration || node.kind() == "assignment_expression";
                let logical = matches!(operator(node, self.file), "||=" | "&&=" | "??=");
                if let Some(target) = target {
                    let member = matches!(
                        peel(target).kind(),
                        "member_expression" | "subscript_expression"
                    );
                    let captured = if !declaration {
                        self.target_reads(target, context, state, !assignment)
                    } else {
                        BTreeSet::new()
                    };
                    let bypass = if logical {
                        let Some(bypass) = self.fork(context, state) else {
                            self.inventory(node, context, state);
                            return;
                        };
                        Some(bypass)
                    } else {
                        None
                    };
                    if let Some(right) = right {
                        self.visit(right, context, state);
                    }
                    let origin = if assignment || logical {
                        right.map_or(Origin::Unknown, Origin::Value)
                    } else {
                        Origin::Unknown
                    };
                    if member {
                        self.mutate(captured, peel(target).into(), state);
                    } else if !super::super::bare_var(node) {
                        self.write(target, origin, context, state);
                    }
                    if let Some(bypass) = bypass {
                        self.join(context, state, &bypass, false);
                    }
                }
            }
            "if_statement" | "ternary_expression" | "binary_expression"
                if node.kind() != "binary_expression"
                    || matches!(operator(node, self.file), "&&" | "||" | "??") =>
            {
                if let Some(test) = field(node, "condition").or_else(|| field(node, "left")) {
                    self.visit(test, context, state);
                }
                let Some(mut alternative) = self.fork(context, state) else {
                    self.inventory(node, context, state);
                    return;
                };
                if let Some(left) = field(node, "consequence").or_else(|| field(node, "right")) {
                    self.visit(left, context, state);
                }
                if let Some(right) = field(node, "alternative") {
                    self.visit(right, context, &mut alternative);
                }
                self.join(context, state, &alternative, false);
            }
            _ if is_loop(node) => self.loop_body(node, context, state),
            "with_statement" => {
                if let Some(object) = field(node, "object") {
                    self.visit(object, outside, state);
                }
                if let Some(body) = field(node, "body") {
                    self.visit(body, context, state);
                }
            }
            "unary_expression" if operator(node, self.file) == "delete" => {
                if let Some(target) = field(node, "argument") {
                    let receiver = self.target_reads(target, context, state, false);
                    self.mutate(receiver, peel(target).into(), state);
                }
            }
            "return_statement" | "throw_statement" | "break_statement" | "continue_statement" => {
                for child in children(node) {
                    self.visit(child, context, state);
                }
                if !state.dead {
                    self.jump(node, context, state);
                }
            }
            "call_expression" | "new_expression" => self.visit_call(node, context, state),
            "catch_clause" => {
                if let Some(parameter) = field(node, "parameter") {
                    self.write(parameter, Origin::Unknown, context, state);
                }
                if let Some(body) = field(node, "body") {
                    self.visit(body, context, state);
                }
            }
            "try_statement" => self.visit_try(node, context, state),
            "switch_statement" => self.visit_switch(node, context, state),
            "class" | "class_declaration" | "abstract_class_declaration" => {
                self.visit_class(node, context, state);
            }
            "import_statement"
            | "type_alias_declaration"
            | "interface_declaration"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "function_signature"
            | "method_signature"
            | "abstract_method_signature" => {}
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion" => {
                let value = peel(node);
                if value != node {
                    self.visit(value, context, state);
                }
            }
            // Explicitly admit grammar containers and expressions whose value
            // children are evaluated in source order. Other nodes have an
            // uncertain transfer even though we still inventory their inputs.
            "program"
            | "statement_block"
            | "expression_statement"
            | "empty_statement"
            | "lexical_declaration"
            | "variable_declaration"
            | "else_clause"
            | "labeled_statement"
            | "finally_clause"
            | "class_static_block"
            | "class_heritage"
            | "extends_clause"
            | "computed_property_name"
            | "decorator"
            | "arguments"
            | "binary_expression"
            | "unary_expression"
            | "sequence_expression"
            | "object"
            | "pair"
            | "array"
            | "spread_element"
            | "template_string"
            | "template_substitution"
            | "string"
            | "string_fragment"
            | "escape_sequence"
            | "number"
            | "regex"
            | "true"
            | "false"
            | "null"
            | "undefined"
            | "export_statement"
            | "export_clause"
            | "export_specifier"
            | "jsx_element"
            | "jsx_opening_element"
            | "jsx_closing_element"
            | "jsx_self_closing_element"
            | "jsx_attribute"
            | "jsx_expression"
            | "jsx_text" => {
                for child in value_children(self.file, node) {
                    self.visit(child, context, state);
                }
            }
            _ => self.uncertain(node, context, state),
        }
        if matches!(node.kind(), "labeled_statement" | "switch_statement") {
            if let Some(exit) = self.jumps.remove(&(node.id(), "break_statement")) {
                self.join(context, state, &exit.state, false);
            }
        }
    }
    fn defer_owner(&mut self, node: Node<'a>, context: Context<'a>) {
        self.deferred
            .entry(context.owner.id())
            .or_default()
            .insert(node.start_byte(), (node, context));
    }

    fn uncertain(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        // Keep both skipped and evaluated effects for a syntax form whose
        // transfer is outside the closed list. Widen before as well as after.
        let has_effects = self.effects.writes_in(node).next().is_some();
        if has_effects {
            self.widen(node, context, state);
        }
        let bypass = if has_effects {
            let Some(bypass) = self.fork(context, state) else {
                self.inventory(node, context, state);
                return;
            };
            Some(bypass)
        } else {
            None
        };
        for child in value_children(self.file, node) {
            self.visit(child, context, state);
        }
        if let Some(bypass) = bypass {
            self.join(context, state, &bypass, true);
        }
    }

    fn visit_class(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let name = field(node, "name");
        let body = field(node, "body");
        for child in value_children(self.file, node)
            .filter(|child| Some(*child) != body && child.kind() != "decorator")
        {
            self.visit(child, context, state);
        }
        let Some(body) = body else { return };
        // All member keys are evaluated before any static initializer runs.
        // Method bodies and instance field values may run much later.
        for member in children(body) {
            if is_owner(member) {
                for input in super::super::eager_owner_inputs(member) {
                    self.visit(input, context, state);
                }
                self.defer_owner(member, context);
            } else {
                for input in children(member)
                    .filter(|child| matches!(child.kind(), "computed_property_name" | "decorator"))
                {
                    self.visit(input, context, state);
                }
                if member.kind() == "decorator" {
                    self.visit(member, context, state);
                }
            }
        }
        for decorator in children(node).filter(|child| child.kind() == "decorator") {
            self.visit(decorator, context, state);
        }
        if let Some(name) = name {
            state.install(name.start_byte(), name.into());
        }
        for member in children(body) {
            if member.kind() == "class_static_block" {
                self.visit(member, context, state);
            } else if let Some(value) = field(member, "value") {
                if super::super::instance_initializer(member).is_some() {
                    if let Some(mut instance) = self.fork(context, state) {
                        let outer = self.deferred_initializer.replace(context.owner.id());
                        self.visit(value, context, &mut instance);
                        self.deferred_initializer = outer;
                    } else {
                        self.inventory(value, context, state);
                    }
                } else {
                    self.visit(value, context, state);
                }
            }
        }
    }

    fn visit_call(&mut self, node: Node<'a>, context: Context<'a>, state: &mut State) {
        let mutations: Vec<_> = mutation_inputs(self.file, &self.lexical, node, context).collect();
        if let Some(callee) = field(node, "function").or_else(|| field(node, "constructor")) {
            self.visit(callee, context, state);
        }
        let bypass = if optional_chain(node) {
            let Some(bypass) = self.fork(context, state) else {
                if let Some(args) = field(node, "arguments") {
                    self.inventory(args, context, state);
                }
                return;
            };
            Some(bypass)
        } else {
            None
        };
        let iteration = super::super::iteration(self.file, node);
        let mut callback = None;
        if let Some(arguments) = field(node, "arguments") {
            for (index, arg) in children(arguments).enumerate() {
                if index == 0 && iteration.is_some() && super::super::is_inline_callback(peel(arg))
                {
                    callback = Some(peel(arg));
                } else {
                    self.visit(arg, context, state);
                }
            }
        }
        if let Some(callback) = callback {
            self.function(callback, context, state, iteration);
        }
        for receiver in mutations {
            let objects = self.receiver_objects(receiver, context);
            self.mutate(objects, node.into(), state);
        }
        if let Some(bypass) = bypass {
            self.join(context, state, &bypass, true);
        }
    }
}

// Parentheses terminate the chain. A later access or call on the bare
// chain's result still short-circuits on the original optional segment.
pub(super) fn optional_chain(mut node: Node<'_>) -> bool {
    for _ in 0..128 {
        if !matches!(
            node.kind(),
            "member_expression"
                | "subscript_expression"
                | "call_expression"
                | "non_null_expression"
        ) {
            return false;
        }
        if field(node, "optional_chain").is_some() || super::super::has_token(node, "?.") {
            return true;
        }
        let Some(receiver) = field(node, "object")
            .or_else(|| field(node, "function"))
            .or_else(|| {
                (node.kind() == "non_null_expression")
                    .then(|| children(node).next())
                    .flatten()
            })
        else {
            return false;
        };
        node = receiver;
    }
    // Beyond the proof depth, preserve the bypass as a possibility.
    true
}
