//! Certificate checks 0-5 (DESIGN.md 4.2 and 18.2), a line-for-line port of `census/check.py`.
//! Compiled for tests only: it is the grammar-drift guard, not census behaviour.

use super::schema::{self, NodeType, Slots};
use super::tables::{self, Tables};
use std::collections::{BTreeMap, BTreeSet};

/// One grammar under test: its name, parsed schema and (when checked) its recorded signature.
pub(super) struct Grammar {
    pub name: &'static str,
    pub schema: Vec<NodeType>,
    pub recorded_signature: Option<BTreeSet<String>>,
}

#[derive(Default)]
pub(super) struct Report {
    pub problems: Vec<String>,
    pub slots: usize,
    pub rows: usize,
    pub leaves: usize,
    pub target_kinds: usize,
    pub order_rows: usize,
}

impl Report {
    /// The summary line `check.py` prints.
    pub(super) fn summary(&self) -> String {
        format!(
            "{}: {} problem(s); slots {}, rows {}, leaves {}, target kinds {}, order rows {}",
            if self.problems.is_empty() {
                "PASS"
            } else {
                "FAIL"
            },
            self.problems.len(),
            self.slots,
            self.rows,
            self.leaves,
            self.target_kinds,
            self.order_rows
        )
    }

    pub(super) fn has(&self, token: &str) -> bool {
        self.problems.iter().any(|p| p.contains(token))
    }
}

/// Check 1 only: the computed signature against the recorded one.
pub(super) fn check_signature(g: &Grammar, bad: &mut Vec<String>) {
    let Some(old) = &g.recorded_signature else {
        return;
    };
    let new = schema::signature(&g.schema);
    for l in new.difference(old) {
        bad.push(format!("SIGNATURE-ADDED {} {l}", g.name));
    }
    for l in old.difference(&new) {
        bad.push(format!("SIGNATURE-REMOVED {} {l}", g.name));
    }
}

/// Run checks 0-5 over `grammars` and `t`.
pub(super) fn certify(grammars: &[Grammar], t: &Tables) -> Report {
    let mut bad: Vec<String> = t.diagnostics.clone(); // check 0
    for g in grammars {
        check_signature(g, &mut bad); // check 1
    }
    let slot_maps: Vec<(&str, Slots)> = grammars
        .iter()
        .map(|g| (g.name, schema::slots(&g.schema)))
        .collect();
    // check 2: coverage
    for (key, row) in &t.slots {
        if !tables::role_is_valid(&row.role) {
            bad.push(format!("BAD-ROLE {} {} {}", key.0, key.1, row.role));
        }
        if !tables::ORDER_HINTS.contains(&row.order.as_str()) {
            bad.push(format!("BAD-ORDER {} {} {}", key.0, key.1, row.order));
        }
    }
    let mut rel: BTreeMap<(String, String), BTreeMap<&str, BTreeSet<String>>> = BTreeMap::new();
    for (lang, sm) in &slot_maps {
        for (key, hit) in schema::relevant(sm) {
            rel.entry(key).or_default().insert(*lang, hit);
        }
    }
    for (key, langs) in &rel {
        if !t.slots.contains_key(key) {
            bad.push(format!("UNCLASSIFIED {} {} {langs:?}", key.0, key.1));
        }
    }
    for key in t.slots.keys() {
        if !rel.contains_key(key) {
            bad.push(format!("STALE {} {}", key.0, key.1));
        }
    }
    // check 3: leaves
    let lk: BTreeSet<String> = grammars
        .iter()
        .flat_map(|g| schema::leaves(&g.schema))
        .collect();
    for k in lk.iter().filter(|k| !t.leaves.contains_key(*k)) {
        bad.push(format!("UNCLASSIFIED-LEAF {k}"));
    }
    for k in t.leaves.keys().filter(|k| !lk.contains(*k)) {
        bad.push(format!("STALE-LEAF {k}"));
    }
    let bind: BTreeSet<&str> = t
        .leaves
        .iter()
        .filter(|(_, v)| v.bind)
        .map(|(k, _)| k.as_str())
        .collect();
    let want: BTreeSet<&str> = schema::NAME_LEAVES
        .iter()
        .copied()
        .chain(["type_identifier"])
        .collect();
    if bind != want {
        bad.push(format!("BIND-SET-DRIFT {bind:?}"));
    }
    // check 4: target closure
    for (lang, sm) in &slot_maps {
        check_target_closure(lang, sm, &rel, t, &mut bad);
    }
    let all_kinds: BTreeSet<&str> = grammars
        .iter()
        .flat_map(|g| g.schema.iter().map(|x| x.kind.as_str()))
        .collect();
    for k in t.targets.keys().filter(|k| !all_kinds.contains(k.as_str())) {
        bad.push(format!("STALE-TARGET {k}"));
    }
    // check 5: order
    for (k, cls) in &t.order {
        if !tables::ORDER_CLASSES.contains(&cls.as_str()) {
            bad.push(format!("BAD-ORDER-CLASS {k} {cls}"));
        }
    }
    let named: BTreeSet<&str> = grammars
        .iter()
        .flat_map(|g| {
            g.schema
                .iter()
                .filter(|x| x.named && x.subtypes.is_none())
                .map(|x| x.kind.as_str())
        })
        .collect();
    for k in named.iter().filter(|k| !t.order.contains_key(**k)) {
        bad.push(format!("UNCLASSIFIED-ORDER {k}"));
    }
    for k in t.order.keys().filter(|k| !named.contains(k.as_str())) {
        bad.push(format!("STALE-ORDER {k}"));
    }
    for (k, cls) in &t.order {
        if named.contains(k.as_str()) && (cls == "leaf") != lk.contains(k) {
            bad.push(format!("ORDER-LEAF-MISMATCH {k} {cls}"));
        }
    }
    Report {
        problems: bad,
        slots: rel.len(),
        rows: t.slots.len(),
        leaves: lk.len(),
        target_kinds: t.targets.len(),
        order_rows: t.order.len(),
    }
}

