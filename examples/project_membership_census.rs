//! Read-only research census, not production admission or runtime authority.
//! The caller must first bound a complete audit-root snapshot and must bound this
//! subprocess's time/output. Stdin IDs classify language support, never select
//! loader inputs. Usage: project_membership_census /absolute/audit/root < ids.json

use anyhow::Result;
use prism::languages::Language;
use serde_json::{json, Value};
use std::{collections::BTreeSet, io::Read, path::Path};

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_IDS: usize = 100_000;

fn valid_id(id: &str) -> bool {
    let Some(path) = id
        .strip_prefix("project/")
        .or_else(|| id.strip_prefix("compiler/"))
    else {
        return false;
    };
    id.encode_utf16().count() <= 4096
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn project_id(path: &str, directory: bool) -> Result<String> {
    if directory && matches!(path, "." | "/" | "") {
        return Ok("project/".into());
    }
    let id = format!("project/{path}");
    anyhow::ensure!(
        valid_id(if directory {
            id.trim_end_matches('/')
        } else {
            &id
        }),
        "unsafe native path"
    );
    Ok(id)
}

fn census(root: &Path, input: impl Read) -> Result<Value> {
    anyhow::ensure!(root.is_absolute(), "expected absolute audit root");
    let root = root.canonicalize()?;
    anyhow::ensure!(
        root.parent().is_some() && root.is_dir(),
        "invalid audit root"
    );
    let mut bytes = Vec::new();
    input
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= MAX_INPUT_BYTES, "input byte limit");
    let ids: Vec<String> = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(ids.len() <= MAX_IDS, "input ID limit");
    let mut unique = BTreeSet::new();
    for id in ids {
        anyhow::ensure!(valid_id(&id) && unique.insert(id), "unsafe or duplicate ID");
    }
    let loaded = prism::build_pool::install(|| prism::repo_loader::load_repo(&root))?;
    let mut files = loaded.files.iter().map(|(path, file)| Ok(json!({
        "id":project_id(path, false)?, "sha256":loaded.file_hashes[path],
        "size":file.source.len(), "language":file.language, "parse_errors":file.parse_error_count
    }))).collect::<Result<Vec<_>>>()?;
    // Match the compiler observer's JavaScript UTF-16 code-unit ordering.
    files.sort_by_cached_key(|row| {
        row["id"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect::<Vec<_>>()
    });
    let mut skipped = loaded
        .skipped
        .iter()
        .map(|skip| {
            Ok(json!({
                "id":project_id(&skip.path, true)?, "reason":skip.reason
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    skipped.sort_by_cached_key(|row| {
        (
            row["id"]
                .as_str()
                .unwrap()
                .encode_utf16()
                .collect::<Vec<_>>(),
            row["reason"].to_string().encode_utf16().collect::<Vec<_>>(),
        )
    });
    let mut supported: Vec<_> = unique
        .into_iter()
        .map(|id| {
            let language = Language::from_path(&id);
            json!({"id":id, "language":language})
        })
        .collect();
    supported.sort_by_cached_key(|row| {
        row["id"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect::<Vec<_>>()
    });
    Ok(
        json!({"schema":"prism.native-membership/1", "authorizes_runtime_edge":false,
        "files":files, "skipped":skipped, "supported":supported,
        "type_database_present":loaded.type_db.is_some()}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(args.len() == 1, "expected one absolute audit root");
    println!("{}", census(Path::new(&args[0]), std::io::stdin().lock())?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.ts"), "export const value = 1;\n").unwrap();
        dir
    }

    fn run(root: &Path, ids: Value) -> Value {
        census(root, ids.to_string().as_bytes()).unwrap()
    }

    #[test]
    fn actual_whole_root_not_supplied_or_config_subset() {
        let dir = fixture();
        fs::write(dir.path().join("tsconfig.json"), r#"{"files":["main.ts"]}"#).unwrap();
        fs::write(dir.path().join("outside.ts"), "export const outside = 2;").unwrap();
        let result = run(dir.path(), json!([]));
        assert_eq!(result["schema"], "prism.native-membership/1");
        assert_eq!(result["authorizes_runtime_edge"], false);
        assert_eq!(result["type_database_present"], false);
        assert_eq!(result["files"].as_array().unwrap().len(), 2);
        assert_eq!(result["files"][0]["id"], "project/main.ts");
        assert_eq!(result["files"][1]["id"], "project/outside.ts");
        assert_eq!(result["files"][0]["language"], "TypeScript");
        assert_eq!(result["files"][0]["size"], 24);
        assert_eq!(result["files"][0]["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn classifications_include_dependencies_without_loading_them() {
        let dir = fixture();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(
            dir.path().join("node_modules/dep.d.ts"),
            "declare const dep: number;",
        )
        .unwrap();
        let result = run(
            dir.path(),
            json!([
                "project/z.mts",
                "project/a.json",
                "project/node_modules/dep.d.ts",
                "compiler/lib.d.ts"
            ]),
        );
        assert_eq!(result["files"].as_array().unwrap().len(), 1);
        assert_eq!(
            result["supported"],
            json!([
                {"id":"compiler/lib.d.ts", "language":"TypeScript"},
                {"id":"project/a.json", "language":null},
                {"id":"project/node_modules/dep.d.ts", "language":"TypeScript"},
                {"id":"project/z.mts", "language":null}
            ])
        );
        assert!(result["skipped"]
            .as_array()
            .unwrap()
            .contains(&json!({"id":"project/node_modules/", "reason":"Ignored"})));
    }

    #[test]
    fn retains_nonfatal_parse_error_counts() {
        let dir = fixture();
        let source = format!(
            "{}\nconst broken = ;",
            (0..50)
                .map(|i| format!("const a{i} = {i};"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        fs::write(dir.path().join("errors.ts"), source).unwrap();
        let result = run(dir.path(), json!([]));
        let row = result["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "project/errors.ts")
            .unwrap();
        assert!(row["parse_errors"].as_u64().unwrap() > 0);
    }

    #[test]
    fn retains_skips_and_deterministic_order() {
        let dir = fixture();
        fs::write(dir.path().join("bad.ts"), [0xff]).unwrap();
        fs::write(dir.path().join("data.json"), "{}").unwrap();
        fs::create_dir(dir.path().join(".hidden")).unwrap();
        let result = run(dir.path(), json!([]));
        assert_eq!(
            result["skipped"],
            json!([
                {"id":"project/.hidden/", "reason":"Hidden"},
                {"id":"project/bad.ts", "reason":"NotUtf8"},
                {"id":"project/data.json", "reason":"Unsupported"}
            ])
        );
        assert_eq!(result, run(dir.path(), json!([])));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_is_reported_not_followed() {
        let dir = fixture();
        std::os::unix::fs::symlink("main.ts", dir.path().join("link.ts")).unwrap();
        let result = run(dir.path(), json!([]));
        assert_eq!(result["files"].as_array().unwrap().len(), 1);
        assert_eq!(
            result["skipped"],
            json!([{"id":"project/link.ts", "reason":"Symlink"}])
        );
    }

    #[test]
    fn rejects_noncanonical_duplicate_or_unsafe_ids() {
        let dir = fixture();
        for id in [
            "",
            "main.ts",
            "project/",
            "compiler/",
            "project/../a.ts",
            "project/./a.ts",
            "project//a.ts",
            "project/a/",
            "project/a\\b.ts",
            "project/a:b.ts",
            "project/a\n.ts",
            "project/a\0.ts",
        ] {
            assert!(
                census(dir.path(), json!([id]).to_string().as_bytes()).is_err(),
                "{id:?}"
            );
        }
        assert!(census(
            dir.path(),
            json!([format!("project/{}.ts", "a".repeat(4096))])
                .to_string()
                .as_bytes()
        )
        .is_err());
        assert!(census(dir.path(), br#"["project/a.ts","project/a.ts"]"#.as_slice()).is_err());
    }

    #[test]
    fn rejects_malformed_input_and_limits() {
        let dir = fixture();
        for bytes in [b"{}".as_slice(), b"null", b"[1]", b"[", b"[] trailing"] {
            assert!(census(dir.path(), bytes).is_err());
        }
        assert!(census(dir.path(), vec![b' '; MAX_INPUT_BYTES + 1].as_slice()).is_err());
        let ids: Vec<_> = (0..=MAX_IDS).map(|i| format!("project/{i}.ts")).collect();
        assert!(census(dir.path(), serde_json::to_vec(&ids).unwrap().as_slice()).is_err());
    }

    #[test]
    fn rejects_relative_missing_file_and_filesystem_roots() {
        let dir = fixture();
        for root in [
            Path::new("."),
            Path::new("/"),
            dir.path().join("absent").as_path(),
            dir.path().join("main.ts").as_path(),
        ] {
            assert!(census(root, b"[]".as_slice()).is_err());
        }
    }

    #[test]
    fn directory_and_root_skips_keep_identity() {
        for root in [".", "/", ""] {
            assert_eq!(project_id(root, true).unwrap(), "project/");
        }
        assert_eq!(
            project_id("node_modules/", true).unwrap(),
            "project/node_modules/"
        );
        assert!(project_id("../outside", true).is_err());
        assert!(project_id(".", false).is_err());
    }

    fn unicode_ledger(ledger: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        for name in ["\u{e000}.ts", "\u{10000}.ts"] {
            let bytes: &[u8] = if ledger == "skipped" {
                &[0xff]
            } else {
                b"export const x = 1;"
            };
            fs::write(dir.path().join(name), bytes).unwrap();
        }
        run(
            dir.path(),
            json!(["project/\u{e000}.ts", "project/\u{10000}.ts"]),
        )[ledger]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn files_match_javascript_utf16_order() {
        assert_eq!(
            unicode_ledger("files"),
            ["project/\u{10000}.ts", "project/\u{e000}.ts"]
        );
    }

    #[test]
    fn supported_match_javascript_utf16_order() {
        assert_eq!(
            unicode_ledger("supported"),
            ["project/\u{10000}.ts", "project/\u{e000}.ts"]
        );
    }

    #[test]
    fn skipped_match_javascript_utf16_order() {
        assert_eq!(
            unicode_ledger("skipped"),
            ["project/\u{10000}.ts", "project/\u{e000}.ts"]
        );
    }
}
