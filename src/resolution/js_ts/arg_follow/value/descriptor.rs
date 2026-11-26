use super::*;

#[derive(Clone, Copy)]
enum Own<'a> {
    Absent,
    Data(Value<'a>),
    Accessor { getter: bool },
    Opaque,
}

#[derive(Clone, Copy)]
enum Prototype<'a> {
    Ordinary,
    Null,
    Value(Value<'a>),
}

struct ObjectRead<'a, 'path> {
    key: &'path str,
    remaining: &'path [Selection<'a>],
    observation: Node<'a>,
    deferred: bool,
}

const OBJECT_PROTOTYPE: &[&str] = &[
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

impl<'a> Evaluation<'a, '_> {
    pub(super) fn select_object(
        &self,
        value: Value<'a>,
        key: &str,
        remaining: &[Selection<'a>],
        deferred: bool,
    ) -> Arm<'a> {
        match value {
            Value::Props(file, node) => self.jsx_attribute(file, node, key),
            Value::Node(file, node) if node.kind() == "object" => {
                let observation =
                    if file == self.caller && self.node.byte_range().contains(&node.start_byte()) {
                        self.node
                    } else {
                        node
                    };
                self.read_object(
                    file,
                    node,
                    &ObjectRead {
                        key,
                        remaining,
                        observation,
                        deferred,
                    },
                    &mut BTreeSet::new(),
                )
            }
            Value::Node(file, node) if node.kind() == "array" => {
                if let Ok(index) = key.parse::<usize>() {
                    if index.to_string() == key {
                        let entries = param_slot::array_entries(node);
                        if entries
                            .iter()
                            .flatten()
                            .any(|entry| entry.kind() == "spread_element")
                        {
                            return Err("path_step");
                        }
                        if let Some(Some(entry)) = entries.get(index) {
                            return Ok(Value::Node(file, *entry));
                        }
                    }
                }
                Err("undefined_lookup")
            }
            _ => Err("path_step"),
        }
    }

    fn read_object(
        &self,
        file: &'a str,
        node: Node<'a>,
        read: &ObjectRead<'a, '_>,
        visited: &mut BTreeSet<(&'a str, usize)>,
    ) -> Arm<'a> {
        if !visited.insert((file, node.start_byte())) {
            return Err("undefined_lookup");
        }
        let (own, prototype) = self.own_descriptor(file, node, read.key);
        match own {
            Own::Data(value) => Ok(value),
            Own::Accessor { getter: false } => Ok(Value::Undefined),
            Own::Accessor { getter: true } => Err("undefined_lookup"),
            Own::Opaque => Err("hidden_literal_key"),
            Own::Absent => match prototype {
                Prototype::Ordinary if OBJECT_PROTOTYPE.contains(&read.key) => {
                    Ok(Value::NonCallable)
                }
                Prototype::Ordinary | Prototype::Null => Ok(Value::Undefined),
                Prototype::Value(Value::Node(file, node)) => {
                    let value = self.static_object(file, node, read)?;
                    match value.kind() {
                        "null" => Ok(Value::Undefined),
                        // The caller-side stability proof ends at invocation.
                        "object" | "array" if read.deferred => Err("stale_path"),
                        "object" => self.read_object(file, value, read, visited),
                        "array" => self.select_object(
                            Value::Node(file, value),
                            read.key,
                            read.remaining,
                            read.deferred,
                        ),
                        _ => Err("undefined_lookup"),
                    }
                }
                Prototype::Value(_) => Err("undefined_lookup"),
            },
        }
    }

    fn own_descriptor(&self, file: &'a str, node: Node<'a>, key: &str) -> (Own<'a>, Prototype<'a>) {
        let parsed = &self.context.files[file];
        let mut own = Own::Absent;
        let mut prototype = Prototype::Ordinary;
        for entry in param_slot::children(node) {
            match entry.kind() {
                "spread_element" => {
                    let Some(operand) = param_slot::children(entry)
                        .next()
                        .and_then(param_slot::peel)
                    else {
                        own = Own::Opaque;
                        continue;
                    };
                    match operand.kind() {
                        "object" => match self.own_descriptor(file, operand, key).0 {
                            Own::Absent => {}
                            Own::Data(data) => own = Own::Data(data),
                            Own::Accessor { getter: false } => own = Own::Data(Value::Undefined),
                            Own::Accessor { getter: true } | Own::Opaque => own = Own::Opaque,
                        },
                        "undefined" | "null" | "false" | "true" | "number" => {}
                        _ => own = Own::Opaque,
                    }
                }
                "pair" => {
                    let name = entry.child_by_field_name("key");
                    let value = entry
                        .child_by_field_name("value")
                        .map(|node| Value::Node(file, node));
                    let Some((name, spelling)) = name.and_then(|name| {
                        param_slot::property_key(parsed, name).map(|spelling| (name, spelling))
                    }) else {
                        own = Own::Opaque;
                        continue;
                    };
                    if spelling == "__proto__" && name.kind() != "computed_property_name" {
                        prototype = match value.and_then(|v| self.static_literal_kind(v)) {
                            Some("null") => Prototype::Null,
                            Some("undefined" | "number" | "string" | "true" | "false") => {
                                Prototype::Ordinary
                            }
                            _ => value.map_or(Prototype::Ordinary, Prototype::Value),
                        };
                    } else if spelling == key {
                        own = value.map_or(Own::Opaque, Own::Data);
                    }
                }
                "method_definition" => {
                    let name = entry.child_by_field_name("name");
                    match name
                        .and_then(|name| param_slot::property_key(parsed, name))
                        .as_deref()
                    {
                        None => own = Own::Opaque,
                        Some(spelling) if spelling == key => {
                            let mut cursor = entry.walk();
                            let accessor = entry
                                .children(&mut cursor)
                                .take_while(|child| Some(*child) != name)
                                .find(|child| matches!(child.kind(), "get" | "set"));
                            own = match accessor.map(|child| child.kind()) {
                                Some("get") => Own::Accessor { getter: true },
                                Some("set") => match own {
                                    Own::Opaque => Own::Opaque,
                                    Own::Accessor { getter: true } => own,
                                    _ => Own::Accessor { getter: false },
                                },
                                _ => Own::Data(Value::Node(file, entry)),
                            };
                        }
                        _ => {}
                    }
                }
                "shorthand_property_identifier" => {
                    if param_slot::property_key(parsed, entry).as_deref() == Some(key) {
                        own = Own::Data(Value::Node(file, entry));
                    }
                }
                _ => own = Own::Opaque,
            }
        }
        (own, prototype)
    }

    fn static_literal_kind(&self, value: Value<'a>) -> Option<&'static str> {
        let Value::Node(file, node) = value else {
            return None;
        };
        let node = param_slot::peel(node)?;
        if node.kind() == "undefined" {
            let mut cache = self.context.bindings[file].cache();
            if !matches!(
                self.context.files[file].js_ts_binding_walk(node, "undefined", &mut cache),
                Walk::Unbound
            ) {
                return None;
            }
        }
        Some(node.kind())
    }

    fn static_object(
        &self,
        file: &'a str,
        node: Node<'a>,
        read: &ObjectRead<'a, '_>,
    ) -> Result<Node<'a>, &'static str> {
        let parsed = &self.context.files[file];
        let node = param_slot::peel(node).ok_or("undefined_lookup")?;
        if matches!(node.kind(), "object" | "array" | "null") {
            return Ok(node);
        }
        let name = param_slot::name(parsed, node).ok_or("undefined_lookup")?;
        let bindings = &self.context.bindings[file];
        let mut cache = bindings.cache();
        let (scope, declaration) = parsed
            .js_ts_site_declaration(node, &name, &mut cache)
            .ok_or("undefined_lookup")?;
        if !param_slot::is_const(declaration)
            || !initialized_at(parsed, declaration, node)
            || parsed.js_ts_scoped_written(scope, &name, &mut cache)
        {
            return Err("undefined_lookup");
        }
        let value = declaration
            .child_by_field_name("value")
            .and_then(param_slot::peel)
            .filter(|value| matches!(value.kind(), "object" | "array" | "null"))
            .ok_or("undefined_lookup")?;
        let selected: Vec<_> = std::iter::once(Some(read.key.to_owned()))
            .chain(stability::selected_keys(read.remaining))
            .collect();
        let path = stability::PathRead {
            source: node,
            scope,
            observation: read.observation,
            name: &name,
            selected: &selected,
        };
        let exported = declaration
            .parent()
            .and_then(|list| list.parent())
            .is_some_and(|parent| parent.kind() == "export_statement");
        if value.kind() != "null"
            && (exported
                || self.context.dynamic_for(file, scope, &name, &mut cache)
                || stability::path_unstable(
                    parsed,
                    bindings.references(parsed),
                    path,
                    &|usage, skip| stability::plain_access(parsed, bindings, value, usage, skip),
                    &mut cache,
                ))
        {
            return Err("stale_path");
        }
        Ok(value)
    }

    fn jsx_attribute(&self, file: &'a str, node: Node<'a>, key: &str) -> Arm<'a> {
        let parsed = &self.context.files[file];
        // The JSX syntax alone does not prove whether the lowering delivers
        // reserved attributes in props.
        if matches!(key, "key" | "ref") || key == "children" && node.kind() == "jsx_opening_element"
        {
            return Err("jsx_attribute");
        }
        let mut found = Ok(Value::Undefined);
        for part in param_slot::children(node) {
            match part.kind() {
                "jsx_expression"
                    if param_slot::children(part)
                        .next()
                        .is_some_and(|expr| expr.kind() == "spread_element") =>
                {
                    found = Err("jsx_attribute");
                }
                "jsx_attribute" => {
                    let mut fields = param_slot::children(part);
                    if fields
                        .next()
                        .is_none_or(|field| parsed.node_text(&field) != key)
                    {
                        continue;
                    }
                    found = match fields.next() {
                        None => Ok(Value::NonCallable),
                        Some(expression) if expression.kind() == "jsx_expression" => {
                            param_slot::children(expression)
                                .next()
                                .map(|value| Value::Node(file, value))
                                .ok_or("jsx_attribute")
                        }
                        Some(_) => Ok(Value::NonCallable),
                    };
                }
                _ => {}
            }
        }
        found
    }
}
