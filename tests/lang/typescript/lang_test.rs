#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ====== TS "basic" → "full" coverage upgrades (8 algorithms) ======

#[test]
fn test_original_diff_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff TS should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
    let block = &result.blocks[0];
    assert!(
        block.file_line_map.contains_key("src/client.ts"),
        "OriginalDiff should reference the TS source file"
    );
}

#[test]
fn test_original_diff_typescript_interface() {
    let source = r#"
interface User {
    name: string;
    email: string;
}

function createUser(name: string, email: string): User {
    const user: User = { name, email };
    return user;
}
"#;
    let path = "user.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow TS should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_left_flow_typescript_generics() {
    let source = r#"
function map<T, U>(items: T[], fn: (item: T) => U): U[] {
    const results: U[] = [];
    for (const item of items) {
        results.push(fn(item));
    }
    return results;
}
"#;
    let path = "util.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_parent_function_typescript_class() {
    let source = r#"
class Calculator {
    private accumulator: number = 0;

    add(a: number, b: number): number {
        const sum = a + b;
        this.accumulator += sum;
        return sum;
    }

    getAccumulator(): number {
        return this.accumulator;
    }
}
"#;
    let path = "calc.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6, 7]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should work with TS classes"
    );
}

#[test]
fn test_thin_slice_typescript_async() {
    let source = r#"
async function fetchData(url: string): Promise<any> {
    const response = await fetch(url);
    const data = await response.json();
    const processed = transform(data);
    return processed;
}
"#;
    let path = "fetch.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice TS should produce blocks for async code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_taint_typescript_user_input() {
    let source = r#"
function handler(req: Request): Response {
    const userInput = req.body.name;
    const query = "SELECT * FROM users WHERE name = '" + userInput + "'";
    const result = db.execute(query);
    return new Response(JSON.stringify(result));
}
"#;
    let path = "handler.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "Taint TS should detect user input flow"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::Taint);
}

#[test]
fn test_taint_typescript_destructured_input() {
    let source = r#"
function processRequest(req: Request): void {
    const { username, password } = req.body;
    const sanitized = username.trim();
    db.query("INSERT INTO logs VALUES('" + sanitized + "')");
}
"#;
    let path = "auth.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "Taint should work with TS destructured request body"
    );
}

#[test]
fn test_absence_slice_typescript_resource() {
    let source = r#"
function processFile(path: string): Buffer {
    const fd = fs.openSync(path, 'r');
    const data = fs.readFileSync(fd);
    return data;
}
"#;
    let path = "file.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_absence_slice_typescript_event_listener() {
    let source = r#"
function setup(element: HTMLElement): void {
    element.addEventListener('click', handler);
    doSomething();
}
"#;
    let path = "dom.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_circular_slice_typescript_class_methods() {
    let source = r#"
class StateMachine {
    stateA(input: string): string {
        return this.stateB(input + "a");
    }

    stateB(input: string): string {
        return this.stateA(input + "b");
    }
}
"#;
    let path = "state.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_membrane_slice_typescript_multifile() {
    let source_api = r#"
function fetchUser(id: number): User {
    const user = db.get(id);
    if (!user) throw new Error("not found");
    return user;
}
"#;
    let source_caller = r#"
function renderProfile(id: number): string {
    const user = fetchUser(id);
    return template(user);
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "api.ts".to_string(),
        ParsedFile::parse("api.ts", source_api, Language::TypeScript).unwrap(),
    );
    files.insert(
        "profile.ts".to_string(),
        ParsedFile::parse("profile.ts", source_caller, Language::TypeScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "api.ts".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
}
