use super::*;
use crate::algorithms::taint::{
    self, TaintConfig, TaintReached, TaintRootOrigin, TaintSeed, TaintTrace,
};
use prism::{
    access_path::AccessPath,
    call_graph::CallGraph,
    cpg::CpgContext,
    data_flow::VarAccessKind,
    diff::{DiffInfo, DiffInput, ModifyType},
    finding_confidence::EvidencePath,
    languages::Language,
};
use std::collections::BTreeMap;

fn parse(source: &str) -> ParsedFile {
    ParsedFile::parse("a.js", source, Language::JavaScript).unwrap()
}
fn express(body: &str, named: bool) -> String {
    let header = "const express = require('express');\nconst app = express();\n";
    if named {
        format!("{header}function handler(req, res) {{\n{body}\n}}\napp.post('/run', handler);\n")
    } else {
        format!("{header}app.post('/run', (req, res) => {{\n{body}\n}});\n")
    }
}
fn trace(parsed: &ParsedFile) -> TaintTrace {
    assert_eq!(parsed.parse_error_count, 0);
    let files = BTreeMap::from([(parsed.path.clone(), parsed.clone())]);
    let ctx = CpgContext::build(&files, None);
    let line = parsed
        .source
        .lines()
        .position(|line| line.contains("SOURCE"))
        .unwrap_or(0)
        + 1;
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: parsed.path.clone(),
            modify_type: ModifyType::Modified,
            diff_lines: [line].into(),
        }],
    };
    taint::slice_with_trace(
        &ctx,
        &diff,
        &TaintConfig {
            taint_from_diff: false,
            ..Default::default()
        },
    )
    .unwrap()
    .1
}
fn send(parsed: &ParsedFile) -> CallSite {
    CallGraph::build(&BTreeMap::from([(parsed.path.clone(), parsed.clone())]))
        .calls
        .into_values()
        .flatten()
        .find(|call| call.callee_name == "send")
        .expect("actual graph send")
}
fn selected(trace: &TaintTrace, target: &str, call: &CallSite) -> TaintRoot {
    let mut root = trace
        .roots
        .iter()
        .find(|root| {
            root.seed.target.as_ref() == Some(&AccessPath::from_expr(target))
                && root.reached.iter().any(|r| {
                    r.location.line == call.line && r.location.start_byte == r.location.end_byte
                })
        })
        .expect("real empty reach from specified target")
        .clone();
    root.reached
        .retain(|r| r.location.line == call.line && r.location.start_byte == r.location.end_byte);
    assert!(!root.reached.is_empty());
    root
}
fn matches(ctx: &RecoveryContext<'_>, root: &TaintRoot, call: &CallSite) -> bool {
    let before = format!("{root:?}");
    let prepared = ctx.prepare(root);
    let result = (0..root.reached.len()).any(|i| {
        prepared
            .value(i)
            .is_some_and(|v| ctx.matches_argument(call, v))
    });
    assert_eq!(before, format!("{root:?}"));
    result
}
fn alias() -> (ParsedFile, TaintRoot, CallSite) {
    let parsed = parse(&express(
        "  const command = req.query.command; // SOURCE\n  send(command);",
        false,
    ));
    let call = send(&parsed);
    let root = selected(&trace(&parsed), "command", &call);
    (parsed, root, call)
}

#[test]
fn all_six_real_field_sources_recover_the_specific_root() {
    for (body, named, target) in [
        ("  send(req.body); // SOURCE", false, "req"),
        ("  send(req.body); // SOURCE", true, "req"),
        ("  send(req.query.command); // SOURCE", false, "req"),
        ("  send(req.query.command); // SOURCE", true, "req"),
        (
            "  req.body = req.query.command; // SOURCE\n  send(req.body);",
            false,
            "req.body",
        ),
        (
            "  req.query.command = req.body; // SOURCE\n  send(req.query.command);",
            false,
            "req.query.command",
        ),
    ] {
        let parsed = parse(&express(body, named));
        let call = send(&parsed);
        let root = selected(&trace(&parsed), target, &call);
        assert_eq!(root.location.path, AccessPath::from_expr(target));
        if target != "req" {
            assert_eq!(root.seed.start_byte, Some(94));
        }
        let ctx = RecoveryContext::new(&parsed);
        assert!(matches(&ctx, &root, &call), "{target}, named={named}");
        let prepared = ctx.prepare(&root);
        let value = prepared.value(0).unwrap();
        assert_eq!(value.path, AccessPath::simple("req"));
        assert_eq!(&parsed.source[value.start_byte..value.end_byte], "req");
    }
}

