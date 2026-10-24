//! Wire assertions use literal bytes so they also execute on the slice parent.
use super::control::{ControlRequest, DaemonStatus};
use super::preamble::{PreambleError, Reply};

#[test]
fn daemon_idle_wire_policy() {
    let key = "ab".repeat(32);
    for build in ["idle", "unloaded", "building", "ready", "failed"] {
        for (timeout, relief) in [(0, false), (600, false), (600, true)] {
            let ready = format!("READY {{\"protocol\":2,\"identity\":\"{key}\",\"pid\":42,\"start_time\":10,\"generation\":0,\"build\":\"{build}\",\"index_idle_timeout_secs\":{timeout},\"index_idle_pressure_relief\":{relief}}}");
            let decoded = Reply::decode(ready.as_bytes());
            assert!(decoded.is_ok(), "wire v2 READY must decode: {decoded:?}");
            assert_eq!(decoded.unwrap().encode(), format!("{ready}\n"));
            let status = format!("STATUS {{\"control\":2,\"identity\":\"{key}\",\"pid\":42,\"start_time\":10,\"build_identity\":\"test\",\"repo_root\":\"/repo\",\"admitted\":0,\"pending\":0,\"max_clients\":64,\"build\":\"{build}\",\"attempt\":1,\"generation\":0,\"idle_secs\":null,\"idle_ttl_secs\":900,\"index_idle_timeout_secs\":{timeout},\"index_idle_pressure_relief\":{relief},\"shutdown\":false}}");
            let decoded = DaemonStatus::decode(status.as_bytes());
            assert!(decoded.is_ok(), "wire v2 STATUS must decode: {decoded:?}");
            assert_eq!(decoded.unwrap().encode(), format!("{status}\n"));
        }
    }
}

#[test]
fn daemon_idle_v1_is_typed_incompatible() {
    assert_eq!(
        Reply::decode(b"READY {\"protocol\":1}"),
        Err(PreambleError::UnsupportedProtocol { got: 1 })
    );
    assert_eq!(
        DaemonStatus::decode(b"STATUS {\"control\":1}"),
        Err(PreambleError::UnsupportedControl { got: 1 })
    );
    assert_eq!(
        ControlRequest::decode(b"STATUS {\"control\":1}"),
        Err(PreambleError::UnsupportedControl { got: 1 })
    );
}
