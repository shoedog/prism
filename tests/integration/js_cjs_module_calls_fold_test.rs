//! #598, review round 1: the shapes the first review found. An inline `require` inside a `with`
//! body, a consumer that stubs a member of the module object, and the two ways a proven site
//! replaces what the site had before (a NameOnly rung record, indirect records). Every refusal is
//! pinned to the outcome the call had before the route existed.
use super::js_cjs_module_calls_test::{app, case, run_cases, Case, FG, NONE};
use super::js_relative_extension_test::{graph, rows};

const F_AT_M: &str = "m.js:f@1-1 Exact/import_member";

fn object() -> String {
    format!("{FG}module.exports = {{ f, g }};\n")
}

fn module_fn() -> String {
    format!("{FG}module.exports = f;\n")
}

const WRONG: &str = "function wrong() {}\nfunction replacement() { return { f: wrong }; }\n";

#[test]
fn with_bodies_do_not_prove_an_inline_require() {
    let o = object();
    let wrap = |decl: &str, body: &str| format!("{WRONG}{decl}\nfunction run() {{ {body} }}\n");
    run_cases(vec![
        // The `with` object supplies `require`: this call invokes `wrong`.
        case(
            "inline-in-with-body",
            &o,
            &wrap("", "with ({ require: replacement }) { require('./m').f(1); }"),
            "f",
            NONE,
        ),
        case(
            "inline-in-with-body-any-object",
            &o,
            &wrap("", "with (env) { require('./m').f(1); }"),
            "f",
            NONE,
        ),
        // A function declared in a `with` body is called with that environment on its chain.
        Case {
            id: "inline-in-function-declared-in-with-body",
            files: vec![
                ("m.js", o.clone()),
                (
                    "app.js",
                    format!(
                        "{WRONG}function run() {{ with (env) {{ function inner() {{ require('./m').f(1); }} inner(); }} }}\n"
                    ),
                ),
            ],
            caller: "inner",
            callee: "f",
            expect: NONE.to_vec(),
        },
        // A top-level `with` (a sloppy script): the declarator is no direct statement of the
        // program, and the `require` inside it is in the object environment.
        case(
            "top-level-with-declarator",
            &o,
            &app("with (env) { var m = require('./m'); }", "m.f(1)"),
            "f",
            NONE,
        ),
        case(
            "top-level-with-member-alias",
            &o,
            &app("with (env) { var f = require('./m').f; }", "f(1)"),
            "f",
            NONE,
        ),
        case(
            "top-level-with-module-fn",
            &module_fn(),
            &app("with (env) { var m = require('./m'); }", "m(1)"),
            "m",
            NONE,
        ),
        // The name used inside a `with` body may be the object's property (the binder's own walk).
        case(
            "declared-require-used-in-with-body",
            &o,
            &app(
                "const m = require('./m');",
                "with (env) { m.f(1); }",
            ),
            "f",
            NONE,
        ),
        // Controls: the same shapes outside a `with` body bind, and a call evaluated in the
        // `with` object expression keeps the outer environment.
        case(
            "inline-ordinary",
            &o,
            &wrap("", "require('./m').f(1);"),
            "f",
            &[F_AT_M],
        ),
        case(
            "inline-in-with-object-expression",
            &o,
            &wrap("", "with (require('./m').f(1)) { }"),
            "f",
            &[F_AT_M],
        ),
        case(
            "inline-beside-a-with-block",
            &o,
            &wrap("", "with (env) { wrong(); } require('./m').f(1);"),
            "f",
            &[F_AT_M],
        ),
        case(
            "declared-ordinary",
            &o,
            &app("const m = require('./m');", "m.f(1)"),
            "f",
            &[F_AT_M],
        ),
    ]);
}

#[test]
fn consumer_member_writes_refuse_the_member_site() {
    let o = object();
    let decl = "const m = require('./m');";
    let site = |extra: &str| format!("{decl}\n{extra}\nfunction run() {{ m.f(1); }}\n");
    let stub = "function stub() {}";
    let mut cases = Vec::new();
    for (id, write) in [
        ("assign", "m.f = stub;"),
        ("assign-parenthesized", "(m).f = stub;"),
        ("assign-compound-logical", "m.f ||= stub;"),
        (
            "assign-in-nested-function",
            "function patch() { m.f = stub; }",
        ),
        ("object-assign", "Object.assign(m, { f: stub });"),
        (
            "define-property",
            "Object.defineProperty(m, 'f', { value: stub });",
        ),
        ("reflect-set", "Reflect.set(m, 'f', stub);"),
        ("delete", "delete m.f;"),
        ("delete-computed", "delete m['f'];"),
        ("destructuring-object", "({ x: m.f } = src);"),
        ("destructuring-array", "[m.f] = src;"),
        ("for-of-head", "for (m.f of src) {}"),
        ("computed-write", "m[key] = stub;"),
        ("string-keyed-write", "m['f'] = stub;"),
        // Spelling-based and file-wide: a shadowing parameter named `m` refuses too.
        (
            "write-through-shadowing-parameter",
            "function p(m) { m.f = stub; }",
        ),
    ] {
        cases.push(case(id, &o, &site(&format!("{stub}\n{write}")), "f", NONE));
    }
    // The write is before the declaration, after the call site, and in a TypeScript file.
    cases.push(case(
        "write-after-call-site",
        &o,
        &format!("{decl}\nfunction run() {{ m.f(1); }}\nm.f = function stub() {{}};\n"),
        "f",
        NONE,
    ));
    cases.push(Case {
        id: "typescript-asserted-write",
        files: vec![
            ("m.js", o.clone()),
            (
                "app.ts",
                format!(
                    "{decl}\n(m as any).f = function stub() {{}};\nfunction run() {{ m.f(1); }}\n"
                ),
            ),
        ],
        caller: "run",
        callee: "f",
        expect: NONE.to_vec(),
    });
    cases.push(Case {
        id: "typescript-nonnull-write",
        files: vec![
            ("m.js", o.clone()),
            (
                "app.ts",
                format!("{decl}\nm!.f = function stub() {{}};\nfunction run() {{ m.f(1); }}\n"),
            ),
        ],
        caller: "run",
        callee: "f",
        expect: NONE.to_vec(),
    });
    // Controls: other members, other objects, deeper paths, reads, and other `Object` calls.
    for (id, extra, expect) in [
        (
            "other-member-written",
            "m.g = function stub() {};",
            vec![F_AT_M],
        ),
        (
            "other-object-written",
            "const o = {};\no.f = function stub() {};",
            vec![F_AT_M],
        ),
        ("deeper-path-written", "m.f.tag = 1;", vec![F_AT_M]),
        ("member-read", "const read = m.f;", vec![F_AT_M]),
        ("object-keys-is-no-mutator", "Object.keys(m);", vec![F_AT_M]),
        (
            "object-assign-onto-another-target",
            "Object.assign(other, m);",
            vec![F_AT_M],
        ),
    ] {
        cases.push(case(id, &o, &site(extra), "f", &expect));
    }
    // A write to `g` leaves the sibling member `f` bound and `g` itself refused.
    cases.push(case(
        "written-member-is-the-refused-one",
        &o,
        &format!("{decl}\nm.g = function stub() {{}};\nfunction run() {{ m.g(1); }}\n"),
        "g",
        NONE,
    ));
    cases.push(case(
        "sibling-of-written-member-binds",
        &o,
        &format!("{decl}\nm.g = function stub() {{}};\nfunction run() {{ m.f(1); }}\n"),
        "f",
        &[F_AT_M],
    ));
    run_cases(cases);
}

