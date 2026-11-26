use std::{fs, path::Path};

#[test]
fn admitted_status_is_constructed_only_at_the_dependency_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let choke_point = root.join("revision/admission.rs");
    let mut pending = vec![root.clone()];
    let mut constructors = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            let gate = (path == choke_point).then(|| {
                let start = source.find("pub(crate) fn from_reasons").unwrap();
                let end = start + source[start..].find("\n    }").unwrap();
                start..end
            });
            for (offset, _) in source.match_indices("\"admitted\"") {
                let prefix = source[..offset]
                    .trim_end_matches('#')
                    .strip_suffix('r')
                    .unwrap_or(&source[..offset])
                    .trim_end();
                if prefix.ends_with("==") || prefix.ends_with("!=") {
                    continue;
                }
                let line = source[..offset].lines().count();
                assert!(
                    gate.as_ref().is_some_and(|gate| gate.contains(&offset))
                        && prefix.ends_with("status: if admitted {"),
                    "{}:{line} constructs an admitted status outside the dependency gate",
                    path.strip_prefix(&root).unwrap().display()
                );
                constructors.push(line);
            }
        }
    }
    assert_eq!(constructors.len(), 1, "{constructors:?}");
}
