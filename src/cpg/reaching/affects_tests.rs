//! #674: recorded defs of a JS/TS/TSX owner kill by `affects`. The per-base trie is checked against
//! `affects` itself and against the frozen all-pairs scanner (with the writer flags of
//! `affects_writers.rs`), its work is bounded, and the trace names the kills it adds.
use super::def_bucket_tests::site;
use super::*;
use crate::access_path::ARRAY_FIELD;

fn path(base: &str, fields: &[&str]) -> AccessPath {
    AccessPath::with_fields(base, fields.iter().map(|f| f.to_string()).collect())
}

/// Every path over a small alphabet (with `[]`), a few bases, up to depth 3.
fn universe() -> Vec<AccessPath> {
    let mut all = Vec::new();
    for base in ["o", "p"] {
        let mut level = vec![Vec::<&str>::new()];
        for _ in 0..=3 {
            for fields in &level {
                all.push(path(base, fields));
            }
            level = level
                .iter()
                .flat_map(|f| {
                    ["a", "b", ARRAY_FIELD].map(|x| {
                        let mut g = f.clone();
                        g.push(x);
                        g
                    })
                })
                .collect();
        }
    }
    all
}

/// The trie answers exactly `affects`: for every writer path and every def path of the universe (`[]`
/// on either side, prefixes, siblings, other bases), the defs it yields are the defs the relation names.
#[test]
fn the_trie_yields_exactly_the_affected_defs() {
    let all = universe();
    let mut tries = BTreeMap::<&str, def_buckets::Trie<'_>>::new();
    for (index, p) in all.iter().enumerate() {
        tries
            .entry(p.base.as_str())
            .or_default()
            .insert(&p.fields, index);
    }
    for w in &all {
        let mut got = Vec::new();
        tries[w.base.as_str()].affected(&w.fields, &mut |d| got.push(d));
        got.sort();
        let want: Vec<usize> = (0..all.len())
            .filter(|&i| def_buckets::affects(w, &all[i]))
            .collect();
        assert_eq!(got, want, "writer {w:?}");
    }
}

const SOURCE: &str = "function f(o, k) {\no.a = 1;\no.a.b = 2;\no[k] = 3;\no.a[k] = 4;\no.b = 5;\no.a.b.c = 6;\no[k].x = 7;\no = {};\nconsume(o.a);\n}\n";

fn owner_defs(parsed: &ParsedFile, owner: &Node<'_>) -> Vec<DefSite> {
    parsed
        .assignment_lvalue_spans_on_lines(owner, &(1..=10).collect())
        .into_iter()
        .enumerate()
        .map(|(i, s)| site(i as u32, s.path, s.line, s.start_byte, false))
        .collect()
}

