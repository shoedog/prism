mod anchor;
pub(super) mod source;
pub(super) mod walk;

use super::{
    py_receiver::Resolver, Family, FunctionBounds, InventoryOutput, InventorySite,
    UnsupportedReason, UnsupportedSite,
};
use prism::ast::ParsedFile;
use prism::finding_confidence::ParseQuality;
use std::collections::BTreeSet;
use std::path::Path;
use walk::ScopeKind;

pub fn function_bounds(parsed: &ParsedFile) -> Vec<FunctionBounds> {
    if parsed.language != prism::languages::Language::Python {
        return Vec::new();
    }
    let source = source::Source::new(parsed);
    let parsed = source.parsed();
    let recovered = source::recover_bounds_parse(parsed);
    let parsed = recovered.as_ref().unwrap_or(parsed);
    let walk = walk::Walk::new(parsed);
    let mut functions: Vec<_> = walk
        .scopes
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Function)
        .collect();
    functions.sort_by_key(|scope| scope.node.start_byte());
    functions
        .into_iter()
        .filter_map(|scope| source::bounds(parsed, scope.node))
        .collect()
}

pub(super) fn collect_file(
    file: &str,
    parsed: &ParsedFile,
    repo: &Path,
    families: &BTreeSet<Family>,
    output: &mut InventoryOutput,
) {
    let source = source::Source::new(parsed);
    let parsed = source.parsed();
    let rate = parsed.error_rate();
    let quality = if rate > 0.3 {
        ParseQuality::Unparseable
    } else if rate > 0.1 {
        ParseQuality::Poor
    } else if rate > 0.01 {
        ParseQuality::Degraded
    } else {
        ParseQuality::Clean
    };
    output.parse_quality.insert(file.to_owned(), quality);
    if families.is_empty() {
        return;
    }

    let recovered = walk::recover_starred_annotations(parsed);
    let parsed = recovered.parsed.as_ref().unwrap_or(parsed);
    let walk = walk::Walk::with_refused_annotations(parsed, &recovered.refused);
    let resolver = Resolver::new(parsed, &walk, repo, file);
    let anchors = anchor::winners(&walk.calls);
    let owners: Vec<_> = walk
        .scopes
        .iter()
        .map(|scope| {
            (scope.kind == ScopeKind::Function)
                .then(|| source::bounds(parsed, scope.node))
                .flatten()
        })
        .collect();
    for call in walk.calls.iter().filter(|call| call.context.evaluated) {
        let Some(result) = resolver.resolve_call(call.callee, call.context.scope) else {
            continue;
        };
        if !families.contains(&result.family) {
            continue;
        }
        let resolved_callee = match result.identity {
            Ok(callee) => callee,
            Err(cause) => {
                *output
                    .unproven
                    .entry(file.to_owned())
                    .or_default()
                    .entry(cause)
                    .or_default() += 1;
                continue;
            }
        };
        let line = call.callee.start_position().row + 1;
        let owner = call.context.owner.and_then(|id| owners[id].as_ref());
        let reason = if call
            .context
            .owner
            .is_some_and(|id| walk.scopes[id].node.has_error())
        {
            Some(UnsupportedReason::ParseErrorInFunction)
        } else if owner.is_none() {
            Some(UnsupportedReason::ModuleLevel)
        } else if call.context.frame == Some(ScopeKind::Class) {
            Some(UnsupportedReason::ClassBodyFrame)
        } else if call.context.frame == Some(ScopeKind::Lambda) {
            Some(UnsupportedReason::LambdaFrame)
        } else if call.callee.end_position().row + 1 != line {
            Some(UnsupportedReason::MultilineCallee)
        } else if anchors
            .get(&line)
            .is_none_or(|anchor| anchor.node != call.node)
        {
            Some(UnsupportedReason::NotLineAnchor)
        } else {
            None
        };

        #[cfg(any(test, feature = "inventory-evidence"))]
        output.occurrences.push(super::evidence::Occurrence {
            file: file.to_owned(),
            symbol: owner.map(|owner| owner.name.clone()),
            canonical: resolved_callee.clone(),
            reason,
            proof: super::evidence::Proof {
                route: result.route.expect("proven inventory identity has a route"),
                call: super::evidence::Span::new(&source, call.node),
                callee: super::evidence::Span::new(&source, call.callee),
                receiver: result.receiver,
                declaration: result.declaration,
            },
        });

        if let Some(reason) = reason {
            output.unsupported.push(UnsupportedSite {
                file: file.to_owned(),
                line,
                symbol: owner.map(|owner| owner.name.clone()),
                family: result.family,
                resolved_callee,
                reason,
                grade: result.grade,
            });
        } else if let Some(owner) = owner {
            output.sites.push(InventorySite {
                file: file.to_owned(),
                line,
                symbol: owner.name.clone(),
                function_start_line: owner.start_line,
                function_end_line: owner.end_line,
                family: result.family,
                resolved_callee,
                source_callee: source.original_text(call.callee).to_owned(),
                grade: result.grade,
            });
        }
    }
}

#[cfg(test)]
mod tests;
