//! Operation counts only: no clocks, resource samples, or profiling session.
//! The differential compares stored DFG output plus the old parameter lookup,
//! not complete per-owner RdOutcome/RdResult values or an independent scanner.
//! Parameter rows on the signature line do not map to CFG statements, so this
//! differential cannot guard the parameter-region step; scope::reuse_tests does.
use super::*;
use crate::ast::call_membership_tests::inventory_count;
use crate::data_flow::DataFlowGraph;
use std::cell::{Cell, RefCell};
use std::path::Path;

#[derive(Default, Debug, Clone, serde::Serialize)]
pub(super) struct Counts {
    owner: String,
    start: usize,
    language: String,
    pub all_functions: usize,
    // Debug-only invariant inventories are separate from release-path work.
    pub debug_root_inventories: usize,
    pub parameter_queries: usize,
    pub introductions: usize,
    pub binding_facts: usize,
    pub binding_nodes: usize,
    pub cfg_candidates: usize,
    pub join_map_entries: usize,
    pub use_byte_misses: usize,
    pub dominance_calls: usize,
    pub dominance_nodes: usize,
}

thread_local! {
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    static CURRENT: RefCell<Counts> = RefCell::default();
    static RECORDS: RefCell<Vec<Counts>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn bump(f: impl FnOnce(&mut Counts)) {
    if RECORDING.with(Cell::get) {
        CURRENT.with(|counts| f(&mut counts.borrow_mut()));
    }
}

struct Recording;
impl Recording {
    fn begin() -> Self {
        RECORDING.with(|enabled| assert!(!enabled.replace(true)));
        take_records();
        Self
    }
}
impl Drop for Recording {
    fn drop(&mut self) {
        RECORDING.with(|enabled| enabled.set(false));
    }
}

pub(super) struct Pass(Option<usize>);
impl Pass {
    pub(super) fn begin(parsed: &ParsedFile, owner: &Node<'_>) -> Self {
        if !RECORDING.with(Cell::get) {
            return Self(None);
        }
        CURRENT.with(|counts| {
            *counts.borrow_mut() = Counts {
                owner: parsed.dfg_owner_name(owner).unwrap_or_default(),
                start: owner.start_position().row + 1,
                language: format!("{:?}", parsed.language),
                ..Counts::default()
            };
        });
        Self(Some(inventory_count()))
    }
}
impl Drop for Pass {
    fn drop(&mut self) {
        let Some(start) = self.0 else { return };
        bump(|counts| {
            counts.all_functions = inventory_count() - start - counts.debug_root_inventories;
        });
        let counts = CURRENT.with(|counts| counts.borrow().clone());
        RECORDS.with(|records| records.borrow_mut().push(counts));
    }
}

fn take_records() -> Vec<Counts> {
    RECORDS.with(|records| std::mem::take(&mut *records.borrow_mut()))
}

fn build(parsed: ParsedFile) -> (DataFlowGraph, Vec<Counts>) {
    // Both the DFG producer and the RD pass run on this worker. Independent
    // test threads/pools cannot contaminate counters or the differential mode.
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            let _recording = Recording::begin();
            let graph = DataFlowGraph::build(&BTreeMap::from([(parsed.path.clone(), parsed)]));
            (graph, take_records())
        })
}

#[test]
fn zero_introductions_do_not_query_parameters() {
    let parsed = ParsedFile::parse(
        "zero.ts",
        "function f() {\n consume(1);\n return 1;\n}",
        Language::TypeScript,
    )
    .unwrap();
    let func = parsed.all_functions()[0];
    let cfg = cfg::build_cfg_edges_with_arms(&parsed);
    let _recording = Recording::begin();
    let outcome = reaching_definitions_with_exact(&parsed, &func, &[], &[], &[], &cfg);
    assert!(matches!(outcome, RdOutcome::Available(_)));
    let counts = take_records().pop().unwrap();
    assert_eq!(counts.binding_facts, 1, "{counts:?}");
    assert!(counts.binding_nodes > 0, "{counts:?}");
    assert_eq!(counts.introductions, 0, "{counts:?}");
    assert_eq!(counts.all_functions, 0, "{counts:?}");
    assert_eq!(counts.parameter_queries, 0, "{counts:?}");
}

