//! README truth-pass gate (spec §2.5.8 / §7.5).
//!
//! (a) Every fenced-code-block line in README.md that starts with `prism ` and contains no
//!     `<` placeholder is split shell-style and parsed (never executed) with
//!     `prism::cli::Cli::try_parse_from` — the whole reason the clap derive tree moved from
//!     `src/main.rs` into `src/cli.rs` (a pure move; `scripts/phase0-byte-control.sh` plus a
//!     direct `--help`/`--version` `cmp` prove no behaviour changed). This means every README
//!     example must use flags that actually exist on the real CLI.
//! (b) The README's documented `--format` value list (the Options-reference table row) equals
//!     the CLI's `value_parser` allow-list, read programmatically from `Cli::command()` rather
//!     than duplicated by hand — so the two can never silently drift again.
//! (c) `prism --help` mentions every format value.

use clap::{CommandFactory, Parser};
use prism::cli::Cli;
use std::fs;
use std::path::Path;

fn readme_text() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("README.md must be readable")
}

/// Lines found inside ``` fences (fence markers themselves excluded).
fn fenced_lines(readme: &str) -> Vec<&str> {
    let mut in_fence = false;
    let mut lines = Vec::new();
    for line in readme.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            lines.push(line);
        }
    }
    lines
}

/// Shell metacharacters that end the "prism ..." portion of a piped/redirected README
/// example (e.g. `prism --repo . --diff x.patch | pbcopy`, `... --format sarif > out.sarif`).
/// The gate only needs prism's OWN argv to parse — clap has no notion of shell pipelines —
/// so the naive whitespace split truncates at the first such token rather than misreading it
/// as an extra positional argument. This is the same spirit as the design's `<placeholder>`
/// exclusion: both are "this isn't really CLI argv" cases the naive split can't represent.
const SHELL_METACHARS: &[&str] = &["|", ">", ">>", "&&", ";", "&"];

fn shell_split_for_clap(line: &str) -> Vec<String> {
    let mut argv = Vec::new();
    for tok in line.split_whitespace() {
        if SHELL_METACHARS.contains(&tok) {
            break;
        }
        argv.push(tok.to_string());
    }
    argv
}

/// README `prism ...` invocation lines eligible for the gate, plus a count of lines skipped
/// because they contain a quote character. Quoted values (e.g. `--barrier-symbols
/// "React.createElement,useEffect"`, a single shell word containing a comma) can't be
/// recovered by a naive whitespace split without a real shell tokenizer — splitting them
/// apart would produce extra positional tokens clap correctly rejects. Per spec §2.5.8 these
/// are skipped rather than mis-parsed; this function documents and counts that exception so
/// the skip path itself stays exercised (a test that silently skips everything is not a gate).
fn prism_invocation_lines(readme: &str) -> (Vec<String>, usize) {
    let mut kept = Vec::new();
    let mut skipped_quoted = 0usize;
    for line in fenced_lines(readme) {
        let trimmed = line.trim();
        if !trimmed.starts_with("prism ") || trimmed.contains('<') {
            continue;
        }
        if trimmed.contains('"') || trimmed.contains('\'') {
            skipped_quoted += 1;
            continue;
        }
        kept.push(trimmed.to_string());
    }
    (kept, skipped_quoted)
}

#[test]
fn every_readme_prism_invocation_parses() {
    let readme = readme_text();
    let (lines, skipped_quoted) = prism_invocation_lines(&readme);
    assert!(
        lines.len() >= 10,
        "expected many README `prism ...` invocations inside fenced code blocks to check; found {}",
        lines.len()
    );
    assert!(
        skipped_quoted > 0,
        "expected at least one quoted README example (e.g. --barrier-symbols \"a,b\") to \
         exercise the documented quote-skip path; found none — has the quote convention changed?"
    );

    let mut failures = Vec::new();
    for line in &lines {
        let argv = shell_split_for_clap(line);
        if let Err(e) = Cli::try_parse_from(&argv) {
            failures.push(format!("  {line:?}\n    -> {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "README examples that fail to parse against prism::cli::Cli (parse-only, never executed):\n{}",
        failures.join("\n")
    );
}

/// Extract the `--format` row's value list from the "### Universal flags" options-reference
/// table: `| \`--format\`, \`-f\` | \`text\` | \`text\`, \`json\`, ... |`.
fn readme_format_values(readme: &str) -> std::collections::BTreeSet<String> {
    let row = readme
        .lines()
        .find(|l| l.trim_start().starts_with("| `--format`"))
        .unwrap_or_else(|| panic!("README must have an options-table row starting `| `--format``"));
    let cells: Vec<&str> = row
        .split('|')
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .collect();
    let values_cell = *cells
        .last()
        .unwrap_or_else(|| panic!("--format row has no values cell: {row:?}"));
    values_cell
        .split(',')
        .map(|v| v.trim().trim_matches('`').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

fn cli_format_values() -> std::collections::BTreeSet<String> {
    let command = Cli::command();
    let format_arg = command
        .get_arguments()
        .find(|a| a.get_id() == "format")
        .expect("Cli must define a --format argument");
    format_arg
        .get_possible_values()
        .iter()
        .map(|v| v.get_name().to_string())
        .collect()
}

#[test]
fn readme_format_list_matches_cli() {
    let readme = readme_text();
    let readme_values = readme_format_values(&readme);
    let cli_values = cli_format_values();
    assert_eq!(
        readme_values, cli_values,
        "README's --format options-table row must equal the CLI's value_parser allow-list (set comparison)"
    );
}

#[test]
fn help_output_contains_every_format_value() {
    // The real compiled binary's `--help`, not a simulated render — this is the actual
    // execution surface a user (or the README) sees.
    let output = assert_cmd::Command::cargo_bin("prism")
        .expect("prism binary must be built")
        .arg("--help")
        .output()
        .expect("prism --help must run");
    let help = String::from_utf8_lossy(&output.stdout);
    for value in cli_format_values() {
        assert!(
            help.contains(&value),
            "`prism --help` must mention format value {value:?}; help was:\n{help}"
        );
    }
}
