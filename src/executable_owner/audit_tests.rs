use super::*;
use crate::{languages::Language, resolution::ResolutionConfidence};
use std::fs;

fn fixture(
    extension: &str,
    value: u8,
) -> (tempfile::TempDir, BTreeMap<String, ParsedFile>, String) {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut files = BTreeMap::new();
    for (name, value_source) in corpus["files"].as_object().unwrap() {
        let name = name.replace("app.ts", &format!("app.{extension}"));
        let source = value_source
            .as_str()
            .unwrap()
            .replace("return 1", &format!("return {value}"));
        fs::create_dir_all(root.path().join(&name).parent().unwrap()).unwrap();
        fs::write(root.path().join(&name), &source).unwrap();
        let lang = if name.ends_with(".tsx") {
            Language::Tsx
        } else {
            Language::TypeScript
        };
        files.insert(
            name.clone(),
            ParsedFile::parse(&name, &source, lang).unwrap(),
        );
    }
    fs::write(root.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::write(
        root.path().join("tsconfig.json"),
        serde_json::json!({"compilerOptions":corpus["compiler_options"],"include":["src"]})
            .to_string(),
    )
    .unwrap();
    (root, files, format!("src/app.{extension}"))
}

#[test]
fn detached_constructor_proves_genuine_ts_tsx_and_rejects_other_epochs() {
    let compiler = std::env::var("PRISM_TYPESCRIPT")
        .expect("audit requires explicit pinned compiler; no silent skip");
    for extension in ["ts", "tsx"] {
        let (root_a, files_a, app) = fixture(extension, 1);
        let graph = crate::call_graph::CallGraph::build(&files_a);
        let call = graph
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.file == app && s.callee_name == "m")
            .unwrap();
        assert!(
            !graph
                .resolve_call_site(call)
                .iter()
                .any(|e| e.confidence == ResolutionConfidence::Exact),
            "detached proof must not wire the public resolver"
        );
        let a = AuthenticatedProgramEpoch::acquire(
            root_a.path(),
            "tsconfig.json",
            Path::new(&compiler),
            files_a,
        )
        .expect("genuine A must acquire");
        let proof = a
            .prove(&app, 164, 174)
            .expect("direct contextual owner proof");
        assert_eq!(
            a.target_for(&proof, &app, 164, 174).unwrap().file,
            "src/client.ts"
        );
        assert!(a.target_for(&proof, &app, 165, 174).is_none());
        assert!(a.prove(&app, 165, 174).is_none());
        let (root_b, files_b, _) = fixture(extension, 2);
        let b = AuthenticatedProgramEpoch::acquire(
            root_b.path(),
            "tsconfig.json",
            Path::new(&compiler),
            files_b,
        )
        .expect("genuine B must acquire");
        let other = b.prove(&app, 164, 174).expect("genuine B proof");
        assert!(a.target_for(&other, &app, 164, 174).is_none());
        assert!(b.target_for(&proof, &app, 164, 174).is_none());
        // Same bytes/root in a NEW session still do not supply the same owning Arc.
        let (_, files_same, _) = fixture(extension, 1);
        let same = AuthenticatedProgramEpoch::acquire(
            root_a.path(),
            "tsconfig.json",
            Path::new(&compiler),
            files_same,
        )
        .unwrap();
        let same_proof = same.prove(&app, 164, 174).unwrap();
        assert!(a.target_for(&same_proof, &app, 164, 174).is_none());
    }
}

fn put(root: &Path, files: &mut BTreeMap<String, ParsedFile>, file: &str, source: &str) {
    fs::create_dir_all(root.join(file).parent().unwrap()).unwrap();
    fs::write(root.join(file), source).unwrap();
    files.insert(
        file.into(),
        ParsedFile::parse(file, source, Language::from_path(file).unwrap()).unwrap(),
    );
}
fn compiler() -> std::path::PathBuf {
    std::env::var("PRISM_TYPESCRIPT")
        .expect("audit requires explicit pinned compiler")
        .into()
}

#[test]
fn real_acquisition_rejects_extra_missing_changed_and_excluded_prism_inputs() {
    for id in [
        "missing",
        "changed",
        "extra",
        "excluded_effect",
        "excluded_effect_omitted",
    ] {
        let (root, mut files, _) = fixture("ts", 1);
        match id {
            "missing" => {
                files.remove("src/client.ts");
            }
            "changed" => {
                files.insert(
                    "src/client.ts".into(),
                    ParsedFile::parse(
                        "src/client.ts",
                        "export class Client {m(){return 2;}}",
                        Language::TypeScript,
                    )
                    .unwrap(),
                );
            }
            "extra" => {
                files.insert(
                    "extra.ts".into(),
                    ParsedFile::parse("extra.ts", "export const x=1;", Language::TypeScript)
                        .unwrap(),
                );
            }
            _ => put(
                root.path(),
                &mut files,
                "outside/effect.ts",
                "import {Client} from '../src/client'; Client.prototype.m = () => 2;",
            ),
        }
        if id == "excluded_effect_omitted" {
            files.remove("outside/effect.ts");
        }
        assert!(
            AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), files)
                .is_err(),
            "{id}"
        );
    }
}