#[test]
fn absent_parameters_are_cached_across_introductions() {
    let _recording = Recording::begin();
    for count in [1, 32, 128] {
        let source = format!(
            "consume(p => {{\n{}consume(p);\nreturn p;\n}});",
            (0..count)
                .map(|i| format!("let x{i} = {i};\n"))
                .collect::<String>()
        );
        let parsed = ParsedFile::parse("absent.ts", &source, Language::TypeScript).unwrap();
        let owner = parsed.all_functions()[0];
        assert!(parsed.find_parameters_node(&owner).is_none());
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        take_records();
        assert!(matches!(
            reaching_definitions_with_exact(&parsed, &owner, &[], &[], &[], &cfg),
            RdOutcome::Available(_)
        ));
        let counts = take_records().pop().unwrap();
        assert!(counts.introductions >= count, "{counts:?}");
        assert_eq!(counts.all_functions, 0, "{counts:?}");
        assert_eq!(counts.parameter_queries, 1, "{counts:?}");
    }
}

#[test]
fn early_exits_do_not_discover_parameters() {
    let _recording = Recording::begin();
    let short = "def f(p):\n    consume(p)\n    return p\n";
    let long = format!("def f(p):\n{}", "    consume(p)\n".repeat(RD_MAX_LINES + 1));
    for (source, def_count, expected) in [
        (
            short,
            RD_MAX_DEFS + 1,
            RdUnavailable::DefinitionsCapExceeded {
                actual: RD_MAX_DEFS + 1,
            },
        ),
        (
            long.as_str(),
            0,
            RdUnavailable::StatementLinesCapExceeded {
                actual: RD_MAX_LINES + 1,
            },
        ),
        (short, 0, RdUnavailable::NoCfgEdges),
    ] {
        let parsed = ParsedFile::parse("exit.py", source, Language::Python).unwrap();
        let owner = parsed.all_functions()[0];
        let defs: Vec<_> = (0..def_count)
            .map(|i| DefSite {
                id: DefId(i as u32),
                path: AccessPath::simple(format!("v{i}")),
                line: 1,
                start_byte: 0,
                alias_derived: false,
            })
            .collect();
        take_records();
        assert_eq!(
            reaching_definitions_with_exact(&parsed, &owner, &defs, &[], &[], &[]),
            RdOutcome::Unavailable(expected)
        );
        let counts = take_records().pop().unwrap();
        assert_eq!(counts.binding_facts, 0, "{counts:?}");
        assert_eq!(counts.introductions, 0, "{counts:?}");
        assert_eq!(counts.parameter_queries, 0, "{counts:?}");
        assert_eq!(counts.all_functions, 0, "{counts:?}");
        assert_eq!(counts.debug_root_inventories, 0, "{counts:?}");
    }
}

#[test]
fn ordinary_passes_do_not_record() {
    let parsed = ParsedFile::parse(
        "disabled.ts",
        "function f(p) {\nconsume(p);\nreturn p;\n}",
        Language::TypeScript,
    )
    .unwrap();
    let owner = parsed.all_functions()[0];
    let cfg = cfg::build_cfg_edges_with_arms(&parsed);
    take_records();
    assert!(!RECORDING.with(Cell::get));
    let run = || reaching_definitions_with_exact(&parsed, &owner, &[], &[], &[], &cfg);
    assert!(matches!(run(), RdOutcome::Available(_)));
    assert!(take_records().is_empty());
    {
        let _recording = Recording::begin();
        assert!(matches!(run(), RdOutcome::Available(_)));
        assert_eq!(take_records().len(), 1);
    }
    assert!(matches!(run(), RdOutcome::Available(_)));
    assert!(take_records().is_empty());
}

