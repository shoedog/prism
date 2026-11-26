//! Opt-in conformance of `PRISM_ELEMENT_RECALL_TRACE` (lane 10, #847, I0). A child `prism` process
//! (the release binary named by `PRISM_DIAG_BIN`) reads the Q01/Q03/Q04/Q05/Q12 programs of the
//! T2 matrix cold; the stderr records are compared with expectations frozen from each program's
//! text, and the default output must not move with the flag unset, `0` or `1`.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const CASES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/subscript_matrix/cases"
);
const PREFIX: &str = "element-recall\tv1\t";

/// The child binary named by `bin`. An unset variable skips the checks unless `require` (the
/// verification job sets `PRISM_DIAG_REQUIRE=1`), where a missing or unusable binary is an error.
fn resolve_binary(
    bin: Option<std::ffi::OsString>,
    require: bool,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = bin else {
        return match require {
            true => Err("PRISM_DIAG_REQUIRE=1 but PRISM_DIAG_BIN is unset".to_string()),
            false => Ok(None),
        };
    };
    match Path::new(&path).is_file() {
        true => Ok(Some(path.into())),
        false => Err(format!("PRISM_DIAG_BIN {path:?} is not a file")),
    }
}

fn binary() -> Option<PathBuf> {
    let require = std::env::var("PRISM_DIAG_REQUIRE").is_ok_and(|v| v == "1");
    match resolve_binary(std::env::var_os("PRISM_DIAG_BIN"), require) {
        Ok(None) => {
            eprintln!("PRISM_DIAG_BIN is unset: the child-process trace checks are not run");
            None
        }
        Ok(found) => found,
        Err(why) => panic!("{why}"),
    }
}

#[test]
fn child_trace_checks_fail_when_required_and_the_binary_is_missing() {
    use std::ffi::OsString;
    // required: an unset variable and a missing file both fail instead of skipping
    assert!(resolve_binary(None, true).is_err());
    let gone = Some(OsString::from("/nonexistent/prism-diag-bin"));
    assert!(resolve_binary(gone.clone(), true).is_err());
    // not required: unset skips (the notice is printed by `binary`), a bad path still fails
    assert_eq!(resolve_binary(None, false), Ok(None));
    assert!(resolve_binary(gone, false).is_err());
    // a usable binary resolves either way
    let here = std::env::current_exe().unwrap();
    for require in [false, true] {
        let got = resolve_binary(Some(here.clone().into_os_string()), require);
        assert_eq!(got, Ok(Some(here.clone())));
    }
}

/// Expected projections of one program: `«x»` marks the span inside its context text.
struct Case {
    file: &'static str,
    lines: Vec<String>,
}

fn source(file: &str) -> String {
    std::fs::read_to_string(format!("{CASES}/{file}.py")).unwrap()
}

fn span(src: &str, marked: &str) -> String {
    let (pre, rest) = marked.split_once('«').expect(marked);
    let (inner, post) = rest.split_once('»').expect(marked);
    let plain = format!("{pre}{inner}{post}");
    assert_eq!(
        src.matches(&plain).count(),
        1,
        "context must be unique: {plain}"
    );
    let at = src.find(&plain).unwrap() + pre.len();
    format!("{at}:{}", at + inner.len())
}

/// A store, a whole read and their pair, all joined (the Q01 shape).
fn plain(file: &'static str, w: (&str, &str), r: (&str, &str), role: &str) -> Case {
    let src = source(file);
    let (ws, rs) = (span(&src, w.0), span(&src, r.0));
    let owner = if file == "Q01.return" { "make" } else { "case" };
    let lines = vec![
        format!("O {owner} w=1 n=1 p=1"),
        format!("W {ws} {} Assign joined", w.1),
        format!("N {rs} {} {role} joined", r.1),
        format!("P {ws}>{rs} {role} joined"),
    ];
    Case { file, lines }
}

/// One admitted store whose only read is refused by the syntax (no pair, no join).
fn refused(file: &'static str, w: &str, r: &str, why: &str) -> Case {
    let src = source(file);
    let (ws, rs) = (span(&src, w), span(&src, r));
    let lines = vec![
        "O case w=1 n=1 p=0".into(),
        format!("W {ws} d[] Assign joined"),
        format!("N {rs} d refused:{why} -"),
    ];
    Case { file, lines }
}

