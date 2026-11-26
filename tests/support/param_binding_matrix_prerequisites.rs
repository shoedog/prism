//! Base probes for the source prerequisites and their independent controls.

use super::*;
use crate::ast::ParsedFile;
use crate::call_graph::CallSiteOrigin;

struct ConstructorCase {
    id: &'static str,
    expression: &'static str,
    implementation: &'static str,
    ordinals: &'static str,
    arguments: &'static [&'static str],
}

const CONSTRUCTORS: &[ConstructorCase] = &[
    ConstructorCase {
        id: "t4-3",
        expression: "new C(x, y)",
        implementation: "C",
        ordinals: "2; fixed-loss 1",
        arguments: &["x", "y"],
    },
    ConstructorCase {
        id: "s2-4",
        expression: "new Packet(label, count, text)",
        implementation: "Packet",
        ordinals: "2,3; fixed-loss 1",
        arguments: &["label", "count", "text"],
    },
    ConstructorCase {
        id: "s3-3",
        expression: "new Derived(first, last)",
        implementation: "Derived",
        ordinals: "1",
        arguments: &["first", "last"],
    },
    ConstructorCase {
        id: "s1-25",
        expression: "new Runner(handler, input)",
        implementation: "Runner",
        ordinals: "1,2",
        arguments: &["handler", "input"],
    },
    ConstructorCase {
        id: "s5-6",
        expression: "new Derived(0, value)",
        implementation: "Base",
        ordinals: "2",
        arguments: &["0", "value"],
    },
    ConstructorCase {
        id: "x3-5",
        expression: "new Explicit(lost)",
        implementation: "Explicit",
        ordinals: "1",
        arguments: &["lost"],
    },
    ConstructorCase {
        id: "x3-5",
        expression: "new Implicit(kept)",
        implementation: "Base",
        ordinals: "2",
        arguments: &["kept"],
    },
    ConstructorCase {
        id: "x5-3",
        expression: "new Shape(direct)",
        implementation: "Shape",
        ordinals: "3; historical skip",
        arguments: &["direct"],
    },
];

#[test]
fn constructor_sites_and_supplied_inputs() {
    let rows = read_rows(ROWS).unwrap();
    for case in CONSTRUCTORS {
        let row = rows.iter().find(|r| r.id == case.id).unwrap();
        let (fixture_name, source) = row.files.iter().next().unwrap();
        let start = source.find(case.expression).unwrap();
        let end = start + case.expression.len();
        for ext in &row.langs {
            let fixture = row.fixture(ext);
            let (_, file) = fixture.file(fixture_name).unwrap();
            let parsed = ParsedFile::parse(&file, source, language(ext).unwrap()).unwrap();
            let node = parsed
                .tree
                .root_node()
                .named_descendant_for_byte_range(start, end)
                .unwrap();
            assert_eq!(
                (node.kind(), node.byte_range()),
                ("new_expression", start..end)
            );
            let args = node.child_by_field_name("arguments").unwrap();
            let arguments: Vec<_> = args
                .named_children(&mut args.walk())
                .map(|arg| (parsed.node_text(&arg), arg.byte_range()))
                .collect();
            assert_eq!(
                arguments.iter().map(|(text, _)| *text).collect::<Vec<_>>(),
                case.arguments
            );
            let constructor = parsed
                .functions()
                .iter()
                .find(|f| {
                    f.owner.as_deref() == Some(case.implementation)
                        && f.name.as_deref() == Some("constructor")
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{} [{ext}]: no {} constructor declaration",
                        case.id, case.implementation
                    )
                });
            let caller = parsed
                .functions()
                .iter()
                .find(|f| {
                    f.name.as_deref() == Some("main") && f.start_byte <= start && f.end_byte >= end
                })
                .unwrap();
            let sites: Vec<_> = fixture
                .cpg
                .call_graph
                .calls
                .values()
                .flatten()
                .filter(|site| {
                    site.caller.file == file && site.start_byte == start && site.end_byte == end
                })
                .map(|site| {
                    (
                        site.origin,
                        &site.callee_name,
                        fixture.cpg.call_graph.resolve_call_site_full(site),
                    )
                })
                .collect();
            let indexed = parsed.call_argument_texts_and_spans_at(start, "constructor");
            println!(
                "CONSTRUCTOR\t{}\t{ext}\t{}\tSource {file} {start}..{end} main@{}\tExact {}.constructor@{}-{} {}..{}\t{sites:?}\t{arguments:?}\t{indexed:?}",
                case.id, case.ordinals, caller.start_line, case.implementation, constructor.start_line,
                constructor.end_line, constructor.start_byte, constructor.end_byte,
            );
        }
    }
}

