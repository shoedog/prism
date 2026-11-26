//! I1 limits and complete predictions. The expectations come from the shape of each program and
//! from DESIGN section 5 (the Q09-Q11 matrix rows), never from the preview: every program runs
//! through the real producer, and limits that the matrix scales down are injected per file.
use super::limits::*;
use super::preview::Occupied;
use super::trace::{CAPTURE, LIMITS, PREFIX};
use super::*;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::{DataFlowGraph, VarAccessKind, VarLocation};
use crate::languages::Language;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/subscript_matrix/cases/"
);

fn case(name: &str) -> String {
    std::fs::read_to_string(format!("{DIR}{name}.py")).unwrap()
}

/// Every trace record of one program through the real producer, with `limits` injected for it.
fn run(file: &str, src: &str, limits: Option<Limits>) -> Vec<Value> {
    CAPTURE
        .lock()
        .unwrap()
        .get_or_insert_with(BTreeMap::new)
        .insert(file.into(), vec![]);
    limits.into_iter().for_each(|l| {
        LIMITS.lock().unwrap().insert(file.into(), l);
    });
    let parsed = ParsedFile::parse(file, src, Language::Python).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{file}");
    DataFlowGraph::build(&BTreeMap::from([(file.to_string(), parsed)]));
    LIMITS.lock().unwrap().remove(file);
    let lines = CAPTURE
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .remove(file)
        .unwrap();
    let json = |l: String| serde_json::from_str(l.strip_prefix(PREFIX).expect(&l)).unwrap();
    lines.into_iter().map(json).collect()
}

fn owner<'a>(rs: &'a [Value], name: &str) -> &'a Value {
    rs.iter()
        .find(|r| r["kind"] == "owner" && r["owner"] == name)
        .unwrap_or_else(|| panic!("{name}"))
}

/// `outcome -> count` over the predictions of one owner.
fn outcomes(rs: &[Value], name: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for r in rs
        .iter()
        .filter(|r| r["kind"] == "prediction" && r["owner"] == name)
    {
        *out.entry(r["outcome"].as_str().unwrap().to_string())
            .or_default() += 1;
    }
    out
}

fn pick(o: &Value, keys: &[&str]) -> Value {
    Value::Array(keys.iter().map(|k| o[*k].clone()).collect())
}

fn counts(pairs: &[(&str, usize)]) -> BTreeMap<String, usize> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn limit(raw_pairs: u64) -> Limits {
    Limits {
        raw_pairs,
        ..Limits::PRODUCTION
    }
}

fn loc(path: &AccessPath, kind: VarAccessKind, line: usize) -> VarLocation {
    VarLocation {
        file: "f.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: path.clone(),
        start_byte: 0,
        end_byte: 1,
        kind,
    }
}

