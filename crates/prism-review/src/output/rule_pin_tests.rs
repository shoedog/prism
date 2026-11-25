use prism::rule_registry::{check_emitted, RuleRef};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const FORMATS: &str = "json review json-taint sarif sarif-taint targets";
const ALGORITHMS: &str = "taint primitive absence contract echo membrane symmetry \
    provenance peer_consistency callback_dispatcher";
type Counts = BTreeMap<String, (usize, usize)>;

fn array<'a>(object: &'a Value, key: &str) -> Result<&'a [Value], String> {
    object[key]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("missing or invalid {key} array"))
}

fn check_capture(format: &str, document: &Value, counts: &mut Counts) -> Result<(), String> {
    let mut copies = Vec::new();
    match format {
        "json" | "review" => {
            for result in array(document, "results")? {
                copies.extend(array(result, "findings")?);
            }
            copies.extend(array(document, "all_findings")?);
        }
        "json-taint" => copies.extend(array(document, "findings")?),
        "sarif" | "sarif-taint" => {
            for run in array(document, "runs")? {
                copies.extend(
                    array(run, "results")?
                        .iter()
                        .map(|result| &result["properties"]),
                );
            }
        }
        "targets" => {
            array(document, "targets")?;
        }
        _ => return Err(format!("unknown capture format {format}")),
    }

    for (index, finding) in copies.into_iter().enumerate() {
        let algorithm = finding["algorithm"]
            .as_str()
            .filter(|algorithm| !algorithm.is_empty())
            .ok_or_else(|| format!("{format} finding copy {index}: missing algorithm"))?;
        let (findings, refs) = counts.entry(algorithm.to_owned()).or_default();
        *findings += 1;
        if let Some(raw) = finding.get("rules") {
            for rule in Vec::<RuleRef>::deserialize(raw).map_err(|error| error.to_string())? {
                check_emitted(algorithm, &rule).map_err(|error| {
                    format!("{format} finding copy {index} ({algorithm}): {error}")
                })?;
                *refs += 1;
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires host Stage M captures"]
fn check_stage_m_pins() {
    let manifest =
        PathBuf::from(env::var_os("PRISM_RULE_PIN_MANIFEST").expect("pin manifest path"));
    let outputs = PathBuf::from(env::var_os("PRISM_RULE_PIN_OUTPUTS").expect("pin output path"));
    assert!(manifest.is_absolute() && outputs.is_absolute());
    assert!(
        outputs.is_dir() && !outputs.join("FAILED").exists(),
        "invalid capture directory {outputs:?}"
    );
    let input =
        fs::read_to_string(&manifest).unwrap_or_else(|error| panic!("{manifest:?}: {error}"));
    let mut keys = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut totals = BTreeMap::<&str, Counts>::new();
    for line in input
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && *line != "id\trepo\tdiff")
    {
        let fields: Vec<_> = line.split('\t').collect();
        let [id, repo, diff] = fields.as_slice() else {
            panic!("invalid pin manifest row: {line:?}");
        };
        assert_eq!(
            Path::new(id).file_name().and_then(|name| name.to_str()),
            Some(*id)
        );
        assert!(
            !repo.is_empty() && !diff.is_empty() && sources.insert((*repo, *diff)),
            "invalid input: {line:?}"
        );
        assert!(keys.insert(*id), "duplicate capture key {id:?}");
        for format in FORMATS.split_whitespace() {
            let path = outputs.join(id).join(format!("{format}.json"));
            let raw = fs::read(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
            let document =
                serde_json::from_slice(&raw).unwrap_or_else(|error| panic!("{path:?}: {error}"));
            check_capture(format, &document, totals.entry(format).or_default())
                .unwrap_or_else(|error| panic!("{path:?}: {error}"));
        }
    }
    assert_eq!(keys.len(), 35, "Stage M input count");
    println!("inputs={}", keys.len());
    for format in FORMATS.split_whitespace() {
        let counts = totals.get_mut(format).unwrap();
        for algorithm in ALGORITHMS.split_whitespace() {
            counts.entry(algorithm.into()).or_default();
        }
        let (findings, refs) = counts
            .values()
            .fold((0, 0), |(f, r), (df, dr)| (f + df, r + dr));
        println!(
            "{format}: rows={} findings={findings} refs={refs} by_algorithm={counts:?}",
            keys.len()
        );
        assert_eq!(refs, 0, "S2-R has no activated producer in {format}");
    }
}

#[test]
fn pin_reader_checks_every_finding_copy_and_its_algorithm() {
    let neutral = json!({"algorithm":"taint","rules":[{"id":"legacy"}]});
    let bad = json!({"algorithm":"taint","rules":[{"id":"unknown"}]});
    let multi = json!({"results":[{"findings":[neutral]}],"all_findings":[neutral]});
    let single = json!({"findings":[neutral, neutral]});
    let sarif = json!({"runs":[{"results":[{"properties":neutral},{"properties":neutral}]}]});
    for format in FORMATS
        .split_whitespace()
        .filter(|&format| format != "targets")
    {
        let (mut document, later) = match format {
            "json" | "review" => (multi.clone(), "/all_findings/0"),
            "json-taint" => (single.clone(), "/findings/1"),
            _ => (sarif.clone(), "/runs/0/results/1/properties"),
        };
        let mut counts = Default::default();
        check_capture(format, &document, &mut counts).unwrap();
        assert_eq!(counts["taint"], (2, 2));
        document.pointer_mut(later).unwrap()["algorithm"] = json!("primitive");
        assert!(check_capture(format, &document, &mut counts)
            .unwrap_err()
            .contains("not registered"));
        *document.pointer_mut(later).unwrap() = bad.clone();
        assert!(check_capture(format, &document, &mut counts)
            .unwrap_err()
            .contains("unknown rule id"));
        *document.pointer_mut(later).unwrap() = Value::Null;
        assert!(check_capture(format, &document, &mut counts).is_err());
    }
}

#[test]
fn pin_reader_requires_the_output_collections() {
    for format in FORMATS.split_whitespace() {
        assert!(check_capture(format, &json!({}), &mut Default::default()).is_err());
    }
    let empty = json!({"findings":[]});
    let mut counts = Default::default();
    check_capture("json-taint", &empty, &mut counts).unwrap();
    assert!(counts.is_empty());
}
