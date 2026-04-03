// Shared test helpers and fixture generators used across test files.

#![allow(dead_code)]

pub use prism::access_path::AccessPath;
pub use prism::algorithms;
pub use prism::ast::ParsedFile;
pub use prism::call_graph::CallGraph;
pub use prism::cpg::CpgContext;
pub use prism::data_flow::DataFlowGraph;
pub use prism::diff::{DiffInfo, DiffInput, ModifyType};
pub use prism::languages::Language;
pub use prism::output;
pub use prism::output::{to_review_output, MultiReviewOutput};
pub use prism::slice::{MultiSliceResult, SliceConfig, SliceFinding, SlicingAlgorithm};
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::path::Path;
pub use tempfile::TempDir;

pub fn make_python_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import os

GLOBAL_VAR = 42

def calculate(x, y):
    total = x + y
    if total > 10:
        result = total * 2
        print(result)
    else:
        result = total
    return result

def helper(val):
    return val + GLOBAL_VAR

def process(data):
    filtered = [d for d in data if d > 0]
    total = calculate(filtered[0], filtered[1])
    extra = helper(total)
    return extra
"#;

    let path = "src/calc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: lines 7 (total = x + y) and 9 (result = total * 2) were changed
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 9]),
        }],
    };

    (files, sources, diff)
}

pub fn make_javascript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
function fetchData(url, options) {
    const headers = options.headers || {};
    const timeout = options.timeout || 5000;

    if (timeout > 10000) {
        throw new Error("Timeout too long");
    }

    const response = fetch(url, { headers, timeout });
    const data = response.json();

    if (data.error) {
        console.error(data.error);
        return null;
    }

    return data.result;
}

function processItems(items) {
    const results = [];
    for (const item of items) {
        const processed = fetchData(item.url, item.options);
        if (processed) {
            results.push(processed);
        }
    }
    return results;
}
"#;

    let path = "src/api.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11]),
        }],
    };

    (files, sources, diff)
}

pub fn make_go_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"package main

import "fmt"

func sum(numbers []int) int {
	total := 0
	for _, n := range numbers {
		if n > 0 {
			total += n
		}
	}
	return total
}

func main() {
	data := []int{1, -2, 3, -4, 5}
	result := sum(data)
	fmt.Println(result)
}
"#;

    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    (files, sources, diff)
}

pub fn make_java_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"public class Calculator {
    private int accumulator = 0;

    public int add(int a, int b) {
        int sum = a + b;
        accumulator += sum;
        return sum;
    }

    public int getAccumulator() {
        return accumulator;
    }

    public void reset() {
        accumulator = 0;
    }
}
"#;

    let path = "Calculator.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    (files, sources, diff)
}

pub fn make_typescript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
interface Config {
    baseUrl: string;
    retries: number;
}

function createClient(config: Config) {
    const url = config.baseUrl;
    const maxRetries = config.retries;

    async function request(path: string): Promise<any> {
        let attempts = 0;
        while (attempts < maxRetries) {
            attempts += 1;
            try {
                const response = await fetch(url + path);
                return response.json();
            } catch (e) {
                if (attempts >= maxRetries) throw e;
            }
        }
    }

    return { request };
}
"#;

    let path = "src/client.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14, 16, 17]),
        }],
    };

    (files, sources, diff)
}

// ── Shared fixture helpers ──

pub fn make_c_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_BUF_SIZE 256

typedef struct {
    char *name;
    int id;
    int active;
} device_t;

device_t *create_device(const char *name, int id) {
    device_t *dev = malloc(sizeof(device_t));
    if (dev == NULL) {
        return NULL;
    }
    dev->name = strdup(name);
    dev->id = id;
    dev->active = 1;
    return dev;
}

void destroy_device(device_t *dev) {
    if (dev != NULL) {
        free(dev->name);
        free(dev);
    }
}

int process_packet(const char *buf, size_t len) {
    char local_buf[MAX_BUF_SIZE];
    int result = 0;

    memcpy(local_buf, buf, len);
    local_buf[len] = '\0';

    if (strlen(local_buf) > 10) {
        result = atoi(local_buf);
    }

    return result;
}

int handle_request(const char *input, size_t input_len) {
    device_t *dev = create_device(input, 42);
    if (dev == NULL) {
        return -1;
    }

    int status = process_packet(input, input_len);

    if (status < 0) {
        return status;
    }

    destroy_device(dev);
    return status;
}