#[test]
fn element_preview_counts_duplicate_candidates_before_allocation() {
    // Two stores share a line and so a key (a collision the join refuses): P still counts both,
    // 2 stores x 1 read. At the limit the owner passes and the join refuses each pair; one below
    // it the owner is refused before any join or route runs.
    let dup = case("Q04.duplicate-write");
    let at = run("dup_w_2.py", &dup, Some(limit(2)));
    assert_eq!(owner(&at, "case")["raw_pairs"], 2);
    assert_eq!(
        outcomes(&at, "case"),
        counts(&[("refused:join:collapsed", 2)])
    );
    let below = run("dup_w_1.py", &dup, Some(limit(1)));
    assert_eq!(
        pick(owner(&below, "case"), &["raw_pairs", "raw_capped"]),
        json!([2, true])
    );
    assert_eq!(outcomes(&below, "case"), counts(&[("refused:rawcap", 2)]));
    // one store, two competing reads of one key: 1 x 2
    let reads = case("Q04.duplicate-read");
    let at = run("dup_r_2.py", &reads, Some(limit(2)));
    assert_eq!(owner(&at, "case")["raw_pairs"], 2);
    assert_eq!(
        outcomes(&at, "case"),
        counts(&[("refused:join:collapsed", 2)])
    );
    assert_eq!(
        outcomes(&run("dup_r_1.py", &reads, Some(limit(1))), "case"),
        counts(&[("refused:rawcap", 2)])
    );
    // the matrix's scaled raw pairs: 3 stores x 4 reads = 12 at the injected limit 12, plus one
    // more store and read (13) over it; the neighbour owner keeps its own pair either way
    let at = run("raw_12.py", &case("Q10.raw-1000000"), Some(limit(12)));
    assert_eq!(
        pick(owner(&at, "case"), &["raw_pairs", "raw_capped"]),
        json!([12, false])
    );
    assert_eq!(outcomes(&at, "case"), counts(&[("predicted", 12)]));
    let over = run("raw_13.py", &case("Q10.raw-1000001"), Some(limit(12)));
    assert_eq!(
        pick(owner(&over, "case"), &["raw_pairs", "raw_capped"]),
        json!([13, true])
    );
    assert_eq!(outcomes(&over, "case"), counts(&[("refused:rawcap", 13)]));
    assert_eq!(outcomes(&over, "independent"), counts(&[("predicted", 1)]));
    // production: the same 13 pairs are far below one million
    let prod = run("raw_prod.py", &case("Q10.raw-1000001"), None);
    assert_eq!(outcomes(&prod, "case"), counts(&[("predicted", 13)]));
    // raw rows are counted by multiplicity: two identical rows at one span are two units of work
    let (d, w) = (
        AccessPath::simple("d"),
        Span {
            start: 5,
            end: 9,
            line: 2,
        },
    );
    let mut buckets = super::join::Buckets::new();
    let row = |start, end, line| VarLocation {
        start_byte: start,
        end_byte: end,
        line,
        ..loc(&d, VarAccessKind::Def, 2)
    };
    let key = ("f.py".to_string(), "f".to_string(), 1, d.clone());
    buckets.insert(
        key,
        vec![row(5, 9, 2), row(5, 9, 2), row(5, 8, 2), row(5, 9, 3)],
    );
    let n = raw_rows(&buckets, ("f.py", "f", 1), &d, VarAccessKind::Def, w);
    assert_eq!(
        n, 2,
        "duplicates count; other spans, lines and kinds do not"
    );
    assert_eq!(
        raw_rows(&buckets, ("f.py", "f", 1), &d, VarAccessKind::Use, w),
        0
    );
    assert_eq!(
        Work::count([(n, 3)]),
        Work {
            total: 6,
            overflow: false
        }
    );
}

#[test]
fn element_preview_omits_entire_read_or_owner_at_cap() {
    // Q09: eight distinct stores feed `sink(d)`: all eight pairs pass; nine omit the whole read,
    // not the ninth. The unrelated `e -> copy` pair survives in both.
    let eight = run("fanin_8.py", &case("Q09.fanin-8"), None);
    assert_eq!(outcomes(&eight, "case"), counts(&[("predicted", 9)]));
    assert_eq!(owner(&eight, "case")["reads_over_fanin"], 0);
    let nine = run("fanin_9.py", &case("Q09.fanin-9"), None);
    assert_eq!(
        outcomes(&nine, "case"),
        counts(&[("predicted", 1), ("refused:fanin", 9)])
    );
    let o = owner(&nine, "case");
    assert_eq!(
        pick(
            o,
            &["reads_over_fanin", "omitted_fanin_pairs", "over_edge_cap"]
        ),
        json!([1, 9, false])
    );
    assert_eq!(o["attrition"]["distinct"], 10);
    assert_eq!(
        pick(&o["attrition"], &["after_fanin", "published"]),
        json!([1, 1])
    );
    // Q10: 8 stores x 32 reads = 256 pairs pass; the 257th pair omits the whole remaining owner,
    // while the neighbour owner is unaffected
    let at = run("owner_256.py", &case("Q10.owner-256"), None);
    assert_eq!(outcomes(&at, "case"), counts(&[("predicted", 256)]));
    assert_eq!(outcomes(&at, "independent"), counts(&[("predicted", 1)]));
    let over = run("owner_257.py", &case("Q10.owner-257"), None);
    assert_eq!(
        outcomes(&over, "case"),
        counts(&[("refused:ownercap", 257)])
    );
    assert_eq!(outcomes(&over, "independent"), counts(&[("predicted", 1)]));
    let o = owner(&over, "case");
    assert_eq!(
        pick(
            o,
            &["over_edge_cap", "omitted_cap_pairs", "reads_over_fanin"]
        ),
        json!([true, 257, 0])
    );
    // the fan-in omission comes first: nine stores into `d` leave 9 + 256 pairs, and only the
    // 256 of `e` (8 stores x 32 reads) remain once `d`'s read is gone, so the owner passes
    let d: String = (0..9).map(|i| format!("    d[{i}] = t\n")).collect();
    let e: String = (0..8).map(|i| format!("    e[{i}] = t\n")).collect();
    let copies: String = (0..32).map(|i| format!("    copy{i} = e\n")).collect();
    let src = format!("def case(t):\n    d = {{}}\n    e = {{}}\n{d}{e}    sink(d)\n{copies}");
    let at = run("owner_order.py", &src, None);
    assert_eq!(
        outcomes(&at, "case"),
        counts(&[("predicted", 256), ("refused:fanin", 9)])
    );
}

