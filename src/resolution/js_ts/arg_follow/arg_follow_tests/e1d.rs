use super::*;

fn mapped(mutation: &str) {
    let fixture = classic_case(
        &format!(
            "function holder(p) {{ {mutation} const {{cb}} = p; cb('changed'); }}
                  function keeps(p) {{ const {{cb}} = p; cb('kept'); }}"
        ),
        "holder({cb: stale}); keeps({cb: verified});",
    );
    fixture.targets("cb('kept')", &["verified"]);
    fixture.refuses("cb('changed')", "mapped_arguments");
}

#[test]
fn mapped_slot_replacement_before_body_read() {
    for mutation in [
        "arguments[0] = {cb: fallback};",
        "arguments[0] &&= {cb: fallback};",
        "[arguments[0]] = [{cb: fallback}];",
        "for (arguments[0] of [{cb: fallback}]) {}",
        "Reflect.defineProperty(arguments, '0', {value: {cb: fallback}});",
        "arguments[0] = {cb: fallback}; var arguments = [];",
    ] {
        mapped(mutation);
    }
}

#[test]
fn mapped_object_escape_before_body_read() {
    for mutation in [
        "const a = arguments; a[0] = {cb: fallback};",
        "let a; a = arguments; a[0] = {cb: fallback};",
        "const box = {args: arguments}; box.args[0] = {cb: fallback};",
        "((a) => {a[0] = {cb: fallback};})(arguments);",
        "(() => { arguments[0] = {cb: fallback}; })();",
        "Reflect.set = a => { a[0] = {cb: fallback}; }; Reflect.set(arguments, '0', 0, {});",
    ] {
        mapped(mutation);
    }
}

#[test]
fn mapped_element_path_before_body_read() {
    for mutation in [
        "arguments[0].cb = fallback;",
        "const a = arguments[0]; delete a.cb;",
        "const [a] = arguments; a.cb = fallback;",
        "const a = [...arguments]; a[0].cb = fallback;",
        "Object.assign(arguments[0], {cb: fallback});",
        "arguments[0].change();",
    ] {
        mapped(mutation);
    }
}

#[test]
fn mapped_detachment_still_refuses_both_body_reads() {
    let fixture = classic_case(
        "function holder(p) { delete arguments[0]; arguments[0] = {cb: fallback}; const {cb} = p; cb('detached'); }
         function changed(p) { const old = arguments[0]; delete arguments[0]; old.cb = fallback; const {cb} = p; cb('changed'); }",
        "holder({cb: verified}); changed({cb: stale});",
    );
    for site in ["cb('detached')", "cb('changed')"] {
        assert_eq!(
            fixture.outcome(site),
            Some(&ArgOutcome::Empty("mapped_arguments".into()))
        );
    }
    assert_eq!(fixture.index.refusal_count("mapped_arguments"), 2);
}

#[test]
fn unmapped_arguments_can_still_mutate_the_shared_container() {
    for (formals, directive) in [("p", "'use strict';"), ("p, unused = 0", "")] {
        let fixture = direct_case(
            &format!(
                "function holder({formals}) {{ {directive} arguments[0].cb = fallback;
                     const {{cb}} = p; cb('changed'); }}
                 function keeps({formals}) {{ {directive} const {{cb}} = p; cb('kept'); }}"
            ),
            "holder({cb: stale}); keeps({cb: verified});",
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["verified"]);
        fixture.refuses("cb('changed')", "stale_path");
    }
}

#[test]
fn mapped_relay_requires_stable_container() {
    let fixture = classic_case(
        "function holder(cb) { cb('mixed'); }
         function relay(o) { const a = arguments; a[0] = {f: fallback}; const {f} = o; holder(f); }",
        "relay({f: stale}); holder(verified);",
    );
    fixture.targets("cb('mixed')", &["verified"]);
    assert_eq!(fixture.index.refusal_count("mapped_arguments"), 1);
}

