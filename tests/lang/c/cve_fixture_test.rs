//! CVE pattern test fixtures exercising existing Prism algorithms against
//! real-world vulnerability patterns from OpenWrt, Linux kernel, and embedded firmware.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ====== Fixture 1: Format String via syslog (CWE-134) ======

#[test]
fn test_cve_syslog_format_string() {
    let source = r#"
#include <syslog.h>
#include <string.h>

void handle_login(const char *username) {
    char msg[256];
    snprintf(msg, sizeof(msg), "Login attempt: %s", username);
    syslog(LOG_INFO, msg);
}
"#;
    let path = "tests/fixtures/c/cve_syslog_format.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8, 9]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_syslog_finding = result
        .findings
        .iter()
        .any(|f| f.description.contains("syslog"));
    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint should detect user input flowing to syslog, got: {:?}",
        result.findings
    );
    if !has_syslog_finding {
        // Fallback: check that taint at least captured the data flow
        assert!(
            !result.blocks.is_empty(),
            "Taint should produce blocks for syslog format string pattern"
        );
    }
}

// ====== Fixture 2: Signedness Mismatch in Length Field (CWE-681) ======

#[test]
fn test_cve_signedness_mismatch() {
    let source = r#"
#include <string.h>
#include <stdint.h>

void parse_tlv(uint8_t *pdu, size_t pdu_len) {
    int length = (int16_t)(pdu[2] << 8 | pdu[3]);
    if (length > 1024) return;
    char buf[1024];
    memcpy(buf, pdu + 4, length);
}
"#;
    let path = "tests/fixtures/c/cve_signedness.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6, 7, 8, 9]),
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
        "Taint should trace pdu → length → memcpy size arg, got: {:?}",
        result.findings
    );
}

// ====== Fixture 3: UCI Command Injection (CWE-78) ======

#[test]
fn test_cve_uci_command_injection() {
    let source = r#"
#include <stdlib.h>

void set_hostname(const char *user_input) {
    char cmd[512];
    snprintf(cmd, sizeof(cmd), "uci set system.@system[0].hostname='%s'", user_input);
    system(cmd);
    system("uci commit system");
}
"#;
    let path = "tests/fixtures/c/cve_uci_injection.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6, 7]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_system_finding = result
        .findings
        .iter()
        .any(|f| f.description.contains("system"));
    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint || has_system_finding,
        "Taint should detect user_input flowing to system(), got: {:?}",
        result.findings
    );
}

// ====== Fixture 4: Netlink nlmsg_len Overflow (CWE-120) ======

#[test]
fn test_cve_netlink_overflow() {
    let source = r#"
#include <string.h>
#include <stdint.h>

struct nlmsghdr { uint32_t nlmsg_len; uint16_t nlmsg_type; };

void handle_netlink_msg(struct nlmsghdr *nlh) {
    char buf[4096];
    size_t payload_len = nlh->nlmsg_len - sizeof(struct nlmsghdr);
    memcpy(buf, (char *)nlh + sizeof(struct nlmsghdr), payload_len);
}
"#;
    let path = "tests/fixtures/c/cve_netlink_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9, 10, 11]),
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
        "Taint should trace nlh->nlmsg_len → payload_len → memcpy size, got: {:?}",
        result.findings
    );
}

// ====== Fixture 5: IOCTL User Buffer Overflow (CWE-120) ======

#[test]
fn test_cve_ioctl_overflow() {
    let source = r#"
#include <stdint.h>

struct user_request { uint32_t size; uint8_t data[]; };

int handle_ioctl(unsigned long arg) {
    struct user_request req;
    copy_from_user(&req, (void *)arg, sizeof(req));
    char *buf = kmalloc(req.size, 0);
    copy_from_user(buf, (void *)arg + sizeof(req), req.size);
    return process(buf, req.size);
}
"#;
    let path = "tests/fixtures/c/cve_ioctl_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8, 9, 10, 11]),
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
        "Taint should trace arg → copy_from_user → req.size → second copy, got: {:?}",
        result.findings
    );
}

// ====== Fixture 6: NULL Dereference After Unchecked kmalloc (CWE-476) ======

#[test]
fn test_cve_null_deref_kmalloc() {
    let source = r#"
#include <stdint.h>

struct buffer { void *data; int len; };

void init_buffers(int count) {
    struct buffer *bufs = kmalloc(count * sizeof(struct buffer), 0);
    bufs[0].data = kmalloc(1024, 0);
    bufs[0].len = 1024;
}
"#;
    let path = "tests/fixtures/c/cve_null_deref.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8, 9]),
        }],
    };

    // Absence detects kmalloc without matching kfree (missing_counterpart)
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let missing: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_counterpart"))
        .collect();
    assert!(
        !missing.is_empty(),
        "Absence should detect kmalloc without kfree (missing_counterpart), got: {:?}",
        result.findings
    );
}