#[test]
fn every_design_refusal_reaches_the_actual_constructor_in_ts_and_tsx() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    for extension in ["ts", "tsx"] {
        for case in corpus["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let (root, mut files, app) = fixture(extension, 1);
            if let Some(overrides) = case["files"].as_object() {
                for (file, source) in overrides {
                    let file = file.replace("src/app.ts", &app);
                    put(root.path(), &mut files, &file, source.as_str().unwrap());
                }
            }
            if let Some(remove) = case["remove"].as_array() {
                for file in remove {
                    let file = file.as_str().unwrap();
                    files.remove(file);
                    fs::remove_file(root.path().join(file)).unwrap();
                }
            }
            if let Some(replace) = case["replace_app"].as_array() {
                let source = files[&app]
                    .source
                    .replace(replace[0].as_str().unwrap(), replace[1].as_str().unwrap());
                put(root.path(), &mut files, &app, &source);
            }
            let mut options = corpus["compiler_options"].as_object().unwrap().clone();
            if let Some(extra) = case["options"].as_object() {
                options.extend(extra.clone());
            }
            if let Some(remove) = case["delete_options"].as_array() {
                for key in remove {
                    options.remove(key.as_str().unwrap());
                }
            }
            fs::write(
                root.path().join("tsconfig.json"),
                serde_json::json!({"compilerOptions":options,"include":["src"]}).to_string(),
            )
            .unwrap();
            let got = AuthenticatedProgramEpoch::acquire(
                root.path(),
                "tsconfig.json",
                &compiler(),
                files,
            );
            let has_proof = got.as_ref().is_ok_and(|epoch| !epoch.0.members.is_empty());
            assert_eq!(
                has_proof,
                ["candidate", "different_genuine_input"].contains(&id),
                "{id}/{extension}: {:?}",
                got.err()
            );
        }
    }
}

#[test]
fn all_anchor_fields_from_two_genuine_inputs_cannot_be_spliced() {
    let mut epochs = Vec::new();
    for value in [1, 2] {
        let (root, mut files, _) = fixture("ts", value);
        for file in ["src/app.ts", "src/contract.ts"] {
            let source = format!("{}// epoch {value}\n", files[file].source);
            put(root.path(), &mut files, file, &source);
        }
        let epoch =
            AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), files);
        epochs.push((root, epoch));
    }
    let a = epochs[0].1.as_ref().unwrap();
    let b = epochs[1].1.as_ref().unwrap();
    let ca = &a.0._evidence.candidates[0];
    let cb = &b.0._evidence.candidates[0];
    let graph = CallGraph::build(&a.0._files);
    for (name, _) in ANCHORS {
        assert_ne!(
            ca.anchors[name].sha256, cb.anchors[name].sha256,
            "genuine source hashes must differ for {name}"
        );
        let mut mixed = ca.clone();
        mixed.anchors.insert(name.into(), cb.anchors[name].clone());
        assert!(map_member(&mixed, &a.0._files, &graph).is_err(), "{name}");
    }
    for changed in [
        "class_name",
        "member_name",
        "class_module",
        "callable_module",
    ] {
        let mut mixed = ca.clone();
        match changed {
            "class_name" => mixed.class_name = "Other".into(),
            "member_name" => mixed.member_name = "other".into(),
            "class_module" => mixed.class_module = "./contract".into(),
            _ => mixed.callable_module = "./client".into(),
        }
        assert!(
            map_member(&mixed, &a.0._files, &graph).is_err(),
            "{changed}"
        );
    }
}

#[test]
fn parser_mapping_rejects_line_id_collisions_and_accepts_unicode_original_bytes() {
    let (root, mut files, app) = fixture("tsx", 1);
    let source = format!("// 🦊\r\n{}", files[&app].source);
    let start = source.find("client.m()").unwrap();
    put(root.path(), &mut files, &app, &source);
    let epoch =
        AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), files)
            .unwrap();
    let proof = epoch.prove(&app, start, start + 10).unwrap();
    assert!(epoch.target_for(&proof, &app, start, start + 10).is_some());
    let (root, mut files, _) = fixture("ts", 1);
    put(
        root.path(),
        &mut files,
        "src/client.ts",
        "export class Client { m(): number { return 1; } } function m() {}\n",
    );
    assert!(
        AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), files)
            .is_err()
    );
}

#[test]
fn same_root_positive_changed_and_unproven_epochs_do_not_reuse_authority() {
    let (root, files, app) = fixture("ts", 1);
    let a = AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), files)
        .unwrap();
    let proof_a = a.prove(&app, 164, 174).unwrap();
    fs::write(
        root.path().join("src/client.ts"),
        "export class Client { m(): number { return 2; } }\n",
    )
    .unwrap();
    let loaded = crate::repo_loader::load_repo(root.path()).unwrap();
    let b =
        AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), loaded.files)
            .unwrap();
    let proof_b = b.prove(&app, 164, 174).unwrap();
    assert!(b.target_for(&proof_a, &app, 164, 174).is_none());
    assert!(a.target_for(&proof_b, &app, 164, 174).is_none());
    fs::write(
        root.path().join("src/client.ts"),
        "export class Client { m = () => 3; }\n",
    )
    .unwrap();
    let loaded = crate::repo_loader::load_repo(root.path()).unwrap();
    let unproven =
        AuthenticatedProgramEpoch::acquire(root.path(), "tsconfig.json", &compiler(), loaded.files)
            .unwrap();
    assert!(unproven.prove(&app, 164, 174).is_none());
    assert!(unproven.target_for(&proof_b, &app, 164, 174).is_none());
    let config_path = root.path().join("tsconfig.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["compilerOptions"]["types"] = serde_json::json!(["missing-owner-test-type"]);
    fs::write(config_path, config.to_string()).unwrap();
    let loaded = crate::repo_loader::load_repo(root.path()).unwrap();
    assert!(AuthenticatedProgramEpoch::acquire(
        root.path(),
        "tsconfig.json",
        &compiler(),
        loaded.files
    )
    .is_err());
    // These remain immutable HISTORICAL epochs, not an active-analysis manager.
    assert!(a.target_for(&proof_a, &app, 164, 174).is_some());
    assert!(b.target_for(&proof_b, &app, 164, 174).is_some());
}
