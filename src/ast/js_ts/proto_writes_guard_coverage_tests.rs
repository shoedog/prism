//! Refusal-branch coverage for the guard proofs. Each family takes a witness
//! (the control, which keeps its guard) and changes one premise per case; a
//! case pins the exact fact states and counted refusals, in every language.
use super::proto_writes::GuardState;
use crate::{ast::ParsedFile, languages::Language};
use std::collections::BTreeMap;

const LANGUAGES: [(&str, Language); 3] = [
    ("js", Language::JavaScript),
    ("ts", Language::TypeScript),
    ("tsx", Language::Tsx),
];

pub(super) struct Case {
    pub(super) label: &'static str,
    pub(super) edits: &'static [(&'static str, &'static str)],
    /// One letter per fact in source order: N none, P partial, S sufficient.
    pub(super) states: &'static str,
    pub(super) refusals: &'static [(&'static str, usize)],
}

fn observe(source: &str, ext: &str, language: Language) -> (String, BTreeMap<&'static str, usize>) {
    let file = ParsedFile::parse(&format!("case.{ext}"), source, language).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    let states = file
        .js_ts_proto_writes()
        .iter()
        .map(|fact| match fact.guard {
            GuardState::None => 'N',
            GuardState::Partial => 'P',
            GuardState::Sufficient => 'S',
        })
        .collect();
    (states, file.js_ts_proto_write_guard_refusals())
}