#[test]
fn real_aliases_preserve_scope_and_count_all_competitors() {
    for (body, expected) in [
        (
            "  const command = req.query.command; // SOURCE\n  send(command);",
            true,
        ),
        (
            "  {\n    const command = req.query.command; // SOURCE\n    send(command);\n  }",
            true,
        ),
        (
            "  const command = req.query.command; // SOURCE\n  send(command, command);",
            false,
        ),
        (
            "  const command = req.query.command; send(command); // SOURCE",
            false,
        ),
    ] {
        let parsed = parse(&express(body, false));
        let call = send(&parsed);
        let root = selected(&trace(&parsed), "command", &call);
        let ctx = RecoveryContext::new(&parsed);
        assert_eq!(matches(&ctx, &root, &call), expected, "{body}");
        let mut anchored = root.clone();
        anchored.reached[0].location.start_byte = parsed.line_start_byte(call.line);
        anchored.reached[0].location.end_byte = anchored.reached[0].location.start_byte;
        assert_eq!(
            matches(&ctx, &anchored, &call),
            expected,
            "nonzero empty anchor"
        );
    }
}

#[test]
fn synthetic_bad_source_and_reached_metadata_refuse() {
    let (parsed, root, call) = alias();
    let ctx = RecoveryContext::new(&parsed);
    let changes: &[fn(&mut TaintRoot)] = &[
        |r| r.seed.file = "else.js".into(),
        |r| r.location.file = "else.js".into(),
        |r| r.seed.line += 1,
        |r| r.origin = TaintRootOrigin::Diff,
        |r| r.location.kind = VarAccessKind::Use,
        |r| r.location.start_byte = 1,
        |r| r.location.path = AccessPath::simple("wrong"),
        |r| r.seed.start_byte = Some(usize::MAX),
        |r| r.seed.scope = Some((9, 2)),
        |r| r.seed.scope = Some((1, usize::MAX)),
        |r| r.seed.byte_scope = Some((9, 2)),
        |r| r.seed.byte_scope = Some((0, usize::MAX)),
        |r| r.reached[0].location.path = AccessPath::from_expr("command.field"),
        |r| r.reached[0].location.file = "else.js".into(),
        |r| r.reached[0].location.function = "else".into(),
        |r| r.reached[0].location.start_byte = 1,
        |r| {
            r.reached[0].location.start_byte = usize::MAX;
            r.reached[0].location.end_byte = usize::MAX;
        },
        |r| r.reached[0].location.kind = VarAccessKind::Def,
    ];
    for (i, change) in changes.iter().enumerate() {
        let mut altered = root.clone();
        change(&mut altered);
        assert!(!matches(&ctx, &altered, &call), "synthetic case {i}");
    }
}

#[test]
fn synthetic_scopes_are_root_specific_and_never_select_a_competitor() {
    let (parsed, root, call) = alias();
    let ctx = RecoveryContext::new(&parsed);
    assert!(matches(&ctx, &root, &call));
    let mut short = root.clone();
    short.seed.scope = Some((root.seed.line, root.seed.line));
    assert!(!matches(&ctx, &short, &call));
    short = root.clone();
    short.seed.byte_scope = Some((0, parsed.line_start_byte(call.line)));
    assert!(!matches(&ctx, &short, &call));
    let parsed = parse(&express(
        "  const command = req.query.command; // SOURCE\n  send(command, command);",
        false,
    ));
    let call = send(&parsed);
    let mut root = selected(&trace(&parsed), "command", &call);
    root.seed.byte_scope = Some((0, parsed.source.rfind("command").unwrap()));
    assert!(!matches(&RecoveryContext::new(&parsed), &root, &call));
}