void bulk_process(const char **inputs, int count) {
    for (int i = 0; i < count; i++) {
        int result = handle_request(inputs[i], strlen(inputs[i]));
        if (result < 0) {
            fprintf(stderr, "Error processing input %d\n", i);
        }
    }
}
"#;

    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: process_packet function modified (lines 34-44: the buffer handling code)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([36, 37, 38]),
        }],
    };

    (files, sources, diff)
}

pub fn make_c_multifile_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let device_source = r#"
#include "device.h"
#include <stdlib.h>
#include <string.h>

device_t *create_device(const char *name, int id) {
    device_t *dev = malloc(sizeof(device_t));
    dev->name = strdup(name);
    dev->id = id;
    return dev;
}

void destroy_device(device_t *dev) {
    free(dev->name);
    free(dev);
}

int get_device_status(device_t *dev) {
    return dev->active;
}
"#;

    let handler_source = r#"
#include "device.h"
#include <stdio.h>

int handle_create(const char *name) {
    device_t *dev = create_device(name, 1);
    int status = get_device_status(dev);
    printf("Device status: %d\n", status);
    return status;
}

int handle_batch(const char **names, int count) {
    for (int i = 0; i < count; i++) {
        handle_create(names[i]);
    }
    return 0;
}
"#;

    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();

    let dev_parsed = ParsedFile::parse("src/device.c", device_source, Language::C).unwrap();
    let handler_parsed = ParsedFile::parse("src/handler.c", handler_source, Language::C).unwrap();

    files.insert("src/device.c".to_string(), dev_parsed);
    files.insert("src/handler.c".to_string(), handler_parsed);
    sources.insert("src/device.c".to_string(), device_source.to_string());
    sources.insert("src/handler.c".to_string(), handler_source.to_string());

    // Diff: create_device modified (return type change, error handling change)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/device.c".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8, 9]),
        }],
    };

    (files, sources, diff)
}

pub fn make_cpp_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
#include <string>
#include <vector>
#include <memory>
#include <mutex>
#include <stdexcept>

class DeviceManager {
private:
    std::vector<std::string> devices;
    std::mutex mtx;
    int max_devices;

public:
    DeviceManager(int max) : max_devices(max) {}

    ~DeviceManager() {
        devices.clear();
    }

    bool add_device(const std::string& name) {
        std::lock_guard<std::mutex> lock(mtx);
        if (devices.size() >= max_devices) {
            return false;
        }
        devices.push_back(name);
        return true;
    }

    std::string get_device(int index) {
        if (index < 0 || index >= devices.size()) {
            throw std::out_of_range("Invalid device index");
        }
        return devices[index];
    }

    int count() const {
        return devices.size();
    }

    std::string serialize() {
        std::string result = "{";
        for (size_t i = 0; i < devices.size(); i++) {
            result += "\"" + devices[i] + "\"";
            if (i < devices.size() - 1) {
                result += ",";
            }
        }
        result += "}";
        return result;
    }
};

int process_devices(DeviceManager& mgr, const std::vector<std::string>& names) {
    int added = 0;
    for (const auto& name : names) {
        if (mgr.add_device(name)) {
            added++;
        }
    }
    return added;
}
"#;

    let path = "src/device_manager.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: add_device and get_device methods modified
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([23, 24, 25, 33, 34]),
        }],
    };

    (files, sources, diff)
}

pub fn make_snmp_overflow_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>
#include <string.h>

void handle_snmp_set(uint8_t *pdu, size_t pdu_len) {
    char community[64];
    size_t community_len = pdu[7];
    memcpy(community, pdu + 8, community_len);
    if (strcmp(community, "public") == 0) {
        process_set_request(pdu + 8 + community_len, pdu_len - 8 - community_len);
    }
}
"#;

    let path = "tests/fixtures/c/snmp_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 7-8 (community_len extraction and memcpy without bounds check)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 8]),
        }],
    };

    (files, diff)
}

pub fn make_double_free_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    uint8_t *payload;
    size_t len;
} frame_t;

