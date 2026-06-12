//! Whole-repo function inventory from the S1 FunctionTable (Tier-A spec §2.3).
//! Deliberately NOT the nav CPG index: CpgNode::Function carries no kind.
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FunctionRecord {
    pub file: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String,
}

pub fn functions_inventory(repo: &Path) -> anyhow::Result<Vec<FunctionRecord>> {
    let loaded = crate::repo_loader::load_repo(repo)?;
    let mut out = Vec::new();
    for (file, pf) in &loaded.files {
        let lang = pf.language.tree_sitter_language();
        let recs: Vec<FunctionRecord> = pf
            .functions()
            .iter()
            .map(|f| FunctionRecord {
                file: file.clone(),
                name: f.name.clone(),
                start_line: f.start_line,
                end_line: f.end_line,
                kind: lang
                    .node_kind_for_id(f.kind_id)
                    .unwrap_or("unknown")
                    .to_string(),
            })
            .collect();
        // §2.3 dedup: tree-sitter-python captures BOTH (decorated_definition)
        // wrappers and the inner function_definition. Wrapper kinds never survive
        // over their inner definition.
        let mut keep = vec![true; recs.len()];
        for i in 0..recs.len() {
            for j in 0..recs.len() {
                if i == j {
                    continue;
                }
                let (outer, inner) = (&recs[i], &recs[j]);
                let contains = outer.start_line <= inner.start_line
                    && inner.end_line <= outer.end_line
                    && (outer.start_line, outer.end_line) != (inner.start_line, inner.end_line);
                let wrapper = outer.kind == "decorated_definition";
                if contains && wrapper {
                    keep[i] = false;
                }
            }
        }
        let mut it = keep.iter();
        let mut recs = recs;
        recs.retain(|_| *it.next().unwrap());
        out.extend(recs);
    }
    out.sort_by(|a, b| {
        (&a.file, a.start_line, a.end_line).cmp(&(&b.file, b.start_line, b.end_line))
    });
    out.dedup();
    Ok(out)
}
