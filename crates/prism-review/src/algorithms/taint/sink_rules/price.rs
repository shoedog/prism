//! Census of the syntax that would survive certification of production flat hits.

use super::{
    super::flat::SINK_PATTERNS,
    certificate::{
        candidates, certify_operand, certify_operation, operation_head, target_for_hit, Reason,
        Refusals, Table,
    },
};
use crate::algorithms::taint_sinks::SinkClassifier;
use prism::{ast::ParsedFile, languages::Language};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, BufWriter, Write},
    iter::successors,
    path::Path,
    time::{Duration, Instant},
};
use tree_sitter::Node;

#[derive(Default)]
struct Counts {
    hits: usize,
    certified: usize,
    failed: [usize; 3],
    refusals: Refusals,
    no_operation: usize,
    parse_error_files: usize,
    // Certified hits whose input shape the owner has not yet ruled on (PENDING rows).
    shapes: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Copy)]
enum Hit {
    Certified,
    NoOperation,
    Refused(Reason),
}

impl Counts {
    fn record(&mut self, hit: Hit) {
        self.hits += 1;
        match hit {
            Hit::Certified => self.certified += 1,
            Hit::NoOperation => {
                self.no_operation += 1;
                self.failed[0] += 1;
            }
            Hit::Refused(reason) => {
                self.refusals.record(reason);
                let index = match reason.table() {
                    Table::Operation => 0,
                    Table::Callee => 1,
                    Table::Operand => 2,
                };
                self.failed[index] += 1;
            }
        }
    }
}

// The operand table checks position/role, slot effects, slot number, exact
// node equality and value kind in that order. Keep the first tie.
fn operand_progress(reason: Reason) -> usize {
    match reason {
        Reason::OperandLabel | Reason::OperandComment => 0,
        Reason::RoleNotBound | Reason::ParallelAssignment | Reason::AttributeOverwritten => 1,
        Reason::OperandSpread | Reason::OperandAttribute | Reason::OperandMacroBlock => 2,
        Reason::SlotMismatch => 3,
        Reason::OperandNotDirectChild => 4,
        Reason::OperandKindUnlisted => 5,
        _ => unreachable!("operand certification returned {}", reason.as_str()),
    }
}

fn operation_of<'tree>(parsed: &ParsedFile, id: Node<'tree>) -> Option<Node<'tree>> {
    successors(id.parent(), Node::parent).find(|&op| {
        operation_head(parsed.language, op)
            .is_some_and(|head| successors(Some(id), Node::parent).any(|node| node == head))
    })
}

fn blank(parsed: &ParsedFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "identifier" | "field_identifier" | "blank_identifier"
    ) && parsed.node_text(&node) == "_"
}

// A `_` read, or a `_` selector segment, inside the operation (not a declaring position).
fn go_blank_read(parsed: &ParsedFile, operation: Node<'_>) -> bool {
    let mut cursor = operation.walk();
    let mut stack = vec![operation];
    while let Some(node) = stack.pop() {
        if blank(parsed, node) {
            let declaring = successors(Some(node), Node::parent)
                .take_while(|&up| up != operation.parent().unwrap_or(operation))
                .any(|up| {
                    up.parent().is_some_and(|parent| {
                        matches!(
                            parent.kind(),
                            "short_var_declaration" | "assignment_statement" | "range_clause"
                        ) && parent.child_by_field_name("left") == Some(up)
                    })
                });
            if !declaring {
                return true;
            }
        }
        stack.extend(node.children(&mut cursor));
    }
    false
}

fn python_invalid_argument_list(parsed: &ParsedFile, list: Node<'_>) -> bool {
    let mut cursor = list.walk();
    let (mut keyword, mut double_star) = (false, false);
    let mut names = Vec::new();
    for arg in list.named_children(&mut cursor) {
        match arg.kind() {
            "keyword_argument" => {
                keyword = true;
                let name = arg
                    .child_by_field_name("name")
                    .map(|name| parsed.node_text(&name).to_owned());
                if names.contains(&name) {
                    return true;
                }
                names.push(name);
            }
            "dictionary_splat" => double_star = true,
            "list_splat" if double_star => return true,
            "comment" | "list_splat" => {}
            _ if keyword || double_star => return true,
            _ => {}
        }
    }
    false
}

