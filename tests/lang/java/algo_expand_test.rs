//! Expanded algorithm coverage tests for Java language.
//!
//! Covers 9 algorithms not yet tested with Java fixtures:
//! DeltaSlice, SpiralSlice, CircularSlice, VerticalSlice, ThreeDSlice,
//! ResonanceSlice, SymmetrySlice, GradientSlice, PhantomSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// DeltaSlice — changed method signature between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_java() {
    let tmp = TempDir::new().unwrap();

    let old_source = "public class Calc {\n    public int add(int a, int b) {\n        return a + b;\n    }\n}\n";
    std::fs::write(tmp.path().join("Calc.java"), old_source).unwrap();

    let new_source = "public class Calc {\n    public int add(int a, int b) {\n        int result = a + b;\n        return result;\n    }\n}\n";
    let path = "Calc.java";
    let parsed = ParsedFile::parse(path, new_source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

// ---------------------------------------------------------------------------
// SpiralSlice — adaptive depth around changed method
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_java() {
    let (files, _, diff) = make_java_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
}

// ---------------------------------------------------------------------------
// CircularSlice — circular dependency between classes
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_java() {
    let source = r#"public class NodeA {
    public int process(int x) {
        NodeB b = new NodeB();
        return b.transform(x + 1);
    }
}

class NodeB {
    public int transform(int y) {
        NodeA a = new NodeA();
        return a.process(y - 1);
    }
}
"#;
    let path = "NodeA.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — Controller → Service → DAO path
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_java() {
    let source_ctrl = r#"public class UserController {
    private UserService service;

    public String getUser(String id) {
        return service.findById(id);
    }
}
"#;
    let source_svc = r#"public class UserService {
    private UserDAO dao;

    public String findById(String id) {
        return dao.query(id);
    }
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "UserController.java".to_string(),
        ParsedFile::parse("UserController.java", source_ctrl, Language::Java).unwrap(),
    );
    files.insert(
        "UserService.java".to_string(),
        ParsedFile::parse("UserService.java", source_svc, Language::Java).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "UserController.java".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
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

// ---------------------------------------------------------------------------
// ThreeDSlice — temporal-structural risk (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_threed_slice_java() {
    let source_v1 = "public class Svc {\n    public int calc(int x) { return x; }\n}\n";
    let source_v2 = "public class Svc {\n    public int calc(int x) { return x + 1; }\n}\n";
    let filename = "Svc.java";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let ctx = CpgContext::build(&files, None);
    let config = prism::algorithms::threed_slice::ThreeDConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        temporal_days: 365,
    };
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

// ---------------------------------------------------------------------------
// ResonanceSlice — git co-change coupling
// ---------------------------------------------------------------------------

#[test]
fn test_resonance_slice_java() {
    let source_v1 = "public class Init {\n    public void setup() { }\n}\n";
    let source_v2 = "public class Init {\n    public void setup() { configure(); }\n}\n";
    let filename = "Init.java";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Java).unwrap();
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

// ---------------------------------------------------------------------------
// SymmetrySlice — serialize/deserialize pair
// ---------------------------------------------------------------------------

#[test]
fn test_symmetry_slice_java() {
    let source = r#"public class Codec {
    public byte[] serialize(Object obj) {
        return obj.toString().getBytes();
    }

    public Object deserialize(byte[] data) {
        return new String(data);
    }
}
"#;
    let path = "Codec.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
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

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

#[test]
fn test_gradient_slice_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

// ---------------------------------------------------------------------------
// PhantomSlice — recently deleted code (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_phantom_slice_java() {
    let source_v1 = "public class Helper {\n    public void cleanup() { }\n}\nclass Main {\n    void work() {\n        new Helper().cleanup();\n    }\n}\n";
    let source_v2 = "class Main {\n    void work() {\n        // simplified\n    }\n}\n";
    let filename = "Helper.java";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
}
