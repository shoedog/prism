//! Replay the first-wins lookup on the immutable, masked Step 5b input.

use super::props_generated_tests::{endpoint, exact_node, Access, Endpoint, Span};
use super::props_s2_test_support::{capture, PreBinding};
use super::var_index::{VarIndex, VarKeyRef};
use super::{CodePropertyGraph, CpgEdge, FlowConfidence, VarAccess};
use crate::access_path::AccessPath;
use crate::ast::{JsBindingCache, ParsedFile};
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
pub(super) struct Replay {
    lines: Vec<usize>,
    root_line: usize,
    root: Span,
    full: Endpoint,
    base: Endpoint,
    def: Endpoint,
    dest: Endpoint,
    binding: Span,
    full_binding: Span,
    write: Span,
    conflict: String,
}

pub(super) fn matching(pre: &PreBinding, want: &Endpoint) -> Option<NodeIndex> {
    let ids: Vec<_> = pre
        .graph
        .node_indices()
        .filter(|&idx| endpoint(&pre.graph[idx]).as_ref() == Some(want))
        .collect();
    assert!(ids.len() <= 1, "ambiguous marked node {want:?}: {ids:?}");
    ids.first().copied()
}

pub(super) fn indexed(
    pre: &PreBinding,
    lookup: &VarIndex,
    want: &Endpoint,
    line: usize,
) -> Option<NodeIndex> {
    let mut fields = want.name.split('.');
    let path = AccessPath::with_fields(fields.next().unwrap(), fields.map(str::to_owned).collect());
    lookup.get_ref(
        &pre.graph,
        VarKeyRef {
            file: &want.file,
            function: &want.owner,
            function_start_line: want.owner_line,
            line,
            path: &path,
            access: VarAccess::Use,
        },
    )
}

pub(super) fn prove_inputs(file: &ParsedFile, pre: &PreBinding, want: &Replay) {
    let mut cache = JsBindingCache::for_sites();
    let mut declaration = |range: Span, binding: Span| {
        let ident = exact_node(file, range);
        let binding_node = exact_node(file, binding);
        let declared = file
            .js_ts_site_declaration(ident, "box", &mut cache)
            .expect("syntax resolves the operand root")
            .1;
        assert!(
            declared.start_byte() <= binding_node.start_byte()
                && binding_node.end_byte() <= declared.end_byte(),
            "{range:?} binds at {binding:?}"
        );
    };
    declaration(want.root, want.binding);
    declaration(want.base.bytes, want.binding);
    declaration(
        [want.full.bytes[0], want.full.bytes[0] + 3],
        want.full_binding,
    );
    let assignment = exact_node(file, want.write);
    assert_eq!(assignment.kind(), "assignment_expression");
    match want.conflict.as_str() {
        "shadow" => assert_ne!(want.binding, want.full_binding),
        "version" => {
            assert_eq!(want.binding, want.full_binding);
            assert!(want.root[1] < want.write[0] && want.write[1] < want.full.bytes[0]);
            declaration([want.write[0], want.write[0] + 3], want.binding);
        }
        other => panic!("unknown conflict {other}"),
    }
    let full = matching(pre, &want.full).expect("actual full Use at competing bytes");
    let base = matching(pre, &want.base).expect("actual base Use at the root-line bytes");
    let def = matching(pre, &want.def).expect("outer Def at marked declaration");
    assert!(matching(pre, &want.dest).is_some(), "marked target Def");
    assert_eq!(
        indexed(pre, &pre.live_lookup, &want.full, want.full.line),
        Some(full)
    );
    assert_eq!(
        indexed(pre, &pre.live_lookup, &want.base, want.root_line),
        Some(base)
    );
    assert!(want.lines.contains(&want.full.line));
    assert!(pre
        .graph
        .edges_connecting(def, base)
        .any(|edge| edge.weight() == &CpgEdge::DataFlow(FlowConfidence::Exact)));
}

