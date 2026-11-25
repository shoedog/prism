//! Python walk of the I0 inventory over the pinned tree-sitter-python grammar.
use super::*;
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Scopes of their own: the owner scan never reads a store or operand through them.
const FENCES: [&str; 8] = [
    "function_definition",
    "decorated_definition",
    "lambda",
    "class_definition",
    "list_comprehension",
    "set_comprehension",
    "dictionary_comprehension",
    "generator_expression",
];
const CONTAINERS: [&str; 6] = [
    "pattern_list",
    "tuple_pattern",
    "list_pattern",
    "list_splat_pattern",
    "expression_list",
    "tuple",
];
/// Calls spelled like these supply no R, shadowed or not.
const EXCLUDED: [&str; 5] = ["len", "cap", "id", "delete", "clear"];

fn span(n: Node) -> Span {
    Span {
        start: n.start_byte(),
        end: n.end_byte(),
        line: n.start_position().row + 1,
    }
}

fn kids(n: Node) -> Vec<Node> {
    n.children(&mut n.walk())
        .filter(|c| c.kind() != "comment")
        .collect()
}

fn is_field(parent: Node, field: &str, n: Node) -> bool {
    parent
        .child_by_field_name(field)
        .is_some_and(|f| f.id() == n.id())
}

fn peel_down(mut n: Node) -> Node {
    while n.kind() == "parenthesized_expression" {
        let named: Vec<_> = kids(n).into_iter().filter(|c| c.is_named()).collect();
        match named[..] {
            [only] => n = only,
            _ => break,
        }
    }
    n
}

/// One inventory per parsed owner (a decorated owner is scanned through its function).
pub(crate) fn scan_file(parsed: &ParsedFile) -> Vec<OwnerScan> {
    let src = parsed.source.as_str();
    let mut scans = Vec::new();
    for owner in parsed.all_functions() {
        let def = kids(owner)
            .into_iter()
            .find(|c| c.kind() == "function_definition");
        let Some(body) = def.unwrap_or(owner).child_by_field_name("body") else {
            continue;
        };
        let name = parsed
            .language
            .function_name(&owner)
            .map(|n| parsed.node_text(&n));
        let (mut writes, mut redirected) = (Vec::new(), BTreeSet::new());
        collect_writes(body, src, &mut writes, &mut redirected);
        for w in &mut writes {
            let hit = w
                .path
                .root
                .as_ref()
                .is_some_and(|r| redirected.contains(r.as_str()));
            if w.verdict.is_ok() && hit {
                w.verdict = Err(WriteRefusal::Redirected);
            }
        }
        let roots: BTreeSet<_> = writes
            .iter()
            .filter_map(|w| w.path.root.as_deref())
            .collect();
        let mut operands = Vec::new();
        if !roots.is_empty() {
            collect_operands(body, false, src, &roots, &mut operands);
        }
        let name = name.unwrap_or_default().to_string();
        scans.push(OwnerScan {
            name,
            span: span(owner),
            writes,
            operands,
        });
    }
    scans
}

fn text<'a>(n: Node, src: &'a str) -> &'a str {
    &src[n.start_byte()..n.end_byte()]
}

fn collect_writes<'a>(
    n: Node<'a>,
    src: &str,
    out: &mut Vec<Write>,
    redirected: &mut BTreeSet<String>,
) {
    for c in kids(n) {
        if FENCES.contains(&c.kind()) {
            continue;
        }
        match c.kind() {
            "assignment" | "augmented_assignment" => {
                let kind = if c.kind() == "assignment" {
                    WriteKind::Assign
                } else {
                    WriteKind::Augmented
                };
                let value = c.child_by_field_name("right").is_some();
                let left = c.child_by_field_name("left");
                targets(left, Ok((kind, value)), out, src);
            }
            "delete_statement" => kids(c)
                .into_iter()
                .for_each(|t| targets(Some(t), Err(WriteRefusal::Delete), out, src)),
            "for_statement" => targets(
                c.child_by_field_name("left"),
                Err(WriteRefusal::BindingTarget),
                out,
                src,
            ),
            "as_pattern_target" => kids(c)
                .into_iter()
                .for_each(|t| targets(Some(t), Err(WriteRefusal::BindingTarget), out, src)),
            "global_statement" | "nonlocal_statement" => {
                redirected.extend(
                    kids(c)
                        .iter()
                        .filter(|i| i.is_named())
                        .map(|i| text(*i, src).to_string()),
                );
            }
            _ => {}
        }
        collect_writes(c, src, out, redirected);
    }
}

