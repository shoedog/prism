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
use unicode_normalization::UnicodeNormalization;

#[derive(Default)]
struct Counts {
    hits: usize,
    certified: usize,
    failed: [usize; 3],
    refusals: Refusals,
    no_operation: usize,
    parse_error_files: usize,
    // Certified hits whose shape is a known limit of the input contract, plus whole-file
    // counts of statically nullish optional chains (`js_nullish_optional_chain_any`).
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
            let declaring = node.parent().is_some_and(|parent| {
                matches!(
                    parent.kind(),
                    "parameter_declaration"
                        | "variadic_parameter_declaration"
                        | "var_spec"
                        | "const_spec"
                        | "field_declaration"
                ) || [Some(node), Some(parent)].into_iter().flatten().any(|up| {
                    up.parent().is_some_and(|outer| {
                        matches!(
                            outer.kind(),
                            "short_var_declaration"
                                | "assignment_statement"
                                | "range_clause"
                                | "receive_statement"
                        ) && outer.child_by_field_name("left") == Some(up)
                    })
                }) && (parent.kind() == "expression_list"
                    || parent.child_by_field_name("left") == Some(node))
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
                    .map(|name| parsed.node_text(&name).nfkc().collect::<String>());
                if names.contains(&name) {
                    return true;
                }
                names.push(name);
            }
            "dictionary_splat" => double_star = true,
            "list_splat" if double_star => return true,
            "comment" | "list_splat" => {}
            "parenthesized_expression"
                if arg
                    .named_child(0)
                    .is_some_and(|inner| inner.kind() == "list_splat") =>
            {
                return true
            }
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
    if matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) && under_nullish_chain(parsed, id)
    {
        shapes.push("js_certified_under_nullish_optional_chain");
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
    shapes.sort_unstable();
    shapes.dedup();
    shapes
}

// A value that is statically `null` or `undefined`: the literals, `void <literal>`, and
// parenthesized, non-null or `as` wrappers of them.
fn nullish_value(parsed: &ParsedFile, node: Node<'_>) -> bool {
    match node.kind() {
        "null" | "undefined" => true,
        "identifier" => parsed.node_text(&node) == "undefined",
        "parenthesized_expression" | "non_null_expression" | "as_expression" => node
            .named_child(0)
            .is_some_and(|inner| nullish_value(parsed, inner)),
        "unary_expression" => {
            parsed.node_text(&node).trim_start().starts_with("void")
                && node.child_by_field_name("argument").is_some_and(|arg| {
                    matches!(
                        arg.kind(),
                        "number" | "string" | "true" | "false" | "null" | "undefined"
                    )
                })
        }
        _ => false,
    }
}

// An optional chain link (`?.`) whose base is statically nullish; returns the base.
fn nullish_chain_base<'t>(parsed: &ParsedFile, node: Node<'t>) -> Option<Node<'t>> {
    let field = match node.kind() {
        "member_expression" | "subscript_expression" => "object",
        "call_expression" => "function",
        _ => return None,
    };
    let base = node.child_by_field_name(field)?;
    let optional = node.child_by_field_name("optional_chain").is_some()
        || node.children(&mut node.walk()).any(|c| c.kind() == "?.");
    (optional && nullish_value(parsed, base)).then_some(base)
}