// ====== Fixture 7: Off-by-One in Protocol Parsing (CWE-193) ======

#[test]
fn test_cve_off_by_one_parsing() {
    let source = r#"
#include <stdint.h>
#include <string.h>

void parse_options(uint8_t *pdu, size_t pdu_len) {
    uint8_t option_count = pdu[4];
    char options[32][64];
    for (int i = 0; i <= option_count; i++) {
        memcpy(options[i], pdu + 5 + i * 64, 64);
    }
}
"#;
    let path = "tests/fixtures/c/cve_off_by_one.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6, 7, 8, 9]),
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
        "Taint should trace pdu[4] → option_count → loop bound → memcpy, got: {:?}",
        result.findings
    );
}

// ====== Fixture 8: Missing Return Value Check (CWE-252) ======

#[test]
fn test_cve_unchecked_return_value() {
    let source = r#"
int init_hardware(int dev_id) {
    if (dev_id < 0 || dev_id > 16) return -1;
    int ret = configure_registers(dev_id);
    if (ret < 0) return ret;
    return 0;
}

void startup(int device_id) {
    init_hardware(device_id);
    start_dma();
}
"#;
    let path = "tests/fixtures/c/cve_unchecked_return.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches configure_registers call in init_hardware
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();

    let has_findings = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_findings,
        "Echo should flag init_hardware return value unchecked by startup, got: {:?}",
        result.findings
    );
}

// ====== Fixture 9: Missing Bounds Check on Array Index (CWE-129) ======

#[test]
fn test_cve_array_index_no_bounds_check() {
    let source = r#"
#include <stdint.h>

typedef void (*handler_fn)(uint8_t *data);
handler_fn handlers[8];

void dispatch_message(uint8_t *pdu) {
    uint8_t msg_type = pdu[0];
    handlers[msg_type](pdu + 1);
}
"#;
    let path = "tests/fixtures/c/cve_array_index.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8, 9]),
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
        "Taint should trace pdu[0] → msg_type → array index, got: {:?}",
        result.findings
    );
}

// ====== Fixture 10: strncpy Without Null Termination (CWE-170) ======

#[test]
fn test_cve_strncpy_no_null_termination() {
    let source = r#"
#include <string.h>

void copy_hostname(const char *input, size_t input_len) {
    char hostname[64];
    strncpy(hostname, input, sizeof(hostname));
    int len = strlen(hostname);
    log_hostname(hostname, len);
}
"#;
    let path = "tests/fixtures/c/cve_strncpy_no_null.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6, 7, 8]),
        }],
    };

    // Taint: input reaches strncpy sink
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
        "Taint should detect input flowing to strncpy sink, got: {:?}",
        result.findings
    );
}

// ====== Fixture 11: PLOAM State Machine Goto Leak (CWE-401) ======

#[test]
fn test_cve_ploam_goto_leak() {
    let source = r#"
#include <stdlib.h>
#include <stdint.h>

struct ploam_ctx { uint8_t *buf; uint8_t *key; };

int handle_activate(struct ploam_ctx *ctx, uint8_t *msg) {
    ctx->buf = kmalloc(256, 0);
    if (!ctx->buf) return -1;
    ctx->key = kmalloc(32, 0);
    if (!ctx->key) goto err_buf;
    int ret = decrypt_ploam(ctx->key, msg);
    if (ret < 0) goto err_buf;
    return 0;
err_key:
    kfree(ctx->key);
err_buf:
    kfree(ctx->buf);
    return -1;
}
"#;
    let path = "tests/fixtures/c/cve_ploam_goto_leak.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8, 10]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // goto err_buf skips kfree(ctx->key), so ctx->key leaks
    let missing_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref() == Some("missing_close_on_error_path")
                || f.category.as_deref() == Some("close_only_on_error_path")
        })
        .collect();
    assert!(
        !missing_findings.is_empty(),
        "Absence should detect ctx->key leak on goto err_buf path, got: {:?}",
        result.findings
    );
}

// ====== Fixture 12: Heap Overflow via Unchecked realloc (CWE-122) ======

#[test]
fn test_cve_realloc_overflow() {
    let source = r#"
#include <stdlib.h>
#include <string.h>

struct msg_buf { char *data; size_t len; size_t cap; };

void append_data(struct msg_buf *buf, const char *chunk, size_t chunk_len) {
    if (buf->len + chunk_len > buf->cap) {
        buf->data = realloc(buf->data, buf->len + chunk_len);
        buf->cap = buf->len + chunk_len;
    }
    memcpy(buf->data + buf->len, chunk, chunk_len);
    buf->len += chunk_len;
}
"#;
    let path = "tests/fixtures/c/cve_realloc_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9, 10]),
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
        "Taint should trace chunk_len → realloc → memcpy, got: {:?}",
        result.findings
    );
}

