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
    pub(crate) binding_fence_calls: Option<u64>,
    pub(crate) shadow_check_calls: Option<u64>,
    pub(crate) def_scope_calls: Option<u64>,
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
    flow_search_ns: u64,
    pub(super) reaching_definitions_ns: u64,
    rd_cpu_ns: Option<u64>,
    peak_rss_bytes: Option<u64>,
    time_kind: &'static str,
    counters: Counters,
}

pub(crate) struct Owner {
    row: OwnerRecord,
    start: Instant,
    rd: Option<Instant>,
    rd_cpu: Option<u64>,
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
        let start = Instant::now();
        let mut ast_nodes = 0;
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
                            flow_search_ns: 0,
                            reaching_definitions_ns: 0,
                            rd_cpu_ns: None,
                            peak_rss_bytes: None,
                            time_kind: "parallel_work",
                            counters: Counters::default(),
                        },
                        start,
                        rd: None,
                        rd_cpu: None,
                    });
                }
            }
        }
    }
    pub(crate) fn reaching(owner: &mut Option<Self>, definitions: usize, assignments: usize) {
        if let Some(owner) = owner {
            owner.row.definitions = definitions;
            owner.row.assignment_paths = assignments;
            owner.row.flow_edges_ns = ns(owner.start);
            owner.rd = Some(Instant::now());
            owner.rd_cpu = cpu_ns(true);
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.row.total_ns = ns(self.start);
        self.row.reaching_definitions_ns = self.rd.map(ns).unwrap_or(0);
        self.row.rd_cpu_ns = delta(self.rd_cpu, cpu_ns(true));
        self.row.peak_rss_bytes = ru_maxrss_bytes();
        self.row.counters = COUNTERS.with(|c| c.borrow_mut().take().unwrap_or_default());
        self.row.flow_search_ns = self.row.counters.search_ns;
        if let Some(profile) = collector() {
            if let Ok(mut p) = profile.lock() {
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
        flow_search_ns: 0,
        reaching_definitions_ns: 0,
        rd_cpu_ns: None,
        peak_rss_bytes: None,
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
                wall_ns: 0,
                cpu_ns: Some(0),
                peak_rss_bytes: None,
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
            wall_ns: 0,
            cpu_ns: Some(0),
            peak_rss_bytes: None,
        });
        p.status = "measured";
        p.wall_ns += owner.reaching_definitions_ns;
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
