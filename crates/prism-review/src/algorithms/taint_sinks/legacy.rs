//! The legacy flat list for languages that have no call-position rules (#622 S1).
//!
//! Every language outside JS/TS/TSX, Python and Go keeps the whole substring list
//! except for the names in `SHELL_ONLY`, and `get_user`, which is exact and
//! C/C++ only. A name is `SHELL_ONLY` only when it is a shell or busybox command
//! with no library or API meaning in another language: as an identifier outside
//! Bash it is a variable, field or parameter (`let source = ..`, `char *find`,
//! `ip`), never a call to that command.
//!
//! Names that look like shell commands but are also library entry points are NOT
//! gated and keep firing everywhere they did: `sqlite3` (the `sqlite3_*` C API),
//! `=chmod` / `=chown` (POSIX calls), `=curl` (libcurl handle), `=mtd`,
//! `=fw_setenv`, `=fw_printenv` (U-Boot tools C API), `=uci` (the Lua UCI cursor
//! `uci:set(..)`), `=bridge`, `=swconfig`.

use prism::languages::Language;

/// Shell-only command names, matched by the same `matches_sink` convention as the
/// flat list (`=` prefix is exact). Gated to Bash:
/// - `=eval`, `=exec`: redundant elsewhere (the generic `eval` / `exec` fragments
///   still cover every language).
/// - `=source`, `=find`, `=awk`, `=sed`, `=su`, `=sudo`, `=wget`, `xargs`: shell
///   builtins/utilities; outside Bash these are ordinary variable names
///   (M0: `var named source`).
/// - `=ip`, `=ifconfig`, `=iptables`, `=ip6tables`, `=brctl`, `=vconfig`,
///   `=insmod`, `=modprobe`, `=rmmod`, `procd_set_param`: network/kernel-module
///   utilities and the OpenWrt init helper; no API of that spelling.
const SHELL_ONLY: &[&str] = &[
    "=eval",
    "=exec",
    "=source",
    "=find",
    "=awk",
    "=sed",
    "=su",
    "=sudo",
    "=wget",
    "xargs",
    "=ip",
    "=ifconfig",
    "=iptables",
    "=ip6tables",
    "=brctl",
    "=vconfig",
    "=insmod",
    "=modprobe",
    "=rmmod",
    "procd_set_param",
];

pub(super) fn legacy_hit(
    language: Language,
    text: &str,
    patterns: &[&str],
) -> Option<super::SinkHit> {
    let c_family = matches!(language, Language::C | Language::Cpp);
    patterns
        .iter()
        .filter(|p| language == Language::Bash || !SHELL_ONLY.contains(p))
        .any(|p| match *p {
            // Kernel `get_user`: an exact C/C++ name, not `get_user_id`.
            "get_user" => c_family && text == "get_user",
            p => crate::algorithms::taint::matches_sink(text, p),
        })
        .then_some(super::SinkHit {
            rule: "legacy",
            cwe: None,
            strength: super::Strength::Strong,
        })
}