#[test]
fn root_inventory_cost_is_constant_per_owner() {
    let _recording = Recording::begin();
    for count in [0, 1, 32, 128] {
        let source = format!(
            "function f(p) {{\n{}consume(p);\nreturn p;\n}}\nfunction sibling() {{ return 0; }}",
            (0..count)
                .map(|i| format!("let x{i} = {i};\n"))
                .collect::<String>()
        );
        let parsed = ParsedFile::parse("cost.ts", &source, Language::TypeScript).unwrap();
        let func = parsed.all_functions()[0];
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        take_records();
        let outcome = reaching_definitions_with_exact(&parsed, &func, &[], &[], &[], &cfg);
        assert!(matches!(outcome, RdOutcome::Available(_)));
        let counts = take_records().pop().unwrap();
        assert!(counts.introductions >= count, "{counts:?}");
        assert_eq!(counts.all_functions, 0, "{counts:?}");
        assert_eq!(counts.parameter_queries, 1, "{counts:?}");
    }
}

#[test]
#[ignore = "explicit external scaling/checker receipt; set PRISM_RD_COUNTS_DIR and PRISM_RD_COUNTS_OUT"]
fn scaling_operation_counts() {
    let root = std::env::var("PRISM_RD_COUNTS_DIR").unwrap();
    let out = std::env::var("PRISM_RD_COUNTS_OUT").unwrap();
    let mut directories: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    directories.sort_by_key(|entry| entry.path());
    let mut observations = BTreeMap::new();
    for directory in directories {
        let path = directory.path();
        assert!(path.is_dir());
        let file = std::fs::read_dir(&path)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let parsed = ParsedFile::parse(
            file.to_str().unwrap(),
            &std::fs::read_to_string(&file).unwrap(),
            Language::TypeScript,
        )
        .unwrap();
        let (_, records) = build(parsed);
        eprintln!("counted {}: {} RD passes", path.display(), records.len());
        observations.insert(
            path.file_name().unwrap().to_string_lossy().to_string(),
            records,
        );
    }
    std::fs::write(
        Path::new(&out),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

thread_local! {
    static LEGACY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Frozen 433b252c lookup; only the lookup is switched, so both paths run
/// the actual producer, RD solver, classifiers, kill lines and statistics.
pub(super) fn parameters<'a>(
    parsed: &'a ParsedFile,
    root: Node<'a>,
    cached: Option<Node<'a>>,
) -> Option<Node<'a>> {
    if LEGACY.with(std::cell::Cell::get) {
        parsed.find_parameters_node(
            &parsed
                .all_functions()
                .into_iter()
                .find(|function| function.id() == root.id())
                .expect("root function must remain indexed"),
        )
    } else {
        cached
    }
}

fn compare(source: &str, path: &str, language: Language) -> Vec<Counts> {
    // Match the existing all-fixture controls: deep C parsing needs this stack.
    rayon::ThreadPoolBuilder::new().num_threads(1).stack_size(64 * 1024 * 1024).build().unwrap().install(|| {
        let parsed = ParsedFile::parse(path, source, language).unwrap();
        let files = BTreeMap::from([(path.to_string(), parsed)]);
        let _recording = Recording::begin();
        LEGACY.with(|mode| mode.set(true));
        let old = DataFlowGraph::build(&files);
        LEGACY.with(|mode| mode.set(false));
        let old_counts = take_records();
        if old_counts.iter().any(|counts| counts.introductions > 0) {
            assert!(old_counts.iter().map(|counts| counts.all_functions).sum::<usize>() > 0,
                "legacy lookup must execute: {path}");
        }
        let new = DataFlowGraph::build(&files);
        let new_counts = take_records();
        assert_eq!(old_counts.len(), new_counts.len(), "pass count: {path}");
        assert_eq!(
            crate::cpg::dfg_observation::dfg_storage(&old),
            crate::cpg::dfg_observation::dfg_storage(&new),
            "RD labels (including exact spans and kill lines), DFG storage and RD statistics: {path}"
        );
        new_counts
    })
}

fn fixture_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "private-corpus"
            || name.starts_with("f-")
            || name.starts_with("F-private")
            || name.to_lowercase().contains("secbench")
        {
            continue;
        }
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            fixture_files(&entry.path(), files);
        } else if kind.is_file() && Language::from_path(entry.path().to_str().unwrap()).is_some() {
            files.push(entry.path());
        }
    }
}

