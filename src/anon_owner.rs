//! Shared owner-name helper for anonymous JS/TS/TSX callables (lane A2, S1).
//!
//! One module owns the name format and the predicate that recognises it, so
//! every guard (position lookups, symbol seeds, navigation visibility, MCP
//! suggestions, Mermaid labels) agrees on what an anonymous identity is.
//!
//! The js-param-defs PR-B DFG pass mints these names as `<anon@L:C>`.
//! Minting and recognition share [`ANON_OWNER_PREFIX`].
//!
//! `<` cannot start an ECMAScript identifier, and every name `function_name`
//! can return for JS/TS is an identifier, a property name, a string literal
//! (with its quotes) or a computed key (with its bracket), so no real owner can
//! spell this prefix.

/// Prefix of an anonymous callable's owner name (lane A2 key format).
pub const ANON_OWNER_PREFIX: &str = "<anon@";

/// Prefix [`anonymous_owner_name`] emits.
const MINT_OWNER_PREFIX: &str = ANON_OWNER_PREFIX;

/// Whether an owner name denotes an anonymous callable identity.
pub fn is_anonymous_owner_name(name: &str) -> bool {
    name.starts_with(ANON_OWNER_PREFIX)
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
    fn recognises_only_anon_spelling() {
        assert!(!is_anonymous_owner_name("<cb@1:10>"));
        assert!(is_anonymous_owner_name("<anon@12:3>"));
    }

    #[test]
    fn rejects_real_owner_names() {
        for name in [
            "",
            "f",
            "anon@1:1",
            "anon",
            "\"<anon@1:3>\"",
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
        // The minted spelling is the lane A2 owner key.
        assert!(minted.starts_with("<anon@"), "{minted}");
        assert!(is_anonymous_owner_name(&minted));
    }
}
