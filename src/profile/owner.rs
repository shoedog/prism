use super::*;
use crate::ast::ParsedFile;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Default, Serialize)]
pub(crate) struct Counters {
    pub(crate) simple_searches: u64,
    pub(crate) member_searches: u64,
    pub(crate) searched_bytes: u64,
    pub(crate) binding_fence_calls: u64,
    pub(crate) shadow_check_calls: u64,
    pub(crate) def_scope_calls: u64,
    #[serde(skip)]
    search_ns: u64,
}
thread_local! { pub(crate) static COUNTERS: RefCell<Option<Counters>> = const { RefCell::new(None) }; }

#[derive(Serialize)]
pub(crate) struct OwnerRecord {
    record: &'static str,
    pub(super) file: String,
    pub(super) owner: String,
    pub(super) start_line: usize,
    pub(super) start_byte: usize,
    language: String,
    byte_size: usize,
    ast_nodes: usize,
    definitions: usize,
    assignment_paths: usize,
    pub(super) total_ns: u64,
    flow_edges_ns: u64,
    file_cfg_init_ns: u64,
    file_cfg_rss_rise_bytes: Option<u64>,
    rd_rss_rise_bytes: Option<u64>,
    flow_search_ns: u64,
    pub(super) reaching_definitions_ns: u64,
    rd_cpu_ns: Option<u64>,
    peak_rss_bytes: Option<u64>,
    rss_rise_bytes: Option<u64>,
    time_kind: &'static str,
    counters: Counters,
}

pub(crate) struct Owner {
    row: OwnerRecord,
    start: Instant,
    rd: Option<Instant>,
    rd_cpu: Option<u64>,
    rd_rss: Option<u64>,
    rss: Option<u64>,
}

impl Owner {
    pub(crate) fn begin(
        file: &str,
        parsed: &ParsedFile,
        node: Node<'_>,
        name: &str,
    ) -> Option<Self> {
        if !enabled() {
            return None;
        }
        let mut ast_nodes = 0;
        #[cfg(test)]
        tests::metadata_delay();
        let mut cursor = node.walk();
        loop {
            ast_nodes += 1;
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    COUNTERS.with(|c| *c.borrow_mut() = Some(Counters::default()));
                    return Some(Self {
                        row: OwnerRecord {
                            record: "owner",
                            file: file.to_owned(),
                            owner: name.to_owned(),
                            start_line: node.start_position().row + 1,
                            start_byte: node.start_byte(),
                            language: format!("{:?}", parsed.language),
                            byte_size: node.end_byte() - node.start_byte(),
                            ast_nodes,
                            definitions: 0,
                            assignment_paths: 0,
                            total_ns: 0,
                            flow_edges_ns: 0,
                            file_cfg_init_ns: 0,
                            file_cfg_rss_rise_bytes: Some(0),
                            rd_rss_rise_bytes: None,
                            flow_search_ns: 0,
                            reaching_definitions_ns: 0,
                            rd_cpu_ns: None,
                            peak_rss_bytes: None,
                            rss_rise_bytes: None,
                            time_kind: "parallel_work",
                            counters: Counters::default(),
                        },
                        start: Instant::now(),
                        rss: ru_maxrss_bytes(),
                        rd: None,
                        rd_cpu: None,
                        rd_rss: None,
                    });
                }
            }
        }
    }
    pub(crate) fn reaching(owner: &mut Option<Self>, definitions: usize, assignments: usize) {
        if let Some(owner) = owner {
            owner.row.definitions = definitions;
            owner.row.assignment_paths = assignments;
            owner.row.flow_edges_ns = ns(owner.start).saturating_sub(owner.row.file_cfg_init_ns);
            owner.rd = Some(Instant::now());
            owner.rd_cpu = cpu_ns(true);
            owner.rd_rss = ru_maxrss_bytes();
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.row.total_ns = ns(self.start).saturating_sub(self.row.file_cfg_init_ns);
        self.row.reaching_definitions_ns = self.rd.map(ns).unwrap_or(0);
        self.row.rd_cpu_ns = delta(self.rd_cpu, cpu_ns(true));
        self.row.peak_rss_bytes = ru_maxrss_bytes();
        self.row.rd_rss_rise_bytes = delta(self.rd_rss, self.row.peak_rss_bytes);
        self.row.rss_rise_bytes = delta(self.rss, self.row.peak_rss_bytes)
            .zip(self.row.file_cfg_rss_rise_bytes)
            .map(|(rise, cfg)| rise.saturating_sub(cfg));
        self.row.counters = COUNTERS.with(|c| c.borrow_mut().take().unwrap_or_default());
        self.row.flow_search_ns = self.row.counters.search_ns;
        if let Ok(mut slot) = collector().lock() {
            if let Some(p) = slot.as_mut() {
                // Move the record without cloning counters or owner metadata.
                let row = std::mem::replace(&mut self.row, empty_record());
                p.owners.push(row);
            }
        }
    }
}

