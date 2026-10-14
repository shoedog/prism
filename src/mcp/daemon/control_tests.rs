//! The rule that decides what a call is told about its own attempt's ending (D26, D32),
//! tested for every ending and every phase, without threads.
use super::*;

fn building() -> BuildPhase {
    BuildPhase::Building {
        attempt: 2,
        started: Instant::now(),
        outcome: Arc::new(AttemptOutcome::default()),
    }
}

fn failed(error: &str) -> BuildPhase {
    BuildPhase::Failed {
        attempt: 2,
        error: error.to_string(),
    }
}

fn every_phase() -> Vec<BuildPhase> {
    vec![
        BuildPhase::Idle,
        building(),
        BuildPhase::Ready,
        failed("a later attempt's error"),
    ]
}

/// An attempt that published the index is answered `Ready`, whatever the phase says now.
#[test]
fn an_attempt_that_published_is_answered_ready_in_every_phase() {
    for phase in every_phase() {
        assert_eq!(answer_for(&Ok(()), &phase), Answer::Ready, "{phase:?}");
    }
}

/// A failed attempt is superseded when a later attempt is building or the index is ready.
#[test]
fn a_failed_attempt_is_superseded_by_a_building_retry_and_by_a_ready_index() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(answer_for(&own, &building()), Answer::Superseded);
    assert_eq!(answer_for(&own, &BuildPhase::Ready), Answer::Superseded);
}

/// Otherwise the call is told its own attempt's failure, with its own text: never the text of
/// the attempt the phase names.
#[test]
fn a_failed_attempt_nothing_has_superseded_is_answered_with_its_own_text() {
    let own: Result<(), String> = Err("own error".to_string());
    assert_eq!(
        answer_for(&own, &failed("a later attempt's error")),
        Answer::Failed {
            error: "own error".to_string()
        }
    );
    assert_eq!(
        answer_for(&own, &BuildPhase::Idle),
        Answer::Failed {
            error: "own error".to_string()
        }
    );
}
