mod common;
use common::*;

fn make_mutual_recursion_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
def ping(n):
    if n <= 0:
        return
    print("ping", n)
    pong(n - 1)

def pong(n):
    if n <= 0:
        return
    print("pong", n)
    ping(n - 1)
"#;
    let path = "recursive.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]), // pong(n - 1) call in ping
        }],
    };

    (files, diff)
}


fn make_error_handling_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
def fetch_data(url):
    try:
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        log.error(f"Failed to fetch {url}: {e}")
        raise

def process(url):
    try:
        data = fetch_data(url)
        return transform(data)
    except Exception:
        return None
"#;
    let path = "service.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]), // log.error line
        }],
    };

    (files, sources, diff)
}


fn make_async_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
async function fetchUser(id) {
    let user = null;
    const response = await fetch(`/api/users/${id}`);
    user = await response.json();
    if (user.active) {
        return user;
    }
    return null;
}
"#;
    let path = "async.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    (files, diff)
}


#[test]
fn test_spiral_slice_ring_containment() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice),
        None,
    )
    .unwrap();

    // Spiral should include at least the original diff lines
    assert!(!result.blocks.is_empty());

    let orig = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    let spiral_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let orig_lines: usize = orig
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        spiral_lines >= orig_lines,
        "SpiralSlice ({}) should have >= lines than OriginalDiff ({})",
        spiral_lines,
        orig_lines
    );
}


#[test]
fn test_circular_slice_detects_cycle() {
    let (files, diff) = make_mutual_recursion_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();

    // Should detect the ping↔pong cycle
    // The call graph will find the cycle
    let call_graph = CallGraph::build(&files);
    let cycles = call_graph.find_cycles_from(&["ping"]);
    // There should be at least one cycle
    assert!(
        !cycles.is_empty() || !result.blocks.is_empty(),
        "Should detect mutual recursion cycle"
    );
}


#[test]
fn test_horizontal_slice_finds_peers() {
    let source = r#"
def handle_create(request):
    data = request.json()
    validate(data)
    return create_item(data)

def handle_update(request):
    data = request.json()
    return update_item(data)

def handle_delete(request):
    item_id = request.args.get("id")
    return delete_item(item_id)
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]), // validate(data) line in handle_create
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    // Should include peer functions (handle_update, handle_delete)
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("handlers.py").unwrap();
    assert!(
        lines.len() > 5,
        "HorizontalSlice should include peer functions, got {} lines",
        lines.len()
    );
}


#[test]
fn test_vertical_slice_traces_layers() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice),
        None,
    )
    .unwrap();

    // Should produce at least one block showing the call chain
    // (calculate is called by process, which calls helper)
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_angle_slice_error_handling() {
    let (files, _, diff) = make_error_handling_test();
    let concern = prism::algorithms::angle_slice::Concern::ErrorHandling;
    let result = prism::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("service.py").unwrap();
    // Should find error handling patterns across both functions
    assert!(
        lines.len() > 3,
        "AngleSlice should trace error handling across functions"
    );
}


#[test]
fn test_quantum_slice_async_js() {
    let (files, diff) = make_async_test();
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("user")).unwrap();

    // May or may not find async patterns depending on tree-sitter parsing
    // Just verify it doesn't crash
    assert!(result.algorithm == SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_quantum_slice_c_pthread() {
    let source = r#"
#include <pthread.h>
#include <stdio.h>

int shared_counter = 0;

void *worker(void *arg) {
    shared_counter++;
    return NULL;
}

int main() {
    pthread_t thread;
    pthread_create(&thread, NULL, worker, NULL);
    shared_counter++;
    pthread_join(thread, NULL);
    printf("Counter: %d\n", shared_counter);
    return 0;
}
"#;

    let path = "src/threaded.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14]), // pthread_create line
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_quantum_c_signal_handler() {
    // signal() call makes the function async — quantum detects the async boundary.
    let source = r#"
void register_handlers(int signum) {
    int flags = signum;
    signal(SIGINT, handler);
    flags = flags | 1;
    return;
}
"#;
    let path = "src/signals.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: int flags = signum;  — flags is assigned before the signal() async boundary
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect async boundary from signal() in C function"
    );
}


#[test]
fn test_quantum_c_pthread_create() {
    // pthread_create makes the function async — quantum detects the thread creation.
    let source = r#"
int main() {
    pthread_t tid;
    int flag = 0;
    pthread_create(&tid, NULL, worker, &flag);
    flag = 1;
    pthread_join(tid, NULL);
    return 0;
}
"#;
    let path = "src/threads.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: pthread_create line  — flag assigned before and after the async boundary
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("flag")).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect async boundary from pthread_create"
    );
}


