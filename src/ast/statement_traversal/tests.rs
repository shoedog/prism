use super::*;
use crate::languages::Language;
use std::cell::{Cell, RefCell};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static WORK: Cell<[(usize, usize); 6]> = const { Cell::new([(0, 0); 6]) };
    static STATES: RefCell<HashSet<(usize, usize)>> = RefCell::new(HashSet::new());
    static REPEATS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record(mode: usize, node: usize, children: usize) {
    if !ACTIVE.with(Cell::get) {
        return;
    }
    if !STATES.with(|s| s.borrow_mut().insert((node, mode))) {
        REPEATS.with(|r| r.set(r.get() + 1));
    }
    WORK.with(|w| {
        let mut counts = w.get();
        counts[mode].0 += 1;
        counts[mode].1 += children;
        w.set(counts);
    });
}

fn extract(p: &ParsedFile, f: Node<'_>, mode: usize) {
    match mode {
        0 => {
            p.statements_in_function(&f);
        }
        1 => {
            p.statement_spans_in_function(&f);
        }
        2 => {
            p.statement_arms_in_function(&f);
        }
        _ => unreachable!(),
    }
}

fn linear(mode: usize) {
    let mut failures = Vec::new();
    for bare_blocks in [false, true] {
        // Keep the unoptimized bare-block control small: it compounds both forks.
        let depths: &[usize] = if bare_blocks {
            &[0, 1, 2, 4]
        } else {
            &[0, 1, 4, 8, 12]
        };
        for &depth in depths {
            let source = format!(
                "function f(x) {{\n{}x++;\n{}\n}}",
                (if bare_blocks {
                    "if (x) { {\n"
                } else {
                    "if (x) {\n"
                })
                .repeat(depth),
                (if bare_blocks { "} }\n" } else { "}\n" }).repeat(depth)
            );
            let p = ParsedFile::parse("cost.js", &source, Language::JavaScript).unwrap();
            assert_eq!(p.parse_error_count, 0);
            WORK.with(|w| w.set([(0, 0); 6]));
            STATES.with(|s| s.borrow_mut().clear());
            REPEATS.with(|r| r.set(0));
            ACTIVE.with(|a| a.set(true));
            extract(&p, p.all_functions()[0], mode);
            ACTIVE.with(|a| a.set(false));
            let work = WORK.with(Cell::get);
            let visits: usize = work.iter().map(|c| c.0).sum();
            let children: usize = work.iter().map(|c| c.1).sum();
            let nodes = p.parse_node_count;
            println!("mode={mode} bare_blocks={bare_blocks} depth={depth} nodes={nodes} visits={visits} children={children}");
            // Each node has at most two modes; each syntax edge is scanned at most twice.
            if visits > 2 * p.parse_node_count
                || children > 2 * p.parse_node_count
                || REPEATS.with(Cell::get) != 0
            {
                failures.push((bare_blocks, depth, p.parse_node_count, visits, children));
            }
        }
    }
    assert!(failures.is_empty(), "nonlinear traversal: {failures:?}");
}

#[test]
fn statements_visit_linear() {
    linear(0);
}
#[test]
fn spans_visit_linear() {
    linear(1);
}
#[test]
fn arms_visit_linear() {
    linear(2);
}

fn compare_node(p: &ParsedFile, f: Node<'_>) {
    assert_eq!(
        p.statements_in_function(&f),
        p.legacy_statements_in_function(&f),
        "statements {}",
        p.source
    );
    assert_eq!(
        p.statement_spans_in_function(&f),
        p.legacy_statement_spans_in_function(&f),
        "spans {}",
        p.source
    );
    let actual = p.statement_arms_in_function(&f);
    let old = p.legacy_statement_arms_in_function(&f);
    assert_eq!(actual.of_line, old.of_line, "of_line {}", p.source);
    assert_eq!(
        actual.subtree_last, old.subtree_last,
        "subtree_last {}",
        p.source
    );
    for (line, _) in p.statements_in_function(&f) {
        assert!(actual.of_line.contains_key(&line));
    }
}