/// A proven site replaces what the site had before, as slice L does: a NameOnly rung record
/// (R6 `single_owner`) becomes the Exact export, and the indirect records of the textual
/// function-pointer path stand down for it.
#[test]
fn a_proven_site_replaces_earlier_name_only_and_indirect_records() {
    let consumer = app("const m = require('./m');", "m.f(1)");
    let decoy = "class K { f() {} }\n";
    let sources = |producer: &str| {
        vec![
            ("m.js", producer.to_owned()),
            ("decoy.js", decoy.to_owned()),
            ("app.js", consumer.clone()),
        ]
    };
    let rows_of = |producer: &str| {
        let owned = sources(producer);
        let refs: Vec<(&str, &str)> = owned.iter().map(|(p, s)| (*p, s.as_str())).collect();
        rows(&graph(&refs), "run", "f")
    };
    // Control: a producer the proof refuses leaves the site to R6, whose lone owner is `K`.
    let refused = format!("{FG}exports.f = f;\nexports.f = g;\n");
    assert_eq!(
        rows_of(&refused),
        ["decoy.js:f@1-1 NameOnly/r6_single_owner"],
        "control: before the route, R6 answered"
    );
    // The proven producer: the one Exact record, no NameOnly `K.f`.
    assert_eq!(rows_of(&object()), [F_AT_M]);

    // Indirect records: `exports.g = h` is also a textual function-pointer registration, and so
    // is `obj.g = z` elsewhere. The records are named for their targets (`h`, `z`).
    let other = "function z() {}\nconst obj = {};\nobj.g = z;\n";
    let call = app("const m = require('./m');", "m.g(1)");
    let build = |producer: &str| {
        let files = [
            ("m.js", producer),
            ("other.js", other),
            ("app.js", call.as_str()),
        ];
        let cg = graph(&files);
        ["g", "h", "z"]
            .map(|callee| rows(&cg, "run", callee))
            .concat()
    };
    // Control: with the producer unproven (a conditional registration) the call has no source
    // record and the textual path holds both candidates.
    assert_eq!(
        build("function h() {}\nif (env) { exports.g = h; }\n"),
        [
            "drop UnknownName",
            "m.js:h@1-1 Exact/free_single",
            "other.js:z@1-1 Exact/free_single"
        ],
        "control: before the route"
    );
    // Proven: the one Source record; both textual records stand down for it.
    assert_eq!(
        build("function h() {}\nexports.g = h;\n"),
        ["m.js:h@1-1 Exact/import_member"]
    );
}

/// Same-name functions in the producer file other than the exported declaration: an object
/// literal method on another line, and one on the callable's own line. Each is pinned to what the
/// call resolves to; the route binds only where exactly one function has the recorded name and
/// span.
#[test]
fn same_name_functions_in_the_producer_file() {
    let call = app("const m = require('./m');", "m.f(1)");
    run_cases(vec![
        case(
            "object-method-on-another-line",
            "function f() {}\nconst o = { f() {} };\nmodule.exports = { f };\n",
            &call,
            "f",
            NONE,
        ),
        case(
            "object-method-on-the-same-line",
            "function f() {} const o = { f() {} };\nmodule.exports = { f };\n",
            &call,
            "f",
            NONE,
        ),
    ]);
}

/// `q = require('./m').f` names one export, not the module: a member call through `q` is no
/// module member call, even where the module also exports a member of that name.
#[test]
fn member_of_a_member_alias_is_no_module() {
    run_cases(vec![
        case(
            "member-of-member-alias",
            &object(),
            &app("const q = require('./m').f;", "q.g(1)"),
            "g",
            NONE,
        ),
        case(
            "member-alias-itself-binds",
            &object(),
            &app("const q = require('./m').f;", "q(1)"),
            "q",
            &[F_AT_M],
        ),
    ]);
}
