//! #829: macros that change bracket structure (commit A) and comma-carrying macros and
//! brace-split macro arguments (commit B). Per-row tests for the inline cases
//! `20_macro_brackets` / `21_macro_args` (rows b*, a*, x*, y*), the owner-gate verdicts (E01),
//! and the multi-file rows mf1 to mf4. Written against stubs: every keep row is red until the
//! census facts and the owner gates exist. The pair rows read the same `obligations.jsonl` as
//! the matrices, one test per row so the mutation lane can cite them.
use super::c_prune::Off;
use super::c_prune_inline_cases::{inline_source as c_source, INLINE_CASES as C_CASES};
use super::cpp_prune_inline_cases::{inline_source as cpp_source, INLINE_CASES as CPP_CASES};
use super::Producer;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tree_sitter::Node;

fn lang_of(f: &str) -> Language {
    if f.ends_with(".cc") || f.ends_with(".cpp") || f.ends_with(".hpp") {
        Language::Cpp
    } else {
        Language::C
    }
}

type Owners = BTreeSet<(String, usize)>;
type Key = (String, usize, usize);

fn owner_pairs(refs: &super::FileRefs) -> BTreeMap<Key, Owners> {
    let mut out: BTreeMap<Key, Owners> = BTreeMap::new();
    for e in &refs.edges {
        out.entry((e.from.path.to_string(), e.from.line, e.to.line))
            .or_default()
            .insert((e.from.function.clone(), e.from.function_start_line));
    }
    out
}

pub(super) fn c_case(name: &str) -> String {
    let (_, s) = C_CASES.iter().find(|(n, _)| *n == name).expect(name);
    c_source(s)
}

pub(super) fn cpp_case(name: &str) -> String {
    let (_, s) = CPP_CASES.iter().find(|(n, _)| *n == name).expect(name);
    cpp_source(s)
}

