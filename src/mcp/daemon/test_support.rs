//! Pieces the daemon's tests share.
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};

/// The settings of a client that waits `wait_secs` for the index and always gets
/// `structuredContent`.
pub(super) fn client_settings(wait_secs: u64) -> ClientSettings {
    client_settings_with(wait_secs, StructuredContentMode::Always)
}

pub(super) fn client_settings_with(
    wait_secs: u64,
    structured_content: StructuredContentMode,
) -> ClientSettings {
    let output = ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    };
    ClientSettings::new(output, wait_secs, false).expect("settings within the accepted bounds")
}
