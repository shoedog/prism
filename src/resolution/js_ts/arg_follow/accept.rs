//! Admit only baseline-grounded routes covered by the E1 accept-list.

use super::baseline::{Hop, Reason, Route};
use super::value::Value;
use super::*;
use crate::ast::JsBinding;
use param_slot::{children, name, peel, Walk};

mod evidence;
mod replay;
use evidence::*;
use replay::baseline_expr;
pub(super) use replay::Replay;

type Check<T> = Result<T, Reason>;
type Expr<'a> = (&'a str, Node<'a>);
type Span = (usize, usize);

const ORDER: &[Reason] = &[
    "source",
    "holder_outcome",
    "holder_identity",
    "no_caller",
    "jsx_attribute",
    "entry_write",
    "dynamic_binding",
    "mapped_arguments",
    "stale_path",
    "read_timing",
    "unknown_spread",
    "path_step",
    "default_dependency",
    "default_form",
    "hidden_literal_key",
    "undefined_lookup",
    "opaque_terminal",
    "stale_terminal",
    "callable_collision",
    "relay_form",
    "cycle",
    "depth",
    "fanout",
    "no_target",
];

pub(super) fn rank(reason: &str) -> usize {
    ORDER
        .iter()
        .position(|item| *item == reason)
        .unwrap_or(ORDER.len())
}

fn first(left: Reason, right: Reason) -> Reason {
    if rank(left) <= rank(right) {
        left
    } else {
        right
    }
}

pub(super) struct Proof<'a, 'context> {
    context: &'context Context<'a>,
}

#[derive(Clone, Copy)]
struct Read<'a> {
    call: Node<'a>,
    binding: Node<'a>,
    relay: bool,
}

pub(super) struct Decision<'a> {
    pub reason: Option<Reason>,
    pub recipes: Vec<Option<Replay<'a>>>,
}

impl<'a, 'context> Proof<'a, 'context> {
    pub(super) fn new(context: &'context Context<'a>) -> Self {
        Self { context }
    }

