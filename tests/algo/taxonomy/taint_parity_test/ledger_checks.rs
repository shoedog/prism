use super::*;
use std::collections::BTreeMap;
use std::process::Command;

fn ledger(test: &str, id: &str, run: impl FnOnce(&Row, &Fixture)) -> Option<String> {
    const CHILD: &str = "PRISM_PARITY_LEDGER_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let row = rows().into_iter().find(|row| row.id == id).unwrap();
        run(&row, &Fixture::load(&row));
        return None;
    }
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("taint_parity_test::ledger_checks::{test}"),
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("PRISM_TAINT_CROSSING_TRACE", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8(child.stdout).unwrap();
    let stderr = String::from_utf8(child.stderr).unwrap();
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{stderr}"
    );
    Some(stderr)
}

fn walk_id(line: &str) -> &str {
    line.split_whitespace()
        .find_map(|field| field.strip_prefix("walk_id="))
        .unwrap_or_else(|| panic!("{line}"))
}

#[test]
fn return_flow_keeps_its_site_without_an_order_view() {
    let Some(stderr) = ledger(
        "return_flow_keeps_its_site_without_an_order_view",
        "tr2-js-wrapper",
        |row, fixture| {
            assert_eq!(review(row, fixture, true), Finding::Unlabeled);
            let two_sites = Fixture::load_with(row, |_, source| {
                *source = source.replace("exec(y);", "exec(y);\n  wrap('fixed');");
            });
            assert_eq!(review(row, &two_sites, true), Finding::Unlabeled);
            assert_eq!(walk(row, &two_sites).reaches, Verdict::Reached);
        },
    ) else {
        return;
    };
    let mut returns = BTreeMap::<usize, Vec<&str>>::new();
    for line in stderr.lines().filter(|line| {
        line.starts_with("taint-crossing ") && line.contains(":y:Def\" relation=ReturnFlow ")
    }) {
        returns
            .entry(walk_id(line).parse().unwrap())
            .or_default()
            .push(line);
    }
    assert_eq!(returns.len(), 3, "{stderr}");
    for (lines, site) in returns.values().zip([
        "site=163..170 slot=- grades=[Exact]",
        "site=- slot=- grades=[]",
        "site=163..170 slot=0 grades=[Exact]",
    ]) {
        assert!(
            lines
                .iter()
                .all(|line| { line.contains(&format!("{site} edge_grade=None ")) }),
            "{stderr}"
        );
    }
    assert!(
        stderr.lines().any(|line| {
            line.starts_with("taint-crossing ") && line.contains("edge_grade=Some(Exact) ")
        }),
        "{stderr}"
    );
}

#[test]
fn alias_refusal_omits_the_rejected_sites_grade() {
    let Some(stderr) = ledger(
        "alias_refusal_omits_the_rejected_sites_grade",
        "s2-js-alias-same-line",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited),
    ) else {
        return;
    };
    let line = stderr
        .lines()
        .find(|line| {
            line.starts_with("taint-refusal ") && line.ends_with("reason=no_matching_site")
        })
        .unwrap_or_else(|| panic!("{stderr}"));
    assert!(
        line.contains(":164..165#") && line.contains("site=- slot=- grades=[] "),
        "{line}"
    );
}

#[test]
fn shortcut_refusal_reports_its_certified_grade() {
    let Some(stderr) = ledger(
        "shortcut_refusal_reports_its_certified_grade",
        "x01",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::NotReached),
    ) else {
        return;
    };
    let line = stderr
        .lines()
        .find(|line| {
            line.starts_with("taint-refusal ") && line.ends_with("reason=shortcut_suppressed")
        })
        .unwrap_or_else(|| panic!("{stderr}"));
    assert!(
        line.contains("site=113..122 slot=0 grades=[Exact] "),
        "{line}"
    );
}

#[test]
fn sanitizer_bypass_records_are_countable_per_walk() {
    let Some(stderr) = ledger(
        "sanitizer_bypass_records_are_countable_per_walk",
        "a1-js-sanitizer-boundary-bypass",
        |row, fixture| {
            for _ in 0..2 {
                assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited);
            }
        },
    ) else {
        return;
    };
    let mut walks = BTreeMap::new();
    for line in stderr.lines() {
        let (event, fields) = line.split_once(' ').unwrap();
        let walk = fields
            .strip_prefix("walk=")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        let (kind, crossings, refusals) = walks.entry(walk_id(line)).or_insert((walk, 0, 0));
        assert_eq!(*kind, walk, "{line}");
        match event {
            "taint-crossing" => *crossings += 1,
            "taint-refusal" => *refusals += 1,
            _ => panic!("{line}"),
        }
    }
    let mut counts = walks.into_values().collect::<Vec<_>>();
    counts.sort_unstable();
    assert_eq!(
        counts,
        [
            ("bypass", 2, 1),
            ("bypass", 2, 1),
            ("primary", 3, 1),
            ("primary", 3, 1),
        ],
        "{stderr}"
    );
}
