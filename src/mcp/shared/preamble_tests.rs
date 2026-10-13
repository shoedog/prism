//! Tests for `HELLO`, the line a daemon receives: its exact wire text, the round trip, every
//! way a line from another process can be wrong, and the identity comparison. The replies'
//! tests are in `preamble_reply_tests.rs`.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::{StructuredContentMode, MAX_RESULT_CHARS_FLOOR};
use crate::mcp::settings::{ClientSettings, ConnectionSettings, SettingsError};
use crate::mcp::shared::identity::IdentityKey;

const KEY: &str = "0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210";
/// Same first 16 digits as `KEY`, so the same socket file; a different identity.
const NEIGHBOUR: &str = "0123456789abcdef000000000000000000000000000000000000000000000000";

/// The `HELLO` line of spec §4.4 for the default settings, without its newline.
const HELLO_LINE: &str = r#"HELLO {"protocol":1,"identity":"0123456789abcdeffedcba98765432100123456789abcdeffedcba9876543210","settings":{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}}"#;

fn key(hex: &str) -> IdentityKey {
    IdentityKey::from_hex(hex).unwrap()
}

fn settings(
    structured_content: StructuredContentMode,
    concise_shape: ConciseShapeMode,
    result_cap: usize,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
) -> ClientSettings {
    ClientSettings::new(
        ConnectionSettings {
            result_cap,
            structured_content,
            concise_shape,
        },
        first_call_wait_secs,
        warm_at_startup,
    )
    .unwrap()
}

fn default_hello() -> Hello {
    Hello {
        identity: key(KEY),
        settings: settings(
            StructuredContentMode::OmitDefaultPath,
            ConciseShapeMode::Slim,
            80_000,
            20,
            false,
        ),
    }
}

fn invalid_detail(outcome: Result<Hello, PreambleError>) -> String {
    match outcome {
        Err(PreambleError::Invalid { detail }) => detail,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

// ── Wire text ──

#[test]
fn hello_is_written_exactly_as_the_spec_shows_it() {
    assert_eq!(default_hello().encode(), format!("{HELLO_LINE}\n"));
}

#[test]
fn the_settings_use_the_same_words_as_the_environment_variables() {
    use crate::mcp::concise_shape::resolve_concise_shape_mode_from;
    use crate::mcp::output::resolve_structured_content_mode_from;
    for mode in [
        StructuredContentMode::Always,
        StructuredContentMode::OmitDefaultPath,
    ] {
        let word = structured_content_word(mode);
        assert_eq!(resolve_structured_content_mode_from(Some(word)), mode);
    }
    for mode in [ConciseShapeMode::Legacy, ConciseShapeMode::Slim] {
        let word = concise_shape_word(mode);
        assert_eq!(resolve_concise_shape_mode_from(Some(word)), mode);
    }
}

// ── Round trips ──

/// Spec §9 "preamble encode/decode": every combination of the settings survives encode then
/// decode, including the extremes of the two numeric settings.
#[test]
fn every_hello_survives_encode_then_decode() {
    for structured_content in [
        StructuredContentMode::Always,
        StructuredContentMode::OmitDefaultPath,
    ] {
        for concise_shape in [ConciseShapeMode::Legacy, ConciseShapeMode::Slim] {
            for (result_cap, wait) in [(MAX_RESULT_CHARS_FLOOR, 0), (80_000, 20), (usize::MAX, 600)]
            {
                for warm_at_startup in [false, true] {
                    let hello = Hello {
                        identity: key(NEIGHBOUR),
                        settings: settings(
                            structured_content,
                            concise_shape,
                            result_cap,
                            wait,
                            warm_at_startup,
                        ),
                    };
                    let line = hello.encode();
                    assert!(line.ends_with('\n') && line.len() <= PREAMBLE_MAX_BYTES);
                    let decoded = Hello::decode(line.trim_end_matches('\n').as_bytes());
                    assert_eq!(decoded, Ok(hello), "{line}");
                }
            }
        }
    }
}

// ── What a daemon must refuse in a HELLO ──

/// Rules out decoding first and measuring afterwards, and an off-by-one at the limit. The
/// oversized line is well-formed apart from its length, so only the length can refuse it.
#[test]
fn a_line_over_4_kib_is_refused_for_its_length_alone() {
    let pad_to = |len: usize| {
        let mut line = HELLO_LINE.as_bytes().to_vec();
        line.resize(len, b' ');
        line
    };
    assert_eq!(
        Hello::decode(&pad_to(PREAMBLE_MAX_BYTES + 1)),
        Err(PreambleError::TooLong {
            len: PREAMBLE_MAX_BYTES + 1
        })
    );
    // At the limit the length is fine; the padding is what is wrong with this one.
    let at_limit = Hello::decode(&pad_to(PREAMBLE_MAX_BYTES));
    assert!(
        matches!(at_limit, Err(PreambleError::Invalid { .. })),
        "{at_limit:?}"
    );
    assert_eq!(PREAMBLE_MAX_BYTES, 4096);
}

#[test]
fn bytes_that_are_not_utf8_are_refused() {
    let mut line = HELLO_LINE.as_bytes().to_vec();
    line[20] = 0xff;
    assert_eq!(Hello::decode(&line), Err(PreambleError::NotUtf8));
    assert_eq!(Hello::decode(b"\xff\xfe"), Err(PreambleError::NotUtf8));
}

#[test]
fn a_line_that_is_not_a_verb_and_a_payload_is_refused() {
    for line in ["", "HELLO", "HELLO{}", "\tHELLO {}"] {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(
                outcome,
                Err(PreambleError::Malformed | PreambleError::UnknownVerb { .. })
            ),
            "{line:?}: {outcome:?}"
        );
    }
}

/// An MCP client that connects to the socket directly, or a reply sent in the wrong direction,
/// is answered as an unknown verb, and a long first word is not echoed back in full.
#[test]
fn a_verb_the_receiver_does_not_expect_is_refused() {
    let jsonrpc = r#"{"jsonrpc":"2.0","id":1,"method": "initialize"}"#;
    for line in [
        "READY {\"protocol\":1}",
        "RETIRING {}",
        "hello {}",
        "HELLO2 {}",
        " HELLO {}",
        jsonrpc,
    ] {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(&outcome, Err(PreambleError::UnknownVerb { verb }) if verb.len() <= 16),
            "{line:?}: {outcome:?}"
        );
    }
}

