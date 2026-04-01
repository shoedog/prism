//! Structured variable access path for field-sensitive analysis.
//!
//! Replaces bare string variable names (`var_name: String`) with a structured
//! `AccessPath` that tracks the base variable and its field access chain.
//! This enables field-sensitive data flow analysis: `dev->name` and `dev->id`
//! are distinct access paths, eliminating false taint propagation across
//! unrelated struct fields.
//!
//! Part of the Code Property Graph architecture (Phase 1).
//! See `docs/cpg-architecture.md` for the full design.

use std::fmt;

/// A structured representation of a variable access.
///
/// Examples:
/// - `x` → `AccessPath { base: "x", fields: [] }`
/// - `dev->name` → `AccessPath { base: "dev", fields: ["name"] }`
/// - `self.config.timeout` → `AccessPath { base: "self", fields: ["config", "timeout"] }`
/// - `buf[i]` → `AccessPath { base: "buf", fields: ["[]"] }`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessPath {
    /// The root variable name.
    pub base: String,
    /// Field access chain, outermost first. Empty for plain variables.
    /// Array access is represented as `"[]"` (index-insensitive).
    pub fields: Vec<String>,
}

/// The sentinel field name for array subscript access.
pub const ARRAY_FIELD: &str = "[]";

/// Default k-limit for access path depth. Paths deeper than this are
/// truncated and fall back to field-insensitive matching.
pub const DEFAULT_K_LIMIT: usize = 5;

impl AccessPath {
    /// Create an access path for a plain variable (no field access).
    pub fn simple(name: impl Into<String>) -> Self {
        AccessPath {
            base: name.into(),
            fields: Vec::new(),
        }
    }

    /// Create an access path with a field chain.
    pub fn with_fields(base: impl Into<String>, fields: Vec<String>) -> Self {
        AccessPath {
            base: base.into(),
            fields,
        }
    }

    /// Parse an access path from a source expression string.
    ///
    /// Handles:
    /// - `x` → simple path
    /// - `dev->field` → base "dev", field "field"
    /// - `dev->config->timeout` → base "dev", fields ["config", "timeout"]
    /// - `obj.field` → base "obj", field "field"
    /// - `obj.a.b.c` → base "obj", fields ["a", "b", "c"]
    /// - `(*dev).field` → base "dev", field "field" (normalized)
    /// - `buf[i]` → base "buf", field "[]"
    /// - `*ptr` → simple path "ptr" (dereference stripped)
    pub fn from_expr(expr: &str) -> Self {
        let expr = expr.trim();

        // Pointer dereference: *p, **p → strip to base
        if expr.starts_with('*') {
            let inner = expr.trim_start_matches('*').trim();
            let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
            // Could be *dev->field
            if inner.contains("->") || inner.contains('.') {
                return AccessPath::from_expr(inner);
            }
            if is_plain_ident(inner) {
                return AccessPath::simple(inner);
            }
            return AccessPath::simple(expr);
        }

        // Parenthesized dereference: (*dev).field
        if expr.starts_with('(') {
            if let Some(paren_end) = expr.find(')') {
                let inner = expr[1..paren_end].trim();
                let after = expr[paren_end + 1..].trim();
                if let Some(rest) = after.strip_prefix('.') {
                    let base_path = AccessPath::from_expr(inner);
                    let mut fields = base_path.fields;
                    for field in rest.split('.') {
                        let f = field.trim();
                        if !f.is_empty() {
                            fields.push(f.to_string());
                        }
                    }
                    return AccessPath {
                        base: base_path.base,
                        fields,
                    };
                }
            }
        }

        // Arrow access: dev->field or dev->config->timeout
        if expr.contains("->") {
            let parts: Vec<&str> = expr.split("->").collect();
            if parts.len() >= 2 {
                let base = parts[0].trim();
                if is_plain_ident(base) {
                    let fields: Vec<String> = parts[1..]
                        .iter()
                        .map(|p| {
                            let p = p.trim();
                            // Handle trailing array: dev->buf[i]
                            if let Some(bracket) = p.find('[') {
                                p[..bracket].trim().to_string()
                            } else {
                                p.to_string()
                            }
                        })
                        .filter(|f| !f.is_empty())
                        .collect();
                    return AccessPath {
                        base: base.to_string(),
                        fields,
                    };
                }
            }
        }

        // Array subscript: buf[i] → base "buf", field "[]"
        if let Some(bracket) = expr.find('[') {
            let base = expr[..bracket].trim();
            // Could be obj.field[i]
            if base.contains('.') {
                let mut path = AccessPath::from_expr(base);
                path.fields.push(ARRAY_FIELD.to_string());
                return path;
            }
            if is_plain_ident(base) {
                return AccessPath {
                    base: base.to_string(),
                    fields: vec![ARRAY_FIELD.to_string()],
                };
            }
        }

        // Dot access: obj.field or self.config.timeout
        if expr.contains('.') {
            let parts: Vec<&str> = expr.split('.').collect();
            if parts.len() >= 2 && is_plain_ident(parts[0].trim()) {
                let base = parts[0].trim().to_string();
                let fields: Vec<String> = parts[1..]
                    .iter()
                    .map(|p| {
                        let p = p.trim();
                        if let Some(bracket) = p.find('[') {
                            p[..bracket].trim().to_string()
                        } else {
                            p.to_string()
                        }
                    })
                    .filter(|f| !f.is_empty())
                    .collect();
                return AccessPath { base, fields };
            }
        }

        // Plain identifier
        AccessPath::simple(expr)
    }

