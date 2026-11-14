//! Rule E (`js_keep.rs`) rows of slice 4c fold 4: one test per K-clause with a negative control each, plus the
//! round-2 and round-3 witnesses that pin keep decisions. Written against `Producer::file_refs` only.
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

/// True when the twin of `u = o` on line 2 of `src` (a `.js` file) is kept.
fn kept_src(src: &str) -> bool {
    twins("a.js", src, Language::JavaScript, "u").contains(&("o".to_string(), 2))
}

/// The body wrapped in a function whose alias `u = o` is on line 2.
fn body(b: &str) -> String {
    format!("function g(o, c, z, run, f, arr, mk, xs) {{\n  const u = o;\n{b}  use(o.x);\n}}\n")
}

fn kept(b: &str) -> bool {
    kept_src(&body(b))
}

#[test]
fn k0_a_census_refused_owner_keeps_every_twin() {
    // An escaped identifier spelling refuses the owner.
    let refused = "function g(o) {\n  const u = o;\n  const \\u0077 = 1;\n  use(o.x);\n}\n";
    assert!(kept_src(refused));
    // Control: the same owner without the escape has no write target and loses the twin.
    let clean = "function g(o) {\n  const u = o;\n  const w = 1;\n  use(o.x);\n}\n";
    assert!(!kept_src(clean));
}

#[test]
fn k1_a_write_rooted_at_another_name_keeps_the_twin() {
    for b in [
        "  z.x = 1;\n",
        "  const w = mk();\n  w.x = 1;\n",
        "  [o].forEach(a => { use(a.y); });\n  [z].forEach(a => {\n    a.x = 1;\n  });\n",
        "  f(o, z);\n  z.x = 1;\n",
        "  console.log(o, z);\n  z.x = 1;\n",
        "  try {\n    validate(z);\n  } catch (e) {\n    use(e);\n  }\n  z.x = 1;\n",
        "  const s = { z };\n  s.z.x = 1;\n",
        // a formal of another function spelled like the alias (row o10)
        "  const h = function (u) { u.x = 1; };\n",
        // a `switch` comparison joins nothing the rule can see: the written `z` is another root (consult probe)
        "  switch (o) {\n    case z:\n      z.x = 1;\n  }\n",
    ] {
        assert!(kept(b), "{b}");
    }
    // Control: reads only, no write target at all.
    for b in [
        "  use(z.x);\n",
        "  f(o, z);\n",
        "  const w = mk();\n  use(w.x);\n",
    ] {
        assert!(!kept(b), "{b}");
    }
}

#[test]
fn k1_a_write_target_with_no_plain_name_root_keeps_the_twin() {
    for b in [
        "  (0, o).x = 1;\n",
        "  (0, u).x = 1;\n",
        "  (c ? o : z).x = 1;\n",
        "  f(o).x = 1;\n",
        "  mk(z).x = 1;\n",
        "  ({}).x = 1;\n",
        "  new.target.x = 1;\n",
        "  [o][0].x = 1;\n",
    ] {
        assert!(kept(b), "{b}");
    }
    // `import.meta` roots (a module-level owner is an arrow here so the syntax is valid in a script).
    let meta = "export function g(o) {\n  const u = o;\n  import.meta.x = 1;\n  use(o.x);\n}\n";
    assert!(twins("a.js", meta, Language::JavaScript, "u").contains(&("o".to_string(), 2)));
    // Control: the same shapes read instead of written.
    for b in [
        "  use((0, o).x);\n",
        "  use(f(o).x);\n",
        "  use(new.target.x);\n",
    ] {
        assert!(!kept(b), "{b}");
    }
}

#[test]
fn k2_a_write_rooted_at_a_single_target_is_the_direct_write_and_the_gain_survives() {
    for src in [
        // formal
        "function g(o) {\n  const u = o;\n  o.x = 1;\n  use(o.x);\n}\n",
        // direct child of the body block: const, var, function, class
        "function g() {\n  const o = mk();\n  const u = o;\n  o.x = 1;\n  use(o.x);\n}\n",
        "function g() {\n  var o = mk();\n  const u = o;\n  o['x'] = 1;\n  use(o.x);\n}\n",
        "function g() {\n  const { o } = mk();\n  const u = o;\n  o.x++;\n  use(o.x);\n}\n",
        // an arrow owner, a method owner
        "const g = (o) => {\n  const u = o;\n  o.x = 1;\n  use(o.x);\n};\n",
        "class K {\n  m(o) {\n    const u = o;\n    o.x = 1;\n    use(o.x);\n  }\n}\n",
    ] {
        let t = twins("a.js", src, Language::JavaScript, "u");
        assert!(t.is_empty(), "{src}: {t:?}");
    }
}

