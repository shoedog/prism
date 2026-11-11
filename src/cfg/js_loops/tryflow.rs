//! The try-flow admission boundary of the loop plan (#693, #697; T2, option A3).
//!
//! Generic JS try construction draws header -> catch, header -> finally and first-catch-statement -> finally
//! edges as `Structured`, and a finalizer's completion is not tied to the completion that entered it, so a loop
//! rewrite that leaves or repeats through a try (or a line projection that reaches one) can complete a route
//! around a write (probes `ret`, `thr`, `lc`, `nofin`, `tc`). Rather than whitelist shapes, an owner with loop
//! trees is REFUSED (every tree; the owner keeps main's geometry, labels and availability) when
//!
//! * (a) its own executable syntax contains a `try_statement` (scanned recursively over the owner's body,
//!   independent of the completion rules, so a class static block, `with`, label, `switch` or bare block cannot
//!   hide one; it follows EXECUTION regions: the owner's own parameter defaults and the eager computed keys and
//!   decorators of nested callables count, nested callable bodies and deferred defaults are the nested owner's), or
//! * (b) its selected statement-line universe, under the plan's assumption, lies on a line of ANY try region of
//!   the file (nested callable bodies and foreign projections included): downstream CFG selection is by endpoint
//!   line, so a line interval is the refusal check, never deletion authority.
//!
//! The decision is made inside the plan fixpoint (not as an early `tree_checked` failure), so a TryFlow-refused
//! tree refuses its owner whole: main's geometry, labels and availability stand. Limit: A3
//! contains EXPLICIT try flow. `await`, `yield` and iterator close without try syntax are out of model.
use super::Cx;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

type Range = (usize, usize);

/// Every `try_statement` of the file as an inclusive line interval, sorted by start line.
pub(super) struct Regions(pub(super) Vec<(usize, usize)>);

impl Regions {
    pub(super) fn of(parsed: &ParsedFile) -> Self {
        let mut out = Vec::new();
        // A `try_statement` starts with the keyword: a file without the word holds no region (speed gate).
        if !parsed.source.contains("try") {
            return Self(out);
        }
        let mut stack = vec![parsed.tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "try_statement" {
                out.push((n.start_position().row + 1, n.end_position().row + 1));
            }
            let mut cursor = n.walk();
            stack.extend(n.children(&mut cursor));
        }
        out.sort_unstable();
        Self(out)
    }

    /// A try region holding `line`.
    fn holding(&self, line: usize) -> Option<(usize, usize)> {
        let upto = self.0.partition_point(|r| r.0 <= line);
        self.0[..upto].iter().rev().find(|r| line <= r.1).copied()
    }
}

/// The first `try_statement` the OWNER executes, as its line. The scan follows execution regions, not syntax
/// ownership: the owner's body and its OWN parameter defaults, plus the eager parts of the callables nested in it
/// (a method's computed key, decorators and parameter decorators run when the class or object is built, whether or
/// not the method is ever called; any class static block reached through them is walked as ordinary syntax), and
/// class fields: names, keys, decorators and the initializers of `static` fields (instance initializers are deferred). A nested callable's body and
/// its deferred parameter defaults are the nested owner's and stay excluded.
fn own_try(owner: Node<'_>, body: Node<'_>, boundaries: &[&str]) -> Option<usize> {
    let mut stack = vec![body];
    if let Some(p) = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"))
    {
        stack.push(p);
    }
    let mut first: Option<usize> = None;
    while let Some(n) = stack.pop() {
        if n.id() != body.id() && boundaries.contains(&n.kind()) {
            // Only the eager parts of a nested callable belong to the owner: its computed key, its decorators and
            // its parameter decorators (never its parameter defaults or its body).
            if let Some(key) = n
                .child_by_field_name("name")
                .filter(|k| k.kind() == "computed_property_name")
            {
                stack.push(key);
            }
            let mut cursor = n.walk();
            stack.extend(n.children(&mut cursor).filter(|c| c.kind() == "decorator"));
            if let Some(params) = n.child_by_field_name("parameters") {
                let mut pc = params.walk();
                for param in params.named_children(&mut pc) {
                    let mut dc = param.walk();
                    stack.extend(param.children(&mut dc).filter(|c| c.kind() == "decorator"));
                }
            }
            continue;
        }
        // A class field: its name, computed key and decorators are eager; its initializer VALUE runs per instance
        // (deferred) unless the field is `static`.
        if matches!(n.kind(), "field_definition" | "public_field_definition") {
            let mut cursor = n.walk();
            let kids: Vec<Node<'_>> = n.children(&mut cursor).collect();
            if !kids.iter().any(|c| c.kind() == "static") {
                let value = n.child_by_field_name("value").map(|v| v.id());
                stack.extend(kids.into_iter().filter(|c| Some(c.id()) != value));
                continue;
            }
        }
        if n.kind() == "try_statement" {
            let l = n.start_position().row + 1;
            first = Some(first.map_or(l, |f| f.min(l)));
        }
        let mut cursor = n.walk();
        stack.extend(n.children(&mut cursor));
    }
    first
}

/// The outermost loops of every owner that is try-affected under the CURRENT assumption (see
/// `plan::with_admission`), with the first offender.
pub(super) fn affected(parsed: &ParsedFile, regions: &Regions) -> BTreeMap<Range, String> {
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut out = BTreeMap::new();
    for owner in parsed.all_functions() {
        let Some(body) = owner.child_by_field_name("body") else {
            continue;
        };
        let cx = Cx {
            parsed,
            body,
            boundaries: &boundaries,
        };
        let roots = cx.outermost();
        if roots.is_empty() {
            continue;
        }
        let why = own_try(owner, body, &boundaries)
            .map(|l| format!("try@L{l} in the owner's syntax"))
            .or_else(|| {
                parsed
                    .statements_in_function(&owner)
                    .iter()
                    .find_map(|(line, _)| {
                        regions
                            .holding(*line)
                            .map(|r| format!("statement line L{line} on try@L{}-{}", r.0, r.1))
                    })
            });
        if let Some(why) = why {
            for r in roots {
                out.insert((r.start_byte(), r.end_byte()), why.clone());
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "tryflow_tests.rs"]
mod tests;
