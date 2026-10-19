//! Statement-kind census and differential regression tests (operation counts only).
use super::*;
use crate::data_flow::DataFlowGraph;
use std::path::Path;

fn family() -> Vec<(String, String)> {
    let mut cases = Vec::new();
    for n in [1, 16, 64] {
        let tries = (0..n)
            .map(|_| {
                "try {\nx=p;\nconsume(x);\n} catch(e) {\nconsume(x);\n} finally {\nconsume(x);\n}\n"
            })
            .collect::<String>();
        for ext in ["js", "ts", "tsx"] {
            cases.push((
                format!("try-{n}.{ext}"),
                format!("function f(p) {{\nlet x=p;\n{tries}return x;\n}}\n"),
            ));
        }
        cases.push((
            format!("try-{n}.java"),
            format!(
                "class C {{ int f(int p) {{\nint x=p;\n{}return x;\n}} }}\n",
                tries.replace("catch(e)", "catch(Exception e)")
            ),
        ));
        cases.push((format!("try-{n}.py"), format!("def f(p):\n    x = p\n{}    return x\n", "    try:\n        x = p\n        consume(x)\n    except Exception:\n        consume(x)\n    finally:\n        consume(x)\n".repeat(n))));
        cases.push((
            format!("defer-{n}.go"),
            format!(
                "package p\nfunc f(p int) int {{\nx := p\n{}x = 2\nreturn x\n}}\n",
                "defer consume(x)\n".repeat(n)
            ),
        ));
        cases.push((format!("branch-{n}.ts"), format!("function f(p) {{\nlet x=p;\n{}return x;\n}}\n", "if(p) {\nx=1;\n} else {\nx=2;\n}\nswitch(p) {\ncase 1: x=3; break;\ndefault: x=4;\n}\nconsume(x);\n".repeat(n))));
    }
    cases
}