fn compare(lang: Language, source: &str) {
    let p = ParsedFile::parse("identity", source, lang).unwrap();
    assert_eq!(p.parse_error_count, 0, "{lang:?}: {source}");
    let functions = p.all_functions();
    assert!(!functions.is_empty(), "{lang:?}: {source}");
    for f in functions {
        compare_node(&p, f);
    }
    // Exercise the decorated_definition API input as well as the unwrapped table node.
    if lang == Language::Python {
        let mut cursor = p.tree.root_node().walk();
        for child in p.tree.root_node().named_children(&mut cursor) {
            if child.kind() == "decorated_definition" {
                compare_node(&p, child);
            }
        }
    }
}

fn c_like() -> &'static str {
    "int f(int x) {\n if (x) { if (x > 1) { x++; } } else if (x < 0) { x--; } else {}\n while (x) { x--; }\n for (int i=0;i<2;i++) { x++; }\n do { x++; } while (x < 1);\n switch(x) { case 1: x++; break; default: x--; }\n { { x++; } x--; }\n return x;\n}\nint empty() {}"
}
fn js_like() -> &'static str {
    "function f(x) {\n if (x) { if (x > 1) { x++; } } else if (x < 0) { x--; } else {}\n while (x) { x--; }\n for (let i=0;i<2;i++) { x++; }\n switch(x) { case 1: x++; break; default: x--; }\n try { x++; } catch (e) { x--; } finally { x++; }\n { { x++; } x--; }\n if (x) x++; else x--;\n label: while (x) { break label; }\n const nested = () => { if (x) { x++; } };\n return x;\n}\nfunction empty() {}"
}

#[test]
fn identity_javascript() {
    compare(Language::JavaScript, js_like());
}
#[test]
fn identity_typescript() {
    compare(Language::TypeScript, js_like());
}
#[test]
fn identity_tsx() {
    compare(Language::Tsx, js_like());
    compare(
        Language::Tsx,
        "function view(x: boolean) { if(x) { return <div/>; } else { return <span/>; } }",
    );
}
#[test]
fn identity_c() {
    compare(Language::C, c_like());
}
#[test]
fn identity_cpp() {
    compare(Language::Cpp, c_like());
    compare(
        Language::Cpp,
        "int f(int x) { try { if(x) { x++; } } catch (...) { x--; } return x; }",
    );
}
#[test]
fn identity_java() {
    compare(Language::Java, &format!("class A {{ {} int g(int x) {{ try {{ x++; }} catch(Exception e) {{ x--; }} finally {{ x++; }} synchronized(this) {{ x++; }} return x; }} }}", c_like()));
}
#[test]
fn identity_python() {
    compare(Language::Python, "@decorate\ndef f(x):\n    if x:\n        if x > 1:\n            x += 1\n    elif x < 0:\n        x -= 1\n    else:\n        pass\n    while x:\n        x -= 1\n    for i in range(2):\n        x += i\n    try:\n        x += 1\n    except ValueError:\n        x -= 1\n    finally:\n        x += 1\n    with manager():\n        x += 1\n    if x: x += 1; x += 2\n    return x\ndef empty():\n    pass\n");
}
#[test]
fn identity_rust() {
    compare(Language::Rust, "fn f(mut x: i32) -> i32 {\n if x > 0 { if x > 1 { x += 1; } } else if x < 0 { x -= 1; } else {}\n while x > 0 { x -= 1; }\n for i in 0..2 { x += i; }\n loop { break; }\n match x { 0 => { x += 1; }, 1 => x -= 1, _ => {} }\n unsafe { { x += 1; } }\n let _f = async { if x > 0 { x += 1; } };\n x\n}\nfn empty() {}");
}
#[test]
fn identity_go() {
    compare(Language::Go, "package p\nfunc f(x int) int {\n if x > 0 { if x > 1 { x++ } } else if x < 0 { x-- } else {}\n for x > 0 { x-- }\n switch x { case 1: x++; default: x-- }\n select { default: x++ }\n { { x++ }; x-- }\n return x\n}\nfunc empty() {}\n");
}
#[test]
fn identity_bash() {
    compare(Language::Bash, "f() {\n if true; then\n  if false; then echo a; fi\n elif false; then echo b\n else echo c; fi\n for x in a b; do echo $x; done\n while false; do echo d; done\n case $x in a) echo a;; *) echo b;; esac\n ( (echo nested); echo tail ) > /dev/null\n { echo brace; }\n}\nempty() { :; }\n");
}
#[test]
fn identity_lua() {
    compare(Language::Lua, "function f(x)\n if x > 0 then\n  if x > 1 then x = x + 1 end\n elseif x < 0 then x = x - 1\n else end\n while x > 0 do x = x - 1 end\n for i=1,2 do x=x+i end\n repeat x=x+1 until x > 0\n do do x=x+1 end x=x-1 end\n return x\nend\nfunction empty() end\n");
}

