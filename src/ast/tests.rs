use super::*;

fn fns(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn descendant_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = descendant_of_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn assert_statement_markers(path: &str, source: &str, language: Language, markers: &[&str]) {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    let func = parsed.all_functions()[0];
    let lines: Vec<usize> = parsed
        .statements_in_function(&func)
        .into_iter()
        .map(|(line, _)| line)
        .collect();
    let span_lines: Vec<usize> = parsed
        .statement_spans_in_function(&func)
        .into_iter()
        .map(|span| span.line)
        .collect();
    assert_eq!(
        span_lines, lines,
        "{path}: byte-span and line-only statement walkers must stay identical"
    );
    let arms = parsed.statement_arms_in_function(&func);
    let untagged: Vec<usize> = lines
        .iter()
        .copied()
        .filter(|line| arms.arm_of_line(*line).is_none())
        .collect();
    assert!(
        untagged.is_empty(),
        "{path}: statement lines missing lexical arms: {untagged:?}; got {lines:?}"
    );
    let missing: Vec<(&str, usize)> = markers
        .iter()
        .map(|marker| {
            let line = source
                .lines()
                .position(|source_line| source_line.contains(marker))
                .map(|line| line + 1)
                .unwrap_or_else(|| panic!("marker `{marker}` must occur in the fixture"));
            (*marker, line)
        })
        .filter(|(_, line)| !lines.contains(line))
        .collect();
    assert!(
        missing.is_empty(),
        "{path}: nested statement markers missing from CFG universe: {missing:?}; got {lines:?}"
    );
}

#[test]
fn statements_in_function_descends_rust_expression_statement_wrappers() {
    let source = "fn h(x: i32) {\n    let a = x + 1;\n    if a > 0 {\n        sink_rust_if(a);\n    }\n    match a {\n        _ => { sink_rust_match(a); }\n    }\n    loop {\n        sink_rust_loop(a);\n        break;\n    }\n    while a > 0 {\n        sink_rust_while(a);\n        break;\n    }\n    for v in 0..a {\n        sink_rust_for(v);\n    }\n    unsafe {\n        sink_rust_unsafe(a);\n    }\n    async {\n        sink_rust_async(a);\n    };\n    const {\n        sink_rust_const(a);\n    };\n    try {\n        sink_rust_try(a);\n    };\n    gen {\n        sink_rust_gen(a);\n    };\n    sink(a);\n}\n";
    assert_statement_markers(
        "m.rs",
        source,
        Language::Rust,
        &[
            "sink_rust_if",
            "sink_rust_match",
            "sink_rust_loop",
            "sink_rust_while",
            "sink_rust_for",
            "sink_rust_unsafe",
            "sink_rust_async",
            "sink_rust_const",
            "sink_rust_try",
            "sink_rust_gen",
        ],
    );
}

#[test]
fn statements_in_function_descends_python_statement_wrappers() {
    let source = "def h(x, lock):\n    with lock:\n        sink_py_with(x)\n    match x:\n        case 1:\n            sink_py_case(x)\n    try:\n        sink_py_try(x)\n    except ValueError:\n        sink_py_except(x)\n    finally:\n        sink_py_finally(x)\n    try:\n        sink_py_group_try(x)\n    except* TypeError:\n        sink_py_except_group(x)\n    return x\n";
    assert_statement_markers(
        "m.py",
        source,
        Language::Python,
        &[
            "sink_py_with",
            "sink_py_case",
            "sink_py_except(x)",
            "sink_py_finally",
            "sink_py_except_group",
        ],
    );
}

const JS_WRAPPER_SOURCE: &str = "function h(x, obj) {\n  let y = x;\nlabel:\n  while (y < 2) {\n    sink_js_label(y);\n    break;\n  }\n  switch (y) {\n    case 1:\n      sink_js_case(y);\n      break;\n    default:\n      sink_js_default(y);\n  }\n  try {\n    sink_js_try(y);\n  } catch (e) {\n    sink_js_catch(y);\n  } finally {\n    sink_js_finally(y);\n  }\n  with (obj) {\n    sink_js_with(y);\n  }\n  return y;\n}\n";

#[test]
fn statements_in_function_descends_javascript_statement_wrappers() {
    assert_statement_markers(
        "m.js",
        JS_WRAPPER_SOURCE,
        Language::JavaScript,
        &[
            "sink_js_label",
            "sink_js_case",
            "sink_js_default",
            "sink_js_catch",
            "sink_js_finally",
            "sink_js_with",
        ],
    );
}

const TS_WRAPPER_SOURCE: &str = "function h(x: number): number {\n  let y = x;\nlabel:\n  while (y < 2) {\n    sink_ts_label(y);\n    break;\n  }\n  switch (y) {\n    case 1:\n      sink_ts_case(y);\n      break;\n    default:\n      sink_ts_default(y);\n  }\n  try {\n    sink_ts_try(y);\n  } catch (e) {\n    sink_ts_catch(y);\n  } finally {\n    sink_ts_finally(y);\n  }\n  return y;\n}\n";

#[test]
fn statements_in_function_descends_typescript_statement_wrappers() {
    assert_statement_markers(
        "m.ts",
        TS_WRAPPER_SOURCE,
        Language::TypeScript,
        &[
            "sink_ts_label",
            "sink_ts_case",
            "sink_ts_default",
            "sink_ts_catch",
            "sink_ts_finally",
        ],
    );
}

#[test]
fn statements_in_function_descends_tsx_statement_wrappers() {
    assert_statement_markers(
        "m.tsx",
        TS_WRAPPER_SOURCE,
        Language::Tsx,
        &[
            "sink_ts_label",
            "sink_ts_case",
            "sink_ts_default",
            "sink_ts_catch",
            "sink_ts_finally",
        ],
    );
}

#[test]
fn statements_in_function_descends_go_statement_wrappers() {
    let source = "package p\nfunc h(ch chan any, x any) any {\n    y := x\nlabel:\n    for y == nil {\n        sink_go_label(y)\n        break\n    }\n    select {\n    case v := <-ch:\n        sink_go_select(v)\n    default:\n        sink_go_select_default(y)\n    }\n    switch y {\n    case 1:\n        sink_go_switch(y)\n    default:\n        sink_go_switch_default(y)\n    }\n    switch v := y.(type) {\n    case int:\n        sink_go_type(v)\n    default:\n        sink_go_type_default(y)\n    }\n    return y\n}\n";
    assert_statement_markers(
        "m.go",
        source,
        Language::Go,
        &[
            "sink_go_label",
            "sink_go_select(v)",
            "sink_go_select_default",
            "sink_go_switch(y)",
            "sink_go_switch_default",
            "sink_go_type(v)",
            "sink_go_type_default",
        ],
    );
}

#[test]
fn statements_in_function_descends_c_labeled_statement_wrapper() {
    let source = "int h(int x) {\n    int y = x;\nlabel:\n    while (y < 2) {\n        sink_c_label(y);\n        y++;\n    }\n    [[likely]] while (y < 3) {\n        sink_c_attribute(y);\n        break;\n    }\n    return y;\n}\n";
    assert_statement_markers(
        "m.c",
        source,
        Language::C,
        &["sink_c_label", "sink_c_attribute"],
    );
}

#[test]
fn statements_in_function_descends_cpp_statement_wrappers() {
    let source = "int h(int x) {\n    int y = x;\nlabel:\n    while (y < 2) {\n        sink_cpp_label(y);\n        y++;\n    }\n    [[likely]] while (y < 3) {\n        sink_cpp_attribute(y);\n        break;\n    }\n    try {\n        sink_cpp_try(y);\n    } catch (...) {\n        sink_cpp_catch(y);\n    }\n    return y;\n}\n";
    assert_statement_markers(
        "m.cpp",
        source,
        Language::Cpp,
        &["sink_cpp_label", "sink_cpp_attribute", "sink_cpp_catch"],
    );
}

#[test]
fn statements_in_function_descends_java_statement_wrappers() {
    let source = "class M {\n  static int h(int x, Object lock) {\n    int y = x;\nlabel:\n    while (y < 0) {\n      sink_java_label(y);\n      break;\n    }\n    synchronized (lock) {\n      sink_java_sync(y);\n    }\n    switch (y) {\n      case 1:\n        sink_java_case(y);\n        break;\n      default:\n        sink_java_default(y);\n    }\n    switch (y) {\n      case 2 -> sink_java_rule(y);\n      default -> sink_java_rule_default(y);\n    }\n    try (Resource r = open()) {\n      sink_java_try(y);\n    } catch (RuntimeException e) {\n      sink_java_catch(y);\n    } finally {\n      sink_java_finally(y);\n    }\n    return y;\n  }\n}\n";
    assert_statement_markers(
        "M.java",
        source,
        Language::Java,
        &[
            "sink_java_label",
            "sink_java_sync",
            "sink_java_case",
            "sink_java_default",
            "sink_java_rule(y)",
            "sink_java_rule_default",
            "sink_java_try",
            "sink_java_catch",
            "sink_java_finally",
        ],
    );
}

#[test]
fn statements_in_function_descends_bash_statement_wrappers() {
    let source = "h() {\n  local x=0\n  for i in 1; do\n    sink_bash_do \"$x\"\n  done\n  case \"$x\" in\n    1) sink_bash_case \"$x\" ;;\n  esac\n  (\n    sink_bash_subshell \"$x\"\n  )\n  {\n    sink_bash_redirect \"$x\"\n  } > /dev/null\n}\n";
    assert_statement_markers(
        "m.sh",
        source,
        Language::Bash,
        &[
            "sink_bash_do",
            "sink_bash_case",
            "sink_bash_subshell",
            "sink_bash_redirect",
        ],
    );
}

#[test]
fn statements_in_function_descends_lua_statement_wrappers() {
    let source = "function h(x)\n  if x == 1 then\n    sink_lua_first(x)\n  elseif x == 2 then\n    sink_lua_middle(x)\n  else\n    sink_lua_last(x)\n  end\n  return x\nend\n";
    assert_statement_markers(
        "m.lua",
        source,
        Language::Lua,
        &["sink_lua_middle", "sink_lua_last"],
    );
}

#[test]
fn parsed_file_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ParsedFile>();
}