/// A refused store (`why`) with the one admitted whole read `sink(d)`, which still joins.
fn store(file: &'static str, w: &str, path: &str, why: &str) -> Case {
    let src = source(file);
    let (ws, rs) = (span(&src, w), span(&src, "sink(«d»)"));
    let lines = vec![
        "O case w=1 n=1 p=0".into(),
        format!("W {ws} {path} refused:{why} -"),
        format!("N {rs} d Argument joined"),
    ];
    Case { file, lines }
}

fn cases() -> Vec<Case> {
    let (a, d, sink) = (("«d[\"a\"]» = t", "d[]"), ("", "d"), "sink(«d»)");
    let at = |ctx: &'static str| (ctx, d.1);
    let o = |ctx: &'static str, path: &'static str| (ctx, path);
    let it = ("«o.items[0][0]» = t", "o.items[][]");
    let mut all = vec![
        plain("Q01.argument", ("«d[k]» = t", "d[]"), at(sink), "Argument"),
        plain("Q01.assignment", a, at("e = «d»"), "AssignmentRhs"),
        plain(
            "Q01.fstring",
            ("«d[f\"slot-{k}\"]» = t", "d[]"),
            at(sink),
            "Argument",
        ),
        plain("Q01.integer", ("«d[0]» = t", "d[]"), at(sink), "Argument"),
        plain(
            "Q01.key-effect",
            ("«d[(key := \"a\")]» = t", "d[]"),
            at(sink),
            "Argument",
        ),
        plain("Q01.keyword", a, at("take(value=«d»)"), "Keyword"),
        plain("Q01.parenthesized", a, at("sink((«d»))"), "Argument"),
        plain(
            "Q01.rebind-index",
            ("«d[(d := {}) or \"a\"]» = t", "d[]"),
            at(sink),
            "Argument",
        ),
        plain(
            "Q01.rebind-rhs",
            ("«d[\"a\"]» = ((", "d[]"),
            at(sink),
            "Argument",
        ),
        plain(
            "Q01.return",
            ("«d[\"a\"]» = value", "d[]"),
            at("return «d»"),
            "Return",
        ),
        plain("Q03.parenthesized", a, at("( «d» )"), "AssignmentRhs"),
        plain("Q04.different-line", a, at(sink), "Argument"),
        plain("Q04.same-line", a, at(sink), "Argument"),
        plain("Q05.root", it, o("sink(«o»)", "o"), "Argument"),
        plain(
            "Q05.prefix",
            it,
            o("sink(«o.items»)", "o.items"),
            "Argument",
        ),
        plain("Q05.replace-prefix", it, o("sink(«o»)", "o"), "Argument"),
        plain("Q05.write-sibling", it, o("sink(«o»)", "o"), "Argument"),
    ];
    // Excluded or wrapped operand contexts: the store joins, no operand is admitted.
    let w = "«d[\"a\"]» = t";
    for (file, r, why) in [
        ("Q03.identity", "id(«d»)", "ExcludedCallee"),
        ("Q03.length", "len(«d»)", "ExcludedCallee"),
        ("Q03.shadowed-len", "len(«d»)", "ExcludedCallee"),
        ("Q03.literal-leaf", "[«d»]", "Wrapped"),
        ("Q03.membership", "in «d»", "Wrapped"),
        ("Q03.projected", "x = «d»[\"a\"]", "Projected"),
        ("Q03.receiver", "x = «d».get", "Receiver"),
        ("Q12.deferred", "lambda: «d»", "Fenced"),
    ] {
        all.push(refused(file, w, r, why));
    }
    all.push(refused(
        "Q12.slice-read",
        "«d[0]» = t",
        "x = «d»[:1]",
        "Slice",
    ));
    all.push(store(
        "Q12.extended-slice",
        "«d[::2]» = [t, None]",
        "d[]",
        "Slice",
    ));
    all.push(store("Q12.slice-write", "«d[:1]» = [t]", "d[]", "Slice"));
    all.push(store(
        "Q12.unpack",
        "«d[\"a\"]», other = t, 0",
        "d[]",
        "Unpack",
    ));
    all.push(store(
        "Q12.loop-head",
        "for «d[\"a\"]» in [t]",
        "d[]",
        "BindingTarget",
    ));
    let call = span(&source("Q12.call-root"), "«make()[\"a\"]» = t");
    let lines = vec![
        "O case w=1 n=0 p=0".into(),
        format!("W {call} [] refused:NonNameRoot -"),
    ];
    all.push(Case {
        file: "Q12.call-root",
        lines,
    });
    // Q03.projected-with-whole: a whole read (joined) beside a projected one on another line.
    let src = source("Q03.projected-with-whole");
    let (ws, whole, proj) = (
        span(&src, w),
        span(&src, "copy = «d»"),
        span(&src, "x = «d»[\"a\"]"),
    );
    let lines = vec![
        "O case w=1 n=2 p=1".into(),
        format!("W {ws} d[] Assign joined"),
        format!("N {whole} d AssignmentRhs joined"),
        format!("N {proj} d refused:Projected -"),
        format!("P {ws}>{whole} AssignmentRhs joined"),
    ];
    all.push(Case {
        file: "Q03.projected-with-whole",
        lines,
    });
    // Q05.field-after-index and Q05.indexed-read: the store is refused or has no whole read.
    let src = source("Q05.field-after-index");
    let (ws, rs) = (span(&src, "«o.items[0].name» = t"), span(&src, "sink(«o»)"));
    let lines = vec![
        "O case w=1 n=1 p=0".into(),
        format!("W {ws} o.items[] refused:DotAfterBracket -"),
        format!("N {rs} o Argument joined"),
    ];
    all.push(Case {
        file: "Q05.field-after-index",
        lines,
    });
    let src = source("Q05.indexed-read");
    let (ws, rs) = (span(&src, it.0), span(&src, "x = «o.items»[0]"));
    let lines = vec![
        "O case w=1 n=1 p=0".into(),
        format!("W {ws} o.items[][] Assign joined"),
        format!("N {rs} o.items refused:Projected -"),
    ];
    all.push(Case {
        file: "Q05.indexed-read",
        lines,
    });
    let src = source("Q05.other-prefix");
    let (ws, rs) = (span(&src, it.0), span(&src, "sink(«o.other»)"));
    let lines = vec![
        "O case w=1 n=1 p=0".into(),
        format!("W {ws} o.items[][] Assign joined"),
        format!("N {rs} o.other Argument joined"),
    ];
    all.push(Case {
        file: "Q05.other-prefix",
        lines,
    });
    all
}

