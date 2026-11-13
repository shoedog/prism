//! #820 performance guard (deterministic, no wall clock): lexical resolution and the entry walk
//! of rules U and D must do work that grows about linearly with the owner's statement count.
//! The counter (`c_prune_resolve::WORK`) counts declarator visits of the resolver and node
//! visits of `entry_in_range`; a per-occurrence block rescan makes 4x statements cost ~16x.
use super::c_prune_resolve::WORK;
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;

fn chain_owner(n: usize) -> String {
    let mut s = String::from("int src();\nvoid f() {\n");
    for i in 0..=n {
        s.push_str(&format!("  int v{i} = src();\n"));
    }
    for i in 0..n {
        s.push_str(&format!("  v{i} = v{};\n", i + 1));
    }
    s.push_str("}\n");
    s
}

fn work(name: &str, language: Language, n: usize) -> (usize, usize) {
    let source = chain_owner(n);
    let parsed = ParsedFile::parse(name, &source, language).unwrap();
    WORK.with(|w| w.set(0));
    let refs = Producer::file_refs(&name.to_owned(), &parsed);
    (WORK.with(|w| w.get()), refs.edges.len())
}

#[test]
fn resolution_work_grows_sub_quadratically() {
    for (name, language) in [("a.cc", Language::Cpp), ("a.c", Language::C)] {
        let (small, e1) = work(name, language, 100);
        let (large, e4) = work(name, language, 400);
        assert!(small > 0 && e1 > 0 && e4 > e1, "{name}: probe did no work");
        // Linear is 4x; quadratic is 16x. Allow a generous constant.
        assert!(
            large < small * 8,
            "{name}: work grew {small} -> {large} for 4x statements (quadratic?)"
        );
    }
}

/// The indexed entry query answers exactly what the walk answers, for every window of every
/// compound region of a source with labels, direct and nested `case`s.
#[test]
fn entry_index_matches_the_walk() {
    use super::c_prune_resolve::ResolveCache;
    let src = "int f(int c){int x=0; a: x=1; switch(c){case 1: x=2; if(c){ b: x=3; } case 2: switch(x){case 3: x=4;} break; default: x=5;} while(c){ x=6; } return x;}";
    let parsed = ParsedFile::parse("a.c", src, Language::C).unwrap();
    let cache = ResolveCache::default();
    let mut stack = vec![parsed.tree.root_node()];
    let mut regions = 0;
    while let Some(n) = stack.pop() {
        let mut cursor = n.walk();
        stack.extend(n.children(&mut cursor));
        if !matches!(n.kind(), "compound_statement" | "case_statement") {
            continue;
        }
        regions += 1;
        for from in 0..src.len() {
            for to in (from..=src.len()).step_by(3) {
                assert_eq!(
                    cache.entry_in_window(n, from, to),
                    super::c_prune_write::entry_in_range(n, from, to),
                    "region {}..{} window {from}..{to}",
                    n.start_byte(),
                    n.end_byte()
                );
            }
        }
    }
    assert!(regions >= 5);
}
