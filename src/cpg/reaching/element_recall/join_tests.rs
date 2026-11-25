//! Physical joins of the I0 inventory. The expectations are written from the shape of each
//! program (which occurrences share an endpoint key), not read back from the join: handcrafted
//! buckets pin every refusal, and the real producer pins the same-line collisions.
use super::join::*;
use super::trace::{CAPTURE, PREFIX};
use super::*;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::{DataFlowGraph, VarAccessKind, VarLocation};
use crate::languages::Language;
use serde_json::Value;
use std::collections::BTreeMap;

const SRC: &str = "def f(d, t):\n    d[\"a\"] = t\n    sink(d)\n";

fn span_of(src: &str, needle: &str, nth: usize) -> (usize, usize) {
    let (i, _) = src.match_indices(needle).nth(nth).expect(needle);
    (i, i + needle.len())
}

fn line_of(src: &str, at: usize) -> usize {
    src[..at].matches('\n').count() + 1
}

fn loc(kind: VarAccessKind, path: &AccessPath, line: usize, (s, e): (usize, usize)) -> VarLocation {
    VarLocation {
        file: "f.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: path.clone(),
        start_byte: s,
        end_byte: e,
        kind,
    }
}

fn put(buckets: &mut Buckets, l: VarLocation) {
    let key = ("f.py".to_string(), "f".to_string(), 1, l.path.clone());
    buckets.entry(key).or_default().push(l);
}

fn paths() -> (AccessPath, AccessPath) {
    (
        AccessPath::with_fields("d", vec!["[]".to_string()]),
        AccessPath::simple("d"),
    )
}

/// One row of the refusal table: rows each side holds, and the verdict each side must get.
struct Row {
    defs: Vec<(usize, usize)>,
    uses: Vec<(usize, usize)>,
    ordinary_defs: Vec<(usize, usize)>,
    ordinary_uses: Vec<(usize, usize)>,
    owners: usize,
}

fn run(row: &Row) -> OwnerJoin {
    let parsed = ParsedFile::parse("f.py", SRC, Language::Python).unwrap();
    let scan = &python::scan_file(&parsed)[0];
    let (dp, up) = paths();
    let (def, uses) = (&mut Buckets::new(), &mut Buckets::new());
    let (dl, ul) = (line_of(SRC, 20), 3);
    row.defs
        .iter()
        .for_each(|b| put(def, loc(VarAccessKind::Def, &dp, dl, *b)));
    row.uses
        .iter()
        .for_each(|b| put(uses, loc(VarAccessKind::Use, &up, ul, *b)));
    let originals = Originals {
        name: "f".into(),
        start_line: 1,
        defs: row
            .ordinary_defs
            .iter()
            .map(|b| (dp.clone(), dl, b.0, b.1))
            .collect(),
        uses: row
            .ordinary_uses
            .iter()
            .map(|b| (up.clone(), ul, b.0, b.1))
            .collect(),
    };
    join_owner("f.py", scan, row.owners, Some(&originals), def, uses)
}

fn good() -> Row {
    let (w, call) = (span_of(SRC, "d[\"a\"]", 0), span_of(SRC, "sink(d)", 0));
    let r = (call.0 + 5, call.0 + 6);
    assert_eq!(&SRC[r.0..r.1], "d");
    Row {
        defs: vec![w],
        uses: vec![r],
        ordinary_defs: vec![w],
        ordinary_uses: vec![r],
        owners: 1,
    }
}

fn verdicts(row: &Row) -> (Option<Endpoint>, Option<Endpoint>, Endpoint) {
    let j = run(row);
    assert_eq!(j.pairs.len(), 1, "one admitted pair");
    (j.writes[0], j.operands[0], j.pairs[0].1)
}

