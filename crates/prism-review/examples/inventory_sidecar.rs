//! Print projected Python inventory occurrence evidence as JSONL.
use anyhow::{ensure, Context, Result};
use prism_review::api::{
    collect, inventory_document, load_python_files, InventoryOptions, TargetsMeta,
};
use prism_review::targets::inventory::evidence::sidecar;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let usage = "usage: inventory_sidecar <repo> [import-root]";
    let mut args = std::env::args_os().skip(1);
    let repo = PathBuf::from(args.next().context(usage)?).canonicalize()?;
    let import_root = args.next().map(|root| repo.join(root));
    ensure!(args.next().is_none(), "{usage}");
    let lookup_root = import_root.as_deref().unwrap_or(&repo);
    ensure!(
        lookup_root.is_dir(),
        "import root is not a directory: {}",
        lookup_root.display()
    );

    let (files, skipped) = load_python_files(&repo, None)?;
    let mut output = collect(&files, lookup_root, &InventoryOptions::default());
    output.skipped.extend(skipped);
    let mut meta = TargetsMeta::default();
    meta.repo_root = repo;
    let document = inventory_document(&meta, &output);
    let mut stdout = io::stdout().lock();
    for row in sidecar(&document.targets, &output.occurrences) {
        serde_json::to_writer(&mut stdout, &row)?;
        writeln!(stdout)?;
    }
    Ok(())
}