// Injected source classification and empty metadata; never claimed as upstream reachability.
fn synthetic(
    parsed: &ParsedFile,
    call: &CallSite,
    anchor: &str,
    target: &str,
    reached: &str,
) -> TaintRoot {
    let start = parsed.source.find(anchor).unwrap();
    let line = parsed.line_for_byte(start);
    let location = VarLocation {
        file: parsed.path.clone(),
        function: call.caller.name.clone(),
        function_start_line: call.caller.start_line,
        line,
        path: AccessPath::from_expr(target),
        start_byte: 0,
        end_byte: 0,
        kind: VarAccessKind::Def,
    };
    let mut value = location.clone();
    value.line = call.line;
    value.path = AccessPath::simple(reached);
    value.kind = VarAccessKind::Use;
    TaintRoot {
        seed: TaintSeed {
            file: parsed.path.clone(),
            line,
            target: Some(location.path.clone()),
            start_byte: Some(start),
            scope: None,
            byte_scope: None,
        },
        origin: TaintRootOrigin::Framework,
        location,
        reached: vec![TaintReached {
            location: value,
            evidence: EvidencePath::default(),
        }],
    }
}

#[test]
fn synthetic_anchors_require_real_complete_static_syntax() {
    for (statement, anchor, target, expected) in [
        ("req.body = input;", "req.body", "req.body", true),
        (
            "req.query.command = input;",
            "req.query.command",
            "req.query.command",
            true,
        ),
        ("req.other = input;", "req.other", "req.body", false),
        ("req.body.more = input;", "req.body", "req.body", false),
        ("req['body'] = input;", "req[", "req.body", false),
        ("read(req?.body);", "req?", "req.body", false),
        ("r\\u0065q.body = input;", "r\\u0065q", "req.body", false),
        ("const s = 'req.body';", "req.body", "req.body", false),
        ("/* req.body */", "req.body", "req.body", false),
    ] {
        let parsed = parse(&format!(
            "const z = 0;\nfunction h(req) {{\n {statement}\n send(req);\n}}"
        ));
        assert_eq!(parsed.parse_error_count, 0);
        let call = send(&parsed);
        let root = synthetic(&parsed, &call, anchor, target, "req");
        let ctx = RecoveryContext::new(&parsed);
        assert_eq!(matches(&ctx, &root, &call), expected, "{statement}");
        let mut wrong_field = root.clone();
        wrong_field.location.path = AccessPath::from_expr("req.different");
        assert!(!matches(&ctx, &wrong_field, &call), "full source target");
        let mut unanchored = root.clone();
        unanchored.seed.start_byte = None;
        assert!(!matches(&ctx, &unanchored, &call), "unanchored field");
    }
}

#[test]
fn synthetic_before_source_and_sibling_branch_refuse() {
    for source in [
        "const z = 0;\nfunction h(req) {\n send(req);\n req.body = input;\n}",
        "const z = 0;\nfunction h(req) {\n if (flag) { req.body = input; } else {\n send(req);\n }\n}",
    ] {
        let parsed = parse(source); let call = send(&parsed);
        let mut root = synthetic(&parsed, &call, "req.body", "req.body", "req");
        if call.line < root.seed.line { root.seed.target = Some(AccessPath::simple("req")); root.location.path = AccessPath::simple("req"); root.seed.start_byte = None; }
        assert!(!matches(&RecoveryContext::new(&parsed), &root, &call));
    }
}

#[test]
fn targetless_def_proof_is_not_a_positive_value_requirement() {
    let parsed = parse("const z=0;\nfunction h(req) {\n send(req);\n}");
    let call = send(&parsed);
    let mut root = synthetic(&parsed, &call, "req", "req", "req");
    let ctx = RecoveryContext::new(&parsed);
    let start = root.seed.start_byte.unwrap();
    root.seed.target = None;
    root.seed.start_byte = None;
    root.origin = TaintRootOrigin::Diff;
    root.location.start_byte = start;
    root.location.end_byte = start + 3;
    assert!(
        !ctx.facts
            .values(start..start + 3)
            .iter()
            .any(|fact| fact.semantic_value && fact.owner.is_some()),
        "parameter header is not an owned positive value fact"
    );
    assert!(matches(&ctx, &root, &call));
    root.location.end_byte = root.location.start_byte;
    assert!(!matches(&ctx, &root, &call));
    // Distinct source/reached names prevent census ambiguity from masking byte order.
    for (body, expected) in [
        ("const input=req.body; send(command);", true),
        ("send(command); const input=req.body;", false),
    ] {
        let parsed = parse(&format!("const z=0;\nfunction h(req) {{\n {body}\n}}"));
        let call = send(&parsed);
        let mut root = synthetic(&parsed, &call, "input", "input", "command");
        root.location.start_byte = root.seed.start_byte.take().unwrap();
        root.location.end_byte = root.location.start_byte + 5;
        root.seed.target = None;
        root.origin = TaintRootOrigin::Diff;
        assert_eq!(
            matches(&RecoveryContext::new(&parsed), &root, &call),
            expected,
            "{body}"
        );
    }
}