#[test]
fn independent_constructor_controls() {
    let rows = read_rows(ROWS).unwrap();
    for (id, ordinals) in [
        ("t4-3", &[3][..]),
        ("s2-4", &[4]),
        ("s3-3", &[2, 3, 4]),
        ("s5-6", &[1]),
        ("x5-3", &[2, 4]),
    ] {
        let row = rows.iter().find(|r| r.id == id).unwrap();
        for ext in &row.langs {
            let fixture = row.fixture(ext);
            for &ordinal in ordinals {
                let (matches, actual) = fixture
                    .check(&row.expect[ordinal - 1].check)
                    .unwrap_or_else(|error| panic!("{id} [{ext}] #{ordinal}: {error}"));
                assert!(matches, "{id} [{ext}] #{ordinal}: {actual}");
                println!("CONSTRUCTOR-CONTROL\t{id}\t{ext}\t{ordinal}\tpass\t{actual}");
            }
        }
    }
}

#[test]
fn holder_admission_pins() {
    let rows = read_rows(ROWS).unwrap();
    let map: Check = serde_json::from_str(
        r#"{
        "kind":"resolves","call":{"site":"m.js@12:1","callee":"map","end":"12:17","owner":"main"},
        "targets":["m.js:map@2"],"confidence":"exact","route":"constructor_local"
    }"#,
    )
    .unwrap();
    for (id, check) in [
        ("s4-13", &map),
        (
            "s2-15",
            &rows.iter().find(|r| r.id == "s2-15").unwrap().expect[3].check,
        ),
    ] {
        let row = rows.iter().find(|r| r.id == id).unwrap();
        for ext in &row.langs {
            let (matches, actual) = row
                .fixture(ext)
                .check(check)
                .unwrap_or_else(|error| panic!("{id} [{ext}]: {error}"));
            println!("HOLDER\t{id}\t{ext}\t{actual}");
            assert!(matches, "{id} [{ext}]: {actual}");
        }
    }
    let row = rows.iter().find(|r| r.id == "x4-18").unwrap();
    let fixture = row.fixture("tsx");
    let site = Site {
        site: "m.tsx@21:3".into(),
        callee: "Slot".into(),
        end: None,
        owner: Some("main".into()),
    };
    let ground = fixture
        .site(&site)
        .ok()
        .filter(|site| site.origin == CallSiteOrigin::Source)
        .map(|site| fixture.cpg.call_graph.resolve_call_site_full(site));
    let ground = ground.expect("x4-18 JSX must be an extracted Source site");
    let [target] = ground.resolved.as_slice() else {
        panic!("x4-18 JSX must have one target: {ground:?}");
    };
    assert_eq!(
        (
            &*target.target.file,
            &*target.target.name,
            target.target.start_line,
            target.target.end_line
        ),
        ("m.tsx", "Slot", 3, 7)
    );
    assert_eq!(
        target.confidence,
        crate::resolution::ResolutionConfidence::Exact
    );
    println!("HOLDER\tx4-18\ttsx\t{target:?}");
}

