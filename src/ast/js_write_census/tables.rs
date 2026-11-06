//! The four reviewed census tables, loaded from embedded TSV text (#660 S0, DESIGN.md 18.1).
//!
//! Every loader rejects a duplicate key (identical or not) before anything is read from the
//! table (certificate check 0, `DUPLICATE-ROW`) and a malformed row (`MALFORMED-ROW`).

use std::collections::BTreeMap;

pub(crate) const TABLE_TSV: &str = include_str!("data/table.tsv");
pub(crate) const LEAVES_TSV: &str = include_str!("data/leaves.tsv");
pub(crate) const TARGETS_TSV: &str = include_str!("data/targets.tsv");
pub(crate) const ORDER_TSV: &str = include_str!("data/order.tsv");

pub(crate) const ROLES: [&str; 11] = [
    "W",
    "RW",
    "DEL",
    "FORHEAD",
    "INHERIT",
    "WRAP",
    "R",
    "RDEF",
    "N",
    "T",
    "IMPORTNAME",
];
pub(crate) const DECLS: [&str; 10] = [
    "declarator",
    "param",
    "catch",
    "fn-hoist",
    "fn-inner",
    "class",
    "class-inner",
    "import",
    "enum",
    "namespace",
];
pub(crate) const ORDER_HINTS: [&str; 4] = ["text", "before-target", "after-body", "deferred"];
pub(crate) const ORDER_CLASSES: [&str; 6] = ["seq", "loop", "unknown", "deferred", "none", "leaf"];

/// A role is in the closed set: a plain role or `DECL:<declaration kind>`.
pub(crate) fn role_is_valid(role: &str) -> bool {
    ROLES.contains(&role)
        || role
            .strip_prefix("DECL:")
            .is_some_and(|d| DECLS.contains(&d))
}

/// Roles whose slot holds a write or binding target (the roots of target closure).
pub(crate) fn is_target_root(role: &str) -> bool {
    matches!(role, "W" | "RW" | "DEL" | "FORHEAD" | "IMPORTNAME") || role.starts_with("DECL:")
}

#[derive(Clone, Debug)]
pub(crate) struct SlotRow {
    pub role: String,
    pub order: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LeafRow {
    pub bind: bool,
    pub name_token: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct TargetRow {
    pub action: String,
    pub carrying: Vec<String>,
}

/// The loaded tables. `diagnostics` holds the load-time findings (check 0 and malformed rows).
#[derive(Debug, Default)]
pub(crate) struct Tables {
    pub slots: BTreeMap<(String, String), SlotRow>,
    pub leaves: BTreeMap<String, LeafRow>,
    pub targets: BTreeMap<String, TargetRow>,
    pub order: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
}

/// Parse `text` into rows of `columns` fields keyed by the first `key_len` fields.
/// Comment (`#`) and blank lines are skipped. The first row of a key is kept.
fn rows(
    label: &str,
    text: &str,
    columns: usize,
    key_len: usize,
    diagnostics: &mut Vec<String>,
) -> BTreeMap<Vec<String>, Vec<String>> {
    let mut out: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<String> = line.split('\t').map(str::to_string).collect();
        if parts.len() != columns {
            diagnostics.push(format!("MALFORMED-ROW {label} {}", line.trim()));
            continue;
        }
        let key = parts[..key_len].to_vec();
        if out.contains_key(&key) {
            diagnostics.push(format!("DUPLICATE-ROW {label} {}", key.join(" ")));
            continue;
        }
        out.insert(key, parts);
    }
    out
}

impl Tables {
    /// The embedded tables, loaded once per process (production callers run per owner).
    pub(crate) fn shared() -> &'static Tables {
        static SHARED: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
        SHARED.get_or_init(Tables::embedded)
    }

    /// The reviewed tables shipped with the crate.
    pub(crate) fn embedded() -> Tables {
        Tables::load(
            ("table.tsv", TABLE_TSV),
            ("leaves.tsv", LEAVES_TSV),
            ("targets.tsv", TARGETS_TSV),
            ("order.tsv", ORDER_TSV),
        )
    }

    /// Load from `(label, text)` pairs, so a test can replace one table.
    pub(crate) fn load(
        table: (&str, &str),
        leaves: (&str, &str),
        targets: (&str, &str),
        order: (&str, &str),
    ) -> Tables {
        let mut t = Tables::default();
        let d = &mut t.diagnostics;
        for (k, r) in rows(table.0, table.1, 5, 2, d) {
            t.slots.insert(
                (k[0].clone(), k[1].clone()),
                SlotRow {
                    role: r[2].clone(),
                    order: r[3].clone(),
                },
            );
        }
        for (k, r) in rows(leaves.0, leaves.1, 4, 1, d) {
            t.leaves.insert(
                k[0].clone(),
                LeafRow {
                    bind: r[1] == "BIND",
                    name_token: r[2] == "NAME",
                },
            );
        }
        for (k, r) in rows(targets.0, targets.1, 4, 1, d) {
            let carrying = r[2].split(',').map(str::to_string).collect();
            t.targets.insert(
                k[0].clone(),
                TargetRow {
                    action: r[1].clone(),
                    carrying,
                },
            );
        }
        for (k, r) in rows(order.0, order.1, 3, 1, d) {
            t.order.insert(k[0].clone(), r[1].clone());
        }
        t
    }

    pub(crate) fn role(&self, kind: &str, slot: &str) -> Option<&str> {
        self.slots
            .get(&(kind.to_string(), slot.to_string()))
            .map(|r| r.role.as_str())
    }

    pub(crate) fn order_class(&self, kind: &str) -> Option<&str> {
        self.order.get(kind).map(String::as_str)
    }

    /// Kinds with a NAME token (the escape precondition scans these).
    pub(crate) fn is_name_token(&self, kind: &str) -> bool {
        self.leaves.get(kind).is_some_and(|l| l.name_token)
    }
}