#[test]
fn body_source_dynamic_binding_is_independent_of_selected_binding() {
    let fixture = direct_case(
        "function holder(p) { eval('p = fallback'); const {cb} = p; cb('changed'); }
         function keeps(p) { const {cb} = p; cb('kept'); }",
        "holder({cb: stale}); keeps({cb: verified});",
        Language::JavaScript,
    );
    fixture.targets("cb('kept')", &["verified"]);
    fixture.refuses("cb('changed')", "dynamic_binding");
}

fn prototype(mutation: &str, reason: &str) {
    let fixture = direct_case(
        "function holder({cb = fallback}) { cb('mixed'); }",
        &format!("holder({{cb: verified}}); const proto = {{cb: stale}}; {mutation}"),
        Language::JavaScript,
    );
    fixture.targets("cb('mixed')", &["verified"]);
    assert_eq!(fixture.index.refusal_count(reason), 1, "{mutation}");
}

#[test]
fn prototype_writes_before_call() {
    for mutation in [
        "proto.cb = fallback; holder({__proto__: proto});",
        "proto.cb &&= fallback; holder({__proto__: proto});",
        "delete proto.cb; holder({__proto__: proto});",
        "proto.cb++; holder({__proto__: proto});",
        "[proto.cb] = [fallback]; holder({__proto__: proto});",
        "for (proto.cb of [fallback]) {} holder({__proto__: proto});",
        "proto['c' + 'b'] = fallback; holder({__proto__: proto});",
    ] {
        prototype(mutation, "stale_path");
    }
}

#[test]
fn prototype_escape_and_reflective_mutation_before_call() {
    for mutation in [
        "Object.defineProperty(proto, 'cb', {value: fallback}); holder({__proto__: proto});",
        "Object.assign(proto, {cb: fallback}); holder({__proto__: proto});",
        "Object.setPrototypeOf(proto, null); holder({__proto__: proto});",
        "((p) => {p.cb = fallback;})(proto); holder({__proto__: proto});",
        "const alias = proto; alias.cb = fallback; holder({__proto__: proto});",
        "const box = {proto}; box.proto.cb = fallback; holder({__proto__: proto});",
        "new (function(p) {p.cb = fallback;})(proto); holder({__proto__: proto});",
        "const change = () => {proto.cb = fallback;}; change(); holder({__proto__: proto});",
        "const other = {__proto__: proto}; other.__proto__.cb = fallback; holder({__proto__: proto});",
    ] {
        prototype(mutation, "stale_path");
    }
}

#[test]
fn prototype_eval_with_lexically_independent_control() {
    prototype(
        "{ const holder = null, control = null, verified = null, safe = null, stale = null;
           eval('proto.cb = fallback'); }
         holder({__proto__: proto});",
        "stale_path",
    );
}

#[test]
fn projected_use_can_mutate_through_an_accessor_or_receiver() {
    for (object, usage) in [
        ("{cb: stale, change() {this.cb = fallback;}}", "p.change()"),
        (
            "{cb: stale, get change() {this.cb = fallback;}}",
            "void p.change",
        ),
        (
            "{cb: stale, set change(value) {this.cb = value;}}",
            "p.change = fallback",
        ),
    ] {
        let fixture = direct_case(
            &format!(
                "function entry({{cb}}) {{ cb('inherited'); }}
                 function body(p) {{ {usage}; const {{cb}} = p; cb('direct'); }}"
            ),
            &format!(
                "body({object});
                 const p = {object}; {usage}; entry({{__proto__: p}}); entry({{cb: verified}});"
            ),
            Language::JavaScript,
        );
        fixture.targets("cb('inherited')", &["verified"]);
        assert_eq!(
            fixture.outcome("cb('direct')"),
            Some(&ArgOutcome::Empty("stale_path".into()))
        );
        assert_eq!(fixture.index.refusal_count("stale_path"), 2);
    }
    let fixture = direct_case(
        "function holder({cb}) { cb('mixed'); }",
        "const p = {cb: stale, get change() {this.cb = fallback;}};
         const key = 'change'; void p[key]; holder({__proto__: p}); holder({cb: verified});",
        Language::JavaScript,
    );
    fixture.targets("cb('mixed')", &["verified"]);
    assert_eq!(fixture.index.refusal_count("stale_path"), 1);
}

