//! The reusable route table of source admission (#602 S3a.1s). S3b re-runs it.
//!
//! Fixtures are REAL (parsed through the real detectors and `slice_scoped`) unless a test
//! wires a controlled edge itself. Each paired fixture puts the SAME text in an original file and in a
//! loaded context file (`ctx/...`); the original set holds only the original file. A run
//! with every loaded file original must first show the evidence (`Needs`) in BOTH copies.
use super::*;
use crate::algorithms::taint::source_scope::admission::{slice_scoped, ScopedTaintOutcome};

/// Evidence both copies of a paired fixture must show on the all-original run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Needs {
    Root,
    Sink,
    Nothing,
}

/// One named route fixture. S3b re-runs this table through its combined entry.
#[derive(Clone)]
pub(super) struct RouteCase {
    pub(super) name: &'static str,
    pub(super) files: Vec<(&'static str, &'static str)>,
    pub(super) original: Vec<&'static str>,
    pub(super) diff: Vec<(&'static str, Vec<usize>)>,
    pub(super) config: TaintConfig,
    pub(super) needs: Needs,
}

impl RouteCase {
    pub(super) fn input(&self) -> DiffInput {
        let files = self.diff.iter().map(|(path, lines)| DiffInfo {
            file_path: (*path).into(),
            modify_type: ModifyType::Modified,
            diff_lines: lines.iter().copied().collect(),
        });
        DiffInput {
            files: files.collect(),
        }
    }

    pub(super) fn loaded_set(&self) -> BTreeSet<String> {
        self.files
            .iter()
            .map(|(path, _)| path.to_string())
            .collect()
    }

    pub(super) fn original_path(&self) -> &'static str {
        self.files[0].0
    }

    pub(super) fn context_path(&self) -> &'static str {
        self.files[1].0
    }

    pub(super) fn source(&self) -> &'static str {
        self.files[0].1
    }

    /// Parse, build, wire a controlled edge (or nothing), then run the scoped entry.
    pub(super) fn run(
        &self,
        original: &BTreeSet<String>,
        wire: impl FnOnce(&mut CpgContext),
    ) -> ScopedTaintOutcome {
        let files = parse(&self.files);
        let mut ctx = CpgContext::build(&files, None);
        wire(&mut ctx);
        slice_scoped(&ctx, &self.input(), &self.config, original).expect("scoped taint")
    }

    pub(super) fn scoped(&self) -> ScopedTaintOutcome {
        self.run(&originals(&self.original), |_| {})
    }

    /// The control run: every loaded file original. Asserts `needs` for every file.
    pub(super) fn all_original(&self) -> ScopedTaintOutcome {
        let all = self.run(&self.loaded_set(), |_| {});
        for (path, _) in &self.files {
            let root = all.trace.roots.iter().any(|r| r.seed.file == *path);
            let sink = !sink_lines(&all.result, path).is_empty();
            let ok = match self.needs {
                Needs::Root => root,
                Needs::Sink => sink,
                Needs::Nothing => true,
            };
            let msg = "inadmissible precondition";
            assert!(
                ok,
                "{msg}: {} shows no {:?} in {path}",
                self.name, self.needs
            );
        }
        all
    }
}

pub(super) fn sink_lines(result: &SliceResult, file: &str) -> Vec<usize> {
    let sinks = result.findings.iter();
    let sinks = sinks.filter(|f| f.category.as_deref() == Some("taint_sink") && f.file == file);
    sinks.map(|f| f.line).collect()
}

pub(super) fn line_of(source: &str, needle: &str) -> usize {
    let at = source.find(needle);
    let at = at.unwrap_or_else(|| panic!("inadmissible precondition: {needle:?} not in fixture"));
    source[..at].matches('\n').count() + 1
}

pub(super) fn snaps<'a>(roots: impl IntoIterator<Item = &'a TaintRoot>) -> Vec<String> {
    roots.into_iter().map(snapshot).collect()
}

pub(super) fn roots_of<'a>(outcome: &'a ScopedTaintOutcome, file: &str) -> Vec<&'a TaintRoot> {
    outcome
        .trace
        .roots
        .iter()
        .filter(|r| r.seed.file == file)
        .collect()
}

pub(super) const APP_JS: &str = "function h(x) {\n  unknown(x);\n}\n";
const EXPRESS_JS: &str = "const express = require('express');\nconst app = express();\napp.get('/', (req, res) => {\n  const x = req.query.q;\n  eval(x);\n});\n";
const EXPRESS_TS: &str = "import express, { Request, Response } from \"express\";\nconst app = express();\napp.get(\"/\", (req: Request, res: Response) => {\n  const x = req.query.q;\n  eval(x);\n});\n";
const DJANGO: &str = "def lookup_view(request):\n    q = request.GET.get(\"q\")\n    cursor.execute(f\"SELECT * FROM users WHERE name = '{q}'\")\n";
pub(super) const GIN_QUERY: &str = "package main\n\nimport \"github.com/gin-gonic/gin\"\n\nfunc h(c *gin.Context) {\n\tq := c.Query(\"q\")\n\tc.File(q)\n}\n";
const FLASK_INLINE: &str = "from flask import Flask, request, render_template_string\n\napp = Flask(__name__)\n\n@app.route(\"/profile\")\ndef profile():\n    return render_template_string(\"{{ greeting | safe }}\", greeting=request.args.get(\"name\"))\n";
const FLASK_ESCAPED: &str = "from flask import Flask, request, render_template_string\n\napp = Flask(__name__)\n\n@app.route(\"/profile\")\ndef profile():\n    name = request.args.get(\"name\")\n    return render_template_string(\"Hello {{ name }}\", name=name)\n";
const IPC_C: &str =
    "void h(void *data) {\n  char *x = g_hash_table_lookup(data, \"key\");\n  system(x);\n}\n";