/// Bucketed `affects` kills equal the frozen all-pairs scan, every def tried as alias-derived, with each
/// def at its own node and all at one node, on the real recorded defs of a member-write owner.
#[test]
fn bucketed_recorded_affects_matches_the_all_pairs_scan() {
    let parsed = ParsedFile::parse("a.ts", SOURCE, Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    let originals = owner_defs(&parsed, &owner);
    assert!(originals.len() >= 7, "{originals:?}");
    let mut wider_than_equal = false;
    for alias in 0..=originals.len() {
        let mut defs = originals.clone();
        if alias < defs.len() {
            defs[alias].alias_derived = true;
        }
        let facts = crate::cpg::binding_facts::BindingFacts::new(
            &parsed,
            &owner,
            &defs,
            std::iter::empty(),
        );
        for together in [false, true] {
            let mut gen = vec![BitSet::new(defs.len()); 13];
            for (index, def) in defs.iter().enumerate() {
                gen[if together { 1 } else { def.line }].insert(index);
            }
            let flagged = affects_writers::affects_writers(&parsed, &defs, None);
            assert!(
                flagged.iter().any(|f| !f),
                "the truncated `o[k].x` writer is unflagged"
            );
            for writers in [vec![true; defs.len()], flagged] {
                let buckets = def_buckets::DefinitionBuckets::with_kill_only(
                    &defs,
                    defs.len(),
                    Some(&writers),
                );
                let actual = buckets.kill_sets(&defs, &gen, &facts);
                let expected = def_buckets::legacy_kill_sets_with(
                    &defs,
                    &gen,
                    &facts,
                    defs.len(),
                    Some(&writers),
                );
                assert_eq!(actual, expected, "alias {alias}, together {together}");
            }
            let every = vec![true; defs.len()];
            let buckets =
                def_buckets::DefinitionBuckets::with_kill_only(&defs, defs.len(), Some(&every));
            let actual = buckets.kill_sets(&defs, &gen, &facts);
            let equal_only = def_buckets::legacy_kill_sets(&defs, &gen, &facts);
            wider_than_equal |= actual.1 != equal_only.1;
            assert!(
                equal_only
                    .1
                    .iter()
                    .zip(&actual.1)
                    .all(|(eq, aff)| eq.members().all(|m| aff.contains(m))),
                "equal-path kills are a subset"
            );
        }
    }
    assert!(wider_than_equal, "the owner must exercise a non-equal kill");
}

fn pair_work(source: &str) -> (usize, usize) {
    let parsed = ParsedFile::parse("cost.ts", source, Language::TypeScript).unwrap();
    let (_, records) = work_tests::build(parsed);
    let c = &records[0];
    (
        c.definition_pair_comparisons,
        c.projected_definition_pair_comparisons,
    )
}

/// Sibling field writes of one base do not scan each other: a field writer visits only its own def, and the
/// parameter def `o` and the closing `o = {}` visit the `n` fields they affect, so the work is linear in `n`.
#[test]
fn sibling_fields_do_no_cross_field_scan() {
    let mut works = Vec::new();
    for n in [8, 32, 128] {
        let body: String = (0..n)
            .map(|i| format!("o.f{i} = {i};\nconsume(o.f{i});\n"))
            .collect();
        let (work, projected) = pair_work(&format!("function f(o) {{\n{body}return 0;\n}}"));
        assert_eq!(
            work,
            2 * n + 1,
            "n={n}: n own visits, the parameter `o` visits itself and the n fields"
        );
        assert_eq!(work, projected);
        let (work, projected) =
            pair_work(&format!("function f(o) {{\n{body}o = {{}};\nreturn 0;\n}}"));
        assert_eq!(work, 3 * n + 4, "n={n}: the parameter `o` and the closing `o = {{}}` each visit both `o` defs and the n fields");
        assert_eq!(work, projected, "every visit is an affected def");
        works.push(work);
    }
    assert_eq!(works, [28, 100, 388]);
}

/// `[]` on either side is followed without scanning the base: `o.b<i>` visits itself and the `o[]` def it
/// may overwrite, never `o.a[]` or the other siblings; `o.a[k]` visits itself; `o[k]` visits the whole run.
#[test]
fn wildcard_writers_and_defs_visit_only_affected_defs() {
    let n = 64;
    let siblings: String = (0..n).map(|i| format!("o.b{i} = {i};\n")).collect();
    let (work, projected) = pair_work(&format!(
        "function f(o, k) {{\n{siblings}o.a[k] = 1;\no[k] = 2;\nreturn 0;\n}}"
    ));
    // `o` (n + 3: itself, `o[]`, `o.a[]` and the n siblings), `k` (1), the n sibling writers (2 each), `o.a[k]` (1:
    // `o[]` is shorter than it), `o[k]` (n + 2).
    assert_eq!(work, (n + 3) + 1 + 2 * n + 1 + (n + 2), "{work}");
    assert_eq!(work, projected, "every visit is an affected def");
}

fn mechanisms(language: Language, src: &str) -> Vec<String> {
    super::census_defs_tests::trace_lines(language, src)
        .iter()
        .map(|l| l.split('\t').nth(1).unwrap().to_string())
        .filter(|m| !m.starts_with("d3-"))
        .collect()
}

#[test]
fn the_trace_names_a_recorded_affects_kill() {
    let js = Language::JavaScript;
    // The overwritten value is also read bare, so the producer records `o = {}` as an edge source.
    let prefix = "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o.x, o);\n}\n";
    assert_eq!(mechanisms(js, prefix), ["kill:affects"]);
    let subscript = "function f(o, k) {\n  o.x = 1;\n  o[k] = 2;\n  sink(o.x, o[k].x);\n}\n";
    assert_eq!(mechanisms(js, subscript), ["kill:affects"]);
    // A truncated writer (`o[k].x` is recorded as `o.[]`) and a nested-callable write keep path equality.
    let truncated = "function f(o, k) {\n  o.n = 1;\n  o[k].x = 2;\n  sink(o.n, o[k].x);\n}\n";
    assert!(!mechanisms(js, truncated).contains(&"kill:affects".to_string()));
}

#[test]
fn the_trace_stays_silent_without_a_recorded_affects_move() {
    let js = Language::JavaScript;
    // Equal path: main's kill, not a move.
    let equal = "function f(o) {\n  o.x = 1;\n  o.x = 2;\n  sink(o.x, o);\n}\n";
    assert_eq!(mechanisms(js, equal), Vec::<String>::new());
    // A census kill-only write keeps its own mechanism (no recorded affects kill involved).
    let update = "function f(o) {\n  o.x = 1;\n  o++;\n  sink(o.x);\n}\n";
    let m = mechanisms(js, update);
    assert!(
        !m.is_empty() && m.iter().all(|m| m == "kill:update"),
        "{m:?}"
    );
    // Other languages are main's equal-path kills.
    let py = "def f(o):\n    o.x = 1\n    o = {}\n    sink(o.x, o)\n";
    assert_eq!(mechanisms(Language::Python, py), Vec::<String>::new());
}