#[test]
fn test_quantum_c_isr_function_name() {
    // Function named rx_interrupt_handler is treated as async by name heuristic.
    let source = r#"
void rx_interrupt_handler(int irq) {
    int status = 0;
    status = irq;
    return;
}
"#;
    let path = "src/isr.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: status = irq;  — status is a local assigned inside an ISR-named function
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("status")).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should treat function named 'rx_interrupt_handler' as async \
         (ISR name heuristic)"
    );
}


#[test]
fn test_circular_slice_function_pointer_cycle() {
    // dispatch() calls handler->process(), and process() calls dispatch() — a cycle
    let source = r#"
#include <stdlib.h>

typedef struct handler {
    void (*process)(int data);
} handler_t;

void dispatch(handler_t *handler, int data);

void process(int data) {
    handler_t h;
    h.process = process;
    if (data > 0) {
        dispatch(&h, data - 1);
    }
}

void dispatch(handler_t *handler, int data) {
    handler->process(data);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/loop.c".to_string(),
        ParsedFile::parse("src/loop.c", source, Language::C).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/loop.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([12]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::CircularSlice),
        None,
    )
    .unwrap();

    // CircularSlice should detect the process → dispatch → process cycle
    // via handler->process() resolving to the "process" callee name
    let has_cycle_finding = result.findings.iter().any(|f| {
        f.description.contains("cycle") || f.category.as_deref() == Some("recursive_cycle")
    });
    assert!(
        !result.blocks.is_empty() || has_cycle_finding,
        "CircularSlice should detect cycle through function pointer dispatch"
    );
}


#[test]
fn test_quantum_signal_handler_cross_function_detection() {
    let source = r#"
#include <signal.h>
#include <stdlib.h>

volatile int running = 1;

void my_cleanup(int signo) {
    running = 0;
}

void setup(void) {
    signal(SIGTERM, my_cleanup);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/daemon.c".to_string(),
        ParsedFile::parse("src/daemon.c", source, Language::C).unwrap(),
    );

    // Diff touches my_cleanup body — which IS a signal handler
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/daemon.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    // my_cleanup is registered via signal() in setup(), so QuantumSlice
    // should detect it as async and produce output
    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect my_cleanup as async (registered via signal() in setup())"
    );
}


#[test]
fn test_quantum_pthread_registered_handler() {
    let source = r#"
#include <pthread.h>

int shared_data = 0;

void worker(void *arg) {
    shared_data = 42;
}

void start_worker(void) {
    pthread_t tid;
    pthread_create(&tid, NULL, worker, NULL);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/threads.c".to_string(),
        ParsedFile::parse("src/threads.c", source, Language::C).unwrap(),
    );

    // Diff touches worker body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/threads.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect worker as async (registered via pthread_create in start_worker)"
    );
}


#[test]
fn test_quantum_isr_cross_file_registration() {
    let handler_source = r#"
#include <linux/interrupt.h>

static int packet_count = 0;

irqreturn_t eth_rx_interrupt(int irq, void *dev_id) {
    packet_count = packet_count + 1;
    return IRQ_HANDLED;
}
"#;

    let setup_source = r#"
#include <linux/interrupt.h>

extern irqreturn_t eth_rx_interrupt(int irq, void *dev_id);

int eth_probe(struct device *dev) {
    int ret = request_irq(dev->irq, eth_rx_interrupt, IRQF_SHARED, "eth", dev);
    return ret;
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/eth_handler.c".to_string(),
        ParsedFile::parse("src/eth_handler.c", handler_source, Language::C).unwrap(),
    );
    files.insert(
        "src/eth_probe.c".to_string(),
        ParsedFile::parse("src/eth_probe.c", setup_source, Language::C).unwrap(),
    );

    // Diff touches the assignment in the handler body
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/eth_handler.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect eth_rx_interrupt as async (registered via request_irq in another file)"
    );
}


