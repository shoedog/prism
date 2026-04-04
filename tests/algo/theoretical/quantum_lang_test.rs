#[path = "../../common/mod.rs"]
mod common;
use common::*;

// === Tier 1: Quantum — Rust thread::spawn (std) ===

#[test]
fn test_quantum_rust_std_thread_spawn() {
    // Rust std::thread::spawn should be detected as async context,
    // separate from tokio::spawn which is already tested.
    let source = r#"
use std::thread;

fn process(data: Vec<u8>) -> usize {
    let mut count = 0;
    let handle = thread::spawn(move || {
        count = data.len();
        count
    });
    let result = handle.join().unwrap();
    result
}
"#;
    let path = "src/worker.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Rust std::thread::spawn as async context"
    );
}

#[test]
fn test_quantum_rust_rayon_spawn() {
    // rayon::spawn should be detected as async context.
    // Variable assigned before and after the async boundary.
    let source = r#"
fn parallel_compute(items: Vec<i32>) -> i32 {
    let mut result = 0;
    result = items.len() as i32;
    rayon::spawn(move || {
        process(items);
    });
    result = result + 1;
    result
}
"#;
    let path = "src/parallel.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect rayon::spawn as Rust async context"
    );
}

// === Tier 1: Quantum — C++ std::async / std::jthread ===

#[test]
fn test_quantum_cpp_std_async() {
    // std::async should be detected as C++ async context.
    let source = r#"
#include <future>

int compute(int x) {
    int result = 0;
    auto fut = std::async(std::launch::async, [&]() {
        result = x * 2;
        return result;
    });
    return fut.get();
}
"#;
    let path = "src/compute.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect C++ std::async as async context"
    );
}

#[test]
fn test_quantum_cpp_std_jthread() {
    // std::jthread (C++20) should be detected as async context.
    let source = r#"
#include <thread>

void process_data(int* data, int size) {
    int sum = 0;
    std::jthread worker([&]() {
        for (int i = 0; i < size; i++) {
            sum += data[i];
        }
    });
    return;
}
"#;
    let path = "src/jthread.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect C++ std::jthread as async context"
    );
}

#[test]
fn test_quantum_cpp_std_thread() {
    // std::thread should also be detected as async context.
    let source = r#"
#include <thread>

void worker(int* counter) {
    int local = 0;
    std::thread t([&]() {
        local = *counter + 1;
    });
    t.join();
}
"#;
    let path = "src/threading.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect C++ std::thread as async context"
    );
}

// === Tier 2: Quantum — Go channel send/receive (item 12) ===

#[test]
fn test_quantum_go_channel_send_receive() {
    // Go channel send (<-) should be detected as async boundary.
    let source = r#"
package main

func producer(ch chan int) {
    result := 0
    result = compute()
    ch <- result
    result = result + 1
}
"#;
    let path = "cmd/producer.go";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Go channel send as async boundary"
    );
}

#[test]
fn test_quantum_go_select_statement() {
    // Go select with multiple channel cases — async context.
    let source = r#"
package main

func mux(a chan int, b chan string) int {
    result := 0
    select {
    case v := <-a:
        result = v
    case s := <-b:
        result = len(s)
    }
    return result
}
"#;
    let path = "cmd/mux.go";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect Go select statement as async context"
    );
}

// === Tier 3: Quantum — Python asyncio.create_task/gather (item 16) ===

#[test]
fn test_quantum_python_asyncio_create_task() {
    // asyncio.create_task should be detected as async context.
    let source = r#"
import asyncio

async def process(items):
    result = 0
    task = asyncio.create_task(compute(items))
    result = result + 1
    await task
    return result
"#;
    let path = "app/async_proc.py";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect asyncio.create_task as Python async context"
    );
}

#[test]
fn test_quantum_python_asyncio_gather() {
    // asyncio.gather should be detected as async context.
    let source = r#"
import asyncio

async def fetch_all(urls):
    count = 0
    results = await asyncio.gather(*[fetch(u) for u in urls])
    count = len(results)
    return count
"#;
    let path = "app/fetcher.py";
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

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "QuantumSlice should detect asyncio.gather as Python async context"
    );
}
