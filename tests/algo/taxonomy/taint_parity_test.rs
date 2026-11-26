use prism::ast::ParsedFile;
use prism::cpg::{CpgContext, CpgNode};
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::finding_confidence::{classify_with_evidence, FindingConfidence, ParseQuality};
use prism::navigation::types::{Reachability, SymbolRef};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::reasoning::seeds::{resolve, SeedRole, SeedSpec};
use prism::reasoning::taint_reaches::taint_reaches;
use prism::repo_loader::{load_repo, LoadedRepo};
use prism_review::algorithms::taint::{slice, slice_with_trace, TaintConfig, TaintRootOrigin};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};
use std::sync::Arc;
use tempfile::TempDir;

#[path = "taint_parity_test/ledger_checks.rs"]
mod ledger_checks;
#[path = "taint_parity_test/reason_checks.rs"]
mod reason_checks;
mod rows;
use rows::{
    rows, validate_labels, Bytes, Expectations, Finding, Level, Row, Slot, Verdict, Walk, GROUPS,
};

const ENABLED: &[&str] = &["control", "guard", "reasons"];
const SHIPPED_SLICES: &[&str] = &["guard", "reasons"];

/// `PARITY_ENABLED=control,binding` replaces `ENABLED` at run time (a worker's hand-enabled RED record);
/// the committed constant is what CI runs.
fn enabled_groups() -> Vec<String> {
    match std::env::var("PARITY_ENABLED") {
        Ok(list) => list.split(',').map(str::to_string).collect(),
        Err(_) => ENABLED.iter().map(|group| group.to_string()).collect(),
    }
}
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/taint_parity");
struct Fixture {
    _dir: TempDir,
    repo: Arc<LoadedRepo>,
}

impl Fixture {
    fn load(row: &Row) -> Self {
        Self::load_with(row, |_, _| {})
    }

    fn load_with(row: &Row, edit: impl Fn(&str, &mut String)) -> Self {
        let fixture = Path::new(&row.fixture);
        assert_eq!(fixture.components().count(), 1, "{}", row.id);
        assert!(
            matches!(fixture.components().next(), Some(Component::Normal(_))),
            "{}",
            row.id
        );
        let dir = tempfile::tempdir().unwrap();
        let path = Path::new(ROOT).join(fixture);
        for entry in fs::read_dir(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display())) {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file(), "{}", row.id);
            let name = entry.file_name();
            let mut source = fs::read_to_string(entry.path()).unwrap();
            edit(name.to_str().unwrap(), &mut source);
            fs::write(dir.path().join(name), source).unwrap();
        }
        let repo = Arc::new(load_repo(dir.path()).unwrap());
        assert_eq!(
            repo.files.len(),
            fs::read_dir(dir.path()).unwrap().count(),
            "{}",
            row.id
        );
        Self { _dir: dir, repo }
    }

    fn session(&self) -> NavigationSession {
        NavigationSession {
            repo: self.repo.clone(),
            index: Arc::new(NavigationIndex::build(&self.repo)),
        }
    }

    fn context(&self) -> CpgContext<'_> {
        CpgContext::build_with_scope_graph_inputs(
            &self.repo.files,
            self.repo.type_db.as_ref(),
            self.repo.scope_graph_inputs.as_ref(),
        )
    }

    fn diff(&self) -> DiffInput {
        DiffInput {
            files: self
                .repo
                .files
                .iter()
                .map(|(path, file)| DiffInfo {
                    file_path: path.clone(),
                    modify_type: ModifyType::Modified,
                    diff_lines: (1..=file.source.lines().count()).collect(),
                })
                .collect(),
        }
    }
}

impl Row {
    fn config(&self, return_flow: bool) -> TaintConfig {
        TaintConfig {
            sources: vec![(self.src.file.clone(), self.src.line)],
            taint_from_diff: false,
            extra_sinks: vec![],
            return_flow,
        }
    }

    fn source_seed(&self) -> SeedSpec {
        SeedSpec::Loc {
            file: self.src.file.clone(),
            line: self.src.line,
        }
    }
}

fn intersects(node: Bytes, operand: Bytes) -> bool {
    if node[0] == node[1] {
        return node[0] == operand[0];
    }
    node[0] < operand[1] && operand[0] < node[1]
}

fn symbol_bytes(symbol: &SymbolRef) -> Bytes {
    match symbol {
        SymbolRef::Variable {
            start_byte,
            end_byte,
            ..
        } => [*start_byte, *end_byte],
        _ => panic!("expected variable: {symbol:?}"),
    }
}

