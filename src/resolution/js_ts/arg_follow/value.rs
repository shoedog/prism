//! Per-caller argument values and declaration-local default evaluation.

use super::identity::initialized_at;
use super::*;
use param_slot::Walk;

mod descriptor;

#[derive(Clone, Copy, Debug)]
pub(super) enum Value<'a> {
    Node(&'a str, Node<'a>),
    Props(&'a str, Node<'a>),
    Undefined,
    Missing,
    NonCallable,
}

type Arm<'a> = Result<Value<'a>, &'static str>;

struct Evaluation<'a, 'evaluation> {
    context: &'evaluation Context<'a>,
    holder: &'a str,
    caller: &'a str,
    call: &'a CallSite,
    node: Node<'a>,
}

impl<'a> Context<'a> {
    pub(super) fn values_of(
        &self,
        holder: &'a str,
        parameter: &Parameter<'a>,
        parsed: &'a ParsedFile,
        call: &'a CallSite,
        node: Node<'a>,
    ) -> Vec<Arm<'a>> {
        debug_assert!(std::ptr::eq(parsed, &self.files[&call.caller.file]));
        let evaluation = Evaluation {
            context: self,
            holder,
            caller: &call.caller.file,
            call,
            node,
        };
        evaluation.parameter(parameter, None, &mut BTreeSet::new())
    }

    fn stability(
        &self,
        file: &str,
        parameter: &Parameter<'a>,
        entry: Option<Node<'a>>,
    ) -> Option<&'static str> {
        if parameter.replaced {
            return Some("entry_write");
        }
        let parsed = &self.files[file];
        let mut cache = self.bindings[file].cache();
        let name = param_slot::name(parsed, parameter.binding)?;
        let written = match entry {
            Some(formals) => parsed.js_ts_formal_written(formals, &name, &mut cache),
            None => parsed.js_ts_scoped_written(parameter.scope, &name, &mut cache),
        };
        if written {
            return Some("entry_write");
        }
        if self.dynamic_for(file, entry.unwrap_or(parameter.scope), &name, &mut cache) {
            return Some("dynamic_binding");
        }
        if let Some(read) = parameter.body_read {
            let Some(source) = param_slot::name(parsed, read) else {
                return Some("stale_path");
            };
            let Walk::Found(scope, None) = parsed.js_ts_binding_walk(read, &source, &mut cache)
            else {
                return Some("stale_path");
            };
            if self.dynamic_for(file, scope, &source, &mut cache) {
                return Some("dynamic_binding");
            }
        }
        None
    }
}