#[test]
fn prototype_uses_prefix_and_execution_order() {
    prototype(
        "const inspect = proto.cb; holder({__proto__: proto}); proto.cb = fallback;",
        "undefined_lookup",
    );
    prototype(
        "for (let i = 0; i < 2; i++) { holder({__proto__: proto}); proto.cb = fallback; }",
        "stale_path",
    );
    prototype(
        "holder({__proto__: proto, extra: (proto.cb = fallback)});",
        "stale_path",
    );
    prototype(
        "switch (0) { default: holder({__proto__: proto}); break; case (proto.cb = fallback, 1): break; }",
        "stale_path",
    );
}

fn key(holder: &str, calls: &str, reason: &'static str) {
    let holder =
        format!("{holder} function keeps(key, p) {{ const {{['left']: cb}} = p; cb('kept'); }}");
    let calls = format!("{calls} keeps('left', {{left: verified, right: stale}});");
    let fixture = classic_case(&holder, &calls);
    fixture.targets("cb('kept')", &["verified"]);
    fixture.refuses("cb('changed')", reason);
}

#[test]
fn computed_body_key_requires_stable_binding() {
    for write in [
        "key = 'right';",
        "key += 'ight';",
        "key &&= 'right';",
        "key++;",
        "[key = 'right'] = [];",
        "for (key of ['right']) {}",
        "var key = 'right';",
        "(() => {key = 'right';})();",
    ] {
        key(
            &format!(
                "function holder(key, p) {{ {write} const {{[key]: cb}} = p; cb('changed'); }}"
            ),
            "holder('left', {left: stale, right: fallback});",
            "entry_write",
        );
    }
}

#[test]
fn computed_key_accounts_for_parameter_initializers() {
    key(
        "function holder(key, ignored = (key = 'right'), {[key]: cb}) { cb('changed'); }",
        "holder('left', undefined, {left: stale, right: fallback});",
        "entry_write",
    );
}

#[test]
fn computed_key_accounts_for_arguments_map() {
    for write in [
        "arguments[0] = 'right';",
        "const a = arguments; a[0] = 'right';",
    ] {
        key(
            &format!(
                "function holder(key, p) {{ {write} const {{[key]: cb}} = p; cb('changed'); }}"
            ),
            "holder('left', {left: stale, right: fallback});",
            "mapped_arguments",
        );
    }
}

#[test]
fn computed_key_requires_initialized_formal_of_this_holder() {
    key(
        "function holder({[key]: cb}, key) { cb('changed'); }",
        "holder({left: stale}, 'left');",
        "path_step",
    );
    key(
        "function outer(key) { function inner(unrelated, {[key]: cb}) { cb('changed'); } inner('left', {left: stale, right: fallback}); }",
        "outer('right');",
        "path_step",
    );
}

#[test]
fn jsx_reserved_attributes_are_not_props_witnesses() {
    for key in ["key", "ref"] {
        let fixture = direct_case(
            &format!(
                "function Holder({{{key}: cb = fallback}}) {{ cb('reserved'); }}
                function Separate({{normal}}) {{ normal('normal'); }}"
            ),
            &format!("<Holder {key}={{stale}} normal={{verified}} />; Holder({{{key}: verified}}); Separate({{normal: verified}});"),
            Language::Tsx,
        );
        fixture.targets("normal('normal')", &["verified"]);
        fixture.targets("cb('reserved')", &["verified"]);
        assert_eq!(fixture.index.refusal_count("jsx_attribute"), 1, "{key}");
    }
}