#[test]
fn element_join_rejects_missing_or_hidden_endpoint() {
    use JoinRefusal as J;
    let (w, r) = (good().defs[0], good().uses[0]);
    assert_eq!(verdicts(&good()), (Some(Ok(())), Some(Ok(())), Ok(())));
    // No row and no ordinary occurrence: the endpoint is missing, and so is the pair.
    let missing = Row {
        defs: vec![],
        ordinary_defs: vec![],
        ..good()
    };
    assert_eq!(
        verdicts(&missing),
        (Some(Err(J::Missing)), Some(Ok(())), Err(J::Missing))
    );
    let no_use = Row {
        uses: vec![],
        ordinary_uses: vec![],
        ..good()
    };
    assert_eq!(
        verdicts(&no_use),
        (Some(Ok(())), Some(Err(J::Missing)), Err(J::Missing))
    );
    // The owner pass produced it, the final bucket lost it.
    let hidden = Row {
        defs: vec![],
        ..good()
    };
    assert_eq!(verdicts(&hidden).0, Some(Err(J::Hidden)));
    let hidden_use = Row {
        uses: vec![],
        ..good()
    };
    assert_eq!(verdicts(&hidden_use).1, Some(Err(J::Hidden)));
    // A row strictly inside the syntax bytes is truncated; shifted or enlarged bytes disagree.
    let cut = Row {
        defs: vec![(w.0, w.0 + 1)],
        ordinary_defs: vec![(w.0, w.0 + 1)],
        ..good()
    };
    assert_eq!(verdicts(&cut).0, Some(Err(J::Truncated)));
    let moved = Row {
        uses: vec![(r.0 + 1, r.1 + 1)],
        ordinary_uses: vec![(r.0 + 1, r.1 + 1)],
        ..good()
    };
    assert_eq!(verdicts(&moved).1, Some(Err(J::Mismatched)));
    let wide = Row {
        defs: vec![(w.0, w.1 + 1)],
        ordinary_defs: vec![(w.0, w.1 + 1)],
        ..good()
    };
    assert_eq!(verdicts(&wide).0, Some(Err(J::Mismatched)));
    // A zero-width anchor alone is not the occurrence.
    let anchor = Row {
        defs: vec![(w.0, w.0)],
        ordinary_defs: vec![],
        ..good()
    };
    assert_eq!(verdicts(&anchor).0, Some(Err(J::Mismatched)));
    // The exact row with no ordinary location behind it is supplemental.
    let supplemental = Row {
        ordinary_defs: vec![],
        ..good()
    };
    assert_eq!(verdicts(&supplemental).0, Some(Err(J::Supplemental)));
    let supplemental = Row {
        ordinary_uses: vec![],
        ..good()
    };
    assert_eq!(verdicts(&supplemental).1, Some(Err(J::Supplemental)));
    // A competing anchor, row or ordinary location collapses the key.
    let line = (w.0 - 4, w.0 - 4);
    let anchored = Row {
        defs: vec![w, line],
        ordinary_defs: vec![w],
        ..good()
    };
    assert_eq!(verdicts(&anchored).0, Some(Err(J::Collapsed)));
    let twice = Row {
        uses: vec![r, r],
        ordinary_uses: vec![r, r],
        ..good()
    };
    assert_eq!(verdicts(&twice).1, Some(Err(J::Collapsed)));
    let unplaced = Row {
        ordinary_defs: vec![w, (w.0 + 2, w.0 + 3)],
        ..good()
    };
    assert_eq!(verdicts(&unplaced).0, Some(Err(J::Collapsed)));
    // An owner that is not the only one of its name and start line refuses everything.
    let twins = Row {
        owners: 2,
        ..good()
    };
    assert_eq!(verdicts(&twins).2, Err(J::AmbiguousOwner));
    // No owner pass at all: nothing is hidden or found.
    let parsed = ParsedFile::parse("f.py", SRC, Language::Python).unwrap();
    let scan = &python::scan_file(&parsed)[0];
    let none = join_owner("f.py", scan, 1, None, &Buckets::new(), &Buckets::new());
    assert_eq!(none.pairs[0].1, Err(J::Missing));
    // A syntax-refused candidate is never joined.
    let slice = "def f(d, t):\n    d[:1] = t\n    sink(d)\n";
    let parsed = ParsedFile::parse("f.py", slice, Language::Python).unwrap();
    let scan = &python::scan_file(&parsed)[0];
    let none = join_owner("f.py", scan, 1, None, &Buckets::new(), &Buckets::new());
    assert_eq!((none.writes[0], none.pairs.len()), (None, 0));
}

