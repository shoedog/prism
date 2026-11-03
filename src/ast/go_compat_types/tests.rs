use super::*;
use std::path::Path;

fn compare(path: &str, source: &str) -> usize {
    let p = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut cursor = p.tree.walk();
    let mut identifiers = 0;
    loop {
        let n = cursor.node();
        // Compare all nodes as well as @ident candidates: selectors feed the
        // broad-use adapter and recovery boundaries must have identical rules.
        assert_eq!(
            p.is_go_legacy_type_context(n),
            p.legacy_go_type_context(n),
            "{path}:{}:{} {} {:?}",
            n.start_position().row + 1,
            n.start_position().column,
            n.kind(),
            p.node_text(&n)
        );
        identifiers += usize::from(n.kind() == "identifier");
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                assert_eq!(
                    p.go_compat.contexts.get().unwrap().visits,
                    2 * p.parse_node_count
                );
                return identifiers;
            }
        }
    }
}

fn fixtures(dir: &Path, count: &mut (usize, usize)) {
    let mut entries = std::fs::read_dir(dir)
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "private-corpus"
            || name.starts_with("f-")
            || name.starts_with("F-private")
            || name.to_ascii_lowercase().contains("secbench")
        {
            continue;
        }
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            fixtures(&path, count);
        } else if path.extension().is_some_and(|e| e == "go") {
            count.0 += 1;
            count.1 += compare(
                &path.to_string_lossy(),
                &std::fs::read_to_string(&path).unwrap(),
            );
        }
    }
}

#[test]
fn context_index_matches_ancestor_oracle_all_go_fixtures() {
    let mut count = (0, 0);
    fixtures(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        &mut count,
    );
    assert!(
        count.0 > 20 && count.1 > 100,
        "nonempty fixture coverage: {count:?}"
    );
    println!(
        "Go type-context differential: {} fixtures, {} identifiers",
        count.0, count.1
    );
}

#[test]
fn context_index_matches_generic_new_and_recovery_shapes() {
    let mut identifiers = 0;
    // Call and non-call indices, pointers, parentheses, qualified arguments,
    // nested instantiations, real value indices and non-type operands.
    for expression in [
        "F[T]()",
        "pkg.F[T](x,x)",
        "x.M[T]()",
        "pkg.F[*T](x,x)",
        "pkg.F[(T)](x,x)",
        "pkg.F[types.Item](x,x)",
        "pkg.F[T]",
        "pkg.F[*T]",
        "handlers[i]()",
        "table[k]()",
        "handlers[keys[i]]()",
        "matrix[i][j]()",
        "a[b[obj.Field]]",
        "a[b[i]]",
        "a[obj.Indices[i]]",
        "obj.Values[b[i]]",
        "obj.Values[i][j]",
        "obj.Values[obj.Indices[i]]",
        "matrix[i][j]",
        "obj.Values[i][other.Field]",
        "matrix[i][other.Field]",
        "obj.Values[other.Field][j]",
        "obj.Values[b[*ptr]]",
        "*obj.Values[i][j]",
        "a[(b[i])]",
        "a[b[i]][j]",
        "a[i]",
        "a[b[0]]",
        "a[b[i+1]]",
        "obj.Values[obj.Other.Indices[i]]",
        "matrix[i][j][k]",
        "new(string(x))",
        "new(int(3))",
        "new(T{})",
        "make([]int, 3)",
        "new[int](x)",
        "make[int](x)",
        "new[i]",
        "new[0].name",
        "make[k]",
        "new(",
        "a[b[",
        "a[func() { sink(x) }()]",
        "a[(func() { sink(x) })]",
    ] {
        identifiers += compare(
            "synthetic.go",
            &format!("package p\nfunc f() {{ sink({expression}) }}\n"),
        );
    }
    for source in [
        "package p\ntype Alias[T any] = []T\n",
        "package p\nvar K = 1\ntype Set[U comparable] struct{}\nfunc (s *Set[K]) Run(m map[K]int) { lib.Keys[K](m,0) }\n",
        "package p\nfunc f() { if x := F[T](); x != nil { sink(a[b[i]]) }; switch { case true: sink(a[i]) }; select { case x := <-ch: sink(x) } }\n",
    ] { identifiers += compare("synthetic.go", source); }
    assert!(identifiers > 100);
    println!("Go synthetic type-context differential: {identifiers} identifiers");
}

#[test]
fn context_index_is_linear_and_reused_and_non_go_stays_lazy() {
    use std::time::Instant;
    for size in [16, 64, 256] {
        let mut source = "package p\nfunc f() {\n".to_string();
        for i in 0..size {
            source += &format!("sink(a[b[key{i}]], pkg.F[T](x,x), a[key{i}])\n");
        }
        source += "}\n";
        let p = ParsedFile::parse("cost.go", &source, Language::Go).unwrap();
        let mut nodes = Vec::new();
        let mut pending = vec![p.tree.root_node()];
        while let Some(n) = pending.pop() {
            if n.kind() == "identifier" {
                nodes.push(n);
            }
            let mut c = n.walk();
            pending.extend(n.children(&mut c));
        }
        let started = Instant::now();
        let old = nodes
            .iter()
            .map(|&n| p.legacy_go_type_context(n))
            .collect::<Vec<_>>();
        let old_time = started.elapsed();
        let started = Instant::now();
        let new = nodes
            .iter()
            .map(|&n| p.is_go_legacy_type_context(n))
            .collect::<Vec<_>>();
        let cold_time = started.elapsed();
        assert_eq!(new, old);
        let index = p.go_compat.contexts.get().unwrap();
        assert_eq!(index.visits, 2 * p.parse_node_count);
        let started = Instant::now();
        for _ in 0..4 {
            for (&node, &expected) in nodes.iter().zip(&old) {
                assert_eq!(p.is_go_legacy_type_context(node), expected);
                assert!(std::ptr::eq(index, p.go_compat.contexts.get().unwrap()));
            }
        }
        println!("size={size} identifiers={} nodes={} cursor_visits={} old={old_time:?} cold={cold_time:?} warm4={:?}", nodes.len(), p.parse_node_count, index.visits, started.elapsed());
    }
    let p = ParsedFile::parse(
        "other.js",
        "function f(x) { return x; }",
        Language::JavaScript,
    )
    .unwrap();
    assert!(!p.is_go_legacy_type_context(p.tree.root_node()));
    assert!(p.go_compat.contexts.get().is_none());
}
