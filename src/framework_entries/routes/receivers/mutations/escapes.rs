use super::{metadata::definition, *};
use crate::framework_entries::routes::{
    python::flask::registration::native_view_argument, receivers::PythonMember,
};

pub(super) struct ViewArgument<'a> {
    pub file: &'a ParsedFile,
    pub call: Node<'a>,
    pub value: Node<'a>,
}

impl<'a> PythonBindings<'a> {
    pub(crate) fn view_metadata_escape(
        &self,
        member: PythonMember<'a>,
        context: &SetupContext<'a>,
    ) -> Option<&'a str> {
        let writes = self.mutations.as_ref().expect("indexed Python bindings");
        let mut escaped = None;
        for argument in writes.view_arguments.get(&definition(member))? {
            let ViewArgument { file, call, value } = *argument;
            // Use the completed mutation index to check the callee and receiver.
            let proved = native_view_argument(self, file, call, value, context)
                || call.child_by_field_name("function").is_some_and(|callee| {
                    matches!(self.direct_callee(file, callee, call),
                        PythonValue::Definition(member) if no_effect_body(member.node))
                });
            if !proved && escaped.is_none() {
                escaped = Some(file.node_text(&call));
            }
        }
        escaped
    }
}

fn no_effect_body(node: Node<'_>) -> bool {
    node.kind() == "function_definition"
        && !node.has_error()
        && node
            .parent()
            .is_none_or(|p| p.kind() != "decorated_definition")
        && node.child_by_field_name("body").is_some_and(|body| {
            children(body)
                .into_iter()
                .all(|statement| match statement.kind() {
                    "comment" | "pass_statement" => true,
                    "return_statement" => statement
                        .named_child(0)
                        .is_none_or(|value| ungroup(value).kind() == "none"),
                    _ => false,
                })
        })
}