// Shapes of a certified hit that are outside the input contract (known limits: invalid programs)
// or counted for disclosure (python_assert is certified: a real call in a default build).
fn pending_shapes(parsed: &ParsedFile, id: Node<'_>) -> Vec<&'static str> {
    let mut shapes = Vec::new();
    if parsed.language == Language::Go {
        if operation_of(parsed, id).is_some_and(|operation| go_blank_read(parsed, operation)) {
            shapes.push("go_blank_read_or_selector");
        }
    }
    for node in successors(id.parent(), Node::parent) {
        match (parsed.language, node.kind()) {
            (Language::Python, "assert_statement") => shapes.push("python_assert"),
            (Language::Python, "argument_list") if python_invalid_argument_list(parsed, node) => {
                shapes.push("python_invalid_outer_arguments");
            }
            (Language::TypeScript | Language::Tsx, "enum_declaration")
                if node
                    .children(&mut node.walk())
                    .any(|child| child.kind() == "const") =>
            {
                shapes.push("ts_const_enum");
            }
            _ => {}
        }
    }
    shapes
}

fn certify_hit(parsed: &ParsedFile, id: Node<'_>) -> Hit {
    let operation = operation_of(parsed, id);
    let Some(operation) = operation else {
        return Hit::NoOperation;
    };

    let shape = match certify_operation(parsed, operation) {
        Ok(shape) => shape,
        Err(reason) => return Hit::Refused(reason),
    };
    let target = match target_for_hit(parsed, operation, &shape, id) {
        Ok(target) => target,
        Err(reason) => return Hit::Refused(reason),
    };
    let mut furthest: Option<Reason> = None;
    for (operand, role) in candidates(parsed, &shape) {
        match certify_operand(parsed, operation, &shape, target, operand, role) {
            Ok(()) => return Hit::Certified,
            Err(reason)
                if furthest
                    .is_none_or(|prior| operand_progress(reason) > operand_progress(prior)) =>
            {
                furthest = Some(reason);
            }
            Err(_) => {}
        }
    }
    Hit::Refused(furthest.unwrap_or(Reason::RoleNotBound))
}

fn visit_file(
    path: &Path,
    languages: &mut BTreeMap<Language, Counts>,
    corpus: &mut Counts,
) -> io::Result<()> {
    let Some(language) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(Language::from_extension)
    else {
        return Ok(());
    };
    let source = match String::from_utf8(fs::read(path)?) {
        Ok(source) => source,
        Err(_) => return Ok(()),
    };
    let file = path.to_string_lossy();
    let parsed = ParsedFile::parse(&file, &source, language)
        .unwrap_or_else(|err| panic!("parse {file}: {err:#}"));
    let root = parsed.tree.root_node();
    let totals = languages.get_mut(&language).expect("known language");
    if root.has_error() {
        totals.parse_error_files += 1;
        corpus.parse_error_files += 1;
    }

    let classifier = SinkClassifier::default();
    let mut cursor = root.walk();
    loop {
        let id = cursor.node();
        if language.is_identifier_node(id.kind())
            && classifier
                .classify(&file, &parsed, id, parsed.node_text(&id), SINK_PATTERNS)
                .is_some()
        {
            let hit = certify_hit(&parsed, id);
            if let (Hit::Refused(reason), Some(want)) = (hit, env::var_os("PRICE_LIST")) {
                if want == "all" || want.to_string_lossy() == format!("{language:?}") {
                    eprintln!(
                        "price-miss\t{language:?}\t{}:{}\t{}\t{}",
                        file,
                        id.start_position().row + 1,
                        reason.as_str(),
                        source
                            .lines()
                            .nth(id.start_position().row)
                            .unwrap_or("")
                            .trim()
                    );
                }
            }
            totals.record(hit);
            corpus.record(hit);
            if matches!(hit, Hit::Certified) {
                for shape in pending_shapes(&parsed, id) {
                    *totals.shapes.entry(shape).or_default() += 1;
                    *corpus.shapes.entry(shape).or_default() += 1;
                }
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return Ok(());
            }
        }
    }
}

fn walk(root: &Path, visit: &mut impl FnMut(&Path) -> io::Result<()>) -> io::Result<()> {
    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_unstable_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type()?;
        if kind.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with('.') && !matches!(&*name, "node_modules" | "vendor" | "target") {
                walk(&entry.path(), visit)?;
            }
        } else if kind.is_file() {
            visit(&entry.path())?;
        }
    }
    Ok(())
}

#[test]
#[ignore]
fn price_corpora() -> io::Result<()> {
    let paths = env::var_os("PRICE_ROOTS").expect("PRICE_ROOTS is required");
    let output = env::var_os("PRICE_OUT").expect("PRICE_OUT is required");
    run_census(env::split_paths(&paths).collect(), Path::new(&output))
}

