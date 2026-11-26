use super::*;
use crate::call_graph::CallSiteOrigin;
use crate::languages::Language;

mod grounds;
mod lifecycle;
mod values;

struct Fixture {
    graph: CallGraph,
    index: ArgIndex,
    source: String,
}

impl Fixture {
    fn build(source: String, language: Language) -> Self {
        let parsed = ParsedFile::parse("m.js", &source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0, "{source}");
        let files = BTreeMap::from([("m.js".to_owned(), parsed)]);
        let graph = CallGraph::build(&files);
        let index = graph.js_anonymous.arg_follow.clone();
        Self {
            graph,
            index,
            source,
        }
    }

    fn outcome(&self, invocation: &str) -> Option<&ArgOutcome> {
        self.site(invocation)
            .and_then(|site| self.index.lookup(site))
    }

    fn site(&self, invocation: &str) -> Option<&CallSite> {
        let start = self.source.find(invocation).unwrap();
        let end = start + invocation.len();
        self.graph.calls.values().flatten().find(|site| {
            site.origin == CallSiteOrigin::Source
                && site.start_byte == start
                && site.end_byte == end
        })
    }

    fn targets(&self, invocation: &str, names: &[&str]) {
        let outcome = self.outcome(invocation);
        let Some(ArgOutcome::Targets(targets)) = outcome else {
            panic!(
                "expected {names:?} at {invocation}: {outcome:?}\n{}",
                self.source
            );
        };
        let observed: Vec<_> = targets.iter().map(|target| target.name.as_str()).collect();
        assert_eq!(observed, names, "{invocation}\n{}", self.source);
    }

    fn refuses(&self, invocation: &str, reason: &'static str) {
        assert_eq!(
            self.outcome(invocation),
            Some(&ArgOutcome::Empty(reason.into())),
            "{invocation}\n{}",
            self.source
        );
        assert_eq!(
            self.index.refusal_count(reason),
            1,
            "{invocation}\n{}",
            self.source
        );
    }
}

const CONTROL: &str = "function verified(value) {}\nfunction control(good) { good('control'); }\n";

fn direct_case(holder: &str, calls: &str, language: Language) -> Fixture {
    let source = format!("{CONTROL}function stale(value) {{}}\nfunction fallback(value) {{}}\n{holder}\nfunction run() {{ control(verified); {calls} }}");
    let fixture = Fixture::build(source, language);
    fixture.targets("good('control')", &["verified"]);
    fixture
}