fn obligation_rows(cpp: bool) -> Vec<serde_json::Value> {
    let text = if cpp {
        include_str!("../../../tests/fixtures/dfg_prune/cpp/obligations.jsonl")
    } else {
        include_str!("../../../tests/fixtures/dfg_prune/c/obligations.jsonl")
    };
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// One obligation row of an inline case, judged like the matrices judge it: the pair must
/// exist before the rules (else the row is inadmissible) and, for a keep row, every owner that
/// had it must still have it; for a drop row none may.
pub(super) fn check_row(file: &str, id: &str) {
    let cpp = lang_of(file) == Language::Cpp;
    let source = if cpp { cpp_case(file) } else { c_case(file) };
    let case = if cpp {
        file.to_owned()
    } else {
        file.strip_suffix(".c").unwrap().to_owned()
    };
    let parsed = ParsedFile::parse(file, &source, lang_of(file)).unwrap();
    let name = file.to_owned();
    let before = Producer::file_refs_frozen(&name, &parsed);
    let after = super::alias_occurrence::without_policy(|| Producer::file_refs(&name, &parsed));
    let (bo, ao) = (owner_pairs(&before), owner_pairs(&after));
    let rows: Vec<_> = obligation_rows(cpp)
        .into_iter()
        .filter(|r| r["case"] == case.as_str() && r["id"] == id)
        .collect();
    assert!(!rows.is_empty(), "no obligation row {id} in {case}");
    for row in rows {
        let key = (
            row["path"].as_str().unwrap().to_owned(),
            row["def_line"].as_u64().unwrap() as usize,
            row["use_line"].as_u64().unwrap() as usize,
        );
        let only = |set: Option<&Owners>| -> Owners {
            set.into_iter()
                .flatten()
                .filter(|(f, _)| row.get("owner").is_none_or(|o| o == f.as_str()))
                .cloned()
                .collect()
        };
        let (had, has) = (only(bo.get(&key)), only(ao.get(&key)));
        assert!(!had.is_empty(), "inadmissible: no baseline pair for {row}");
        if row["keep"].as_bool().unwrap() {
            assert!(
                had.is_subset(&has),
                "{row}: dropped (before={had:?} after={has:?})"
            );
        } else {
            assert!(has.is_empty(), "{row}: kept (before={had:?} after={has:?})");
        }
    }
}

macro_rules! rows {
    ($file:expr => $($name:ident $id:expr),+ $(,)?) => {
        $(#[test] fn $name() { check_row($file, $id); })+
    };
}

// M2, commit A. b10, b11, x5 are controls (K): pruning must survive.
rows!("20_macro_brackets.c" =>
    row_b1 "b1", row_b2 "b2", row_b3 "b3", row_b4 "b4", row_b5 "b5", row_b6 "b6",
    row_b7 "b7", row_b8 "b8", row_b9 "b9", row_b10 "b10", row_b11 "b11", row_b12 "b12",
    row_b13 "b13");
rows!("20_macro_brackets.cc" =>
    row_x1 "x1", row_x2 "x2", row_x3 "x3", row_x4 "x4", row_x5 "x5", row_x6 "x6");
// M5, commit B. a7, a8, y3, y4 are controls (K). a1 is gate-only (E01), see TESTS.md.
rows!("21_macro_args.c" =>
    row_a2 "a2", row_a3 "a3", row_a4 "a4", row_a5 "a5", row_a6 "a6", row_a7 "a7", row_a8 "a8");
rows!("21_macro_args.cc" => row_y1 "y1", row_y2 "y2", row_y3 "y3", row_y4 "y4");

// ---- E01: owner verdicts through `c_prune::scan` ----

pub(super) fn owner_nodes<'t>(parsed: &'t ParsedFile) -> Vec<(String, Node<'t>)> {
    fn walk<'t>(n: Node<'t>, parsed: &ParsedFile, out: &mut Vec<(String, Node<'t>)>) {
        if n.kind() == "function_definition" {
            if let Some(d) = n.child_by_field_name("declarator") {
                let text = parsed.node_text(&d);
                if let Some(open) = text.find('(') {
                    let name: String = text[..open]
                        .chars()
                        .rev()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    let owner = match n.parent() {
                        Some(p) if p.kind() == "template_declaration" => p,
                        _ => n,
                    };
                    out.push((name, owner));
                }
            }
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, parsed, out);
        }
    }
    let mut out = Vec::new();
    walk(parsed.tree.root_node(), parsed, &mut out);
    out
}

/// The gate `scan` names for each owner of `source` (`None` is `Ok`).
pub(super) fn verdicts(file: &str, source: &str) -> BTreeMap<String, Option<Off>> {
    let parsed = ParsedFile::parse(file, source, lang_of(file)).unwrap();
    let census = CMacroCensus::of_file(&parsed);
    owner_nodes(&parsed)
        .into_iter()
        .map(|(name, node)| {
            let v = super::c_prune::scan(&parsed, node, &census)
                .map(|_| ())
                .err();
            (name, v)
        })
        .collect()
}

pub(super) fn expect(v: &BTreeMap<String, Option<Off>>, gate: Option<Off>, names: &[&str]) {
    for n in names {
        assert_eq!(v.get(*n), Some(&gate), "owner {n}: {v:?}");
    }
}

#[test]
fn e01_20_macro_brackets_c() {
    let v = verdicts("20_macro_brackets.c", &c_case("20_macro_brackets.c"));
    // The scope-shift shapes of b1, b2, b5, b12, b13 (gate-only) and the b1..b9 owners.
    expect(
        &v,
        Some(Off::MacroBrackets),
        &[
            "brace_scope",
            "brace_transitive",
            "brace_control",
            "brace_function_like",
            "brace_digraph",
            "paren_macro",
            "bracket_macro",
            "balanced_wrapper_of_unbalanced",
            "one_definition_unbalanced",
            "external_pthread_pair",
            "external_cpython_pair",
            "brace_if_guard",
            "brace_transitive_if_guard",
            "brace_digraph_if_guard",
            "external_pthread_if_guard",
            "external_cpython_if_guard",
        ],
    );
    expect(
        &v,
        None,
        &["balanced_macros_still_prune", "not_spelled_still_prunes"],
    );
}