#[test]
fn loaded_repo_is_send_and_sync() {
    // Informational companion: a !Sync field elsewhere in LoadedRepo must NOT
    // gate C2 — if only this one fails, that field is investigated separately.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<crate::repo_loader::LoadedRepo>();
}

#[test]
fn decorated_function_field_readers_unwrap() {
    let p = ParsedFile::parse(
        "a.py",
        "@deco\ndef f(x, y):\n    z = x\n    return z\n",
        Language::Python,
    )
    .unwrap();
    let deco =
        descendant_of_kind(p.tree.root_node(), "decorated_definition").expect("deco node");

    assert!(p.find_parameters_node(&deco).is_some());
    assert!(p.function_body_node(&deco).is_some());
    assert!(!p.statements_in_function(&deco).is_empty());
    assert!(!p.statement_spans_in_function(&deco).is_empty());
    assert!(!p.return_value_nodes(&deco).is_empty());
}

#[test]
fn return_value_nodes_excludes_nested_function_returns() {
    // A nested `def` has the SAME kind as the outer, so the recursion fence must
    // be by node identity, not kind — else the nested return is mis-collected as
    // the outer's (corrupts contract postconditions; reachable for decorated fns
    // via the unwrap). Here outer returns 1; the nested `inner` returns 99.
    let p = ParsedFile::parse(
        "a.py",
        "@deco\ndef outer():\n    def inner():\n        return 99\n    return 1\n",
        Language::Python,
    )
    .unwrap();
    let deco =
        descendant_of_kind(p.tree.root_node(), "decorated_definition").expect("deco node");
    let returns = p.return_value_nodes(&deco);
    assert_eq!(
        returns.len(),
        1,
        "only the outer's return, not the nested fn's"
    );
    assert_eq!(returns[0].value_text.as_deref(), Some("1"));
}

