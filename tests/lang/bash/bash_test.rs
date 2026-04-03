#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_bash_basic_parsing() {
    let source = "#!/bin/bash\n\nmy_func() {\n    echo \"hello\"\n}\n\nmy_func\n";
    let path = "test.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();

    assert!(
        parsed.error_rate() < 0.1,
        "Bash file should parse cleanly, error rate: {}",
        parsed.error_rate()
    );

    let funcs = parsed.all_functions();
    let func_names: Vec<String> = funcs
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();
    assert!(
        func_names.contains(&"my_func".to_string()),
        "Should find my_func function, got: {:?}",
        func_names
    );
}

#[test]
fn test_bash_original_diff() {
    let (files, _, diff) = make_bash_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Bash code"
    );
}

#[test]
fn test_bash_parent_function() {
    let (files, _, diff) = make_bash_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include enclosing bash function"
    );
}

#[test]
fn test_bash_taint_eval() {
    // eval with variable input is a command injection sink
    let source =
        "#!/bin/bash\n\nprocess() {\n    local cmd=\"$1\"\n    eval \"$cmd\"\n}\n\nprocess \"$@\"\n";
    let path = "script.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag eval as a command injection sink"
    );
}

#[test]
fn test_bash_taint_sudo() {
    // sudo with variable input is a privilege escalation sink
    let source =
        "#!/bin/bash\n\nrun_as_root() {\n    local cmd=\"$1\"\n    sudo $cmd\n}\n\nrun_as_root \"$1\"\n";
    let path = "admin.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag sudo as a privilege escalation sink"
    );
}

#[test]
fn test_bash_provenance_read_input() {
    // read command is a user input origin
    let source =
        "#!/bin/bash\n\nget_input() {\n    read -r user_input\n    echo \"Got: $user_input\"\n}\n\nget_input\n";
    let path = "script.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_provenance = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_provenance,
        "Provenance should detect read as user input origin"
    );
}

#[test]
fn test_bash_absence_mktemp_without_cleanup() {
    // mktemp without rm is a temp file leak
    let source =
        "#!/bin/bash\n\ndo_work() {\n    local tmp=$(mktemp)\n    echo \"data\" > \"$tmp\"\n}\n\ndo_work\n";
    let path = "script.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([4, 5]),
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
        .any(|f| f.description.contains("mktemp") || f.description.contains("Temp file"));
    assert!(
        has_absence,
        "AbsenceSlice should flag mktemp without cleanup. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_bash_absence_mount_without_umount() {
    // mount without umount is a resource leak
    let source =
        "#!/bin/bash\n\nsetup() {\n    mount /dev/sda1 /mnt\n    echo \"mounted\"\n}\n\nsetup\n";
    let path = "mount.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        .any(|f| f.description.contains("mount") || f.description.contains("umount"));
    assert!(
        has_absence,
        "AbsenceSlice should flag mount without umount. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_bash_quantum_background_process() {
    // Background processes with & are async patterns
    let source =
        "#!/bin/bash\n\nworker() {\n    sleep 10 &\n    local pid=$!\n    wait $pid\n}\n\nworker\n";
    let path = "async.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();

    let has_quantum = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_quantum,
        "QuantumSlice should detect background processes with &"
    );
}

#[test]
fn test_bash_taint_unquoted_variable() {
    // Unquoted variable in command argument is the #1 shell injection vector
    let source =
        "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    cat $file\n    rm -rf /tmp/$file\n}\n\nprocess \"$@\"\n";
    let path = "unsafe.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_unquoted = result
        .findings
        .iter()
        .any(|f| f.description.contains("unquoted"));
    assert!(
        has_unquoted,
        "Taint should flag unquoted $file in command arguments. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_bash_taint_quoted_variable_safe() {
    // Quoted variable should NOT trigger unquoted warning
    let source =
        "#!/bin/bash\n\nprocess() {\n    local file=\"$1\"\n    cat \"$file\"\n    rm -rf \"/tmp/$file\"\n}\n\nprocess \"$@\"\n";
    let path = "safe.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_unquoted = result
        .findings
        .iter()
        .any(|f| f.description.contains("unquoted"));
    assert!(
        !has_unquoted,
        "Quoted variables should NOT trigger unquoted expansion warning"
    );
}

#[test]
fn test_bash_taint_exec_sink() {
    // exec as process replacement sink
    let source = "#!/bin/bash\n\nrun() {\n    local cmd=\"$1\"\n    exec $cmd\n}\n\nrun \"$@\"\n";
    let path = "exec.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag exec as a process replacement sink"
    );
}

#[test]
fn test_bash_provenance_curl_network() {
    // curl output is network-sourced (user input origin)
    let source =
        "#!/bin/bash\n\nfetch() {\n    data=$(curl -s \"$1\")\n    echo \"$data\"\n}\n\nfetch \"$@\"\n";
    let path = "fetch.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_provenance = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_provenance,
        "Provenance should detect curl output as network input origin"
    );
}

