use super::*;
use crate::languages::Language;

fn parse(srcs: &[(&str, &str)], lang: Language) -> BTreeMap<String, ParsedFile> {
    srcs.iter()
        .map(|(p, s)| (p.to_string(), ParsedFile::parse(p, s, lang).unwrap()))
        .collect()
}

fn indirect(cg: &CallGraph, caller: &str) -> Vec<String> {
    let mut out: Vec<String> = cg
        .calls
        .iter()
        .filter(|(id, _)| id.name == caller)
        .flat_map(|(_, sites)| sites.iter())
        .filter(|s| s.origin == CallSiteOrigin::IndirectResolution)
        .map(|s| s.callee_name.clone())
        .collect();
    out.sort();
    out
}

fn c_sites(
    cg: &CallGraph,
    files: &BTreeMap<String, ParsedFile>,
) -> (Vec<String>, go::callback::GoLevel3Telemetry) {
    let (sites, telemetry) = cg.compute_indirect_call_sites(files, false);
    let mut names: Vec<String> = sites.into_iter().map(|(_, s)| s.callee_name).collect();
    names.sort();
    (names, telemetry)
}

#[test]
fn level2_function_local_table_resolves_each_entry_once() {
    let files = parse(
        &[(
            "a.c",
            "void h1(void) {}\nvoid h2(void) {}\nvoid run(void) {\n    void (*handlers[])(void) = { h1, h2 };\n    handlers[0]();\n}\n",
        )],
        Language::C,
    );
    let cg = CallGraph::build(&files);
    assert_eq!(indirect(&cg, "run"), ["h1", "h2"]);
    // A table found in the function body is not scanned again at file scope, so the
    // raw producer output holds each entry once rather than twice.
    let (names, telemetry) = c_sites(&cg, &files);
    assert_eq!(names, ["h1", "h2"]);
    assert_eq!(telemetry.candidates, 0);
}

#[test]
fn level2_file_scope_table_resolves_when_the_function_holds_no_initializer() {
    let files = parse(
        &[(
            "a.c",
            "void h1(void) {}\nvoid h2(void) {}\nvoid (*handlers[])(void) = { h1, h2 };\nvoid run(void) {\n    handlers[1]();\n}\n",
        )],
        Language::C,
    );
    let cg = CallGraph::build(&files);
    assert_eq!(indirect(&cg, "run"), ["h1", "h2"]);
    assert_eq!(c_sites(&cg, &files).0, ["h1", "h2"]);
}

#[test]
fn indirect_producer_skips_callers_whose_file_is_not_supplied() {
    let files = parse(
        &[(
            "a.c",
            "void h1(void) {}\nvoid (*handlers[])(void) = { h1 };\nvoid run(void) {\n    handlers[0]();\n}\n",
        )],
        Language::C,
    );
    let cg = CallGraph::build(&files);
    assert_eq!(c_sites(&cg, &files).0, ["h1"]);
    assert!(c_sites(&cg, &BTreeMap::new()).0.is_empty());

    // The same holds for Go callback candidates (Level 3): no parsed file, no candidate.
    let go = parse(
        &[(
            "a.go",
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
        )],
        Language::Go,
    );
    let cg = CallGraph::build(&go);
    assert_eq!(c_sites(&cg, &go).1.candidates, 1);
    let (names, telemetry) = c_sites(&cg, &BTreeMap::new());
    assert!(names.is_empty());
    assert_eq!(telemetry.candidates, 0);
}

#[test]
fn level1_pointer_resolves_only_from_source_inside_the_caller() {
    let files = parse(
        &[(
            "a.c",
            "void target(void) {}\nvoid run(void) {\n    void (*fp)(void) = target;\n    fp();\n}\n",
        )],
        Language::C,
    );
    let mut cg = CallGraph::build(&files);
    assert_eq!(indirect(&cg, "run"), ["target"]);

    // A call-site offset past the end of its caller has no source prefix to scan.
    let caller = cg.calls.keys().find(|id| id.name == "run").unwrap().clone();
    let sites = cg.calls.remove(&caller).unwrap();
    cg.calls.insert(
        caller,
        sites
            .into_iter()
            .filter(|s| s.origin == CallSiteOrigin::Source)
            .map(|mut s| {
                s.start_byte = usize::MAX;
                s
            })
            .collect(),
    );
    assert!(c_sites(&cg, &files).0.is_empty());
}

const FIELD_FIXTURE: &str = "struct S { void (*cb)(void); void (*ops[2])(void); };\n\
void target(void) {}\nvoid other(void) {}\n\
void setup(struct S t) { t.cb = other; }\n\
void run(struct S s) {\n    void (*cb)(void) = target;\n    s.cb();\n}\n\
void run2(struct S s) {\n    s.cb();\n}\n";

#[test]
fn level4_field_callback_defers_to_a_same_line_level1_resolution() {
    let files = parse(&[("a.c", FIELD_FIXTURE)], Language::C);
    let cg = CallGraph::build(&files);
    // `run2` has no local pointer: the field assignment elsewhere supplies the target.
    assert_eq!(indirect(&cg, "run2"), ["other"]);
    // `run` already resolved `cb` on that line through its local pointer, so the
    // struct-field target is not added next to it.
    assert_eq!(indirect(&cg, "run"), ["target"]);
}

#[test]
fn level4_skips_qualified_sites_whose_callee_is_not_a_plain_field_name() {
    let files = parse(&[("a.c", FIELD_FIXTURE)], Language::C);
    let mut cg = CallGraph::build(&files);
    let caller = cg
        .calls
        .keys()
        .find(|id| id.name == "run2")
        .unwrap()
        .clone();
    let mut sites = cg.calls.remove(&caller).unwrap();
    let mut site = sites.iter().next().unwrap().clone();
    sites.clear();
    site.callee_name = "s.ops[0]".to_string();
    site.qualifier = Some("s".to_string());
    sites.insert(site);
    cg.calls.insert(caller, sites);
    // Level 2 finds no initializer for `s.ops`; Level 4 must not treat the subscripted
    // callee as a field name either.
    let (sites, _) = cg.compute_indirect_call_sites(&files, false);
    assert!(sites.iter().all(|(caller, _)| caller.name != "run2"));
}

#[test]
fn level3_drops_a_non_go_inbound_site_without_an_edge() {
    let mut files = parse(
        &[(
            "a.go",
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\n",
        )],
        Language::Go,
    );
    files.extend(parse(
        &[(
            "b.py",
            "def safe():\n    pass\n\ndef caller():\n    invoke(safe)\n",
        )],
        Language::Python,
    ));
    let cg = CallGraph::build(&files);
    assert_eq!(cg.level3_indirect_resolved, 0);
    let t = &cg.go_level3_b1_telemetry;
    assert_eq!((t.candidates, t.accepted_inbound_sites, t.edges), (1, 0, 0));
    assert_eq!(t.drops.get("non_go_or_test"), Some(&1));
    assert_eq!(t.drops.len(), 1);
    assert!(indirect(&cg, "invoke").is_empty());
}

#[test]
fn level3_drops_a_call_with_no_argument_in_the_callback_slot() {
    let files = parse(
        &[(
            "a.go",
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke() }\n",
        )],
        Language::Go,
    );
    let cg = CallGraph::build(&files);
    let t = &cg.go_level3_b1_telemetry;
    assert_eq!((t.candidates, t.accepted_inbound_sites, t.edges), (1, 0, 0));
    assert_eq!(t.drops.get("missing_slot_or_argument_identity"), Some(&1));
    assert_eq!(t.drops.len(), 1);
    assert_eq!(cg.level3_indirect_resolved, 0);
}