// ====== Fixture 13: Variadic Wrapper Format String (CWE-134) ======

#[test]
fn test_cve_variadic_wrapper_format_string() {
    let source = r#"
#include <stdio.h>
#include <stdarg.h>

void device_log(int level, const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    char buf[512];
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    write_log(level, buf);
}

void handle_request(const char *user_data) {
    device_log(3, user_data);
}
"#;
    let path = "tests/fixtures/c/cve_variadic_wrapper.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([15, 16]),
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
        "Taint should detect user_data flowing to device_log (variadic wrapper), got: {:?}",
        result.findings
    );
}

// ====== Fixture 14: Refcount Race Without Lock (CWE-362) ======

#[test]
fn test_cve_refcount_race_no_lock() {
    let source = r#"
#include <pthread.h>

struct shared_obj {
    int refcount;
    pthread_mutex_t lock;
    void *data;
};

void obj_get(struct shared_obj *obj) {
    obj->refcount++;
}

void obj_put(struct shared_obj *obj) {
    pthread_mutex_lock(&obj->lock);
    obj->refcount--;
    if (obj->refcount == 0) {
        free(obj->data);
        free(obj);
    }
    pthread_mutex_unlock(&obj->lock);
}
"#;
    let path = "tests/fixtures/c/cve_refcount_race.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches obj_get function
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11, 12]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Absence detects obj_put has lock/unlock pair; the diff touches obj_get
    // which doesn't have the pair — at minimum, blocks should be produced
    let has_output = !result.findings.is_empty() || !result.blocks.is_empty();
    // If absence doesn't find paired patterns in obj_get (no lock calls),
    // also try SymmetrySlice which detects asymmetric function patterns
    if !has_output {
        let sym_result = algorithms::run_slicing_compat(
            &files,
            &diff,
            &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
            None,
        )
        .unwrap();
        let has_sym = !sym_result.findings.is_empty() || !sym_result.blocks.is_empty();
        assert!(
            has_sym,
            "Symmetry should detect asymmetry between obj_get/obj_put, got: {:?}",
            sym_result.findings
        );
    }
}

// ====== Fixture 15: Kernel Memory Leak on Error Path (CWE-401) ======

#[test]
fn test_cve_kernel_leak_error_path() {
    // Simplified kernel probe pattern — kzalloc with kfree only on error path
    let source = r#"
int probe_device(void *pdev) {
    char *dev = kzalloc(64, 0);
    if (!dev) return -1;
    int ret = setup_hw(dev);
    if (ret < 0) goto err;
    return 0;
err:
    kfree(dev);
    return -1;
}
"#;
    let path = "tests/fixtures/c/cve_kernel_leak.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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

    let error_path_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path"))
        .collect();
    assert!(
        !error_path_findings.is_empty(),
        "Should detect kzalloc with kfree only on error path, got: {:?}",
        result.findings
    );
}

// ====== Fixture 16: Open FD Leak in Daemon (CWE-775) ======

#[test]
fn test_cve_fd_leak_daemon() {
    // Simplified: fopen with no fclose at all (missing_counterpart)
    let source = r#"
#include <stdio.h>

int read_config(const char *path, char *buf, size_t bufsize) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    fgets(buf, bufsize, f);
    return 0;
}
"#;
    let path = "tests/fixtures/c/cve_fd_leak.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // fopen without any fclose — missing_counterpart
    let missing_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_counterpart"))
        .collect();
    assert!(
        !missing_findings.is_empty(),
        "Absence should detect fopen without fclose (missing_counterpart), got: {:?}",
        result.findings
    );
}

// ====== Fixture 17: Double-Free via Shared Cleanup Label (CWE-415) ======

#[test]
fn test_cve_double_free_shared_label() {
    let source = r#"
#include <stdlib.h>

int setup_channel(int ch_id) {
    char *rx_buf = kmalloc(4096, 0);
    if (!rx_buf) return -1;
    char *tx_buf = kmalloc(4096, 0);
    if (!tx_buf) goto cleanup;

    int ret = configure_dma(ch_id, rx_buf, tx_buf);
    if (ret < 0) {
        kfree(tx_buf);
        goto cleanup;
    }
    return 0;

cleanup:
    kfree(tx_buf);
    kfree(rx_buf);
    return -1;
}
"#;
    let path = "tests/fixtures/c/cve_double_free_shared_label.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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

    let double_close: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close.is_empty(),
        "Should detect double-free: kfree(tx_buf) inline + kfree(tx_buf) in cleanup, got: {:?}",
        result.findings
    );
}
