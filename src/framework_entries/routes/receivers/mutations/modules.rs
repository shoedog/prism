use super::PythonBindings;
use std::ops::Bound::{Included, Unbounded};

pub(super) fn relevant_module(bindings: &PythonBindings<'_>, module: &str) -> bool {
    if module == "flask" || module.starts_with("flask.") || bindings.module_file(module).is_some() {
        return true;
    }

    // An import can bind a package prefix even when it has no __init__.py.
    // The resolver can still find a child module through that prefix.
    let prefix = format!("{}/", module.replace('.', "/"));
    bindings
        .files
        .range::<str, _>((Included(prefix.as_str()), Unbounded))
        .next()
        .is_some_and(|(path, _)| path.starts_with(&prefix))
}
