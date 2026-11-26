#[path = "support/admission.rs"]
mod admission;

use prism_boundary::index_schema;
use prism_boundary::revision::{Format, Snapshot};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs, io};

enum Pin {
    Verified(Vec<u8>),
    Missing(String),
    Invalid(String),
}

fn verify(pin: &Value) -> Pin {
    let variable = pin["env"].as_str().unwrap();
    let path = env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| pin["default_path"].as_str().map(PathBuf::from));
    let Some(path) = path else {
        return Pin::Missing(format!(
            "set {variable} to the staged {} bytes",
            pin["blob_path"]
        ));
    };
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with("PRIVATE"))
    {
        return Pin::Invalid(format!("{variable} must point to a staged public file"));
    }
    let canonical = match fs::canonicalize(&path) {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Pin::Missing(format!("{variable} is missing at {}", path.display()));
        }
        Err(error) => return Pin::Invalid(format!("cannot read {variable}: {error}")),
    };
    if canonical
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with("PRIVATE"))
    {
        return Pin::Invalid(format!("{variable} must point to a staged public file"));
    }
    let bytes = match fs::read(canonical) {
        Ok(bytes) => bytes,
        Err(error) => return Pin::Invalid(format!("cannot read {variable}: {error}")),
    };
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let expected = pin["sha256"].as_str().unwrap();
    if digest != expected {
        return Pin::Invalid(format!("{variable} sha256 {digest}, expected {expected}"));
    }
    Pin::Verified(bytes)
}

#[test]
#[ignore = "needs the staged public inputs: run with `-- --ignored` and PRISM_BOUNDARY_PUBLIC_SWAGGER set"]
fn public_ids_match_pinned_bytes() {
    let manifest: Value = serde_json::from_str(include_str!("fixtures/public-pins.json")).unwrap();
    let swagger = verify(&manifest["swagger"]);
    let golden = verify(&manifest["ids"]);
    let (Pin::Verified(swagger), Pin::Verified(golden)) = (&swagger, &golden) else {
        let reasons: Vec<_> = [&swagger, &golden]
            .iter()
            .filter_map(|pin| match pin {
                Pin::Missing(reason) | Pin::Invalid(reason) => Some(reason.as_str()),
                Pin::Verified(_) => None,
            })
            .collect();
        let pending = format!("PENDING public IDs golden: {}", reasons.join(", "));
        // Write directly so a successful missing-input override remains visible in libtest.
        writeln!(io::stderr().lock(), "{pending}").expect("report public golden pending status");
        let missing_only =
            !matches!(swagger, Pin::Invalid(_)) && !matches!(golden, Pin::Invalid(_));
        assert!(
            missing_only && env::var("PRISM_BOUNDARY_PUBLIC_PENDING_OK").as_deref() == Ok("1"),
            "{pending}"
        );
        return;
    };
    let pin = &manifest["swagger"];
    let commit = pin["commit"].as_str().unwrap();
    let mut input = admission::admitted(swagger, Format::Json, "pilot-producer", None, &[]);
    input.snapshot = Snapshot::Commit {
        tree: "pinned-public-tree",
        commit,
        subject: None,
        committer_date: None,
    };
    input.entry.path = pin["blob_path"].as_str().unwrap();
    input.entry.tree = input.snapshot.tree();
    input.entry.oid = pin["sha256"].as_str().unwrap();
    let index = index_schema(&input).expect("admitted pinned Swagger inventory");
    let actual = index.public_ids_tsv();
    let difference = actual
        .iter()
        .zip(golden)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| actual.len().min(golden.len()));
    assert!(actual == *golden,
        "public ID bytes must match pinned golden including order and final LF; first difference at byte {difference}, got {} bytes, expected {}",
        actual.len(), golden.len());
}
