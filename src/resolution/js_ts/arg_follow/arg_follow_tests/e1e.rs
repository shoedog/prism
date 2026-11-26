use super::*;
use std::process::Command;

const SELF_DEFAULTS: &[(&str, &str, &str, Language)] = &[
    ("formal", "cb = cb", "", Language::JavaScript),
    (
        "typed formal",
        "cb: (s: string) => void = cb",
        "",
        Language::TypeScript,
    ),
    (
        "object shorthand",
        "{cb = cb} = {}",
        "{}",
        Language::JavaScript,
    ),
    ("object pair", "{f: cb = cb}", "{}", Language::JavaScript),
    ("array", "[cb = cb] = []", "[]", Language::JavaScript),
    (
        "nested element",
        "{a: [cb = cb]}",
        "{a: []}",
        Language::JavaScript,
    ),
    ("outer formal", "{cb} = cb", "", Language::JavaScript),
    (
        "outer element",
        "{a: {cb} = cb}",
        "{}",
        Language::JavaScript,
    ),
    (
        "conditional arm",
        "cb = flag ? cb : fallback",
        "",
        Language::JavaScript,
    ),
    (
        "logical or",
        "cb = cb || fallback",
        "",
        Language::JavaScript,
    ),
    (
        "nullish coalescing",
        "cb = cb ?? fallback",
        "",
        Language::JavaScript,
    ),
];

#[test]
fn self_defaults_refuse_without_aborting() {
    const CHILD: &str = "PRISM_E1E_SELF_DEFAULT_ROW";
    if let Some(row) = std::env::var_os(CHILD) {
        let (name, formals, actual, language) =
            SELF_DEFAULTS[row.to_str().unwrap().parse::<usize>().unwrap()];
        let holder = format!("function holder({formals}) {{ cb('changed'); }}");
        // Isolate each overflow, then run the same fixture through the normal
        // build boundary once the crashing route is in scope.
        let control = direct_case(&holder, "", language);
        assert!(control.site("cb('changed')").is_some());
        eprintln!("{name}: same-file control passed");
        let fixture = direct_case(&holder, &format!("holder({actual});"), language);
        fixture.refuses("cb('changed')", "stale_terminal");
        return;
    }
    let test = format!("{}::self_defaults_refuse_without_aborting", module_path!());
    let executable = std::env::current_exe().unwrap();
    let mut failed = Vec::new();
    for (row, (name, ..)) in SELF_DEFAULTS.iter().enumerate() {
        let child = Command::new(&executable)
            .args([
                "--exact",
                test.strip_prefix("prism::").unwrap(),
                "--nocapture",
            ])
            .env(CHILD, row.to_string())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&child.stdout);
        let stderr = String::from_utf8_lossy(&child.stderr);
        assert!(
            stderr.contains("same-file control passed"),
            "{name}: {stdout}\n{stderr}"
        );
        assert!(
            stdout.contains("running 1 test"),
            "{name}: {stdout}\n{stderr}"
        );
        eprintln!("{name}: {}\n{stderr}", child.status);
        if !child.status.success() {
            failed.push(name);
        }
    }
    assert!(failed.is_empty(), "refusal failed for {failed:?}");
}

fn default_stability(holder: &str, calls: &str, reason: &str) {
    let fixture = direct_case(
        &format!(
            "{holder}
            function entryKeeps({{a = verified, cb = a}} = {{}}) {{ cb('entry kept'); }}
            function bodyKeeps(p, a) {{ const {{cb = a}} = p; cb('body kept'); }}"
        ),
        &format!("{calls} entryKeeps(); bodyKeeps({{}}, verified);"),
        Language::JavaScript,
    );
    fixture.targets("cb('entry kept')", &["verified"]);
    fixture.targets("cb('body kept')", &["verified"]);
    assert_eq!(
        fixture.outcome("cb('changed')"),
        Some(&ArgOutcome::Empty(reason.into())),
        "{}",
        fixture.source
    );
}

#[test]
fn default_checks_parameter_list_writes() {
    default_stability(
        "function holder(a, x = (a = fallback), cb = a) { cb('changed'); }",
        "holder(stale);",
        "entry_write",
    );
}