#[test]
fn element_preview_checked_overflow_and_equality() {
    // production constants
    assert_eq!(
        Limits::PRODUCTION,
        Limits {
            raw_pairs: 1_000_000,
            read_sources: 8,
            owner_edges: 256
        }
    );
    assert_eq!((RAW_PAIRS, READ_SOURCES, OWNER_EDGES), (1_000_000, 8, 256));
    // equality passes the raw limit, one more refuses
    assert!(!Work::count([(1000, 1000)]).exceeds(RAW_PAIRS));
    assert!(Work::count([(1000, 1000), (1, 1)]).exceeds(RAW_PAIRS));
    assert_eq!(
        Work::count([]),
        Work {
            total: 0,
            overflow: false
        }
    );
    // checked arithmetic: a product or a sum that does not fit saturates and refuses, even
    // against a limit equal to the saturated total
    let big = Work::count([(u64::MAX, 2)]);
    assert_eq!(
        big,
        Work {
            total: u64::MAX,
            overflow: true
        }
    );
    assert!(big.exceeds(u64::MAX));
    let sum = Work::count([(u64::MAX, 1), (1, 1)]);
    assert_eq!(
        sum,
        Work {
            total: u64::MAX,
            overflow: true
        }
    );
    let after = Work::count([(u64::MAX / 2 + 1, 2), (1, 1)]);
    assert!(
        after.overflow && after.total == u64::MAX,
        "overflow is sticky"
    );
    let exact = Work::count([(u64::MAX, 1)]);
    assert_eq!(
        exact,
        Work {
            total: u64::MAX,
            overflow: false
        }
    );
    assert!(
        !exact.exceeds(u64::MAX),
        "equality with an exact total passes"
    );
    // fan-in and owner thresholds, equality passes
    let sp = |n: usize| Span {
        start: n,
        end: n + 1,
        line: 1,
    };
    let sources =
        |n: usize| -> std::collections::BTreeSet<_> { (0..n).map(|w| (sp(w), sp(1000))).collect() };
    let l = Limits::PRODUCTION;
    assert_eq!(cap(&l, &sources(8)).kept.len(), 8);
    let nine = cap(&l, &sources(9));
    assert!(nine.kept.is_empty() && nine.fanin_reads.len() == 1 && nine.fanin_pairs == 9);
    let reads = |n: usize| -> std::collections::BTreeSet<_> {
        (0..n).map(|r| (sp(0), sp(1000 + r))).collect()
    };
    let owner = cap(&l, &reads(256));
    assert_eq!((owner.kept.len(), owner.owner_over), (256, false));
    let over = cap(&l, &reads(257));
    assert!(over.kept.is_empty() && over.owner_over && over.owner_pairs == 257);
}

