use super::{
    annotation::Declaration,
    census::{Binding, Kind},
    lookup::Alternative,
    slots::Instance,
    Resolution, Resolver,
};
use crate::targets::inventory::{
    py_tables,
    python::{
        source::{children, name, ungroup},
        walk::{ScopeId, ScopeKind},
    },
    Family, Grade, UnprovenCause,
};
use tree_sitter::Node;

mod returned_local;

#[derive(Clone, Copy)]
struct HelperCall<'tree> {
    call: Node<'tree>,
    callee: Node<'tree>,
    awaited: bool,
}

struct Constructed<'tree> {
    class: String,
    #[cfg_attr(not(any(test, feature = "inventory-evidence")), allow(dead_code))]
    call: Node<'tree>,
}

struct HelperValue<'tree> {
    declaration: Option<Declaration<'tree>>,
    value: Result<Constructed<'tree>, UnprovenCause>,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn declared_receiver(
        &self,
        callee: Node<'tree>,
        scope: ScopeId,
    ) -> Option<Resolution> {
        let (receiver, method) = self.selected_method(callee, scope)?;
        if receiver.kind() != "identifier" {
            return None;
        }
        let key = name(self.parsed.node_text(&receiver), self.walk.class(scope));
        let alternatives = self.lookup(scope, &key, receiver);
        let mut result = None;
        for alt in &alternatives {
            if self.walk.scopes[alt.owner].kind != ScopeKind::Function {
                continue;
            }
            for binding in &alt.bindings {
                result = Resolution::prefer(result, || {
                    self.declared_binding(
                        alt,
                        binding,
                        &key,
                        callee,
                        scope,
                        &method,
                        alternatives.len() == 1,
                    )
                });
            }
        }
        result
    }

    fn declared_binding(
        &self,
        alt: &Alternative<'_, 'tree>,
        binding: &Binding<'tree>,
        key: &str,
        at: Node<'tree>,
        scope: ScopeId,
        method: &str,
        single_namespace: bool,
    ) -> Option<Resolution> {
        let (value, value_scope, local) = self.declaration_binding_value(binding, alt.owner)?;
        let syntax = binding.node.child_by_field_name("type");
        let declaration =
            syntax.and_then(|annotation| self.declaration(annotation, value_scope, local, None));
        let chosen = declaration
            .as_ref()
            .and_then(|declaration| declaration.as_ref().ok());
        let annotation_ties =
            chosen.is_some_and(|decl| py_tables::constructor_method(&decl.class, method));
        let declaration_cause = syntax.and_then(|annotation| {
            self.declaration(annotation, value_scope, local, Some(method))?
                .err()
        });
        let helper_call = value.and_then(|value| self.helper_call(value));
        let helper = helper_call.and_then(|helper| {
            (local || annotation_ties)
                .then(|| self.helper_value(helper, value_scope, method, annotation_ties))
                .flatten()
        });
        if !annotation_ties && declaration_cause.is_none() && (!local || helper.is_none()) {
            return None;
        }

        let fresh = local
            .then(|| self.fresh_suite(alt, binding, at, scope))
            .flatten();
        let mut causes = Vec::new();
        if !self.available(binding, at, scope) || (local && self.type_only(binding)) {
            causes.push(UnprovenCause::BindingUnavailable);
        }
        if !single_namespace || !self.receiver_unique(alt, binding, key, at, scope, fresh) {
            causes.push(UnprovenCause::ReceiverNotUnique);
        }
        let origin =
            Instance::Constructed(helper_call.map_or(binding.node.id(), |helper| helper.call.id()));
        let fresh_instance = fresh.filter(|_| self.unescaped_before(binding, key, at, scope));
        if self.instance_rebound(&origin, method, at, scope, fresh_instance) {
            causes.push(UnprovenCause::MemberRebound);
        }
        let mut built = None;
        if let Some(helper) = &helper {
            match &helper.value {
                Ok(constructed)
                    if local && py_tables::constructor_method(&constructed.class, method) =>
                {
                    if let Err(cause) =
                        self.prove_instance_method(&constructed.class, method, at, scope)
                    {
                        causes.push(cause);
                    }
                    if self.instance_context_rebound(
                        &constructed.class,
                        &origin,
                        binding,
                        at,
                        scope,
                    ) {
                        causes.push(UnprovenCause::MemberRebound);
                    }
                    built = Some(constructed);
                }
                Err(cause) if local => causes.push(*cause),
                _ => {}
            }
        }
        if let Some(built) = built.filter(|_| causes.is_empty()) {
            return Some(Resolution {
                family: Family::Http,
                grade: Grade::AnnotationTyped,
                identity: Ok(format!("{}.{method}", built.class)),
                #[cfg(any(test, feature = "inventory-evidence"))]
                route: Some(if chosen.is_some() {
                    super::super::evidence::Route::R4
                } else {
                    super::super::evidence::Route::R5
                }),
                #[cfg(any(test, feature = "inventory-evidence"))]
                receiver: {
                    use super::super::evidence::{ClassSource, ReceiverProof};
                    let mut proof = ReceiverProof::constructed(&built.class, built.call);
                    proof.conflicting_declarations.extend(
                        [
                            chosen,
                            helper
                                .as_ref()
                                .and_then(|helper| helper.declaration.as_ref()),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|decl| decl.class != built.class)
                        .map(|decl| ClassSource::new(&decl.class, decl.node)),
                    );
                    Some(proof)
                },
            });
        }
        causes.extend(declaration_cause);
        if let Some(declaration) = chosen.filter(|_| annotation_ties) {
            causes.extend(
                self.prove_instance_method(&declaration.class, method, at, scope)
                    .err(),
            );
            if let Some(value) = value {
                if local && value.kind() == "none" {
                    causes.push(UnprovenCause::ReceiverIsNone);
                }
                if let Some(result) = self.constructor(value, value_scope, None, None) {
                    match result {
                        Ok(class) if class != declaration.class => {
                            causes.push(UnprovenCause::AnnotationConflict)
                        }
                        Err(cause) => causes.push(cause),
                        _ => {}
                    }
                }
                if contradicts_literal(value, declaration.nullable)
                    || helper.as_ref().is_some_and(|helper| {
                        helper
                            .declaration
                            .as_ref()
                            .is_some_and(|other| other.class != declaration.class)
                            || match &helper.value {
                                Ok(built) => built.class != declaration.class,
                                Err(UnprovenCause::AnnotationConflict) => true,
                                _ => false,
                            }
                    })
                {
                    causes.push(UnprovenCause::AnnotationConflict);
                }
            }
        }
        let cause = causes
            .into_iter()
            .min()
            .unwrap_or(UnprovenCause::ReceiverValueUnproven);
        Some(Resolution {
            family: Family::Http,
            grade: Grade::AnnotationTyped,
            identity: Err(cause),
            #[cfg(any(test, feature = "inventory-evidence"))]
            route: None,
            #[cfg(any(test, feature = "inventory-evidence"))]
            receiver: None,
        })
    }

    fn declaration_binding_value(
        &self,
        binding: &Binding<'tree>,
        owner: ScopeId,
    ) -> Option<(Option<Node<'tree>>, ScopeId, bool)> {
        match binding.kind {
            Kind::Assigned(Some(value))
                if owner == binding.scope
                    && value.kind() != "assignment"
                    && binding
                        .node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "expression_statement") =>
            {
                Some((Some(ungroup(value)), binding.scope, true))
            }
            Kind::Parameter
                if matches!(
                    binding.node.kind(),
                    "typed_parameter" | "typed_default_parameter"
                ) && children(binding.node).next()?.kind() == "identifier" =>
            {
                Some((
                    binding.node.child_by_field_name("value").map(ungroup),
                    self.walk.scopes[binding.scope].parent,
                    false,
                ))
            }
            _ => None,
        }
    }

    pub(super) fn declared_origin(
        &self,
        binding: &Binding<'tree>,
        owner: ScopeId,
    ) -> Option<usize> {
        if self.walk.scopes[owner].kind != ScopeKind::Function {
            return None;
        }
        let (value, scope, local) = self.declaration_binding_value(binding, owner)?;
        let declared = binding
            .node
            .child_by_field_name("type")
            .is_some_and(|annotation| {
                self.declaration(annotation, scope, local, None)
                    .is_some_and(|result| result.is_ok())
            });
        if let Some(helper) = value.and_then(|value| self.helper_call(value)) {
            if local && (declared || !self.helper_bindings(helper, scope).is_empty()) {
                return Some(helper.call.id());
            }
        }
        declared.then_some(binding.node.id())
    }

    fn helper_call(&self, value: Node<'tree>) -> Option<HelperCall<'tree>> {
        let value = ungroup(value);
        let awaited = value.kind() == "await";
        let call = if awaited {
            ungroup(children(value).next()?)
        } else {
            value
        };
        if call.kind() != "call" {
            return None;
        }
        let callee = ungroup(self.parsed.call_function_node(&call)?);
        (callee.kind() == "identifier").then_some(HelperCall {
            call,
            callee,
            awaited,
        })
    }

    fn helper_bindings(
        &self,
        helper: HelperCall<'tree>,
        scope: ScopeId,
    ) -> Vec<(&Binding<'tree>, bool)> {
        let key = name(
            self.parsed.node_text(&helper.callee),
            self.walk.class(scope),
        );
        let alternatives = self.lookup(scope, &key, helper.callee);
        let deciding = alternatives.first().filter(|_| alternatives.len() == 1);
        let module = self.census.spaces[0].bindings.get(&key);
        let mut bindings: Vec<(&Binding<'tree>, bool)> = Vec::new();
        for candidate in alternatives
            .iter()
            .flat_map(|alt| alt.bindings.iter().copied())
            .chain(module.into_iter().flatten())
            .filter(|binding| binding.node.kind() == "function_definition")
        {
            if bindings
                .iter()
                .any(|(prior, _)| std::ptr::eq(*prior, candidate))
            {
                continue;
            }
            let visible = candidate.scope == 0
                && deciding.is_some_and(|alt| {
                    alt.owner == 0
                        && matches!(&alt.bindings[..], [only] if std::ptr::eq(*only, candidate))
                        && !self.name_rebound(0, &key, helper.callee, scope)
                });
            bindings.push((candidate, visible));
        }
        bindings
    }

    fn helper_value(
        &self,
        helper: HelperCall<'tree>,
        scope: ScopeId,
        method: &str,
        use_declaration: bool,
    ) -> Option<HelperValue<'tree>> {
        self.helper_bindings(helper, scope)
            .into_iter()
            .filter_map(|(binding, visible)| {
                self.helper_definition(binding, visible, helper.awaited, method, use_declaration)
            })
            .min_by_key(|helper| helper.value.as_ref().err().copied())
    }

    fn helper_definition(
        &self,
        binding: &Binding<'tree>,
        visible: bool,
        awaited: bool,
        method: &str,
        use_declaration: bool,
    ) -> Option<HelperValue<'tree>> {
        let definition = binding.node;
        let scope = *self.census.definition_scopes.get(&definition.id())?;
        let body = self.walk.scopes[scope].body;
        let facts = &self.census.spaces[scope];
        let syntax = definition.child_by_field_name("return_type");
        let declaration =
            syntax.and_then(|syntax| self.declaration(syntax, binding.scope, false, None));
        let declaration = match declaration {
            Some(Err(cause))
                if syntax.is_some_and(|annotation| {
                    self.declaration(annotation, binding.scope, false, Some(method))
                        .is_some()
                }) =>
            {
                return Some(HelperValue {
                    declaration: None,
                    value: Err(cause),
                })
            }
            candidate => candidate.and_then(Result::ok),
        };
        let returns: Vec<_> = facts
            .returns
            .iter()
            .map(|&statement| {
                let value = children(statement).next().map(ungroup);
                let constructor =
                    value.and_then(|value| self.constructor(value, scope, None, None));
                (value, constructor)
            })
            .collect();
        let direct_ties = returns.iter().any(|(value, _)| {
            value.is_some_and(|value| self.constructor(value, scope, Some(method), None).is_some())
        });
        let final_return =
            facts.returns.len() == 1 && children(body).last() == facts.returns.first().copied();
        let returned_local = if final_return && declaration.is_some() {
            returns[0]
                .0
                .and_then(|value| self.returned_local(value, scope, method))
        } else {
            None
        };
        let local_ties = matches!(&returned_local, Some(Ok(built))
            if py_tables::constructor_method(&built.class, method));
        if !direct_ties
            && !local_ties
            && !declaration.as_ref().is_some_and(|declaration| {
                use_declaration || py_tables::constructor_method(&declaration.class, method)
            })
        {
            return None;
        }
        let value = (|| {
            for (value, constructor) in &returns {
                if let Some(Err(cause)) = constructor {
                    if use_declaration
                        || value.is_some_and(|value| {
                            self.constructor(value, scope, Some(method), None).is_some()
                        })
                    {
                        return Err(*cause);
                    }
                }
            }
            if let Some(Err(cause)) = &returned_local {
                if *cause < UnprovenCause::ReceiverNotUnique {
                    return Err(*cause);
                }
            }
            let Some(declaration) = &declaration else {
                return Err(UnprovenCause::HelperUnannotated);
            };
            if !visible
                || facts.yields
                || definition
                    .parent()
                    .is_some_and(|parent| parent.kind() == "decorated_definition")
                || awaited
                    != definition
                        .child(0)
                        .is_some_and(|token| token.kind() == "async")
            {
                return Err(UnprovenCause::HelperNotAdmissible);
            }
            if returns
                .iter()
                .all(|(value, _)| value.is_none_or(|value| value.kind() == "none"))
            {
                return Err(UnprovenCause::ReceiverIsNone);
            }
            if final_return {
                if let [(Some(call), Some(Ok(class)))] = returns.as_slice() {
                    if py_tables::constructor_method(class, method) || use_declaration {
                        return Ok(Constructed {
                            call: *call,
                            class: class.clone(),
                        });
                    }
                }
            }
            if let Some(result) = returned_local {
                let built = result?;
                if py_tables::constructor_method(&built.class, method) || use_declaration {
                    return Ok(built);
                }
                if built.class != declaration.class {
                    return Err(UnprovenCause::AnnotationConflict);
                }
            }
            if returns.iter().any(|(value, constructor)| {
                matches!(constructor, Some(Ok(class)) if *class != declaration.class)
                    || value.is_some_and(|value| contradicts_literal(value, declaration.nullable))
            }) {
                return Err(UnprovenCause::AnnotationConflict);
            }
            Err(UnprovenCause::ReceiverValueUnproven)
        })();
        Some(HelperValue { declaration, value })
    }
}

fn contradicts_literal(node: Node<'_>, nullable: bool) -> bool {
    match node.kind() {
        "none" => !nullable,
        "dictionary"
        | "list"
        | "set"
        | "tuple"
        | "expression_list"
        | "string"
        | "concatenated_string"
        | "integer"
        | "float"
        | "true"
        | "false" => true,
        "unary_operator" => node
            .child_by_field_name("argument")
            .is_some_and(|argument| matches!(ungroup(argument).kind(), "integer" | "float")),
        _ => false,
    }
}