#[test]
fn e01_20_macro_brackets_cc() {
    let v = verdicts("20_macro_brackets.cc", &cpp_case("20_macro_brackets.cc"));
    expect(
        &v,
        Some(Off::MacroBrackets),
        &[
            "cpp_brace_scope",
            "cpp_paren_scope",
            "cpp_brace_control",
            "cpp_template_owner",
            "cpp_external_pthread_pair",
        ],
    );
    expect(&v, None, &["cpp_balanced_still_prunes"]);
}

#[test]
fn e01_21_macro_args_c() {
    let v = verdicts("21_macro_args.c", &c_case("21_macro_args.c"));
    expect(
        &v,
        Some(Off::MacroArgSplit),
        &[
            "arg_split_scope",
            "arg_split_chain",
            "arg_split_compound_literal",
        ],
    );
    expect(
        &v,
        Some(Off::MacroComma),
        &[
            "loose_top_level_comma",
            "loose_brace_held_comma",
            "loose_variadic_top",
        ],
    );
    expect(
        &v,
        None,
        &["tight_commas_still_prune", "non_macro_call_still_prunes"],
    );
}

#[test]
fn e01_21_macro_args_cc() {
    let v = verdicts("21_macro_args.cc", &cpp_case("21_macro_args.cc"));
    expect(
        &v,
        Some(Off::MacroArgSplit),
        &["cpp_arg_split_scope", "cpp_init_list_in_macro"],
    );
    expect(
        &v,
        None,
        &[
            "cpp_tight_commas_still_prune",
            "cpp_external_macro_still_prunes",
        ],
    );
}

/// OD-3: the 16 names of DESIGN 6.3, exactly.
#[test]
fn block_macros_list_is_the_design_list() {
    let mut want = vec![
        "pthread_cleanup_push",
        "pthread_cleanup_pop",
        "pthread_cleanup_push_defer_np",
        "pthread_cleanup_pop_restore_np",
        "Py_BEGIN_ALLOW_THREADS",
        "Py_END_ALLOW_THREADS",
        "Py_BEGIN_CRITICAL_SECTION",
        "Py_END_CRITICAL_SECTION",
        "Py_BEGIN_CRITICAL_SECTION2",
        "Py_END_CRITICAL_SECTION2",
        "Py_BEGIN_CRITICAL_SECTION_MUTEX",
        "Py_BEGIN_CRITICAL_SECTION2_MUTEX",
        "Py_TRASHCAN_BEGIN",
        "Py_TRASHCAN_END",
        "Py_TRASHCAN_SAFE_BEGIN",
        "Py_TRASHCAN_SAFE_END",
    ];
    let mut have: Vec<&str> = super::c_prune::BLOCK_MACROS.to_vec();
    want.sort_unstable();
    have.sort_unstable();
    assert_eq!(have, want);
}

// ---- M3: multi-file rows ----

/// `(edges before the rules, edges after)` over `target`, with the census of all `files` and
/// optional census texts stamped on the target (the loader's text-only include files).
pub(super) fn counts(
    files: &[(&str, &str)],
    target: &str,
    texts: &[(&str, &str)],
) -> (usize, usize) {
    let mut parsed: BTreeMap<String, ParsedFile> = files
        .iter()
        .map(|(n, s)| (n.to_string(), ParsedFile::parse(n, s, lang_of(n)).unwrap()))
        .collect();
    if !texts.is_empty() {
        let t: BTreeMap<String, String> = texts
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        parsed
            .get_mut(target)
            .unwrap()
            .set_c_census_texts(Arc::new(t));
    }
    let c_census = CMacroCensus::from_files(&parsed);
    let file = target.to_owned();
    let p = &parsed[target];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &c_census);
    (before.edges.len(), after.edges.len())
}

