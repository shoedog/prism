use crate::call_graph::{CallGraph, CallSite, FunctionId};

/// Last path segment of a `::`-scoped name (`crate::algo::run` -> `run`).
fn last_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// Module hint: the segment immediately before the final one
/// (`crate::algo::run` -> `algo`). `None` if the name is not `::`-scoped.
fn module_hint(name: &str) -> Option<&str> {
    let mut it = name.rsplit("::");
    let _fn = it.next()?; // final segment
    it.next() // the segment before it, if any
}

/// File stem of a path (`src/algorithms/original_diff.rs` -> `original_diff`).
/// NOTE: `.rsplit('.').last()` returns the FIRST dot-component (the stem), not the
/// extension — `.last()` consumes the reverse-order iterator to its final element.
/// This deliberately matches `resolve_callees_qualified`'s existing stem idiom in
/// `call_graph.rs` (so the delegated and fallback paths compute stems identically;
/// e.g. `a.b.rs` -> `a` in both). Do not "simplify" to `rsplit_once`, which would
/// diverge (`a.b.rs` -> `a.b`).
fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .last()
        .unwrap_or(path)
}

/// Nav-local callee resolution. Delegates to the shared (diff-review-shared)
/// `resolve_callees_qualified`; when that finds nothing AND the callee is a
/// `::`-scoped path, resolves the final segment narrowed to files whose stem
/// matches the module/namespace segment (Plan 3b.5 — Rust `mod::fn`, C++
/// `Ns::func`/`Class::method`). Additive: never changes a non-empty delegated
/// result, so existing behavior (incl. `.`-qualified languages) is preserved.
pub fn resolve_callees_nav<'a>(
    cg: &'a CallGraph,
    callee_name: &str,
    caller_file: &str,
    qualifier: Option<&str>,
) -> Vec<&'a FunctionId> {
    let delegated = cg.resolve_callees_qualified(callee_name, caller_file, qualifier);
    if !delegated.is_empty() {
        return delegated;
    }
    let Some(hint) = module_hint(callee_name) else {
        return delegated; // not `::`-scoped -> empty
    };
    // Path keywords are not module/namespace names; never stem-match them.
    if matches!(hint, "self" | "super" | "crate") {
        return Vec::new();
    }
    let fn_name = last_segment(callee_name);
    match cg.functions.get(fn_name) {
        Some(ids) => ids
            .iter()
            .filter(|fid| file_stem(&fid.file) == hint)
            .collect(),
        None => Vec::new(),
    }
}

/// Caller sites targeting a function named `target_name`, including scoped keys
/// (`callers` is keyed by the raw callee name, so a scoped call lives under
/// `"A::target_name"`). Bare and `::target_name`-suffixed keys are both returned;
/// the caller resolves each site to confirm the target file.
pub fn scoped_caller_sites<'a>(cg: &'a CallGraph, target_name: &str) -> Vec<&'a CallSite> {
    let suffix = format!("::{target_name}");
    let mut out: Vec<&CallSite> = Vec::new();
    for (key, sites) in &cg.callers {
        if key == target_name || key.ends_with(&suffix) {
            out.extend(sites.iter());
        }
    }
    out
}