// ====== Busybox / Firmware Shell Tests ======

#[test]
fn test_bash_taint_mtd_write_sink() {
    // mtd write with variable input is a device-bricking risk
    let source =
        "#!/bin/sh\n\nflash_fw() {\n    local image=\"$1\"\n    mtd write \"$image\" firmware\n}\n\nflash_fw \"$@\"\n";
    let path = "sysupgrade.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag mtd write as a firmware flash sink"
    );
}

#[test]
fn test_bash_taint_uci_set_sink() {
    // uci set with variable input is persistent config injection
    let source =
        "#!/bin/sh\n\nset_wifi() {\n    local ssid=\"$1\"\n    uci set wireless.@wifi-iface[0].ssid=\"$ssid\"\n    uci commit wireless\n}\n\nset_wifi \"$@\"\n";
    let path = "wifi_config.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag uci set as a config injection sink"
    );
}

#[test]
fn test_bash_taint_iptables_sink() {
    // iptables with variable input is firewall bypass
    let source =
        "#!/bin/sh\n\nallow_port() {\n    local port=\"$1\"\n    iptables -A INPUT -p tcp --dport $port -j ACCEPT\n}\n\nallow_port \"$@\"\n";
    let path = "firewall.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag iptables as a firewall manipulation sink"
    );
}

#[test]
fn test_bash_taint_insmod_sink() {
    // insmod with variable input is a rootkit installation vector
    let source =
        "#!/bin/sh\n\nload_module() {\n    local mod=\"$1\"\n    insmod \"$mod\"\n}\n\nload_module \"$@\"\n";
    let path = "modules.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        "Taint analysis should flag insmod as a kernel module loading sink"
    );
}

#[test]
fn test_bash_absence_mtd_write_without_hash() {
    // mtd write without hash verification is a firmware integrity risk
    let source =
        "#!/bin/sh\n\nflash_fw() {\n    local image=\"$1\"\n    mtd write \"$image\" firmware\n}\n\nflash_fw \"$@\"\n";
    let path = "sysupgrade.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
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

    let has_absence = result
        .findings
        .iter()
        .any(|f| f.description.contains("mtd") || f.description.contains("hash"));
    assert!(
        has_absence,
        "AbsenceSlice should flag mtd write without hash verification. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_bash_absence_uci_set_without_commit() {
    // uci set without uci commit leaves config in limbo
    let source =
        "#!/bin/sh\n\nset_wifi() {\n    local ssid=\"$1\"\n    uci set wireless.@wifi-iface[0].ssid=\"$ssid\"\n}\n\nset_wifi \"$@\"\n";
    let path = "wifi_config.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
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

    let has_absence = result
        .findings
        .iter()
        .any(|f| f.description.contains("uci") || f.description.contains("commit"));
    assert!(
        has_absence,
        "AbsenceSlice should flag uci set without uci commit. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}
