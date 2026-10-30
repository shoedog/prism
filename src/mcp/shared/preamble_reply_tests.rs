//! Tests for the replies, the line a shim receives: their exact wire text, the round trip,
//! and every way a line from another process can be wrong.
use super::*;
use crate::mcp::shared::identity::IdentityKey;

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";
const NEIGHBOUR: &str = "0123456789abcdef000000000000000000000000000000000000000000000000";
const READY_LINE: &str = r#"READY {"protocol":2,"identity":"0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210","pid":4242,"start_time":1791900090,"generation":3,"build":"building","index_idle_timeout_secs":0,"index_idle_pressure_relief":false}"#;

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

fn ready() -> Ready {
    Ready {
        identity: key(KEY),
        pid: 4242,
        start_time: 1_791_900_090,
        generation: 3,
        build: BuildState::Building,
        index_idle_timeout_secs: 0,
        index_idle_pressure_relief: false,
    }
}

#[test]
fn every_reply_is_written_exactly_as_the_spec_shows_it() {
    assert_eq!(Reply::Ready(ready()).encode(), format!("{READY_LINE}\n"));
    assert_eq!(Reply::Retiring.encode(), "RETIRING {}\n");
    assert_eq!(
        Reply::Busy { max_clients: 64 }.encode(),
        "BUSY {\"max_clients\":64}\n"
    );
    assert_eq!(
        Reply::Mismatch {
            expected: key(NEIGHBOUR)
        }
        .encode(),
        format!("MISMATCH {{\"expected\":\"{NEIGHBOUR}\"}}\n")
    );
}

/// Spec §9: "every reply variant".
#[test]
fn every_reply_survives_encode_then_decode() {
    let mut replies = vec![
        Reply::Retiring,
        Reply::Busy { max_clients: 0 },
        Reply::Busy {
            max_clients: u32::MAX,
        },
        Reply::Mismatch { expected: key(KEY) },
    ];
    for build in [
        BuildState::Idle,
        BuildState::Unloaded,
        BuildState::Building,
        BuildState::Ready,
        BuildState::Failed,
    ] {
        replies.push(Reply::Ready(Ready { build, ..ready() }));
    }
    replies.push(Reply::Ready(Ready {
        identity: key(NEIGHBOUR),
        pid: u32::MAX,
        start_time: u64::MAX,
        generation: u64::MAX,
        build: BuildState::Ready,
        index_idle_timeout_secs: 0,
        index_idle_pressure_relief: false,
    }));
    for reply in replies {
        let line = reply.encode();
        assert!(line.ends_with('\n') && line.len() <= PREAMBLE_MAX_BYTES);
        let decoded = Reply::decode(line.trim_end_matches('\n').as_bytes());
        assert_eq!(decoded, Ok(reply), "{line}");
    }
}

#[test]
fn build_states_are_spelled_as_the_spec_spells_them() {
    let spelled = |build| match Reply::Ready(Ready { build, ..ready() }).encode() {
        line if line.contains("\"build\":\"idle\"") => "idle",
        line if line.contains("\"build\":\"building\"") => "building",
        line if line.contains("\"build\":\"ready\"") => "ready",
        line if line.contains("\"build\":\"failed\"") => "failed",
        line => panic!("{line}"),
    };
    assert_eq!(spelled(BuildState::Idle), "idle");
    assert_eq!(spelled(BuildState::Building), "building");
    assert_eq!(spelled(BuildState::Ready), "ready");
    assert_eq!(spelled(BuildState::Failed), "failed");
}

