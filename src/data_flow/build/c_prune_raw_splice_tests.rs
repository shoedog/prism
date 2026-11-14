//! #630 slice 6d-1 fold 1: a raw string with an interior splice. The owner-side readers see
//! spliced text, so an owner holding one is off, and a directive holding one fails the census.
use super::Producer;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::languages::Language;

fn run(src: &str) -> (Vec<(String, usize, usize)>, Vec<(String, usize, usize)>) {
    let p = ParsedFile::parse("a.cc", src, Language::Cpp).unwrap();
    let parsed = std::iter::once(("a.cc".to_string(), p)).collect();
    let census = CMacroCensus::from_files(&parsed);
    let file = "a.cc".to_owned();
    let p = &parsed[&file];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &census);
    let key = |e: &crate::data_flow::FlowEdge| (e.from.path.base.clone(), e.from.line, e.to.line);
    (
        before.edges.iter().map(key).collect(),
        after.edges.iter().map(key).collect(),
    )
}

fn counts(src: &str) -> (usize, usize) {
    let (b, a) = run(src);
    (b.len(), a.len())
}

#[test]
fn directive_with_a_spliced_raw_string_keeps_the_pair() {
    // A reads `q` (the literal ends at the second `)"` as written).
    let src = "int src();\nvoid sink(int);\n#define A (R\"(x)\\\n\" /* )\", q + 1)\nint f() {\n    int q = src();\n    q = A;\n    sink(q + 1);\n    return 0;\n}\n";
    let (b, a) = run(src);
    let pair = ("q".to_string(), 6, 7);
    assert!(
        b.contains(&pair),
        "control: the legacy producer has the pair"
    );
    assert!(a.contains(&pair), "q@6 -> q@7 must survive: {a:?}");
}

#[test]
fn owner_with_a_spliced_raw_string_is_off() {
    let own = "int src();\nvoid sink(int);\nint f() {\n    const char *s = R\"(a\\\nb)\";\n    int x = src();\n    x = 1;\n    sink(x + 1);\n    return s[0];\n}\n";
    let (b, a) = counts(own);
    assert_eq!(a, b, "owner off: nothing dropped ({b} -> {a})");
    // Control: the same owner with no splice prunes the overwritten def.
    let ctl = own.replace("a\\\nb", "ab");
    let (b, a) = counts(&ctl);
    assert!(a < b, "control prunes ({b} -> {a})");
}

#[test]
fn vt_leader_directive_with_a_spliced_raw_string_keeps_the_pair() {
    for lead in ["\x0b", "\x0c", "/*c*/ "] {
        let src = format!("int src();\nvoid sink(int);\n{lead}#define A (R\"(x)\\\n\" /* )\", q + 1)\nint f() {{\n    int q = src();\n    q = A;\n    sink(q + 1);\n    return 0;\n}}\n");
        let (b, a) = run(&src);
        let pair = ("q".to_string(), 6, 7);
        assert!(b.contains(&pair));
        assert!(
            a.contains(&pair),
            "{lead:?}: q@6 -> q@7 must survive: {a:?}"
        );
    }
}

#[test]
fn multi_line_comment_in_an_owner_does_not_change_the_verdict() {
    // Control: the comment spans lines inside the owner; the overwritten def still prunes.
    let with = "int src();\nvoid sink(int);\nint f() {\n    int x = src(); /* a\n b */\n    x = 1;\n    sink(x + 1);\n    return 0;\n}\n";
    let (b, a) = counts(with);
    assert!(a < b, "comment in owner: prunes as without ({b} -> {a})");
}

#[test]
fn pragma_omp_after_a_multi_line_comment_trips_g2() {
    let src = "int src();\nvoid sink(int);\nint f() {\n    int x = src();\n    /* a\n b */ #pragma omp parallel\n    x = 1;\n    sink(x + 1);\n    return 0;\n}\n";
    let (b, a) = counts(src);
    assert_eq!(a, b, "G2: owner off, nothing dropped ({b} -> {a})");
}