const HDR: &str = "#define END }\n#define BEGIN {\n";
/// The b1 guard owner: BEGIN/END come from elsewhere, so the file holds no `#define`.
const GUARD_OWNER: &str = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void) {\n    int x = src();\n    if (cond) BEGIN;\n    x = 2;\n    END;\n    sink(x);\n}\n";

#[test]
fn mf1_header_defines_reach_the_owner() {
    let (b, a) = counts(&[("a.c", GUARD_OWNER), ("h.h", HDR)], "a.c", &[]);
    assert_eq!(
        a, b,
        "the header's brackets turn the owner off ({b} -> {a})"
    );
}

#[test]
fn mf1_control_without_defines_prunes() {
    // Nothing defines BEGIN/END, so the dominating write prunes.
    let (b, a) = counts(&[("a.c", GUARD_OWNER)], "a.c", &[]);
    assert!(a < b, "control prunes ({b} -> {a})");
}

#[test]
fn mf2_defines_arrive_as_a_census_text() {
    let (b, a) = counts(&[("a.c", GUARD_OWNER)], "a.c", &[("defs.inc", HDR)]);
    assert_eq!(a, b, "a census text's brackets count ({b} -> {a})");
}

#[test]
fn mf2_control_balanced_census_text_prunes() {
    let (b, a) = counts(
        &[("a.c", GUARD_OWNER)],
        "a.c",
        &[("defs.inc", "#define OK 1\n")],
    );
    assert!(a < b, "control: a balanced text prunes ({b} -> {a})");
}

const DUAL_OWNER: &str = "int src(void);\nvoid sink(int);\nvoid f(void) {\n    int x = src();\n    sink(DUAL);\n    x = 2;\n    sink(x);\n}\n";

#[test]
fn mf3_one_of_two_definitions_in_two_files() {
    let two = [
        ("a.c", DUAL_OWNER),
        ("x.c", "#define DUAL 0\n"),
        ("y.c", "#define DUAL {\n"),
    ];
    let (b, a) = counts(&two, "a.c", &[]);
    assert_eq!(
        a, b,
        "any unbalanced definition taints the spelling ({b} -> {a})"
    );
}

#[test]
fn mf3_control_only_the_balanced_definition_prunes() {
    let (b, a) = counts(
        &[("a.c", DUAL_OWNER), ("x.c", "#define DUAL 0\n")],
        "a.c",
        &[],
    );
    assert!(a < b, "control prunes ({b} -> {a})");
}

const B1S_H: &str = "#define END }\n#define BEGIN {\n";
const B1S_CC: &str = "#include \"h.h\"\nstruct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };\nstruct T { int k; };\nint b(S x) {\n    {\n        int* x = nullptr;\n        END;\n        x = 5;\n        x = 1;\n        BEGIN;\n    }\n    return x.v;\n}\n";

#[test]
fn mf4_the_issue_files() {
    let parsed: BTreeMap<String, ParsedFile> = [("b1s.cc", B1S_CC), ("h.h", B1S_H)]
        .iter()
        .map(|(n, s)| (n.to_string(), ParsedFile::parse(n, s, lang_of(n)).unwrap()))
        .collect();
    let c_census = CMacroCensus::from_files(&parsed);
    let file = "b1s.cc".to_owned();
    let p = &parsed[&file];
    let before = Producer::file_refs_frozen(&file, p);
    let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &c_census);
    let key = |r: &super::FileRefs| -> bool {
        r.edges
            .iter()
            .any(|e| e.from.path.to_string() == "x" && e.from.line == 8 && e.to.line == 9)
    };
    assert!(key(&before), "admissible: the baseline has x@8 -> x@9");
    assert!(
        key(&after),
        "x@8 -> x@9 is live (prints 451) and must be present"
    );
}
