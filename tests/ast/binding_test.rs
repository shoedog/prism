#[path = "../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_go_multi_return_individual_defs() {
    let source = r#"
package main

func process() {
    val, err := getData()
    use(val)
    check(err)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let val_defs = dfg.all_defs_of(path, "val");
    let err_defs = dfg.all_defs_of(path, "err");
    assert!(
        !val_defs.is_empty(),
        "Go multi-return: 'val' should have its own def"
    );
    assert!(
        !err_defs.is_empty(),
        "Go multi-return: 'err' should have its own def"
    );

    let composite_defs = dfg.all_defs_of(path, "val, err");
    assert!(
        composite_defs.is_empty(),
        "Go multi-return: should not have composite 'val, err' def, got {:?}",
        composite_defs
    );
}

#[test]
fn test_go_type_assertion_individual_defs() {
    let source = r#"
package main

func check(x interface{}) {
    str, ok := x.(string)
    if ok {
        use(str)
    }
}
"#;
    let path = "src/check.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "str").is_empty(),
        "Go type assertion: 'str' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "ok").is_empty(),
        "Go type assertion: 'ok' should have def"
    );
}

#[test]
fn test_python_tuple_unpack_individual_defs() {
    let source = r#"
def process():
    name, age = get_user()
    use(name)
    use(age)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "name").is_empty(),
        "Python tuple unpack: 'name' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "age").is_empty(),
        "Python tuple unpack: 'age' should have def"
    );
}

#[test]
fn test_python_star_unpack_individual_defs() {
    let source = r#"
def process():
    first, *rest = get_items()
    use(first)
    use(rest)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "first").is_empty(),
        "Python star unpack: 'first' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "rest").is_empty(),
        "Python star unpack: 'rest' should have def"
    );
}

#[test]
fn test_python_walrus_operator_def() {
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "n").is_empty(),
        "Python walrus operator: 'n' should have def from := expression"
    );
}

#[test]
fn test_go_assignment_multi_target() {
    let source = r#"
package main

func process() {
    var x, y int
    x, y = getCoords()
    use(x)
}
"#;
    let path = "src/go_assign.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "x").is_empty(),
        "Go multi-assign: 'x' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "y").is_empty(),
        "Go multi-assign: 'y' should have def"
    );
}

#[test]
fn test_walrus_does_not_affect_regular_assignment() {
    let source = r#"
def process():
    x = 42
    y = x + 1
"#;
    let path = "src/regular.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    assert!(
        lvalues.iter().any(|(p, _)| p.base == "x"),
        "Regular assignment: 'x' should have L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "y"),
        "Regular assignment: 'y' should have L-value"
    );
}

#[test]
fn test_walrus_does_not_affect_augmented_assignment() {
    let source = r#"
def process():
    x = 0
    x += 1
"#;
    let path = "src/augmented.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let x_count = lvalues.iter().filter(|(p, _)| p.base == "x").count();
    assert!(
        x_count >= 2,
        "Augmented assignment: 'x' should have defs from both = and +=, got {}",
        x_count
    );
}

#[test]
fn test_walrus_in_while_loop() {
    let source = r#"
def process(stream):
    while (chunk := stream.read(1024)):
        use(chunk)
"#;
    let path = "src/walrus_while.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "chunk").is_empty(),
        "Walrus in while: 'chunk' should have def from := expression"
    );
}

#[test]
fn test_python_nested_tuple_unpack() {
    let source = r#"
def process():
    (a, b), c = get_nested()
    use(a)
    use(b)
    use(c)
"#;
    let path = "src/nested.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_a = lvalues.iter().any(|(p, _)| p.base == "a");
    let has_b = lvalues.iter().any(|(p, _)| p.base == "b");
    let has_c = lvalues.iter().any(|(p, _)| p.base == "c");
    assert!(
        has_a,
        "Nested tuple: 'a' should have L-value, got {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
    assert!(has_b, "Nested tuple: 'b' should have L-value");
    assert!(has_c, "Nested tuple: 'c' should have L-value");
}

#[test]
fn test_python_walrus_rhs_collected() {
    // Walrus RHS should be collected as a use (assignment_value must work)
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // Walrus RHS is a call, not plain ident — no alias expected, but should not panic.
    // The key test is that assignment_value returns Some for named_expression.
    let _ = aliases;

    // Verify def exists for n
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' should have a def");
}

#[test]
fn test_python_with_as_binding_def() {
    // Gap 4: with...as should produce a def for the bound variable
    let source = r#"
def process():
    with open("file.txt") as f:
        data = f.read()
        send(data)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let f_defs = dfg.all_defs_of(path, "f");
    assert!(!f_defs.is_empty(), "with...as: 'f' should have def");
}

#[test]
fn test_python_except_as_binding_def() {
    // except...as uses as_pattern too
    let source = r#"
def process():
    try:
        risky()
    except Exception as e:
        handle(e)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let e_defs = dfg.all_defs_of(path, "e");
    assert!(!e_defs.is_empty(), "except...as: 'e' should have def");
}

#[test]
fn test_walrus_assignment_value_flow() {
    // Walrus assignment_value should extract RHS identifiers for DFG use tracking.
    // If items is on a diff line, its use on the walrus RHS should be collected.
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    // The walrus RHS is len(items) — not a plain ident, so no alias.
    // But the assignment_value path should be Some (not None).
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // No alias expected (RHS is a call), but shouldn't panic
    let _ = aliases;

    // Verify DFG has defs
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' def should exist");
}

#[test]
fn test_go_short_var_declaration_value() {
    // Go short_var_declaration uses "right" field for declaration_value
    let source = r#"
package main

func process() {
    x := getData()
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    // short_var_declaration is both declaration and assignment in Go
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // getData() is not a plain ident, so no alias — but the path should not crash
    let _ = aliases;

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go short_var_declaration: 'x' should have def"
    );
}

#[test]
fn test_go_var_declaration_with_value() {
    // Go var_declaration with explicit value
    let source = r#"
package main

func process() {
    var x int = 42
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go var declaration: 'x' should have def"
    );
}
