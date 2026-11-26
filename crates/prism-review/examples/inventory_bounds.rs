//! Print AST-only Python function bounds for a repository.
use anyhow::{Context, Result};
use prism_review::api::{function_bounds, load_python_files};
use std::io::{self, Write};
use std::path::Path;

fn main() -> Result<()> {
    let repo = std::env::args_os()
        .nth(1)
        .context("usage: inventory_bounds <repo>")?;
    let (files, _) = load_python_files(Path::new(&repo), None)?;
    let mut stdout = io::stdout().lock();
    for (file, parsed) in files {
        for bounds in function_bounds(&parsed) {
            serde_json::to_writer(
                &mut stdout,
                &serde_json::json!({
                    "file": file, "name": bounds.name,
                    "start_line": bounds.start_line, "end_line": bounds.end_line,
                }),
            )?;
            writeln!(stdout)?;
        }
    }
    Ok(())
}