    /// The depth of this access path (number of field accesses).
    pub fn depth(&self) -> usize {
        self.fields.len()
    }

    /// Whether this is a simple variable with no field access.
    pub fn is_simple(&self) -> bool {
        self.fields.is_empty()
    }

    /// Whether this path has field accesses.
    pub fn has_fields(&self) -> bool {
        !self.fields.is_empty()
    }

    /// Truncate to at most k field levels. Returns a new path.
    pub fn truncate(&self, k: usize) -> Self {
        AccessPath {
            base: self.base.clone(),
            fields: self.fields[..self.fields.len().min(k)].to_vec(),
        }
    }

    /// Check if this path is a prefix of another.
    ///
    /// `dev` is a prefix of `dev->name` (whole-struct contains field).
    /// `dev->config` is a prefix of `dev->config->timeout`.
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        self.base == other.base
            && self.fields.len() <= other.fields.len()
            && self.fields == other.fields[..self.fields.len()]
    }

    /// Field-sensitive match: paths must be exactly equal.
    pub fn matches_field_sensitive(&self, other: &Self) -> bool {
        self == other
    }

    /// Field-insensitive match: only compare base names.
    pub fn matches_base(&self, other: &Self) -> bool {
        self.base == other.base
    }

    /// Match using the appropriate strategy based on whether both paths
    /// have field information.
    ///
    /// - If both have fields → field-sensitive (exact match)
    /// - If either is a simple base → base match (field-insensitive fallback)
    ///
    /// This is the default matching strategy for Phase 1, ensuring backward
    /// compatibility: existing code that only tracks base names will still
    /// match against field-qualified paths via the base.
    pub fn matches_compatible(&self, other: &Self) -> bool {
        if self.has_fields() && other.has_fields() {
            self.matches_field_sensitive(other)
        } else {
            self.matches_base(other)
        }
    }
}

impl fmt::Display for AccessPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)?;
        for field in &self.fields {
            if field == ARRAY_FIELD {
                write!(f, "[]")?;
            } else {
                write!(f, ".{}", field)?;
            }
        }
        Ok(())
    }
}

/// Convert from a plain string (backward compatibility).
impl From<String> for AccessPath {
    fn from(name: String) -> Self {
        AccessPath::simple(name)
    }
}

/// Convert from a string slice (backward compatibility).
impl From<&str> for AccessPath {
    fn from(name: &str) -> Self {
        AccessPath::simple(name)
    }
}