fn leaves<'a>(n: Node<'a>, out: &mut Vec<Node<'a>>, nested: &mut bool) {
    let n = peel_down(n);
    if CONTAINERS.contains(&n.kind()) {
        *nested = true;
        kids(n)
            .into_iter()
            .filter(|c| c.is_named())
            .for_each(|c| leaves(c, out, nested));
    } else {
        out.push(n);
    }
}

/// Record every indexed leaf of a store target. `how` is `Err` for a non-value target,
/// else the assignment kind and whether the statement carries a value.
fn targets(
    left: Option<Node>,
    how: Result<(WriteKind, bool), WriteRefusal>,
    out: &mut Vec<Write>,
    src: &str,
) {
    let (mut found, mut nested) = (Vec::new(), false);
    if let Some(l) = left {
        leaves(l, &mut found, &mut nested);
    }
    for leaf in found
        .into_iter()
        .filter(|l| matches!(l.kind(), "subscript" | "attribute"))
    {
        let (path, shape) = static_path(leaf, src);
        if leaf.kind() == "attribute" && path.brackets == 0 {
            continue;
        }
        let verdict = match (how, shape) {
            (Err(why), _) => Err(why),
            (Ok(_), _) if nested => Err(WriteRefusal::Unpack),
            (Ok(_), Some(why)) => Err(why),
            (Ok((_, false)), None) => Err(WriteRefusal::NoValue),
            (Ok((kind, true)), None) => Ok(kind),
        };
        out.push(Write {
            span: span(leaf),
            path,
            verdict,
        });
    }
}

/// Root, pre-bracket fields and bracket count of a store target, plus the first shape refusal.
fn static_path(leaf: Node, src: &str) -> (StaticPath, Option<WriteRefusal>) {
    let (mut steps, mut n, mut shape) = (Vec::new(), peel_down(leaf), None);
    loop {
        match n.kind() {
            "subscript" => {
                let idx = kids(n)
                    .into_iter()
                    .filter(|c| is_field(n, "subscript", *c))
                    .count();
                let comma = kids(n).iter().any(|c| c.kind() == ",");
                let slice = kids(n).iter().any(|c| c.kind() == "slice");
                let why = if slice {
                    WriteRefusal::Slice
                } else {
                    WriteRefusal::MultiIndex
                };
                if slice || comma || idx != 1 {
                    shape.get_or_insert(why);
                }
                steps.push(None);
                n = peel_down(n.child_by_field_name("value").unwrap_or(n));
            }
            "attribute" => {
                steps.push(
                    n.child_by_field_name("attribute")
                        .map(|a| text(a, src).to_string()),
                );
                n = peel_down(n.child_by_field_name("object").unwrap_or(n));
            }
            _ => break,
        }
        if steps.len() > 4096 {
            break;
        }
    }
    steps.reverse();
    let fields: Vec<_> = steps.iter().map_while(|s| s.clone()).collect();
    let brackets = steps.iter().filter(|s| s.is_none()).count();
    if steps.len() - fields.len() != brackets {
        shape = Some(WriteRefusal::DotAfterBracket).or(shape);
    }
    let root = (n.kind() == "identifier").then(|| text(n, src).to_string());
    if root.is_none() {
        shape.get_or_insert(WriteRefusal::NonNameRoot);
    }
    (
        StaticPath {
            root,
            fields,
            brackets,
        },
        shape,
    )
}

fn collect_operands(
    n: Node,
    fenced: bool,
    src: &str,
    roots: &BTreeSet<&str>,
    out: &mut Vec<Operand>,
) {
    for c in kids(n) {
        let inside = fenced || FENCES.contains(&c.kind());
        if c.kind() == "identifier" && roots.contains(text(c, src)) {
            out.extend(operand(c, inside, src));
        }
        collect_operands(c, inside, src, roots, out);
    }
}

