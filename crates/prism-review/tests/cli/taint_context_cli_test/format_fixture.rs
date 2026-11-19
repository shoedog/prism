//! REAL repository inputs shared by the binary and the independent public-API oracle.
use prism::finding_confidence::{MinConfidence, ResolutionMode};
use prism_review::api::{AlgorithmParams, ReviewOptions, TaintContextEdges, TaintContextMode};
use std::fs;
use std::path::PathBuf;
use std::process::Output;
use tempfile::TempDir;

const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
pub(super) const B: &str = "export function g(x) { eval(x); }\n";
pub(super) const DEGRADED: &str =
    "export function g(x) { eval(x); }\nfunction unrelated() { const broken = ; }\n";
const APP: &str = "import * as N from './missing/../x';\nimport { f } from './x';\nexport function weak(x) { N.f(x); }\nexport function strong(x) { f(x); }\n";
const X: &str = "export function f(x) { eval(x); }\n";

pub(super) struct Fixture {
    pub dir: TempDir,
    pub repo: PathBuf,
    pub diff: String,
    pub original: &'static str,
}

impl Fixture {
    fn new(original: &'static str, lines: &[usize], files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        for (name, text) in files {
            let path = repo.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        let diff = serde_json::json!({"files": [{
            "file_path": original, "modify_type": "Modified", "diff_lines": lines
        }]})
        .to_string();
        fs::write(dir.path().join("diff.json"), &diff).unwrap();
        Self {
            dir,
            repo: repo.canonicalize().unwrap(),
            diff,
            original,
        }
    }

    pub fn js(body: &str) -> Self {
        Self::new(
            "main.js",
            &[2, 3],
            &[("main.js", MAIN), ("a.js", A), ("b.js", body)],
        )
    }

    pub fn ts(lines: &[usize]) -> Self {
        Self::new(
            "app.ts",
            lines,
            &[("app.ts", APP), ("x.ts", X), ("other/x.ts", X)],
        )
    }

    pub fn invoke(&self, format: &str, algorithms: &str, flags: &[&str]) -> Output {
        let mut cmd = crate::prism_std();
        if format == "targets" {
            cmd.arg("targets");
        } else {
            cmd.args(["--format", format]);
        }
        cmd.arg("--repo")
            .arg(&self.repo)
            .arg("--diff")
            .arg(self.dir.path().join("diff.json"))
            .args(["--algorithm", algorithms, "--no-cache"])
            .args(flags)
            .env_remove("PRISM_PROFILE")
            .output()
            .unwrap()
    }
}

// Set the expected API policy independently of the parser being tested.
pub(super) struct Policy {
    pub params: AlgorithmParams,
    pub resolution: ResolutionMode,
    pub floor: MinConfidence,
}

impl Policy {
    pub fn imports() -> Self {
        let mut params = AlgorithmParams::default();
        params.taint_context.mode = TaintContextMode::Imports;
        Self {
            params,
            resolution: ResolutionMode::Scoped,
            floor: MinConfidence::NameOnly,
        }
    }

    pub fn off() -> Self {
        Self {
            params: AlgorithmParams::default(),
            ..Self::imports()
        }
    }

    pub fn bounded() -> Self {
        let mut p = Self::imports();
        p.params.taint_context.depth = 1;
        p
    }

    pub fn rooted(line: usize, edges: TaintContextEdges) -> Self {
        let mut p = Self::imports();
        p.params.taint_sources = vec![format!("app.ts:{line}")];
        p.params.taint_context.edges = edges;
        p
    }

    pub fn options(&self, f: &Fixture) -> ReviewOptions {
        let mut options = ReviewOptions::new(&f.repo);
        options.no_cache = true;
        options.resolution = self.resolution;
        options.min_confidence = self.floor;
        options
    }
}

pub(super) const ON: &[&str] = &["--taint-context", "imports", "--resolution", "scoped"];
pub(super) const BOUND: &[&str] = &[
    "--taint-context",
    "imports",
    "--resolution",
    "scoped",
    "--taint-context-depth",
    "1",
];
