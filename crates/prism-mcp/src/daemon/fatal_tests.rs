//! The guard that turns a panic in a domain into the daemon's end.
use super::*;
use std::sync::Mutex;

fn recorder() -> (OnFatal, Arc<Mutex<Vec<Fatal>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let on_fatal: OnFatal = Arc::new(move |fatal| sink.lock().expect("log").push(fatal));
    (on_fatal, log)
}

fn events(log: &Mutex<Vec<Fatal>>) -> Vec<Fatal> {
    log.lock().expect("log").clone()
}

#[test]
fn code_that_returns_is_not_fatal() {
    let (on_fatal, log) = recorder();
    {
        let _fatal = FatalOnUnwind::new(&on_fatal, Domain::Dispatch);
    }
    assert_eq!(events(&log), vec![]);
}

#[test]
fn a_panic_while_the_guard_is_alive_is_fatal_in_its_domain() {
    let (on_fatal, log) = recorder();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _fatal = FatalOnUnwind::new(&on_fatal, Domain::Control);
        panic!("injected panic in the control plane");
    }));
    assert!(unwound.is_err());
    assert_eq!(events(&log), vec![Fatal::Panicked(Domain::Control)]);
}

/// A guard made in a destructor that runs during another panic does not claim that panic.
/// Rules out a guard that looks only at `thread::panicking()` when it is dropped.
#[test]
fn a_guard_made_while_the_thread_is_already_unwinding_does_not_fire() {
    struct MakesAGuardWhenDropped {
        on_fatal: OnFatal,
    }
    impl Drop for MakesAGuardWhenDropped {
        fn drop(&mut self) {
            let _fatal = FatalOnUnwind::new(&self.on_fatal, Domain::Build);
        }
    }
    let (on_fatal, log) = recorder();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _cleanup = MakesAGuardWhenDropped { on_fatal };
        panic!("a panic outside every domain");
    }));
    assert!(unwound.is_err());
    assert_eq!(events(&log), vec![]);
}

#[test]
fn the_daemon_s_last_line_names_where_it_failed() {
    assert_eq!(
        Fatal::Panicked(Domain::Dispatch).to_string(),
        "prism-mcp daemon: a panic in provider dispatch; the daemon ends so that no client is \
         served from what the panic left"
    );
    assert_eq!(
        Fatal::Poisoned { lock: "control" }.to_string(),
        "prism-mcp daemon: the control lock is poisoned; the daemon ends so that no client is \
         served from what the panic left"
    );
}
