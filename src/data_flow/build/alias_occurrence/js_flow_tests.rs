//! Fold 3 (value-flow closure) rows of slice 4c: each channel that carries `o`'s value to a written name keeps the
//! quiet alias's twin, and each unrelated shape (another binding of the spelling, a plain call of a free name or a
//! formal, an unrelated sibling callback) lets the gain survive. Written against `Producer::file_refs` only.
use crate::ast::ParsedFile;
use crate::data_flow::build::Producer;
use crate::data_flow::VarAccessKind;
use crate::languages::Language;
use std::collections::BTreeSet;

fn twins(file: &str, src: &str, lang: Language, alias: &str) -> Vec<(String, usize)> {
    let parsed = ParsedFile::parse(file, src, lang).unwrap();
    let r = Producer::file_refs(&file.to_string(), &parsed);
    let all: Vec<_> = r
        .defs
        .values()
        .flatten()
        .filter(|d| d.kind == VarAccessKind::Def)
        .collect();
    let starts: BTreeSet<usize> = all
        .iter()
        .filter(|d| d.path.base == alias)
        .map(|d| d.start_byte)
        .collect();
    let mut out: Vec<_> = all
        .iter()
        .filter(|d| d.path.base != alias && starts.contains(&d.start_byte))
        .map(|d| (d.path.to_string(), d.line))
        .collect();
    out.sort();
    out
}

/// The body wrapped in a function whose alias `u = o` is on line 2.
fn body(b: &str) -> String {
    format!("function g(o, c, z, run, f, arr, mk, xs) {{\n  const u = o;\n{b}  use(o.x);\n}}\n")
}

fn kept(b: &str) -> bool {
    twins("a.js", &body(b), Language::JavaScript, "u").contains(&("o".to_string(), 2))
}

#[test]
fn every_channel_to_a_written_name_keeps_the_twin() {
    for b in [
        // exception channel: operands of calls in the try block, a thrown value, an awaited operand
        "  try {\n    validate(o);\n  } catch (e) {\n    e.x = 1;\n  }\n",
        "  try {\n    throw o;\n  } catch (e) {\n    e.x = 1;\n  }\n",
        "  try {\n    run.check(o);\n  } catch (e) {\n    e.x = 1;\n  }\n",
        // generator resume
        "  function* gen() {\n    const v = yield;\n    v.x = 1;\n  }\n  const it = gen();\n  it.next(o);\n",
        // destructured and defaulted formals
        "  const h = ({ k: p }) => {\n    p.x = 1;\n  };\n  h({ k: o });\n",
        "  const h = ({ k: p } = o) => {\n    p.x = 1;\n  };\n  h();\n",
        // shorthand properties
        "  const s = { o };\n  s.o.x = 1;\n",
        "  const s = { o };\n  const { o: t } = s;\n  t.x = 1;\n",
        // logical assignment results
        "  let r = null;\n  r = o;\n  const v = (r ||= {});\n  v.x = 1;\n",
        "  let r;\n  r = o;\n  const v = (r ??= {});\n  v.x = 1;\n",
        // a non-simple parameter list and a body var or function of a formal's spelling
        "  function h(a = o) {\n    var a;\n    a.x = 1;\n  }\n  h();\n",
        "  function h(a = o) {\n    function a() {}\n    a.x = 1;\n  }\n  h();\n",
        // calls through a binding declared in the owner
        "  const store = [];\n  const add = store.push.bind(store);\n  add(o);\n  store[0].x = 1;\n",
        "  const store = [];\n  const mkAdd = (s) => (v) => { s.push(v); };\n  const add = mkAdd(store);\n  add(o);\n  store[0].x = 1;\n",
        // a declared `console` is no namespace
        "  const console = { log(a, b) { b.x = 1; } };\n  console.log(o, z);\n  z.y = 1;\n",
        // class members: every method and constructor formal
        "  class K { m(a) { a.x = 1; } }\n  new K().m(o);\n",
        "  class K { constructor(a) { a.x = 1; } }\n  new K(o);\n",
        // arguments
        "  function h() {\n    arguments[0].x = 1;\n  }\n  h(o);\n",
    ] {
        assert!(kept(b), "{b}");
    }
    // A file-level binding replaces the built-in namespace.
    let file = format!(
        "const console = {{ log(a, b) {{ b.k = a; }} }};\n{}",
        body("  console.log(o, z);\n  z.x = 1;\n")
    );
    assert!(twins("a.js", &file, Language::JavaScript, "u").contains(&("o".to_string(), 3)));
    // Class `super` call and a TS parameter property.
    let sup = "class B extends A {\n  m(o) {\n    const u = o;\n    super.set(o);\n    this.stored.x = 1;\n    use(o.x);\n  }\n}\n";
    assert!(
        twins("a.js", sup, Language::JavaScript, "u").contains(&("o".to_string(), 3)),
        "{:?}",
        twins("a.js", sup, Language::JavaScript, "u")
    );
    let ts = "function g(o: any) {\n  const u = o;\n  class K { constructor(public a: any) {} w() { this.a.x = 1; } }\n  new K(o).w();\n  use(o.x);\n}\n";
    assert!(twins("a.ts", ts, Language::TypeScript, "u").contains(&("o".to_string(), 2)));
    let ns = "function g(o: any) {\n  const u = o;\n  namespace NS { export const p = o; }\n  NS.p.x = 1;\n  use(o.x);\n}\n";
    assert!(twins("a.ts", ns, Language::TypeScript, "u").contains(&("o".to_string(), 2)));
}

