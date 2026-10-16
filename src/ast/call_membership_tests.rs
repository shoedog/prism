//! S4a: counted work and unchanged-main identity receipts.
use super::*;
use serde_json::{json, Value};
use std::cell::Cell;

thread_local! {
    static INVENTORIES: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record_inventory() {
    INVENTORIES.with(|count| count.set(count.get() + 1));
}

fn inventory_count() -> usize {
    INVENTORIES.with(Cell::get)
}

fn meta(site: CallSiteMeta<'_>) -> Value {
    json!({
        "callee": site.callee_name, "line": site.line, "qualifier": site.qualifier,
        "start": site.start_byte, "end": site.end_byte,
        "receiver": site.receiver_node.map(|n| (n.kind(), n.start_byte(), n.end_byte())),
        "arg_count": site.arg_count, "arg_spread": site.arg_spread,
        "kind": site.kind_override, "origin": site.origin_override,
    })
}

fn extract(p: &ParsedFile, owner: Node<'_>, mode: usize) -> Value {
    let lines = (1..=p.source.lines().count().max(1)).collect();
    match mode {
        0 => json!(p.function_calls_on_lines(&owner, &lines)),
        1 => json!(p.function_calls_on_lines_with_qualifier(&owner, &lines)),
        2 => json!(p
            .function_calls_with_spans_on_lines(&owner, &lines, &BTreeSet::new())
            .into_iter()
            .map(meta)
            .collect::<Vec<_>>()),
        3 => {
            let (sites, facts) = p.function_calls_with_qualifier_and_spans_on_lines(
                &owner,
                &lines,
                &BTreeSet::new(),
            );
            assert_eq!(
                (facts.calls_recorded, facts.skipped_macros, facts.ctor_skips),
                (0, 0, 0)
            );
            json!(sites.into_iter().map(meta).collect::<Vec<_>>())
        }
        _ => unreachable!(),
    }
}

fn cost(mode: usize) {
    let mut excess = Vec::new();
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for nested in [false, true] {
            for fallback in [false, true] {
                for count in [1, 32, 128] {
                    let body = "target();".repeat(count);
                    let source = if nested {
                        format!("function outer(){{ {body} consume(() => {{ {body} }}); }}")
                    } else {
                        format!("function outer(){{ {body} }}")
                    };
                    let mut p = ParsedFile::parse("cost.ts", &source, lang).unwrap();
                    if fallback {
                        p.functions[0].kind_id = u16::MAX;
                        assert!(p.all_functions_inner().1, "must force whole-file fallback");
                    }
                    let owners = p.all_functions();
                    assert_eq!(owners.len(), if nested { 2 } else { 1 });
                    // Exact 1/32/128 captures in the flat case; repeat owners
                    // and switch to nested owners in the nested case.
                    for index in [0, 1, 0, 1].into_iter().filter(|&i| i < owners.len()) {
                        INVENTORIES.with(|n| n.set(0));
                        let sites = extract(&p, owners[index], mode);
                        let work = inventory_count();
                        assert_eq!(
                            sites.as_array().unwrap().len(),
                            count + usize::from(nested && index == 0)
                        );
                        if work != 1 {
                            excess.push(format!("{lang:?}, mode={mode}, captures={count}, nested={nested}, owner={index}, fallback={fallback}: {work}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        excess.is_empty(),
        "one reconstruction per extraction; excess work: {excess:#?}"
    );
}

#[test]
fn cost_names() {
    cost(0);
}
#[test]
fn cost_qualifiers() {
    cost(1);
}
#[test]
fn cost_spans() {
    cost(2);
}
#[test]
fn cost_qualifiers_and_spans() {
    cost(3);
}

#[test]
fn zero_inventory_for_empty_no_capture_and_legacy() {
    for (lang, source) in [
        (Language::JavaScript, ""),
        (Language::JavaScript, "function quiet() {}"),
        (Language::Python, "def f():\n    target()\n"),
        (Language::Rust, "fn f() { target(); }"),
    ] {
        let p = ParsedFile::parse("legacy", source, lang).unwrap();
        let owners = std::iter::once(p.tree.root_node()).chain(p.all_functions());
        for owner in owners {
            for mode in 0..4 {
                INVENTORIES.with(|n| n.set(0));
                let sites = extract(&p, owner, mode);
                assert_eq!(inventory_count(), 0, "{lang:?} {mode}");
                assert_eq!(
                    sites.as_array().unwrap().len(),
                    usize::from(source.contains("target"))
                );
            }
        }
    }
}

fn owners(p: &ParsedFile) -> Vec<Node<'_>> {
    fn walk<'a>(n: Node<'a>, out: &mut Vec<Node<'a>>) {
        out.push(n);
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            walk(child, out);
        }
    }
    let mut nodes = Vec::new();
    walk(p.tree.root_node(), &mut nodes);
    nodes
}

fn identity_receipt(p: &ParsedFile) -> Value {
    // Includes unlisted blocks, call nodes, same-span wrappers, roots and boundaries.
    let extracted: Vec<_> = owners(p)
        .into_iter()
        .map(|owner| {
            json!({
                "owner": (owner.kind(), owner.start_byte(), owner.end_byte()),
                "modes": (0..4).map(|m| extract(p, owner, m)).collect::<Vec<_>>()
            })
        })
        .collect();
    let files = BTreeMap::from([("identity.ts".to_string(), p.clone())]);
    let cg = crate::call_graph::CallGraph::build(&files);
    let calls: Vec<_> = cg.calls.values().flatten().collect();
    let cpg = crate::cpg::CodePropertyGraph::build(&files);
    let nodes: Vec<_> = cpg
        .node_indices()
        .map(|i| format!("{:?}", cpg.node(i)))
        .collect();
    json!({"extracted": extracted, "calls": calls, "nodes": nodes, "edges": cpg.edge_dump()})
}

#[test]
fn identity_matches_main_complete_sites_and_edges() {
    let mut receipts = Vec::new();
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in [
            "",
            "function target(x) { return x; } function outer(p = target(1)) { target(p); consume(() => target(p), function(){ target(2); }, function*(){ yield target(3); }); } function* gen(){ yield target(4); }",
            "function target(){} function outer(){ class C { field = () => target(); [target()](){ target(); (() => target())(); } } target(); }",
            "function target(){} function recovered( { target(); }\nconst cb = () => target();",
            "function target(){} const cb = () => obj.target(1, ...xs);",
        ] {
            let p = ParsedFile::parse("identity.ts", source, lang).unwrap();
            receipts.push(json!({"lang": format!("{lang:?}"), "source": source, "receipt": identity_receipt(&p)}));
        }
    }
    let mut p = ParsedFile::parse(
        "identity.ts",
        "function target(){} function outer(){ consume(() => target()); target(); }",
        Language::TypeScript,
    )
    .unwrap();
    // Corrupt the call-bearing owner, so eager-table-only membership loses calls.
    p.functions[1].kind_id = u16::MAX;
    assert!(p.all_functions_inner().1);
    receipts.push(json!({"corrupt": identity_receipt(&p)}));
    // No eager entries: these otherwise valid function nodes are deliberately unlisted.
    p.functions.clear();
    receipts.push(json!({"unlisted": identity_receipt(&p)}));
    let local = ParsedFile::parse(
        "identity.ts",
        "function f(){ target(); }",
        Language::JavaScript,
    )
    .unwrap();
    for source in [
        "function f(){ target(); }",
        "function f(){ target();  }",
        "const f = ()=>target();",
    ] {
        let foreign = ParsedFile::parse("foreign.ts", source, Language::TypeScript).unwrap();
        let owner = foreign.all_functions()[0];
        receipts.push(json!({"foreign": source, "modes": (0..4).map(|m| extract(&local, owner, m)).collect::<Vec<_>>()}));
    }
    let py = ParsedFile::parse(
        "identity.py",
        "def target(x):\n    return x\ndef outer():\n    target(1)\n",
        Language::Python,
    )
    .unwrap();
    receipts.push(json!({"python": identity_receipt(&py)}));
    let got = json!(receipts);
    let expected: Value =
        serde_json::from_str(include_str!("call_membership_snapshot.json")).unwrap();
    assert_eq!(
        got, expected,
        "complete unchanged-main call metadata and CPG edges"
    );
}

#[test]
fn foreign_kind_and_same_span_ancestors() {
    let p = ParsedFile::parse(
        "local.js",
        "function f(){ target(); }",
        Language::JavaScript,
    )
    .unwrap();
    let owner = p.all_functions()[0];
    assert_eq!(owner.byte_range(), owner.parent().unwrap().byte_range());
    let arrow_source = format!(
        "() => {{ target();{} }}",
        " ".repeat(p.source.len() - "() => { target(); }".len())
    );
    let foreign = ParsedFile::parse("foreign.ts", &arrow_source, Language::TypeScript).unwrap();
    let arrow = foreign.all_functions()[0];
    assert_eq!(owner.byte_range(), arrow.byte_range());
    assert_ne!(owner.kind(), arrow.kind());
    assert_eq!(
        extract(&p, owner, 3),
        json!([{
            "callee":"target", "line":1, "qualifier":null, "start":14, "end":22,
            "receiver":null, "arg_count":0, "arg_spread":false, "kind":null, "origin":null
        }])
    );
    for mode in 0..4 {
        assert_eq!(extract(&p, arrow, mode), json!([]));
    }
}
