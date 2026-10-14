//! Slice V unit rows: the returned-value fact of the function named `H` in a small file. A fact
//! is `key=callable`, `slot=callable` or `whole=callable`, with a callable shown as
//! `name@start-end`, `import local` or `ns module.member`. `-` is no fact. Every refusal row is
//! a condition the contract names.
use super::JsBindingCache;
use crate::ast::js_binding_helper_tests::parse;
use crate::ast::JsFollow;
use crate::js_returns::JsReturnFacts;

const ANY: [&str; 3] = ["a.js", "a.ts", "a.tsx"];
const TS: [&str; 2] = ["a.ts", "a.tsx"];

fn show_value(v: &JsFollow) -> String {
    match v {
        JsFollow::Callable(t) => format!(
            "{}@{}-{}{}",
            t.local,
            t.start_line,
            t.end_line,
            if t.wrapped { "!w" } else { "" }
        ),
        JsFollow::ImportLocal(l) => format!("import {l}"),
        JsFollow::NamespaceMember {
            module_path,
            member,
        } => format!("ns {module_path}.{member}"),
    }
}

fn show(facts: Option<JsReturnFacts>) -> String {
    let Some(f) = facts else {
        return "-".into();
    };
    let mut out = Vec::new();
    out.extend(f.whole.as_ref().map(|w| format!("whole={}", show_value(w))));
    out.extend(f.keys.iter().map(|(k, v)| format!("{k}={}", show_value(v))));
    out.extend(
        f.slots
            .iter()
            .map(|(i, v)| format!("#{i}={}", show_value(v))),
    );
    out.join(" ")
}

/// The fact of the function named `H` (a declaration, a `const H = …` arrow or function
/// expression), shown as above.
fn fact_of(path: &str, src: &str) -> String {
    let p = parse(path, src);
    let mut cache = JsBindingCache::default();
    let func = p
        .all_functions()
        .into_iter()
        .find(|f| {
            p.language
                .function_name(f)
                .is_some_and(|n| p.node_text(&n) == "H")
        })
        .unwrap_or_else(|| panic!("no function H in {src:?}"));
    show(p.js_ts_return_facts(func, &mut cache))
}

