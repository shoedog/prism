use super::{install, per_item};
use rayon::prelude::*;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

#[test]
fn adjacent_jobs_are_independently_stealable() {
    install(|| {
        let workers = rayon::current_num_threads();
        if workers == 1 {
            // Blocking one job on another requires a second worker. The edge
            // case test below still exercises the helper on a one-worker pool.
            return;
        }
        // Default thief-splitting leaves several adjacent items in its first
        // sequential fold. Item 1 cannot be stolen out of that fold while 0
        // waits. Singleton leaves leave item 1 available to another worker.
        let count = workers.next_power_of_two() * 8;
        let (started, receiver) = mpsc::channel();
        let receiver = Mutex::new(receiver);
        let outcomes: Vec<_> = per_item(0..count)
            .map(|item| {
                if item == 1 {
                    started.send(()).unwrap();
                }
                let unblocked = item != 0
                    || receiver
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(2))
                        .is_ok();
                (item, unblocked)
            })
            .collect();
        assert!(outcomes[0].1, "item 1 was trapped behind blocked item 0");
        assert_eq!(
            outcomes,
            (0..count).map(|item| (item, true)).collect::<Vec<_>>()
        );
    });
}

#[test]
fn ordered_collection_handles_empty_singleton_and_owned_jobs() {
    install(|| {
        let empty: Vec<usize> = per_item(Vec::new()).collect();
        assert!(empty.is_empty());
        assert_eq!(per_item([7]).collect::<Vec<_>>(), vec![7]);
        let jobs = vec!["z".to_owned(), "a".to_owned(), "m".to_owned()];
        assert_eq!(per_item(&jobs).cloned().collect::<Vec<_>>(), jobs);
        assert_eq!(per_item(jobs).collect::<Vec<_>>(), ["z", "a", "m"]);
    });
}