#[test]
fn return_value_nodes_preserves_go_legacy_expression_list_and_adds_slots() {
    let p = ParsedFile::parse(
        "a.go",
        "package p\nfunc pair(a string) (string, error) { return a, nil }\n",
        Language::Go,
    )
    .unwrap();
    let func = p.all_functions().into_iter().next().expect("pair");
    let returns = p.return_value_nodes(&func);
    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].value_text.as_deref(), Some("a, nil"));
    assert_eq!(returns[0].value_kind.as_deref(), Some("expression_list"));
    assert_eq!(
        returns[0]
            .values
            .iter()
            .map(|value| value.text.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "nil"]
    );
}

#[test]
fn decorated_function_canonical_single_node() {
    let p =
        ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
    let fs: Vec<_> = p
        .all_functions()
        .into_iter()
        .filter(|n| p.language.function_name(n).map(|nm| p.node_text(&nm)) == Some("f".into()))
        .collect();

    assert_eq!(fs.len(), 1, "one canonical record for f");
    assert_eq!(
        fs[0].kind(),
        "decorated_definition",
        "the wrapper is canonical"
    );
}

#[test]
fn decorated_function_canonical_single_node_on_reconstruction_fallback() {
    let mut p =
        ParsedFile::parse("a.py", "@deco\ndef f():\n    return 1\n", Language::Python).unwrap();
    p.functions[0].kind_id = u16::MAX;

    let (nodes, used_fallback) = p.all_functions_inner();
    let fs: Vec<_> = nodes
        .into_iter()
        .filter(|n| p.language.function_name(n).map(|nm| p.node_text(&nm)) == Some("f".into()))
        .collect();

    assert!(used_fallback, "synthetic corruption must force fallback");
    assert_eq!(fs.len(), 1, "fallback must preserve wrapper-canonical f");
    assert_eq!(
        fs[0].kind(),
        "decorated_definition",
        "the wrapper is canonical"
    );
}

