//! #630 slice 6b fold 2: UCN-spelled macro bodies make the repo-wide census incomplete, so
//! a non-ASCII local named by one keeps its pairs (and the control prunes).
use super::Producer;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::languages::Language;

fn counts(files: &[(&str, &str)], target: &str) -> (usize, usize) {
    let parsed: std::collections::BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(n, s)| (n.to_string(), ParsedFile::parse(n, s, Language::C).unwrap()))
        .collect();
    let c_census = CMacroCensus::from_files(&parsed);
    let file = target.to_owned();
    let p = &parsed[target];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &c_census);
    (before.edges.len(), after.edges.len())
}

const OWNER: &str = "long src(void);\nvoid sink(long);\nvoid track(long *p);\nlong peek(void);\nvoid f(void) {\n    long \u{e9} = src();\n    TRACK();\n    \u{e9} = peek();\n    sink(\u{e9});\n}\n";

#[test]
fn fold2_delimited_ucn_macro_body_keeps_the_owner() {
    let (b, a) = counts(&[("a.c", OWNER)], "a.c");
    assert!(a < b, "control: no TRACK definition prunes ({b} -> {a})");
    for body in [
        "track(&\\u{e9})",
        "\\u{00E9} = saved",
        "track(&\\N{LATIN SMALL LETTER E WITH ACUTE})",
    ] {
        let hdr = format!("#define TRACK() {body}\n");
        let (b, a) = counts(&[("a.c", OWNER), ("track.h", hdr.as_str())], "a.c");
        assert_eq!(a, b, "{body}: the census is incomplete, nothing is dropped");
    }
}