fn is_plain_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        && s.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable() {
        let p = AccessPath::from_expr("x");
        assert_eq!(p.base, "x");
        assert!(p.fields.is_empty());
        assert!(p.is_simple());
        assert_eq!(p.to_string(), "x");
    }

    #[test]
    fn test_arrow_access() {
        let p = AccessPath::from_expr("dev->name");
        assert_eq!(p.base, "dev");
        assert_eq!(p.fields, vec!["name"]);
        assert_eq!(p.depth(), 1);
        assert_eq!(p.to_string(), "dev.name");
    }

    #[test]
    fn test_nested_arrow() {
        let p = AccessPath::from_expr("dev->config->timeout");
        assert_eq!(p.base, "dev");
        assert_eq!(p.fields, vec!["config", "timeout"]);
        assert_eq!(p.depth(), 2);
    }

    #[test]
    fn test_dot_access() {
        let p = AccessPath::from_expr("self.config.timeout");
        assert_eq!(p.base, "self");
        assert_eq!(p.fields, vec!["config", "timeout"]);
    }

    #[test]
    fn test_array_access() {
        let p = AccessPath::from_expr("buf[i]");
        assert_eq!(p.base, "buf");
        assert_eq!(p.fields, vec!["[]"]);
    }

    #[test]
    fn test_pointer_deref() {
        let p = AccessPath::from_expr("*ptr");
        assert_eq!(p.base, "ptr");
        assert!(p.is_simple());
    }

    #[test]
    fn test_parenthesized_deref_dot() {
        let p = AccessPath::from_expr("(*dev).field");
        assert_eq!(p.base, "dev");
        assert_eq!(p.fields, vec!["field"]);
    }

    #[test]
    fn test_prefix_check() {
        let base = AccessPath::simple("dev");
        let field = AccessPath::from_expr("dev->name");
        let nested = AccessPath::from_expr("dev->config->timeout");

        assert!(base.is_prefix_of(&field));
        assert!(base.is_prefix_of(&nested));
        assert!(field.is_prefix_of(&field));
        assert!(!field.is_prefix_of(&base));

        let config = AccessPath::from_expr("dev->config");
        assert!(config.is_prefix_of(&nested));
        assert!(!field.is_prefix_of(&nested)); // dev->name is not prefix of dev->config->timeout
    }

    #[test]
    fn test_field_sensitive_matching() {
        let name = AccessPath::from_expr("dev->name");
        let id = AccessPath::from_expr("dev->id");
        let name2 = AccessPath::from_expr("dev->name");

        assert!(name.matches_field_sensitive(&name2));
        assert!(!name.matches_field_sensitive(&id));
    }

    #[test]
    fn test_base_matching() {
        let name = AccessPath::from_expr("dev->name");
        let id = AccessPath::from_expr("dev->id");

        assert!(name.matches_base(&id)); // same base "dev"
    }

    #[test]
    fn test_compatible_matching() {
        let name = AccessPath::from_expr("dev->name");
        let id = AccessPath::from_expr("dev->id");
        let base = AccessPath::simple("dev");

        // Both have fields → field-sensitive
        assert!(!name.matches_compatible(&id));
        // One is simple → base match
        assert!(name.matches_compatible(&base));
        assert!(base.matches_compatible(&name));
    }

    #[test]
    fn test_truncate() {
        let deep = AccessPath::with_fields("a", vec!["b".into(), "c".into(), "d".into()]);
        let truncated = deep.truncate(2);
        assert_eq!(truncated.fields, vec!["b", "c"]);
    }

    #[test]
    fn test_display() {
        assert_eq!(AccessPath::simple("x").to_string(), "x");
        assert_eq!(AccessPath::from_expr("dev->name").to_string(), "dev.name");
        assert_eq!(
            AccessPath::from_expr("dev->config->timeout").to_string(),
            "dev.config.timeout"
        );
        assert_eq!(AccessPath::from_expr("buf[i]").to_string(), "buf[]");
    }
}