fn sink_nodes(row: &Row, session: &NavigationSession) -> Vec<Bytes> {
    let cpg = session.index.cpg();
    let mut nodes = cpg
        .nodes_at(&row.sink.file, row.sink.line)
        .iter()
        .filter_map(|&id| {
            let CpgNode::Variable {
                start_byte,
                end_byte,
                ..
            } = cpg.node(id)
            else {
                return None;
            };
            let span = [*start_byte, *end_byte];
            intersects(span, row.sink.operand).then_some(span)
        })
        .collect::<Vec<_>>();
    nodes.sort_unstable();
    nodes
}

fn marked_lines(file: &ParsedFile, marker: &str) -> Vec<usize> {
    let mut comments = Vec::new();
    let mut stack = vec![file.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "comment" {
            for token in file.source[node.byte_range()]
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '@' || c == '_'))
            {
                if token == marker {
                    comments.push(node.start_position().row + 1);
                }
            }
        }
        stack.extend(node.named_children(&mut node.walk()));
    }
    comments
}

fn occurrence(file: &ParsedFile, line: usize, bytes: Bytes) -> tree_sitter::Node<'_> {
    let [start, end] = bytes;
    let node = syntax_node(file, bytes);
    assert_eq!(file.line_for_byte(start), line, "{}:{bytes:?}", file.path);
    assert_eq!(file.line_for_byte(end - 1), line, "{}:{bytes:?}", file.path);
    node
}

fn syntax_node(file: &ParsedFile, bytes: Bytes) -> tree_sitter::Node<'_> {
    let [start, end] = bytes;
    assert!(
        start < end && file.source.get(start..end).is_some(),
        "{}:{bytes:?}",
        file.path
    );
    let node = file
        .tree
        .root_node()
        .named_descendant_for_byte_range(start, end)
        .unwrap();
    assert_eq!(node.byte_range(), start..end, "{}:{bytes:?}", file.path);
    assert_ne!(node.kind(), "comment", "{}:{bytes:?}", file.path);
    node
}

fn review(row: &Row, fixture: &Fixture, return_flow: bool) -> Finding {
    review_with(row, fixture, &row.config(return_flow))
}

fn review_with(row: &Row, fixture: &Fixture, config: &TaintConfig) -> Finding {
    let result = slice(&fixture.context(), &fixture.diff(), config).unwrap();
    assert_eq!(result.findings.len(), result.evidence.len(), "{}", row.id);
    let mut findings = result
        .findings
        .iter()
        .zip(&result.evidence)
        .filter(|(finding, _)| {
            finding.category.as_deref() == Some("taint_sink")
                && finding.file == row.sink.file
                && finding.line == row.sink.line
        });
    let Some((_, evidence)) = findings.next() else {
        return Finding::None;
    };
    assert!(
        findings.next().is_none(),
        "{}: multiple findings on the marked sink line",
        row.id
    );
    let evidence = evidence
        .as_ref()
        .expect("taint finding must have retained evidence");
    match classify_with_evidence("taint", ParseQuality::Clean, evidence).0 {
        FindingConfidence::Exact => Finding::Exact,
        FindingConfidence::NameOnly => Finding::NameOnly,
        FindingConfidence::Unlabeled => Finding::Unlabeled,
        grade => panic!("unexpected finding confidence {grade:?}"),
    }
}