#[test]
fn s5_5_foreign_name_only_rest() {
    let row = serde_json::json!({
          "id":"s5-5-foreign","group":"B","langs":["js","ts","tsx"],
          "class":"review-companion","truth":"correct",
          "files":{
            "m.js":"import { Box } from './box';\nfunction main(receiver, xs) {\n  const value = read();\n  receiver.pack(...xs, 0, value); new Box().pack(0, 0);\n}\n",
            "box.js":"export class Box {\n  pack(first, ...tail) {\n    use(first, tail);\n  }\n}\n"
          },
          "expect":[
            {"kind":"resolves","call":{"site":"m.js@4:3","callee":"pack","owner":"main"},"targets":["box.js:pack@2"],"confidence":"nameonly","route":"r6_single_owner"},
            {"kind":"edge","from":"m.js:main:value@4:27","to":"box.js:pack:tail@2:18","label":"nameonly/call_nameonly","multiplicity":1},
            {"kind":"no_edge","from":"m.js:main:value@4:27","to":"box.js:pack:tail@2:18","label":"exact"},
            {"kind":"trace_boundary","root":"m.js:main:value@3:9","from":"m.js:main:value@4:27","to":"box.js:pack:tail@2:18"},
            {"kind":"reaches","from":"m.js:main:value@3:9","to":"box.js:pack:tail@3:16"}
          ]
    });
    let rows = read_rows(&row.to_string()).unwrap();
    let failures = matrix_failures(&rows, ACTIVE_SLICES, &[]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn reviewed_companions() {
    let mut rows = read_rows(r#"{"id":"x3-8-indented","group":"D","langs":["js","ts","tsx"],"class":"review-companion","truth":"correct","files":{"m.js":"function visit(item) {\n  use(item);\n}\nfunction main() {\n  const items = [1];\n  const direct = 2;\n  items.forEach(visit); visit(direct);\n}\n"},"expect":[{"kind":"edge","from":"m.js:main:items@7:3","to":"m.js:visit:item@1:16","label":"nameonly/builtin_callback","loss":"Indented receiver has only a line-start Use"},{"kind":"no_edge_into","to":"m.js:visit:item@1:16","label":"nameonly/builtin_callback"},{"kind":"edge","from":"m.js:main:direct@7:31","to":"m.js:visit:item@1:16","label":"exact","multiplicity":1},{"kind":"reaches","from":"m.js:main:direct@6:9","to":"m.js:visit:item@2:7"}]}
{"id":"s2-15-admitted","group":"E","langs":["js","ts","tsx"],"class":"review-companion","truth":"correct","files":{"m.js":"class Thenable {\n  then(ok, fail) {\n    const reason = read();\n    fail(reason);\n  }\n}\nfunction fulfilled(value) {\n  use(value);\n}\nfunction rejected(reason) {\n  use(reason);\n}\nfunction main() {\n  const thenable = new Thenable();\n  thenable.then(fulfilled, rejected);\n}\n"},"expect":[{"kind":"resolves","call":{"site":"m.js@15:3","callee":"then","owner":"main"},"targets":["m.js:then@2"],"confidence":"exact","route":"constructor_local"},{"kind":"resolves","call":{"site":"m.js@4:5","callee":"fail","owner":"then"},"targets":["m.js:rejected@10"],"confidence":"nameonly","route":"argument_follow"},{"kind":"edge","from":"m.js:then:reason@4:10","to":"m.js:rejected:reason@10:19","label":"nameonly/call_nameonly","multiplicity":1},{"kind":"no_edge","from":"m.js:then:reason@4:10","to":"m.js:rejected:reason@10:19","label":"exact"}]}"#).unwrap();
    rows.extend(descriptor_companions());
    let mut failures = Vec::new();
    for row in &rows {
        if row.id == "x3-8-indented" && ACTIVE_SLICES.contains(&"D") {
            for ext in &row.langs {
                let fixture = row.fixture(ext);
                let uses: Vec<_> = fixture
                    .cpg
                    .nodes_at(&format!("m.{ext}"), 7)
                    .into_iter()
                    .filter_map(|node| fixture.cpg.to_var_location(node))
                    .filter(|loc| {
                        loc.function == "main"
                            && loc.kind == VarAccessKind::Use
                            && loc.path.to_string().split('.').next() == Some("items")
                    })
                    .map(|loc| (loc.start_byte, loc.end_byte))
                    .collect();
                let (_, start) = byte_at(&row.files["m.js"], "7:1").unwrap();
                assert_eq!(uses, [(start, start)], "{} [{ext}]", row.id);
            }
        }
        evaluate(
            row,
            ACTIVE_SLICES,
            |ext, n, slice, assertion, outcome, actual| {
                let line = format!(
                    "{}\t{ext}\t{n}\t{slice}\t{}\t{:?}\t{actual}\t{}",
                    row.id,
                    outcome.label(),
                    assertion.check,
                    assertion.loss.as_deref().unwrap_or("")
                );
                println!("COMPANION\t{line}");
                if outcome.fails() {
                    failures.push(line);
                }
            },
        );
    }
    assert!(
        failures.is_empty(),
        "companion failures:\n{}",
        failures.join("\n")
    );
}
fn descriptor_companions() -> Vec<Row> {
    let getter = "get cb() { return handler; }";
    let setter = "set cb(value) { observe(value); }";
    [
        ("getter-then-setter", format!("{getter}, {setter}"), None),
        ("setter-then-getter", format!("{setter}, {getter}"), None),
        ("data-replaces-accessor", format!("{getter}, cb: handler"), Some(("handler", 10, 18))),
        ("accessor-replaces-data", format!("cb: handler, {setter}"), Some(("fallback", 4, 19))),
    ]
    .into_iter()
    .map(|(variant, fields, target)| {
        use serde_json::json;
        let source = format!(
            "function run({{ cb = fallback }}, input) {{\n  cb(input);\n}}\n\
             function fallback(item) {{\n  use(item);\n}}\n\
             function main(input) {{\n  run({{ {fields} }}, input);\n}}\n\
             function handler(item) {{ use(item); }}\n"
        );
        let call = json!({"site":"m.js@2:3","callee":"cb","owner":"run"});
        let mut checks = Vec::new();
        if let Some((name, line, col)) = target {
            checks.push(json!({
                "kind":"resolves", "call":call, "targets":[format!("m.js:{name}@{line}")],
                "confidence":"nameonly", "route":"argument_follow"
            }));
            checks.push(json!({
                "kind":"edge", "from":"m.js:run:input@2:6", "to":format!("m.js:{name}:item@{line}:{col}"),
                "label":"nameonly/call_nameonly","multiplicity":1
            }));
        } else {
            checks.push(json!({"kind":"drop","call":call,"reason":"LocalBindingUnproven"}));
        }
        for (name, endpoint) in [("handler", "m.js:handler:item@10:18"), ("fallback", "m.js:fallback:item@4:19")] {
            if target.is_none_or(|(wanted, _, _)| name != wanted) {
                checks.push(json!({"kind":"no_edge","from":"m.js:run:input@2:6","to":endpoint}));
            }
        }
        let row = json!({
            "id":format!("s5-27-{variant}"), "group":"E", "langs":["js","ts","tsx"], "class":"review-companion",
            "files":{"m.js":source}, "truth":"correct", "expect":checks
        });
        read_rows(&row.to_string()).unwrap().pop().unwrap()
    }).collect()
}

#[test]
fn receiver_byte_pins() {
    let mut rows = read_rows(ROWS).unwrap();
    for (id, receiver, line) in [("h11", "arr", 3), ("x2-9", "batches", 9)] {
        let row = rows.iter_mut().find(|r| r.id == id).unwrap();
        let source = &row.files["m.js"];
        let line_text = source.lines().nth(line - 1).unwrap();
        let call = line_text.trim_start();
        let source_variants =
            [format!("  {call}"), call.to_owned()].map(|text| source.replace(line_text, &text));
        for (indented, source) in [true, false].into_iter().zip(source_variants) {
            row.files.insert("m.js".to_owned(), source);
            for ext in &row.langs {
                let fixture = row.fixture(ext);
                let token = Endpoint::Plain(format!(
                    "m.js:main:{receiver}@{line}:{}",
                    if indented { 3 } else { 1 }
                ));
                let zero = Endpoint::Spanned {
                    at: format!("m.js:main:{receiver}@{line}:1"),
                    text: String::new(),
                };
                let token_nodes = fixture.nodes(&token, VarAccessKind::Use).unwrap();
                let zero_nodes = fixture.nodes(&zero, VarAccessKind::Use).unwrap();
                println!(
                    "RECEIVER\t{id}\t{ext}\tindented={indented}\ttoken={}\tline_start={}",
                    token_nodes.len(),
                    zero_nodes.len()
                );
                assert!(token_nodes.is_empty(), "{id} [{ext}]");
                assert_eq!(zero_nodes.len(), 1, "{id} [{ext}]");
            }
        }
    }
}