pub(super) fn compare_replay(
    file: &ParsedFile,
    opening: Span,
    graph: &CodePropertyGraph,
    pre: &PreBinding,
    want: &Replay,
) -> Result<(), String> {
    let base = matching(pre, &want.base).unwrap();
    let def = matching(pre, &want.def).unwrap();
    let dest = matching(pre, &want.dest).unwrap();
    let full_ids: BTreeSet<_> = want
        .lines
        .iter()
        .filter_map(|&line| indexed(pre, &pre.live_lookup, &want.full, line))
        .collect();
    assert!(!full_ids.is_empty());
    let mut masked = pre.live_lookup.clone();
    masked.hide(full_ids);
    for &line in &want.lines {
        assert_eq!(indexed(pre, &masked, &want.full, line), None);
        assert_eq!(
            indexed(pre, &masked, &want.base, line),
            indexed(pre, &pre.live_lookup, &want.base, line)
        );
    }
    let replay = |lookup, mutant| {
        super::jsx_props_replay::replay(
            file,
            opening[0]..opening[1],
            graph,
            &pre.graph,
            lookup,
            dest,
            mutant,
        )
    };
    let untouched = replay(&pre.live_lookup, false);
    let hidden_full = replay(&masked, false);
    let mutant = replay(&pre.live_lookup, true);
    // Rule R (positive certificate P2) refuses the base representative whatever the lookup
    // shows: the owner holds a second declaration of the spelling (shadow) or a store through
    // it (version), so hiding or ignoring the competing full candidate certifies nothing.
    let expected: Vec<(NodeIndex, NodeIndex, CpgEdge)> = vec![];
    let refusal = graph
        .dfg_label_stats
        .dfg_props
        .get("refused_source_ambiguous")
        .copied()
        .unwrap_or_default();
    let live_path = graph.has_path(def, dest, &CpgEdge::is_data_flow);
    let live_edge = graph
        .graph
        .edges_connecting(base, dest)
        .any(|edge| edge.weight().is_data_flow());
    if untouched.is_empty()
        && hidden_full == expected
        && mutant == expected
        && refusal == 1
        && !live_path
        && !live_edge
    {
        return Ok(());
    }
    Err(format!(
        "live refusal={refusal}, edge={live_edge}, path={live_path}, replay={untouched:?}; masked={hidden_full:?}, fall-through={mutant:?}, expected={expected:?}"
    ))
}