fn walk(row: &Row, fixture: &Fixture) -> Walk {
    let session = fixture.session();
    let sink_seed = SeedSpec::Loc {
        file: row.sink.file.clone(),
        line: row.sink.line,
    };
    let result = taint_reaches(
        &session,
        &[row.source_seed()],
        Some(std::slice::from_ref(&sink_seed)),
    );
    let evidence = match result {
        Ok(evidence) => evidence,
        Err(_) => return Walk::no_witness(Verdict::SeedError),
    };
    let summary = evidence.reasoning.unwrap();
    assert_eq!(summary.sinks_omitted, 0, "{}", row.id);
    reason_checks::check_deferred_refusals(row, &session, &summary);
    let mut pairs = summary
        .per_sink
        .iter()
        .filter(|sink| intersects(symbol_bytes(&sink.sink), row.sink.operand))
        .flat_map(|sink| {
            assert_eq!(sink.sources_omitted, 0, "{}", row.id);
            sink.sources.iter().map(move |source| (sink, source))
        })
        .filter(|(_, source)| symbol_bytes(&source.source) == row.src.bytes)
        .collect::<Vec<_>>();
    if row.group == "option_b" {
        assert!(
            !pairs.is_empty(),
            "{}: missing marked source-operand pair",
            row.id
        );
    }
    let reaches = Reachability::aggregate(pairs.iter().map(|(_, source)| source.reachability))
        .unwrap_or(Reachability::NotReached);
    if row.walk_comparable {
        let (_, trace) =
            slice_with_trace(&fixture.context(), &fixture.diff(), &row.config(true)).unwrap();
        let review_reached = trace
            .roots
            .iter()
            .flat_map(|root| &root.reached)
            .any(|target| {
                let loc = &target.location;
                loc.file == row.sink.file
                    && loc.line == row.sink.line
                    && intersects([loc.start_byte, loc.end_byte], row.sink.operand)
            });
        assert_eq!(
            review_reached,
            matches!(reaches, Reachability::Reached | Reachability::Sanitized),
            "{}: review's pre-cleansing trace and the navigation walk disagree",
            row.id
        );
    }
    // In a tie, choose the first terminal by byte span.
    pairs.sort_by_key(|(sink, _)| symbol_bytes(&sink.sink));
    if let Some((_, source)) = pairs
        .iter()
        .find(|(_, source)| source.reachability == reaches)
    {
        reason_checks::check_walk_reason(row, source);
    }
    for (sink, source) in pairs {
        if source.reachability == reaches {
            if let Some(endpoint) = source.graph_node {
                let graph = evidence.graph.as_ref().expect("witness graph");
                let terminal = symbol_bytes(graph.nodes[endpoint].symbol.as_ref().unwrap());
                assert_eq!(terminal, symbol_bytes(&sink.sink), "{}", row.id);
                assert!(
                    graph
                        .nodes
                        .iter()
                        .any(|node| node.symbol.as_ref() == Some(&source.source)),
                    "{}: witness graph omitted its root",
                    row.id
                );
                return Walk {
                    reaches: reaches.into(),
                    source: Some(symbol_bytes(&source.source)),
                    terminal: Some(terminal),
                    descent_depth: Some(source.descent_depth),
                };
            }
        }
    }
    Walk::no_witness(reaches.into())
}

fn current_cell(row: &Row, group_enabled: bool) -> Option<&rows::Current> {
    row.current.as_ref().filter(|current| {
        !group_enabled
            || current
                .until
                .iter()
                .any(|slice| !SHIPPED_SLICES.contains(&slice.as_str()))
    })
}

fn active_cell(row: &Row, group_enabled: bool) -> Option<(&Expectations, Option<&rows::Reasons>)> {
    match current_cell(row, group_enabled) {
        Some(current) => Some((&current.expect, current.reasons.as_ref())),
        _ if group_enabled => Some((
            row.expect
                .as_ref()
                .expect("enabled row has no expectations"),
            row.reasons.as_ref(),
        )),
        _ => None,
    }
}

