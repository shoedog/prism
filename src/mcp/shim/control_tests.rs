//! Tests for one control exchange (S5b; spec §4.7): what is sent, how each answer is sorted, and
//! that nothing is sent to a peer of another user.
use super::super::control_support::{always_line, fake_status, ControlFake, FakeReply};
use super::super::shim_support::{another_user, runtime, this_user, HANG};
use super::*;
use crate::mcp::shared::preamble::Reply;
use crate::mcp::shared::runtime_dir::current_uid;
use std::sync::Arc;
use std::time::Duration;

fn ask(socket: &Path, request: &ControlRequest, within: Duration) -> Exchanged {
    exchange(
        socket,
        request,
        Instant::now() + within,
        this_user,
        current_uid(),
    )
}

/// The request goes out as `encode` spells it and a status reply comes back decoded.
#[test]
fn a_status_reply_is_read_and_the_request_sent_as_encoded() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(fake_status(1, false).encode()));
    match ask(paths.socket(), &ControlRequest::Status, HANG) {
        Exchanged::Status(status) => assert_eq!(status, fake_status(1, false)),
        other => panic!("{other:?}"),
    }
    assert_eq!(fake.lines(), vec![ControlRequest::Status.encode()]);
}

/// `RETIRING` and `BUSY` are answers of their own, not failures to answer.
#[test]
fn retiring_and_busy_are_answers_of_their_own() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(Reply::Retiring.encode()));
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::Retiring
    ));
    fake.retire();
    let _fake = ControlFake::start(
        &paths,
        true,
        always_line(Reply::Busy { max_clients: 3 }.encode()),
    );
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::Busy { max_clients: 3 }
    ));
}

/// The peer check comes before anything is written (D78, as the shim does it): a listener of
/// another user at this user's socket path learns nothing. Rules out a check made after the
/// request is sent.
#[test]
fn a_peer_of_another_user_is_refused_before_anything_is_sent() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, always_line(fake_status(0, false).encode()));
    let answered = exchange(
        paths.socket(),
        &ControlRequest::Status,
        Instant::now() + HANG,
        another_user,
        current_uid(),
    );
    assert!(
        matches!(answered, Exchanged::Refused(Refusal::ForeignPeer { .. })),
        "{answered:?}"
    );
    super::super::shim_support::wait_until("the fake to see the connection end", || {
        !fake.lines().is_empty()
    });
    assert_eq!(fake.lines(), vec![String::new()], "nothing was sent");
}

/// A daemon that closes without a reply (one that does not know the control exchange closes the
/// line it refuses, S4's D54) and one that never answers are both no reply, the second at the
/// deadline; nothing listening is told apart.
#[test]
fn a_close_a_silence_and_nothing_listening_are_told_apart() {
    let (_base, paths) = runtime();
    let fake = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Close));
    assert!(matches!(
        ask(paths.socket(), &ControlRequest::Status, HANG),
        Exchanged::NoReply(Transient::Closed)
    ));
    fake.retire();
    let _silent = ControlFake::start(&paths, true, Arc::new(|_| FakeReply::Silent));
    assert!(matches!(
        ask(
            paths.socket(),
            &ControlRequest::Status,
            Duration::from_millis(300)
        ),
        Exchanged::NoReply(Transient::NoReply)
    ));
    let (_other_base, absent) = runtime();
    assert!(matches!(
        ask(absent.socket(), &ControlRequest::Status, HANG),
        Exchanged::NotListening(_)
    ));
}

/// A reply this build cannot read, or a preamble reply that does not answer a control request,
/// is refused. Rules out reading a `READY` as a status.
#[test]
fn an_unreadable_or_unexpected_reply_is_refused() {
    let (_base, paths) = runtime();
    for reply in [
        "STATUS {\"control\":2}\n".to_string(),
        "garbage\n".to_string(),
        super::super::shim_support::ready_line(super::super::shim_support::key(), 1),
    ] {
        let fake = ControlFake::start(&paths, false, always_line(reply.clone()));
        let answered = ask(paths.socket(), &ControlRequest::Status, HANG);
        assert!(
            matches!(answered, Exchanged::Refused(Refusal::BadReply(_))),
            "{reply:?}: {answered:?}"
        );
        fake.retire();
    }
}
