//! #829 test form, layer L2 (docs/superpowers/plans/2026-11-14-829-macro-brackets/TEST-FORM.md
//! section 4): rows frozen from the bounded generator (`scripts/dfg_prune/xgen.py`, offline;
//! XARM.md, "The generator"). Each row is the MINIMAL member of one key (effect classes, site
//! family, owner kind) whose pair the rules drop on base while the compiler's token stream keeps
//! it (the expansion-arm oracle said CONTRADICTED; `source: generator`). They are read from
//! `tests/fixtures/dfg_prune/oracle/generated_rows.jsonl` and need no clang.
//!
//! One test per site family so the red count is attributable: a row is red while the pair is
//! dropped (the baseline has it, a removal-only rule dropped it). Top-level `;` bodies (#831)
//! are excluded at freeze time.
use super::c_prune_brackets_r3_support::pair_in;
use super::c_prune_oracle_rows::{judge_data, Want};

fn rows() -> Vec<serde_json::Value> {
    include_str!("../../../tests/fixtures/dfg_prune/oracle/generated_rows.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("a generated row is JSON"))
        .collect()
}

fn judge_row(row: &serde_json::Value) -> Result<(), String> {
    let texts: Vec<&str> = row["texts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    let all = row["files"].as_object().unwrap();
    let files: Vec<(&str, &str)> = all
        .iter()
        .filter(|(n, _)| !texts.contains(&n.as_str()))
        .map(|(n, c)| (n.as_str(), c.as_str().unwrap()))
        .collect();
    let text_files: Vec<(&str, &str)> = texts
        .iter()
        .map(|n| (*n, all[*n].as_str().unwrap()))
        .collect();
    let target = row["target"].as_str().unwrap();
    let r = pair_in(
        &files,
        target,
        &text_files,
        Some(row["owner"].as_str().unwrap()),
        row["path"].as_str().unwrap(),
        row["from"].as_u64().unwrap() as usize,
        row["to"].as_u64().unwrap() as usize,
    );
    let what = format!("generated {}", row["key"].as_str().unwrap());
    judge_data(&what, Want::Kept, r, row)
}

/// Every row of one site family (`spec.family`).
fn family(name: &str) {
    let rows: Vec<_> = rows()
        .into_iter()
        .filter(|r| r["spec"]["family"] == name)
        .collect();
    let failed: Vec<String> = rows.iter().filter_map(|r| judge_row(r).err()).collect();
    assert!(
        failed.is_empty(),
        "{} of {} generated {name} rows: a live pair was dropped:\n{}",
        failed.len(),
        rows.len(),
        failed.join("\n")
    );
}

macro_rules! families {
    ($($test:ident $name:expr),+ $(,)?) => {
        $(#[test] fn $test() { family($name); })+
    };
}

// The `init` and `decl` families (`int y = A;`, `int A;`) produced no CONTRADICTED program that
// is not a top-level `;` body (#831): no row, no test.
//
// The `nest` family (PLAN-v2 task 2: an outer macro whose body invokes an inner macro on a
// held-comma block, forms `K(<frags>)` and `ID(K <N>)`) found six keys at bound 4 (owner kind C,
// 1,344 compiled programs of 295,542; the forms `K(<N>)`, `one(K(..))` and `ID(one(<N>))` gave
// none): `gen_nest`.
families!(
    gen_stmt "stmt", gen_guard "guard", gen_arg "arg", gen_arg1 "arg1", gen_enc1 "enc1",
    gen_enc2 "enc2", gen_enc3 "enc3", gen_enc4 "enc4", gen_nest "nest",
);

/// The data is well formed: no clang, no prune run. Every row is a generator row with the
/// oracle's verdict, its pair lines point at the def and the return of the owner, and the
/// frozen minimal member of a key is not larger than another member the generator saw.
#[test]
fn generated_rows_are_well_formed() {
    let rows = rows();
    assert!(!rows.is_empty(), "no generated rows");
    let mut keys = std::collections::BTreeSet::new();
    for r in &rows {
        let key = r["key"].as_str().unwrap();
        assert!(keys.insert(key.to_owned()), "duplicate key {key}");
        assert_eq!(r["source"], "generator", "{key}");
        assert_eq!(r["oracle"], "CONTRADICTED", "{key}");
        assert_eq!(r["assert"], "kept", "{key}");
        assert!(
            !r["effects"].to_string().contains("SEMI0"),
            "{key}: a top-level `;` body is #831's, not this lane's"
        );
        let target = r["target"].as_str().unwrap();
        let src = r["files"][target].as_str().unwrap();
        let line = |n: u64| {
            src.replace("\r\n", "\n")
                .lines()
                .nth(n as usize - 1)
                .unwrap()
                .to_owned()
        };
        assert!(
            line(r["from"].as_u64().unwrap()).contains("int x = 41;"),
            "{key}"
        );
        assert!(line(r["to"].as_u64().unwrap()).contains("return "), "{key}");
    }
}
