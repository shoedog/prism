use crate::doc_cases::{apply_patches, fixture, json_fixture, schema, Case};
use crate::doc_wire::{decode, serialize_dto};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

pub fn check(case: &Case<'_>) -> Result<(), String> {
    if case.scenario == "hashes" {
        return hashes();
    }
    if case.data["expect"].get("document").is_some() {
        return decode(case);
    }
    let mut baseline = case.baseline();
    let kind = if baseline.get("report_id").is_some() {
        "impact"
    } else {
        "bindings"
    };
    let other = if kind == "impact" {
        "bindings"
    } else {
        "impact"
    };
    apply_patches(&mut baseline, &case.data["control_patches"]);
    let mut input = baseline.clone();
    apply_patches(&mut input, &case.data["patches"]);
    if case.flag("opaque_match") {
        let site_id = "opaque: site/α ~ 01";
        for pair in [0, 2] {
            let old = input["comparisons"][pair]["matches"]
                .as_object_mut()
                .unwrap()
                .remove("call-before")
                .unwrap();
            input["comparisons"][pair]["matches"][site_id] = old;
        }
        for reference in ["match_before", "match_service_first"] {
            input["usages"][0][reference]["site_id"] = json!(site_id);
        }
    }
    if let Some(version) = case.data["schema_version"].as_str() {
        if input["schema_version"] != version {
            return Err(format!("schema_version\texpected {version}"));
        }
    }
    let cross = case.flag("cross");
    if case.flag("dto") || cross {
        input = serialize_dto(&input);
    }

    let (own, opposite) = match (load(kind), load(other)) {
        (Ok(own), Ok(opposite)) => (own, opposite),
        (own, opposite) => {
            let errors: Vec<_> = [own, opposite]
                .into_iter()
                .filter_map(Result::err)
                .collect();
            return Err(format!("schema_file\t{}", errors.join("; ")));
        }
    };
    let expected = case.data["expect"]["valid"].as_bool().unwrap();
    validate(&own, &opposite, &baseline, &input, cross, expected)
}

fn load(kind: &str) -> Result<Value, String> {
    let name = format!("boundary-{kind}.schema.json");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join(&name);
    let bytes = std::fs::read(path).map_err(|error| format!("{name}: {error}"))?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("{name}: {error}"))?;
    if value["$schema"] != DIALECT || value["$id"] != format!("urn:prism:boundary:{kind}:0.1") {
        return Err(format!("{name}: unexpected dialect or stable ID"));
    }
    Ok(value)
}

fn validate(
    own: &Value,
    opposite: &Value,
    control: &Value,
    doc: &Value,
    cross: bool,
    expected: bool,
) -> Result<(), String> {
    let python =
        std::env::var_os("PRISM_BOUNDARY_SCHEMA_PYTHON").unwrap_or_else(|| "python3".into());
    let mut child = Command::new(python)
        .args(["-c", include_str!("doc_validate_schema.py")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("schema_validator\t{error}"))?;
    let input =
        json!({"own":own,"opposite":opposite,"control":control,"cross":cross,"instance":doc});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .map_err(|error| format!("schema_validator\t{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("schema_validator\t{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "schema_validator\t{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let verdict: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("schema_validator\t{error}"))?;
    if verdict["control_error"].is_string() {
        return Err(format!(
            "schema_control\town golden rejected: {}",
            verdict["control_error"]
        ));
    }
    if verdict["own_instance_error"].is_string() {
        return Err(format!(
            "schema_control\town DTO rejected: {}",
            verdict["own_instance_error"]
        ));
    }
    if verdict["valid"] != expected {
        return Err(format!(
            "schema_assertion\texpected valid={expected}: {}",
            verdict["error"]
        ));
    }
    Ok(())
}

fn hashes() -> Result<(), String> {
    fn projected(bytes: &[u8]) -> (Value, String, String) {
        let index = schema(bytes, "service-before");
        let seed = json_fixture("producer.json");
        let dto: prism_boundary::BindingsDocument = serde_json::from_value(seed).unwrap();
        let actual = serde_json::to_value(dto.with_schema_inventory(&index).unwrap()).unwrap();
        (actual, index.byte_sha256, index.content_hash)
    }

    fn numeric(spelling: &str) -> Vec<u8> {
        format!(r#"{{"openapi":"3.1.0","paths":{{"/kiln":{{"post":{{"requestBody":{{"content":{{"application/json":{{"schema":{{"type":"object","properties":{{"x":{{"enum":[{spelling}]}}}}}}}}}}}}}}}}}}}}"#)
            .into_bytes()
    }

    let plain = projected(&fixture("inventory.json"));
    let formatted = projected(&fixture("formatted"));
    let status = projected(&fixture("status.json"));
    let numbers: Vec<_> = ["1", "1.0", "1e0", "\"1\""]
        .iter()
        .map(|n| projected(&numeric(n)))
        .collect();
    assert_ne!(plain.1, formatted.1, "formatting changes byte identity");
    assert_eq!(
        plain.2, formatted.2,
        "formatting preserves structural identity"
    );
    assert_ne!(
        plain.2, status.2,
        "explicit bodyless status participates in content_hash"
    );
    assert_eq!(numbers[0].2, numbers[1].2);
    assert_eq!(numbers[0].2, numbers[2].2);
    assert_ne!(numbers[0].2, numbers[3].2);

    for (actual, sha, content) in [plain, formatted, status].iter().chain(&numbers) {
        if actual.pointer("/schema/source/sha256") != Some(&json!(sha))
            || actual.pointer("/schema/content_hash") != Some(&json!(content))
        {
            return Err(
                "hash_projection\tDTO must preserve byte and structural hashes separately".into(),
            );
        }
    }
    Ok(())
}
