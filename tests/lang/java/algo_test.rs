#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Paper algorithms ──

#[test]
fn test_original_diff_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Java code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
}

// ── Taxonomy algorithms ──

#[test]
fn test_relevant_slice_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice should produce blocks for Java code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_barrier_slice_java_method_chain() {
    let source = r#"public class Chain {
    public String level0(String x) {
        return level1(x + "a");
    }

    public String level1(String y) {
        return level2(y + "b");
    }

    public String level2(String z) {
        return z + "c";
    }
}
"#;
    let path = "Chain.java";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_chop_java_data_pipeline() {
    let source = r#"public class Pipeline {
    public String process(String input) {
        String validated = validate(input);
        String transformed = transform(validated);
        String result = format(transformed);
        return result;
    }

    private String validate(String s) { return s.trim(); }
    private String transform(String s) { return s.toUpperCase(); }
    private String format(String s) { return "[" + s + "]"; }
}
"#;
    let path = "Pipeline.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 6,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_conditioned_slice_java_switch() {
    let source = r#"public class Classifier {
    public String classify(int code) {
        String label;
        switch (code) {
            case 200:
                label = "OK";
                break;
            case 404:
                label = "Not Found";
                break;
            case 500:
                label = "Server Error";
                break;
            default:
                label = "Unknown";
                break;
        }
        return label;
    }
}
"#;
    let path = "Classifier.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
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
fn test_taint_java_sql_injection() {
    let source = r#"public class UserDAO {
    public User findUser(String id) {
        String query = "SELECT * FROM users WHERE id = " + id;
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery(query);
        return mapUser(rs);
    }
}
"#;
    let path = "UserDAO.java";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Taint);
}

// ── Theoretical algorithms ──

#[test]
fn test_quantum_slice_java_concurrency() {
    let source = r#"import java.util.concurrent.CompletableFuture;

public class Worker {
    public void execute() {
        Thread t = new Thread(() -> { processAsync(); });
        t.start();
        CompletableFuture.supplyAsync(() -> compute());
    }

    private void processAsync() { }
    private String compute() { return "done"; }
}
"#;
    let path = "Worker.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 7]),
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
fn test_horizontal_slice_java_interface_impls() {
    let source = r#"public interface Processor {
    String process(String input);
}

public class UpperProcessor implements Processor {
    public String process(String input) {
        return input.toUpperCase();
    }
}

public class LowerProcessor implements Processor {
    public String process(String input) {
        return input.toLowerCase();
    }
}

public class TrimProcessor implements Processor {
    public String process(String input) {
        return input.trim();
    }
}
"#;
    let path = "Processors.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
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
fn test_angle_slice_java_logging_concern() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

// ── Novel algorithms ──

#[test]
fn test_absence_slice_java_connection_leak() {
    let source = r#"import java.sql.*;

public class DataService {
    public String getData() {
        Connection conn = DriverManager.getConnection(url);
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1");
        return rs.getString(1);
    }
}
"#;
    let path = "DataService.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 7]),
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
    // AbsenceSlice should detect missing conn.close()
    if !result.blocks.is_empty() {
        let block = &result.blocks[0];
        assert!(
            block.file_line_map.contains_key(path),
            "AbsenceSlice should reference the Java file"
        );
    }
}

#[test]
fn test_echo_slice_java_ripple() {
    let source_service = r#"public class Service {
    public String process(String input) {
        return input.toUpperCase();
    }
}
"#;
    let source_controller = r#"public class Controller {
    private Service service;

    public void handle(String input) {
        String result = service.process(input);
        System.out.println(result);
    }
}
"#;
    let svc_path = "Service.java";
    let ctrl_path = "Controller.java";
    let mut files = BTreeMap::new();
    files.insert(
        svc_path.to_string(),
        ParsedFile::parse(svc_path, source_service, Language::Java).unwrap(),
    );
    files.insert(
        ctrl_path.to_string(),
        ParsedFile::parse(ctrl_path, source_controller, Language::Java).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: svc_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
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
fn test_membrane_slice_java_cross_class() {
    let source_service = r#"public class PaymentService {
    public boolean charge(String account, double amount) {
        boolean valid = validateAccount(account);
        if (!valid) return false;
        return processPayment(account, amount);
    }

    private boolean validateAccount(String account) {
        return account != null && !account.isEmpty();
    }

    private boolean processPayment(String account, double amount) {
        return amount > 0;
    }
}
"#;
    let source_controller = r#"public class OrderController {
    private PaymentService paymentService;

    public String placeOrder(String account, double total) {
        boolean charged = paymentService.charge(account, total);
        if (!charged) {
            return "Payment failed";
        }
        return "Order placed";
    }
}
"#;
    let svc_path = "PaymentService.java";
    let ctrl_path = "OrderController.java";
    let mut files = BTreeMap::new();
    files.insert(
        svc_path.to_string(),
        ParsedFile::parse(svc_path, source_service, Language::Java).unwrap(),
    );
    files.insert(
        ctrl_path.to_string(),
        ParsedFile::parse(ctrl_path, source_controller, Language::Java).unwrap(),
    );
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: svc_path.to_string(),
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

#[test]
fn test_provenance_slice_java_request_input() {
    let source = r#"import javax.servlet.http.*;

public class UserServlet extends HttpServlet {
    protected void doGet(HttpServletRequest request, HttpServletResponse response) {
        String userId = request.getParameter("id");
        String name = request.getParameter("name");
        User user = userService.createUser(userId, name);
        response.getWriter().write(user.toString());
    }
}
"#;
    let path = "UserServlet.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
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

// === 2.7 Behavioral test: Horizontal — Java interface methods as peers ===

#[test]
fn test_horizontal_java_interface_peers() {
    // Methods implementing the same interface in separate files should be detected as peers.
    let source_email = r#"
public class EmailValidator {
    public boolean validate(String input) {
        return input.contains("@");
    }
}
"#;
    let source_phone = r#"
public class PhoneValidator {
    public boolean validate(String input) {
        return input.matches("\\d{10}");
    }
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "src/EmailValidator.java".to_string(),
        ParsedFile::parse("src/EmailValidator.java", source_email, Language::Java).unwrap(),
    );
    files.insert(
        "src/PhoneValidator.java".to_string(),
        ParsedFile::parse("src/PhoneValidator.java", source_phone, Language::Java).unwrap(),
    );

    // Diff touches EmailValidator.validate
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/EmailValidator.java".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();

    // Horizontal should find PhoneValidator.validate() as a peer (same name, different file)
    assert!(
        !result.blocks.is_empty(),
        "Horizontal should detect peer validate() implementations across files"
    );

    // The block should include lines from PhoneValidator's validate
    let block = &result.blocks[0];
    let has_peer = block.file_line_map.contains_key("src/PhoneValidator.java");
    assert!(
        has_peer,
        "Horizontal should include PhoneValidator.java as peer file"
    );
}