#[test]
fn a_ready_of_another_protocol_version_is_refused_as_such() {
    let line = READY_LINE.replace(r#""protocol":2,"#, r#""protocol":3,"#);
    assert_eq!(
        Reply::decode(line.as_bytes()),
        Err(PreambleError::UnsupportedProtocol { got: 3 })
    );
}

#[test]
fn a_reply_that_is_malformed_in_any_way_is_refused() {
    type Expectation = fn(&PreambleError) -> bool;
    let cases: Vec<(String, Expectation)> = vec![
        ("READY {}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            READY_LINE.replace(r#","build":"building""#, ""),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("missing field `build`")),
        ),
        (
            READY_LINE.replace("building", "warming"),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("build")),
        ),
        (
            READY_LINE.replace("\"pid\":4242", "\"pid\":4294967296"),
            |e| matches!(e, PreambleError::Invalid { .. }),
        ),
        (
            READY_LINE.replace(KEY, &KEY[..16]),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("identity")),
        ),
        ("RETIRING".into(), |e| matches!(e, PreambleError::Malformed)),
        ("RETIRING {\"why\":1}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        ("RETIRING []".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            "BUSY {}".into(),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("max_clients")),
        ),
        ("BUSY {\"max_clients\":-1}".into(), |e| {
            matches!(e, PreambleError::Invalid { .. })
        }),
        (
            "MISMATCH {}".into(),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("expected")),
        ),
        (
            format!("MISMATCH {{\"expected\":\"{}\"}}", &KEY[..16]),
            |e| matches!(e, PreambleError::Invalid { detail } if detail.contains("expected")),
        ),
        ("WELCOME {}".into(), |e| {
            matches!(e, PreambleError::UnknownVerb { .. })
        }),
        ("READY {".into(), |e| {
            matches!(e, PreambleError::NotJson { .. })
        }),
        ("\u{fffd}".repeat(2000), |e| {
            matches!(e, PreambleError::TooLong { len: 6000 })
        }),
    ];
    for (line, expected) in cases {
        let error = Reply::decode(line.as_bytes()).expect_err(&line);
        assert!(expected(&error), "{line}: {error:?}");
    }
    assert_eq!(Reply::decode(b"READY \xff"), Err(PreambleError::NotUtf8));
}

#[test]
fn a_reply_with_a_field_this_version_does_not_define_is_refused_by_name() {
    let extra_field = |line: &str| match Reply::decode(line.as_bytes()) {
        Err(PreambleError::Invalid { detail }) => detail,
        other => panic!("{line}: {other:?}"),
    };
    for line in [
        "RETIRING {\"why\":1}".to_string(),
        "BUSY {\"max_clients\":64,\"why\":1}".to_string(),
        format!("MISMATCH {{\"expected\":\"{KEY}\",\"why\":1}}"),
        READY_LINE.replace(r#""pid":4242,"#, r#""pid":4242,"why":1,"#),
    ] {
        let detail = extra_field(&line);
        assert!(detail.contains("unknown field `why`"), "{line}: {detail}");
    }
}

/// Rules out a reply decoder without the one-spelling rule (each line below parses as JSON
/// into a valid reply).
#[test]
fn a_reply_in_any_spelling_other_than_the_encoder_s_is_refused() {
    let variants = [
        "RETIRING { }".to_string(),
        "RETIRING {} ".to_string(),
        "BUSY {\"max_clients\": 64}".to_string(),
        format!("MISMATCH {{\"expected\":\"{KEY}\"}}\r"),
        READY_LINE.replace(
            r#""pid":4242,"start_time":1791900090,"#,
            r#""start_time":1791900090,"pid":4242,"#,
        ),
    ];
    for line in variants {
        let outcome = Reply::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{line}: {outcome:?}"
        );
    }
    assert!(Reply::decode(READY_LINE.as_bytes()).is_ok());
    assert!(Reply::decode(b"BUSY {\"max_clients\":64}").is_ok());
}

/// A `HELLO` that comes back to a shim (it connected to something that echoes) is not a reply.
#[test]
fn a_hello_is_not_a_reply() {
    assert_eq!(
        Reply::decode(b"HELLO {\"protocol\":2}"),
        Err(PreambleError::UnknownVerb {
            verb: "HELLO".into()
        })
    );
}
