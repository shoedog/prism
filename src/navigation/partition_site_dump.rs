use crate::call_graph::CallSite;
use crate::go_owner_partition::GoOwnerPartitionTelemetry;
use crate::resolution::ResolutionOutcome;

fn partition_site_json(
    site: &CallSite,
    outcome: &ResolutionOutcome<'_>,
    partition: GoOwnerPartitionTelemetry,
) -> serde_json::Value {
    let decision = if partition.drops > 0 {
        "drop"
    } else if partition.recovered > 0 {
        "recovered"
    } else {
        "none"
    };
    let targets: Vec<_> = outcome
        .resolved
        .iter()
        .map(|resolved| {
            serde_json::json!({
                "target": resolved.target,
                "kind": resolved.kind.as_str(),
                "confidence": resolved.confidence,
            })
        })
        .collect();
    serde_json::json!({
        "caller": site.caller,
        "span": {
            "line": site.line,
            "start_byte": site.start_byte,
            "end_byte": site.end_byte,
        },
        "callee": site.callee_name,
        "targets": targets,
        "drop_reason": outcome.drop.map(|reason| format!("{reason:?}")),
        "partition": {
            "decision": decision,
            "drops": partition.drops,
            "recovered": partition.recovered,
            "affected_edges": partition.affected_edges,
        },
    })
}

/// Opt-in developer custody stream for corpus-delta adjudication. Normal
/// `call-stats` stdout is unchanged; affected sites are emitted as JSONL on
/// stderr only when `PRISM_CALL_STATS_PARTITION_SITES=1`.
pub(super) fn emit_if_enabled(
    site: &CallSite,
    outcome: &ResolutionOutcome<'_>,
    partition: GoOwnerPartitionTelemetry,
) {
    if std::env::var("PRISM_CALL_STATS_PARTITION_SITES").as_deref() != Ok("1")
        || partition.affected_sites() == 0
    {
        return;
    }
    eprintln!("{}", partition_site_json(site, outcome, partition));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::{CallKind, CallSiteOrigin, FunctionId};
    use crate::resolution::{
        DropReason, ResolutionConfidence, ResolutionKind, ResolutionTelemetry,
    };

    #[test]
    fn partition_site_json_carries_edge_custody() {
        let caller = FunctionId {
            file: "app/use.go".to_string(),
            name: "invoke".to_string(),
            start_line: 10,
            end_line: 12,
        };
        let target = FunctionId {
            file: "lib/impl.go".to_string(),
            name: "Act".to_string(),
            start_line: 20,
            end_line: 22,
        };
        let site = CallSite {
            caller: caller.clone(),
            callee_name: "h.Act".to_string(),
            line: 11,
            kind: CallKind::Call,
            start_byte: 101,
            end_byte: 108,
            qualifier: Some("h".to_string()),
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_recovery: None,
            receiver_materialized: false,
            arg_count: Some(0),
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::Source,
            pre_resolved_target: None,
        };
        let outcome = ResolutionOutcome {
            resolved: vec![crate::resolution::ResolvedCallee {
                target: &target,
                confidence: ResolutionConfidence::Exact,
                kind: ResolutionKind::InterfaceDispatch,
            }],
            drop: None,
            telemetry: ResolutionTelemetry::default(),
        };
        let dump = partition_site_json(
            &site,
            &outcome,
            GoOwnerPartitionTelemetry {
                drops: 0,
                recovered: 1,
                affected_edges: 1,
            },
        );

        assert_eq!(dump["caller"], serde_json::json!(caller));
        assert_eq!(
            dump["span"],
            serde_json::json!({"line": 11, "start_byte": 101, "end_byte": 108})
        );
        assert_eq!(dump["callee"], "h.Act");
        assert_eq!(dump["partition"]["decision"], "recovered");
        assert_eq!(dump["targets"][0]["target"], serde_json::json!(target));
        assert_eq!(dump["targets"][0]["kind"], "interface_dispatch");
        assert_eq!(dump["targets"][0]["confidence"], "Exact");

        let dropped = partition_site_json(
            &site,
            &ResolutionOutcome {
                resolved: Vec::new(),
                drop: Some(DropReason::ExternalReceiver),
                telemetry: ResolutionTelemetry::default(),
            },
            GoOwnerPartitionTelemetry {
                drops: 1,
                recovered: 0,
                affected_edges: 2,
            },
        );
        assert_eq!(dropped["partition"]["decision"], "drop");
        assert_eq!(dropped["partition"]["affected_edges"], 2);
        assert_eq!(dropped["drop_reason"], "ExternalReceiver");
        assert_eq!(dropped["targets"], serde_json::json!([]));
    }
}