#[test]
fn go_visits_are_linear_in_all_modes() {
    for mode in 0..3 {
        for (depth, bare_blocks) in [0, 1, 4, 8]
            .into_iter()
            .flat_map(|d| [(d, false), (d, true)])
        {
            let mut body = leaf(Language::Go).to_owned();
            for _ in 0..depth {
                if bare_blocks {
                    body = wrap(Language::Go, &body, 2);
                }
                body = wrap(Language::Go, &body, 0);
            }
            let p =
                ParsedFile::parse("cost.go", &program(Language::Go, &body), Language::Go).unwrap();
            assert_eq!(p.parse_error_count, 0);
            WORK.with(|w| w.set([(0, 0); 6]));
            STATES.with(|s| s.borrow_mut().clear());
            REPEATS.with(|r| r.set(0));
            ACTIVE.with(|a| a.set(true));
            extract(&p, p.all_functions()[0], mode);
            ACTIVE.with(|a| a.set(false));
            let work = WORK.with(Cell::get);
            assert!(work.iter().map(|c| c.0).sum::<usize>() <= 2 * p.parse_node_count);
            assert!(work.iter().map(|c| c.1).sum::<usize>() <= 2 * p.parse_node_count);
            assert_eq!(
                REPEATS.with(Cell::get),
                0,
                "mode={mode} depth={depth} bare_blocks={bare_blocks}"
            );
        }
    }
}

#[test]
fn identity_terraform_bodyless_blocks() {
    let p = ParsedFile::parse(
        "bodyless.tf",
        "locals { x = 1 }\nresource \"kind\" \"name\" { value = local.x }",
        Language::Terraform,
    )
    .unwrap();
    assert_eq!(p.parse_error_count, 0);
    let owners = p.all_functions();
    assert!(!owners.is_empty());
    for owner in owners {
        assert!(p.function_body_node(&owner).is_none());
        compare_node(&p, owner);
        assert!(p.statements_in_function(&owner).is_empty());
        assert!(p.statement_spans_in_function(&owner).is_empty());
        let arms = p.statement_arms_in_function(&owner);
        assert!(arms.of_line.is_empty());
        assert_eq!(arms.subtree_last, vec![0]);
    }
}