#[test]
#[ignore = "explicit operation census; PRISM_JOIN_COUNTS_OUT selects receipt"]
fn operation_census() {
    let mut cases = family();
    let checker = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("unused");
    let checker = std::env::var("PRISM_JOIN_CHECKER")
        .map(std::path::PathBuf::from)
        .unwrap_or(checker);
    cases.push((
        "checker.ts".into(),
        std::fs::read_to_string(checker).unwrap(),
    ));
    let mut observations = BTreeMap::new();
    for (path, source) in cases {
        let parsed =
            ParsedFile::parse(&path, &source, Language::from_path(&path).unwrap()).unwrap();
        let (_, records) = work_tests::build(parsed);
        eprintln!("counted {path}: {} RD owners", records.len());
        observations.insert(path, records);
    }
    std::fs::write(
        std::env::var("PRISM_JOIN_COUNTS_OUT").unwrap(),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
}

#[test]
fn owner_builds_kind_map_once() {
    let (path, source) = family()
        .into_iter()
        .find(|(path, _)| path == "try-16.ts")
        .unwrap();
    let parsed = ParsedFile::parse(&path, &source, Language::TypeScript).unwrap();
    let (_, records) = work_tests::build(parsed);
    assert_eq!(records.len(), 1);
    let counts = &records[0];
    assert!(counts.join_queries >= 16, "{counts:?}");
    assert_eq!(counts.join_map_builds, 1, "{counts:?}");
    assert_eq!(
        counts.join_map_entries, counts.statement_entries,
        "{counts:?}"
    );
}

#[test]
#[ignore = "external checker owner identity inventory"]
fn checker_owner_inventory() {
    let path = std::env::var("PRISM_JOIN_CHECKER").unwrap();
    let parsed = ParsedFile::parse(
        &path,
        &std::fs::read_to_string(&path).unwrap(),
        Language::TypeScript,
    )
    .unwrap();
    let owners: Vec<_> = parsed
        .all_functions()
        .into_iter()
        .filter_map(|node| {
            parsed.dfg_owner_name(&node).map(|name| {
                (
                    name,
                    node.start_position().row + 1,
                    node.start_byte(),
                    node.end_byte(),
                )
            })
        })
        .collect();
    assert_eq!(
        owners
            .iter()
            .map(|(_, _, start, end)| (start, end))
            .collect::<BTreeSet<_>>()
            .len(),
        owners.len()
    );
    std::fs::write(
        std::env::var("PRISM_JOIN_INVENTORY_OUT").unwrap(),
        serde_json::to_vec_pretty(&owners).unwrap(),
    )
    .unwrap();
}

// Frozen main per-edge construction, independent of the candidate lookup.
pub(super) fn legacy_join(
    parsed: &ParsedFile,
    statements: &[(Line, String)],
    edge: &CfgEdge,
    provenance: ArmProvenance,
) -> bool {
    if provenance.crosses_lexical_arm() {
        return true;
    }
    if !matches!(provenance.origin, EdgeOrigin::Structured) {
        return false;
    }
    #[cfg(test)]
    work_tests::bump(|counts| {
        counts.join_map_entries += statements.len();
        counts.join_map_builds += 1;
        counts.join_queries += 1;
    });
    let kinds: BTreeMap<Line, &str> = statements
        .iter()
        .map(|(line, kind)| (*line, kind.as_str()))
        .collect();
    let from_kind = kinds.get(&edge.from_line).copied().unwrap_or("");
    let to_kind = kinds.get(&edge.to_line).copied().unwrap_or("");
    let try_join = matches!(
        parsed.language,
        Language::Python
            | Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Java
    ) && from_kind == "try_statement";
    let go_defer = parsed.language == Language::Go
        && parsed.language.is_return_node(from_kind)
        && to_kind == "defer_statement";
    try_join || go_defer
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Observation {
    outcomes: Vec<RdOutcome>,
    sets: Vec<(Vec<BitSet>, Vec<BitSet>)>,
}

thread_local! {
    static LEGACY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OBSERVED: std::cell::RefCell<Option<Observation>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn legacy_enabled() -> bool {
    LEGACY.with(std::cell::Cell::get)
}

pub(super) fn record_outcome(outcome: &RdOutcome) {
    OBSERVED.with(|slot| {
        if let Some(observed) = slot.borrow_mut().as_mut() {
            observed.outcomes.push(outcome.clone());
        }
    });
}

pub(super) fn record_sets(bound: &[BitSet], flat: &[BitSet]) {
    OBSERVED.with(|slot| {
        if let Some(observed) = slot.borrow_mut().as_mut() {
            observed.sets.push((bound.to_vec(), flat.to_vec()));
        }
    });
}

struct Mode;
impl Mode {
    fn begin(legacy: bool) -> Self {
        LEGACY.with(|mode| assert!(!mode.replace(legacy)));
        OBSERVED.with(|slot| assert!(slot.replace(Some(Observation::default())).is_none()));
        Self
    }
    fn take() -> Observation {
        OBSERVED.with(|slot| slot.take().unwrap())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        LEGACY.with(|mode| mode.set(false));
        OBSERVED.with(|slot| slot.take());
    }
}

fn compare(path: &str, source: &str) -> (usize, bool) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            let parsed =
                ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap();
            let files = BTreeMap::from([(path.to_string(), parsed)]);
            let mode = Mode::begin(true);
            let old = DataFlowGraph::build(&files);
            let old_observed = Mode::take();
            drop(mode);
            let mode = Mode::begin(false);
            let new = DataFlowGraph::build(&files);
            let new_observed = Mode::take();
            drop(mode);
            assert_eq!(
                old_observed, new_observed,
                "full RD outcomes and bound/flat definition sets: {path}"
            );
            assert_eq!(
                crate::cpg::dfg_observation::dfg_storage(&old),
                crate::cpg::dfg_observation::dfg_storage(&new),
                "complete DFG storage bytes: {path}"
            );
            let incomplete = new_observed.outcomes.iter().any(|outcome| match outcome {
                RdOutcome::Available(result) => result
                    .labels
                    .values()
                    .chain(result.exact_labels.values())
                    .any(|label| *label == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
                _ => false,
            });
            (new_observed.outcomes.len(), incomplete)
        })
}

fn fixture_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "private-corpus"
            || name == "frontend-portal"
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
fn differential_all_fixtures() {
    let mut files = Vec::new();
    for root in ["tests/fixtures", "eval/fixtures"] {
        fixture_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join(root),
            &mut files,
        );
    }
    files.sort();
    assert!(files.len() >= 100);
    let mut owners = 0;
    let mut incomplete = false;
    for file in &files {
        let (count, flag) = compare(
            file.to_str().unwrap(),
            &std::fs::read_to_string(file).unwrap(),
        );
        owners += count;
        incomplete |= flag;
    }
    assert!(owners > 100);
    assert!(incomplete, "cfg_incomplete labels must be compared");
    eprintln!(
        "join-kind differential: {} fixture files, {owners} RD invocations",
        files.len()
    );
}

#[test]
fn differential_generated() {
    let mut incomplete = false;
    for (path, source) in family()
        .into_iter()
        .filter(|(path, _)| !path.contains("64"))
    {
        incomplete |= compare(&path, &source).1;
    }
    for (path, source) in [
        ("duplicates.ts", "function f(p) {\nlet x=p; x=2; use(x);\ntry { x=3; } catch(e) { use(x); }\nif(p) {\nx=4;\n} else {\nuse(x);\n}\nreturn x;\n}"),
        ("defer-after-return.go", "package p\nfunc f(p int) int {\nx:=p\nreturn x\ndefer use(x)\n}\n"),
        ("joins.rs", "fn f(p: i32) -> i32 {\nlet mut x=p;\nmatch p {\n0 => x=1,\n_ => x=2,\n}\nx\n}"),
    ] { incomplete |= compare(path, source).1; }
    assert!(incomplete);
}

