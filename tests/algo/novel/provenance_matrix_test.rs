use crate::common::{
    BTreeMap, BTreeSet, CpgContext, DiffInfo, DiffInput, Language, ModifyType, ParsedFile,
};
use anyhow::{bail, ensure, Context, Result};
use prism_review::algorithms::provenance_slice;
use serde::Deserialize;
use std::path::Path;

const ENABLED_GROUPS: &[&str] = &["control"];
const ROWS: &str = include_str!("../../fixtures/provenance/rows.jsonl");

#[derive(Deserialize)]
struct Row {
    id: String,
    group: String,
    files: Vec<String>,
    diff: BTreeMap<String, BTreeSet<usize>>,
    want: Vec<Expectation>,
    exact_lines: BTreeMap<String, BTreeSet<usize>>,
    truth: Vec<Expectation>,
    accepted_loss: Option<String>,
    #[serde(default)]
    tentative: bool,
}

#[derive(Deserialize)]
struct Expectation {
    file: String,
    line: usize,
    var: String,
    origin: String,
}

type Site<'a> = (&'a str, usize, &'a str);
type Findings<'a> = BTreeMap<Site<'a>, Vec<&'a str>>;

impl Row {
    /// Invariants of the row itself, checked for every row whether or not its group is enabled.
    fn validate(&self) -> Result<()> {
        let sites = |entries: &[Expectation]| {
            entries
                .iter()
                .map(|e| (e.file.clone(), e.line, e.var.clone()))
                .collect::<BTreeSet<_>>()
        };
        ensure!(
            sites(&self.want) == sites(&self.truth),
            "want and truth must list the same sites"
        );
        let same = |a: &[Expectation], b: &[Expectation]| {
            let key = |e: &Expectation| (e.file.clone(), e.line, e.var.clone(), e.origin.clone());
            a.iter().map(key).collect::<BTreeSet<_>>() == b.iter().map(key).collect::<BTreeSet<_>>()
        };
        ensure!(
            same(&self.want, &self.truth) || self.tentative || self.accepted_loss.is_some(),
            "want differs from truth: the row must be tentative or carry accepted_loss"
        );
        let primary = self.files.first().context("no fixture files")?;
        ensure!(
            self.diff.contains_key(primary),
            "first fixture file must be in diff"
        );
        let lang = Path::new(primary)
            .extension()
            .and_then(|ext| ext.to_str())
            .context("first fixture file has no language extension")?;
        let row_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/provenance/cases")
            .join(lang)
            .join(&self.id);
        for (file, lines) in &self.diff {
            ensure!(self.files.contains(file), "undeclared diff file {file}");
            let source = std::fs::read_to_string(row_dir.join(file))
                .with_context(|| format!("missing fixture {file}"))?;
            let line_count = source.lines().count();
            ensure!(
                !lines.is_empty() && lines.iter().all(|line| (1..=line_count).contains(line)),
                "invalid diff lines for {file}"
            );
        }
        for entry in self.want.iter().chain(&self.truth) {
            ensure!(
                self.diff
                    .get(&entry.file)
                    .is_some_and(|lines| lines.contains(&entry.line)),
                "expectation outside diff for {}",
                entry.var
            );
        }
        Ok(())
    }