impl<'a> Evaluation<'a, '_> {
    fn parameter(
        &self,
        parameter: &Parameter<'a>,
        entry: Option<Node<'a>>,
        active_defaults: &mut BTreeSet<usize>,
    ) -> Vec<Arm<'a>> {
        if let Some(reason) = self.context.stability(self.holder, parameter, entry) {
            return vec![Err(reason)];
        }
        let actual = self.actual(parameter.position);
        let plain_access =
            |usage: &stability::PathUse<'a>, skip| self.plain_access(actual, usage, skip);
        if matches!(actual, Ok(Value::Node(..) | Value::Props(..))) {
            let parsed = &self.context.files[self.holder];
            let bindings = &self.context.bindings[self.holder];
            if let Some(reason) = stability::arguments_instability(
                parsed,
                bindings,
                parameter.owner,
                parameter.position,
                parameter
                    .body_read
                    .map(|read| (read, parameter.selected.as_slice())),
                &plain_access,
                &mut bindings.cache(),
            ) {
                return vec![Err(reason)];
            }
        }
        let mut deferred = false;
        if let Some(read) = parameter.body_read {
            if !self.body_path_stable(parameter, read, &plain_access) {
                return vec![Err("stale_path")];
            }
            let parsed = &self.context.files[self.holder];
            deferred = self.context.bindings[self.holder]
                .references(parsed)
                .deferred_read(parsed, parameter.owner, read);
        }
        let mut values = self.expand_result(actual.map(|value| {
            if matches!(value, Value::Missing) {
                Value::Undefined
            } else {
                value
            }
        }));
        for (index, step) in parameter.selected.iter().enumerate() {
            let remaining = &parameter.selected[index + 1..];
            values = values
                .into_iter()
                .flat_map(|value| match value {
                    Err(reason) => vec![Err(reason)],
                    Ok(value) => match step {
                        Selection::Default(default) if matches!(value, Value::Undefined) => {
                            self.default(*default, parameter.owner, active_defaults)
                        }
                        Selection::Default(_) => vec![Ok(value)],
                        Selection::Array(position) => self.select_array(value, *position),
                        Selection::Object(key) => {
                            self.expand_result(self.select_object(value, key, remaining, deferred))
                        }
                        Selection::Computed(key) => match self.computed_key(*key, parameter) {
                            Ok(key) => self.expand_result(
                                self.select_object(value, &key, remaining, deferred),
                            ),
                            Err(reason) => vec![Err(reason)],
                        },
                        Selection::Rest => vec![Err("path_step")],
                    },
                })
                .collect();
        }
        values
    }

    fn body_path_stable(
        &self,
        parameter: &Parameter<'a>,
        source: Node<'a>,
        plain_access: &stability::DisjointProof<'a, '_>,
    ) -> bool {
        let parsed = &self.context.files[self.holder];
        let bindings = &self.context.bindings[self.holder];
        let mut cache = bindings.cache();
        let Some(name) = param_slot::name(parsed, source) else {
            return false;
        };
        let Walk::Found(scope, None) = parsed.js_ts_binding_walk(source, &name, &mut cache) else {
            return false;
        };
        let selected: Vec<_> = stability::selected_keys(&parameter.selected).collect();
        !stability::path_unstable(
            parsed,
            bindings.references(parsed),
            stability::PathRead {
                source,
                scope,
                observation: source,
                name: &name,
                selected: &selected,
            },
            plain_access,
            &mut cache,
        )
    }

    fn plain_access(&self, value: Arm<'a>, usage: &stability::PathUse<'a>, skip: usize) -> bool {
        let Ok(Value::Node(file, root)) = value else {
            return false;
        };
        stability::plain_access(
            &self.context.files[file],
            &self.context.bindings[file],
            root,
            usage,
            skip,
        )
    }

    fn computed_key(
        &self,
        key: Node<'a>,
        selected: &Parameter<'a>,
    ) -> Result<String, &'static str> {
        let parsed = &self.context.files[self.holder];
        let bindings = &self.context.bindings[self.holder];
        let mut cache = bindings.cache();
        let occurrence = param_slot::children(key)
            .next()
            .and_then(param_slot::peel)
            .ok_or("path_step")?;
        let name = param_slot::name(parsed, occurrence).ok_or("path_step")?;
        let parameter = param_slot::parameter(parsed, occurrence, &mut cache).ok_or("path_step")?;
        if parameter.owner != selected.owner
            || parameter.binding.end_byte() > occurrence.start_byte()
            || !parameter.selected.is_empty()
        {
            return Err("path_step");
        }
        if parameter.replaced || parsed.js_ts_scoped_written(parameter.scope, &name, &mut cache) {
            return Err("entry_write");
        }
        if self
            .context
            .dynamic_for(self.holder, parameter.scope, &name, &mut cache)
        {
            return Err("dynamic_binding");
        }
        let Value::Node(file, argument) = self.actual(parameter.position)? else {
            return Err("path_step");
        };
        if let Some(reason) = stability::arguments_instability(
            parsed,
            bindings,
            parameter.owner,
            parameter.position,
            None,
            &|_, _| false,
            &mut cache,
        ) {
            return Err(reason);
        }
        let argument = param_slot::peel(argument).ok_or("path_step")?;
        matches!(argument.kind(), "number" | "string")
            .then(|| param_slot::property_key(&self.context.files[file], argument))
            .flatten()
            .ok_or("path_step")
    }

    fn default(
        &self,
        node: Node<'a>,
        owner: Node<'a>,
        active_defaults: &mut BTreeSet<usize>,
    ) -> Vec<Arm<'a>> {
        let mut values = Vec::new();
        for arm in self.expand(Value::Node(self.holder, node)) {
            match arm {
                Ok(Value::Node(file, selected)) if file == self.holder => {
                    let resolved = self.default_binding(selected, owner, active_defaults);
                    if resolved
                        .iter()
                        .any(|value| matches!(value, Err("stale_terminal")))
                    {
                        // An unproved binding read can invalidate evaluation
                        // of the whole initializer.
                        return vec![Err("stale_terminal")];
                    }
                    values.extend(resolved);
                }
                _ => values.push(arm),
            }
        }
        values
    }

    fn default_binding(
        &self,
        node: Node<'a>,
        owner: Node<'a>,
        active_defaults: &mut BTreeSet<usize>,
    ) -> Vec<Arm<'a>> {
        let parsed = &self.context.files[self.holder];
        let parameter = param_slot::parameter(
            parsed,
            node,
            &mut self.context.bindings[self.holder].cache(),
        );
        let Some(parameter) = parameter else {
            return vec![Ok(Value::Node(self.holder, node))];
        };
        if parameter.owner != owner
            || !initialized_before(&parameter, node)
            || (parameter.body_read.is_some() && parameter.replaced)
            || !active_defaults.insert(parameter.binding.id())
        {
            return vec![Err("stale_terminal")];
        }
        let entry = owner
            .child_by_field_name("parameters")
            .filter(|list| list.byte_range().contains(&node.start_byte()));
        let values = self.parameter(&parameter, entry, active_defaults);
        active_defaults.remove(&parameter.binding.id());
        values
    }

    fn actual(&self, position: usize) -> Arm<'a> {
        if self.call.jsx_element {
            return Ok(if position == 0 {
                Value::Props(self.caller, self.node)
            } else {
                Value::Missing
            });
        }
        let parsed = &self.context.files[self.caller];
        let Some(list) = self.node.child_by_field_name("arguments") else {
            return if self.node.kind() == "new_expression" {
                Ok(Value::Missing)
            } else {
                Err("source")
            };
        };
        if list.kind() != "arguments" || list.has_error() {
            return Err("source");
        }
        let arguments = param_slot::children(list);
        #[cfg(test)]
        let arguments = if let Some(supplied) = self
            .context
            .supplied_call(self.call)
            .and_then(|call| call.arguments)
        {
            if !arguments.eq(supplied.iter().map(|(node, _)| *node))
                || supplied
                    .iter()
                    .any(|(node, span)| node.byte_range() != *span)
            {
                return Err("source");
            }
            supplied
                .iter()
                .map(|(node, _)| *node)
                .collect::<Vec<_>>()
                .into_iter()
        } else {
            arguments.collect::<Vec<_>>().into_iter()
        };
        let mut offset = 0;
        for argument in arguments {
            if argument.kind() != "spread_element" {
                if offset == position {
                    return Ok(Value::Node(self.caller, argument));
                }
                offset += 1;
                continue;
            }
            let operand = parsed
                .spread_argument_operand(&argument.byte_range())
                .ok_or("unknown_spread")?;
            let length = crate::cpg::dense_array_length(operand).ok_or("unknown_spread")?;
            if position < offset + length {
                return param_slot::children(operand)
                    .nth(position - offset)
                    .map(|node| Value::Node(self.caller, node))
                    .ok_or("unknown_spread");
            }
            offset += length;
        }
        Ok(Value::Missing)
    }

    fn expand_result(&self, value: Arm<'a>) -> Vec<Arm<'a>> {
        value.map_or_else(|reason| vec![Err(reason)], |value| self.expand(value))
    }

    fn expand(&self, value: Value<'a>) -> Vec<Arm<'a>> {
        let Value::Node(file, node) = value else {
            return vec![Ok(value)];
        };
        let Some(node) = param_slot::peel(node) else {
            return vec![Err("opaque_terminal")];
        };
        let parsed = &self.context.files[file];
        let expand_child = |field| {
            node.child_by_field_name(field).map_or_else(
                || vec![Err("opaque_terminal")],
                |child| self.expand(Value::Node(file, child)),
            )
        };
        match node.kind() {
            "undefined" | "identifier"
                if param_slot::name(parsed, node).as_deref() == Some("undefined") =>
            {
                let mut cache = self.context.bindings[file].cache();
                if matches!(
                    parsed.js_ts_binding_walk(node, "undefined", &mut cache),
                    Walk::Unbound
                ) {
                    return vec![Ok(Value::Undefined)];
                }
                vec![Ok(Value::Node(file, node))]
            }
            "unary_expression"
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| operator.kind() == "void") =>
            {
                vec![Ok(Value::Undefined)]
            }
            "ternary_expression" => {
                let mut values = expand_child("consequence");
                values.extend(expand_child("alternative"));
                values
            }
            "binary_expression" => {
                let operator = node
                    .child_by_field_name("operator")
                    .map(|operator| operator.kind());
                match operator {
                    Some("||" | "??" | "&&") => {
                        let left = expand_child("left");
                        let mut values = Vec::new();
                        let mut take_right = false;
                        for arm in left {
                            let truth = truthiness(arm.as_ref().ok().copied());
                            let nullish = match arm.as_ref().ok() {
                                Some(Value::Undefined) => Some(true),
                                Some(Value::Node(_, node)) if node.kind() == "null" => Some(true),
                                Some(Value::Node(_, node))
                                    if matches!(
                                        node.kind(),
                                        "identifier"
                                            | "member_expression"
                                            | "subscript_expression"
                                            | "call_expression"
                                    ) =>
                                {
                                    None
                                }
                                Some(_) => Some(false),
                                None => None,
                            };
                            match operator {
                                Some("&&") => {
                                    take_right |= truth != Some(false);
                                    if truth != Some(true) {
                                        values.push(arm.and_then(|value| match value {
                                            Value::Undefined => Ok(value),
                                            _ => Err("opaque_terminal"),
                                        }));
                                    }
                                }
                                Some("||") => {
                                    take_right |= truth != Some(true);
                                    if truth != Some(false) {
                                        values.push(arm);
                                    }
                                }
                                _ => {
                                    take_right |= nullish != Some(false);
                                    if nullish != Some(true) {
                                        values.push(arm);
                                    }
                                }
                            }
                        }
                        if take_right {
                            values.extend(expand_child("right"));
                        }
                        values
                    }
                    _ => vec![Ok(Value::Node(file, node))],
                }
            }
            _ => vec![Ok(Value::Node(file, node))],
        }
    }

    fn select_array(&self, value: Value<'a>, index: usize) -> Vec<Arm<'a>> {
        let Value::Node(file, node) = value else {
            return vec![Err("path_step")];
        };
        if node.kind() != "array" {
            return vec![Err("path_step")];
        }
        let mut offset = 0;
        for entry in param_slot::array_entries(node) {
            if let Some(entry) = entry.filter(|node| node.kind() == "spread_element") {
                let parsed = &self.context.files[file];
                let Some(array) = parsed.spread_argument_operand(&entry.byte_range()) else {
                    return vec![Err("path_step")];
                };
                let Some(length) = crate::cpg::dense_array_length(array) else {
                    return vec![Err("path_step")];
                };
                if index < offset + length {
                    return self.expand_result(
                        param_slot::children(array)
                            .nth(index - offset)
                            .map(|node| Value::Node(file, node))
                            .ok_or("path_step"),
                    );
                }
                offset += length;
            } else {
                if index == offset {
                    return self
                        .expand(entry.map_or(Value::Undefined, |node| Value::Node(file, node)));
                }
                offset += 1;
            }
        }
        vec![Ok(Value::Undefined)]
    }
}

fn initialized_before(parameter: &Parameter<'_>, occurrence: Node<'_>) -> bool {
    let read = occurrence.start_byte();
    // Every default on the binding's path encloses it in a formal or pattern
    // element. Its initializer runs before that binding is initialized, even
    // when the binding's token precedes the read. An earlier sibling inside
    // the pattern is available after its own element finishes.
    parameter.binding.end_byte() <= read
        && !parameter.selected.iter().any(|step| {
            let Selection::Default(initializer) = step else {
                return false;
            };
            initializer.start_byte() <= read
                && initializer
                    .parent()
                    .is_some_and(|element| read < element.end_byte())
        })
}

fn truthiness(value: Option<Value<'_>>) -> Option<bool> {
    match value {
        Some(Value::Undefined | Value::Missing) => Some(false),
        Some(Value::Props(..)) => Some(true),
        Some(Value::Node(_, node)) => match node.kind() {
            "false" | "null" => Some(false),
            "true"
            | "object"
            | "array"
            | "arrow_function"
            | "function_expression"
            | "generator_function" => Some(true),
            _ => None,
        },
        Some(Value::NonCallable) | None => None,
    }
}