#[test]
fn a_payload_that_is_not_json_is_refused() {
    for payload in [
        "{",
        "{\"protocol\":1",
        "{'protocol':1}",
        "{\"protocol\":1} {}",
        "{\"protocol\":1}}",
    ] {
        let line = format!("HELLO {payload}");
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::NotJson { .. })),
            "{line:?}: {outcome:?}"
        );
    }
}

/// The payload is a JSON object and nothing else. Rules out a decoder that takes the
/// positional array form its JSON library would also accept for a struct.
#[test]
fn a_payload_that_is_not_a_json_object_is_refused() {
    let positional = format!(r#"[1,"{KEY}",["omit-default-path","slim",80000,20,false]]"#);
    for payload in [
        "",
        "not json",
        "[]",
        "5",
        "null",
        "\"HELLO\"",
        " {}",
        &positional,
    ] {
        let line = format!("HELLO {payload}");
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(detail.contains("JSON object"), "{line:?}: {detail}");
    }
}

/// Each of the eight fields, removed one at a time.
#[test]
fn a_hello_with_any_field_missing_is_refused_and_the_field_is_named() {
    let key_field = format!(r#""identity":"{KEY}","#);
    let removals = [
        ("identity", key_field.as_str()),
        (
            "settings",
            r#","settings":{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}"#,
        ),
        (
            "structured_content",
            r#""structured_content":"omit-default-path","#,
        ),
        ("concise_shape", r#""concise_shape":"slim","#),
        ("result_cap", r#""result_cap":80000,"#),
        ("first_call_wait_secs", r#""first_call_wait_secs":20,"#),
        ("warm_at_startup", r#","warm_at_startup":false"#),
    ];
    for (field, text) in removals {
        assert_eq!(HELLO_LINE.matches(text).count(), 1, "{field}");
        let line = HELLO_LINE.replace(text, "");
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.contains(&format!("missing field `{field}`")),
            "{field}: {detail}"
        );
    }
    let no_protocol = HELLO_LINE.replace(r#""protocol":1,"#, "");
    let detail = invalid_detail(Hello::decode(no_protocol.as_bytes()));
    assert!(detail.contains("missing field `protocol`"), "{detail}");
}

/// A version this build does not speak. The version is read before anything else, because
/// another version's payload may have another shape: the last two lines are nonsense as
/// version 1 and must still be reported as a version difference, not as a missing field.
#[test]
fn a_hello_of_another_protocol_version_is_refused_as_such() {
    for (got, line) in [
        (
            2,
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":2,"#),
        ),
        (
            0,
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":0,"#),
        ),
        (2, r#"HELLO {"protocol":2}"#.to_string()),
        (
            7,
            r#"HELLO {"protocol":7,"id":"k","opts":{"result_cap":1}}"#.to_string(),
        ),
    ] {
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::UnsupportedProtocol { got }),
            "{line}"
        );
    }
    for bad in [
        r#""protocol":"1","#,
        r#""protocol":-1,"#,
        r#""protocol":1.5,"#,
    ] {
        let line = HELLO_LINE.replace(r#""protocol":1,"#, bad);
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{line}: {outcome:?}"
        );
    }
}

/// A result cap below the floor, and the other bounded setting. Rules out copying
/// the wire numbers into settings without the validating constructor.
#[test]
fn a_hello_asking_for_settings_a_server_must_not_run_with_is_refused() {
    for cap in [0, 100, MAX_RESULT_CHARS_FLOOR - 1] {
        let line = HELLO_LINE.replace("\"result_cap\":80000", &format!("\"result_cap\":{cap}"));
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::Settings(
                SettingsError::ResultCapBelowFloor {
                    cap,
                    floor: MAX_RESULT_CHARS_FLOOR
                }
            )),
            "{line}"
        );
    }
    let at_floor = HELLO_LINE.replace("\"result_cap\":80000", "\"result_cap\":12000");
    let decoded = Hello::decode(at_floor.as_bytes()).expect("the floor itself is allowed");
    assert_eq!(decoded.settings.output().result_cap, 12_000);

    for secs in [601_u64, u64::MAX] {
        let line = HELLO_LINE.replace(
            "\"first_call_wait_secs\":20",
            &format!("\"first_call_wait_secs\":{secs}"),
        );
        assert_eq!(
            Hello::decode(line.as_bytes()),
            Err(PreambleError::Settings(
                SettingsError::FirstCallWaitTooLong {
                    secs,
                    max_secs: 600
                }
            )),
            "{line}"
        );
    }
}

/// Unlike the environment variables, where an unknown word warns and falls back to the
/// default, an unknown word from a client is refused and the setting is named: the daemon has
/// no stderr the client sees.
#[test]
fn a_hello_with_an_unknown_word_for_a_setting_is_refused_and_the_setting_is_named() {
    let swaps = [
        (
            "structured_content",
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":"sometimes""#,
        ),
        (
            "structured_content",
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":"ALWAYS""#,
        ),
        (
            "concise_shape",
            r#""concise_shape":"slim""#,
            r#""concise_shape":"wide""#,
        ),
        (
            "concise_shape",
            r#""concise_shape":"slim""#,
            r#""concise_shape":"""#,
        ),
    ];
    for (setting, from, to) in swaps {
        assert_eq!(HELLO_LINE.matches(from).count(), 1);
        let line = HELLO_LINE.replace(from, to);
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.starts_with(&format!("{setting} is not")),
            "{to}: {detail}"
        );
    }
}

#[test]
fn a_hello_with_a_value_of_the_wrong_kind_is_refused() {
    let swaps = [
        (
            r#""structured_content":"omit-default-path""#,
            r#""structured_content":true"#,
        ),
        (r#""result_cap":80000"#, r#""result_cap":"80000""#),
        (r#""result_cap":80000"#, r#""result_cap":-80000"#),
        (r#""result_cap":80000"#, r#""result_cap":80000.5"#),
        (
            r#""result_cap":80000"#,
            r#""result_cap":99999999999999999999999"#,
        ),
        (r#""result_cap":80000"#, r#""result_cap":null"#),
        (
            r#""first_call_wait_secs":20"#,
            r#""first_call_wait_secs":"20""#,
        ),
        (r#""warm_at_startup":false"#, r#""warm_at_startup":"false""#),
        (r#""warm_at_startup":false"#, r#""warm_at_startup":0"#),
    ];
    for (from, to) in swaps {
        assert_eq!(HELLO_LINE.matches(from).count(), 1);
        let line = HELLO_LINE.replace(from, to);
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(outcome, Err(PreambleError::Invalid { .. })),
            "{to}: {outcome:?}"
        );
    }
}

#[test]
fn a_hello_whose_identity_is_not_a_full_key_is_refused() {
    let upper = KEY.to_uppercase();
    for identity in [&KEY[..16], "", &upper, &format!("{KEY}00"), "not-a-key"] {
        let line = HELLO_LINE.replace(KEY, identity);
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(detail.contains("identity"), "{identity:?}: {detail}");
    }
}

/// A field this version does not define is refused by name, in the message and in its
/// settings. (The one-spelling rule below would refuse these lines too; this pins the reason.)
#[test]
fn a_hello_with_a_field_this_version_does_not_define_is_refused_by_name() {
    for (field, line) in [
        (
            "extra",
            HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"extra":true,"#),
        ),
        (
            "debug",
            HELLO_LINE.replace(
                r#""warm_at_startup":false"#,
                r#""warm_at_startup":false,"debug":1"#,
            ),
        ),
    ] {
        let detail = invalid_detail(Hello::decode(line.as_bytes()));
        assert!(
            detail.contains(&format!("unknown field `{field}`")),
            "{detail}"
        );
    }
}

/// One message has one spelling: the one `encode` writes. Rules out a decoder that accepts
/// whatever its JSON library accepts (extra fields, repeated fields, the positional array form
/// of the nested settings, other key orders, added whitespace, other number spellings).
#[test]
fn a_hello_in_any_spelling_other_than_the_encoder_s_is_refused() {
    let variants = [
        HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"extra":true,"#),
        HELLO_LINE.replace(r#""warm_at_startup":false"#, r#""warm_at_startup":false,"debug":1"#),
        HELLO_LINE.replace(
            r#""result_cap":80000,"#,
            r#""result_cap":80000,"result_cap":90000,"#,
        ),
        HELLO_LINE.replace(r#""protocol":1,"#, r#""protocol":1,"protocol":1,"#),
        HELLO_LINE.replace(
            r#"{"structured_content":"omit-default-path","concise_shape":"slim","result_cap":80000,"first_call_wait_secs":20,"warm_at_startup":false}"#,
            r#"["omit-default-path","slim",80000,20,false]"#,
        ),
        HELLO_LINE.replace(
            r#""concise_shape":"slim","result_cap":80000,"#,
            r#""result_cap":80000,"concise_shape":"slim","#,
        ),
        HELLO_LINE.replace(r#"{"protocol":1,"#, r#"{ "protocol":1,"#),
        HELLO_LINE.replace(r#""result_cap":80000"#, r#""result_cap":080000"#),
        format!("{HELLO_LINE} "),
        format!("{HELLO_LINE}\r"),
        HELLO_LINE.replacen("HELLO ", "HELLO  ", 1),
    ];
    for line in variants {
        let outcome = Hello::decode(line.as_bytes());
        assert!(
            matches!(
                outcome,
                Err(PreambleError::Invalid { .. } | PreambleError::NotJson { .. })
            ),
            "{line}: {outcome:?}"
        );
    }
    assert!(Hello::decode(HELLO_LINE.as_bytes()).is_ok());
}

// ── The identity comparison ──

/// Spec §4.2 "Refusal" and §4.4: the daemon compares the full identity, not the 16 hex digits
/// in its file names. `NEIGHBOUR` has `KEY`'s first 16 digits, so a shim for it finds `KEY`'s
/// socket. Rules out comparing locators, or only a prefix.
#[test]
fn a_hello_for_a_neighbouring_identity_is_a_mismatch_that_names_the_served_key() {
    let served = key(KEY);
    assert_eq!(key(NEIGHBOUR).locator(), served.locator());

    let mut hello = default_hello();
    assert_eq!(hello.mismatch_against(&served), None);

    hello.identity = key(NEIGHBOUR);
    assert_eq!(
        hello.mismatch_against(&served),
        Some(Reply::Mismatch { expected: served })
    );

    let last_digit_differs = format!("{}1", &KEY[..63]);
    hello.identity = key(&last_digit_differs);
    assert_eq!(
        hello.mismatch_against(&served),
        Some(Reply::Mismatch { expected: served })
    );
}

#[test]
fn every_error_prints_on_one_bounded_line() {
    let long_verb = format!("{} {{}}", "V".repeat(3000));
    let errors = [
        Hello::decode(&[b' '; 5000]).unwrap_err(),
        Hello::decode(b"\xff").unwrap_err(),
        Hello::decode(b"HELLO").unwrap_err(),
        Hello::decode(long_verb.as_bytes()).unwrap_err(),
        Hello::decode(format!("HELLO {}", "[".repeat(3000)).as_bytes()).unwrap_err(),
        Hello::decode(format!("HELLO {{{}", "[".repeat(3000)).as_bytes()).unwrap_err(),
        Hello::decode(b"HELLO {\"protocol\":9}").unwrap_err(),
        Hello::decode(format!("HELLO {{\"protocol\":1,\"{}\":1}}", "x".repeat(3000)).as_bytes())
            .unwrap_err(),
        Hello::decode(HELLO_LINE.replace("80000", "1").as_bytes()).unwrap_err(),
    ];
    for error in errors {
        let text = error.to_string();
        assert!(!text.contains('\n'), "{text}");
        assert!(text.len() <= 400, "{} bytes: {text}", text.len());
    }
}
