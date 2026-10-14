//! Tests for one connection's first-call budget: one deadline per connection per build attempt
//! (spec §4.2), and the ending of the attempt the connection last joined, which its next call
//! hears if no call of its own has. The clock is an argument, so nothing here waits.
use super::super::test_support::client_settings;
use super::*;

fn budget_of(wait_secs: u64) -> ConnectionBudget {
    ConnectionBudget::new(&client_settings(wait_secs))
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// A fresh, empty outcome for an attempt. What a budget keeps of it is tested below; the
/// deadline does not depend on it.
fn cell() -> Arc<AttemptOutcome> {
    Arc::new(AttemptOutcome::default())
}

/// Rules out a fresh deadline for every call: queued calls would then multiply one attempt's
/// wait.
#[test]
fn the_first_call_of_an_attempt_sets_the_deadline_and_later_calls_keep_it() {
    let start = Instant::now();
    let mut budget = budget_of(20);

    assert_eq!(budget.join(1, &cell(), start), start + secs(20));
    assert_eq!(
        budget.join(1, &cell(), start + secs(5)),
        start + secs(20),
        "a later call in the same attempt has only the rest of the budget"
    );
    assert_eq!(
        budget.join(1, &cell(), start + secs(500)),
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

    assert_eq!(budget.join(1, &cell(), start), start + secs(20));
    assert_eq!(
        budget.join(2, &cell(), start + secs(30)),
        start + secs(50),
        "counted from the first call that arrives during the new attempt"
    );
    assert_eq!(budget.join(2, &cell(), start + secs(45)), start + secs(50));
}

#[test]
fn each_connection_has_its_own_deadline_for_the_same_attempt() {
    let start = Instant::now();
    let mut a = budget_of(20);
    let mut b = budget_of(60);

    assert_eq!(a.join(1, &cell(), start), start + secs(20));
    assert_eq!(b.join(1, &cell(), start + secs(10)), start + secs(70));
    assert_eq!(
        a.join(1, &cell(), start + secs(10)),
        start + secs(20),
        "B joining the attempt changed nothing for A"
    );
}

#[test]
fn a_wait_of_zero_has_expired_the_moment_it_is_set() {
    let start = Instant::now();
    let mut budget = budget_of(0);

    assert_eq!(budget.join(1, &cell(), start), start);
}

#[test]
fn the_longest_wait_a_client_may_ask_for_is_kept_whole() {
    let start = Instant::now();
    let longest = crate::mcp::FIRST_CALL_WAIT_MAX;
    let mut budget = budget_of(longest.as_secs());

    assert_eq!(budget.join(1, &cell(), start), start + longest);
}

/// A connection whose calls for an attempt were all answered warming hears how it ended on its
/// next call, once. Rules out a budget that forgets the attempt it joined, and one that
/// reports the same failure on every call.
#[test]
fn a_failure_no_call_was_told_of_is_reported_once() {
    let mut budget = budget_of(0);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    assert_eq!(
        budget.unheard_failure(),
        None,
        "the attempt is still building"
    );

    attempt.fill(Err("it failed".to_string()));
    assert_eq!(budget.unheard_failure(), Some("it failed".to_string()));
    assert_eq!(budget.unheard_failure(), None, "and only once");
}

/// A call that was told how its attempt ended has heard it: nothing is reported again.
#[test]
fn a_failure_a_call_was_told_of_is_not_reported_again() {
    let mut budget = budget_of(600);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    attempt.fill(Err("it failed".to_string()));
    budget.told();

    assert_eq!(budget.unheard_failure(), None);
}

#[test]
fn an_attempt_that_published_the_index_is_never_an_unheard_failure() {
    let mut budget = budget_of(0);
    let attempt = cell();
    budget.join(1, &attempt, Instant::now());
    attempt.fill(Ok(()));

    assert_eq!(budget.unheard_failure(), None);
}

/// What is remembered is the attempt joined last; joining a new one starts it unheard.
#[test]
fn joining_a_new_attempt_remembers_that_attempt_and_not_the_old_one() {
    let mut budget = budget_of(0);
    let first = cell();
    budget.join(1, &first, Instant::now());
    first.fill(Err("first".to_string()));
    budget.told();

    let second = cell();
    budget.join(2, &second, Instant::now());
    second.fill(Err("second".to_string()));
    assert_eq!(budget.unheard_failure(), Some("second".to_string()));
}