#[test]
fn an_unrelated_name_lets_the_gain_survive() {
    for b in [
        // another binding of the spelling
        "  {\n    let o = mk();\n    o.y = 1;\n  }\n",
        "  [o].forEach(a => { use(a.y); });\n  [z].forEach(a => {\n    a.x = 1;\n  });\n",
        // a plain call of a formal or a free name whose result is unused joins nothing among its arguments
        "  f(o, z);\n  z.x = 1;\n",
        "  validate(o, z);\n  z.x = 1;\n",
        // a built-in namespace member call stores nothing
        "  console.log(o, z);\n  z.x = 1;\n",
        // a try block that never mentions the target
        "  try {\n    validate(z);\n  } catch (e) {\n    use(e);\n  }\n  z.x = 1;\n",
        // generator resume with another value
        "  function* gen() {\n    const v = yield;\n    v.x = 1;\n  }\n  const it = gen();\n  it.next(z);\n",
        "  const s = { z };\n  s.z.x = 1;\n",
        "  let r = null;\n  const v = (r ||= {});\n  v.x = 1;\n",
    ] {
        assert!(!kept(b), "{b}");
    }
}

#[test]
fn a_nested_non_arrow_callables_this_is_its_own_identity() {
    // The write through the nested callable's own `this` is unrelated to the owner's `this` unless a call joins them.
    let apart = "class C {\n  m(arr) {\n    const u = this;\n    arr.forEach(function () { this.y = 1; });\n    use(this.x);\n  }\n}\n";
    assert!(
        twins("a.js", apart, Language::JavaScript, "u").is_empty(),
        "{:?}",
        twins("a.js", apart, Language::JavaScript, "u")
    );
    let bound = "class C {\n  m(arr) {\n    const u = this;\n    arr.forEach(function () { this.y = 1; }, this);\n    use(this.x);\n  }\n}\n";
    assert!(twins("a.js", bound, Language::JavaScript, "u").contains(&("this".to_string(), 3)));
}

#[test]
fn a_write_through_a_sequence_or_call_root_keeps_the_twin() {
    for b in [
        "  (0, o).x = 1;\n",
        "  (0, u).x = 1;\n",
        "  (c ? o : z).x = 1;\n",
        "  f(o).x = 1;\n",
    ] {
        assert!(kept(b), "{b}");
    }
}