fn repo(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for f in files {
        std::fs::copy(
            format!("{CASES}/{f}.py"),
            dir.path().join(format!("{f}.py")),
        )
        .unwrap();
    }
    dir
}

fn run(
    bin: &Path,
    dir: &Path,
    flag: Option<&str>,
    threads: &str,
    args: &[&str],
) -> (Vec<u8>, String) {
    let mut cmd = Command::new(bin);
    cmd.env_remove("PRISM_ELEMENT_RECALL_TRACE")
        .env("RAYON_NUM_THREADS", threads);
    cmd.env("PRISM_EDGES_BYTES", "1");
    if let Some(flag) = flag {
        cmd.env("PRISM_ELEMENT_RECALL_TRACE", flag);
    }
    let out = cmd.args(args).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{args:?} in {dir:?}: {stderr}");
    (out.stdout, stderr)
}

fn nav(
    bin: &Path,
    dir: &Path,
    flag: Option<&str>,
    threads: &str,
    tail: &[&str],
) -> (Vec<u8>, String) {
    let mut args = vec![
        "nav",
        "--no-cache",
        "dfg-stats",
        "--repo",
        dir.to_str().unwrap(),
    ];
    args.extend(tail);
    run(bin, dir, flag, threads, &args)
}

/// `file -> projected record lines`, sorted, over every trace line of `stderr`.
fn bag(stderr: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let text = |v: &serde_json::Value| v.as_str().unwrap_or("-").to_string();
    for line in stderr.lines().filter_map(|l| l.strip_prefix(PREFIX)) {
        let r: serde_json::Value = serde_json::from_str(line).unwrap();
        if r["kind"] == "prediction" {
            continue; // the prediction bag is checked by its own test
        }
        let (s, e) = (&r["bytes"][0], &r["bytes"][1]);
        let (path, join) = (text(&r["path"]), text(&r["join"]));
        let file = text(&r["file"]).trim_end_matches(".py").to_string();
        let lines = out.entry(file).or_default();
        match r["kind"].as_str().unwrap() {
            "owner" => {
                let (owner, w, n) = (text(&r["owner"]), &r["writes"], &r["nodes"]);
                let pairs = r["pairs"].as_array().unwrap();
                lines.push(format!("O {owner} w={w} n={n} p={}", pairs.len()));
                for p in pairs {
                    let (w, rd) = (&p["w"], &p["r"]);
                    let ends = format!("{}:{}>{}:{}", w[0], w[1], rd[0], rd[1]);
                    lines.push(format!(
                        "P {ends} {} {}",
                        text(&p["role"]),
                        text(&p["join"])
                    ));
                }
            }
            "write" => lines.push(format!("W {s}:{e} {path} {} {join}", text(&r["write"]))),
            _ => lines.push(format!("N {s}:{e} {path} {} {join}", text(&r["role"]))),
        }
    }
    out.values_mut().for_each(|v| v.sort());
    out
}