const YAML_JS: &str = "import express from \"express\";\n\nconst app = express();\n\napp.post(\"/yaml\", (req, res) => {\n  return load(req.body.payload);\n});\n\nconst { load } = require(\"js-yaml\");\n";
const YAML_TS: &str = "import express from \"express\";\nimport { load } from \"js-yaml\";\n\nconst app = express();\n\napp.post(\"/yaml\", (req, res) => {\n  return load(req.body.payload);\n});\n";
const GO_PARAM: &str = "package main\n\nimport \"github.com/gin-gonic/gin\"\n\nfunc handler(c *gin.Context) {\n\tc.File(c.Param(\"f\"))\n}\n";
const GO_MIXED: &str = "package main\n\nimport (\n\t\"fmt\"\n\n\t\"github.com/gin-gonic/gin\"\n)\n\nfunc handler(c *gin.Context) {\n\tother := c.Query(\"other\"); c.File(c.Param(\"f\")); fmt.Println(other)\n}\n";
const GO_LITERAL: &str = "package main\n\nimport (\n\t\"fmt\"\n\n\t\"github.com/gin-gonic/gin\"\n)\n\nfunc handler(c *gin.Context) {\n\tother := c.Query(\"other\"); c.File(\"public.txt\"); fmt.Println(other)\n}\n";
// RS09 / RS10 / RS14 fixtures: the entry file that holds the eligible original roots.
pub(super) const ORIG_GO: &str = "package main\n\nfunc g(d string) {\n\tp := d\n}\n";
const ORIG_JS: &str = "function g(d) {\n  const p = d;\n}\n";
// Same names, no framework import or handler: the context line is NOT a recognised source.
const PLAIN_GO: &str = "package main\n\nfunc h(c *Ctx) {\n\tq := c.Query(\"q\")\n\tc.File(q)\n}\n";
const FACTS_JS: &str = "const express = require('express');\nconst app = express();\napp.get('/', (req, res) => {\n  const Query = req.query.exec;\n  eval(Query);\n});\n";
const PLAIN_JS: &str =
    "function h(req, res) {\n  const Query = req.query.exec;\n  eval(Query);\n}\n";
const PY_POSITIVE: &str = "import os\ndef h(user):\n    command = user\n    os.system(command)\n";
const PY_NEGATIVE: &str = "import os\ndef h(user):\n    command = user\n    return 42\n";

const JS: (&str, &str) = ("app.js", "ctx/app.js");

fn framework_only() -> TaintConfig {
    TaintConfig {
        taint_from_diff: false,
        ..TaintConfig::default()
    }
}

fn pair(
    name: &'static str,
    paths: (&'static str, &'static str),
    text: &'static str,
    needs: Needs,
    diff_lines: &[usize],
    config: TaintConfig,
) -> RouteCase {
    let diff = |path| (path, diff_lines.to_vec());
    RouteCase {
        name,
        files: vec![(paths.0, text), (paths.1, text)],
        original: vec![paths.0],
        diff: if diff_lines.is_empty() {
            vec![]
        } else {
            vec![diff(paths.0), diff(paths.1)]
        },
        config,
        needs,
    }
}

