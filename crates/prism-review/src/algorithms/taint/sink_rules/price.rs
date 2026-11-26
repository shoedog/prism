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

fn certify_hit(parsed: &ParsedFile, id: Node<'_>) -> Hit {
    let operation = successors(id.parent(), Node::parent).find(|&op| {
        operation_head(parsed.language, op)
            .is_some_and(|head| successors(Some(id), Node::parent).any(|node| node == head))
    });
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
            totals.record(hit);
            corpus.record(hit);
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
    let mut roots: Vec<_> = env::split_paths(&paths).collect();
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
    out.flush()
}
