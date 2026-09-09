use super::*;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

// Compile-time custody for EVERY transitive local worker dependency. A modified
// checkout cannot supply new authority merely by printing matching packet hashes.
macro_rules! assets { ($($name:literal),+ $(,)?) => { &[$(($name, include_bytes!(concat!("../../scripts/callable-observations/", $name)) as &[u8])),+] }; }
const ASSETS: &[(&str, &[u8])] = assets![
    "schema.mjs",
    "index.mjs",
    "worker.mjs",
    "inventory.mjs",
    "provenance.mjs",
    "nested.mjs",
    "props-class.mjs",
    "exact-ambient.mjs",
    "wildcard.mjs",
    "merged-wildcard.mjs",
    "required-paths.mjs",
    "type-lib.mjs",
    "entries.mjs",
    "search-provenance.mjs",
    "identity-domains.mjs",
    "lib-search.mjs",
    "config-provenance.mjs",
    "entry-obligations.mjs",
    "semantic-closure.mjs",
    "semantic-closure-v2.mjs",
    "semantic-closure-v3.mjs",
    "direct-owner.mjs",
    "detached-owner-worker.mjs",
];
const CAP: u64 = 16 * 1024 * 1024;
fn assets_current(directory: &Path) -> Result<()> {
    for (name, expected) in ASSETS {
        let file = directory.join(name);
        let meta = std::fs::symlink_metadata(&file).map_err(|_| "producer_unavailable")?;
        ensure(
            meta.is_file() && !meta.is_symlink() && meta.len() == expected.len() as u64,
            "producer_changed",
        )?;
        ensure(
            std::fs::read(file).map_err(|_| "producer_unavailable")? == *expected,
            "producer_changed",
        )?;
    }
    Ok(())
}
fn run(directory: &Path, input: &[u8]) -> Result<Observation> {
    assets_current(directory)?;
    let mut child = Command::new("node")
        .arg("--max-old-space-size=512")
        .arg(directory.join("detached-owner-worker.mjs"))
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "compiler_worker_unavailable")?;
    let stdout = child.stdout.take().ok_or("worker_stdout")?;
    let stderr = child.stderr.take().ok_or("worker_stderr")?;
    let read = |stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stream.take(CAP + 1).read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let out = read(Box::new(stdout));
    let err = read(Box::new(stderr));
    let input_result = child.stdin.take().ok_or("worker_stdin")?.write_all(input);
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if start.elapsed() < Duration::from_secs(35) && input_result.is_ok() => {
                thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let bytes = out
        .join()
        .map_err(|_| "worker_read")?
        .map_err(|_| "worker_read")?;
    let errors = err
        .join()
        .map_err(|_| "worker_read")?
        .map_err(|_| "worker_read")?;
    assets_current(directory)?;
    ensure(
        status.is_some_and(|s| s.success()) && errors.is_empty() && bytes.len() as u64 <= CAP,
        "worker_refused",
    )?;
    serde_json::from_slice(&bytes).map_err(|_| "worker_schema".into())
}
pub(super) fn reproduce(
    root: &Path,
    config: &str,
    compiler: &Path,
    files: &BTreeMap<String, ParsedFile>,
) -> Result<Observation> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/callable-observations");
    let input = serde_json::to_vec(
        &serde_json::json!({"root":root,"config":config,"compiler":compiler,
        "profile":"default","links":"reject"}),
    )
    .map_err(|_| "acquisition_options")?;
    ensure(input.len() <= 16384, "acquisition_options")?;
    let first = run(&directory, &input)?;
    validate_inputs(&first, files)?;
    // A supplied map is not proof of the full Prism index. Load the actual root
    // between compiler snapshots and compare its complete JS/TS source census.
    // Bound the loader using the first authenticated inventory before parsing.
    let manifest = first.packet["snapshot"]["files"]
        .as_array()
        .ok_or("input_manifest")?;
    loader_budget(manifest)?;
    let loaded = crate::repo_loader::load_repo(root).map_err(|_| "index_unavailable")?;
    ensure(
        !loaded.skipped.iter().any(|s| {
            Language::from_path(&s.path).is_some_and(js)
                && matches!(
                    s.reason,
                    crate::repo_loader::SkipReason::Unreadable
                        | crate::repo_loader::SkipReason::NotUtf8
                        | crate::repo_loader::SkipReason::ParseFailed
                        | crate::repo_loader::SkipReason::TooLarge { .. }
                        | crate::repo_loader::SkipReason::Symlink
                )
        }),
        "index_incomplete",
    )?;
    let actual: BTreeMap<_, _> = loaded
        .files
        .iter()
        .filter(|(_, p)| js(p.language))
        .map(|(f, p)| (f.clone(), hash(p.source.as_bytes())))
        .collect();
    let supplied: BTreeMap<_, _> = files
        .iter()
        .map(|(f, p)| (f.clone(), hash(p.source.as_bytes())))
        .collect();
    ensure(actual == supplied, "prism_input_set")?;
    let second = run(&directory, &input)?;
    ensure(first == second, "unreproduced_evidence")?;
    Ok(second)
}

fn loader_budget(manifest: &[serde_json::Value]) -> Result<()> {
    let supported: Vec<_> = manifest
        .iter()
        .filter(|r| {
            r["id"]
                .as_str()
                .and_then(|p| p.strip_prefix("project/"))
                .is_some_and(|p| Language::from_path(p).is_some())
        })
        .collect();
    ensure(
        supported.len() <= INPUT_FILE_LIMIT
            && supported
                .iter()
                .try_fold(0u64, |sum, r| sum.checked_add(r["size"].as_u64()?))
                .is_some_and(|bytes| bytes <= INPUT_BYTE_LIMIT as u64),
        "loader_budget",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loader_inventory_budget_counts_all_supported_sources_and_checks_sizes() {
        let row = serde_json::json!({"id":"project/outside/effect.rs","size":1});
        assert!(loader_budget(&vec![row.clone(); 512]).is_ok());
        assert!(loader_budget(&vec![row; 513]).is_err());
        for size in [
            serde_json::json!(8 * 1024 * 1024 + 1),
            serde_json::json!(-1),
            serde_json::Value::Null,
        ] {
            assert!(
                loader_budget(&[serde_json::json!({"id":"project/extra.ts","size":size})]).is_err()
            );
        }
        assert!(loader_budget(&[
            serde_json::json!({"id":"project/extra.ts","size":8 * 1024 * 1024})
        ])
        .is_ok());
        assert!(
            loader_budget(&[serde_json::json!({"id":"compiler/lib.d.ts","size":u64::MAX})]).is_ok()
        );
    }
    #[test]
    fn every_pinned_worker_asset_must_be_present_and_byte_identical() {
        let root = tempfile::tempdir().unwrap();
        for (name, bytes) in ASSETS {
            std::fs::write(root.path().join(name), bytes).unwrap();
        }
        assert!(assets_current(root.path()).is_ok());
        for (name, bytes) in ASSETS {
            let mut changed = bytes.to_vec();
            changed[0] ^= 1;
            std::fs::write(root.path().join(name), changed).unwrap();
            assert!(assets_current(root.path()).is_err(), "{name}");
            std::fs::write(root.path().join(name), bytes).unwrap();
        }
        std::fs::remove_file(root.path().join(ASSETS[0].0)).unwrap();
        assert!(assets_current(root.path()).is_err());
    }
}