fn run_census(mut roots: Vec<std::path::PathBuf>, output: &Path) -> io::Result<()> {
    roots.sort();
    let mut languages: BTreeMap<_, _> = Language::all()
        .into_iter()
        .map(|language| (language, Counts::default()))
        .collect();
    let mut corpora = BTreeMap::<String, Counts>::new();
    let mut files = 0;
    for root in roots {
        assert!(
            fs::symlink_metadata(&root)?.is_dir(),
            "expected directory: {}",
            root.display()
        );
        let basename = root
            .file_name()
            .expect("corpus root must have a basename")
            .to_string_lossy();
        assert!(
            !basename.contains(['\t', '\n', '\r']),
            "invalid TSV scope: {}",
            root.display()
        );
        let corpus = corpora.entry(format!("corpus:{basename}")).or_default();
        eprintln!("price: root {}", root.display());
        walk(&root, &mut |path| {
            let start = Instant::now();
            let result = visit_file(path, &mut languages, corpus);
            let elapsed = start.elapsed();
            if elapsed > Duration::from_secs(20) {
                eprintln!("price: slow file {} ({elapsed:.1?})", path.display());
            }
            result?;
            files += 1;
            if files % 1000 == 0 {
                let hits: usize = languages.values().map(|counts| counts.hits).sum();
                eprintln!("price: {files} files, {hits} hits ({})", path.display());
            }
            Ok(())
        })?;
    }

    let scopes = languages
        .iter()
        .map(|(lang, counts)| (format!("{lang:?}"), counts))
        .chain(
            corpora
                .iter()
                .map(|(scope, counts)| (scope.clone(), counts)),
        )
        .collect::<BTreeMap<_, _>>();
    let shape_scopes = scopes.clone();
    let mut out = BufWriter::new(fs::File::create(output)?);
    writeln!(out, "scope\thits\tcertified\tfail_a\tfail_b\tfail_c")?;
    for (scope, counts) in &scopes {
        let [a, b, c] = counts.failed;
        writeln!(
            out,
            "{scope}\t{}\t{}\t{a}\t{b}\t{c}",
            counts.hits, counts.certified
        )?;
        let share = if counts.hits == 0 {
            0.0
        } else {
            100.0 * counts.certified as f64 / counts.hits as f64
        };
        println!(
            "{scope}\t{}/{} ({share:.2}%)\tparse_error_files={}",
            counts.certified, counts.hits, counts.parse_error_files
        );
    }
    writeln!(out, "\nscope\treason\tcount")?;
    for (scope, counts) in scopes {
        let no_operation =
            (counts.no_operation != 0).then_some(("no_operation", counts.no_operation));
        let reasons = counts
            .refusals
            .iter()
            .chain(no_operation)
            .collect::<BTreeMap<_, _>>();
        for (reason, count) in reasons {
            writeln!(out, "{scope}\t{reason}\t{count}")?;
        }
    }
    writeln!(out, "\nscope\tpending_shape\tcertified_hits")?;
    for (scope, counts) in &shape_scopes {
        for (shape, count) in &counts.shapes {
            writeln!(out, "{scope}\t{shape}\t{count}")?;
        }
    }
    out.flush()
}

#[cfg(test)]
mod census_tests {
    use super::*;

    fn census(files: &[(&str, &str)]) -> String {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("corpus");
        fs::create_dir(&root).unwrap();
        for (name, text) in files {
            fs::write(root.join(name), text).unwrap();
        }
        let out = dir.path().join("out.tsv");
        run_census(vec![root], &out).unwrap();
        fs::read_to_string(out).unwrap()
    }

    fn line<'a>(report: &'a str, prefix: &str) -> &'a str {
        report
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("no {prefix} in {report}"))
    }

    #[test]
    fn counts_certified_and_refused_hits_and_pending_shapes() {
        let report = census(&[
            ("a.py", "def g(x):\n    eval(x)\n    assert eval(x)\n    outer(k=0, eval(x))\n    outer(**d, *a, eval(x))\n    outer(k=0, k=1, eval(x))\n    outer(eval(x), k=0)\n"),
            ("b.go", "package p\nfunc g(x int) {\n\texec.Command(x)\n\texec.Command(_)\n\tb._.Command(x)\n\t_ = exec.Command(x)\n}\n"),
            ("c.ts", "const enum E { A = eval(1) }\nenum F { A = eval(1) }\n"),
            ("d.js", "function g(x) {\n  eval(x);\n  eval(...x);\n}\n"),
            ("note.txt", "eval(x)"),
            ("bin.py", ""),
        ]);
        assert!(
            line(&report, "Python\t").starts_with("Python\t6\t6\t"),
            "{report}"
        );
        assert!(
            line(&report, "JavaScript\t").starts_with("JavaScript\t2\t1\t"),
            "{report}"
        );
        for shape in [
            "python_assert",
            "python_invalid_outer_arguments",
            "go_blank_read_or_selector",
            "ts_const_enum",
        ] {
            assert!(report.contains(shape), "{shape} missing in {report}");
        }
        // The valid control shapes are not counted.
        assert!(
            line(&report, "Python\tpython_assert").ends_with("\t1"),
            "{report}"
        );
        assert!(line(&report, "Python\tpython_invalid_outer_arguments").ends_with("\t3"));
        assert!(line(&report, "TypeScript\tts_const_enum").ends_with("\t1"));
        assert!(
            line(&report, "Go\tgo_blank_read_or_selector").ends_with("\t2"),
            "{report}"
        );
    }
}