fn check(paths: &[&str], rows: &[(&str, &str, &str)]) {
    let mut wrong = Vec::new();
    for path in paths {
        for (id, src, want) in rows {
            let got = fact_of(path, src);
            if got != *want {
                wrong.push(format!("{id} {path}: got {got:?}, want {want:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong facts:\n{}", wrong.join("\n"));
}

#[test]
fn each_return_form_selects_a_callable_by_key_slot_or_whole() {
    check(
        &ANY,
        &[
            (
                "shorthand",
                "function f() {}\nfunction H() {\n  return { f };\n}\n",
                "f=f@1-1",
            ),
            (
                "renamed key",
                "function f() {}\nfunction H() {\n  return { run: f };\n}\n",
                "run=f@1-1",
            ),
            (
                "method",
                "function H() {\n  return {\n    run() {},\n  };\n}\n",
                "run=run@3-3",
            ),
            (
                "inline arrow property",
                "function H() {\n  return {\n    run: () => 1,\n  };\n}\n",
                "run=run@3-3",
            ),
            (
                "inline function expression property",
                "function H() {\n  return {\n    run: function () {},\n  };\n}\n",
                "run=run@3-3",
            ),
            (
                "array slot",
                "function f() {}\nfunction g() {}\nfunction H() {\n  return [g, f];\n}\n",
                "#0=g@2-2 #1=f@1-1",
            ),
            (
                "array hole is not a slot",
                "function f() {}\nfunction H() {\n  return [, , f];\n}\n",
                "#2=f@1-1",
            ),
            (
                "array spread after the slot",
                "function f() {}\nfunction H(xs) {\n  return [f, ...xs];\n}\n",
                "#0=f@1-1",
            ),
            (
                "whole callable",
                "function f() {}\nfunction H() {\n  return f;\n}\n",
                "whole=f@1-1",
            ),
            (
                "const object identifier",
                "function f() {}\nfunction H() {\n  const api = { f };\n  return api;\n}\n",
                "f=f@1-1",
            ),
            (
                "const array identifier",
                "function f() {}\nfunction H() {\n  const api = [f];\n  return api;\n}\n",
                "#0=f@1-1",
            ),
            (
                "nested local function",
                "function H() {\n  function f() {}\n  return { f };\n}\n",
                "f=f@2-2",
            ),
            (
                "arrow expression body",
                "function f() {}\nconst H = () => ({ f });\n",
                "f=f@1-1",
            ),
            (
                "function expression const",
                "function f() {}\nconst H = function () {\n  return { f };\n};\n",
                "f=f@1-1",
            ),
            (
                "parenthesized",
                "function f() {}\nfunction H() {\n  return ({ f });\n}\n",
                "f=f@1-1",
            ),
            (
                "spread before the key",
                "function f() {}\nfunction H(base) {\n  return { ...base, f };\n}\n",
                "f=f@1-1",
            ),
            (
                "bare and null returns are ignored",
                "function f() {}\nfunction H(c) {\n  if (c) return;\n  if (!c) return null;\n  \
                 return { f };\n}\n",
                "f=f@1-1",
            ),
            (
                "every return selects the same callable",
                "function f() {}\nfunction H(c) {\n  if (c) return { f };\n  return { f, extra: 1 };\n}\n",
                "f=f@1-1",
            ),
            (
                "non-callable members are not facts",
                "function f() {}\nfunction H() {\n  return { n: 1, s: 'x', f };\n}\n",
                "f=f@1-1",
            ),
            (
                "import local value",
                "import { x as imp } from './m';\nfunction H() {\n  return { run: imp };\n}\n",
                "run=import imp",
            ),
            (
                "alias to a local function",
                "function f() {}\nconst q = f;\nfunction H() {\n  return { run: q };\n}\n",
                "run=f@1-1",
            ),
            (
                "alias to a namespace member",
                "import * as N from './m';\nconst q = N.t;\nfunction H() {\n  return { run: q };\n}\n",
                "run=ns ./m.t",
            ),
            (
                "returns inside nested functions and classes are ignored",
                "function f() {}\nfunction H() {\n  const inner = () => { return { z: f }; };\n  \
                 class K { m() { return { y: f }; } }\n  return { f };\n}\n",
                "f=f@1-1",
            ),
        ],
    );
}

#[test]
fn use_callback_of_an_inline_function_is_a_callable_value() {
    check(
        &ANY,
        &[(
            "useCallback",
            "import { useCallback } from 'react';\nfunction H() {\n  \
             const run = useCallback(() => 1, []);\n  return { run };\n}\n",
            "run=run@3-3",
        )],
    );
}

#[test]
fn assertions_around_the_returned_value_are_unwrapped() {
    check(
        &TS,
        &[
            (
                "as const",
                "function f() {}\nfunction H() {\n  return [f] as const;\n}\n",
                "#0=f@1-1",
            ),
            (
                "satisfies",
                "function f() {}\nfunction H() {\n  return { f } satisfies object;\n}\n",
                "f=f@1-1",
            ),
            (
                "non-null",
                "function f() {}\nfunction H() {\n  return { f }!;\n}\n",
                "f=f@1-1",
            ),
            (
                "assertion on the value",
                "function f() {}\nfunction H() {\n  return { run: f as any };\n}\n",
                "run=f@1-1",
            ),
        ],
    );
}

#[test]
fn a_selection_the_contract_refuses_has_no_fact() {
    check(
        &ANY,
        &[
            (
                "spread after the key",
                "function f() {}\nfunction H(r) {\n  return { f, ...r };\n}\n",
                "-",
            ),
            (
                "only keys before a spread are refused",
                "function f() {}\nfunction g() {}\nfunction H(r) {\n  return { f, ...r, g };\n}\n",
                "g=g@2-2",
            ),
            (
                "computed key anywhere",
                "function f() {}\nfunction H(k) {\n  return { [k]: 1, f };\n}\n",
                "-",
            ),
            (
                "string key",
                "function f() {}\nfunction H() {\n  return { 'f': f };\n}\n",
                "-",
            ),
            (
                "getter anywhere",
                "function f() {}\nfunction H() {\n  return { get a() { return 1; }, f };\n}\n",
                "-",
            ),
            (
                "setter anywhere",
                "function f() {}\nfunction H() {\n  return { set a(v) {}, f };\n}\n",
                "-",
            ),
            (
                "duplicate key",
                "function f() {}\nfunction g() {}\nfunction H() {\n  return { f, f: g };\n}\n",
                "-",
            ),
            (
                "array spread before the slot",
                "function f() {}\nfunction H(xs) {\n  return [...xs, f];\n}\n",
                "-",
            ),
            (
                "divergent returns",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  if (c) return { run: f };\n  \
                 return { run: g };\n}\n",
                "-",
            ),
            (
                "a return that is not a supported shape",
                "function f() {}\nfunction H(c) {\n  if (c) return { f };\n  return other();\n}\n",
                "-",
            ),
            (
                "a return that lacks the key",
                "function f() {}\nfunction H(c) {\n  if (c) return { f };\n  return {};\n}\n",
                "-",
            ),
            (
                "a keyless return that is reached after the keyed one",
                "function f() {}\nfunction H(c) {\n  if (c) return {};\n  return { f };\n}\n",
                "-",
            ),
            (
                "a parse error elsewhere in the function (inline value, no binder lookup)",
                "function H() {\n  const junk = ;\n  return { run: () => 1 };\n}\n",
                "-",
            ),
            (
                "returned parameter",
                "function H(p) {\n  return p;\n}\n",
                "-",
            ),
            (
                "parameter as a value",
                "function H(p) {\n  return { run: p };\n}\n",
                "-",
            ),
            (
                "call result as a value (one step only)",
                "function f() {}\nfunction g() { return f; }\nfunction H() {\n  return { run: g() };\n}\n",
                "-",
            ),
            (
                "destructured call result as a value (one step only)",
                "function inner() { return { f() {} }; }\nfunction H() {\n  const { f } = inner();\n  \
                 return { f };\n}\n",
                "-",
            ),
            (
                "conditional value",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  return { run: c ? f : g };\n}\n",
                "-",
            ),
            (
                "conditional return",
                "function f() {}\nfunction H(c) {\n  return c ? { f } : { f };\n}\n",
                "-",
            ),
            (
                "bare inline function has no identity",
                "function H() {\n  return () => 1;\n}\n",
                "-",
            ),
            (
                "let intermediate",
                "function f() {}\nfunction H() {\n  let api = { f };\n  return api;\n}\n",
                "-",
            ),
            (
                "var intermediate",
                "function f() {}\nfunction H() {\n  var api = { f };\n  return api;\n}\n",
                "-",
            ),
            (
                "written const intermediate",
                "function f() {}\nfunction H() {\n  const api = { f };\n  api = null;\n  return api;\n}\n",
                "-",
            ),
            (
                "written intermediate",
                "function f() {}\nfunction H() {\n  let api = { f };\n  api = null;\n  return api;\n}\n",
                "-",
            ),
            (
                "chained intermediates",
                "function f() {}\nfunction H() {\n  const a = { f };\n  const b = a;\n  return b;\n}\n",
                "-",
            ),
            (
                "intermediate declared outside the function",
                "function f() {}\nconst api = { f };\nfunction H() {\n  return api;\n}\n",
                "-",
            ),
            (
                "async function",
                "function f() {}\nasync function H() {\n  return { f };\n}\n",
                "-",
            ),
            (
                "async arrow",
                "function f() {}\nconst H = async () => ({ f });\n",
                "-",
            ),
            (
                "generator function",
                "function f() {}\nfunction* H() {\n  return { f };\n}\n",
                "-",
            ),
            (
                "no value-returning statement",
                "function f() {}\nfunction H() {\n  f();\n}\n",
                "-",
            ),
            (
                "only null returns",
                "function H() {\n  return null;\n}\n",
                "-",
            ),
            (
                "a parse error inside the function",
                "function f() {}\nfunction H() {\n  return { f } @@;\n}\n",
                "-",
            ),
        ],
    );
}

#[test]
fn a_returned_nested_function_is_not_hoisted_into_the_outer_fact() {
    // The nested arrow's own `return { z }` is not a value-returning statement of `H`.
    check(
        &ANY,
        &[(
            "nested return only",
            "function H() {\n  const inner = () => {\n    return { z: () => 1 };\n  };\n  inner();\n}\n",
            "-",
        )],
    );
}

#[test]
fn an_explicit_return_undefined_gives_no_fact_whatever_undefined_is_bound_to() {
    // F1/G1: `undefined` can be rebound by bindings no lookup is trusted to see, so an explicit
    // `return undefined` is like any other return the contract cannot prove: no fact. `return;`
    // and `return null` stay ignored (the controls).
    check(
        &ANY,
        &[
            (
                "global undefined",
                "function f() {}\nfunction H(c) {\n  if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "control: bare and null returns",
                "function f() {}\nfunction H(c) {\n  if (c) return;\n  if (!c) return null;\n  return { f };\n}\n",
                "f=f@1-1",
            ),
            (
                "parenthesized undefined",
                "function f() {}\nfunction H(c) {\n  if (c) return (undefined);\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by an array pattern",
                "function f() {}\nfunction g() {}\nfunction H(c, xs) {\n  const [undefined] = xs;\n  \
                 if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by an object pattern",
                "function f() {}\nfunction H(c, xs) {\n  const { a: undefined } = xs;\n  if (c) return undefined;\n  \
                 return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a let array pattern of objects",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  let [undefined] = [{ f: g }];\n  \
                 if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a for-of pattern",
                "function f() {}\nfunction H(xs) {\n  for (const [undefined] of xs) {\n    return undefined;\n  }\n  \
                 return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a const (object value)",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  const undefined = { f: g };\n  \
                 if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a const (whole value)",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  const undefined = g;\n  \
                 if (c) return undefined;\n  return f;\n}\n",
                "-",
            ),
            (
                "shadowed by a const (array slot)",
                "function f() {}\nfunction g() {}\nfunction H(c) {\n  const undefined = [g];\n  \
                 if (c) return undefined;\n  return [f];\n}\n",
                "-",
            ),
            (
                "shadowed by a parameter",
                "function f() {}\nfunction H(c, undefined) {\n  if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "a default value spelled undefined, then return undefined",
                "function f() {}\nfunction H(c, d = undefined) {\n  if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a catch parameter",
                "function f() {}\nfunction H(c) {\n  try {\n    g();\n  } catch (undefined) {\n    \
                 if (c) return undefined;\n  }\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a destructured parameter",
                "function f() {}\nfunction H(c, { undefined }) {\n  if (c) return undefined;\n  return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by a local function",
                "function f() {}\nfunction H(c) {\n  function undefined() {}\n  if (c) return undefined;\n  \
                 return { f };\n}\n",
                "-",
            ),
            (
                "shadowed by an import",
                "import undefined from './m';\nfunction f() {}\nfunction H(c) {\n  if (c) return undefined;\n  \
                 return { f };\n}\n",
                "-",
            ),
            (
                "shadowed in an enclosing function",
                "function f() {}\nfunction outer(undefined) {\n  function H(c) {\n    if (c) return undefined;\n    \
                 return { f };\n  }\n  return H;\n}\n",
                "-",
            ),
            (
                "shadowed at module scope",
                "function f() {}\nconst undefined = null;\nfunction H(c) {\n  if (c) return undefined;\n  \
                 return { f };\n}\n",
                "-",
            ),
        ],
    );
}

#[test]
fn a_const_intermediate_that_escapes_before_the_return_has_no_fact() {
    // F2: admitted only when every other reference is the whole value of a `return` of `H`.
    check(
        &ANY,
        &[
            (
                "control: returned from two statements",
                "function f() {}\nfunction H(c) {\n  const api = { f };\n  if (c) return api;\n  return api;\n}\n",
                "f=f@1-1",
            ),
            (
                "control: parenthesized return",
                "function f() {}\nfunction H() {\n  const api = { f };\n  return (api);\n}\n",
                "f=f@1-1",
            ),
            (
                "property write",
                "function f() {}\nfunction g() {}\nfunction H() {\n  const api = { f };\n  api.f = g;\n  return api;\n}\n",
                "-",
            ),
            (
                "Object.assign",
                "function f() {}\nfunction g() {}\nfunction H() {\n  const api = { f };\n  \
                 Object.assign(api, { f: g });\n  return api;\n}\n",
                "-",
            ),
            (
                "array element write",
                "function f() {}\nfunction g() {}\nfunction H() {\n  const arr = [f];\n  arr[0] = g;\n  return arr;\n}\n",
                "-",
            ),
            (
                "passed as an argument",
                "function f() {}\nfunction H() {\n  const api = { f };\n  register(api);\n  return api;\n}\n",
                "-",
            ),
            (
                "spread into another literal",
                "function f() {}\nfunction H() {\n  const api = { f };\n  const copy = { ...api };\n  return api;\n}\n",
                "-",
            ),
            (
                "shorthand member of another literal",
                "function f() {}\nfunction H() {\n  const api = { f };\n  const wrap = { api };\n  return api;\n}\n",
                "-",
            ),
            (
                "returned inside a ternary",
                "function f() {}\nfunction H(c) {\n  const api = { f };\n  return c ? api : api;\n}\n",
                "-",
            ),
            (
                "aliased",
                "function f() {}\nfunction H() {\n  const api = { f };\n  const same = api;\n  return api;\n}\n",
                "-",
            ),
            (
                "captured by a closure",
                "function f() {}\nfunction g() {}\nfunction H() {\n  const api = { f };\n  \
                 const later = () => { api.f = g; };\n  return api;\n}\n",
                "-",
            ),
            (
                "returned from a nested function only",
                "function f() {}\nfunction H() {\n  const api = { f };\n  const get = () => { return api; };\n  return api;\n}\n",
                "-",
            ),
            (
                "read through a member",
                "function f() {}\nfunction H() {\n  const api = { f };\n  log(api.f);\n  return api;\n}\n",
                "-",
            ),
        ],
    );
}

fn wide_object(n: usize) -> String {
    let keys: Vec<String> = (0..n).map(|i| format!("k{i}: f")).collect();
    format!(
        "function f() {{}}\nfunction H() {{\n  return {{ {} }};\n}}\n",
        keys.join(", ")
    )
}

fn wide_array(n: usize) -> String {
    let items = vec!["f"; n].join(", ");
    format!("function f() {{}}\nfunction H() {{\n  return [{items}];\n}}\n")
}

#[test]
fn a_literal_with_more_than_512_entries_has_no_fact_and_512_still_does() {
    for path in ANY {
        assert_eq!(
            fact_of(path, &wide_object(512)).split(' ').count(),
            512,
            "{path}"
        );
        assert_eq!(fact_of(path, &wide_object(513)), "-", "{path}");
        assert_eq!(
            fact_of(path, &wide_array(512)).split(' ').count(),
            512,
            "{path}"
        );
        assert_eq!(fact_of(path, &wide_array(513)), "-", "{path}");
        // A spread counts as an entry.
        let spreads = format!(
            "function f() {{}}\nfunction H(r) {{\n  return {{ f, {} }};\n}}\n",
            vec!["...r"; 512].join(", ")
        );
        assert_eq!(fact_of(path, &spreads), "-", "{path}");
    }
}

#[test]
fn duplicate_keys_and_spread_order_are_unchanged_in_a_wide_literal() {
    let mut keys: Vec<String> = (0..400).map(|i| format!("k{i}: f")).collect();
    keys.push("dup: f".into());
    keys.push("dup: f".into());
    keys.insert(0, "...base".into());
    keys.push("...tail".into());
    let src = format!(
        "function f() {{}}\nfunction H(base, tail) {{\n  return {{ {} }};\n}}\n",
        keys.join(", ")
    );
    for path in ANY {
        // The trailing spread removes every key; with it gone, only `dup` is still refused.
        assert_eq!(fact_of(path, &src), "-", "{path}");
        let no_tail = src.replace(", ...tail", "");
        let got = fact_of(path, &no_tail);
        assert_eq!(got.split(' ').count(), 400, "{path}");
        assert!(!got.contains("dup="), "{path}");
    }
}

#[test]
fn an_escaped_spelling_anywhere_in_the_function_gives_no_fact() {
    // G2: the reference scan compares raw text, so a reference to the intermediate spelled with a
    // unicode escape is invisible to it. Any escape in an identifier-like node of the function
    // refuses it; the plain spelling is refused by the scan, the clean control keeps its fact.
    let object = |enc: &str| {
        format!(
            "function f() {{}}\nfunction g() {{}}\nfunction H() {{\n  const api = {{ run: f }};\n  \
             const wrap = {{ {enc} }};\n  wrap.api.run = g;\n  return api;\n}}\n"
        )
    };
    let array = |enc: &str| {
        format!(
            "function f() {{}}\nfunction g() {{}}\nfunction H() {{\n  const arr = [f];\n  \
             const wrap = [{enc}];\n  wrap[0][0] = g;\n  return arr;\n}}\n"
        )
    };
    for path in ANY {
        for enc in ["api", "\\u{61}pi", "\\u0061pi"] {
            assert_eq!(fact_of(path, &object(enc)), "-", "{path} object {enc}");
        }
        for enc in ["arr", "\\u{61}rr", "\\u0061rr"] {
            assert_eq!(fact_of(path, &array(enc)), "-", "{path} array {enc}");
        }
        // An escape elsewhere (a parameter, a property name, a local) refuses just the same.
        let elsewhere =
            "function f() {}\nfunction H(\\u0063) {\n  const api = { f };\n  return api;\n}\n";
        assert_eq!(fact_of(path, elsewhere), "-", "{path} escaped parameter");
        // Clean control: no escape, one return of the intermediate.
        let clean =
            "function f() {}\nfunction H() {\n  const api = { run: f };\n  return api;\n}\n";
        assert_eq!(fact_of(path, clean), "run=f@1-1", "{path} control");
        // A backslash in a string is not an identifier escape.
        let string =
            "function f() {}\nfunction H() {\n  const note = 'a\\\\b';\n  return { run: f };\n}\n";
        assert_eq!(fact_of(path, string), "run=f@1-1", "{path} string");
    }
}
