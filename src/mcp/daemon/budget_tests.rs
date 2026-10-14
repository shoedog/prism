//! Tests for one connection's first-call budget: one deadline per connection per build attempt
//! (spec §4.2). The clock is an argument, so nothing here waits.
use super::super::test_support::client_settings;
use super::*;

fn budget_of(wait_secs: u64) -> ConnectionBudget {
    ConnectionBudget::new(&client_settings(wait_secs))
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// Rules out a fresh deadline for every call: queued calls would then multiply one attempt's
/// wait.
#[test]
fn the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.deadline_for(1, start), start + secs(20));
    assert_eq!(
        budget.deadline_for(1, start + secs(5)),
        start + secs(20),
        "a later call in the same attempt has only the rest of the budget"
    );
    assert_eq!(
        budget.deadline_for(1, start + secs(500)),
        start + secs(20),
        "and none once it has passed"
    );
}

/// Rules out one deadline for the connection's whole life: after a failed build the retry
/// would be answered with warming at once.
#[test]
fn a_new_attempt_gets_a_fresh_full_budget() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.deadline_for(1, start), start + secs(20));
    assert_eq!(
        budget.deadline_for(2, start + secs(30)),
        start + secs(50),
        "counted from the first call that arrives during the new attempt"
    );
    assert_eq!(budget.deadline_for(2, start + secs(45)), start + secs(50));
}

#[test]
fn each_connection_has_its_own_deadline_for_the_same_attempt() {
    let start = Instant::now();
    let mut a = budget_of(20);
    let mut b = budget_of(60);

    assert_eq!(a.deadline_for(1, start), start + secs(20));
    assert_eq!(b.deadline_for(1, start + secs(10)), start + secs(70));
    assert_eq!(
        a.deadline_for(1, start + secs(10)),
        start + secs(20),
        "B joining the attempt changed nothing for A"
    );
}

#[test]
fn a_wait_of_zero_has_expired_the_moment_it_is_set() {
    let start = Instant::now();
    let mut budget = budget_of(0);

    assert_eq!(budget.deadline_for(1, start), start);
}

#[test]
fn the_longest_wait_a_client_may_ask_for_is_kept_whole() {
    let start = Instant::now();
    let longest = crate::mcp::FIRST_CALL_WAIT_MAX;
    let mut budget = budget_of(longest.as_secs());

    assert_eq!(budget.deadline_for(1, start), start + longest);
}
