//! #598: calls through a top-level relative `require` bind to the one callable the producer's
//! CommonJS export proves. The variant matrix: every admitted shape, and every refusal with the
//! outcome the call had before this route existed (pinned as `none`, so a regression that adds an
//! edge where main had none fails here).
use super::js_relative_extension_test::{graph, rows};
use prism::call_graph::{CallGraph, JsLocalBinding};

/// A fixed producer: `f` on line 1 and `g` on line 2 of `m.js`.
pub(super) const FG: &str = "function f(x) { return x; }\nfunction g(x) { return x; }\n";
const F_AT_M: &str = "m.js:f@1-1 Exact/import_member";
const G_AT_M: &str = "m.js:g@2-2 Exact/import_member";

/// What the call resolved to before this route: no record the route could have produced.
pub(super) const NONE: &[&str] = &["drop UnknownName"];

pub(super) struct Case {
    pub(super) id: &'static str,
    pub(super) files: Vec<(&'static str, String)>,
    pub(super) caller: &'static str,
    pub(super) callee: &'static str,
    pub(super) expect: Vec<&'static str>,
}

pub(super) fn case(
    id: &'static str,
    producer: &str,
    consumer: &str,
    callee: &'static str,
    expect: &[&'static str],
) -> Case {
    Case {
        id,
        files: vec![
            ("m.js", producer.to_owned()),
            ("app.js", consumer.to_owned()),
        ],
        caller: "run",
        callee,
        expect: expect.to_vec(),
    }
}

pub(super) fn run_cases(cases: Vec<Case>) {
    let mut failures = Vec::new();
    for c in &cases {
        let sources: Vec<(&str, &str)> = c.files.iter().map(|(p, s)| (*p, s.as_str())).collect();
        let cg = graph(&sources);
        let actual = if c.caller == "*" {
            rows_any_caller(&cg, c.callee)
        } else {
            rows(&cg, c.caller, c.callee)
        };
        if actual != c.expect {
            failures.push(format!(
                "{}: expected {:?}, got {:?}",
                c.id, c.expect, actual
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Like `rows`, for every caller (anonymous and module-level owners included).
fn rows_any_caller(cg: &CallGraph, callee: &str) -> Vec<String> {
    let mut out = Vec::new();
    for site in cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.callee_name == callee)
    {
        let res = cg.resolve_call_site_full(site);
        match res.drop {
            Some(drop) => out.push(format!("drop {drop:?}")),
            None => out.extend(res.resolved.iter().map(|r| {
                format!(
                    "{}:{}@{}-{} {:?}/{}",
                    r.target.file,
                    r.target.name,
                    r.target.start_line,
                    r.target.end_line,
                    r.confidence,
                    r.kind.as_str()
                )
            })),
        }
    }
    out
}

pub(super) fn app(decl: &str, call: &str) -> String {
    format!("{decl}\nfunction run() {{ {call}; }}\n")
}

#[test]
fn admitted_shapes_bind_exact() {
    let module_fn = format!("{FG}module.exports = f;\n");
    let object = format!("{FG}module.exports = {{ f, g }};\n");
    run_cases(vec![
        // 1. the module value is a declared function.
        case(
            "module-fn-const",
            &module_fn,
            &app("const m = require('./m');", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-var",
            &module_fn,
            &app("var m = require('./m');", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-let",
            &module_fn,
            &app("let m = require('./m');", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-js-suffix",
            &module_fn,
            &app("const m = require('./m.js');", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-double-quotes",
            &module_fn,
            &app("const m = require(\"./m\");", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-multi-declarator",
            &module_fn,
            &app("var a = 1, m = require('./m'), b = 2;", "m(1)"),
            "m",
            &[F_AT_M],
        ),
        case(
            "module-fn-hoisted-declaration",
            "module.exports = f;\nfunction f(x) { return x; }\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            &["m.js:f@2-2 Exact/import_member"],
        ),
        case(
            "module-fn-const-arrow",
            "const f = (x) => x;\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            &["m.js:f@1-1 Exact/import_member"],
        ),
        case(
            "module-fn-const-function-expression",
            "const f = function (x) { return x; };\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            &["m.js:f@1-1 Exact/import_member"],
        ),
        case(
            "module-fn-async",
            "async function f(x) { return x; }\nmodule.exports = f;\n",
            &app("const m = require('./m');", "m(1)"),
            "m",
            &["m.js:f@1-1 Exact/import_member"],
        ),
        // 2. a member of the module value.
        case(
            "member-shorthand",
            &object,
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-second-shorthand",
            &object,
            &app("const m = require('./m');", "m.g(1)"),
            "g",
            &[G_AT_M],
        ),
        case(
            "member-pair-same-name",
            &format!("{FG}module.exports = {{ f: f }};\n"),
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-pair-renamed-key",
            &format!("{FG}module.exports = {{ h: g }};\n"),
            &app("const m = require('./m');", "m.h(1)"),
            "h",
            &[G_AT_M],
        ),
        case(
            "member-string-key",
            &format!("{FG}module.exports = {{ 'f': f }};\n"),
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-module-exports-dot",
            &format!("{FG}module.exports.f = f;\n"),
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-exports-dot",
            &format!("{FG}exports.f = f;\n"),
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-exports-before-declaration",
            "exports.f = f;\nfunction f(x) { return x; }\n",
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &["m.js:f@2-2 Exact/import_member"],
        ),
        case(
            "member-named-function-expression",
            "exports.f = function f(x) { return x; };\n",
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &["m.js:f@1-1 Exact/import_member"],
        ),
        case(
            "member-const-arrow",
            "const f = (x) => x;\nexports.f = f;\n",
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &["m.js:f@1-1 Exact/import_member"],
        ),
        case(
            "member-inline-require",
            &object,
            &app("", "require('./m').f(1)"),
            "f",
            &[F_AT_M],
        ),
        // 3. a member bound once at the top.
        case(
            "member-alias-const",
            &object,
            &app("const f = require('./m').f;", "f(1)"),
            "f",
            &[F_AT_M],
        ),
        case(
            "member-alias-var-renamed-local",
            &object,
            &app("var local = require('./m').g;", "local(1)"),
            "local",
            &[G_AT_M],
        ),
        case(
            "member-alias-of-module-value-function",
            // `require('./m').f` where `m` also exports the member `f`.
            &format!("{FG}exports.f = f;\n"),
            &app("var f = require('./m').f;", "f(1)"),
            "f",
            &[F_AT_M],
        ),
    ]);
}

#[test]
fn parent_directory_target_binds() {
    run_cases(vec![Case {
        id: "member-parent-directory",
        files: vec![
            ("m.js", format!("{FG}module.exports = {{ f, g }};\n")),
            ("sub/app.js", app("const m = require('../m');", "m.f(1)")),
        ],
        caller: "run",
        callee: "f",
        expect: vec![F_AT_M],
    }]);
}

#[test]
fn directory_index_target_binds() {
    run_cases(vec![Case {
        id: "index-module",
        files: vec![
            ("lib/index.js", format!("{FG}module.exports = {{ f }};\n")),
            ("app.js", app("const lib = require('./lib');", "lib.f(1)")),
        ],
        caller: "run",
        callee: "f",
        expect: vec!["lib/index.js:f@1-1 Exact/import_member"],
    }]);
}

#[test]
fn typescript_and_cjs_extensions_bind() {
    run_cases(vec![
        Case {
            id: "ts-caller-ts-producer",
            files: vec![
                ("m.ts", format!("{FG}module.exports = f;\n")),
                ("app.ts", app("const m = require('./m');", "m(1)")),
            ],
            caller: "run",
            callee: "m",
            expect: vec!["m.ts:f@1-1 Exact/import_member"],
        },
        Case {
            id: "cjs-extension",
            files: vec![
                ("m.cjs", format!("{FG}module.exports = f;\n")),
                ("app.cjs", app("const m = require('./m.cjs');", "m(1)")),
            ],
            caller: "run",
            callee: "m",
            expect: vec!["m.cjs:f@1-1 Exact/import_member"],
        },
    ]);
}

#[test]
fn method_and_nested_function_callers_bind() {
    let producer = format!("{FG}module.exports = {{ f }};\n");
    run_cases(vec![
        Case {
            id: "class-method-caller",
            files: vec![
                ("m.js", producer.clone()),
                (
                    "app.js",
                    "const m = require('./m');\nclass K { run() { m.f(1); } }\n".to_owned(),
                ),
            ],
            caller: "run",
            callee: "f",
            expect: vec![F_AT_M],
        },
        Case {
            id: "nested-named-caller",
            files: vec![
                ("m.js", producer),
                (
                    "app.js",
                    "const m = require('./m');\nfunction outer() { function run() { m.f(1); } }\n"
                        .to_owned(),
                ),
            ],
            caller: "run",
            callee: "f",
            expect: vec![F_AT_M],
        },
    ]);
}

/// The rule by construction: a site bound `RequireModule` that the route does not prove behaves
/// exactly as its pre-change binding does (`MayCall("alias")` for a bare callee, `Unchecked` for
/// a member callee), for the resolver.
#[test]
fn unproven_require_sites_behave_as_before() {
    let sources: Vec<(&str, String)> = vec![
        ("m.js", format!("{FG}exports.f = f;\n")),
        ("n.js", "module.exports = function () {};\n".to_owned()),
        (
            "app.js",
            "const m = require('./m');\nconst n = require('./n');\nconst missing = require('./nope');\nfunction run() { m(1); m.zzz(2); n(3); n.f(4); missing(5); missing.f(6); require('./m').zzz(7); }\n"
                .to_owned(),
        ),
        // A decoy of the same spelling in another file must not become the target either.
        ("decoy.js", "function zzz() {}\nfunction missing() {}\nfunction n() {}\n".to_owned()),
    ];
    let refs: Vec<(&str, &str)> = sources.iter().map(|(p, s)| (*p, s.as_str())).collect();
    let cg: CallGraph = graph(&refs);
    let mut compared = 0;
    for site in cg.calls.values().flatten() {
        if !matches!(site.local_binding, JsLocalBinding::RequireModule { .. })
            || cg.js_ts_alias_route_hit(site)
        {
            continue;
        }
        let source = &sources
            .iter()
            .find(|(path, _)| *path == site.caller.file)
            .expect("caller file")
            .1;
        let text = &source[site.start_byte..site.end_byte];
        let mut before = site.clone();
        before.local_binding = if text.starts_with(&format!("{}(", site.callee_name)) {
            JsLocalBinding::MayCall("alias".to_owned())
        } else {
            JsLocalBinding::Unchecked
        };
        let summary = |o: &prism::resolution::ResolutionOutcome<'_>| {
            (
                o.resolved
                    .iter()
                    .map(|r| (r.target.clone(), r.confidence, r.kind))
                    .collect::<Vec<_>>(),
                o.drop,
            )
        };
        assert_eq!(
            summary(&cg.resolve_call_site_full(site)),
            summary(&cg.resolve_call_site_full(&before)),
            "site {text}"
        );
        compared += 1;
    }
    assert!(compared >= 6, "compared {compared} unproven sites");
}

#[test]
fn proven_sites_are_followed_and_counted_once() {
    let sources: Vec<(&str, String)> = vec![
        ("m.js", format!("{FG}module.exports = {{ h: g }};\n")),
        (
            "app.js",
            "const m = require('./m');\nfunction run() { m.h(1); }\n".to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = sources.iter().map(|(p, s)| (*p, s.as_str())).collect();
    let cg = graph(&refs);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "h")
        .expect("site");
    assert!(site.local_binding.is_followed());
    assert!(cg.js_ts_alias_route_hit(site));
    let targets = cg.js_ts_alias_route_outcome(site).expect("proven targets");
    assert_eq!(targets.len(), 1);
    assert_eq!(
        (targets[0].file.as_str(), targets[0].name.as_str()),
        ("m.js", "g")
    );
    // The renamed export is a caller edge of `g`, though the site is spelled `h`.
    let g = targets[0].clone();
    let edges = cg.resolved_caller_edges(&g);
    assert!(
        edges.iter().any(|e| e.caller.name == "run"),
        "edges into g: {edges:?}"
    );
}

/// `call-stats` counts an unproven bare callee under the `MayCall("alias")` reason it always had;
/// it tells bare from member callees by the site's qualifier.
#[test]
fn qualifier_tells_bare_from_member_callees() {
    let sources: Vec<(&str, String)> = vec![
        ("m.js", format!("{FG}exports.f = f;\n")),
        (
            "app.js",
            "const m = require('./m');\nconst f = require('./m').f;\nfunction run() { m(1); m.f(2); f(3); require('./m').f(4); }\n"
                .to_owned(),
        ),
    ];
    let refs: Vec<(&str, &str)> = sources.iter().map(|(p, s)| (*p, s.as_str())).collect();
    let cg = graph(&refs);
    let mut seen = 0;
    for site in cg.calls.values().flatten() {
        if !matches!(site.local_binding, JsLocalBinding::RequireModule { .. }) {
            continue;
        }
        let text = &sources[1].1[site.start_byte..site.end_byte];
        let bare = text.starts_with(&format!("{}(", site.callee_name));
        assert_eq!(site.qualifier.is_none(), bare, "{text}");
        seen += 1;
    }
    assert_eq!(seen, 4);
}