fn carry_role_ok(action: &str, role: &str) -> bool {
    match action {
        "PATTERN" => matches!(role, "INHERIT" | "WRAP"),
        "WRAP" => role == "WRAP",
        "PARAM" => role.starts_with("DECL:"),
        "IMPORTS" => role.starts_with("DECL:") || role == "IMPORTNAME",
        _ => true,
    }
}

fn check_target_closure(
    lang: &str,
    sm: &Slots,
    rel: &BTreeMap<(String, String), BTreeMap<&str, BTreeSet<String>>>,
    t: &Tables,
    bad: &mut Vec<String>,
) {
    let mut stack: Vec<(String, (String, String))> = Vec::new();
    for (key, kinds) in sm {
        if tables::is_target_root(t.role(&key.0, &key.1).unwrap_or("")) {
            stack.extend(kinds.iter().map(|k| (k.clone(), key.clone())));
        }
    }
    let mut seen = BTreeSet::new();
    while let Some((kind, via)) = stack.pop() {
        if !seen.insert(kind.clone()) {
            continue;
        }
        let Some(row) = t.targets.get(&kind) else {
            bad.push(format!(
                "TARGET-KIND-UNCLASSIFIED {lang} {kind} via {} {}",
                via.0, via.1
            ));
            continue;
        };
        if !matches!(
            row.action.as_str(),
            "PATTERN" | "WRAP" | "PARAM" | "IMPORTS" | "CONTAINER"
        ) {
            continue;
        }
        for slot in &row.carrying {
            let key = (kind.clone(), slot.clone());
            let Some(next) = sm.get(&key) else {
                bad.push(format!("CARRYING-SLOT-MISSING {lang} {kind} {slot}"));
                continue;
            };
            let role = t.role(&kind, slot);
            let lost = if row.action == "CONTAINER" {
                rel.contains_key(&key)
                    && !tables::is_target_root(role.unwrap_or(""))
                    && !matches!(role, Some("INHERIT" | "WRAP"))
            } else {
                role.is_none_or(|r| !carry_role_ok(&row.action, r))
            };
            if lost {
                bad.push(format!(
                    "TARGET-ROLE-LOSS {lang} {kind} {slot} role {} action {}",
                    role.unwrap_or("None"),
                    row.action
                ));
                continue;
            }
            stack.extend(next.iter().map(|k| (k.clone(), key.clone())));
        }
    }
}
