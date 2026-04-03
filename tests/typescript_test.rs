mod common;
use common::*;

// ====== TS Algorithm Coverage: 18 "none" algorithms ======

#[test]
fn test_full_flow_typescript() {
    let (files, _, diff) = make_typescript_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty(), "FullFlow TS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

#[test]
fn test_relevant_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "RelevantSlice TS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_gradient_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "GradientSlice TS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

#[test]
fn test_barrier_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty(), "BarrierSlice TS should produce blocks");
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_conditioned_slice_typescript() {
    let source = r#"
function process(x: number): number {
    if (x > 0) {
        const result = x * 2;
        return result;
    } else {
        return 0;
    }
}
"#;
    let path = "cond.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_chop_typescript() {
    let source = r#"
function pipeline(raw: string): string {
    const cleaned = sanitize(raw);
    const parsed = JSON.parse(cleaned);
    const result = transform(parsed);
    return result;
}
function sanitize(s: string): string { return s.trim(); }
function transform(o: any): string { return o.value; }
"#;
    let path = "pipe.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_symmetry_slice_typescript() {
    let source = r#"
function encode(data: object): string {
    return JSON.stringify(data);
}

function decode(str: string): object {
    return JSON.parse(str);
}
"#;
    let path = "codec.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_circular_slice_typescript() {
    let source = r#"
function ping(n: number): number {
    return pong(n + 1);
}

function pong(n: number): number {
    return ping(n - 1);
}
"#;
    let path = "cycle.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

#[test]
fn test_angle_slice_typescript() {
    let source = r#"
function process(data: string): string {
    console.log("Processing:", data);
    const result = data.trim();
    console.log("Result:", result);
    return result;
}
"#;
    let path = "logger.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

#[test]
fn test_horizontal_slice_typescript() {
    let source = r#"
function handleGet(req: Request): Response {
    const data = fetchData(req.params.id);
    return new Response(data);
}

function handlePost(req: Request): Response {
    const data = req.body;
    saveData(data);
    return new Response("ok");
}

function handleDelete(req: Request): Response {
    deleteData(req.params.id);
    return new Response("deleted");
}
"#;
    let path = "handlers.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_echo_slice_typescript() {
    let source_api = "function validate(input: string): string {\n    if (!input) {\n        throw new Error(\"missing\");\n    }\n    return input.trim();\n}\n";
    let source_caller =
        "function process(): string {\n    const result = validate(getData());\n    return result;\n}\n";
    let mut files = BTreeMap::new();
    files.insert(
        "validate.ts".to_string(),
        ParsedFile::parse("validate.ts", source_api, Language::TypeScript).unwrap(),
    );
    files.insert(
        "process.ts".to_string(),
        ParsedFile::parse("process.ts", source_caller, Language::TypeScript).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "validate.ts".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_provenance_slice_typescript() {
    let source = "function handler(req: Request, res: Response): void {\n    const token = req.headers.get('authorization');\n    const userId = parseToken(token);\n    const data = db.query(userId);\n    res.json(data);\n}\n";
    let path = "handler.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_quantum_slice_typescript() {
    let source = r#"
async function fetchData(url: string): Promise<any> {
    const response = await fetch(url);
    const data = await response.json();
    return data;
}

async function processAll(urls: string[]): Promise<any[]> {
    const promises = urls.map(u => fetchData(u));
    return Promise.all(promises);
}
"#;
    let path = "async.ts";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_spiral_slice_typescript() {
    let source = r#"
function inner(x: number): number {
    return x + 1;
}

function outer(y: number): number {
    const z = inner(y);
    return z * 2;
}

function caller(): void {
    const r = outer(10);
    console.log(r);
}
"#;
    let path = "spiral.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(!result.blocks.is_empty(), "SpiralSlice TS should produce blocks");
}

#[test]
fn test_vertical_slice_typescript() {
    let source_handler = r#"
function apiHandler(req: Request): Response {
    const data = req.body;
    const result = serviceProcess(data);
    return new Response(result);
}
"#;
    let source_service = r#"
function serviceProcess(data: any): any {
    const validated = validate(data);
    return repoSave(validated);
}
"#;
    let source_repo = r#"
function repoSave(data: any): boolean {
    db.insert(data);
    return true;
}
"#;
    let handler_path = "handler/api.ts";
    let service_path = "service/processor.ts";
    let repo_path = "repository/store.ts";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::TypeScript).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::TypeScript).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::TypeScript).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: service_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_delta_slice_typescript() {
    let tmp = TempDir::new().unwrap();

    let old_source = "function add(a: number, b: number): number {\n    return a + b;\n}\n";
    std::fs::write(tmp.path().join("calc.ts"), old_source).unwrap();

    let new_source = "function add(a: number, b: number): number {\n    const result = a + b;\n    return result;\n}\n";
    let path = "calc.ts";
    let parsed = ParsedFile::parse(path, new_source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

#[test]
fn test_resonance_slice_typescript() {
    let source = "function update(x: number): number {\n    const y = x + 1;\n    return y;\n}\n";
    let filename = "app.ts";
    let tmp = create_temp_git_repo(
        filename,
        &["function update(x: number): number {\n    return x;\n}\n", source],
    );

    let parsed = ParsedFile::parse(filename, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let config = prism::algorithms::resonance_slice::ResonanceConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        days: 365,
        min_co_changes: 1,
        min_ratio: 0.0,
    };
    let result = prism::algorithms::resonance_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ResonanceSlice);
}

#[test]
fn test_phantom_slice_typescript() {
    let source = "function remaining(x: number): number {\n    return x + 1;\n}\n";
    let filename = "app.ts";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "function deleted(x: number): number {\n    return x * 2;\n}\n\nfunction remaining(x: number): number {\n    return x + 1;\n}\n",
            source,
        ],
    );
    let parsed = ParsedFile::parse(filename, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
}

#[test]
fn test_threed_slice_typescript() {
    let source = "function foo(x: number): number {\n    const y = x + 1;\n    return y;\n}\n\nfunction bar(): void {\n    const r = foo(10);\n    console.log(r);\n}\n";
    let filename = "app.ts";
    let tmp = create_temp_git_repo(
        filename,
        &["function foo(x: number): number {\n    return x;\n}\n", source],
    );

    let parsed = ParsedFile::parse(filename, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

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
    assert!(!result.blocks.is_empty(), "OriginalDiff TS should produce blocks");
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
    assert!(!result.blocks.is_empty(), "LeftFlow TS should produce blocks");
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
    assert!(!result.blocks.is_empty(), "ParentFunction should work with TS classes");
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
    assert!(!result.blocks.is_empty(), "ThinSlice TS should produce blocks for async code");
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
    assert!(!result.blocks.is_empty(), "Taint TS should detect user input flow");
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
    assert!(!result.blocks.is_empty(), "Taint should work with TS destructured request body");
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