fn is_binding(c: Node) -> bool {
    let Some(p) = c.parent() else { return false };
    match p.kind() {
        "function_definition" | "class_definition" | "parameters" | "lambda_parameters" => true,
        "typed_parameter" | "global_statement" | "nonlocal_statement" | "import_statement" => true,
        "import_from_statement" | "aliased_import" | "dotted_name" | "relative_import" => true,
        "default_parameter" | "typed_default_parameter" | "named_expression" => {
            is_field(p, "name", c)
        }
        "keyword_argument" => is_field(p, "name", c),
        "attribute" => is_field(p, "attribute", c),
        _ => false,
    }
}

/// True when the node (through containers and parentheses) is a store, loop or `as` target.
fn is_target(mut t: Node) -> bool {
    while let Some(p) = t.parent() {
        match p.kind() {
            k if CONTAINERS.contains(&k) || k == "parenthesized_expression" => t = p,
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                return is_field(p, "left", t)
            }
            "delete_statement" | "as_pattern_target" => return true,
            _ => return false,
        }
    }
    false
}

fn path_of(mut n: Node, src: &str) -> StaticPath {
    let mut fields = Vec::new();
    while n.kind() == "attribute" {
        fields.extend(
            n.child_by_field_name("attribute")
                .map(|a| text(a, src).to_string()),
        );
        n = n.child_by_field_name("object").unwrap_or(n);
    }
    fields.reverse();
    StaticPath {
        root: Some(text(n, src).to_string()),
        fields,
        brackets: 0,
    }
}

fn callee_verdict(call: Option<Node>, role: Role, src: &str) -> Result<Role, OperandRefusal> {
    let f = call
        .filter(|c| c.kind() == "call")
        .and_then(|c| c.child_by_field_name("function"));
    let Some(f) = f else {
        return Err(OperandRefusal::Wrapped);
    };
    let last = if f.kind() == "attribute" {
        f.child_by_field_name("attribute").unwrap_or(f)
    } else {
        f
    };
    match EXCLUDED.contains(&text(last, src)) {
        true => Err(OperandRefusal::ExcludedCallee),
        false => Ok(role),
    }
}

fn operand(c: Node, fenced: bool, src: &str) -> Option<Operand> {
    if is_binding(c) {
        return None;
    }
    let mut t = c;
    while let Some(p) = t.parent() {
        let step = (p.kind() == "subscript" && is_field(p, "value", t))
            || p.kind() == "parenthesized_expression";
        if !(step || (p.kind() == "attribute" && is_field(p, "object", t))) {
            break;
        }
        t = p;
    }
    if is_target(t) {
        return None;
    }
    let mut top = c;
    while let Some(p) = top
        .parent()
        .filter(|p| p.kind() == "attribute" && is_field(*p, "object", top))
    {
        top = p;
    }
    let called = top
        .parent()
        .is_some_and(|p| p.kind() == "call" && is_field(p, "function", top));
    let (node, early) = match (called, top.kind()) {
        (true, "attribute") => (
            top.child_by_field_name("object")?,
            Some(OperandRefusal::Receiver),
        ),
        (true, _) => (top, Some(OperandRefusal::Callee)),
        _ => (top, None),
    };
    let mut outer = node;
    while let Some(p) = outer
        .parent()
        .filter(|p| p.kind() == "parenthesized_expression")
    {
        outer = p;
    }
    let parent = outer.parent()?;
    let verdict = match (fenced, early, parent.kind()) {
        (true, ..) => Err(OperandRefusal::Fenced),
        (_, Some(why), _) => Err(why),
        (_, _, "subscript") if !is_field(parent, "value", outer) => Err(OperandRefusal::Key),
        (_, _, "subscript") if kids(parent).iter().any(|k| k.kind() == "slice") => {
            Err(OperandRefusal::Slice)
        }
        (_, _, "subscript") => Err(OperandRefusal::Projected),
        (_, _, "argument_list") => callee_verdict(parent.parent(), Role::Argument, src),
        (_, _, "keyword_argument") => {
            callee_verdict(parent.parent().and_then(|a| a.parent()), Role::Keyword, src)
        }
        (_, _, "return_statement") => Ok(Role::Return),
        (_, _, "assignment") if is_field(parent, "right", outer) => Ok(Role::AssignmentRhs),
        _ => Err(OperandRefusal::Wrapped),
    };
    Some(Operand {
        span: span(node),
        outer: span(outer),
        path: path_of(node, src),
        verdict,
    })
}