fn check_cells<T: std::fmt::Debug + PartialEq>(
    level: Level,
    expected: impl Fn(&Expectations) -> &T,
    actual: impl Fn(&Row, &Fixture) -> T,
) {
    let mut failures = Vec::new();
    let rows = rows();
    let enabled = enabled_groups();
    let deferred: BTreeSet<_> = rows
        .iter()
        .filter(|row| row.current.is_some() && !enabled.contains(&row.group))
        .map(|row| &row.group)
        .collect();
    for group in enabled.iter().chain(deferred) {
        let group_enabled = enabled.contains(group);
        let mut executed = 0;
        let mut pending = 0;
        for row in rows.iter().filter(|row| &row.group == group) {
            let Some((expect, _)) = active_cell(row, group_enabled) else {
                continue;
            };
            if let Some(issue) = row.pending_issue(level) {
                eprintln!("{}: pending {level:?} ({issue})", row.id);
                pending += 1;
                continue;
            }
            let got = actual(row, &Fixture::load(row));
            executed += 1;
            let want = expected(expect);
            if &got != want {
                failures.push(format!("{}: expected {want:?}; got {got:?}", row.id));
            }
        }
        if executed == 0 {
            if pending == 0 {
                failures.push(format!("{group}: no {level:?} cells executed"));
            } else {
                eprintln!("{group}: all {pending} {level:?} cells pending");
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn validate_row(row: &Row) {
    let fixture = Fixture::load(row);
    let session = fixture.session();
    validate_labels(row);
    for (marker, name, line) in [
        ("@SRC", &row.src.file, row.src.line),
        ("@SINK", &row.sink.file, row.sink.line),
    ] {
        for (file, parsed) in &fixture.repo.files {
            assert_eq!(parsed.parse_error_count, 0, "{}: {file}", row.id);
            let marks = if file == name { vec![line] } else { vec![] };
            assert_eq!(
                marked_lines(parsed, marker),
                marks,
                "{}: {file} {marker}",
                row.id
            );
        }
    }
    let source = &fixture.repo.files[&row.src.file];
    assert_eq!(
        format!("{:?}", source.language).to_ascii_lowercase(),
        row.lang,
        "{}: source language",
        row.id
    );
    occurrence(source, row.src.line, row.src.bytes);
    occurrence(
        &fixture.repo.files[&row.sink.file],
        row.sink.line,
        row.sink.operand,
    );
    if let Some(site) = &row.site {
        let node = syntax_node(source, site.bytes);
        match site.slot {
            Slot::Argument(slot) => {
                let count = if node.kind() == "jsx_self_closing_element" {
                    node.named_children(&mut node.walk())
                        .filter(|child| child.kind() == "jsx_attribute")
                        .count()
                } else {
                    node.child_by_field_name("arguments")
                        .expect("call arguments")
                        .named_child_count()
                };
                assert!(slot < count, "{}", row.id);
            }
            Slot::Return(_) => assert_eq!(node.kind(), "return_statement", "{}", row.id),
        }
    }
    assert_eq!(
        sink_nodes(row, &session),
        row.sink.nodes,
        "{}: operand nodes",
        row.id
    );
    let expectations = row
        .expect
        .iter()
        .chain(row.current.iter().map(|current| &current.expect));
    for entry in expectations.flat_map(|expect| &expect.catalog) {
        let file = &fixture.repo.files[&entry.file];
        assert!(
            (1..=file.source.lines().count()).contains(&entry.line),
            "{}",
            row.id
        );
        assert!(
            !entry.rule.is_empty() && entry.verdict != Verdict::SeedError,
            "{}",
            row.id
        );
        if let Some([start, end]) = entry.operand {
            assert!(
                start < end && file.source.get(start..end).is_some(),
                "{}",
                row.id
            );
        }
    }
    let sources = resolve(&session, &[row.source_seed()], SeedRole::Source);
    let roots: Vec<_> = sources
        .iter()
        .flat_map(|set| &set.seeds)
        .flat_map(|seed| &seed.locations)
        .map(|loc| [loc.start_byte, loc.end_byte])
        .collect();
    if row
        .expect
        .as_ref()
        .is_none_or(|expect| expect.walk.reaches != Verdict::SeedError)
    {
        assert!(
            roots.contains(&row.src.bytes),
            "{}: source occurrence was not minted",
            row.id
        );
    }
    if row.walk_comparable || row.group == "control" {
        let comparable = roots.as_slice() == [row.src.bytes] && {
            let (_, trace) =
                slice_with_trace(&fixture.context(), &fixture.diff(), &row.config(true)).unwrap();
            matches!(trace.roots.as_slice(), [root]
                if root.origin == TaintRootOrigin::Explicit
                    && root.location.file == row.src.file
                    && root.location.line == row.src.line
                    && [root.location.start_byte, root.location.end_byte] == row.src.bytes)
        };
        assert_eq!(
            row.walk_comparable, comparable,
            "{}: walk comparison requires the same single explicit root in both clients",
            row.id
        );
    }
}

#[test]
fn parity_rows_are_well_formed() {
    let enabled = enabled_groups();
    assert!(!enabled.is_empty(), "at least one group must be enabled");
    for group in &enabled {
        assert!(
            GROUPS.contains(&group.as_str()),
            "unknown enabled group {group}"
        );
        assert_ne!(group, "tbd", "tbd rows cannot be enabled");
    }
    let mut ids = BTreeSet::new();
    let mut invalid = Vec::new();
    for row in rows() {
        let new_id = ids.insert(row.id.clone());
        if !new_id || std::panic::catch_unwind(|| validate_row(&row)).is_err() {
            invalid.push(row.id);
        }
    }
    assert!(!ids.is_empty());
    assert!(invalid.is_empty(), "invalid or duplicate rows: {invalid:?}");
}

#[test]
fn parity_default_review() {
    check_cells(
        Level::Review,
        |expect| &expect.review,
        |row, fixture| review(row, fixture, false),
    );
}

#[test]
fn parity_trace_review() {
    check_cells(
        Level::TraceReview,
        |expect| &expect.trace_review,
        |row, fixture| review(row, fixture, true),
    );
}

#[test]
fn parity_taint_reaches() {
    check_cells(Level::Walk, |expect| &expect.walk, walk);
}