/// The collisions: the programs whose endpoints share a key on one line.
fn collisions(want: &mut BTreeMap<String, Vec<String>>) {
    let mut put = |file: &str, mut lines: Vec<String>| {
        lines.sort();
        want.insert(file.into(), lines);
    };
    let dup = source("Q04.duplicate-write");
    let (a, b) = (span(&dup, "«d[\"a\"]»"), span(&dup, "«d[\"b\"]»"));
    let sink = span(&dup, "sink(«d»)");
    put(
        "Q04.duplicate-write",
        vec![
            "O case w=2 n=1 p=2".into(),
            format!("W {a} d[] Assign collapsed"),
            format!("W {b} d[] Assign collapsed"),
            format!("N {sink} d Argument joined"),
            format!("P {a}>{sink} Argument collapsed"),
            format!("P {b}>{sink} Argument collapsed"),
        ],
    );
    let rd = source("Q04.duplicate-read");
    let w = span(&rd, "«d[\"a\"]» = t");
    let (r1, r2) = (span(&rd, "take(«d», d)"), span(&rd, "take(d, «d»)"));
    put(
        "Q04.duplicate-read",
        vec![
            "O case w=1 n=2 p=2".into(),
            format!("W {w} d[] Assign joined"),
            format!("N {r1} d Argument collapsed"),
            format!("N {r2} d Argument collapsed"),
            format!("P {w}>{r1} Argument collapsed"),
            format!("P {w}>{r2} Argument collapsed"),
        ],
    );
    for (file, whole_ctx, proj_ctx) in [
        (
            "Q04.projected-first",
            "take(d[\"a\"], «d»)",
            "take(«d»[\"a\"], d)",
        ),
        (
            "Q04.whole-first",
            "take(«d», d[\"a\"])",
            "take(d, «d»[\"a\"])",
        ),
    ] {
        let text = source(file);
        let w = span(&text, "«d[\"a\"]» = t");
        let (whole, proj) = (span(&text, whole_ctx), span(&text, proj_ctx));
        put(
            file,
            vec![
                "O case w=1 n=2 p=1".into(),
                format!("W {w} d[] Assign joined"),
                format!("N {whole} d Argument collapsed"),
                format!("N {proj} d refused:Projected -"),
                format!("P {w}>{whole} Argument collapsed"),
            ],
        );
    }
}

