use crate::navigation::onboarding::ProjectOverview;
use anyhow::{bail, Result};
use std::fmt::Write;

pub fn render(report: &ProjectOverview, format: &str) -> Result<String> {
    match format {
        "json" => Ok(format!("{}\n", serde_json::to_string_pretty(report)?)),
        "markdown" => Ok(render_markdown(report)),
        other => bail!("unsupported onboarding format {other:?}"),
    }
}

fn render_markdown(report: &ProjectOverview) -> String {
    let mut output = String::new();
    writeln!(output, "# Prism project overview").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Schema: `{}`", report.schema_version).unwrap();
    writeln!(output, "- Project: {}", inline_code(&report.project)).unwrap();
    writeln!(output).unwrap();

    writeln!(output, "## Inventory").unwrap();
    writeln!(output).unwrap();
    writeln!(
        output,
        "- Indexed files: {}",
        report.inventory.indexed_files
    )
    .unwrap();
    writeln!(
        output,
        "- Skipped files: {}",
        report.inventory.skipped_files
    )
    .unwrap();
    writeln!(output, "- Functions: {}", report.inventory.functions).unwrap();
    if report.inventory.languages.is_empty() {
        writeln!(output, "- Languages: none").unwrap();
    } else {
        writeln!(output, "- Languages:").unwrap();
        for (language, count) in &report.inventory.languages {
            writeln!(output, "  - {language}: {count}").unwrap();
        }
    }
    writeln!(output).unwrap();

    writeln!(output, "## Module architecture").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Nodes: {}", report.modules.nodes).unwrap();
    writeln!(output, "- Edges: {}", report.modules.edges).unwrap();
    writeln!(
        output,
        "- Isolated files: {}",
        report.modules.isolated_files
    )
    .unwrap();
    if report.modules.connected.is_empty() {
        writeln!(output, "- Connected modules: none").unwrap();
    } else {
        writeln!(output, "- Highest-connectivity modules:").unwrap();
        for module in &report.modules.connected {
            writeln!(
                output,
                "  - {}: {} dependencies, {} dependents",
                inline_code(&module.file),
                module.dependencies,
                module.dependents
            )
            .unwrap();
        }
    }
    writeln!(output).unwrap();

    writeln!(output, "## Call resolution").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Total sites: {}", report.calls.total_sites).unwrap();
    writeln!(output, "- Exact edges: {}", report.calls.exact_edges).unwrap();
    writeln!(output, "- NameOnly edges: {}", report.calls.name_only_edges).unwrap();
    writeln!(output, "- Demoted edges: {}", report.calls.demoted_edges).unwrap();
    writeln!(
        output,
        "- Dropped multi-owner: {}",
        report.calls.dropped_multi_owner
    )
    .unwrap();
    writeln!(
        output,
        "- Dropped external receiver: {}",
        report.calls.dropped_external_receiver
    )
    .unwrap();
    writeln!(
        output,
        "- Dropped external import: {}",
        report.calls.dropped_import_external
    )
    .unwrap();
    writeln!(
        output,
        "- Unresolved unknown name: {}",
        report.calls.unresolved_unknown_name
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "## Warnings").unwrap();
    writeln!(output).unwrap();
    if report.warnings.is_empty() {
        writeln!(output, "- None").unwrap();
    } else {
        for warning in &report.warnings {
            writeln!(output, "- {}", inline_code(warning)).unwrap();
        }
    }
    writeln!(output).unwrap();

    writeln!(output, "## Next commands").unwrap();
    writeln!(output).unwrap();
    for command in &report.next_commands {
        writeln!(output, "- {}", inline_code(command)).unwrap();
    }
    output
}

fn inline_code(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    format!("`{escaped}`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::onboarding::{
        CallOverview, InventoryOverview, ModuleOverview, ProjectOverview,
    };
    use std::collections::BTreeMap;

    #[test]
    fn markdown_escapes_untrusted_project_and_warning_lines() {
        let report = ProjectOverview {
            schema_version: "1.0".to_string(),
            project: "bad\n`name`".to_string(),
            inventory: InventoryOverview {
                indexed_files: 0,
                skipped_files: 0,
                functions: 0,
                languages: BTreeMap::new(),
            },
            modules: ModuleOverview {
                nodes: 0,
                edges: 0,
                isolated_files: 0,
                connected: vec![],
            },
            calls: CallOverview {
                total_sites: 0,
                exact_edges: 0,
                name_only_edges: 0,
                demoted_edges: 0,
                dropped_multi_owner: 0,
                dropped_external_receiver: 0,
                dropped_import_external: 0,
                unresolved_unknown_name: 0,
            },
            warnings: vec!["line\nbreak".to_string()],
            next_commands: vec![],
        };
        let rendered = render(&report, "markdown").unwrap();
        assert!(rendered.contains("`bad\\n\\`name\\``"));
        assert!(rendered.contains("`line\\nbreak`"));
        assert!(!rendered.contains("bad\n`name`"));
        assert!(render(&report, "text").is_err());
    }
}