void process_frame(uint8_t *raw, size_t len) {
    frame_t *frame = malloc(sizeof(frame_t));
    frame->payload = malloc(len);
    memcpy(frame->payload, raw, len);

    if (validate_header(frame) < 0) {
        free(frame->payload);
        free(frame);
        goto cleanup;
    }

    dispatch_frame(frame);
    return;

cleanup:
    free(frame->payload);
    free(frame);
}
"#;

    let path = "tests/fixtures/c/double_free.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the cleanup label and double free
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([17, 18, 25, 26]),
        }],
    };

    (files, diff)
}

pub fn make_ring_overflow_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdint.h>
#include <string.h>

#define RING_SIZE 256
static uint8_t ring_buf[RING_SIZE];
static volatile int write_idx = 0;

void ring_write(uint8_t *data, int count) {
    memcpy(ring_buf + write_idx, data, count);
    write_idx += count;
}
"#;

    let path = "tests/fixtures/c/ring_overflow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the memcpy and write_idx update (no bounds check)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11]),
        }],
    };

    (files, diff)
}

pub fn make_timer_uaf_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = r#"
#include <stdlib.h>

struct timer_ctx {
    void (*callback)(void *);
    void *data;
    int active;
};

void cancel_timer(struct timer_ctx *timer) {
    timer->active = 0;
    free(timer->data);
}

void timer_tick(struct timer_ctx *timer) {
    if (timer->active) {
        timer->callback(timer->data);
    }
}
"#;

    let path = "tests/fixtures/c/timer_uaf.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: the free(timer->data) line (potential UAF)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([11, 12]),
        }],
    };

    (files, diff)
}

pub fn make_large_function_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("../fixtures/c/large_function.c");

    let path = "tests/fixtures/c/large_function.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: line 131 (sum += ch->buf[i])
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([131]),
        }],
    };

    (files, diff)
}

pub fn make_deep_switch_test() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let source = include_str!("../fixtures/c/deep_switch.c");

    let path = "tests/fixtures/c/deep_switch.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff: lines 66-67 (bounds check for msg->len)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([66, 67]),
        }],
    };

    (files, diff)
}

pub fn create_temp_git_repo(filename: &str, contents: &[&str]) -> TempDir {
    let tmp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    for (i, content) in contents.iter().enumerate() {
        std::fs::write(tmp.path().join(filename), content).unwrap();
        std::process::Command::new("git")
            .args(["add", filename])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", &format!("commit {}", i)])
            .current_dir(tmp.path())
            .output()
            .unwrap();
    }

    tmp
}

pub fn make_terraform_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
variable "allowed_cidrs" {
  description = "CIDRs allowed to access the service"
  type        = list(string)
}

variable "instance_type" {
  default = "t3.micro"
}

locals {
  merged_cidrs = concat(var.allowed_cidrs, ["10.0.0.0/8"])
  env_name     = "production"
}

resource "aws_security_group" "web" {
  name = "web-${local.env_name}"

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = local.merged_cidrs
  }
}

resource "aws_instance" "web" {
  ami           = "ami-0123456789abcdef0"
  instance_type = var.instance_type
  user_data     = "startup-script"

  vpc_security_group_ids = [aws_security_group.web.id]
}

output "instance_ip" {
  value = aws_instance.web.public_ip
}
"#;

    let path = "main.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: the cidr_blocks line and user_data line were changed
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([24, 32]),
        }],
    };

    (files, sources, diff)
}

pub fn make_bash_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = "#!/bin/bash\n\nLOG_DIR=\"/var/log/myapp\"\n\nsetup_dirs() {\n    mkdir -p \"$LOG_DIR\"\n    chmod 755 \"$LOG_DIR\"\n}\n\nprocess_input() {\n    local input=\"$1\"\n    local output_file=\"$2\"\n\n    if [ -z \"$input\" ]; then\n        echo \"Error: no input\" >&2\n        return 1\n    fi\n\n    cat \"$input\" > \"$output_file\"\n    echo \"Processed: $input\"\n}\n\ncleanup() {\n    local tmpfile=$(mktemp)\n    echo \"cleaning up\" > \"$tmpfile\"\n    rm -f \"$tmpfile\"\n}\n\nmain() {\n    setup_dirs\n    process_input \"$1\" \"$2\"\n    cleanup\n}\n\nmain \"$@\"\n";

    let path = "scripts/deploy.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([19, 20]),
        }],
    };

    (files, sources, diff)
}