#[test]
fn element_trace_exact_write_node_owner_multisets() {
    let Some(bin) = binary() else { return };
    let cases = cases();
    let mut want: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in &cases {
        want.insert(c.file.to_string(), c.lines.iter().cloned().collect());
    }
    collisions(&mut want);
    want.values_mut().for_each(|v| v.sort());
    // Q12.initializer holds no candidate store and so has no records.
    let files: Vec<_> = want
        .keys()
        .map(String::as_str)
        .chain(["Q12.initializer"])
        .collect();
    let dir = repo(&files);
    let (_, stderr) = nav(&bin, dir.path(), Some("1"), "1", &[]);
    let got = bag(&stderr);
    assert!(
        want.values().map(Vec::len).sum::<usize>() > 100 && !got.is_empty(),
        "the trace ran"
    );
    let mut failures = Vec::new();
    for (file, lines) in &want {
        if got.get(file) != Some(lines) {
            failures.push(format!(
                "{file}\n  want {lines:#?}\n  got  {:#?}",
                got.get(file)
            ));
        }
    }
    for file in got.keys().filter(|f| !want.contains_key(*f)) {
        failures.push(format!("unexpected records for {file}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // Rayon: the same multiset on four threads.
    let (_, four) = nav(&bin, dir.path(), Some("1"), "4", &[]);
    assert_eq!(bag(&four), got);
}

#[test]
fn element_trace_is_opt_in() {
    let Some(bin) = binary() else { return };
    let files = [
        "Q01.argument",
        "Q04.duplicate-write",
        "Q05.prefix",
        "Q12.unpack",
    ];
    let dir = repo(&files);
    std::fs::write(
        dir.path().join("q.diff"),
        "--- a/Q01.argument.py\n+++ b/Q01.argument.py\n@@ -2,3 +2,3 @@ def case():\n-    t = src(0)\n+    t = src()\n     d = {}; k = \"a\"\n     d[k] = t\n",
    )
    .unwrap();
    let diff = dir.path().join("q.diff");
    let d = dir.path().to_str().unwrap();
    let review = [
        "--no-cache",
        "--repo",
        d,
        "--diff",
        diff.to_str().unwrap(),
        "--algorithm",
        "leftflow,taint",
        "--format",
        "json",
    ];
    let surfaces: [(&str, Vec<&str>); 3] = [
        ("stats", vec![]),
        ("edges", vec!["--edges"]),
        ("review", review.to_vec()),
    ];
    for (name, tail) in surfaces {
        let go = |flag: Option<&str>| match name {
            "review" => run(&bin, dir.path(), flag, "4", &tail),
            _ => nav(&bin, dir.path(), flag, "4", &tail),
        };
        let (unset, unset_err) = go(None);
        let (zero, zero_err) = go(Some("0"));
        let (other, other_err) = go(Some("true"));
        let (on, on_err) = go(Some("1"));
        assert!(!unset.is_empty(), "{name} wrote output");
        assert_eq!(unset, zero, "{name}: unset against 0");
        assert_eq!(unset, other, "{name}: unset against true");
        assert_eq!(unset, on, "{name}: the trace never changes stdout");
        for err in [&unset_err, &zero_err, &other_err] {
            assert!(!err.contains(PREFIX), "{name}: records without the flag");
        }
        assert_eq!(unset_err, zero_err, "{name}: stderr unset against 0");
        // A cold nav build runs the producer (the review path may cache nothing, `--no-cache`).
        let n = on_err.lines().filter(|l| l.starts_with(PREFIX)).count();
        assert!(
            n >= if name == "review" { 3 } else { 10 },
            "{name}: the trace ran ({n} records)"
        );
    }
    // The stage-T public groups stay enabled; the recall groups stay off (P14 is RED until I2).
    let matrix = include_str!("subscript_matrix_test.rs");
    assert!(
        matrix.contains("const ENABLED: &[&str] = &[\"control\", \"baseline-gap\", \"boundary\"];")
    );
}

const ROWS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/subscript_matrix/rows.jsonl"
);

/// Rows whose matrix expectation is for an injected limit or RD cap. The child process runs the
/// production constants, which none of these programs reach: every pair of the program is
/// predicted (Q10.raw-1000001 is 13 raw pairs, not 1,000,001; the Q11 rows inject an RD exit).
const PRODUCTION_OVERRIDE: &[&str] = &[
    "Q10.raw-1000001",
    "Q11.def-cap",
    "Q11.fallback-cap",
    "Q11.line-cap",
    "Q11.without-cfg",
];

/// The rows call both positional and keyword operands `argument`.
fn norm(role: &str) -> String {
    match role.replace(['_', ' '], "").to_lowercase().as_str() {
        "keyword" => "argument".into(),
        other => other.into(),
    }
}

/// Pairs the design admits that no matrix row marks, written from the program text: another
/// store of the same storage (a clean overwrite or sibling is kept, never compared by key, DESIGN
/// section 4) and a whole read the row's boundary is about something else than (`take(d, clean)`
/// of the first-formal row, `copy = d` of the projected-with-whole row). `(file, W, R, role)`.
const UNMARKED: &[(&str, &str, &str, &str)] = &[
    (
        "Q02.replace-a",
        "«d[\"a\"]» = \"clean\"",
        "sink(«d»)",
        "argument",
    ),
    (
        "Q13.other-slot",
        "«d[\"b\"]» = \"clean\"",
        "sink(«d»)",
        "argument",
    ),
    (
        "Q13.same-slot",
        "«d[\"a\"]» = \"clean\"",
        "sink(«d»)",
        "argument",
    ),
    (
        "Q13.replace-indexed-ancestor",
        "«d[0]» = [None]",
        "sink(«d»)",
        "argument",
    ),
    (
        "Q02.first-formal",
        "«d[\"a\"]» = t",
        "take(«d», clean)",
        "argument",
    ),
    (
        "Q03.projected-with-whole",
        "«d[\"a\"]» = t",
        "copy = «d»",
        "assignmentrhs",
    ),
];

/// `file -> sorted "w>r role"` frozen from the T2 matrix rows (`label` set = the pair is an
/// expected addition), independent of the preview.
fn frozen_predictions() -> BTreeMap<String, Vec<String>> {
    let text = std::fs::read_to_string(ROWS).unwrap();
    let mut want: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let id = row["id"].as_str().unwrap();
        let Some(file) = id.strip_suffix(".py").filter(|f| f.starts_with('Q')) else {
            continue;
        };
        let all = PRODUCTION_OVERRIDE.contains(&file);
        let bytes = |v: &serde_json::Value| format!("{}:{}", v["bytes"][0], v["bytes"][1]);
        let role = |r: &serde_json::Value| {
            let reads = row["reads"].as_array().unwrap();
            let hit = reads.iter().find(|x| bytes(x) == bytes(r));
            norm(hit.map_or("?", |x| x["role"].as_str().unwrap()))
        };
        let lines = want.entry(file.to_string()).or_default();
        for m in row["markers"].as_array().unwrap() {
            for p in m["pairs"].as_array().unwrap() {
                if all || !p["label"].is_null() {
                    let (w, r) = (&p["write"], &p["read"]);
                    lines.push(format!("{}>{} {}", bytes(w), bytes(r), role(r)));
                }
            }
        }
    }
    for (file, w, r, role) in UNMARKED {
        let text = source(file);
        let line = format!("{}>{} {role}", span(&text, w), span(&text, r));
        want.get_mut(*file).expect(file).push(line);
    }
    want.values_mut().for_each(|v| v.sort());
    want
}

/// `file -> sorted "w>r role"` over the `predicted` prediction records, plus every refusal name.
fn predictions(stderr: &str) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, usize>) {
    let (mut got, mut refusals) = (BTreeMap::new(), BTreeMap::new());
    for line in stderr.lines().filter_map(|l| l.strip_prefix(PREFIX)) {
        let r: serde_json::Value = serde_json::from_str(line).unwrap();
        if r["kind"] != "prediction" {
            continue;
        }
        let file = r["file"]
            .as_str()
            .unwrap()
            .trim_end_matches(".py")
            .to_string();
        let outcome = r["outcome"].as_str().unwrap();
        if outcome != "predicted" {
            *refusals.entry(outcome.to_string()).or_insert(0) += 1;
            continue;
        }
        let ends = format!("{}:{}>{}:{}", r["w"][0], r["w"][1], r["r"][0], r["r"][1]);
        let role = norm(r["role"].as_str().unwrap());
        got.entry(file)
            .or_insert_with(Vec::new)
            .push(format!("{ends} {role}"));
    }
    got.values_mut().for_each(|v: &mut Vec<String>| v.sort());
    (got, refusals)
}

