//! Intrinsic results whose runtime type follows from the operation and receiver.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn call_domain(&self, call: Node<'a>, depth: usize) -> Option<Domain> {
        let callee = field(call, "function").map(peel)?;
        if field(call, "optional_chain").is_some() {
            return None;
        }
        if ["Number", "parseInt"]
            .iter()
            .any(|name| self.intrinsic(callee, name))
            || self.static_call(call, "Math", "floor")
            || self.prototype_call(call, "Array", "indexOf")
            || self.prototype_call(call, "String", "indexOf")
            || self.method(call, "indexOf").is_some_and(|receiver| {
                self.native_array(receiver)
                    || self.trusted_intrinsic("String") && self.primitive_string(receiver)
            })
        {
            return Some(Domain::Numeric);
        }
        if self.intrinsic(callee, "Symbol")
            || self.static_call(call, "Symbol", "for")
            || self.intrinsic(callee, "Boolean")
            || self.method(call, "test").is_some_and(|receiver| {
                self.trusted_intrinsic("RegExp") && receiver.kind() == "regex"
            })
        {
            return Some(Domain::Benign);
        }
        if self.intrinsic(callee, "String") {
            let [value] = self.args(call)?;
            return self.key_domain(value, depth).map(|domain| {
                if domain == Domain::Numeric {
                    Domain::NumericString
                } else {
                    domain
                }
            });
        }
        self.finite_map(call, depth)
    }

    fn prototype_call(&self, call: Node<'a>, constructor: &str, method: &str) -> bool {
        let Some(callee) = self.method(call, "call") else {
            return false;
        };
        let Some(prototype) = field(callee, "object").map(peel) else {
            return false;
        };
        self.trusted_intrinsic("Function")
            && field(callee, "property").is_some_and(|name| self.text(name) == method)
            && field(prototype, "property").is_some_and(|name| self.text(name) == "prototype")
            && field(prototype, "object").is_some_and(|base| self.intrinsic(base, constructor))
    }

    pub(super) fn member_domain(&self, node: Node<'a>, depth: usize) -> Option<Domain> {
        let receiver = field(node, "object").map(peel)?;
        if let Some(property) = field(node, "property") {
            let name = self.text(property);
            if name == "index" && self.regexp_result(receiver).is_some()
                || name == "length"
                    && (self.native_array(receiver)
                        || self.primitive_string(receiver)
                        || self.typed_array(receiver))
                || self.numeric_record_field(receiver, name, depth)
            {
                return Some(Domain::Numeric);
            }
        } else if self.typed_array(receiver)
            && matches!(
                self.key_domain(field(node, "index")?, depth),
                Some(Domain::Numeric | Domain::NumericString)
            )
        {
            return Some(Domain::Benign); // A missing index can yield undefined.
        }
        self.finite_member(node, receiver, depth)
    }

    pub(super) fn native_callback_position(&self, site: &AstSpan) -> bool {
        let Some(param) = self.node_at(site) else {
            return false;
        };
        let Some(parent) = param.parent() else {
            return false;
        };
        let parameter = if parent.kind() == "required_parameter"
            && field(parent, "pattern") == Some(param)
            && field(parent, "value").is_none()
        {
            parent
        } else {
            param
        };
        let Some(params) = parameter
            .parent()
            .filter(|n| n.kind() == "formal_parameters")
        else {
            return false;
        };
        let Some(callback) = params.parent() else {
            return false;
        };
        if children(params).nth(1).map(peel) != Some(parameter) {
            return false;
        }
        inline_iteration(self.file, callback).is_some_and(|call| {
            self.method(call, "forEach")
                .is_some_and(|receiver| self.native_array(receiver))
        }) || is_inline_callback(callback)
            && self.expression_use(callback).is_some_and(|arguments| {
                arguments.kind() == "arguments"
                    && arguments.parent().is_some_and(|call| {
                        self.prototype_call(call, "Array", "forEach")
                            && children(arguments)
                                .next()
                                .is_some_and(|first| first.kind() != "spread_element")
                            && children(arguments).nth(1).map(peel) == Some(callback)
                    })
            })
    }

    pub(super) fn iteration_domain(&self, site: &AstSpan, source: Node<'a>) -> Option<Domain> {
        let binding = self.node_at(site)?;
        let loop_node = self.enclosing(binding, |kind| {
            kind == "for_in_statement" || crate::ast::is_js_ts_function_like(kind)
        })?;
        if field(loop_node, "right") != Some(source) || binding.end_byte() > source.start_byte() {
            return None;
        }
        let source = peel(source);
        let token = self.text(field(loop_node, "operator")?);
        if self.primitive_string(source)
            && self.trusted_intrinsic("String")
            && self.trusted_intrinsic("Object")
        {
            return Some(Domain::String);
        }
        if token != "of" {
            return None;
        }
        if self.typed_array(source) {
            return Some(Domain::Numeric);
        }
        if self.static_call(source, "Object", "getOwnPropertySymbols") {
            return Some(Domain::Benign);
        }
        if self.static_call(source, "Object", "keys")
            && self
                .args::<1>(source)
                .is_some_and(|[array]| self.typed_array(array))
        {
            return Some(Domain::String);
        }
        if (self
            .method(source, "entries")
            .is_some_and(|array| self.native_array(array))
            && self.args::<0>(source).is_some()
            || self.prototype_call(source, "Array", "entries"))
            && first_array_binding(binding)
        {
            return Some(Domain::Numeric);
        }
        if self.static_call(source, "Object", "entries") && first_array_binding(binding) {
            let [groups] = self.args(source)?;
            let groups = peel(groups);
            if self.text(field(groups, "property")?) != "groups" {
                return None;
            }
            let regex = self.regexp_result(field(groups, "object")?)?;
            return self.benign_capture_names(regex).then_some(Domain::String);
        }
        // Own-key iteration of opaque receivers supplies no finite domain.
        None
    }

    fn primitive_string(&self, node: Node<'a>) -> bool {
        if !self.proof_charge(1) {
            return false;
        }
        let node = peel(node);
        let init = self.initializer(node).unwrap_or(node);
        matches!(init.kind(), "string" | "template_string")
            || (init.kind() == "call_expression"
                && field(init, "function").is_some_and(|callee| self.intrinsic(callee, "String")))
            || self.type_test_dominates(node, "string")
    }

    fn native_array(&self, receiver: Node<'a>) -> bool {
        if !self.trusted_intrinsic("Array") {
            return false;
        }
        self.closed_native_value(
            receiver,
            &["forEach", "entries", "indexOf", "length"],
            |value| value.kind() == "array" || self.static_call(value, "Array", "from"),
        )
        .is_some()
    }

    fn typed_array(&self, receiver: Node<'a>) -> bool {
        self.closed_native_value(receiver, &["length", "[]"], |value| {
            let constructor = if value.kind() == "new_expression" {
                field(value, "constructor")
            } else {
                self.method(value, "from")
                    .or_else(|| self.method(value, "of"))
            };
            constructor.is_some_and(|name| {
                [
                    "Int8Array",
                    "Uint8Array",
                    "Uint8ClampedArray",
                    "Int16Array",
                    "Uint16Array",
                    "Int32Array",
                    "Uint32Array",
                    "Float16Array",
                    "Float32Array",
                    "Float64Array",
                    "BigInt64Array",
                    "BigUint64Array",
                ]
                .iter()
                .any(|intrinsic| self.intrinsic(name, intrinsic))
            })
        })
        .is_some()
    }

    fn regexp_result(&self, receiver: Node<'a>) -> Option<Node<'a>> {
        if !self.trusted_intrinsic("RegExp") {
            return None;
        }
        let result = self.closed_native_value(receiver, &["index", "groups"], |value| {
            self.method(value, "exec")
                .is_some_and(|regex| regex.kind() == "regex")
        })?;
        self.method(result, "exec")
    }

    pub(super) fn closed_native_value(
        &self,
        node: Node<'a>,
        members: &[&str],
        accepts: impl Fn(Node<'a>) -> bool,
    ) -> Option<Node<'a>> {
        if !self.proof_charge(1) {
            return None;
        }
        let node = peel(node);
        if !identifier(node) {
            return accepts(node).then_some(node);
        }
        let binding = self.analysis.read(node)?.binding.as_ref()?;
        let value = self.initializer(node)?;
        if !accepts(value) || !self.analysis.unmutated_receiver(node) {
            return None;
        }
        for reference in self.binding_references(binding)? {
            if !self.proof_charge(1) {
                return None;
            }
            let parent = self.expression_use(reference)?;
            if matches!(parent.kind(), "member_expression" | "subscript_expression")
                && field(parent, "object").map(peel) == Some(reference)
            {
                let known_named = field(parent, "property")
                    .is_some_and(|name| members.contains(&self.text(name)));
                let numeric_load = members.contains(&"[]")
                    && field(parent, "index").is_some_and(|key| peel(key).kind() == "number");
                if !members.is_empty() && !known_named && !numeric_load {
                    return None;
                }
                if self.is_modified(parent)
                    || field(parent, "property")
                        .is_some_and(|property| self.text(property) == "groups")
                        && !self.expression_use(parent).is_some_and(|use_node| {
                            use_node.kind() == "arguments"
                                && use_node
                                    .parent()
                                    .is_some_and(|call| self.static_call(call, "Object", "entries"))
                        })
                {
                    return None;
                }
            } else if !self.observes_value(parent, reference) {
                return None;
            }
        }
        Some(value)
    }

    pub(in super::super) fn is_modified(&self, node: Node<'a>) -> bool {
        let Some(parent) = self.expression_use(node) else {
            return true;
        };
        let in_field = |name| field(parent, name).map(peel) == Some(peel(node));
        match parent.kind() {
            "assignment_expression" | "augmented_assignment_expression" | "for_in_statement" => {
                in_field("left")
            }
            "update_expression" => true,
            "unary_expression" => operator(parent, self.file) == "delete",
            "array_pattern" | "object_pattern" | "assignment_pattern" | "pair_pattern"
            | "rest_pattern" => true,
            _ => false,
        }
    }

    fn observes_value(&self, parent: Node<'a>, reference: Node<'a>) -> bool {
        match parent.kind() {
            "for_in_statement" => field(parent, "right").map(peel) == Some(reference),
            "if_statement" => field(parent, "condition").map(peel) == Some(reference),
            "unary_expression" => matches!(operator(parent, self.file), "!" | "typeof"),
            "binary_expression" => {
                matches!(operator(parent, self.file), "==" | "!=" | "===" | "!==")
            }
            "arguments" => parent.parent().is_some_and(|call| {
                self.static_call(call, "Object", "keys")
                    || self.static_call(call, "Array", "isArray")
            }),
            _ => false,
        }
    }

    fn benign_capture_names(&self, regex: Node<'a>) -> bool {
        let Some(pattern) = field(regex, "pattern") else {
            return false;
        };
        let text = self.text(pattern);
        if !self.proof_charge(text.len()) {
            return false;
        }
        let mut bracket = false;
        let mut escaped = false;
        let mut found = false;
        for (offset, byte) in text.bytes().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }
            match byte {
                b'\\' => escaped = true,
                b'[' => bracket = true,
                b']' => bracket = false,
                b'(' if !bracket && text[offset..].starts_with("(?<") => {
                    let rest = &text[offset + 3..];
                    if rest.starts_with(['=', '!']) {
                        continue;
                    }
                    let Some((name, _)) = rest.split_once('>') else {
                        return false;
                    };
                    if name.is_empty()
                        || !name
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                        || SHARED_LINKS.contains(&name)
                    {
                        return false;
                    }
                    found = true;
                }
                _ => {}
            }
        }
        found
    }

    pub(super) fn integer_test_dominates(&self, key: Node<'a>) -> bool {
        self.dominating_test(key, |test, succeeds| {
            succeeds
                && self.static_call(test, "Number", "isInteger")
                && self
                    .args::<1>(test)
                    .is_some_and(|[arg]| self.same_tested_value(peel(arg), key))
        }) || self.type_test_dominates(key, "number")
    }

    pub(super) fn type_test_dominates(&self, key: Node<'a>, kind: &str) -> bool {
        if !identifier(peel(key)) {
            return false;
        }
        self.dominating_test(peel(key), |test, succeeds| {
            let op = operator(test, self.file);
            if !matches!((op, succeeds), ("===", true) | ("!==", false)) {
                return false;
            }
            let Some(value) = field(test, "left").map(peel) else {
                return false;
            };
            value.kind() == "unary_expression"
                && operator(value, self.file) == "typeof"
                && field(test, "right").and_then(|rhs| self.plain_string(rhs)) == Some(kind)
                && field(value, "argument")
                    .is_some_and(|arg| self.same_tested_value(peel(arg), key))
        })
    }

    fn same_tested_value(&self, guard: Node<'a>, key: Node<'a>) -> bool {
        [guard, key].into_iter().all(|node| {
            self.analysis
                .read(node)
                .is_some_and(|read| read.resolution_complete)
        }) && self.same_value(guard, key)
    }

    fn dominating_test(
        &self,
        mut node: Node<'a>,
        establishes: impl Fn(Node<'a>, bool) -> bool,
    ) -> bool {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return false;
            }
            let Some(parent) = node.parent() else {
                return false;
            };
            if is_owner(parent) {
                return false;
            }
            if parent.kind() == "if_statement"
                && field(parent, "consequence") == Some(node)
                && field(parent, "condition")
                    .is_some_and(|test| self.test_branch(test, true, &establishes, 0))
            {
                return true;
            }
            if parent.kind() == "statement_block" {
                let mut previous = node.prev_named_sibling();
                while let Some(statement) = previous {
                    if !self.proof_charge(1) {
                        return false;
                    }
                    if statement.kind() == "if_statement"
                        && field(statement, "consequence").is_some_and(immediate_exit)
                        && field(statement, "condition")
                            .is_some_and(|test| self.test_branch(test, false, &establishes, 0))
                    {
                        return true;
                    }
                    previous = statement.prev_named_sibling();
                }
            }
            node = parent;
        }
        false
    }

    fn test_branch(
        &self,
        node: Node<'a>,
        succeeds: bool,
        establishes: &impl Fn(Node<'a>, bool) -> bool,
        depth: usize,
    ) -> bool {
        if depth == 32 || !self.proof_charge(1) {
            return false;
        }
        let node = peel(node);
        if node.kind() == "unary_expression" && operator(node, self.file) == "!" {
            return field(node, "argument")
                .is_some_and(|test| self.test_branch(test, !succeeds, establishes, depth + 1));
        }
        if node.kind() == "binary_expression"
            && matches!(
                (operator(node, self.file), succeeds),
                ("&&", true) | ("||", false)
            )
        {
            return ["left", "right"]
                .into_iter()
                .filter_map(|name| field(node, name))
                .any(|child| self.test_branch(child, succeeds, establishes, depth + 1));
        }
        establishes(node, succeeds)
    }
}

fn first_array_binding(binding: Node<'_>) -> bool {
    binding.parent().is_some_and(|pattern| {
        pattern.kind() == "array_pattern"
            && super::super::super::array_positions(pattern).next() == Some((0, binding))
    })
}

fn immediate_exit(node: Node<'_>) -> bool {
    let node = if node.kind() == "statement_block" {
        children(node).next().unwrap_or(node)
    } else {
        node
    };
    matches!(node.kind(), "return_statement" | "throw_statement")
        || matches!(node.kind(), "continue_statement" | "break_statement")
            && children(node).next().is_none()
}
