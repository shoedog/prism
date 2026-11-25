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
const INPUT_COUNT: usize = 35;
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

fn resolve_pin_path(
    root: &Path,
    id: &str,
    field: &str,
    raw: &str,
    expected_type: fn(&Path) -> bool,
) -> Result<PathBuf, String> {
    let invalid = || format!("{id}: invalid {field} {raw:?}");
    if raw.is_empty() {
        return Err(invalid());
    }
    let resolved = root
        .join(raw)
        .canonicalize()
        .map_err(|error| format!("{}: {error}", invalid()))?;
    if !resolved.starts_with(root) {
        return Err(format!("{}: outside pinned input root {root:?}", invalid()));
    }
    if !expected_type(&resolved) {
        return Err(format!("{}: unexpected file type", invalid()));
    }
    Ok(resolved)
}

fn read_stage_m_pins(
    manifest: &Path,
    input_root: &Path,
    outputs: &Path,
) -> Result<(usize, BTreeMap<&'static str, Counts>), String> {
    if !outputs.is_dir() || outputs.join("FAILED").exists() {
        return Err(format!("invalid capture directory {outputs:?}"));
    }
    let root = input_root
        .canonicalize()
        .map_err(|error| format!("invalid pinned input root {input_root:?}: {error}"))?;
    if !root.is_dir() {
        return Err(format!("pinned input root {root:?} is not a directory"));
    }
    let input = fs::read_to_string(manifest).map_err(|error| format!("{manifest:?}: {error}"))?;
    let mut keys = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut totals = BTreeMap::<&str, Counts>::new();
    for line in input.lines().filter(|line| {
        !line.is_empty()
            && !line.starts_with('#')
            && *line != "id\trepo\tdiff"
            && *line != "name\trepo\tdiff"
    }) {
        let fields: Vec<_> = line.split('\t').collect();
        let [id, repo, diff] = fields.as_slice() else {
            return Err(format!("invalid pin manifest row: {line:?}"));
        };
        if Path::new(id).file_name().and_then(|name| name.to_str()) != Some(*id) {
            return Err(format!("invalid capture key {id:?}"));
        }
        if !keys.insert(*id) {
            return Err(format!("duplicate capture key {id:?}"));
        }
        let repo = resolve_pin_path(&root, id, "repo", repo, Path::is_dir)?;
        let diff = resolve_pin_path(&root, id, "diff", diff, Path::is_file)?;
        if !sources.insert((repo, diff)) {
            return Err(format!("duplicate pin input: {line:?}"));
        }
        for format in FORMATS.split_whitespace() {
            let path = outputs.join(id).join(format);
            let raw = fs::read(&path).map_err(|error| format!("{path:?}: {error}"))?;
            let document =
                serde_json::from_slice(&raw).map_err(|error| format!("{path:?}: {error}"))?;
            check_capture(format, &document, totals.entry(format).or_default())
                .map_err(|error| format!("{path:?}: {error}"))?;
        }
    }
    if keys.len() != INPUT_COUNT {
        return Err(format!(
            "Stage M input count: {} != {INPUT_COUNT}",
            keys.len()
        ));
    }
    Ok((keys.len(), totals))
}

#[test]
#[ignore = "requires host Stage M captures"]
fn check_stage_m_pins() {
    let manifest =
        PathBuf::from(env::var_os("PRISM_RULE_PIN_MANIFEST").expect("pin manifest path"));
    let input_root =
        PathBuf::from(env::var_os("PRISM_RULE_PIN_INPUT_ROOT").expect("pinned input root path"));
    let outputs = PathBuf::from(env::var_os("PRISM_RULE_PIN_OUTPUTS").expect("pin output path"));
    assert!(manifest.is_absolute() && input_root.is_absolute() && outputs.is_absolute());
    let (inputs, mut totals) = read_stage_m_pins(&manifest, &input_root, &outputs)
        .unwrap_or_else(|error| panic!("{error}"));
    println!("inputs={inputs}");
    for format in FORMATS.split_whitespace() {
        let counts = totals.get_mut(format).unwrap();
        for algorithm in ALGORITHMS.split_whitespace() {
            counts.entry(algorithm.into()).or_default();
        }
        let (findings, refs) = counts
            .values()
            .fold((0, 0), |(f, r), (df, dr)| (f + df, r + dr));
        println!("{format}: rows={inputs} findings={findings} refs={refs} by_algorithm={counts:?}");
        assert_eq!(refs, 0, "S2-R has no activated producer in {format}");
    }
}

fn check_bad_pin_input(repo: &str, diff: &str, invalid_field: &str) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("pinned");
    let outputs = temp.path().join("captures");
    let manifest = temp.path().join("inputs.tsv");
    fs::create_dir_all(root.join("repo")).unwrap();
    let empty_capture = r#"{"results":[],"all_findings":[],"findings":[],"runs":[],"targets":[]}"#;
    let mut rows = String::from("id\trepo\tdiff\n");
    for i in 0..INPUT_COUNT {
        fs::write(root.join(format!("{i}.diff")), "").unwrap();
        rows.push_str(&format!("{i}\trepo\t{i}.diff\n"));
        let dir = outputs.join(i.to_string());
        fs::create_dir_all(&dir).unwrap();
        for format in FORMATS.split_whitespace() {
            fs::write(dir.join(format), empty_capture).unwrap();
        }
    }
    fs::write(&manifest, &rows).unwrap();
    assert_eq!(
        read_stage_m_pins(&manifest, &root, &outputs).unwrap().0,
        INPUT_COUNT
    );
    let last = INPUT_COUNT - 1;
    let rows = rows.replace(
        &format!("{last}\trepo\t{last}.diff"),
        &format!("{last}\t{repo}\t{diff}"),
    );
    fs::write(&manifest, rows).unwrap();
    let error = read_stage_m_pins(&manifest, &root, &outputs).unwrap_err();
    assert!(
        error.contains(&format!("{last}: invalid {invalid_field}")),
        "{error}"
    );
}

#[test]
fn pin_reader_rejects_missing_diff_with_intact_captures() {
    check_bad_pin_input("repo", "missing.diff", "diff");
}

#[test]
fn pin_reader_rejects_missing_repo_with_intact_captures() {
    check_bad_pin_input("missing-repo", &format!("{}.diff", INPUT_COUNT - 1), "repo");
}

#[test]
fn pin_reader_requires_the_expected_input_types_inside_the_root() {
    let diff = format!("{}.diff", INPUT_COUNT - 1);
    check_bad_pin_input(&diff, &diff, "repo");
    check_bad_pin_input("repo", "repo", "diff");
    check_bad_pin_input("../captures", &diff, "repo");
    check_bad_pin_input("repo", "../inputs.tsv", "diff");
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