// Three choices at each level, enumerating all 3^d wrapper sequences at d<=4.
// Seeded random trees below also cover siblings and binary branches.
fn wrap(lang: Language, body: &str, choice: usize) -> String {
    match lang {
        Language::Python => {
            let body = body
                .lines()
                .map(|l| format!("    {l}\n"))
                .collect::<String>();
            match choice {
                0 => format!("if x:\n{body}"),
                1 => format!("while x:\n{body}"),
                _ => format!("if x:\n{body}else:\n    pass\n"),
            }
        }
        Language::Lua => match choice {
            0 => format!("if x then\n{body}end\n"),
            1 => format!("while x do\n{body}end\n"),
            _ => format!("do\n{body}end\n"),
        },
        Language::Bash => match choice {
            0 => format!("if true; then\n{body}fi\n"),
            1 => format!("while false; do\n{body}done\n"),
            _ => format!("(\n{body})\n"),
        },
        Language::Rust | Language::Go => match choice {
            0 => format!("if x > 0 {{\n{body}}}\n"),
            1 => format!(
                "{} x > 0 {{\n{body}}}\n",
                if lang == Language::Go { "for" } else { "while" }
            ),
            _ => format!("{{\n{body}}}\n"),
        },
        _ => match choice {
            0 => format!("if (x) {{\n{body}}}\n"),
            1 => format!("while (x) {{\n{body}}}\n"),
            _ => format!("{{\n{body}}}\n"),
        },
    }
}
fn program(lang: Language, body: &str) -> String {
    match lang {
        Language::Python => format!(
            "def f(x):\n{}",
            body.lines()
                .map(|l| format!("    {l}\n"))
                .collect::<String>()
        ),
        Language::Lua => format!("function f(x)\n{body}end\n"),
        Language::Bash => format!("f() {{\n{body}}}\n"),
        Language::Rust => format!("fn f(mut x: i32) {{\n{body}}}\n"),
        Language::Go => format!("package p\nfunc f(x int) {{\n{body}}}\n"),
        Language::Java => format!("class A {{ void f(int x) {{\n{body}}} }}\n"),
        Language::C | Language::Cpp => format!("void f(int x) {{\n{body}}}\n"),
        _ => format!("function f(x) {{\n{body}}}\n"),
    }
}
fn leaf(lang: Language) -> &'static str {
    match lang {
        Language::Python | Language::Lua => "x = 1\n",
        Language::Bash => "echo x\n",
        _ => "x = 1;\n",
    }
}
fn random_tree(lang: Language, seed: &mut u64, depth: usize) -> String {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    if depth == 0 {
        return leaf(lang).to_owned();
    }
    let choice = (*seed >> 32) as usize % 3;
    let a = random_tree(lang, seed, depth - 1);
    let b = if *seed & 1 == 0 {
        random_tree(lang, seed, depth - 1)
    } else {
        leaf(lang).to_owned()
    };
    wrap(lang, &(a + &b), choice)
}
#[test]
fn identity_generated_exhaustive_and_random() {
    for lang in [
        Language::JavaScript,
        Language::TypeScript,
        Language::Tsx,
        Language::C,
        Language::Cpp,
        Language::Go,
        Language::Java,
        Language::Python,
        Language::Rust,
        Language::Bash,
        Language::Lua,
    ] {
        for depth in 0..=4 {
            for mut code in 0..3usize.pow(depth) {
                let mut body = leaf(lang).to_owned();
                for _ in 0..depth {
                    body = wrap(lang, &body, code % 3);
                    code /= 3;
                }
                compare(lang, &program(lang, &body));
            }
        }
        for initial in 0..32 {
            let mut seed = initial;
            let body = random_tree(lang, &mut seed, 4);
            compare(lang, &program(lang, &body));
        }
    }
}

#[test]
fn identity_154_line_c_fixture() {
    let source = include_str!("../../../tests/fixtures/c/large_function.c");
    assert_eq!(source.lines().count(), 154);
    compare(Language::C, source);
}

/// Parent context is deliberately absent from Visits. Test both modes after a
/// completed pass with a different inherited arm; neither may mutate any output.
#[test]
fn completed_arm_state_is_context_independent() {
    let p = ParsedFile::parse(
        "context.js",
        "function f(x) { if (x) { { x++; } if(x) { x--; } } else {} }",
        Language::JavaScript,
    )
    .unwrap();
    let f = p.all_functions()[0];
    let node = f.child_by_field_name("body").unwrap();
    for nested in [false, true] {
        let mut visits = Visits::default();
        let mut alloc = ArmAllocator {
            next: 1,
            seen: BTreeMap::new(),
        };
        let mut arms = LexicalArms {
            of_line: BTreeMap::new(),
            subtree_last: vec![0],
        };
        if nested {
            p.collect_nested_statement_arms(node, &mut visits, 0, &mut alloc, &mut arms);
        } else {
            p.collect_statement_arms(node, &mut visits, 0, &mut alloc, &mut arms);
        }
        assert!(!arms.of_line.is_empty());
        assert!(alloc.next > 1);
        let before = (
            arms.of_line.clone(),
            arms.subtree_last.clone(),
            alloc.next,
            alloc.seen.clone(),
        );
        // A fresh visit set forces a full replay, proving the effect-level
        // no-op independently of the guard that production uses to skip it.
        let mut replay = Visits::default();
        if nested {
            p.collect_nested_statement_arms(node, &mut replay, 999, &mut alloc, &mut arms);
        } else {
            p.collect_statement_arms(node, &mut replay, 999, &mut alloc, &mut arms);
        }
        assert_eq!(
            before,
            (
                arms.of_line.clone(),
                arms.subtree_last.clone(),
                alloc.next,
                alloc.seen.clone()
            )
        );
        // Also exercise the guarded replay with its original visit set.
        if nested {
            p.collect_nested_statement_arms(node, &mut visits, 999, &mut alloc, &mut arms);
        } else {
            p.collect_statement_arms(node, &mut visits, 999, &mut alloc, &mut arms);
        }
        assert_eq!(
            before,
            (arms.of_line, arms.subtree_last, alloc.next, alloc.seen)
        );
    }
}
