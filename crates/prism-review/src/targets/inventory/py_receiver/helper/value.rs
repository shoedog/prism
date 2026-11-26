use super::{Constructed, Resolver};
use crate::targets::inventory::{
    py_receiver::{annotation::Declaration, census::Binding},
    py_tables,
    python::source::{children, ungroup},
    Family, UnprovenCause,
};
use tree_sitter::Node;

pub(super) enum HelperResult<'tree> {
    Constructed(Constructed<'tree>),
    Opaque,
}

pub(super) struct HelperValue<'tree> {
    pub(super) declaration: Option<Declaration<'tree>>,
    pub(super) value: Result<HelperResult<'tree>, UnprovenCause>,
    pub(super) family: Option<Family>,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn helper_definition(
        &self,
        binding: &Binding<'tree>,
        visible: bool,
        awaited: bool,
        method: Option<&str>,
        use_declaration: bool,
    ) -> Option<HelperValue<'tree>> {
        let definition = binding.node;
        let scope = *self.census.definition_scopes.get(&definition.id())?;
        let body = self.walk.scopes[scope].body;
        let facts = &self.census.spaces[scope];
        let syntax = definition.child_by_field_name("return_type");
        let declaration = syntax.and_then(|syntax| self.declaration(syntax, binding.scope, false));
        let declared_family = method.and_then(|method| {
            let (family, _) = self.method_declaration(syntax?, binding.scope, false, method)?;
            Some(family)
        });
        let declaration_cause = declaration.as_ref().and_then(|result| {
            let cause = *result.as_ref().err()?;
            (use_declaration || method.is_none() || declared_family.is_some()).then_some(cause)
        });
        let declaration = declaration.and_then(Result::ok);
        let returns: Vec<_> = facts
            .returns
            .iter()
            .map(|&statement| {
                let value = children(statement).next().map(ungroup);
                let constructor = value.and_then(|value| self.constructor(value, scope, None));
                let family = method.and_then(|method| {
                    self.method_constructor(value?, scope, method, None)
                        .map(|(family, _)| family)
                });
                (value, constructor, family)
            })
            .collect();
        let matches_method =
            |class: &str| method.is_none_or(|method| py_tables::constructor_method(class, method));
        let direct_ties = returns.iter().any(|(_, constructor, family)| {
            family.is_some() || (method.is_none() && constructor.is_some())
        });
        let final_return =
            facts.returns.len() == 1 && children(body).last() == facts.returns.first().copied();
        let returned_local =
            if final_return && (declaration.is_some() || declaration_cause.is_some()) {
                returns[0]
                    .0
                    .and_then(|value| self.returned_local(value, scope, method))
            } else {
                None
            };
        let local_family = returned_local.as_ref().and_then(|(family, _)| *family);
        let local_ties = local_family.is_some()
            || (method.is_none() && matches!(returned_local, Some((_, Ok(_)))));
        let family = local_family
            .or_else(|| returns.iter().filter_map(|(_, _, family)| *family).min())
            .or(declared_family);
        if !direct_ties
            && !local_ties
            && declaration_cause.is_none()
            && !declaration
                .as_ref()
                .is_some_and(|declaration| use_declaration || matches_method(&declaration.class))
        {
            return None;
        }
        let value = (|| {
            let mut causes: Vec<_> = declaration_cause.into_iter().collect();
            causes.extend(returns.iter().filter_map(|(_, constructor, family)| {
                if use_declaration || family.is_some() || method.is_none() {
                    constructor.as_ref()?.as_ref().err().copied()
                } else {
                    None
                }
            }));
            causes.extend(
                returned_local
                    .as_ref()
                    .and_then(|(_, result)| result.as_ref().err())
                    .copied(),
            );
            if declaration.is_none() {
                causes.push(UnprovenCause::HelperUnannotated);
            }
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
                causes.push(UnprovenCause::HelperNotAdmissible);
            }
            if let Some(cause) = causes.into_iter().min() {
                return Err(cause);
            }
            let declaration = declaration
                .as_ref()
                .expect("missing declarations are refused");
            let can_fall_through = children(body)
                .last()
                .is_none_or(|stmt| stmt.kind() != "raise_statement");
            if (!returns.is_empty() || can_fall_through)
                && returns
                    .iter()
                    .all(|(value, _, _)| value.is_none_or(|value| value.kind() == "none"))
            {
                return Err(UnprovenCause::ReceiverIsNone);
            }
            if final_return {
                if let [(Some(call), Some(Ok(class)), family)] = returns.as_slice() {
                    if family.is_some() || method.is_none() || use_declaration {
                        return Ok(HelperResult::Constructed(Constructed {
                            call: *call,
                            class: class.clone(),
                        }));
                    }
                }
            }
            if let Some((_, result)) = returned_local {
                let built = result?;
                if matches_method(&built.class) || use_declaration {
                    return Ok(HelperResult::Constructed(built));
                }
                if built.class != declaration.class {
                    return Err(UnprovenCause::AnnotationConflict);
                }
            }
            if returns.iter().any(|(value, constructor, _)| {
                matches!(constructor, Some(Ok(class)) if *class != declaration.class)
                    || value.is_some_and(|value| contradicts_literal(value, declaration.nullable))
            }) {
                return Err(UnprovenCause::AnnotationConflict);
            }
            Ok(HelperResult::Opaque)
        })();
        Some(HelperValue {
            declaration,
            value,
            family,
        })
    }
}

pub(super) fn contradicts_literal(node: Node<'_>, nullable: bool) -> bool {
    match node.kind() {
        "none" => !nullable,
        "dictionary"
        | "dictionary_comprehension"
        | "list"
        | "list_comprehension"
        | "set"
        | "set_comprehension"
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