#[test]
fn k2_a_write_rooted_at_a_target_that_is_not_single_keeps_the_twin() {
    for src in [
        // two declarations: a block-local shadow, a callback formal, a catch parameter, a `var` redeclaration
        "function g(o) {\n  const u = o;\n  {\n    let o = u;\n    o.x = 1;\n  }\n  use(o.x);\n}\n",
        "function g(o) {\n  const u = o;\n  [o].forEach(o => { o.x = 1; });\n  use(o.x);\n}\n",
        "function g(o) {\n  const u = o;\n  try { throw o; } catch (o) { o.x = 1; }\n  use(o.x);\n}\n",
        "function g(o) {\n  const u = o;\n  var o;\n  o.x = 1;\n  use(o.x);\n}\n",
        // Annex B: a block function spelled like the formal
        "function g(o) {\n  const u = o;\n  { function o() {} }\n  o.x = 1;\n  use(o.x);\n}\n",
        // a callee formal spelled like the target in a function the owner never calls
        "function g(o) {\n  const u = o;\n  const f = function (o) { o.x = 1; };\n  use(o.x);\n}\n",
        // no declaration in the owner: a captured name
        "function g() {\n  const u = o;\n  o.x = 1;\n  use(o.x);\n}\n",
        // one declaration, but not scoped to the body: an inner block, a loop head, a `try` block
        "function g(c) {\n  if (c) {\n    var o = mk();\n  }\n  const u = o;\n  o.x = 1;\n  use(o.x);\n}\n",
        "function g(xs) {\n  for (const o of xs) {\n    const u = o;\n    o.x = 1;\n  }\n}\n",
        "function g() {\n  try {\n    const o = mk();\n    const u = o;\n    o.x = 1;\n  } finally {}\n}\n",
        // `arguments` is no single binding
        "function g() {\n  const u = arguments;\n  arguments.x = 1;\n  use(arguments.x);\n}\n",
    ] {
        let t = twins("a.js", src, Language::JavaScript, "u");
        assert!(!t.is_empty(), "{src}");
    }
}

#[test]
fn k2_for_this_a_write_in_another_this_context_keeps_the_twin() {
    for src in [
        // a nested non-arrow callable's own `this`, with and without a bound receiver
        "class C {\n  m(arr) {\n    const u = this;\n    arr.forEach(function () { this.x = 1; }, this);\n    use(this.x);\n  }\n}\n",
        "class C {\n  m(arr) {\n    const u = this;\n    arr.forEach(function () { this.y = 1; });\n    use(this.x);\n  }\n}\n",
        // class bodies: a field initialiser, a static block, a computed key, a decorated member
        "class C {\n  m() {\n    const u = this;\n    class D { f = (this.x = 1); }\n    use(this.x);\n  }\n}\n",
        "class C {\n  m() {\n    const u = this;\n    class D { static { this.x = 1; } }\n    use(this.x);\n  }\n}\n",
        "class C {\n  m() {\n    const u = this;\n    class D { [this.x = 1]() {} }\n    use(this.x);\n  }\n}\n",
    ] {
        assert!(
            twins("a.js", src, Language::JavaScript, "u").contains(&("this".to_string(), 3)),
            "{src}"
        );
    }
    // Controls: a direct write and an arrow share the owner's `this`, the gain survives.
    for src in [
        "class C {\n  m(arr) {\n    const u = this;\n    this.x = 1;\n    use(this.x);\n  }\n}\n",
        "class C {\n  m(arr) {\n    const u = this;\n    arr.forEach(() => { this.x = 1; });\n    use(this.x);\n  }\n}\n",
    ] {
        let t = twins("a.js", src, Language::JavaScript, "u");
        assert!(t.is_empty(), "{src}: {t:?}");
    }
}

#[test]
fn every_value_flow_channel_row_of_fold_3_is_kept_by_k1_or_k2() {
    for b in [
        "  try {\n    validate(o);\n  } catch (e) {\n    e.x = 1;\n  }\n",
        "  try {\n    throw o;\n  } catch (e) {\n    e.x = 1;\n  }\n",
        "  function* gen() {\n    const v = yield;\n    v.x = 1;\n  }\n  const it = gen();\n  it.next(o);\n",
        "  const h = ({ k: p }) => {\n    p.x = 1;\n  };\n  h({ k: o });\n",
        "  const s = { o };\n  s.o.x = 1;\n",
        "  let r = null;\n  r = o;\n  const v = (r ||= {});\n  v.x = 1;\n",
        "  function h(a = o) {\n    var a;\n    a.x = 1;\n  }\n  h();\n",
        "  const store = [];\n  const add = store.push.bind(store);\n  add(o);\n  store[0].x = 1;\n",
        "  const console = { log(a, b) { b.x = 1; } };\n  console.log(o, z);\n  z.y = 1;\n",
        "  class K { m(a) { a.x = 1; } }\n  new K().m(o);\n",
        "  function h() {\n    arguments[0].x = 1;\n  }\n  h(o);\n",
    ] {
        assert!(kept(b), "{b}");
    }
    let sup = "class B extends A {\n  m(o) {\n    const u = o;\n    super.set(o);\n    this.stored.x = 1;\n    use(o.x);\n  }\n}\n";
    assert!(twins("a.js", sup, Language::JavaScript, "u").contains(&("o".to_string(), 3)));
    let ts = "function g(o: any) {\n  const u = o;\n  class K { constructor(public a: any) {} w() { this.a.x = 1; } }\n  new K(o).w();\n  use(o.x);\n}\n";
    assert!(twins("a.ts", ts, Language::TypeScript, "u").contains(&("o".to_string(), 2)));
    // TS `as` wrappers on the root, and the single-target control in TypeScript.
    let tsw = "function g(o: any) {\n  const u: any = o;\n  (o as any).x = 1;\n  use(o.x);\n}\n";
    assert!(twins("a.ts", tsw, Language::TypeScript, "u").is_empty());
}