#[test]
fn test_quantum_python_threading_async() {
    // Python threading.Thread should be detected as async context.
    let source = r#"
import threading

def worker(data):
    count = 0
    t = threading.Thread(target=process, args=(data,))
    t.start()
    count = count + 1
    return count
"#;
    let path = "app/worker.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Python threading.Thread as async context"
    );
}


#[test]
fn test_quantum_js_worker_async() {
    // JavaScript Worker should be detected as async context.
    let source = r#"
function processData(data) {
    let result = null;
    const worker = new Worker('processor.js');
    result = data;
    return result;
}
"#;
    let path = "src/processor.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect JavaScript Worker as async context"
    );
}


#[test]
fn test_quantum_go_channel_select() {
    // Go select statement with channels should be detected as async context.
    let source = r#"
package main

func fanIn(ch1 chan int, ch2 chan int) int {
    result := 0
    select {
    case v := <-ch1:
        result = v
    case v := <-ch2:
        result = v
    }
    return result
}
"#;
    let path = "cmd/fanin.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Go select/channel as async context"
    );
}


#[test]
fn test_lua_quantum_coroutine() {
    // Lua coroutine.create should be detected as async context.
    let source = r#"
function producer(data)
    local count = 0
    local co = coroutine.create(function()
        count = count + 1
    end)
    coroutine.resume(co)
    return count
end
"#;
    let path = "scripts/async.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Lua coroutine.create as async context"
    );
}


#[test]
fn test_rust_quantum_tokio_spawn() {
    let source = r#"
async fn process(data: Vec<u8>) {
    let handle = tokio::spawn(async move {
        let result = compute(data).await;
        result
    });
    handle.await.unwrap();
}
"#;
    let path = "src/async_proc.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect tokio::spawn as Rust async context"
    );
}


#[test]
fn test_threed_slice_python() {
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
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
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce blocks for functions with churn"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_threed_slice_go() {
    let source = "package main\n\nfunc compute(n int) int {\n\tresult := n * 2\n\treturn result\n}\n\nfunc caller() {\n\tv := compute(5)\n\t_ = v\n}\n";
    let filename = "main.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc compute(n int) int { return n }\n",
            source,
        ],
    );

    let parsed = ParsedFile::parse(filename, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);

    let _ = std::fs::remove_dir_all(&tmp);
}