#[test]
fn differential_repository_fixtures() {
    let mut files = Vec::new();
    for relative in ["tests/fixtures", "eval/fixtures"] {
        fixture_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
            &mut files,
        );
    }
    files.sort();
    assert!(files.len() >= 100, "fixture selection unexpectedly empty");
    let mut passes = 0;
    let mut languages = BTreeSet::new();
    for file in &files {
        let path = file.to_str().unwrap();
        let records = compare(
            &std::fs::read_to_string(file).unwrap(),
            path,
            Language::from_path(path).unwrap(),
        );
        for record in records {
            if record.binding_facts > 0 {
                languages.insert(record.language);
            }
            passes += 1;
        }
    }
    eprintln!("differential repository fixtures: {} files, {passes} passes, BindingFacts languages {languages:?}", files.len());
}

#[test]
fn differential_generated_owners() {
    let cases = [
        ("nested.js", "function f(p) {\n let x=p;\n function inner(p) { let x=p; return x; }\n consume(x);\n { let x=2; consume(x); }\n return x;\n}\nconsume((p) => { let x=p; consume(x); return x; });\nconsume(p => { let x=p; return x; });"),
        ("nested.ts", "function f(p: number) {\n let {x, y: z} = source();\n for (const [x] of values) { consume(x); }\n consume(x);\n return (q: number) => { let x=q; consume(x); return x; };\n}"),
        ("nested.tsx", "const C = (p: Props) => {\n const [x, ...xs] = p.items;\n { let x=2; consume(x); }\n return <X value={x} f={(x) => { consume(x); return x; }} />;\n};"),
        ("nested.py", "def f(p):\n    x = p\n    ys = [x for x in p if x]\n    zs = {x: x for x in p}\n    ss = {x for x in p}\n    gs = (x for x in p)\n    a, b = p\n    def inner(x):\n        x = 1\n        return x\n    return x\n"),
        ("nested.go", "package p\nfunc f(p int) int {\n x := p\n if x > 0 { x := 2; consume(x) }\n consume(x)\n return x\n}\n"),
        ("nested.rs", "fn f(p: i32) -> i32 {\n let (x, y) = (p, p);\n { let x = y; consume(x); }\n consume(x);\n x\n}\n"),
        ("nested.c", "int f(int p) {\n int x = p;\n { int x = 2; consume(x); }\n consume(x);\n return x;\n}\n"),
        ("nested.cpp", "int f(int p) {\n auto [x, y] = pair();\n { int x = p; consume(x); }\n consume(x);\n return x;\n}\n"),
        ("nested.java", "class C { int f(int p) {\n int x = p;\n { int y = x; consume(y); }\n consume(x);\n return x;\n} }\n"),
        ("nested.lua", "function f(p)\n local x = p\n do local x = 2; consume(x) end\n consume(x)\n return x\nend\n"),
        ("nested.sh", "f() {\n local x=1\n echo \"$x\"\n x=2\n echo \"$x\"\n}\n"),
        ("nested.tf", "locals {\n x = 1\n y = local.x\n}\n"),
    ];
    let mut languages = BTreeSet::new();
    let mut synthetic = false;
    let mut passes = 0;
    for (path, source) in cases {
        for repeat in [1, 2, 4] {
            let records = compare(
                &source.repeat(repeat),
                path,
                Language::from_path(path).unwrap(),
            );
            for record in records {
                if record.binding_facts > 0 {
                    languages.insert(record.language);
                }
                synthetic |= record.owner.starts_with("<anon@");
                passes += 1;
            }
        }
    }
    assert!(synthetic, "synthetic owners must execute RD");
    let expected: BTreeSet<_> = Language::all()
        .into_iter()
        .filter(|lang| *lang != Language::Terraform)
        .map(|lang| format!("{lang:?}"))
        .collect();
    assert_eq!(languages, expected);
    eprintln!("differential generated: {} cases, {passes} RD passes, BindingFacts languages {languages:?}; Terraform has no callable owners", cases.len()*3);
}