/// The records of one Python program through the real producer.
fn records(file: &str, src: &str) -> Vec<Value> {
    CAPTURE
        .lock()
        .unwrap()
        .get_or_insert_with(BTreeMap::new)
        .insert(file.into(), vec![]);
    let parsed = ParsedFile::parse(file, src, Language::Python).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{src}");
    DataFlowGraph::build(&BTreeMap::from([(file.to_string(), parsed)]));
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

/// `(kind, bytes, join)` of every write and node record, in emission order.
fn joins(records: &[Value]) -> Vec<(String, (usize, usize), String)> {
    let get = |r: &Value, k: &str| r[k].as_str().map(str::to_string).unwrap_or_default();
    let at = |r: &Value| {
        (
            r["bytes"][0].as_u64().unwrap() as usize,
            r["bytes"][1].as_u64().unwrap() as usize,
        )
    };
    records
        .iter()
        .filter(|r| r["kind"] != "owner")
        .map(|r| (get(r, "kind"), at(r), get(r, "join")))
        .collect()
}

fn pairs(records: &[Value]) -> Vec<String> {
    let owner = records.iter().find(|r| r["kind"] == "owner").unwrap();
    let all = owner["pairs"].as_array().unwrap();
    all.iter()
        .map(|p| p["join"].as_str().unwrap().to_string())
        .collect()
}

fn program(body: &str) -> String {
    format!("def case():\n    t = src_a()\n    u = src_b()\n    d = {{}}\n{body}\n")
}

fn kinds<'a>(
    src: &str,
    rows: &'a [(&'a str, &'a str, usize, &'a str)],
) -> Vec<(String, (usize, usize), String)> {
    rows.iter()
        .map(|(kind, needle, nth, join)| {
            (
                kind.to_string(),
                span_of(src, needle, *nth),
                join.to_string(),
            )
        })
        .collect()
}

#[test]
fn element_join_refuses_same_line_collision_in_both_orders() {
    // Two stores with the same endpoint key on one line: neither bytes are the one node.
    for (n, (first, second)) in [("\"a\"", "\"b\""), ("\"b\"", "\"a\"")]
        .into_iter()
        .enumerate()
    {
        let src = program(&format!("    d[{first}] = t; d[{second}] = u\n    sink(d)"));
        let r = records(&format!("collide_w{n}.py"), &src);
        let want = kinds(
            &src,
            &[
                ("write", &format!("d[{first}]"), 0, "collapsed"),
                ("write", &format!("d[{second}]"), 0, "collapsed"),
            ],
        );
        let got = joins(&r);
        assert_eq!(&got[..2], &want[..2], "{src}");
        assert_eq!(
            (got[2].0.as_str(), got[2].2.as_str()),
            ("node", "joined"),
            "the sink read is alone on its line"
        );
        assert_eq!(
            pairs(&r),
            ["collapsed", "collapsed"],
            "both pairs refused: {src}"
        );
    }
    // A projected and a whole argument of one call, in each order, and a repeated whole argument.
    for (n, call) in ["take(d[\"a\"], d)", "take(d, d[\"a\"])", "take(d, d)"]
        .into_iter()
        .enumerate()
    {
        let src = program(&format!("    d[\"a\"] = t\n    {call}"));
        let r = records(&format!("collide_r{n}.py"), &src);
        let nodes: Vec<_> = joins(&r).into_iter().filter(|j| j.0 == "node").collect();
        let admitted: Vec<_> = nodes.iter().filter(|j| !j.2.is_empty()).collect();
        assert!(!admitted.is_empty(), "{call}");
        assert!(
            admitted.iter().all(|j| j.2 == "collapsed"),
            "{call}: {nodes:?}"
        );
        assert!(pairs(&r).iter().all(|p| p == "collapsed"), "{call}");
        let write = joins(&r).into_iter().find(|j| j.0 == "write").unwrap();
        assert_eq!(write.2, "joined", "the store has its own line: {call}");
    }
    // Control: the same program with the read on another line joins both ends.
    let src = program("    d[\"a\"] = t\n    sink(d)");
    let r = records("collide_control.py", &src);
    assert_eq!(pairs(&r), ["joined"]);
    let all_joined = joins(&r).iter().all(|j| j.2 == "joined");
    assert!(all_joined, "{:?}", joins(&r));
    // Same line W and R: the two endpoints have different keys (path `d[]` against `d`), so the
    // physical join holds both; ordering two occurrences of one line is the route stage's refusal.
    let src = program("    d[\"a\"] = t; sink(d)");
    let r = records("collide_same_line.py", &src);
    assert_eq!(pairs(&r), ["joined"], "{:?}", joins(&r));
}