fn provenance(origin: EdgeOrigin, cross: bool) -> ArmProvenance {
    ArmProvenance {
        origin,
        from_arm: cfg::ArmSpan {
            id: 1,
            subtree_last: 1,
        },
        to_arm: cfg::ArmSpan {
            id: if cross { 2 } else { 1 },
            subtree_last: if cross { 2 } else { 1 },
        },
    }
}

#[test]
fn duplicate_missing_lines_and_language_rules() {
    for language in Language::all() {
        let parsed = ParsedFile::parse("kinds", "", language).unwrap();
        let is_try = matches!(
            language,
            Language::Python
                | Language::JavaScript
                | Language::TypeScript
                | Language::Tsx
                | Language::Java
        );
        // Both duplicate orders, absent endpoints, return-to-nondefer and
        // nonreturn-to-defer distinguish every original predicate.
        for (statements, from, to, expected) in [
            (
                vec![(1, "try_statement"), (1, "expression_statement")],
                1,
                2,
                false,
            ),
            (
                vec![(1, "expression_statement"), (1, "try_statement")],
                1,
                2,
                is_try,
            ),
            (vec![(2, "defer_statement")], 1, 2, false),
            (vec![(1, "return_statement")], 1, 2, false),
            (
                vec![(1, "return_statement"), (2, "defer_statement")],
                1,
                2,
                language == Language::Go,
            ),
            (
                vec![(1, "expression_statement"), (2, "defer_statement")],
                1,
                2,
                false,
            ),
        ] {
            let statements: Vec<_> = statements
                .into_iter()
                .map(|(line, kind)| (line, kind.to_string()))
                .collect();
            let kinds = join_kind::StatementKinds::new(&statements);
            let edge = CfgEdge {
                file: "kinds".into(),
                from_line: from,
                to_line: to,
            };
            for origin in [EdgeOrigin::Sequential, EdgeOrigin::Structured] {
                for cross in [false, true] {
                    let prov = provenance(origin, cross);
                    let want = (cross && origin == EdgeOrigin::Sequential)
                        || (origin == EdgeOrigin::Structured && expected);
                    assert_eq!(
                        legacy_join(&parsed, &statements, &edge, prov),
                        want,
                        "oracle {language:?}"
                    );
                    assert_eq!(
                        join_kind::is_incomplete_join(&parsed, &kinds, &edge, prov),
                        want,
                        "candidate {language:?} {statements:?} {origin:?} cross={cross}"
                    );
                }
            }
        }
    }
}

#[test]
fn provenance_short_circuits_without_building() {
    let parsed = ParsedFile::parse("lazy.ts", "", Language::TypeScript).unwrap();
    let statements = vec![
        (1, "try_statement".to_string()),
        (2, "expression_statement".to_string()),
    ];
    let edge = CfgEdge {
        file: "lazy.ts".into(),
        from_line: 1,
        to_line: 2,
    };
    let kinds = join_kind::StatementKinds::new(&statements);
    let counts = work_tests::cfg_selection_counts(|| {
        assert!(join_kind::is_incomplete_join(
            &parsed,
            &kinds,
            &edge,
            provenance(EdgeOrigin::Sequential, true)
        ));
        assert!(!join_kind::is_incomplete_join(
            &parsed,
            &kinds,
            &edge,
            provenance(EdgeOrigin::Sequential, false)
        ));
    });
    assert_eq!(counts.join_map_builds, 0);
    assert_eq!(counts.join_map_entries, 0);
    let counts = work_tests::cfg_selection_counts(|| {
        for _ in 0..7 {
            assert!(join_kind::is_incomplete_join(
                &parsed,
                &kinds,
                &edge,
                provenance(EdgeOrigin::Structured, false)
            ));
        }
    });
    assert_eq!(counts.join_queries, 7);
    assert_eq!(counts.join_map_builds, 1);
    assert_eq!(counts.join_map_entries, statements.len());
}

#[test]
fn separate_owners_get_separate_lazy_maps() {
    let source = "function f(p) {\nlet x=p;\ntry {\nx=2;\n} catch(e) {\nuse(x);\n}\nreturn x;\n}\n";
    let parsed = ParsedFile::parse(
        "owners.ts",
        &format!(
            "{source}{}function straight(p) {{\nuse(p);\nreturn p;\n}}\n",
            source.replace("f(p)", "g(p)")
        ),
        Language::TypeScript,
    )
    .unwrap();
    let (_, records) = work_tests::build(parsed);
    assert_eq!(records.len(), 3);
    assert_eq!(
        records
            .iter()
            .map(|r| r.join_map_builds)
            .collect::<Vec<_>>(),
        [1, 1, 0]
    );
    for r in records.iter().take(2) {
        assert_eq!(r.join_map_entries, r.statement_entries);
    }
}