fn empty_record() -> OwnerRecord {
    OwnerRecord {
        record: "owner",
        file: String::new(),
        owner: String::new(),
        start_line: 0,
        start_byte: 0,
        language: String::new(),
        byte_size: 0,
        ast_nodes: 0,
        definitions: 0,
        assignment_paths: 0,
        total_ns: 0,
        flow_edges_ns: 0,
        file_cfg_init_ns: 0,
        file_cfg_rss_rise_bytes: Some(0),
        rd_rss_rise_bytes: None,
        flow_search_ns: 0,
        reaching_definitions_ns: 0,
        rd_cpu_ns: None,
        peak_rss_bytes: None,
        rss_rise_bytes: None,
        time_kind: "parallel_work",
        counters: Counters::default(),
    }
}

pub(super) fn file_rd(owners: &[OwnerRecord], phases: &[PhaseRecord]) -> Vec<PhaseRecord> {
    let mut files = BTreeMap::<&str, PhaseRecord>::new();
    for file in phases
        .iter()
        .filter(|p| p.name == "dfg.file")
        .filter_map(|p| p.file.as_deref())
    {
        files.insert(
            file,
            PhaseRecord {
                record: "phase",
                name: "dfg.file_rd",
                file: Some(file.to_owned()),
                time_kind: "parallel_work",
                aggregation: "sum_owner_intervals",
                status: "not_run",
                wall_ns: None,
                work_ns: Some(0),
                cpu_ns: Some(0),
                peak_rss_bytes: None,
                rss_rise_bytes: Some(0),
            },
        );
    }
    for owner in owners {
        let p = files.entry(&owner.file).or_insert_with(|| PhaseRecord {
            record: "phase",
            name: "dfg.file_rd",
            file: Some(owner.file.clone()),
            time_kind: "parallel_work",
            aggregation: "sum_owner_intervals",
            status: "measured",
            wall_ns: None,
            work_ns: Some(0),
            cpu_ns: Some(0),
            peak_rss_bytes: None,
            rss_rise_bytes: Some(0),
        });
        p.status = "measured";
        *p.work_ns.as_mut().unwrap() += owner.reaching_definitions_ns;
        p.rss_rise_bytes = match (p.rss_rise_bytes, owner.rd_rss_rise_bytes) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };
        p.cpu_ns = match (p.cpu_ns, owner.rd_cpu_ns) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };
        p.peak_rss_bytes = p.peak_rss_bytes.max(owner.peak_rss_bytes);
    }
    files.into_values().collect()
}

/// Counts and clocks only an owner search, never individual visited nodes.
pub(crate) fn search<T>(simple: bool, bytes: usize, run: impl FnOnce() -> T) -> T {
    if !enabled() {
        return run();
    }
    let start = Instant::now();
    let result = run();
    COUNTERS.with(|cell| {
        if let Some(c) = cell.borrow_mut().as_mut() {
            if simple {
                c.simple_searches += 1;
            } else {
                c.member_searches += 1;
            }
            c.searched_bytes += bytes as u64;
            c.search_ns += ns(start);
        }
    });
    result
}

/// Time only the actual lazy initialization; run before opening the RD clock.
pub(crate) fn file_cfg<T>(owner: &mut Option<Owner>, file: &str, run: impl FnOnce() -> T) -> T {
    let mut phase = Phase::file("dfg.file_cfg", file);
    let result = run();
    if let (Some(owner), Some(phase)) = (owner.as_mut(), phase.as_mut()) {
        owner.row.file_cfg_init_ns += ns(phase.started);
        owner.row.file_cfg_rss_rise_bytes = phase.rss_rise();
    }
    drop(phase);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    thread_local! { static DELAY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    pub(super) fn metadata_delay() {
        if DELAY.with(|v| v.get()) {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
    #[test]
    fn owner_clock_excludes_metadata_work() {
        const CHILD: &str = "PRISM_METADATA_CLOCK_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let session = Session::start();
            let parsed = ParsedFile::parse(
                "a.js",
                "function f() { return 0; }",
                crate::languages::Language::JavaScript,
            )
            .unwrap();
            DELAY.with(|v| v.set(true));
            let observed = Instant::now();
            let owner = Owner::begin("a.js", &parsed, parsed.all_functions()[0], "f");
            assert!(owner.is_some());
            drop(owner);
            let elapsed = ns(observed);
            assert!(elapsed >= 40_000_000, "delay witness did not execute");
            let slot = collector().lock().unwrap();
            let row = &slot.as_ref().unwrap().owners[0];
            assert!(
                row.total_ns < elapsed / 2,
                "metadata work charged to owner: {} / {elapsed}",
                row.total_ns
            );
            drop(slot);
            drop(session);
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "profile::owner::tests::owner_clock_excludes_metadata_work",
            ])
            .env(CHILD, "1")
            .env("PRISM_PROFILE", dir.path().join("profile.jsonl"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