#[test]
fn element_preview_reports_each_unavailable_rd_exit() {
    let head = "def case(d, t):\n    d[\"a\"] = t\n";
    let avail = run("rd_ok.py", &case("Q01.argument"), None);
    assert_eq!(
        pick(owner(&avail, "case"), &["rd", "rd_early"]),
        json!(["available", false])
    );
    assert!(owner(&avail, "case")["raw_pairs"].is_u64());
    // too many defs
    let locals: String = (0..2049).map(|i| format!("    v{i} = 0\n")).collect();
    let defs = run("rd_def.py", &format!("{head}{locals}    sink(d)\n"), None);
    let o = owner(&defs, "case");
    assert_eq!(
        pick(o, &["rd", "rd_early", "raw_pairs"]),
        json!(["def_cap", false, null])
    );
    assert_eq!(
        outcomes(&defs, "case"),
        counts(&[("refused:rd:def_cap", 1)])
    );
    // too many statement lines
    let calls = "    noop()\n".repeat(4097);
    let lines = run("rd_line.py", &format!("{head}{calls}    sink(d)\n"), None);
    assert_eq!(owner(&lines, "case")["rd"], "line_cap");
    assert_eq!(
        outcomes(&lines, "case"),
        counts(&[("refused:rd:line_cap", 1)])
    );
    // one statement line: no CFG edge
    let flat = run("rd_cfg.py", "def case(d, t): d[\"a\"] = t; sink(d)\n", None);
    assert_eq!(
        pick(owner(&flat, "case"), &["rd", "rd_early"]),
        json!(["no_cfg", false])
    );
    assert_eq!(outcomes(&flat, "case"), counts(&[("refused:rd:no_cfg", 1)]));
    // def cap and more than one million raw fallback pairs (2100 defs of the field `o.f`, which
    // no dominating write drops, each paired with 500 reads of it): skipped before the RD call
    let head = "def case(d, t, o):\n    d[\"a\"] = t\n";
    let x: String = "    o.f = 0\n".repeat(2100);
    let uses: String = "    use(o.f)\n".repeat(500);
    let early = run(
        "rd_early.py",
        &format!("{head}{x}{uses}    sink(d)\n"),
        None,
    );
    let o = owner(&early, "case");
    assert_eq!(
        pick(o, &["rd", "rd_early"]),
        json!(["fallback_cap:def_cap", true])
    );
    assert_eq!(
        outcomes(&early, "case"),
        counts(&[("refused:rd:fallback_cap:def_cap", 1)])
    );
}

#[test]
fn element_preview_requires_distinct_mapped_statements() {
    // W and R on one statement line map to one RD statement: no ordering edge exists
    let same = run(
        "same_stmt.py",
        "def case(d, t):\n    x = 1\n    d[\"a\"] = t; sink(d)\n",
        None,
    );
    assert_eq!(outcomes(&same, "case"), counts(&[("refused:placement", 1)]));
    let apart = run(
        "apart.py",
        "def case(d, t):\n    x = 1\n    d[\"a\"] = t\n    sink(d)\n",
        None,
    );
    assert_eq!(outcomes(&apart, "case"), counts(&[("predicted", 1)]));
}

#[test]
fn element_preview_refuses_occupied_keys() {
    let (w, r) = (
        AccessPath::with_fields("d", vec!["[]".into()]),
        AccessPath::simple("d"),
    );
    let (wl, rl) = (
        loc(&w, VarAccessKind::Def, 2),
        loc(&r, VarAccessKind::Use, 3),
    );
    let moved = VarLocation {
        start_byte: 40,
        end_byte: 41,
        ..rl.clone()
    };
    let occupied = Occupied::new([(&wl, &rl)]);
    // line-and-path identity, not bytes: a use at other bytes of the same key is occupied too
    assert!(occupied.0.contains(&(wl.clone(), moved)));
    // other keys, and sources that are not indexed stores, are not
    assert!(!occupied
        .0
        .contains(&(wl.clone(), loc(&r, VarAccessKind::Use, 4))));
    let plain = loc(&r, VarAccessKind::Def, 2);
    assert!(Occupied::new([(&plain, &rl)]).0.is_empty());
}