#[test]
fn refuses_default_without_ground() {
    for (actual, reason) in [(None, "no_caller"), (Some("opaque"), "opaque_terminal")] {
        let calls = actual.map_or_else(String::new, |actual| format!("holder({actual});"));
        let fixture = direct_case(
            "function holder(cb = fallback) { cb('refusal'); }",
            &calls,
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", reason);
    }
    for actual in ["", "undefined", "void stale"] {
        let fixture = direct_case(
            "function holder(cb = fallback) { cb('default'); }",
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.targets("cb('default')", &["fallback"]);
    }
}

#[test]
fn refuses_written_entry_binding() {
    for write in [
        "cb = fallback;",
        "cb += fallback;",
        "cb ||= fallback;",
        "cb++;",
        "[cb] = [];",
        "({cb} = {});",
        "for (cb in {}) {}",
        "for (cb of []) {}",
        "var cb = fallback;",
        "if (false) cb = fallback;",
    ] {
        let fixture = direct_case(
            &format!("function holder(cb) {{ cb('refusal'); {write} }}"),
            "holder(stale);",
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "entry_write");
    }
    for write in [
        "var cb;",
        "{ let cb; cb = fallback; }",
        "try {} catch (cb) { cb = fallback; }",
    ] {
        let fixture = direct_case(
            &format!("function holder(cb) {{ cb('kept'); {write} }}"),
            "holder(stale);",
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["stale"]);
    }
}

#[test]
fn refuses_unknown_spread_slot() {
    for (formal, actual) in [
        ("first, cb", "...unknown, 0, stale"),
        ("cb", "...unknown"),
        ("cb = fallback", "...[, stale]"),
    ] {
        let fixture = direct_case(
            &format!("function holder({formal}) {{ cb('refusal'); }}"),
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "unknown_spread");
    }
    for actual in ["...[stale]", "stale, ...unknown"] {
        let fixture = direct_case(
            "function holder(this: void, cb: () => void) { cb('kept'); }",
            &format!("holder({actual});"),
            Language::TypeScript,
        );
        fixture.targets("cb('kept')", &["stale"]);
    }
}

#[test]
fn refuses_dynamic_binding_environment() {
    let fixture = direct_case(
        "function holder(cb) { eval('cb = fallback'); cb('refusal'); }",
        "holder(stale);",
        Language::JavaScript,
    );
    fixture.refuses("cb('refusal')", "dynamic_binding");
    for evaluation in [
        "function eval(code) {} eval('ignored');",
        "eval?.('ignored');",
        "(0, eval)('ignored');",
        "function disjoint(cb) { eval('cb = fallback'); }",
    ] {
        let fixture = direct_case(
            &format!("function holder(cb) {{ {evaluation} cb('kept'); }}"),
            "holder(stale);",
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["stale"]);
    }
}

#[test]
fn refuses_hidden_literal_key() {
    for actual in [
        "{ cb: stale, ...opaque }",
        "{ cb: stale, [opaque]: undefined }",
    ] {
        let fixture = direct_case(
            "function holder({cb = fallback}) { cb('refusal'); }",
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "hidden_literal_key");
    }
    for actual in [
        "{ ...opaque, cb: stale }",
        "{ cb: fallback, ...{ cb: stale } }",
        "{ cb: fallback, [opaque]: undefined, cb: stale }",
    ] {
        let fixture = direct_case(
            "function holder({cb = fallback}) { cb('kept'); }",
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["stale"]);
    }
}

#[test]
fn prototype_setters_and_computed_data_choose_different_values() {
    for (key, want) in [
        ("__proto__", "stale"),
        ("\"__proto__\"", "stale"),
        ("[\"__proto__\"]", "fallback"),
    ] {
        let fixture = direct_case(
            "function holder({cb = fallback}) { cb('selected'); }",
            &format!("const proto = {{cb: stale}}; holder({{{key}: proto}});"),
            Language::JavaScript,
        );
        fixture.targets("cb('selected')", &[want]);
    }
}

#[test]
fn final_descriptor_preserves_getters_and_resets_data() {
    for actual in [
        "get cb() { return stale; }, set cb(v) {}",
        "set cb(v) {}, get cb() { return stale; }",
        "cb: stale, get cb() { return stale; }",
    ] {
        let fixture = direct_case(
            "function holder({cb = fallback}) { cb('refusal'); }",
            &format!("holder({{{actual}}});"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "undefined_lookup");
    }
    for (actual, want) in [
        ("get cb() {}, cb: stale", "stale"),
        ("cb: stale, set cb(v) {}", "fallback"),
        ("set cb(v) {}", "fallback"),
    ] {
        let fixture = direct_case(
            "function holder({cb = fallback}) { cb('kept'); }",
            &format!("holder({{{actual}}});"),
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &[want]);
    }
}

#[test]
fn refuses_third_lookup_only_on_that_path() {
    let fixture = direct_case(
        "function holder(cb) { cb('derived'); }\nfunction relay(forwarded) { holder(forwarded); forwarded('two'); }\nfunction outer(carried) { relay(carried); carried('one'); }",
        "outer(stale); holder(verified);",
        Language::JavaScript,
    );
    fixture.targets("cb('derived')", &["verified"]);
    fixture.targets("forwarded('two')", &["stale"]);
    fixture.targets("carried('one')", &["stale"]);
    assert_eq!(fixture.index.refusal_count("depth"), 1);
}

#[test]
fn refuses_ninth_distinct_target_per_site() {
    for count in [8, 9] {
        let declarations: String = (0..count)
            .map(|i| format!("function t{i}(v) {{}}\n"))
            .collect();
        let calls: String = (0..count)
            .map(|i| format!("holder(t{i}, verified); "))
            .collect();
        let fixture = direct_case(
            &format!(
                "{declarations}function holder(cb, narrow) {{ cb('broad'); narrow('narrow'); }}"
            ),
            &format!("{calls}holder(t0, verified);"),
            Language::JavaScript,
        );
        fixture.targets("narrow('narrow')", &["verified"]);
        if count == 9 {
            assert_eq!(fixture.outcome("cb('broad')"), Some(&ArgOutcome::Fanout(9)));
            assert_eq!(fixture.index.refusal_count("fanout"), 1);
        } else {
            fixture.targets(
                "cb('broad')",
                &["t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7"],
            );
        }
    }
}

#[test]
fn refuses_live_mapped_arguments_mutation() {
    for mutation in [
        "arguments[0] = fallback;",
        "arguments[0]++;",
        "Object.defineProperty(arguments, '0', {value: fallback});",
        "Reflect.set(arguments, '0', fallback);",
        "external(arguments);",
        "(() => { arguments[0] = fallback; })();",
    ] {
        let fixture = direct_case(
            &format!("function holder(cb) {{ {mutation} cb('refusal'); }}"),
            "holder(stale);",
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "mapped_arguments");
    }
    for mutation in [
        "'use strict'; arguments[0] = fallback;",
        "delete arguments[0]; arguments[0] = fallback;",
        "arguments[1] = fallback;",
        "const copy = [...arguments]; copy[0] = fallback;",
        "{ const arguments = []; arguments[0] = fallback; }",
        "Reflect.set(arguments, '0', fallback, {});",
    ] {
        let fixture = direct_case(
            &format!("function holder(cb) {{ {mutation} cb('kept'); }}"),
            "holder(stale);",
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["stale"]);
    }
    let fixture = direct_case(
        "function holder(cb, cb) { arguments[0] = fallback; cb('kept'); }",
        "holder(verified, stale);",
        Language::JavaScript,
    );
    fixture.targets("cb('kept')", &["stale"]);
}

#[test]
fn refuses_stale_selected_path() {
    for mutation in [
        "properties.cb = fallback;",
        "delete properties.cb;",
        "properties[key] = fallback;",
        "const alias = properties; alias.cb = fallback;",
    ] {
        let fixture = direct_case(&format!("function holder(properties) {{ {mutation} const {{cb}} = properties; cb('refusal'); }}"), "holder({cb: stale});", Language::JavaScript);
        fixture.refuses("cb('refusal')", "stale_path");
    }
    for (before, after) in [
        ("properties.other = fallback;", ""),
        ("", "properties.cb = fallback;"),
        ("properties.cb.rank = 2;", ""),
    ] {
        let fixture = direct_case(&format!("function holder(properties) {{ {before} const {{cb}} = properties; {after} cb('kept'); }}"), "holder({cb: stale});", Language::JavaScript);
        fixture.targets("cb('kept')", &["stale"]);
    }
}

#[test]
fn refuses_fallback_on_inherited_builtin_keys() {
    for key in [
        "constructor",
        "toString",
        "toLocaleString",
        "valueOf",
        "hasOwnProperty",
        "isPrototypeOf",
        "propertyIsEnumerable",
        "__proto__",
        "__defineGetter__",
        "__defineSetter__",
        "__lookupGetter__",
        "__lookupSetter__",
    ] {
        let fixture = direct_case(
            &format!("function holder({{{key}: cb = fallback}}) {{ cb('refusal'); }}"),
            "holder({});",
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "opaque_terminal");
    }
    for actual in ["{constructor: undefined}", "{__proto__: null}"] {
        let fixture = direct_case(
            "function holder({constructor: cb = fallback}) { cb('kept'); }",
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["fallback"]);
    }
}

#[test]
fn refuses_cycle_without_erasing_independent_ground() {
    let fixture = direct_case(
        "function holder(cb) { cb('cycle'); holder(cb); }",
        "",
        Language::JavaScript,
    );
    fixture.refuses("cb('cycle')", "cycle");
    let fixture = direct_case(
        "function holder(cb, other) { cb('first'); other('second'); holder(other, cb); }",
        "holder(stale, fallback);",
        Language::JavaScript,
    );
    fixture.targets("cb('first')", &["fallback", "stale"]);
    fixture.targets("other('second')", &["fallback", "stale"]);
    assert_eq!(fixture.index.refusal_count("cycle"), 2);
}

#[test]
fn refuses_writes_in_other_execution_regions() {
    for holder in [
        "function holder(cb) { function change() { cb = fallback; } change(); cb('refusal'); }",
        "function holder(cb, unused = (cb = fallback)) { cb('refusal'); }",
        "function holder(cb) { function cb() {} cb('refusal'); }",
    ] {
        let fixture = direct_case(holder, "holder(stale);", Language::JavaScript);
        fixture.refuses("cb('refusal')", "entry_write");
    }
    for holder in [
        "function holder(cb, result = cb('copied')) { function cb() {} }",
        "function holder(source, cb = source) { source = fallback; cb('copied'); }",
        "function holder(cb, fire = () => cb('copied')) { var cb = fallback; fire(); }",
        "function holder(cb) { function change(cb) { cb = fallback; } cb('copied'); }",
    ] {
        let fixture = direct_case(holder, "holder(stale);", Language::JavaScript);
        fixture.targets("cb('copied')", &["stale"]);
    }
}

#[test]
fn refuses_stale_terminal_identity() {
    for declaration in [
        "let handler = (value) => value;",
        "var handler = (value) => value;",
        "function handler(v) {} handler = fallback;",
    ] {
        let fixture = direct_case(
            "function holder(cb) { cb('refusal'); }",
            &format!("{declaration} holder(handler);"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "stale_terminal");
    }
    let fixture = direct_case(
        "function holder(cb) { cb('refusal'); }",
        "holder(handler); const handler = () => {};",
        Language::JavaScript,
    );
    fixture.refuses("cb('refusal')", "stale_terminal");
    let fixture = direct_case(
        "function holder(cb) { cb('kept'); }",
        "const a = stale; const b = a; holder(b);",
        Language::JavaScript,
    );
    fixture.targets("cb('kept')", &["stale"]);
}

#[test]
fn refuses_shadow_and_scalar_alias() {
    let fixture = direct_case(
        "function holder(cb) { cb('direct'); { const cb = fallback; cb('block'); } try {} catch (cb) { cb('catch'); } const alias = cb; alias('alias'); }\nfunction fromObject(input) { const {cb} = input; cb('destructure'); }",
        "holder(stale); fromObject({cb: verified});",
        Language::JavaScript,
    );
    fixture.targets("cb('direct')", &["stale"]);
    fixture.targets("cb('destructure')", &["verified"]);
    for refused in ["cb('block')", "cb('catch')", "alias('alias')"] {
        assert_eq!(fixture.outcome(refused), None, "{refused}");
    }
}

#[test]
fn refuses_nonformal_invocation_shapes() {
    let fixture = direct_case(
        "function holder(cb, ...rest) { cb('direct'); cb.call(null); cb.apply(null, []); cb.member(); rest[0](); cb`tag`; new cb(); }",
        "holder(stale, fallback);",
        Language::JavaScript,
    );
    fixture.targets("cb('direct')", &["stale"]);
    for refused in [
        "cb.call(null)",
        "cb.apply(null, [])",
        "cb.member()",
        "rest[0]()",
        "cb`tag`",
        "new cb()",
    ] {
        assert_eq!(fixture.outcome(refused), None, "{refused}");
    }
    let fixture = direct_case(
        "function holder(cb: (...args: unknown[]) => void) { cb?.('optional'); (cb as Function)('transparent'); }",
        "holder(stale);",
        Language::TypeScript,
    );
    fixture.targets("cb?.('optional')", &["stale"]);
    fixture.targets("(cb as Function)('transparent')", &["stale"]);
}

#[test]
fn refuses_opaque_terminal_forms() {
    for actual in [
        "state.cb",
        "this.fn",
        "provider()",
        "stale.bind(null)",
        "class {}",
        "new Proxy(stale, {})",
        "Object.values(items)[0]",
    ] {
        let fixture = direct_case(
            "function holder(cb) { cb('mixed'); } function provider() { return stale; }",
            &format!("holder({actual}); holder(verified);"),
            Language::JavaScript,
        );
        fixture.targets("cb('mixed')", &["verified"]);
        assert_eq!(
            fixture.index.refusal_count("opaque_terminal"),
            1,
            "{actual}"
        );
    }
}

#[test]
fn refuses_foreign_or_unwitnessed_holder() {
    let holder = "export function timer(cb) { cb('bound'); }\nfunction localTarget(o) {}\nfunction localCall() { timer(localTarget); }";
    let caller = "import { timer as imported } from './holder.js';\nfunction decoy(o) {}\nfunction valid(o) {}\nfunction run() { timer(decoy); imported(valid); }";
    let files = BTreeMap::from([
        (
            "holder.js".into(),
            ParsedFile::parse("holder.js", holder, Language::JavaScript).unwrap(),
        ),
        (
            "caller.js".into(),
            ParsedFile::parse("caller.js", caller, Language::JavaScript).unwrap(),
        ),
    ]);
    let graph = CallGraph::build(&files);
    let site = |file: &str, start: usize| {
        graph
            .calls
            .values()
            .flatten()
            .find(|site| {
                site.caller.file == file
                    && site.start_byte == start
                    && site.origin == CallSiteOrigin::Source
            })
            .unwrap()
    };
    let foreign = site("caller.js", caller.find("timer(decoy)").unwrap());
    let outcome = graph.resolve_call_site_full(foreign);
    assert!(
        matches!(outcome.resolved.as_slice(), [target] if target.kind == crate::resolution::ResolutionKind::FreeSingle && target.confidence == ResolutionConfidence::Exact && target.target.file == "holder.js")
    );
    let fact = graph
        .js_anonymous
        .arg_follow
        .lookup(site("holder.js", holder.find("cb('bound')").unwrap()));
    let Some(ArgOutcome::Targets(targets)) = fact else {
        panic!("expected local and imported grounds: {fact:?}");
    };
    assert_eq!(
        targets
            .iter()
            .map(|id| (id.file.as_str(), id.name.as_str()))
            .collect::<Vec<_>>(),
        [("caller.js", "valid"), ("holder.js", "localTarget")]
    );
    assert_eq!(
        graph
            .js_anonymous
            .arg_follow
            .refusal_count("holder_identity"),
        1
    );
}