// A certified hit inside a chain that a nullish optional link short-circuits (the callee,
// arguments and computed keys the chain skips), but not inside the link's evaluated base.
// The skipped region is the whole chain: the link and every call, member and subscript that
// continues it; a parenthesis ends the chain.
fn under_nullish_chain(parsed: &ParsedFile, id: Node<'_>) -> bool {
    let inside = |outer: Node<'_>| {
        outer.start_byte() <= id.start_byte() && id.end_byte() <= outer.end_byte()
    };
    for expr in successors(id.parent(), Node::parent) {
        let mut node = expr;
        loop {
            if nullish_chain_base(parsed, node).is_some_and(|base| !inside(base)) {
                return true;
            }
            let field = match node.kind() {
                "member_expression" | "subscript_expression" => "object",
                "call_expression" => "function",
                _ => break,
            };
            match node.child_by_field_name(field) {
                Some(next) => node = next,
                None => break,
            }
        }
    }
    false
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
        if matches!(
            language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) && nullish_chain_base(&parsed, id).is_some()
        {
            *totals
                .shapes
                .entry("js_nullish_optional_chain_any")
                .or_default() += 1;
            *corpus
                .shapes
                .entry("js_nullish_optional_chain_any")
                .or_default() += 1;
        }
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
        census_with(|root| {
            for (name, text) in files {
                fs::write(root.join(name), text).unwrap();
            }
        })
    }

    fn census_with(setup: impl FnOnce(&Path)) -> String {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("corpus");
        fs::create_dir(&root).unwrap();
        setup(&root);
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

    // The count in the `reason` row of a scope, 0 when the row is absent.
    fn reason_count(report: &str, scope: &str, reason: &str) -> usize {
        let prefix = format!("{scope}\t{reason}\t");
        report
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .map_or(0, |count| count.parse().unwrap())
    }

    #[test]
    fn counts_every_refusal_table_and_hit_outcome() {
        let report = census_with(|root| {
            let python = [
                "def g(x, a, b):",
                // Certified through the keyword claim after the positional claim fails.
                "    eval(k=x)",
                // Refused: the positional claim fails first, the keyword value is not a plain value.
                "    eval(k=f())",
                "    eval(*a, *b)",
                "    eval(x for x in a)",
                // Certified: the argument is a plain value (the receiver is also a candidate).
                "    x.eval(1)",
                "",
            ]
            .join("\n");
            fs::write(root.join("a.py"), python).unwrap();
            // A file with a syntax error is still read.
            fs::write(root.join("bad.py"), "eval(x\n").unwrap();
            // Not UTF-8: skipped, not an error.
            fs::write(root.join("latin.py"), b"\xff\xfe eval(x)\n").unwrap();
            // A parallel assignment target is refused, not certified.
            fs::write(root.join("s.lua"), "eval, b = 1, 2\n").unwrap();
            fs::write(
                root.join("j.js"),
                "function g(x, el) {\n  <X dangerouslySetInnerHTML={{__html: x}} />;\n  <X dangerouslySetInnerHTML={f(x)} />;\n  el.innerHTML = x;\n}\n",
            )
            .unwrap();
            // The callee is not the hit; a bare read is no operation.
            fs::write(
                root.join("a.c"),
                "void g(A a, int x) {\n  a.eval.b(1);\n  y = eval;\n  eval(x);\n}\n",
            )
            .unwrap();
            // A blank target that only declares inside the operation is not a blank read.
            fs::write(
                root.join("b.go"),
                "package p\nfunc g(x int) {\n\texec.Command(1, func() { _ = x })\n}\n",
            )
            .unwrap();
            // Directories the walk skips, and a link it does not follow.
            for skipped in ["node_modules", ".hidden", "vendor", "target"] {
                fs::create_dir(root.join(skipped)).unwrap();
                fs::write(root.join(skipped).join("n.py"), "eval(x)\n").unwrap();
            }
            fs::create_dir(root.join("sub")).unwrap();
            fs::write(root.join("sub").join("s.py"), "def g(x):\n    eval(x)\n").unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(root.join("a.py"), root.join("link.py")).unwrap();
        });

        // Five hits in a.py and one in sub/s.py; the skipped directories and the link add none.
        let python = line(&report, "Python\t");
        let fields: Vec<_> = python.split('\t').collect();
        assert_eq!(&fields[1..6], ["6", "3", "1", "0", "2"], "{report}");
        for (scope, reason, count) in [
            // Operation table.
            ("Python", "op_arguments_unlisted", 1),
            // Operand table: the furthest failing claim is reported.
            ("Python", "operand_spread", 1),
            ("Python", "operand_kind_unlisted", 1),
            ("Lua", "parallel_assignment", 1),
            ("JavaScript", "operand_kind_unlisted", 1),
            ("C", "no_operation", 1),
        ] {
            assert_eq!(
                reason_count(&report, scope, reason),
                count,
                "{scope} {reason}: {report}"
            );
        }
        // Callee table.
        assert!(
            reason_count(&report, "C", "callee_target_unlisted") >= 1,
            "{report}"
        );
        assert!(
            line(&report, "JavaScript\t").starts_with("JavaScript\t3\t2\t0\t0\t1"),
            "{report}"
        );
        // The declaring blank is no pending shape; the call is certified.
        assert!(line(&report, "Go\t").starts_with("Go\t1\t1\t"), "{report}");
        assert!(!report.contains("go_blank_read_or_selector"), "{report}");
    }

    #[test]
    fn valid_outer_argument_lists_are_not_pending_shapes() {
        let report = census(&[(
            "a.py",
            "def g(x, a):\n    outer(*a, eval(x))\n    outer(  # note\n        eval(x),\n        2)\n    outer(eval(x), k=0)\n",
        )]);
        assert!(
            line(&report, "Python\t").starts_with("Python\t3\t3\t"),
            "{report}"
        );
        assert!(
            !report.contains("python_invalid_outer_arguments"),
            "{report}"
        );
        // Control: the same calls with a positional argument after a keyword are pending.
        let report = census(&[("a.py", "def g(x):\n    outer(k=0, eval(x))\n")]);
        assert!(
            line(&report, "Python\tpython_invalid_outer_arguments").ends_with("\t1"),
            "{report}"
        );
    }

    fn run_child(root: &Path, out: &Path, list: &str) -> String {
        let name = format!(
            "{}::price_corpora",
            module_path!()
                .trim_start_matches("prism_review::")
                .trim_end_matches("::census_tests")
        );
        let output = std::process::Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", "--nocapture", &name])
            .env("PRICE_ROOTS", root)
            .env("PRICE_OUT", out)
            .env("PRICE_LIST", list)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    #[test]
    fn corpus_entry_point_reports_progress_and_lists_misses() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("corpus");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.py"), "def g(x):\n    eval(x)\n    eval(*x)\n").unwrap();
        fs::write(root.join("s.lua"), "eval, b = 1, 2\n").unwrap();
        for i in 0..1000 {
            fs::write(root.join(format!("n{i}.txt")), "").unwrap();
        }
        let out = dir.path().join("out.tsv");

        let python = run_child(&root, &out, "Python");
        // Files are visited in name order: the progress line comes before s.lua is read.
        assert!(python.contains("price: 1000 files, 2 hits"), "{python}");
        let misses: Vec<_> = python
            .lines()
            .filter(|line| line.starts_with("price-miss\t"))
            .collect();
        // Only the refused Python hit is listed, with its language, line, reason and source line.
        assert_eq!(misses.len(), 1, "{python}");
        assert!(misses[0].starts_with("price-miss\tPython\t"), "{python}");
        assert!(
            misses[0].contains("a.py:3\toperand_spread\teval(*x)"),
            "{python}"
        );
        let table = fs::read_to_string(&out).unwrap();
        assert!(table.contains("Python\t2\t1\t"), "{table}");

        let all = run_child(&root, &out, "all");
        assert_eq!(all.matches("price-miss\t").count(), 2, "{all}");
        assert!(all.contains("price-miss\tLua\t"), "{all}");
        assert!(all.contains("parallel_assignment"), "{all}");

        let none = run_child(&root, &out, "Rust");
        assert!(!none.contains("price-miss"), "{none}");
    }

    #[test]
    fn counts_nullish_chains_and_invalid_shapes_once_per_hit() {
        // Callee, argument and computed-key positions under a nullish optional base.
        let report = census(&[(
            "a.js",
            "function g(x, a) {\n  (void 0)?.eval(x);\n  a?.m(eval(x));\n  null?.[eval(x)];\n  undefined?.m(eval(x));\n  (null?.m)(eval(x));\n}\n",
        )]);
        assert!(
            line(&report, "JavaScript\tjs_nullish_optional_chain_any").ends_with("\t4"),
            "{report}"
        );
        // The argument of `(null?.m)(..)` is evaluated, `a?.m(..)` has no nullish base.
        assert!(
            line(
                &report,
                "JavaScript\tjs_certified_under_nullish_optional_chain"
            )
            .ends_with("\t3"),
            "{report}"
        );
        // Python: an NFKC-repeated keyword and a parenthesized unpacking are invalid lists, and
        // two enclosing invalid lists count the hit once.
        let report = census(&[(
            "a.py",
            "def g(x, a):\n    outer(k=0, \u{ff4b}=eval(x))\n    outer(eval(x), (*a))\n    outer(k=0, middle(j=0, eval(x)))\n",
        )]);
        assert!(
            line(&report, "Python\tpython_invalid_outer_arguments").ends_with("\t3"),
            "{report}"
        );
        // Go: a blank selector inside an assignment target is a read; a blank parameter is not.
        let report = census(&[(
            "a.go",
            "package p\nfunc g(x int) {\n\texec.Command(x, func(_ string) string { return \"ok\" }(\"s\"))\n\texec.Command(x, b._.f)\n}\n",
        )]);
        assert!(
            line(&report, "Go\tgo_blank_read_or_selector").ends_with("\t1"),
            "{report}"
        );
    }
}