pub(super) fn witness(name: &str) -> String {
    let path = format!(
        "{}/tests/proto_writes_matrix/cases/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

/// The TypeScript witness with its annotations erased; the control keeps its
/// runtime behaviour in all three grammars.
pub(super) fn erased(name: &str) -> String {
    let mut source = witness(name);
    for annotation in [
        ": Record<string, Record<string, unknown> | undefined>",
        ": asserts key is \"draft\"",
        ": Record<string, unknown>",
        ": [string, string]",
        ": unknown",
        ": string[]",
        ": string",
        ": key is \"draft\"",
    ] {
        source = source.replace(annotation, "");
    }
    source
}

/// `control` must keep a sufficient fact; a `neg` case must not, a `pos`
/// case must.
pub(super) fn run(family: &str, base: &str, cases: &[Case]) {
    run_in(&LANGUAGES, family, base, cases);
}

/// The TypeScript grammars only: for syntax JavaScript cannot parse.
pub(super) fn run_ts(family: &str, base: &str, cases: &[Case]) {
    run_in(&LANGUAGES[1..], family, base, cases);
}

fn run_in(languages: &[(&str, Language)], family: &str, base: &str, cases: &[Case]) {
    let mut failures = Vec::new();
    for case in cases {
        let mut source = base.to_owned();
        for (before, after) in case.edits {
            assert!(
                source.contains(before),
                "{family}/{}: {before:?}",
                case.label
            );
            source = source.replacen(before, after, 1);
        }
        for &(ext, language) in languages {
            let (states, refusals) = observe(&source, ext, language);
            let sufficient = states.contains('S');
            let level_matches = match case.label.split(' ').next().unwrap() {
                "control" | "pos" => sufficient,
                "neg" => !sufficient,
                _ => true,
            };
            let want: BTreeMap<_, _> = case.refusals.iter().copied().collect();
            if !level_matches || states != case.states || refusals != want {
                failures.push(format!(
                    "{family}/{} [{ext}]: got {states} {refusals:?}, want {} {want:?}\n{source}",
                    case.label, case.states
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every premise of the header redirect, one at a time.
#[test]
fn header_redirect_premises() {
    let base = witness("g28.js");
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "PS",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg undeclared receiver",
            edits: &[(
                r#"let section = out;
"#,
                r#""#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg capture index",
            edits: &[(r#"header[1]"#, r#"header[2]"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg test not exec",
            edits: &[(r#".exec(line)"#, r#".test(line)"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg other pattern",
            edits: &[(r#"/^\[([^\]]+)\]$/"#, r#"/^\[(.+)\]$/"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg check on another name",
            edits: &[(r#"name === '__proto__'"#, r#"name === 'constructor'"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg consequence is not a store",
            edits: &[(r#"section = scratch;"#, r#"continue;"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg alternative has two statements",
            edits: &[(
                r#"section = out[name] = out[name] || {};"#,
                r#"out[name] = out[name] || {}; section = out[name];"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg install is not an assignment",
            edits: &[(
                r#"section = out[name] = out[name] || {};"#,
                r#"section = out[name] || {};"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg slot is a member",
            edits: &[(
                r#"section = out[name] = out[name] || {};"#,
                r#"section = out.x = out[name] || {};"#,
            )],
            states: "P",
            refusals: &[("incomplete_coverage", 1), ("unsupported_predicate", 1)],
        },
        Case {
            label: "neg lookup is not a fallback",
            edits: &[(
                r#"section = out[name] = out[name] || {};"#,
                r#"section = out[name] = out[name];"#,
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg no skip",
            edits: &[(
                r#"      continue;
"#,
                r#""#,
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg statement after skip",
            edits: &[(
                r#"      continue;
"#,
                r#"      continue;
      foo();
"#,
            )],
            states: "PP",
            refusals: &[
                ("incomplete_coverage", 2),
                ("unsupported_predicate", 1),
                ("wrong_exit", 1),
            ],
        },
        Case {
            label: "neg labelled skip",
            edits: &[(
                r#"      continue;
"#,
                r#"      continue outer;
"#,
            )],
            states: "PP",
            refusals: &[
                ("incomplete_coverage", 2),
                ("unsupported_predicate", 1),
                ("wrong_exit", 1),
            ],
        },
        Case {
            label: "neg check is not a comparison",
            edits: &[(r#"name === '__proto__'"#, r#"!name"#)],
            states: "NN",
            refusals: &[],
        },
        Case {
            label: "neg check outside the header branch",
            edits: &[(
                r#"    if (header) {
      const name = header[1];
      if (name === '__proto__') {
        section = scratch;
      } else {
        section = out[name] = out[name] || {};
      }
      continue;
    }
"#,
                r#"    const name = header[1];
    if (name === '__proto__') {
      section = scratch;
    } else if (header) {
      section = out[name] = out[name] || {};
    }
"#,
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 2)],
        },
        Case {
            label: "neg parenthesised redirect target",
            edits: &[(r#"section = scratch;"#, r#"(section) = scratch;"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg parenthesised advance target",
            edits: &[(r#"section = out[name] ="#, r#"(section) = out[name] ="#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg another write to the receiver",
            edits: &[(
                r#"const separator"#,
                r#"if (text) section = out;
    const separator"#,
            )],
            states: "PP",
            refusals: &[
                ("incomplete_coverage", 2),
                ("unsupported_predicate", 1),
                ("wrong_version", 1),
            ],
        },
        Case {
            label: "neg consequence has two statements",
            edits: &[(r#"section = scratch;"#, r#"section = scratch; foo();"#)],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
        Case {
            label: "neg alternative block has two statements",
            edits: &[(
                r#"} else {
        section"#,
                r#"} else {
        foo();
        section"#,
            )],
            states: "PP",
            refusals: &[("incomplete_coverage", 2), ("unsupported_predicate", 2)],
        },
    ];
    run("header_redirect_premises", &base, &cases);
}

/// Flow predicates fail closed on with statements and finalizer exits.
#[test]
fn flow_premises() {
    let base = r#"function put(root, raw, field) {
  const key = String(raw);
  if (key === '__proto__' || key === 'constructor') return;
  root[key][field] = 1;
}
"#
    .to_owned();
    let cases = [
        Case {
            label: "control",
            edits: &[],
            states: "S",
            refusals: &[],
        },
        Case {
            label: "neg with statement",
            edits: &[(
                r#"  root[key][field] = 1;"#,
                r#"  with (root) { root[key][field] = 1; }"#,
            )],
            states: "N",
            refusals: &[("unmodelled_flow", 1)],
        },
        Case {
            label: "pos store inside a ternary condition",
            edits: &[(
                r#"  root[key][field] = 1;"#,
                r#"  const out = (root[key][field] = 1) ? 1 : 0;"#,
            )],
            states: "S",
            refusals: &[],
        },
    ];
    run("flow_premises", &base, &cases);
}