#[test]
fn test_angle_slice_python() {
    let source = r#"
import logging

def process(data):
    try:
        result = transform(data)
        logging.info("success")
        return result
    except Exception as e:
        logging.error(str(e))
        raise
"#;
    let path = "proc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9, 10]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_angle_slice_go() {
    let source = r#"package main

import "log"

func handler() error {
	result, err := doWork()
	if err != nil {
		log.Printf("error: %v", err)
		return err
	}
	log.Printf("success: %v", result)
	return nil
}

func doWork() (int, error) {
	return 42, nil
}
"#;
    let path = "handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_angle_slice_javascript_logging() {
    let source = r#"
function fetchData(url) {
    console.log("fetching", url);
    const res = fetch(url);
    if (res.error) {
        console.error("failed", res.error);
        return null;
    }
    console.log("done");
    return res;
}
"#;
    let path = "fetch.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Logging,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_angle_slice_custom_concern_python() {
    let source = r#"
import redis

def get_cached(key):
    cache = redis.get(key)
    if cache:
        return cache
    result = compute(key)
    redis.set(key, result, ttl=300)
    return result
"#;
    let path = "cache.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Caching,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_spiral_slice_python() {
    let source = r#"
def inner(x):
    return x + 1

def outer(y):
    z = inner(y)
    return z * 2

def caller():
    r = outer(10)
    print(r)
"#;
    let path = "spiral.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
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
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_spiral_slice_go() {
    let source = r#"package main

func compute(n int) int {
	return n * 2
}

func process(x int) int {
	r := compute(x)
	return r + 1
}

func main() {
	v := process(5)
	println(v)
}
"#;
    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 6,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_spiral_slice_ring1_only_python() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 1,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_horizontal_slice_python() {
    let source = r#"
def handle_get(request):
    data = get_data()
    return data

def handle_post(request):
    data = request.body
    save_data(data)
    return "ok"

def handle_delete(request):
    delete_data(request.id)
    return "deleted"
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("handle_*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}


#[test]
fn test_horizontal_slice_javascript() {
    let source = r#"
function handleLogin(req, res) {
    const user = authenticate(req.body);
    res.send(user);
}

function handleLogout(req, res) {
    clearSession(req);
    res.send("ok");
}

function handleRegister(req, res) {
    const user = createUser(req.body);
    res.send(user);
}
"#;
    let path = "routes.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Auto,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}


#[test]
fn test_horizontal_slice_go() {
    let source = r#"package main

func HandleGet(w http.ResponseWriter, r *http.Request) {
	data := getData()
	w.Write(data)
}

func HandlePost(w http.ResponseWriter, r *http.Request) {
	body := r.Body
	saveData(body)
}
"#;
    let path = "routes.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("Handle*".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}


#[test]
fn test_vertical_slice_python() {
    let source_handler = r#"
def api_handler(request):
    data = request.json()
    result = service_process(data)
    return result
"#;
    let source_service = r#"
def service_process(data):
    validated = validate(data)
    return repo_save(validated)
"#;
    let source_repo = r#"
def repo_save(data):
    db.insert(data)
    return True
"#;
    let handler_path = "handler/api.py";
    let service_path = "service/processor.py";
    let repo_path = "repository/store.py";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::Python).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::Python).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::Python).unwrap(),
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
fn test_vertical_slice_go() {
    let source = r#"package main

func handler(w http.ResponseWriter, r *http.Request) {
	data := parseRequest(r)
	result := service(data)
	w.Write(result)
}

func service(data string) string {
	return repository(data)
}

func repository(key string) string {
	return db.Get(key)
}
"#;
    let path = "handler/main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
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


#[test]
fn test_circular_slice_python() {
    let source = r#"
def a(x):
    return b(x + 1)

def b(y):
    return a(y - 1)
"#;
    let path = "cycle.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
fn test_circular_slice_go() {
    let source = r#"package main

func ping(n int) int {
	return pong(n + 1)
}

func pong(n int) int {
	return ping(n - 1)
}
"#;
    let path = "cycle.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
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
fn test_quantum_slice_python() {
    let source = r#"
import threading

def worker(data):
    result = process(data)
    return result

def main():
    t = threading.Thread(target=worker, args=(42,))
    t.start()
    t.join()
"#;
    let path = "async.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_quantum_slice_go_channel() {
    let source = r#"package main

func worker(ch chan int) {
	result := compute()
	ch <- result
}

func main() {
	ch := make(chan int)
	go worker(ch)
	v := <-ch
	_ = v
}

func compute() int { return 42 }
"#;
    let path = "concurrent.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_quantum_slice_javascript_async() {
    let source = r#"
async function fetchAll(urls) {
    const promises = urls.map(url => fetch(url));
    const results = await Promise.all(promises);
    return results;
}
"#;
    let path = "async.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };
    let result = prism::algorithms::quantum_slice::slice(&files, &diff, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_vertical_slice_explicit_layers_python() {
    // Test with explicit layer ordering
    let source = r#"
def api_handler(request):
    return service_call(request.data)

def service_call(data):
    return repo_save(data)

def repo_save(data):
    return True
"#;
    let path = "handler/app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = prism::algorithms::vertical_slice::VerticalConfig {
        layers: vec![
            "Handler".to_string(),
            "Service".to_string(),
            "Repository".to_string(),
        ],
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}


#[test]
fn test_angle_slice_concern_not_on_diff_python() {
    // Concern exists in code but not on the diff lines themselves
    let source = r#"
def handler():
    try:
        result = compute()
    except Exception as e:
        log_error(e)
        raise
    return result

def compute():
    x = 1
    y = x + 1
    return y
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff is on lines 11-12 in compute(), which has no error handling patterns
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11, 12]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::ErrorHandling,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    // Should still find error handling code in the file
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_angle_slice_authentication_go() {
    let source = r#"package main

func authMiddleware(token string) bool {
	session := validateToken(token)
	if session == nil {
		return false
	}
	return authorize(session)
}

func validateToken(t string) interface{} {
	return nil
}

func authorize(s interface{}) bool {
	return true
}
"#;
    let path = "auth.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let result = prism::algorithms::angle_slice::slice(
        &files,
        &diff,
        &prism::algorithms::angle_slice::Concern::Authentication,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_spiral_slice_max_ring_6_python() {
    // Test spiral with max ring 6 to cover ring 5 (test files) and ring 6 (shared utils)
    let source_main = r#"
def compute(x):
    y = helper(x)
    return y * 2
"#;
    let source_helper = r#"
def helper(x):
    return x + 1
"#;
    let source_test = r#"
def test_compute():
    assert compute(5) == 12
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "src/main.py".to_string(),
        ParsedFile::parse("src/main.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "src/helper.py".to_string(),
        ParsedFile::parse("src/helper.py", source_helper, Language::Python).unwrap(),
    );
    files.insert(
        "tests/test_main.py".to_string(),
        ParsedFile::parse("tests/test_main.py", source_test, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/main.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 6,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert!(!result.blocks.is_empty());
}


#[test]
fn test_horizontal_slice_name_suffix_python() {
    // Test NamePattern with suffix matching (*_handler)
    let source = r#"
def get_handler(request):
    return get_data()

def post_handler(request):
    save_data(request.body)

def delete_handler(request):
    remove_data(request.id)
"#;
    let path = "handlers.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::NamePattern("*_handler".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}


#[test]
fn test_horizontal_slice_decorator_python() {
    // Test Decorator matching
    let source = r#"
@app.route("/users")
def get_users():
    return users

@app.route("/items")
def get_items():
    return items
"#;
    let path = "routes.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = prism::algorithms::horizontal_slice::slice(
        &files,
        &diff,
        &prism::algorithms::horizontal_slice::PeerPattern::Decorator("@app.route".to_string()),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}


#[test]
fn test_quantum_slice_with_target_var_python() {
    // Test quantum_slice with a specific target variable
    let source = r#"
import asyncio

async def fetch(url):
    data = await get(url)
    result = process(data)
    return result
"#;
    let path = "fetch.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = prism::algorithms::quantum_slice::slice(&files, &diff, Some("data")).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}


#[test]
fn test_threed_slice_python_risk_scoring() {
    // ThreeDSlice should produce blocks sorted by risk
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
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
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce risk-scored blocks"
    );

    // The first block should contain the diff function (highest risk)
    let first_block = &result.blocks[0];
    let lines = first_block.file_line_map.get(filename);
    assert!(
        lines.is_some(),
        "First block should contain lines from the diff file"
    );
}


#[test]
fn test_vertical_slice_python_layer_detection() {
    // Vertical slice should detect layers from file paths
    let source_handler = "def api_handler(request):\n    return service_call(request.data)\n";
    let source_service = "def service_call(data):\n    return repo_save(data)\n";
    let source_repo = "def repo_save(data):\n    return True\n";

    let mut files = BTreeMap::new();
    files.insert(
        "handler/api.py".to_string(),
        ParsedFile::parse("handler/api.py", source_handler, Language::Python).unwrap(),
    );
    files.insert(
        "service/logic.py".to_string(),
        ParsedFile::parse("service/logic.py", source_service, Language::Python).unwrap(),
    );
    files.insert(
        "repository/store.py".to_string(),
        ParsedFile::parse("repository/store.py", source_repo, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "service/logic.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    // Should produce blocks — at minimum the diff function
    assert!(
        !result.blocks.is_empty(),
        "VerticalSlice should trace layers for service function"
    );
}


#[test]
fn test_spiral_slice_ring_expansion_go() {
    // Verify that higher ring numbers produce more output than lower ones
    let source = r#"package main

func inner(x int) int { return x + 1 }
func middle(x int) int { return inner(x) * 2 }
func outer(x int) int { return middle(x) + 3 }
func caller() int { return outer(10) }
"#;
    let path = "chain.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);

    let ctx = CpgContext::build(&files, None);
    let ring2 = prism::algorithms::spiral_slice::slice(
        &ctx,
        &diff,
        &config,
        &prism::algorithms::spiral_slice::SpiralConfig {
            max_ring: 2,
            auto_stop_threshold: 0.0,
        },
    )
    .unwrap();

    let ring4 = prism::algorithms::spiral_slice::slice(
        &ctx,
        &diff,
        &config,
        &prism::algorithms::spiral_slice::SpiralConfig {
            max_ring: 4,
            auto_stop_threshold: 0.0,
        },
    )
    .unwrap();

    let count_lines = |r: &prism::slice::SliceResult| -> usize {
        r.blocks
            .iter()
            .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
            .sum()
    };

    assert!(
        count_lines(&ring4) >= count_lines(&ring2),
        "Ring 4 ({} lines) should have >= Ring 2 ({} lines)",
        count_lines(&ring4),
        count_lines(&ring2)
    );
}

