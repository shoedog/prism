use super::*;

pub(in crate::framework_entries::routes) fn native_view_argument<'a>(
    bindings: &PythonBindings<'a>,
    file: &ParsedFile,
    call: Node<'a>,
    value: Node<'a>,
    context: &SetupContext<'a>,
) -> bool {
    let Some((object, method)) = call_method(file, call) else {
        return false;
    };
    let args = arguments(call);
    if method != "add_url_rule"
        || args.unpacked()
        || args.get(file, "view_func", 2).written().map(ungroup) != Some(value)
    {
        return false;
    }
    let blueprint = record_once_owner(bindings, file, python_scope(call), object);
    let (owner, slots) = blueprint.map_or((object, ADD_URL_RULE), |owner| {
        (owner, SETUP_STATE_ADD_URL_RULE)
    });
    let default_context = SetupContext::default();
    // A factory frame establishes lookup order only in its own body.
    let context = context
        .factory
        .filter(|factory| factory.file.path == file.path && factory.node == python_scope(owner))
        .map_or(&default_context, |_| context);
    args.accepts(file, slots, true)
        && matches!(bindings.capture_value(&file.path, owner, context),
            PythonValue::Router(router) if blueprint.is_none() || router.kind == PythonKind::Blueprint)
}