#[test]
fn props_s3_live_lookup_pinned_multiline_replays() {
    let rows = include_str!("../../tests/fixtures/props_matrix/rows.jsonl");
    let cases = [
        ("d1-multiline-selection", "shadow", "shadow"),
        ("d1-multiline-selection", "stale", "version"),
        ("d2-multiline-shadow", "conflict", "shadow"),
        ("d2-multiline-version", "conflict", "version"),
    ];
    let mut errors = Vec::new();
    for (id, case, conflict) in cases {
        let row = rows
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|row| row["id"] == id)
            .unwrap();
        let source = row["files"][format!("{case}/m")].as_str().unwrap();
        for ext in ["jsx", "tsx"] {
            let path = format!("m.{ext}");
            let files = super::props_generated_tests::parse(&BTreeMap::from([(
                path.clone(),
                source.into(),
            )]));
            let file = &files[&path];
            assert_eq!(file.parse_error_count, 0);
            let (graph, capture) = capture(&files, || CodePropertyGraph::build(&files));
            let pre = capture.pre_binding.as_ref().unwrap();
            let var = |owner: &str, owner_line, line, name: &str, access, bytes| Endpoint {
                file: path.clone(),
                owner: owner.into(),
                owner_line,
                line,
                name: name.into(),
                access,
                bytes,
            };
            let (full, full_binding, write) = if conflict == "shadow" {
                ([169, 178], [135, 138], [150, 163])
            } else {
                ([146, 155], [63, 66], [127, 140])
            };
            let want = Replay {
                lines: vec![6, 7],
                root_line: 6,
                root: [107, 110],
                full: var("App", 4, 7, "box.value", Access::Use, full),
                base: var("App", 4, 6, "box", Access::Use, [83, 86]),
                def: var("App", 4, 5, "box", Access::Def, [63, 66]),
                dest: var("D", 1, 1, "a", Access::Def, [12, 13]),
                binding: [63, 66],
                full_binding,
                write,
                conflict: conflict.into(),
            };
            let start = source.find("<D").unwrap();
            let end = start + source[start..].find("/>").unwrap() + 2;
            let tag = format!("{id}/{case}/{ext}");
            let evidence = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prove_inputs(file, pre, &want);
            }));
            let (status, issue) = if evidence.is_err() {
                (
                    "UNMET",
                    Some("lookup/byte witness precondition failed".into()),
                )
            } else {
                let result = compare_replay(file, [start, end], &graph, pre, &want);
                (if result.is_ok() { "PASS" } else { "RED" }, result.err())
            };
            println!("S3_REPLAY {tag} {status} {issue:?}");
            if let Some(issue) = issue {
                errors.push(format!("{tag}: {issue}"));
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn fold_row(source: &str, ext: &str, path: &str, refusal: Option<&str>) -> Result<(), String> {
    fold_source_row(source, ext, path, false, refusal)
}

fn fold_source_row(
    source: &str,
    ext: &str,
    path: &str,
    own_occurrence: bool,
    refusal: Option<&str>,
) -> Result<(), String> {
    let file_name = format!("m.{ext}");
    let files =
        super::props_generated_tests::parse(&BTreeMap::from([(file_name.clone(), source.into())]));
    let file = &files[&file_name];
    assert_eq!(file.parse_error_count, 0, "{source}");
    let start = source.find("<D").unwrap();
    let end = start + source[start..].find("/>").unwrap() + 2;
    let (graph, observed) = capture(&files, || CodePropertyGraph::build(&files));
    let pre = observed.pre_binding.as_ref().unwrap();
    let site = observed
        .literal_attempts
        .iter()
        .find(|attempt| attempt.bytes == (start, end) && attempt.exact && attempt.refusal.is_none())
        .ok_or_else(|| format!("UNMET: no admission attempt at {start}..{end}"))?;
    let dest = matching(
        pre,
        &Endpoint {
            file: file_name.clone(),
            owner: "D".into(),
            owner_line: 1,
            line: 1,
            name: "a".into(),
            access: Access::Def,
            bytes: [13, 14],
        },
    )
    .expect("concrete destination Def");
    let line = source[..start].bytes().filter(|&ch| ch == b'\n').count() + 1;
    let want = Endpoint {
        file: file_name,
        owner: site.caller.name.clone(),
        owner_line: site.caller.start_line,
        line,
        name: path.into(),
        access: Access::Use,
        bytes: [0, 0],
    };
    let use_id = indexed(pre, &pre.live_lookup, &want, line)
        .ok_or_else(|| format!("UNMET: missing masked lookup for {want:?}"))?;
    let use_site = endpoint(&pre.graph[use_id]).unwrap();
    let [use_start, use_end] = use_site.bytes;
    let expected_bytes = if own_occurrence {
        use_start == start + "<D a={".len()
    } else {
        use_end <= start && source[..use_start].ends_with("observe(")
    };
    if !expected_bytes || &source[use_start..use_end] != path {
        return Err(format!(
            "UNMET: lookup selected unexpected bytes {use_site:?} (own={own_occurrence})"
        ));
    }
    let connected = pre
        .graph
        .edges_directed(use_id, petgraph::Incoming)
        .any(|edge| {
            use petgraph::visit::EdgeRef;
            edge.weight().is_data_flow()
                && endpoint(&pre.graph[edge.source()]).is_some_and(|def| {
                    def.access == Access::Def && def.owner == want.owner && def.name == path
                })
        });
    if !connected {
        return Err(format!("UNMET: disconnected masked lookup {use_site:?}"));
    }
    let pending = super::jsx_props_replay::replay(
        file,
        start..end,
        &graph,
        &pre.graph,
        &pre.live_lookup,
        dest,
        false,
    );
    let live: Vec<_> = graph
        .graph
        .edges_connecting(use_id, dest)
        .filter(|edge| edge.weight().is_data_flow())
        .map(|edge| edge.weight().clone())
        .collect();
    let stats = &graph.dfg_label_stats.dfg_props;
    let observed = |key: &str| stats.get(key).copied().unwrap_or(0);
    let expected_edge = CpgEdge::DataFlow(FlowConfidence::Exact);
    let expected = match refusal {
        Some(key) => {
            use petgraph::visit::EdgeRef;
            pending.is_empty()
                && !graph
                    .graph
                    .edges_directed(dest, petgraph::Incoming)
                    .any(|edge| {
                        edge.weight().is_data_flow()
                            && endpoint(&graph.graph[edge.source()]).is_some_and(|source| {
                                source.owner == want.owner && source.owner_line == want.owner_line
                            })
                    })
                && observed(key) == 1
        }
        None => {
            pending == [(use_id, dest, expected_edge.clone())]
                && live == [expected_edge]
                && observed("refused_source_ambiguous") == 0
                && observed("refused_source_unconnected") == 0
        }
    };
    if expected {
        Ok(())
    } else {
        Err(format!(
            "masked Use {use_site:?}, pending={pending:?}, live={live:?}, stats={stats:?}"
        ))
    }
}

#[test]
fn props_s3_fold_class_receivers() {
    let forms = [
        ("instance", "slot = JSX;"),
        ("static", "static slot = JSX;"),
        ("block", "static { observe(JSX); }"),
    ];
    let mut errors = Vec::new();
    for ext in ["jsx", "tsx"] {
        for receiver in ["this", "super"] {
            let path = format!("{receiver}.v");
            let jsx = format!("<D a={{{path}}} />");
            let make = |body: &str| {
                format!("function D({{ a }}) {{ return use(a); }}\nclass Outer extends Base {{\n run(secret) {{\n  {path} = secret;\n  {body}\n }}\n}}\n")
            };
            let mut check = |name: &str, body: &str, refusal| {
                let case = format!("{name}/{receiver}/{ext}");
                let result = fold_row(&make(body), ext, &path, refusal);
                println!("S3_FOLD {case} {result:?}");
                if let Err(error) = result {
                    errors.push(format!("{case}: {error}"));
                }
            };
            let refusal = Some("refused_source_unconnected");
            for (name, template) in forms {
                let field = template.replace("JSX", &jsx);
                check(
                    name,
                    &format!("observe({path}); class C extends Base {{ {field} }}"),
                    refusal,
                );
                let arrow = format!("(secret) => {{\n    {path} = secret;\n    observe({path}); return {jsx};\n  }}");
                let field = if name == "block" {
                    format!("static {{ const slot = {arrow}; }}")
                } else {
                    template.replace("JSX", &arrow)
                };
                check(
                    &format!("{name}-arrow"),
                    &format!("class C extends Base {{ {field} }}"),
                    refusal,
                );
            }
            let ambiguous = Some("refused_source_ambiguous");
            check(
                "method",
                &format!("observe({path}); return {jsx};"),
                ambiguous,
            );
            check(
                "key",
                &format!("observe({path}); class C {{ [observe({jsx})] = 0; }}"),
                ambiguous,
            );
            check(
                "heritage",
                &format!("observe({path}); class C extends observe({jsx}) {{}}"),
                ambiguous,
            );
            for body in [
                format!("return {jsx};"),
                format!("class C {{ [observe({jsx})] = 0; }}"),
                format!("class C extends observe({jsx}) {{}}"),
            ] {
                let result = fold_source_row(&make(&body), ext, &path, true, None);
                println!("S3_FOLD own-receiver/{body}/{ext} {result:?}");
                if let Err(error) = result {
                    errors.push(format!("own-receiver/{body}/{ext}: {error}"));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn props_s3_fold_destructuring_order() {
    let forms = [
        ("array-default", "[x, y = JSX] = [0, undefined];"),
        (
            "object-default",
            "({ first: x, second: y = JSX } = { first: 0 });",
        ),
        (
            "computed-key",
            "({ first: x, [observe(JSX)]: y } = { first: 0 });",
        ),
        ("computed-target", "[x, box[observe(JSX)]] = [0, 1];"),
        ("rhs", "[x, y] = [0, JSX];"),
        ("before-store", "[y = JSX, x] = [undefined, 0];"),
        ("own-default", "[x = JSX, y] = [undefined, 0];"),
        ("nested-own-default", "[[x] = [JSX], y] = [undefined, 0];"),
        ("disjoint", "[z, y = JSX] = [0, undefined];"),
    ];
    let mut errors = Vec::new();
    for ext in ["jsx", "tsx"] {
        for (index, (name, template)) in forms.into_iter().enumerate() {
            let assignment = template.replace("JSX", "<D a={x} />");
            let source = format!(
                "function D({{ a }}) {{ return use(a); }}\nfunction App(x, box) {{\n let y, z;\n observe(x); {assignment}\n return y;\n}}\n"
            );
            let case = format!("{name}/{ext}");
            let refusal = (name != "disjoint").then_some("refused_source_ambiguous");
            let result = fold_row(&source, ext, "x", refusal);
            println!("S3_FOLD {case} {result:?}");
            if let Err(error) = result {
                errors.push(format!("{case}: {error}"));
            }
            let own_source = source
                .replace("observe(x); ", "")
                .replace("<D a={x} />", "keep(<D a={x} />)");
            let refusal = (index < 4).then_some("refused_source_ambiguous");
            let result = fold_source_row(&own_source, ext, "x", true, refusal);
            println!("S3_FOLD own-{case} {result:?}");
            if let Err(error) = result {
                errors.push(format!("own-{case}: {error}"));
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn props_s3_fold_owner_write_review_cases() {
    let forms = [
        (
            "array-default",
            "x",
            "observe(x); [x = 0] = [5]; return JSX;",
        ),
        (
            "array-taken-default",
            "x",
            "observe(x); [x = 0] = [undefined]; return JSX;",
        ),
        (
            "object-shorthand-default",
            "x",
            "observe(x); ({ x = 0 } = { x: 5 }); return JSX;",
        ),
        (
            "nested-array-default",
            "x",
            "observe(x); [[x = 0]] = [[5]]; return JSX;",
        ),
        (
            "non-first-array-default",
            "x",
            "observe(x); [y, x = 0] = [1, 5]; return JSX;",
        ),
        (
            "nested-object-default",
            "x",
            "observe(x); ({ k: [x = 0] } = { k: [5] }); return JSX;",
        ),
        (
            "iteration-array-default",
            "x",
            "observe(x); for ([x = 0] of values) {} return JSX;",
        ),
        (
            "iteration-object-default",
            "x",
            "observe(x); for ({ x = 0 } of values) {} return JSX;",
        ),
        (
            "member-base",
            "x",
            "observe(x); [x.y] = [5]; return <D a={x.y} />;",
        ),
        (
            "projected-member",
            "x.y",
            "observe(x.y); [x.y] = [0]; return JSX;",
        ),
        (
            "bare-array-member",
            "x",
            "observe(x); [x.y] = [5]; return JSX;",
        ),
        ("direct-member", "x.y", "observe(x.y); x.y = 5; return JSX;"),
        ("plain-array", "x", "observe(x); [x] = [5]; return JSX;"),
        (
            "object-shorthand",
            "x",
            "observe(x); return JSX; ({x} = values);",
        ),
        (
            "pair-default",
            "x",
            "observe(x); ({k: x = 0} = {}); return JSX;",
        ),
        (
            "assignment-rhs-first",
            "x",
            "observe(x); [y = JSX] = (x = 0, [undefined]);",
        ),
        (
            "declaration-rhs-first",
            "x",
            "observe(x); const [z = JSX] = (x = 0, [undefined]);",
        ),
        (
            "iteration-target-order",
            "x",
            "for ([r = observe(x), x, y = JSX] of [[undefined, 0, undefined]]) {}",
        ),
        (
            "static-after-key",
            "x",
            "observe(x); class C { static slot = JSX; [x = 0]() {} }",
        ),
        (
            "deferred-instance",
            "x",
            "observe(x); class C { slot = JSX; } x = 0; return new C().slot;",
        ),
        (
            "disjoint-branch",
            "x",
            "if (flag) { observe(x); return JSX; } else { x = 0; }",
        ),
        (
            "nested-capture-store",
            "x",
            "observe(x); const mutate = () => { x = 0; }; return JSX;",
        ),
        ("before-both-reads", "x", "x = 0; observe(x); return JSX;"),
        ("compound", "x", "observe(x); return JSX; x += 1;"),
        ("update", "x", "observe(x); return JSX; ++x;"),
        (
            "delete",
            "x",
            "observe(x); return <D a={x.y} />; delete x.y;",
        ),
        (
            "bracket-member",
            "x",
            "observe(x); return <D a={x.y} />; [x[k]] = [5];",
        ),
        (
            "escaped-member",
            "x",
            r"observe(x); return <D a={x.y} />; [x.\u0079] = [5];",
        ),
        (
            "rest-target",
            "x",
            "observe(x); return <D a={x.y} />; [...x.y] = values;",
        ),
        (
            "for-in-target",
            "x",
            "observe(x); return <D a={x.y} />; for (x.y in values) {}",
        ),
        // Refused by Rule R (P2) by spelling, though none writes the operand's own binding:
        // a second declaration of the spelling (a2), a sibling-field store (b2, not
        // field-sensitive) and a computed key (b2, off the read list). Priced cost, PRICE.md.
        (
            "declaring-iteration",
            "x",
            "observe(x); return JSX; for (let x of values) {}",
        ),
        (
            "nested-shadow",
            "x",
            "observe(x); const mutate = (x) => { x = 0; }; return JSX;",
        ),
        (
            "sibling-path",
            "x",
            "observe(x); [x.z] = [5]; return <D a={x.y} />;",
        ),
        (
            "pattern-key-is-read",
            "x",
            "observe(x); ({ [x]: y = x } = {}); return JSX;",
        ),
    ];
    let controls = [
        ("no-write", "x", "observe(x); return JSX;"),
        ("other-root", "x", "observe(x); return JSX; undefined = 0;"),
        // No token spells `x`: a store through an unknown target cannot be a store through `x`.
        (
            "unknown-delete-target",
            "x",
            "observe(x); return JSX; delete get();",
        ),
        (
            "other-member-root",
            "x",
            "observe(x); return JSX; undefined.y = 0;",
        ),
        (
            "call-result-member",
            "x",
            "observe(x); return JSX; get().x = 0;",
        ),
        (
            "other-function",
            "x",
            "observe(x); return JSX;\n}\nfunction Other(x) { x = 0;",
        ),
        (
            "same-phase",
            "x",
            "class C { static { observe(x); take(JSX); } }",
        ),
    ];
    let mut errors = Vec::new();
    for ext in ["jsx", "tsx"] {
        for (rows, refusal) in [
            (forms.as_slice(), Some("refused_source_ambiguous")),
            (controls.as_slice(), None),
        ] {
            for (name, path, body) in rows {
                let body = body.replace("JSX", &format!("<D a={{{path}}} />"));
                let caller = if *path == "x.y" {
                    "function App(seed) {\n const x = make();\n x.y = seed;"
                } else {
                    "function App(x) {\n let r, y;"
                };
                let source =
                    format!("function D({{ a }}) {{ return use(a); }}\n{caller}\n {body}\n}}\n");
                let result = fold_row(&source, ext, path, refusal);
                println!("S3_RULE_R {name}/{ext} {result:?}");
                if let Err(error) = result {
                    errors.push(format!("{name}/{ext}: {error}"));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
