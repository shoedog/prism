#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ====== C Taint Sink Tests — Buffer Overflow / Firmware ======

#[test]
fn test_c_taint_memcpy_sink() {
    // memcpy with unvalidated length is a buffer overflow sink
    let source = r#"
#include <string.h>
void handle_packet(uint8_t *pdu, size_t pdu_len) {
    char buf[64];
    size_t len = pdu[7];
    memcpy(buf, pdu + 8, len);
}
"#;
    let path = "handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag memcpy as a buffer overflow sink"
    );
}

#[test]
fn test_c_taint_strcpy_sink() {
    // strcpy with no bounds check is a classic buffer overflow
    let source = r#"
#include <string.h>
void process_name(const char *input) {
    char name[32];
    strcpy(name, input);
}
"#;
    let path = "process.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag strcpy as a buffer overflow sink"
    );
}

#[test]
fn test_c_taint_sprintf_sink() {
    // sprintf without bounds is a format string / overflow sink
    let source = r#"
#include <stdio.h>
void log_message(const char *user_msg) {
    char buf[128];
    sprintf(buf, "User said: %s", user_msg);
}
"#;
    let path = "logger.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag sprintf as a format string / overflow sink"
    );
}

#[test]
fn test_c_taint_copy_from_user_sink() {
    // copy_from_user is a kernel user-space data ingress — taint source AND sink
    let source = r#"
#include <linux/uaccess.h>
int my_ioctl(struct file *f, unsigned int cmd, unsigned long arg) {
    struct my_data data;
    copy_from_user(&data, (void __user *)arg, sizeof(data));
    return 0;
}
"#;
    let path = "driver.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag copy_from_user as kernel data ingress"
    );
}

// ====== C Provenance Tests ======

#[test]
fn test_c_provenance_recv_network_input() {
    // recv() data should be classified as user input (network)
    let source = r#"
#include <sys/socket.h>
void handle_client(int sock) {
    char buf[1024];
    int n = recv(sock, buf, sizeof(buf), 0);
    process(buf, n);
}
"#;
    let path = "server.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_provenance = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_provenance,
        "Provenance should classify recv() as network input origin"
    );
}

#[test]
fn test_c_provenance_copy_from_user_input() {
    // copy_from_user is kernel user-space data ingress
    let source = r#"
#include <linux/uaccess.h>
int my_read(struct file *f, char __user *buf, size_t count, loff_t *pos) {
    struct my_data data;
    copy_from_user(&data, buf, count);
    return process(&data);
}
"#;
    let path = "driver.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input") || f.description.contains("UserInput"));
    assert!(
        has_user_input,
        "Provenance should classify copy_from_user as user input. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

// ====== C Absence Tests ======

#[test]
fn test_c_absence_pthread_mutex_no_unlock() {
    // pthread_mutex_lock without unlock is a deadlock risk
    let source = r#"
#include <pthread.h>
static pthread_mutex_t mtx = PTHREAD_MUTEX_INITIALIZER;

void critical_section(int *counter) {
    pthread_mutex_lock(&mtx);
    *counter += 1;
}
"#;
    let path = "thread.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let has_absence = result
        .findings
        .iter()
        .any(|f| f.description.contains("pthread") || f.description.contains("mutex"));
    assert!(
        has_absence,
        "AbsenceSlice should flag pthread_mutex_lock without unlock. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_c_absence_mmap_no_munmap() {
    // mmap without munmap is a memory leak
    let source = r#"
#include <sys/mman.h>
void map_device(int fd) {
    void *addr = mmap(NULL, 4096, PROT_READ, MAP_SHARED, fd, 0);
    process(addr);
}
"#;
    let path = "mapper.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let has_absence = result
        .findings
        .iter()
        .any(|f| f.description.contains("mmap") || f.description.contains("munmap"));
    assert!(
        has_absence,
        "AbsenceSlice should flag mmap without munmap. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_c_absence_kmalloc_no_kfree() {
    // kmalloc without kfree is a kernel memory leak
    let source = r#"
#include <linux/slab.h>
int init_device(struct device *dev) {
    struct my_data *data = kmalloc(sizeof(*data), GFP_KERNEL);
    dev->priv = data;
    return 0;
}
"#;
    let path = "driver.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let has_absence = result
        .findings
        .iter()
        .any(|f| f.description.contains("kernel") || f.description.contains("kfree"));
    assert!(
        has_absence,
        "AbsenceSlice should flag kmalloc without kfree. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}