    fn check(&self) -> Result<()> {
        let primary = self.files.first().context("no fixture files")?;
        ensure!(
            self.diff.contains_key(primary),
            "first fixture file must be in diff"
        );
        let lang = Path::new(primary)
            .extension()
            .and_then(|ext| ext.to_str())
            .context("first fixture file has no language extension")?;
        let row_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/provenance/cases")
            .join(lang)
            .join(&self.id);
        for file in &self.files {
            ensure!(row_dir.join(file).is_file(), "missing fixture {file}");
        }

        let mut files = BTreeMap::new();
        let mut diff = DiffInput { files: Vec::new() };
        for (file, lines) in &self.diff {
            ensure!(self.files.contains(file), "undeclared diff file {file}");
            let language = Language::from_path(file)
                .with_context(|| format!("unsupported language: {file}"))?;
            let source = std::fs::read_to_string(row_dir.join(file))?;
            let line_count = source.lines().count();
            ensure!(
                !lines.is_empty() && lines.iter().all(|line| (1..=line_count).contains(line)),
                "invalid diff lines for {file}"
            );
            files.insert(file.clone(), ParsedFile::parse(file, &source, language)?);
            diff.files.push(DiffInfo {
                file_path: file.clone(),
                modify_type: ModifyType::Modified,
                diff_lines: lines.clone(),
            });
        }
        for (file, lines) in &self.exact_lines {
            ensure!(
                self.diff
                    .get(file)
                    .is_some_and(|changed| lines.is_subset(changed)),
                "exact lines outside diff for {file}"
            );
        }

        let ctx = CpgContext::build(&files, None);
        let output = provenance_slice::slice(&ctx, &diff)?;
        let mut actual = Findings::new();
        for finding in &output.findings {
            let (var, origin) = finding
                .description
                .strip_prefix("variable '")
                .and_then(|s| s.split_once("' has "))
                .and_then(|(var, tail)| {
                    tail.split_once(" origin:").map(|(origin, _)| (var, origin))
                })
                .with_context(|| {
                    format!("invalid provenance description: {:?}", finding.description)
                })?;
            let site = (finding.file.as_str(), finding.line, var);
            actual.entry(site).or_default().push(origin);
        }

        let errors = self.differences(&self.want, &actual)?;
        if let Some(reason) = &self.accepted_loss {
            if self.differences(&self.truth, &actual)?.is_empty() {
                println!(
                    "provenance-matrix IMPROVEMENT [{}] {}: {reason}",
                    self.group, self.id
                );
                // Stage D adjudicates improvements; set PROVENANCE_MATRIX_STRICT=1 to fail on one.
                ensure!(
                    std::env::var_os("PROVENANCE_MATRIX_STRICT").is_none(),
                    "improvement on an accepted-loss row (strict mode): {reason}"
                );
            }
        }
        if !errors.is_empty() {
            bail!("{}", errors.join("; "));
        }
        Ok(())
    }

    fn differences(&self, expected: &[Expectation], actual: &Findings<'_>) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        let mut mentioned = BTreeSet::new();
        for entry in expected {
            let site = (entry.file.as_str(), entry.line, entry.var.as_str());
            ensure!(
                self.diff
                    .get(&entry.file)
                    .is_some_and(|lines| lines.contains(&entry.line)),
                "expectation outside diff at {site:?}"
            );
            ensure!(mentioned.insert(site), "duplicate expectation at {site:?}");
            let found = actual.get(&site).map(Vec::as_slice).unwrap_or_default();
            let origins = [entry.origin.as_str()];
            let wanted = if entry.origin == "none" {
                &[][..]
            } else {
                &origins[..]
            };
            if found != wanted {
                errors.push(format!(
                    "{site:?}: expected {}, got {found:?}",
                    entry.origin
                ));
            }
        }
        for (&site @ (file, line, _), origins) in actual {
            if self
                .exact_lines
                .get(file)
                .is_some_and(|lines| lines.contains(&line))
                && !mentioned.contains(&site)
            {
                errors.push(format!("{site:?}: unexpected {origins:?}"));
            }
        }
        Ok(errors)
    }
}

#[test]
fn test_provenance_matrix_rows() {
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    let mut enabled = 0;
    for (index, line) in ROWS.lines().enumerate() {
        let row: Row = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("rows.jsonl:{}: {error}", index + 1));
        assert!(ids.insert(row.id.clone()), "duplicate row id: {}", row.id);
        assert!(
            ["control", "structural", "py", "js", "go"].contains(&row.group.as_str()),
            "unknown group on {}: {}",
            row.id,
            row.group
        );
        if let Err(error) = row.validate() {
            failures.push(format!(
                "provenance-matrix INVALID [{}] {}: {error:#}",
                row.group, row.id
            ));
            continue;
        }
        if !ENABLED_GROUPS.contains(&row.group.as_str()) {
            continue;
        }
        enabled += 1;
        if let Err(error) = row.check() {
            failures.push(format!(
                "provenance-matrix FAIL [{}] {}: {error:#}",
                row.group, row.id
            ));
        }
    }
    assert!(enabled > 0, "no enabled provenance rows");
    println!(
        "provenance-matrix: {enabled} enabled, {} passed, {} failed",
        enabled - failures.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
