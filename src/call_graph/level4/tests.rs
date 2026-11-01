use super::*;

fn known() -> BTreeSet<String> {
    ["f", "g", "handler", "café", "do_open", "do_close"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn same(sources: &[(&str, &str)]) -> Index {
    let known = known();
    let old = reference(sources, &known);
    let new = build_sources(sources, &known);
    assert_eq!(new, old, "whole index: {sources:?}");
    assert_eq!(
        serde_json::to_vec(&new).unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
    new
}

#[test]
fn selector_skips_accessor_only_and_equality_lines() {
    for line in [
        "",
        "x.cb; t->cbx; obj.café",
        "a.cb == f; b->next === g;",
        "const x = obj.cb; p->cbx;",
    ] {
        assert!(assignment_fields(line).is_empty(), "{line}");
    }
}

#[test]
fn selector_keeps_only_maximal_assignment_fields() {
    let line = "x.read; y->unused; a.cb = f; b->cbx = g; c.cb == f; z.café\u{a0}= &handler;";
    let expected = ["cb", "cbx", "café"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(assignment_fields(line), expected);
}

#[test]
fn whole_index_pins_legacy_quirks_and_raw_source() {
    let sources = [
        ("z.ts", "s.cb = f; t->cb = g;\ns.cb = f; t->cbx = g;\n"),
        ("a.c", "static struct ops o = { .open = do_open, .close = do_close };\r\na.cb == f;\r\nb.cb = &&handler;\r\n"),
        ("raw.py", "# x.cb = f;\ntext = 'x.café\u{2003}= café); y.cb = g}'\nobj.cb = missing;\nobj.cb = &f, x;\nobj.cb = g extra;\n"),
        ("empty.rs", "// obj.cb; obj.cbx == f\n"),
    ];
    let index = same(&sources);
    assert_eq!(index["cb"]["z.ts"], BTreeSet::from(["g".into()]));
    assert_eq!(index["cb"]["a.c"], BTreeSet::from(["handler".into()]));
    assert_eq!(index["café"]["raw.py"], BTreeSet::from(["café".into()]));
    assert!(!index.values().any(|files| files.contains_key("empty.rs")));
    assert!(same(&[]).is_empty());
    assert!(build_sources(&sources, &BTreeSet::new()).is_empty());
}

#[test]
fn generated_lines_match_whole_legacy_index() {
    let pieces = [
        "s.cb = f;",
        "t->cbx = g;",
        "t->cb = g;",
        "x.cb == f;",
        "x.cb = &handler,",
        "x.café\u{a0}= café)",
        "// x.cb = f;",
        "'x.cb = g}'",
        "o.read.read2",
        "->",
        ".",
        "->.cb = f;",
        "..cb = g;",
        "x.123 = f;",
        "x.cb = unknown;",
        "x.cb = &&f;",
        "x.cb = f\u{2003}g",
        "x.cb = f==g;",
        "x.cb = &;",
        "x.cb\r= g;",
        "x.cbé = f;",
        "x.cb =",
        "x.cb = &g;",
        "x.β_1 = f;",
        "x.cb => f;",
        "/* ->cbxyz */",
    ];
    let mut seed = 0x437_u64;
    let mut source = String::new();
    for _ in 0..8000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        for _ in 0..1 + (seed >> 32) % 12 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            source.push_str(pieces[(seed >> 32) as usize % pieces.len()]);
            source.push(' ');
        }
        source.push_str(if seed & 1 == 0 { "\r\n" } else { "\n" });
    }
    // Compare per-line as well as aggregates: a later line must not mask a miss.
    for line in source.lines() {
        same(&[("generated.ts", line)]);
    }
    same(&[("z.ts", &source), ("a.ts", &source), ("b.ts", "x.cb = &f;")]);
}

#[test]
#[ignore = "synthetic timing; reads only the explicitly generated tree"]
fn synthetic_index_timing() {
    let root = std::env::var("LEVEL4_SYNTHETIC_ROOT").expect("generated synthetic root");
    let mut owned = std::fs::read_dir(root)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            (
                path.display().to_string(),
                std::fs::read_to_string(path).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    owned.sort();
    let sources: Vec<_> = owned
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect();
    let known = known();
    // Warm the pool; parsing and filesystem reads are outside measured regions.
    crate::build_pool::install(|| {});
    for round in 0..3 {
        let start = std::time::Instant::now();
        let old = reference(&sources, &known);
        let old_elapsed = start.elapsed();
        let start = std::time::Instant::now();
        let new = build_sources(&sources, &known);
        let new_elapsed = start.elapsed();
        assert_eq!(new, old);
        let bytes = serde_json::to_vec(&new).unwrap();
        println!(
            "round={round} files={} source_bytes={} old_ms={:.3} new_ms={:.3} index_bytes={}",
            sources.len(),
            owned.iter().map(|(_, s)| s.len()).sum::<usize>(),
            old_elapsed.as_secs_f64() * 1000.0,
            new_elapsed.as_secs_f64() * 1000.0,
            bytes.len()
        );
    }
}
