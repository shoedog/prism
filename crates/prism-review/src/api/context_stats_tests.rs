//! CONTROLLED assembly and REAL isolated-session cases. No fixture result is used as its own guard.
use super::*;
use crate::algorithms::taint::{
    boundary::records::PhysicalSite,
    context_budget::{AdmissionBound, AttemptCounters, Published, SkipReason, StopCause},
    frontier::{CallProvenance, Disposition, FrontierState},
    result_union::{UnionError, UnionStats},
    rounds::{RoundStats, Stop},
};
use crate::api::context_run::UnionState;
use prism::{
    api::{ReviewHintBasis, ReviewImportHint},
    call_graph::FunctionId,
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "context_stats_tests/assembly.rs"]
mod assembly;
#[path = "context_stats_tests/profile.rs"]
mod profile;

fn context() -> ReviewContext {
    ReviewContext {
        files: BTreeMap::new(),
        sources: BTreeMap::new(),
        parse_quality: BTreeMap::new(),
        parse_warnings: Vec::new(),
        context_files: BTreeSet::from(["b.js".into()]),
        round: 2,
        published: Published {
            files: 2,
            bytes: 3000,
        },
        stats: RoundStats {
            rounds_started: 3,
            rounds_completed: 2,
            ..Default::default()
        },
        stop: Stop::FrontierExhausted,
        attempts: AttemptCounters {
            attempted_files: 5,
            attempted_read_bytes: 9000,
            skips: Vec::new(),
        },
        frontier: FrontierState::default(),
        union: UnionState::Applied {
            stats: UnionStats::default(),
            added_block_ids: vec![],
        },
    }
}
fn provenance(site: usize) -> CallProvenance {
    CallProvenance {
        site: PhysicalSite {
            caller: FunctionId {
                file: "main.js".into(),
                name: "main".into(),
                start_line: 1,
                end_line: 4,
            },
            start_byte: site,
            end_byte: site + 1,
        },
        line: 2,
        callee: "f".into(),
        language: Language::JavaScript,
        hint: ReviewImportHint::InRepo {
            files: vec!["b.js".into()],
            declaring: vec![],
            spec: "./b".into(),
            basis: ReviewHintBasis::Bound,
            member: Some("f".into()),
        },
        first_round: 0,
    }
}