#[test]
fn element_trace_exact_prediction_multiset() {
    let Some(bin) = binary() else { return };
    let want = frozen_predictions();
    // every Q program of the matrix, predictions included only where the frozen rows expect one
    assert!(want.len() > 170, "the Q matrix: {} programs", want.len());
    let files: Vec<_> = want.keys().map(String::as_str).collect();
    let dir = repo(&files);
    let (_, stderr) = nav(&bin, dir.path(), Some("1"), "1", &[]);
    let (got, refusals) = predictions(&stderr);
    let expected: usize = want.values().map(Vec::len).sum();
    assert!(expected > 150, "frozen {expected} predicted pairs");
    let mut failures = Vec::new();
    for (file, lines) in want.iter().filter(|(_, v)| !v.is_empty()) {
        if got.get(file) != Some(lines) {
            failures.push(format!(
                "{file}\n  want {lines:?}\n  got  {:?}",
                got.get(file)
            ));
        }
    }
    for file in got
        .keys()
        .filter(|f| want.get(*f).is_none_or(|v| v.is_empty()))
    {
        failures.push(format!(
            "unexpected predictions for {file}: {:?}",
            got[file]
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // Q09 and Q10 at the production limits: nine sources lose the whole read, 257 pairs the owner
    assert_eq!(refusals.get("refused:fanin"), Some(&9));
    assert_eq!(refusals.get("refused:ownercap"), Some(&257));
    assert!(
        refusals.keys().all(|k| k.starts_with("refused:")),
        "{refusals:?}"
    );
    // Rayon: the same multiset on four threads
    let (_, four) = nav(&bin, dir.path(), Some("1"), "4", &[]);
    assert_eq!(predictions(&four), (got, refusals));
}
