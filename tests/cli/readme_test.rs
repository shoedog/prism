//! README truth-pass gate (spec §2.5.8 / §7.5).
//!
//! (a) Every fenced-code-block line in README.md that starts with `prism ` and contains no
//!     `<` placeholder is split shell-style and parsed (never executed) with
//!     `prism::cli::Cli::try_parse_from` — the whole reason the clap derive tree moved from
//!     `src/main.rs` into `src/cli.rs` (a pure move; `scripts/phase0-byte-control.sh` plus a
//!     direct `--help`/`--version` `cmp` prove no behaviour changed). This means every README
//!     example must use flags that actually exist on the real CLI.
//! (a2) Neither README.md nor the `prism-code-slicing` skill's SKILL.md may regain a `slicing
//!     ...` example line — the installed binary is `prism` (Task 5 review, Important 2). This
//!     is the negative counterpart of (a): (a) proves every *kept* example parses; this proves
//!     the stale-binary-name examples this task fixed don't quietly come back (a plain parse
//!     check can't catch that on its own, since `slicing` simply wouldn't match the `prism `
//!     prefix filter and would be silently skipped rather than flagged).
//! (b) The README's documented `--format` value list — BOTH the Options-reference table row
//!     and the "## Output formats" table's `Flag value` column — equals the CLI's
//!     `value_parser` allow-list, read programmatically from `Cli::command()` rather than
//!     duplicated by hand, so none of the three can silently drift from each other.
//! (c) `prism --help`'s `--format` entry's own `[possible values: ...]` line mentions every
//!     format value (matched against that specific line, not the whole help text — several
//!     other flags' doc comments also contain the words "review"/"json"/etc., so a bare
//!     `help.contains(value)` would pass even if `--format` itself listed the wrong values).

use clap::{CommandFactory, Parser};
use prism::cli::Cli;
use std::fs;
use std::path::Path;

fn readme_text() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("README.md must be readable")
}

fn skill_text() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/prism-code-slicing/SKILL.md"),
    )
    .expect("skills/prism-code-slicing/SKILL.md must be readable")
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

/// Fail if any fenced line in `text` starts with the stale `slicing ` binary invocation — the
/// installed executable is `prism` (`Cargo.toml`'s `[[bin]] name = "prism"`; no `slicing`
/// binary exists). This is deliberately independent of `prism_invocation_lines`'s `prism `
/// prefix filter: a regressed `slicing ...` line would simply fail to match that filter and be
/// silently skipped, so the positive parse check alone can never catch this regression class.
fn assert_no_stale_slicing_invocations(source_label: &str, text: &str) {
    for line in fenced_lines(text) {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with("slicing "),
            "{source_label} regained a `slicing …` example; the binary is `prism` (line: {trimmed:?})"
        );
    }
}

#[test]
fn every_readme_prism_invocation_parses() {
    let readme = readme_text();

    assert_no_stale_slicing_invocations("README.md", &readme);
    assert_no_stale_slicing_invocations("skills/prism-code-slicing/SKILL.md", &skill_text());

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
fn readme_options_table_format_values(readme: &str) -> std::collections::BTreeSet<String> {
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

/// Extract the "## Output formats" section's table `Flag value` column (2nd cell of each data
/// row: `| Text (default) | \`text\` | ... |`). Scoped to that section only — stops at the next
/// heading of any level (including the "### SARIF ..." subheading) — so it can't accidentally
/// pick up an unrelated table elsewhere in the file.
fn readme_output_formats_table_values(readme: &str) -> std::collections::BTreeSet<String> {
    let mut values = std::collections::BTreeSet::new();
    let mut in_section = false;
    for line in readme.lines() {
        if line.trim() == "## Output formats" {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if line.trim_start().starts_with('#') {
            break;
        }
        let cells: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
        if cells.len() < 4 {
            continue;
        }
        let value_cell = cells[2];
        if value_cell.len() > 2 && value_cell.starts_with('`') && value_cell.ends_with('`') {
            values.insert(value_cell.trim_matches('`').to_string());
        }
    }
    assert!(
        !values.is_empty(),
        "found no `Flag value` entries under the \"## Output formats\" table — has its shape changed?"
    );
    values
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
    let cli_values = cli_format_values();

    let options_table_values = readme_options_table_format_values(&readme);
    assert_eq!(
        options_table_values, cli_values,
        "README's Options-reference --format row must equal the CLI's value_parser allow-list (set comparison)"
    );

    let output_formats_table_values = readme_output_formats_table_values(&readme);
    assert_eq!(
        output_formats_table_values, cli_values,
        "README's \"## Output formats\" table (`Flag value` column) must equal the CLI's value_parser allow-list (set comparison)"
    );
}

/// Find the `[possible values: ...]` line belonging specifically to the `--format` entry in
/// `prism --help` (the first such line after the `-f, --format` entry header) — NOT a bare
/// substring search over the whole help text. Several other flags (`--review-min-severity`,
/// `--caller-depth`'s doc comment, etc.) also contain words like "review"/"json" in their own
/// descriptions, so `help.contains(value)` alone could pass even if `--format` itself listed
/// the wrong allow-list.
fn help_format_possible_values_line(help: &str) -> String {
    let lines: Vec<&str> = help.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("-f, --format"))
        .expect("`prism --help` must show the `-f, --format` entry");
    lines[start..]
        .iter()
        .find(|l| l.trim_start().starts_with("[possible values:"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| {
            panic!("`prism --help`'s --format entry must have its own [possible values: ...] line")
        })
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
    let possible_values_line = help_format_possible_values_line(&help);
    for value in cli_format_values() {
        assert!(
            possible_values_line.contains(&value),
            "`prism --help`'s --format [possible values: ...] line must mention {value:?}; line was {possible_values_line:?}"
        );
    }
}