#[test]
fn default_checks_parameter_list_eval() {
    default_stability(
        "function holder(a, x = eval('a = fallback'), cb = a) { cb('changed'); }",
        "holder(stale);",
        "dynamic_binding",
    );
}

#[test]
fn default_checks_pattern_sibling_writes() {
    default_stability(
        "function holder({a, x = (a = fallback), cb = a}) { cb('changed'); }",
        "holder({a: stale});",
        "entry_write",
    );
}

#[test]
fn default_checks_transitive_parameter_reads() {
    default_stability(
        "function holder(a, x = (a = fallback), b = a, cb = b) { cb('changed'); }",
        "holder(stale);",
        "entry_write",
    );
}

#[test]
fn body_default_checks_holder_writes() {
    default_stability(
        "function holder(p, a) { a = fallback; const {cb = a} = p; cb('changed'); }",
        "holder({}, stale);",
        "entry_write",
    );
}

#[test]
fn body_default_checks_holder_eval() {
    default_stability(
        "function holder(p, a) { eval('a = fallback'); const {cb = a} = p; cb('changed'); }",
        "holder({}, stale);",
        "dynamic_binding",
    );
}

#[test]
fn entry_default_preserves_credit_despite_body_write() {
    let fixture = direct_case(
        "function holder(a, cb = a) { a = fallback; cb('kept'); }",
        "holder(verified);",
        Language::JavaScript,
    );
    fixture.targets("cb('kept')", &["verified"]);
}

#[test]
fn default_does_not_read_a_foreign_holders_argument_slot() {
    default_stability(
        "function outer(a) {
             function holder(ignored, cb = a) { cb('changed'); }
             holder(stale);
         }",
        "outer(verified);",
        "stale_terminal",
    );
}

#[test]
fn grounded_and_relayed_parameters_keep_their_stability_check() {
    let fixture = direct_case(
        "function holder(cb) { cb('mixed'); }
         function relay(f) { f = fallback; holder(f); f('direct'); }",
        "relay(stale); holder(verified);",
        Language::JavaScript,
    );
    fixture.targets("cb('mixed')", &["verified"]);
    assert_eq!(
        fixture.outcome("f('direct')"),
        Some(&ArgOutcome::Empty("entry_write".into()))
    );
    assert_eq!(fixture.index.refusal_count("entry_write"), 2);
}

