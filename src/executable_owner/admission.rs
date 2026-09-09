//! Observations only. Never deserialized or consulted by an admission predicate.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Phase {
    LoadInputs,
    SelectInputs,
    PrepareInputs,
    LocateInputs,
    CompilerEvidence,
    OwnerMapping,
    SessionBuild,
}

const PHASES: [&str; 7] = [
    "load_inputs",
    "select_inputs",
    "prepare_inputs",
    "locate_inputs",
    "compiler_evidence",
    "owner_mapping",
    "session_build",
];

pub(super) struct Report {
    pub phase: Phase,
    inputs: Option<serde_json::Value>,
}

/// Internal error custody. Text is for humans; MCP receives the original value,
/// never a JSON substring recovered from a truncated/user-controlled message.
#[derive(Debug)]
pub(crate) struct Failure {
    reason: String,
    report: serde_json::Value,
}

impl Failure {
    pub(crate) fn message(&self) -> String {
        format!("owner acquisition failed: {}", self.reason)
    }

    pub(crate) fn diagnostic(&self) -> serde_json::Value {
        self.report.clone()
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}; owner_admission={}", self.message(), self.report)
    }
}
impl std::error::Error for Failure {}

impl Report {
    pub fn new() -> Self {
        Self {
            phase: Phase::LoadInputs,
            inputs: None,
        }
    }

    pub fn observe(&mut self, repo: &crate::repo_loader::LoadedRepo) {
        let mut languages = BTreeMap::<String, usize>::new();
        let mut js_ts_files = 0;
        let mut js_ts_bytes = Some(0usize);
        for parsed in repo.files.values() {
            *languages
                .entry(format!("{:?}", parsed.language))
                .or_default() += 1;
            if js(parsed.language) {
                js_ts_files += 1;
                js_ts_bytes = js_ts_bytes.and_then(|n| n.checked_add(parsed.source.len()));
            }
        }
        self.inputs = Some(serde_json::json!({
            "loaded_files":repo.files.len(), "js_ts_files":js_ts_files,
            "js_ts_bytes":js_ts_bytes, "languages":languages,
            "type_database_present":repo.type_db.is_some(),
            "file_limit":INPUT_FILE_LIMIT, "byte_limit":INPUT_BYTE_LIMIT,
            "within_file_limit":repo.files.len() <= INPUT_FILE_LIMIT,
            "within_byte_limit":js_ts_bytes.map(|n| n <= INPUT_BYTE_LIMIT),
        }));
    }

    pub fn refusal(&self, reason: String) -> Failure {
        let current = self.phase as usize;
        let phases: BTreeMap<_, _> = PHASES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    *name,
                    match i.cmp(&current) {
                        std::cmp::Ordering::Less => "completed",
                        std::cmp::Ordering::Equal => "failed",
                        std::cmp::Ordering::Greater => "not_reached",
                    },
                )
            })
            .collect();
        let report = serde_json::json!({
            "schema":"prism.owner-admission/1", "authorizes_runtime_edge":false,
            "reason":reason, "failed_phase":PHASES[current],
            "inputs":self.inputs, "phases":phases,
        });
        Failure { reason, report }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_counts_use_loaded_bytes_and_type_database_presence_without_source_leakage() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("private_name.ts"), "// λ\n").unwrap();
        std::fs::write(root.path().join("other.py"), "def f(): pass\n").unwrap();
        let mut loaded = crate::repo_loader::load_repo(root.path()).unwrap();
        for present in [false, true] {
            loaded.type_db = present.then(crate::type_db::TypeDatabase::default);
            let mut report = Report::new();
            report.observe(&loaded);
            report.phase = Phase::SelectInputs;
            let failure = report.refusal("owner_requires_js_ts_only".into());
            let value = failure.diagnostic();
            assert_eq!(value["inputs"]["loaded_files"], 2);
            assert_eq!(value["inputs"]["js_ts_files"], 1);
            assert_eq!(value["inputs"]["js_ts_bytes"], "// λ\n".len());
            assert_eq!(value["inputs"]["type_database_present"], present);
            assert_eq!(
                value["inputs"]["languages"],
                serde_json::json!({"Python":1,"TypeScript":1})
            );
            let text = failure.to_string();
            assert!(!text.contains("private_name") && !text.contains('λ'));
            assert!(serde_json::to_vec(&value).unwrap().len() < 2048);
        }
    }

    #[test]
    fn every_phase_reports_only_prior_completion_and_preserves_unknown_inputs() {
        for (current, phase) in [
            Phase::LoadInputs,
            Phase::SelectInputs,
            Phase::PrepareInputs,
            Phase::LocateInputs,
            Phase::CompilerEvidence,
            Phase::OwnerMapping,
            Phase::SessionBuild,
        ]
        .into_iter()
        .enumerate()
        {
            let mut report = Report::new();
            report.phase = phase;
            let error = report.refusal("test_refusal".into()).to_string();
            let value: serde_json::Value =
                serde_json::from_str(error.split_once("; owner_admission=").unwrap().1).unwrap();
            assert!(value["inputs"].is_null());
            assert_eq!(value["authorizes_runtime_edge"], false);
            assert_eq!(value["failed_phase"], PHASES[current]);
            for (i, name) in PHASES.iter().enumerate() {
                assert_eq!(
                    value["phases"][name],
                    if i < current {
                        "completed"
                    } else if i == current {
                        "failed"
                    } else {
                        "not_reached"
                    }
                );
            }
        }
    }
}
