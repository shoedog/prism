//! Stage T pins sink presence for the rule identity ledger.

use super::taint_sink_rules_support::{assert_rows, findings_with_sinks, marker_line, Row};
use crate::common::Language;

const WRAPPER_ONLY: &str = r#"#include <stdarg.h>
#include <stdio.h>
void report(const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    vprintf(fmt, args);
    va_end(args);
}
void h(const char *input) {
    const char *x = input; // SEED
    report(x); // SINK
}
"#;

const CALLER_SINK_ONLY: &str = r#"void h(const char *input) {
    const char *x = input; // SEED
    audit_payload(x); // SINK
}
"#;

#[rustfmt::skip]
const IDENTITY_ROWS: &[Row] = &[
    Row { sink: true, label: "ri01", path: "a.js", lang: Language::JavaScript, source: "function h(input, res) {\n  const x = input.v; // SEED\n  res.send(eval(x)); // SINK\n}\n" },
    Row { sink: true, label: "ri02", path: "a.js", lang: Language::JavaScript, source: "const cp = require('child_process');\nfunction h(input, el) {\n  const x = input.v; // SEED\n  el.innerHTML = x; cp.exec(x); // SINK\n}\n" },
    Row { sink: true, label: "ri03", path: "a.js", lang: Language::JavaScript, source: "function h(input) {\n  const x = input.v; // SEED\n  fetch(x); // SINK\n}\n" },
    Row { sink: true, label: "ri04", path: "a.py", lang: Language::Python, source: "import pickle\ndef h(inp):\n    x = inp.v  # SEED\n    pickle.loads(x)  # SINK\n" },
    Row { sink: true, label: "ri05", path: "a.go", lang: Language::Go, source: "package p\nimport \"os\"\nfunc h(input string) {\n  x := input // SEED\n  os.Remove(x) // SINK\n}\n" },
    Row { sink: true, label: "ri05b", path: "a.go", lang: Language::Go, source: "package p\nimport \"os\"\nfunc h(input string) {\n  x := input // SEED\n  os.ReadFile(x) // SINK\n}\n" },
    Row { sink: true, label: "ri06", path: "a.py", lang: Language::Python, source: "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(\"SELECT \" + x)  # SINK\n" },
    Row { sink: true, label: "ri07", path: "a.js", lang: Language::JavaScript, source: "function h(input, res) {\n  const x = input.v; // SEED\n  res.send(x); res.send(x); // SINK\n}\n" },
    Row { sink: true, label: "ri08", path: "a.py", lang: Language::Python, source: "import os\ndef h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(\"SELECT 1\"); os.system(x)  # SINK\n" },
    Row { sink: true, label: "ri09", path: "a.c", lang: Language::C, source: "void h(const char *input) {\n  const char *x = input; // SEED\n  system(x); // SINK\n}\n" },
    Row { sink: true, label: "ri10", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  eval \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "ri11", path: "a.go", lang: Language::Go, source: r#"package p
import "github.com/gin-gonic/gin" // SEED
func h(c *gin.Context) {
    c.File(c.Param("f")) // SINK
}
"# },
    Row { sink: true, label: "ri12", path: "a.py", lang: Language::Python, source: r#"from flask import Flask, request  # SEED
import pickle

app = Flask(__name__)

@app.route("/payload")
def h():
    return pickle.loads(request.args.get("payload").encode())  # SINK
"# },
    Row { sink: true, label: "ri13", path: "a.js", lang: Language::JavaScript, source: r#"import express from 'express'; // SEED
import yaml from 'js-yaml';

const app = express();
app.post('/yaml', (req, res) => {
  return yaml.load(req.body); // SINK
});
"# },
    Row { sink: true, label: "ri14", path: "a.c", lang: Language::C, source: WRAPPER_ONLY },
    Row { sink: true, label: "ri15", path: "a.go", lang: Language::Go, source: r#"package p
import (
    "fmt"
    "os"
    "path/filepath"
    "strings"

    "github.com/gin-gonic/gin"
)
func h(c *gin.Context) {
    name := c.Param("file") // SEED
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/safe/") {
        return
    }
    _ = os.WriteFile(cleaned, []byte("data"), 0644); _ = fmt.Sprintf(cleaned) // SINK
}
"# },
    Row { sink: true, label: "ri16", path: "A.java", lang: Language::Java, source: "class A {\n  void h(Runtime runtime, String input) throws Exception {\n    String x = input; // SEED\n    String copy = x; runtime.exec(copy); // SINK\n  }\n}\n" },
    Row { sink: true, label: "ri17", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n  let x = inp.v.clone(); // SEED\n  std::process::Command::new(x).spawn(); // SINK\n}\n" },
    Row { sink: true, label: "ri18", path: "a.lua", lang: Language::Lua, source: "function h(inp)\n  local x = inp.v -- SEED\n  os.execute(x) -- SINK\nend\n" },
    Row { sink: true, label: "ri19", path: "a.tsx", lang: Language::Tsx, source: "export function H(input: Input, res: Response) {\n  const x = input.v; // SEED\n  res.send(eval(x)); // SINK\n  return <div />;\n}\n" },
    Row { sink: true, label: "ri20", path: "a.py", lang: Language::Python, source: "def h(cur, inp):\n    x = inp.v  # SEED\n    cur.execute(\n        \"SELECT \" + x  # SINK\n    )\n" },
    Row { sink: true, label: "ri21a", path: "a.py", lang: Language::Python, source: "import pickle\nimport requests\ndef h(inp):\n    x = inp.v  # SEED\n    requests.get(x); pickle.loads(x)  # SINK\n" },
    Row { sink: true, label: "ri21b", path: "a.py", lang: Language::Python, source: "import pickle\nimport requests\ndef h(inp):\n    x = inp.v  # SEED\n    pickle.loads(x); requests.get(x)  # SINK\n" },
    Row { sink: true, label: "ri22", path: "a.py", lang: Language::Python, source: r#"import os
import requests
from django.utils.html import format_html
from django.utils.safestring import mark_safe

def h(inp):
    x = inp.v  # SEED
    escaped = format_html("{}", x)
    mark_safe(escaped); requests.get(escaped); os.system(escaped)  # SINK
"# },
    Row { sink: true, label: "ri24", path: "a.js", lang: Language::JavaScript, source: "const sqlite3 = require('sqlite3');\nconst db = new sqlite3.Database(':memory:');\nfunction h(input) {\n  const sql = input.v; // SEED\n  db.exec(sql); // SINK\n}\n" },
    Row { sink: true, label: "ri25", path: "a.go", lang: Language::Go, source: r#"package p
import (
    "os/exec"

    "github.com/gin-gonic/gin"
)
func h(c *gin.Context, input string) {
    x := input // SEED
    c.HTML(200, "t", x); _ = exec.Command("ls") // SINK
}
"# },
    Row { sink: true, label: "ri26", path: "a.js", lang: Language::JavaScript, source: "function h(input, res, el) {\n  const x = input.v; // SEED\n  res.send('ok'); el.innerHTML = x; // SINK\n}\n" },
    // The review taint graph does not connect references across Terraform blocks.
    Row { sink: false, label: "ri27", path: "a.tf", lang: Language::Terraform, source: r#"locals {
  payload = var.payload # SEED
}
resource "aws_instance" "example" {
  user_data = local.payload # SINK
}
"# },
];

#[rustfmt::skip]
const CONFIG_ROWS: &[(Row, &[&str])] = &[
    (Row { sink: true, label: "ri23a", path: "a.c", lang: Language::C, source: CALLER_SINK_ONLY }, &["audit"]),
    (Row { sink: true, label: "ri23b", path: "a.c", lang: Language::C, source: WRAPPER_ONLY }, &[]),
    (Row { sink: true, label: "ri23c", path: "a.c", lang: Language::C, source: r#"#include <stdarg.h>
#include <stdio.h>
void report(const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    vprintf(fmt, args);
    va_end(args);
}
void h(const char *input) {
    const char *x = input; // SEED
    audit_payload(x); report(x); // SINK
}
"# }, &["audit"]),
    (Row { sink: true, label: "ri23d", path: "a.c", lang: Language::C, source: "void h(const char *input) {\n  const char *x = input; // SEED\n  system(x); // SINK\n}\n" }, &["=system"]),
    (Row { sink: false, label: "ri23e", path: "a.c", lang: Language::C, source: CALLER_SINK_ONLY }, &["=audit"]),
    (Row { sink: false, label: "ri23f", path: "a.c", lang: Language::C, source: CALLER_SINK_ONLY }, &[]),
];

#[test]
fn identity_rows_fire_as_marked() {
    assert_rows(IDENTITY_ROWS);
}

#[test]
fn config_rows_fire_as_marked() {
    let mut wrong = Vec::new();
    for (row, extra_sinks) in CONFIG_ROWS {
        let sink_line = marker_line(row.source, "SINK");
        let result = findings_with_sinks(row.path, row.lang, row.source, extra_sinks);
        let has_sink = result.findings.iter().any(|finding| {
            finding.line == sink_line && finding.category.as_deref() == Some("taint_sink")
        });
        if has_sink != row.sink {
            wrong.push(format!("{} (expected sink={})", row.label, row.sink));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} rows differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
