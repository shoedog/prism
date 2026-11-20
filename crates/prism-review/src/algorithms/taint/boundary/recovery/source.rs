use super::*;
use crate::algorithms::taint::{TaintRootOrigin, TaintSeed};
use prism::{
    access_path::{AccessPath, DEFAULT_K_LIMIT},
    anon_owner::{is_anonymous_owner_name, is_module_owner_name},
    api::ReviewOccurrenceOwner,
};
use std::ops::Range;
use tree_sitter::Node;

pub(super) struct Proof<'a> {
    pub owner: &'a ReviewOccurrenceOwner,
    pub seed: TaintSeed,
}
pub(super) fn owner<'a>(
    ctx: &'a RecoveryContext<'_>,
    value: &VarLocation,
) -> Option<&'a ReviewOccurrenceOwner> {
    let mut candidates = ctx.facts.owners().iter().filter(|row| {
        if value.function == "<anonymous>" {
            ctx.parsed.line_for_byte(row.span.start) == value.function_start_line
                && row.function.as_ref().is_none_or(|f| {
                    is_anonymous_owner_name(&f.name) && !is_module_owner_name(&f.name)
                })
        } else {
            row.function.as_ref().is_some_and(|f| {
                f.file == value.file
                    && f.name == value.function
                    && f.start_line == value.function_start_line
            })
        }
    });
    let result = candidates.next()?;
    (candidates.next().is_none() && result.unique && result.function.is_some()).then_some(result)
}
pub(super) fn admit<'a>(ctx: &'a RecoveryContext<'_>, root: &TaintRoot) -> Option<Proof<'a>> {
    let p = ctx.parsed;
    let seed = &root.seed;
    let loc = &root.location;
    let max_line = p.line_for_byte(p.source.len());
    if p.parse_error_count > 0
        || seed.file != p.path
        || loc.file != seed.file
        || seed.line != loc.line
        || seed.line == 0
        || seed.line > max_line
        || p.source.get(loc.start_byte..loc.end_byte).is_none()
        || seed
            .scope
            .is_some_and(|(a, b)| a == 0 || a > seed.line || seed.line > b || b > max_line)
        || seed
            .byte_scope
            .is_some_and(|(a, b)| p.source.get(a..b).is_none())
    {
        return None;
    }
    let owner = owner(ctx, loc)?;
    let mut effective = seed.clone();
    let range = if let Some(target) = &seed.target {
        if root.origin != TaintRootOrigin::Framework
            || loc.kind != VarAccessKind::Def
            || loc.start_byte != loc.end_byte
            || target != &loc.path
            || !literal(&target.base)
        {
            return None;
        }
        if let Some(start) = seed.start_byte {
            anchor(p, start, None, target)?
        } else {
            if !target.fields.is_empty() {
                return None;
            }
            p.line_start_byte(seed.line).max(owner.span.start)
                ..p.line_start_byte(seed.line + 1).min(owner.span.end)
        }
    } else {
        if loc.start_byte >= loc.end_byte
            || seed.start_byte.is_some_and(|start| start != loc.start_byte)
        {
            return None;
        }
        effective.start_byte = Some(loc.start_byte);
        anchor(p, loc.start_byte, Some(loc.end_byte), &loc.path)?
    };
    if range.start >= range.end
        || p.line_for_byte(range.start) != seed.line
        || range.start < owner.span.start
        || range.end > owner.span.end
        || effective.start_byte.is_some_and(|_| {
            seed.byte_scope
                .is_some_and(|(a, b)| range.start < a || range.end > b)
        })
        || ctx.facts.owners().iter().any(|other| {
            !std::ptr::eq(other, owner)
                && owner.span.start <= other.span.start
                && other.span.end <= owner.span.end
                && other.span.start < range.end
                && range.start < other.span.end
        })
    {
        return None;
    }
    Some(Proof {
        owner,
        seed: effective,
    })
}
fn literal(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'_' | b'$'))
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$'))
}
fn anchor(
    p: &ParsedFile,
    start: usize,
    end: Option<usize>,
    expected: &AccessPath,
) -> Option<Range<usize>> {
    let finish = end.unwrap_or(start.checked_add(1)?);
    p.source.get(start..finish)?;
    let mut node = p
        .tree
        .root_node()
        .descendant_for_byte_range(start, finish)?;
    if end.is_none() {
        while let Some(parent) = node.parent() {
            if !matches!(
                parent.kind(),
                "member_expression"
                    | "subscript_expression"
                    | "parenthesized_expression"
                    | "as_expression"
                    | "type_assertion"
                    | "non_null_expression"
                    | "satisfies_expression"
            ) {
                break;
            }
            node = parent;
        }
    }
    (node.start_byte() == start
        && end.is_none_or(|end| node.end_byte() == end)
        && syntax_path(p, node, 0).as_ref() == Some(expected))
    .then(|| node.byte_range())
}
fn syntax_path(p: &ParsedFile, node: Node<'_>, depth: usize) -> Option<AccessPath> {
    if depth > DEFAULT_K_LIMIT {
        return None;
    }
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let text = p.node_text(&node);
            literal(text).then(|| AccessPath::simple(text))
        }
        "member_expression" if is_js_ts_language(p.language) => {
            let object = node.child_by_field_name("object")?;
            let property = node.child_by_field_name("property")?;
            let text = p.node_text(&property);
            if property.kind() != "property_identifier"
                || !literal(text)
                || p.source
                    .get(object.end_byte()..property.start_byte())?
                    .trim()
                    != "."
            {
                return None;
            }
            let mut path = syntax_path(p, object, depth + 1)?;
            path.fields.push(text.into());
            Some(path)
        }
        _ => None,
    }
}