#[test]
fn failed_empty_proof_preserves_exact_cross_file_values_and_isolated_root_uses() {
    let parsed = parse("function h(x) { send(x.field); }");
    let call = send(&parsed);
    let mut root = synthetic(&parsed, &call, "x)", "x", "x");
    let start = parsed.source.find("x.field").unwrap();
    let value = VarLocation {
        start_byte: start,
        end_byte: start + 7,
        path: AccessPath::from_expr("x.field"),
        kind: VarAccessKind::Use,
        ..root.reached[0].location.clone()
    };
    root.seed.file = "different.js".into();
    root.location.file = "different.js".into();
    root.location.function = "different".into();
    root.reached[0].location = value.clone();
    let ctx = RecoveryContext::new(&parsed);
    assert!(
        matches(&ctx, &root, &call),
        "exact original must survive invalid empty proof"
    );
    root.location = value;
    root.reached.clear();
    assert!(
        ctx.matches_argument(&call, ctx.prepare(&root).root_value()),
        "isolated root Use survives"
    );
}

#[test]
fn synthetic_owner_ambiguity_refuses_before_call_selection() {
    let (_, _, template) = alias();
    for (path, text, name, start_line, source_line) in [
        (
            "a.js",
            "const z=0;\nfunction h(req) {} function h(req) {\n send(req);\n}",
            "h",
            2,
            3,
        ),
        (
            "a.js",
            "const z=0;\nconst a=[(req)=>{}, (req)=>{\n send(req);\n}];",
            "<anonymous>",
            2,
            3,
        ),
        (
            "a.py",
            "# prefix\ndef h(req):\n    thunk = lambda: send(req)\n",
            "h",
            2,
            3,
        ),
        ("a.js", "function h(req) {\n send(req);\n}", "h", 1, 2),
    ] {
        let parsed = ParsedFile::parse(path, text, Language::from_path(path).unwrap()).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let mut call = template.clone();
        call.caller.name = name.into();
        call.caller.start_line = start_line;
        call.line = source_line;
        let mut root = synthetic(&parsed, &call, "req", "req", "req");
        root.seed.start_byte = None;
        root.seed.line = source_line;
        root.location.line = source_line;
        let ctx = RecoveryContext::new(&parsed);
        assert!(ctx.facts.owners().len() >= 2);
        assert!(source::admit(&ctx, &root).is_none(), "{text}");
    }
}

#[test]
fn synthetic_unique_labels_and_callees_never_become_values() {
    let (_, _, template) = alias();
    let mut outcomes = Vec::new();
    let mut expected = Vec::new();
    for (path, text, wanted) in [
        (
            "a.js",
            "const z=0;\nfunction h(req) {\n send(req('safe'));\n}",
            (false, false),
        ),
        (
            "a.js",
            "const z=0;\nfunction h(req) {\n send({req: 'safe'});\n}",
            (false, false), // Collector adapter qualifies the inherited exact helper in .2c.
        ),
        (
            "a.ts",
            "const z=0;\nfunction h(req: R) {\n send(inner<req>('safe'));\n}",
            (false, false),
        ),
        (
            "a.py",
            "# prefix\ndef h(req):\n    send(req='safe')\n",
            (false, false),
        ),
        (
            "a.js",
            "const z=0;\nfunction h(req) {\n send({req});\n}",
            (true, true),
        ),
    ] {
        let parsed = ParsedFile::parse(path, text, Language::from_path(path).unwrap()).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let mut call = template.clone();
        call.caller.name = "h".into();
        call.caller.start_line = 2;
        call.line = 3;
        let mut root = synthetic(&parsed, &call, "req", "req", "req");
        root.seed.start_byte = None;
        let ctx = RecoveryContext::new(&parsed);
        let recovered = ctx.prepare(&root).value(0).is_some();
        let start = text.rfind("req").unwrap();
        let mut exact = root.reached[0].location.clone();
        exact.start_byte = start;
        exact.end_byte = start + 3;
        outcomes.push((recovered, ctx.matches_argument(&send(&parsed), &exact)));
        expected.push(wanted);
    }
    assert_eq!(outcomes, expected);
}