fn entry_and_context(
    name: &'static str,
    entry: (&'static str, &'static str),
    context: (&'static str, &'static str),
    diff: (&'static str, usize),
) -> RouteCase {
    RouteCase {
        name,
        files: vec![entry, context],
        original: vec![entry.0],
        diff: vec![(diff.0, vec![diff.1])],
        config: TaintConfig::default(),
        needs: Needs::Nothing,
    }
}

/// Ordered explicit exclusions: scenarios this table cannot hold, with the reason.
pub(super) const ROUTE_EXCLUSIONS: [(&str, &str); 3] = [
    (
        "RS15",
        "public-API integration test; it cannot import crate-private items",
    ),
    (
        "RS09 wiring",
        "controlled edges are test code: `facts_go`/`facts_js` hold only files",
    ),
    (
        "RS10 wiring",
        "the NameOnly cross edge is injected by `consumer_nameonly`'s test",
    ),
];

pub(super) const ROUTE_NAMES: [&str; 24] = [
    "diff_roots",
    "explicit",
    "express_js",
    "express_ts",
    "django",
    "gin_query",
    "flask_inline",
    "ipc_c",
    "yaml_js",
    "yaml_ts",
    "yaml_js_literal",
    "yaml_ts_literal",
    "flask_escaped",
    "go_param",
    "go_mixed",
    "go_literal",
    "facts_go",
    "facts_js",
    "plain_go",
    "plain_js",
    "py_positive",
    "py_negative",
    "consumer_nameonly",
    "express_targeted",
];

pub(super) fn route_cases() -> Vec<RouteCase> {
    let (py, ts, go) = (
        ("app.py", "ctx/app.py"),
        ("app.ts", "ctx/app.ts"),
        ("a.go", "ctx/a.go"),
    );
    let mut diff_roots = pair(
        "diff_roots",
        JS,
        APP_JS,
        Needs::Root,
        &[2],
        TaintConfig::default(),
    );
    diff_roots.diff.push(("gone.js", vec![2])); // not loaded: silently omitted (inherited)
    let mut explicit = pair("explicit", JS, APP_JS, Needs::Root, &[], framework_only());
    explicit.original.push("missing.js"); // listed as original, never loaded
    explicit.config.sources = [
        ("app.js", 2),
        ("ctx/app.js", 2),
        ("missing.js", 7),
        ("missing2.js", 9),
        ("ctx/app.js", 2),
        ("app.js", 3), // a loaded original file at a line with no variable
    ]
    .iter()
    .map(|&(file, line)| (file.to_string(), line))
    .collect();
    let yaml_literal = |name, paths, text: &'static str| {
        let literal = Box::leak(
            text.replace("load(req.body.payload)", "load(\"a: 1\")")
                .into_boxed_str(),
        );
        pair(name, paths, literal, Needs::Nothing, &[], framework_only())
    };
    let mut cases = vec![
        diff_roots,
        explicit,
        pair(
            "express_js",
            JS,
            EXPRESS_JS,
            Needs::Root,
            &[5],
            framework_only(),
        ),
        pair(
            "express_ts",
            ts,
            EXPRESS_TS,
            Needs::Root,
            &[5],
            framework_only(),
        ),
        pair(
            "django",
            ("views.py", "ctx/views.py"),
            DJANGO,
            Needs::Root,
            &[3],
            framework_only(),
        ),
        pair(
            "gin_query",
            go,
            GIN_QUERY,
            Needs::Root,
            &[],
            framework_only(),
        ),
        pair(
            "flask_inline",
            py,
            FLASK_INLINE,
            Needs::Sink,
            &[],
            framework_only(),
        ),
        pair(
            "ipc_c",
            ("app.c", "ctx/app.c"),
            IPC_C,
            Needs::Root,
            &[3],
            framework_only(),
        ),
        pair("yaml_js", JS, YAML_JS, Needs::Sink, &[], framework_only()),
        pair("yaml_ts", ts, YAML_TS, Needs::Sink, &[], framework_only()),
        yaml_literal("yaml_js_literal", JS, YAML_JS),
        yaml_literal("yaml_ts_literal", ts, YAML_TS),
        pair(
            "flask_escaped",
            py,
            FLASK_ESCAPED,
            Needs::Root,
            &[],
            framework_only(),
        ),
        pair("go_param", go, GO_PARAM, Needs::Sink, &[], framework_only()),
        pair("go_mixed", go, GO_MIXED, Needs::Sink, &[], framework_only()),
        pair(
            "go_literal",
            go,
            GO_LITERAL,
            Needs::Root,
            &[],
            framework_only(),
        ),
    ];
    cases.push(entry_and_context(
        "facts_go",
        ("a.go", ORIG_GO),
        ("ctx/b.go", GIN_QUERY),
        ("a.go", 4),
    ));
    cases.push(entry_and_context(
        "facts_js",
        ("a.js", ORIG_JS),
        ("ctx/b.js", FACTS_JS),
        ("a.js", 2),
    ));
    cases.push(entry_and_context(
        "plain_go",
        ("a.go", ORIG_GO),
        ("ctx/b.go", PLAIN_GO),
        ("a.go", 4),
    ));
    cases.push(entry_and_context(
        "plain_js",
        ("a.js", ORIG_JS),
        ("ctx/b.js", PLAIN_JS),
        ("a.js", 2),
    ));
    cases.push(pair(
        "py_positive",
        py,
        PY_POSITIVE,
        Needs::Sink,
        &[3],
        TaintConfig::default(),
    ));
    cases.push(pair(
        "py_negative",
        py,
        PY_NEGATIVE,
        Needs::Root,
        &[3],
        TaintConfig::default(),
    ));
    // RS10: framework file sorts first; the diff file is original too (both loaded).
    let mut consumer = entry_and_context(
        "consumer_nameonly",
        ("a.go", GIN_QUERY),
        ("b.go", ORIG_GO),
        ("b.go", 4),
    );
    consumer.original.push("b.go");
    cases.push(consumer);
    cases.push(pair(
        "express_targeted",
        JS,
        EXPRESS_JS,
        Needs::Root,
        &[5],
        TaintConfig::default(),
    ));
    cases
}

pub(super) fn route(name: &str) -> RouteCase {
    let found = route_cases().into_iter().find(|case| case.name == name);
    found.unwrap_or_else(|| panic!("inadmissible precondition: no route case {name}"))
}
