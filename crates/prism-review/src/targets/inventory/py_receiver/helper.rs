use super::{
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
mod value;

use value::{contradicts_literal, HelperResult, HelperValue};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclarationPosition {
    Parameter,
    Local,
    Module,
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
            if !matches!(
                self.walk.scopes[alt.owner].kind,
                ScopeKind::Function | ScopeKind::Module
            ) {
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
        let (value, value_scope, position) = self.declaration_binding_value(binding, alt.owner)?;
        let local = position == DeclarationPosition::Local;
        let syntax = binding.node.child_by_field_name("type");
        let declaration =
            syntax.and_then(|annotation| self.declaration(annotation, value_scope, local));
        let chosen = declaration
            .as_ref()
            .and_then(|declaration| declaration.as_ref().ok());
        let declared_method = syntax
            .and_then(|annotation| self.method_declaration(annotation, value_scope, local, method))
            .filter(|(family, _)| {
                position != DeclarationPosition::Module || *family != Family::Http
            });
        let declared_family = declared_method.as_ref().map(|(family, _)| *family);
        let annotation_ties = declared_family.is_some() && chosen.is_some();
        let declaration_cause = declared_method.and_then(|(_, declaration)| declaration.err());
        let helper_call = value.and_then(|value| self.helper_call(value));
        let helper = helper_call.and_then(|helper| {
            (local || annotation_ties)
                .then(|| self.helper_value(helper, value_scope, Some(method), annotation_ties))
                .flatten()
        });
        let family =
            declared_family.or_else(|| local.then(|| helper.as_ref()?.family).flatten())?;

        let fresh = local
            .then(|| self.fresh_suite(alt, binding, at, scope))
            .flatten();
        let mut causes = Vec::new();
        if !self.available(binding, at, scope)
            || (position != DeclarationPosition::Parameter && self.type_only(binding))
        {
            causes.push(UnprovenCause::BindingUnavailable);
        }
        if !single_namespace || !self.receiver_unique(alt, binding, key, at, scope, fresh) {
            causes.push(UnprovenCause::ReceiverNotUnique);
        }
        let origin = if let Some(owner) = self.owning_self(binding) {
            Instance::OwningSelf(owner)
        } else {
            let helper_origin = helper_call.filter(|_| local);
            Instance::Constructed(
                helper_origin.map_or(binding.node.id(), |helper| helper.call.id()),
            )
        };
        let fresh_instance = fresh.filter(|_| self.unescaped_before(binding, key, at, scope));
        if self.instance_rebound(&origin, Some(method), at, scope, fresh_instance) {
            causes.push(UnprovenCause::MemberRebound);
        }
        let mut built = None;
        if let Some(helper) = &helper {
            match &helper.value {
                Ok(HelperResult::Constructed(constructed))
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
                Err(cause) if position != DeclarationPosition::Parameter => causes.push(*cause),
                _ => {}
            }
        }
        if let Some(built) = built.filter(|_| causes.is_empty()) {
            return Some(Resolution {
                family: py_tables::method_family(&built.class, method)?,
                grade: Grade::AnnotationTyped,
                identity: Ok(format!("{}.{method}", built.class)),
                #[cfg(any(test, feature = "inventory-evidence"))]
                declaration: None,
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
        let caller_declaration = chosen.filter(|_| annotation_ties);
        if let Some(declaration) = caller_declaration {
            if let Some(value) = value {
                if position != DeclarationPosition::Parameter && value.kind() == "none" {
                    causes.push(UnprovenCause::ReceiverIsNone);
                }
                if let Some(result) = self.constructor(value, value_scope, None) {
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
                                Ok(HelperResult::Constructed(built)) => {
                                    built.class != declaration.class
                                }
                                Err(UnprovenCause::AnnotationConflict) => true,
                                _ => false,
                            }
                    })
                {
                    causes.push(UnprovenCause::AnnotationConflict);
                }
            }
        }
        let selected = caller_declaration.or_else(|| {
            let declaration = helper.as_ref()?.declaration.as_ref()?;
            (local && chosen.is_none()).then_some(declaration)
        });
        if let Some(declaration) = selected {
            if let Some(selected_family) = py_tables::method_family(&declaration.class, method) {
                causes.extend(
                    self.prove_instance_method(&declaration.class, method, at, scope)
                        .err(),
                );
                if selected_family != Family::Http && causes.is_empty() {
                    return Some(Resolution {
                        family: selected_family,
                        grade: Grade::AnnotationTyped,
                        identity: Ok(format!("{}.{method}", declaration.class)),
                        #[cfg(any(test, feature = "inventory-evidence"))]
                        route: Some(if caller_declaration.is_some() {
                            super::super::evidence::Route::R4
                        } else {
                            super::super::evidence::Route::R5
                        }),
                        #[cfg(any(test, feature = "inventory-evidence"))]
                        receiver: None,
                        #[cfg(any(test, feature = "inventory-evidence"))]
                        declaration: Some(super::super::evidence::ClassSource::new(
                            &declaration.class,
                            declaration.node,
                        )),
                    });
                }
            }
        }
        let cause = causes
            .into_iter()
            .min()
            .unwrap_or(UnprovenCause::ReceiverValueUnproven);
        let refusal_family = built
            .and_then(|built| py_tables::method_family(&built.class, method))
            .unwrap_or(family);
        Some(Resolution::refused(
            refusal_family,
            Grade::AnnotationTyped,
            cause,
        ))
    }

    fn declaration_binding_value(
        &self,
        binding: &Binding<'tree>,
        owner: ScopeId,
    ) -> Option<(Option<Node<'tree>>, ScopeId, DeclarationPosition)> {
        match binding.kind {
            Kind::Assigned(Some(value))
                if owner == binding.scope
                    && value.kind() != "assignment"
                    && binding
                        .node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "expression_statement") =>
            {
                let position = match self.walk.scopes[owner].kind {
                    ScopeKind::Function => DeclarationPosition::Local,
                    ScopeKind::Module if binding.node.child_by_field_name("type").is_some() => {
                        DeclarationPosition::Module
                    }
                    _ => return None,
                };
                Some((Some(ungroup(value)), binding.scope, position))
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
                    DeclarationPosition::Parameter,
                ))
            }
            _ => None,
        }
    }

    pub(super) fn declared_origin(
        &self,
        binding: &Binding<'tree>,
        owner: ScopeId,
    ) -> Option<(usize, bool)> {
        let (value, scope, position) = self.declaration_binding_value(binding, owner)?;
        let local = position == DeclarationPosition::Local;
        let declared = binding
            .node
            .child_by_field_name("type")
            .is_some_and(|annotation| {
                self.declaration(annotation, scope, local)
                    .and_then(Result::ok)
                    .is_some_and(|decl| {
                        position != DeclarationPosition::Module
                            || py_tables::declaration_only_class(&decl.class)
                    })
            });
        if let Some(helper) = value
            .and_then(|value| self.helper_call(value))
            .filter(|_| local)
        {
            let bindings = self.helper_bindings(helper, scope);
            if declared || !bindings.is_empty() {
                let constructed = bindings.into_iter().any(|(binding, visible)| {
                    visible
                        && self
                            .helper_definition(binding, visible, helper.awaited, None, false)
                            .is_some_and(|helper| {
                                matches!(helper.value, Ok(HelperResult::Constructed(_)))
                            })
                });
                return Some((helper.call.id(), constructed));
            }
        }
        declared.then_some((binding.node.id(), false))
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
        let lexical = std::iter::successors(Some(scope), |&scope| {
            (scope != 0).then(|| self.walk.scopes[scope].parent)
        })
        .filter(|&scope| self.walk.scopes[scope].kind != ScopeKind::Class)
        .filter_map(|scope| self.census.spaces[scope].bindings.get(&key))
        .flatten();
        let mut bindings: Vec<(&Binding<'tree>, bool)> = Vec::new();
        for candidate in alternatives
            .iter()
            .flat_map(|alt| alt.bindings.iter().copied())
            .chain(lexical)
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
                        && self.available(candidate, helper.callee, scope)
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
        method: Option<&str>,
        use_declaration: bool,
    ) -> Option<HelperValue<'tree>> {
        self.helper_bindings(helper, scope)
            .into_iter()
            .filter_map(|(binding, visible)| {
                self.helper_definition(binding, visible, helper.awaited, method, use_declaration)
            })
            .min_by_key(|helper| match &helper.value {
                Ok(HelperResult::Constructed(_)) => (0, None),
                Ok(HelperResult::Opaque) => (1, None),
                Err(cause) => (2, Some(*cause)),
            })
    }
}