fn all_call_sites(pf: &ParsedFile) -> Vec<(usize, String)> {
    // Every call node's (start_byte, callee_name), pre-order — the keys the index must serve.
    fn walk(pf: &ParsedFile, node: tree_sitter::Node<'_>, out: &mut Vec<(usize, String)>) {
        if pf.language.is_call_node(node.kind()) {
            if let Some(name_node) = pf.call_function_name(&node) {
                out.push((node.start_byte(), pf.node_text(&name_node).to_string()));
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(pf, child, out);
        }
    }
    let mut out = Vec::new();
    walk(pf, pf.tree.root_node(), &mut out);
    out
}

fn index_texts(pf: &ParsedFile, sb: usize, name: &str) -> Vec<String> {
    pf.call_args_index()
        .by_call
        .get(&(sb, name.to_string()))
        .map(|spans| spans.iter().map(|a| pf.arg_text(a)).collect())
        .unwrap_or_default()
}

#[test]
fn call_args_index_matches_reference_all_languages() {
    // One fixture per Language::all() variant; each contains calls with varied arg shapes.
    let cases: Vec<(&str, Language, &str)> = vec![
        (
            "fn f(){ g(a, &b); o.m(x); a.b().c(d); h(); }",
            Language::Rust,
            "t.rs",
        ),
        ("func f(){ g(a, b); o.M(x); h() }", Language::Go, "t.go"),
        (
            "def f():\n    g(a, b)\n    o.m(x)\n    h()\n",
            Language::Python,
            "t.py",
        ),
        (
            "function f(){ g(a, b); o.m(x); h(); }",
            Language::JavaScript,
            "t.js",
        ),
        (
            "function f(x: number){ g(a, b); o.m(x); }",
            Language::TypeScript,
            "t.ts",
        ),
        (
            "const f = () => { g(a, b); o.m(x); };",
            Language::Tsx,
            "t.tsx",
        ),
        (
            "class C { void f(){ g(a, b); o.m(x); h(); } }",
            Language::Java,
            "T.java",
        ),
        ("void f(){ g(a, b); o->m(x); h(); }", Language::C, "t.c"),
        ("void f(){ g(a, b); o.m(x); h(); }", Language::Cpp, "t.cpp"),
        ("function f() g(a, b) o.m(x) end", Language::Lua, "t.lua"),
        ("locals { x = max(1, 2) }\n", Language::Terraform, "t.tf"),
        ("f() { cmd arg1 arg2; g; }\n", Language::Bash, "t.sh"),
    ];
    for (src, lang, path) in cases {
        let pf = ParsedFile::parse(path, src, lang).unwrap();
        let sites = all_call_sites(&pf);
        assert!(!sites.is_empty(), "no call sites parsed for {path}");
        for (sb, name) in sites {
            assert_eq!(
                index_texts(&pf, sb, &name),
                pf.call_argument_texts_at_reference(sb, &name),
                "mismatch in {path} at byte {sb} call `{name}`"
            );
        }
    }
}

#[test]
fn call_args_index_edge_cases() {
    // For each fixture, every call site: index == reference walk.
    let fixtures: &[(&str, Language, &str)] = &[
        ("fn f(){ g(&x, &&y); }", Language::Rust, "ref.rs"), // leading-& strip (repeated)
        ("fn f(){ a(b(c), d(e)); }", Language::Rust, "nest.rs"), // nested calls
        ("fn f(){ p(1); q(2); }", Language::Rust, "twoline.rs"), // two calls, one line
        ("fn f(){ z(); }", Language::Rust, "zero.rs"),       // zero-arg
        ("fn f(){ a.b().c(d); }", Language::Rust, "chain.rs"), // same start_byte, distinct names
        ("fn f(){ a.b().b(d); }", Language::Rust, "samechain.rs"), // same start_byte AND name -> outer wins
        (
            "fn f(){\n  obj\n    .m(\n      arg,\n    );\n}",
            Language::Rust,
            "multi.rs",
        ), // multi-line
    ];
    for (src, lang, path) in fixtures {
        let pf = ParsedFile::parse(path, src, *lang).unwrap();
        for (sb, name) in all_call_sites(&pf) {
            assert_eq!(
                index_texts(&pf, sb, &name),
                pf.call_argument_texts_at_reference(sb, &name),
                "{path} byte {sb} `{name}`"
            );
        }
    }
}

#[test]
fn call_argument_texts_at_production_matches_reference() {
    let cases: &[(&str, Language, &str)] = &[
        (
            "fn f(){ g(a, &b); o.m(x); a.b().c(d); a.b().b(z); }",
            Language::Rust,
            "p.rs",
        ),
        ("func f(){ g(a, b); o.M(x) }", Language::Go, "p.go"),
        (
            "def f():\n    g(a, b)\n    o.m(x)\n",
            Language::Python,
            "p.py",
        ),
    ];
    for (src, lang, path) in cases {
        let pf = ParsedFile::parse(path, src, *lang).unwrap();
        for (sb, name) in all_call_sites(&pf) {
            assert_eq!(
                pf.call_argument_texts_at(sb, &name),
                pf.call_argument_texts_at_reference(sb, &name),
                "{path} byte {sb} `{name}`"
            );
        }
    }
}

#[test]
fn call_args_index_name_mismatch_is_empty() {
    // Querying a real call's start_byte with the WRONG name returns empty (legacy parity).
    let pf = ParsedFile::parse("m.rs", "fn f(){ g(a, b); }", Language::Rust).unwrap();
    let (sb, name) = all_call_sites(&pf)
        .into_iter()
        .find(|(_, n)| n == "g")
        .unwrap();
    assert_eq!(index_texts(&pf, sb, "not_g"), Vec::<String>::new());
    assert_eq!(
        index_texts(&pf, sb, &name),
        pf.call_argument_texts_at_reference(sb, &name)
    );
}

#[test]
fn call_args_index_same_start_same_name_outer_wins() {
    // a.b().b(d): outer and inner call both start at byte(a) and are named `b`.
    // First-write-wins (pre-order = outer) must match the reference walk.
    let pf =
        ParsedFile::parse("s.rs", "fn f(){ a.b().b(outer_arg); }", Language::Rust).unwrap();
    // The shared start_byte is the byte of `a` inside the body.
    let a_byte = pf.source.find("a.b()").unwrap();
    let got = index_texts(&pf, a_byte, "b");
    assert_eq!(got, pf.call_argument_texts_at_reference(a_byte, "b"));
}

#[test]
fn candidate_fields_are_maximal_post_accessor_identifiers() {
    let got = candidate_fields_on_line("s.cb = f; t->cbx = g; obj.data->next = h; x = 3.14;");
    let want: BTreeSet<String> = ["cb", "cbx", "data", "next", "14"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(got, want); // "14" is harmless noise: digits are identifier chars,
                           // and no callee filter ever queries it
}

#[test]
fn candidate_fields_use_the_pinned_unicode_predicate() {
    // same predicate as the Level-4 call-site filter (char::is_alphanumeric || '_'),
    // Unicode-aware — `café` is ONE identifier.
    let got = candidate_fields_on_line("obj.café = handler; p->x_1 = g;");
    let want: BTreeSet<String> = ["café", "x_1"].iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want);
}

#[test]
fn level4_legacy_quirk_arrow_anywhere_priority() {
    // ->field ANYWHERE in the line beats a closer .field (quirk 1):
    // the .cb assignment to `f` is dropped because ->cb is found first.
    let src = "s.cb = f; t->cb = g;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
    assert_eq!(got, vec!["g".to_string()]);
}

#[test]
fn level4_legacy_quirk_prefix_consumption() {
    // ->cbx matches find("->cb"); scan position advances past it, and the
    // REAL earlier .cb assignment is never revisited (quirk 2).
    let src = "s.cb = f; t->cbx = g;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
    assert_eq!(got, Vec::<String>::new());
}

#[test]
fn level4_legacy_rhs_rules() {
    // single-= anchor (== rejected), &-strip, stop at ; , } ) and whitespace,
    // known_fns filter, BTreeSet dedup+sort.
    let src =
        "a.cb == f;\nb.cb = &handler;\nc.cb = handler, x;\nd.cb = unknown_fn;\ne.cb = handler;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["handler", "f"]));
    assert_eq!(got, vec!["handler".to_string()]); // deduped, == rejected, unknown filtered
}

#[test]
fn level4_legacy_designated_initializer_multi_field() {
    let src = "static struct ops o = { .open = do_open, .close = do_close };\n";
    assert_eq!(
        resolve_struct_field_assignment(src, "open", &fns(&["do_open", "do_close"])),
        vec!["do_open".to_string()]
    );
    assert_eq!(
        resolve_struct_field_assignment(src, "close", &fns(&["do_open", "do_close"])),
        vec!["do_close".to_string()]
    );
}

#[test]
fn function_table_captures_named_and_anonymous_in_query_order() {
    // JS: named fn + anonymous callback lambda (function_name() returns None for the latter)
    let src = "function alpha(a, b) { return a; }\nitems.forEach((x) => { use(x); });\n";
    let pf = ParsedFile::parse("t.js", src, Language::JavaScript).unwrap();
    let table = pf.functions();
    // Full captured sequence preserved, query order, including unnamed entries
    let direct = pf.all_functions();
    assert_eq!(table.len(), direct.len());
    for (info, node) in table.iter().zip(direct.iter()) {
        assert_eq!(info.start_byte, node.start_byte());
        assert_eq!(info.end_byte, node.end_byte());
        assert_eq!(info.kind_id, node.kind_id());
    }
    assert_eq!(table[0].name.as_deref(), Some("alpha"));
    assert_eq!(
        table[0].param_names,
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert!(table.iter().any(|f| f.name.is_none())); // the arrow callback
}

#[test]
fn function_table_rust_and_same_named_functions() {
    let src = "fn f(x: u32) -> u32 { x }\nmod a { pub fn f(y: u32) -> u32 { y } }\n";
    let pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
    let named: Vec<_> = pf
        .functions()
        .iter()
        .filter(|f| f.name.as_deref() == Some("f"))
        .collect();
    assert_eq!(named.len(), 2); // both kept, query order — no dedup/last-writer-wins
    assert_eq!(named[0].param_names, Some(vec!["x".to_string()]));
}

#[test]
fn all_functions_reconstructed_matches_direct_query_per_language() {
    // (source, language, path) for all supported languages; each has >=2 functions.
    let cases: Vec<(&str, Language, &str)> = vec![
        (
            "fn a() {}\nfn b(x: u32) { let _ = x; }",
            Language::Rust,
            "t.rs",
        ),
        ("func A() {}\nfunc B(x int) { _ = x }", Language::Go, "t.go"),
        (
            "def a():\n    pass\n\ndef b(x):\n    return x\n",
            Language::Python,
            "t.py",
        ),
        (
            "function a() {}\nitems.map((x) => x + 1);",
            Language::JavaScript,
            "t.js",
        ),
        (
            "function a(): void {}\nconst b = (x: number) => x + 1;",
            Language::TypeScript,
            "t.ts",
        ),
        (
            "function A(): JSX.Element { return <div/>; }\nconst B = () => <span/>;",
            Language::Tsx,
            "t.tsx",
        ),
        (
            "class K { void a() {} int b(int x) { return x; } }",
            Language::Java,
            "t.java",
        ),
        (
            "void a(void) {}\nint b(int x) { return x; }",
            Language::C,
            "t.c",
        ),
        (
            "class K { void a() {} };\nint b() { return 0; }",
            Language::Cpp,
            "t.cpp",
        ),
        (
            "function a() end\nlocal function b(x) return x end",
            Language::Lua,
            "t.lua",
        ),
        (
            "resource \"aws_s3_bucket\" \"a\" {}\nresource \"aws_s3_bucket\" \"b\" {}",
            Language::Terraform,
            "t.tf",
        ),
        (
            "a() { echo hi; }\nb() { echo bye; }",
            Language::Bash,
            "t.sh",
        ),
    ];
    for (src, lang, path) in cases {
        let pf = ParsedFile::parse(path, src, lang).unwrap();
        let direct = pf.all_functions_via_tree();
        let (reconstructed, used_fallback) = pf.all_functions_inner();
        // Anti-vacuous guard: a grammar bump that stops capturing functions for a
        // language would otherwise make this drift detector pass on empty sets.
        assert!(
            direct.len() >= 2,
            "{path}: fixture must parse to >=2 functions (got {})",
            direct.len()
        );
        assert!(!used_fallback, "{path}: reconstruction must not fall back");
        assert_eq!(direct.len(), reconstructed.len(), "{path}");
        for (d, r) in direct.iter().zip(reconstructed.iter()) {
            assert_eq!(
                (d.kind_id(), d.start_byte(), d.end_byte()),
                (r.kind_id(), r.start_byte(), r.end_byte()),
                "{path}"
            );
        }
    }
}

#[test]
fn all_functions_falls_back_to_direct_query_on_reconstruction_miss() {
    let src = "fn a() {}\nfn b() {}\n";
    let mut pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
    pf.functions[0].kind_id = u16::MAX; // synthetic corruption: no node can match
    let (nodes, used_fallback) = pf.all_functions_inner();
    assert!(used_fallback); // the drift detector
    assert_eq!(nodes.len(), 2); // full sequence via fallback — never silently skipped
}

#[test]
fn go_dot_import_marker_pins_single_grouped_and_named_shapes() {
    for source in [
        "package p\nimport . \"example.com/a\"\n",
        "package p\nimport (\n . \"example.com/a\"\n x \"example.com/x\"\n)\n",
    ] {
        let parsed = ParsedFile::parse("p.go", source, Language::Go).unwrap();
        assert!(parsed.go_has_dot_import(), "{source}");
        assert!(!parsed.extract_imports().contains_key("."), "{source}");
    }
    let named = ParsedFile::parse(
        "p.go",
        "package p\nimport x \"example.com/x\"\n",
        Language::Go,
    )
    .unwrap();
    assert!(!named.go_has_dot_import());
}

#[test]
fn go_type_parameter_binder_pins_function_and_method_receiver_shapes() {
    let parsed = ParsedFile::parse(
        "p.go",
        "package p\nfunc free[T any](v T) {}\ntype Store[U any] struct{}\nfunc (s *Store[T]) method(v T) {}\nfunc plain(v T) {}\nfunc nested(v T) { type Local[T any] struct{}; _ = Local[int]{} }\n",
        Language::Go,
    )
    .unwrap();
    let function = |name: &str| {
        parsed
            .all_functions()
            .into_iter()
            .find(|node| {
                parsed
                    .language
                    .function_name(node)
                    .is_some_and(|n| parsed.node_text(&n) == name)
            })
            .unwrap_or_else(|| panic!("missing {name}"))
    };
    assert!(parsed.go_type_parameter_binds_receiver(&function("free"), "T"));
    assert!(parsed.go_type_parameter_binds_receiver(&function("method"), "T"));
    assert!(!parsed.go_type_parameter_binds_receiver(&function("plain"), "T"));
    assert!(!parsed.go_type_parameter_binds_receiver(&function("nested"), "T"));
    assert!(!parsed.go_type_parameter_binds_receiver(&function("free"), "pkg.T"));
    assert!(!parsed.go_type_parameter_binds_receiver(&function("free"), "Store[T]"));
}