fn prototype_at_read(holder: &str, invoke: &str, prototype: &str) -> Fixture {
    let fixture = direct_case(
        &format!("{holder}
            function entryKeeps({{cb = fallback}}) {{ cb('entry kept'); }}
            function bodyKeeps(p) {{ const {{cb}} = p; cb('body kept'); }}
            function ownKeeps(p) {{ return () => {{ const {{cb = verified}} = p; cb('own kept'); }}; }}"),
        &format!("const entry = {{cb: verified}}, body = {{cb: verified}};
            entryKeeps({{__proto__: entry}}); bodyKeeps({{__proto__: body}});
            ownKeeps({{}})(); const proto = {prototype}; {invoke}"),
        Language::JavaScript,
    );
    fixture.targets("cb('entry kept')", &["verified"]);
    fixture.targets("cb('body kept')", &["verified"]);
    fixture.targets("cb('own kept')", &["verified"]);
    fixture
}

fn deferred(holder: &str, invoke: &str, prototype: &str) {
    prototype_at_read(holder, invoke, prototype).refuses("cb('changed')", "stale_path");
}

#[test]
fn nested_arrow_prototype_hit() {
    deferred(
        "function holder(p) { return () => { const {cb} = p; cb('changed'); }; }",
        "const g = holder({__proto__: proto}); proto.cb = fallback; g();",
        "{cb: stale}",
    );
}

#[test]
fn nested_function_prototype_miss() {
    deferred(
        "function holder(p) { return function later() { const {cb = fallback} = p; cb('changed'); }; }",
        "const g = holder({__proto__: proto}); proto.cb = stale; g();",
        "{}",
    );
}

#[test]
fn nested_method_computed_prototype_lookup() {
    deferred(
        "function holder(p, key) { return {later() { const {[key]: cb} = p; cb('changed'); }}; }",
        "const g = holder({__proto__: proto}, 'cb'); proto.cb = fallback; g.later();",
        "{cb: stale}",
    );
}

#[test]
fn generator_body_prototype_hit() {
    deferred(
        "function* holder(p) { const {cb} = p; cb('changed'); }",
        "const g = holder({__proto__: proto}); proto.cb = fallback; g.next();",
        "{cb: stale}",
    );
}

#[test]
fn async_generator_body_prototype_miss() {
    deferred(
        "async function* holder(p) { const {cb = fallback} = p; cb('changed'); }",
        "const g = holder({__proto__: proto}); proto.cb = stale; g.next();",
        "{}",
    );
}

#[test]
fn generator_expression_without_ground_keeps_no_caller() {
    prototype_at_read(
        "const holder = function* (p) { const {cb} = p; cb('changed'); };",
        "const g = holder({__proto__: proto}); proto.cb = fallback; g.next();",
        "{cb: stale}",
    )
    .refuses("cb('changed')", "no_caller");
}

#[test]
fn async_body_read_after_await() {
    deferred(
        "async function holder(p) { await 0; const {cb} = p; cb('changed'); }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_arrow_without_unique_ground_keeps_holder_identity() {
    prototype_at_read(
        "const holder = async (p) => { await 0; const {cb} = p; cb('changed'); };",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    )
    .refuses("cb('changed')", "holder_identity");
}

#[test]
fn async_read_after_await_in_a_destructuring_default() {
    deferred(
        "async function holder(p) { const {x = await 0, cb} = p; cb('changed'); }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_body_read_inside_for_await() {
    deferred(
        "async function holder(p) { for await (const x of [0]) { const {cb = fallback} = p; cb('changed'); } }",
        "holder({__proto__: proto}); proto.cb = stale;",
        "{}",
    );
}

#[test]
fn async_body_read_after_for_await() {
    deferred(
        "async function holder(p) { for await (const x of [0]) {} const {cb} = p; cb('changed'); }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_body_read_on_await_loop_back_edge() {
    deferred(
        "async function holder(p) { for (const x of [0, 1]) { const {cb} = p; cb('changed'); await 0; } }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_evaluation_order_await_in_method_name() {
    deferred(
        "async function holder(p) { ({[await 0]() {}}); const {cb} = p; cb('changed'); }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_evaluation_order_await_in_later_case_label() {
    deferred(
        "async function holder(p) {
             switch (1) { default: { const {cb} = p; cb('changed'); break; } case await 0: break; }
         }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn async_evaluation_order_await_in_later_class_key() {
    deferred(
        "async function holder(p) {
             class C { static { const {cb} = p; cb('changed'); } [await 0]() {} }
         }",
        "holder({__proto__: proto}); proto.cb = fallback;",
        "{cb: stale}",
    );
}

#[test]
fn deferred_relay_read_keeps_refusal() {
    deferred(
        "function holder(cb) { cb('changed'); }
         function relay(p) { return () => { const {f} = p; holder(f); }; }",
        "const g = relay({__proto__: proto}); proto.f = fallback; g();",
        "{f: stale}",
    );
}

#[test]
fn reads_before_suspension_keep_credit() {
    let fixture = direct_case(
        "async function holder(p) { async function later() { await 0; }
             const {cb} = p; await 0; cb('async kept'); }
         function* generator({cb}) { cb('generator kept'); }
         function relay(p) { const {cb} = p; return () => cb('copied kept'); }
         async function matchingCase(p) {
             switch (1) { case 1: { const {cb} = p; cb('case kept'); break; } case await 0: break; }
         }
         function own(p) { return () => { const {cb} = p; cb('own kept'); }; }",
        "const asyncProto = {cb: verified}, generatorProto = {cb: verified},
             relayProto = {cb: verified}, caseProto = {cb: verified};
         holder({__proto__: asyncProto}); generator({__proto__: generatorProto});
         relay({__proto__: relayProto})(); matchingCase({__proto__: caseProto});
         own({__proto__: {}, cb: verified})();",
        Language::JavaScript,
    );
    for invocation in [
        "cb('async kept')",
        "cb('generator kept')",
        "cb('copied kept')",
        "cb('case kept')",
        "cb('own kept')",
    ] {
        fixture.targets(invocation, &["verified"]);
    }
}
