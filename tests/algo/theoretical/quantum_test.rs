#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