    pub(super) fn route(&self, candidate: &Candidate<'a>, route: &Route<'a>) -> Decision<'a> {
        let mut reason = route.terminal.as_ref().err().copied();
        let mut recipes = Vec::new();
        let mut refuse = |failure| {
            reason = Some(reason.map_or(failure, |prior| first(prior, failure)));
        };
        let parsed = &self.context.files[&candidate.site.caller.file];
        let mut read = call_node(parsed, candidate.site).and_then(|call| {
            invocation(call, candidate.site)
                .and_then(peel)
                .map(|binding| Read {
                    call,
                    binding,
                    relay: false,
                })
        });
        if read.is_none_or(|read| read.call.has_error()) || candidate.parameter.owner.has_error() {
            refuse("source");
        }
        if candidate.site.jsx_element || route.hops.iter().any(|hop| hop.call.jsx_element) {
            refuse("jsx_attribute");
        }
        for hop in &route.hops {
            recipes.push(None);
            let source = self.source(hop);
            let (callee, arguments) = match source {
                Ok(Some(call)) => call,
                Ok(None) => continue, // JSX delivery was refused above.
                Err(reason) => {
                    refuse(reason);
                    continue;
                }
            };
            if hop.ground_failure.is_some() {
                continue;
            }
            let Some(read_at) = read else { continue };
            let (recipe, failure) = self.hop(hop, read_at, callee, arguments);
            if let Err(reason) = failure {
                refuse(reason);
            }
            if let Some(parameter) = &hop.relay {
                // The next occurrence is the complete actual, proved against
                // the relay's exact formal and I2 before it may supply a target.
                match self.transfer(hop, parameter, &recipe) {
                    Ok((transfer, timing)) => {
                        read = Some(transfer);
                        if !timing {
                            refuse("read_timing");
                        }
                    }
                    Err(reason) => {
                        refuse(reason);
                        read = None;
                    }
                }
            }
            *recipes.last_mut().unwrap() = Some(recipe);
        }
        if let Some((hop, Some(recipe))) = route.hops.last().zip(recipes.last()) {
            if hop.relay.is_none() {
                if let Some(expr) = recipe.value {
                    match self.callable(expr, 0) {
                        Ok((actual, bytes, _)) => {
                            if route
                                .terminal
                                .as_ref()
                                .is_ok_and(|target| target != &(actual, bytes))
                            {
                                refuse("stale_terminal");
                            }
                        }
                        Err("dynamic_binding") => refuse("stale_terminal"),
                        Err(reason) => refuse(reason),
                    }
                }
            }
        }
        Decision { reason, recipes }
    }

    fn source(&self, hop: &Hop<'a>) -> Check<Option<(Node<'a>, Node<'a>)>> {
        let node = hop.node.filter(|node| !node.has_error()).ok_or("source")?;
        if !source(hop.call) || hop.parameter.owner.has_error() {
            return Err("source");
        }
        if hop.call.jsx_element {
            return invocation(node, hop.call).map(|_| None).ok_or("source");
        }
        let (callee, arguments) = ordinary_call(node).ok_or("source")?;
        let file = hop.call.caller.file.as_str();
        let parsed = &self.context.files[file];
        if matches!(callee.kind(), "member_expression" | "subscript_expression") {
            let (object, _) = static_member(parsed, callee).ok_or("source")?;
            if !matches!(
                parsed.js_ts_namespace_binding(object, &mut self.context.bindings[file].cache()),
                Some((JsBinding::Import, Some(_)))
            ) {
                return Err("source");
            }
        } else if callee.kind() == "super" {
            return Err("source");
        }
        Ok(Some((callee, arguments)))
    }

    fn hop(
        &self,
        hop: &Hop<'a>,
        read: Read<'a>,
        callee: Node<'a>,
        arguments: Node<'a>,
    ) -> (Replay<'a>, Check<()>) {
        let (file, parsed) = self
            .context
            .files
            .get_key_value(&hop.slot.holder.file)
            .expect("indexed holder file");
        let file = file.as_str();
        let p = &hop.parameter;
        let args: Vec<_> = children(arguments).collect();
        let copy = p
            .body_read
            .and_then(|source| copy_declaration(source, read.call));
        let mut recipe = Replay::new(hop.call.caller.file.as_str(), args.get(p.position).copied());
        let replay = self.replay(hop, &args, copy, &mut recipe);
        let i0 = !read.relay && scalar(p) && !recipe.tracked_default;
        let loose = i0 && hop.relay.is_none();
        recipe.timing = Some(if read.relay {
            "I2"
        } else if i0 {
            "I0"
        } else {
            "I1"
        });
        let supplied =
            recipe.actual.is_some() || args.iter().any(|arg| arg.kind() == "spread_element");
        let checked = (|| {
            let stability = self.stability(file, p, Some(read.binding), loose, supplied);
            let dynamic =
                self.source_dynamic(hop.call.caller.file.as_str(), hop.node.unwrap(), callee);
            match (stability, dynamic) {
                (Err(reason), true) => return Err(first(reason, "dynamic_binding")),
                (Err(reason), false) => return Err(reason),
                (Ok(()), true) => return Err("dynamic_binding"),
                (Ok(()), false) => {}
            }
            if i0 {
                if !p
                    .owner
                    .child_by_field_name("body")
                    .is_some_and(|body| body.byte_range().contains(&read.call.start_byte()))
                    && !recipe.defaults.iter().any(|&(binding, default)| {
                        self.deferred_default_read(
                            file, parsed, p.owner, binding, default, read.call,
                        )
                    })
                {
                    return Err("read_timing");
                }
            } else if !placement(p.owner, read.call)
                .is_some_and(|position| self.prefix(file, position, copy))
            {
                return Err("read_timing");
            }
            replay?;
            if let Some(value) = baseline_expr(hop) {
                if Some(value) != recipe.value {
                    return Err(if hop.relay.is_some() {
                        "relay_form"
                    } else {
                        "opaque_terminal"
                    });
                }
            }
            Ok(())
        })();
        (recipe, checked)
    }

    fn transfer(
        &self,
        hop: &Hop<'a>,
        p: &Parameter<'a>,
        recipe: &Replay<'a>,
    ) -> Check<(Read<'a>, bool)> {
        let file = hop.call.caller.file.as_str();
        let call = hop.node.ok_or("source")?;
        let (_, operands) = ordinary_call(call).ok_or("source")?;
        let binding = recipe.actual.and_then(peel).ok_or("relay_form")?;
        if !scalar(&hop.parameter)
            || !scalar(p)
            || recipe.tracked_default
            || Some((file, binding)) != baseline_expr(hop)
            || p.owner == hop.parameter.owner && hop.slot.holder.file == file
        {
            return Err("relay_form");
        }
        let timing = placement(p.owner, call)
            .is_some_and(|position| self.prefix(file, position, None))
            && children(operands).enumerate().all(|(index, expr)| {
                index == hop.parameter.position || self.closed((file, expr), 0).is_ok()
            });
        Ok((
            Read {
                call,
                binding,
                relay: true,
            },
            timing,
        ))
    }

    fn deferred_default_read(
        &self,
        file: &'a str,
        parsed: &'a ParsedFile,
        owner: Node<'a>,
        binding: Node<'a>,
        default: Node<'a>,
        read: Node<'a>,
    ) -> bool {
        let Some(closure) = peel(default) else {
            return false;
        };
        let Some(spelling) = name(parsed, binding) else {
            return false;
        };
        let Some(formals) = owner.child_by_field_name("parameters") else {
            return false;
        };
        if !matches!(
            closure.kind(),
            "arrow_function" | "function_expression" | "generator_function"
        ) || !closure.byte_range().contains(&read.start_byte())
            || evaluated_in(parsed, default, read)
        {
            return false;
        }
        let mut cache = self.context.bindings[file].cache();
        !descendants(formals).any(|node| {
            node != binding
                && name(parsed, node).as_deref() == Some(spelling.as_str())
                && !parsed.language.is_in_erased_type_context(node)
                && !parsed.js_ts_identifier_is_not_a_read(node)
                && !matches!(parsed.js_ts_binding_walk(node, &spelling, &mut cache),
                    Walk::Found(scope, None) if canonical_scope(scope) != owner)
        })
    }

    fn stability(
        &self,
        file: &'a str,
        p: &Parameter<'a>,
        read: Option<Node<'a>>,
        loose: bool,
        supplied: bool,
    ) -> Check<()> {
        let parsed = &self.context.files[file];
        let bindings = &self.context.bindings[file];
        let mut cache = bindings.cache();
        let spelling = name(parsed, p.binding).ok_or("stale_path")?;
        let mut tracked = vec![(p.binding, p.scope, spelling, read, true)];
        if let Some(carrier) = p.body_read {
            let source = param_slot::parameter(parsed, carrier, &mut cache).ok_or("stale_path")?;
            let spelling = name(parsed, source.binding).ok_or("stale_path")?;
            if source.replaced {
                return Err("entry_write");
            }
            tracked.push((source.binding, source.scope, spelling, Some(carrier), false));
        }
        if p.replaced
            || tracked
                .iter()
                .any(|(_, scope, name, _, _)| parsed.js_ts_scoped_written(*scope, name, &mut cache))
        {
            return Err("entry_write");
        }
        if self.dynamic(file, p.owner, &mut cache) {
            return Err("dynamic_binding");
        }

        let refs = bindings.references(parsed);
        let mapped = supplied
            && p.owner.kind() != "arrow_function"
            && !parsed.js_ts_is_module()
            && !parsed.js_ts_callable_is_strict(&p.owner)
            && p.owner
                .child_by_field_name("parameters")
                .is_some_and(|list| parsed.js_ts_parameters_are_simple(list));
        for &occurrence in refs.in_scope("arguments", p.owner) {
            if parsed.language.is_in_erased_type_context(occurrence)
                || (parsed.js_ts_identifier_is_not_a_read(occurrence)
                    && stability::write_operation(parsed, occurrence).is_none())
            {
                continue;
            }
            match parsed.js_ts_binding_walk(occurrence, "arguments", &mut cache) {
                Walk::Unchecked(_) | Walk::Found(_, Some(_)) => {
                    return Err(if mapped {
                        "mapped_arguments"
                    } else {
                        "stale_path"
                    });
                }
                Walk::Found(scope, None) => {
                    let owner = canonical_scope(scope);
                    let captured = owner.byte_range().contains(&p.owner.start_byte());
                    if (owner == p.owner || (!loose && captured))
                        && owner.kind() != "arrow_function"
                        && parsed
                            .language
                            .callable_boundary_node_types()
                            .contains(&owner.kind())
                        && param_slot::arguments_binding(parsed, occurrence, owner, &mut cache)
                    {
                        if mapped && owner == p.owner {
                            return Err("mapped_arguments");
                        }
                        if !loose {
                            return Err("stale_path");
                        }
                    }
                }
                Walk::Unbound => {}
            }
        }
        for (binding, scope, spelling, allowed, leaf) in tracked {
            for &occurrence in refs.in_scope(&spelling, p.owner) {
                if occurrence == binding
                    || Some(occurrence) == allowed
                    || parsed.language.is_in_erased_type_context(occurrence)
                    || parsed.js_ts_identifier_is_not_a_read(occurrence)
                {
                    continue;
                }
                match parsed.js_ts_binding_walk(occurrence, &spelling, &mut cache) {
                    Walk::Found(found, None)
                        if canonical_scope(found) == canonical_scope(scope)
                            && !loose
                            && !(leaf && invoked_read(occurrence)) =>
                    {
                        return Err("stale_path");
                    }
                    Walk::Found(_, None) | Walk::Unbound => {}
                    _ => return Err("stale_path"),
                }
            }
        }
        Ok(())
    }

    fn prefix(
        &self,
        file: &'a str,
        (body, observation): (Node<'a>, Node<'a>),
        copy: Option<Node<'a>>,
    ) -> bool {
        if body == observation {
            return true;
        }
        let mut directives = true;
        for statement in children(body).take_while(|node| *node != observation) {
            if directives
                && statement.kind() == "expression_statement"
                && children(statement)
                    .next()
                    .is_some_and(|node| node.kind() == "string")
            {
                continue;
            }
            directives = false;
            if statement.kind() == "empty_statement"
                || ordinary_declaration(statement)
                || copy.and_then(|decl| decl.parent()) == Some(statement)
            {
                continue;
            }
            let mut nodes = children(statement);
            if !nodes.next().is_some_and(|decl| {
                param_slot::is_const(decl)
                    && decl
                        .child_by_field_name("name")
                        .is_some_and(|name| name.kind() == "identifier")
                    && decl
                        .child_by_field_name("value")
                        .is_some_and(|node| self.closed((file, node), 0).is_ok())
            }) || nodes.next().is_some()
            {
                return false;
            }
        }
        true
    }
}

fn scalar(p: &Parameter<'_>) -> bool {
    p.body_read.is_none()
        && p.selected
            .iter()
            .all(|step| matches!(step, Selection::Default(_)))
}

fn invoked_read(read: Node<'_>) -> bool {
    outer(read)
        .parent()
        .and_then(ordinary_call)
        .is_some_and(|(callee, _)| callee == read)
}

fn placement<'a>(owner: Node<'a>, call: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if !matches!(
        owner.kind(),
        "function_declaration" | "function_expression" | "arrow_function" | "method_definition"
    ) || has_token(owner, "async")
        || has_token(owner, "*")
        || call.kind() != "call_expression"
    {
        return None;
    }
    let body = owner.child_by_field_name("body")?;
    let expression = outer(call);
    if expression == body && owner.kind() == "arrow_function" {
        return Some((body, body));
    }
    let statement = expression.parent()?;
    (statement.parent() == Some(body)
        && matches!(
            statement.kind(),
            "return_statement" | "expression_statement"
        )
        && children(statement).eq([expression]))
    .then_some((body, statement))
}
