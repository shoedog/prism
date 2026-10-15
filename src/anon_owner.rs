//! Shared owner-name helper for anonymous JS/TS/TSX callables (lane A2, S1).
//!
//! One module owns the name format and the predicate that recognises it, so
//! every guard (position lookups, symbol seeds, navigation visibility, MCP
//! suggestions, Mermaid labels) agrees on what an anonymous identity is.
//!
//! Today only the js-param-defs PR-B DFG pass mints these names, under the
//! `<cb@L:C>` spelling. Lane A2 S0 renames the format to `<anon@L:C>`; until it
//! lands the predicate accepts BOTH prefixes. S0 then deletes
//! [`LEGACY_CB_OWNER_PREFIX`] and flips [`MINT_OWNER_PREFIX`].
//!
//! `<` cannot start an ECMAScript identifier, and every name `function_name`
//! can return for JS/TS is an identifier, a property name, a string literal
//! (with its quotes) or a computed key (with its bracket), so no real owner can
//! spell either prefix.

/// Target prefix of an anonymous callable's owner name (lane A2 key format).
pub const ANON_OWNER_PREFIX: &str = "<anon@";

/// Prefix PR-B mints today. Recognised by [`is_anonymous_owner_name`] until S0
/// renames every minted name to [`ANON_OWNER_PREFIX`].
pub const LEGACY_CB_OWNER_PREFIX: &str = "<cb@";

/// Prefix [`anonymous_owner_name`] currently emits. S0 changes this single
/// constant; it must stay the legacy spelling until then so S1 changes no
/// output.
const MINT_OWNER_PREFIX: &str = LEGACY_CB_OWNER_PREFIX;

/// Whether an owner name denotes an anonymous callable identity (either
/// spelling).
pub fn is_anonymous_owner_name(name: &str) -> bool {
    name.starts_with(ANON_OWNER_PREFIX) || name.starts_with(LEGACY_CB_OWNER_PREFIX)
}

/// Owner name of the anonymous callable whose first byte is at 1-based `line`
/// and 1-based byte column `col`.
pub fn anonymous_owner_name(line: usize, col: usize) -> String {
    format!("{MINT_OWNER_PREFIX}{line}:{col}>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_both_spellings() {
        assert!(is_anonymous_owner_name("<cb@1:10>"));
        assert!(is_anonymous_owner_name("<anon@12:3>"));
    }

    #[test]
    fn rejects_real_owner_names() {
        for name in [
            "",
            "f",
            "cb@1:1",
            "anon",
            "\"<cb@1:3>\"",
            "[<anon@1:1>]",
            "<module>",
            "<anonymous>",
            "<c",
        ] {
            assert!(!is_anonymous_owner_name(name), "{name:?}");
        }
    }

    #[test]
    fn mint_round_trips_through_the_predicate() {
        let minted = anonymous_owner_name(4, 19);
        assert!(minted.ends_with("4:19>"), "{minted}");
        // S1 changes no output: the minted spelling stays `<cb@` until S0 renames it.
        assert!(minted.starts_with("<cb@"), "{minted}");
        assert!(is_anonymous_owner_name(&minted));
    }
}
