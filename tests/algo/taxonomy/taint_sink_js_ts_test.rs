use crate::common::*;

fn run_taint_js_ts_single(
    source: &str,
    path: &str,
    language: Language,
    diff_lines: BTreeSet<usize>,
) -> prism::slice::SliceResult {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink_on(result: &prism::slice::SliceResult, line: usize) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

#[test]
fn test_tsx_nest_body_reaches_dangerously_set_inner_html() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    return <div dangerouslySetInnerHTML={{ __html: html }} />;
  }
}
"#;
    let result = run_taint_js_ts_single(source, "page.tsx", Language::Tsx, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "NestJS @Body DTO field should reach TSX dangerouslySetInnerHTML.__html"
    );
}

#[test]
fn test_nestjs_body_comment_does_not_seed_source() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";
import yaml from "js-yaml";

@Controller("/config")
export class ConfigController {
  @Post("/parse")
  parseConfig(/* @Body() */ body: ConfigDto) {
    return yaml.load(body.yamlContent);
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        !has_taint_sink(&result),
        "NestJS source decorators must be AST decorators, not comment text in a parameter"
    );
}

#[test]
fn test_nestjs_controller_without_method_route_does_not_seed_source() {
    let source = r#"import { Body, Controller } from "@nestjs/common";
import yaml from "js-yaml";

@Controller("/config")
export class ConfigController {
  helper(@Body() body: ConfigDto) {
    return yaml.load(body.yamlContent);
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        !has_taint_sink(&result),
        "NestJS @Controller alone should not make a method a route source"
    );
}

#[test]
fn test_tsx_text_interpolation_is_not_xss_sink() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    return <div>{html}</div>;
  }
}
"#;
    let result = run_taint_js_ts_single(source, "page.tsx", Language::Tsx, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "React text interpolation is escaped by default and should not be a Phase 3 XSS sink"
    );
}

#[test]
fn test_express_request_query_reaches_sequelize_query() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Express req.query assignment should reach SQL raw query sink"
    );
}

#[test]
fn test_express_inline_query_reaches_inner_html() {
    let source = r#"import express from "express";

const app = express();

app.get("/profile", function(req, res) {
  document.getElementById("out").innerHTML = req.query.name;
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 6),
        "framework-source line suppression should not hide real JS/TS flat sinks on the same line"
    );
}

#[test]
fn test_express_compound_same_line_query_reaches_inner_html() {
    let source = r#"import express from "express";

const app = express();

app.get("/profile", function(req, res) {
  req.query.enabled && (document.getElementById("out").innerHTML = req.query.name);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 6),
        "source access ranges should stay narrow enough for later same-line flat sinks to fire"
    );
}

#[test]
fn test_express_nested_flat_sink_inside_safe_structured_call_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.get("/profile", function(req, res) {
  const name = req.query.name;
  return fetch("https://example.com", document.getElementById("out").innerHTML = name);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "safe structured calls must not suppress nested flat sinks in other arguments"
    );
}

#[test]
fn test_node_http_request_tainted_second_arg_still_fires_when_first_arg_safe() {
    let source = r#"import express from "express";
import * as http from "node:http";

const app = express();

app.get("/proxy", function(req, res) {
  const options = { path: req.query.path };
  return http.request("https://safe.example", options);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "structured sink suppression must not hide a tainted later argument when an earlier argument is safe"
    );
}

#[test]
fn test_express_same_line_source_target_name_does_not_suppress_real_flat_sink() {
    let source = r#"import express from "express";

const app = express();

app.get("/profile", function(req, res) {
  const innerHTML = req.query.name; document.getElementById("out").innerHTML = innerHTML;
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 6),
        "source target bindings should not suppress same-named flat sinks elsewhere on the line"
    );
}

#[test]
fn test_express_import_unregistered_handler_shape_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

function helper(req, res) {
  const term = req.query.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
}
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "Express import plus req/res-shaped helper must not seed request taint without route registration"
    );
}

#[test]
fn test_express_named_route_handler_still_taints() {
    let source = r#"import express from "express";

const app = express();

function search(req, res) {
  const term = req.query.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
}

app.get("/search", search);
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "registered named Express handlers should still seed request taint"
    );
}

#[test]
fn test_express_multiline_query_arg_reaches_sequelize_query() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  return sequelize.query(
    `SELECT * FROM users WHERE name = '${term}'`
  );
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "multi-line structured sink args should match tainted identifiers on their own lines"
    );
}

#[test]
fn test_express_request_param_multi_hop_alias_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/item/:id", function(req, res) {
  const a = req.params.id;
  const b = a;
  return sequelize.query(`SELECT * FROM items WHERE id = '${b}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express request-data aliases should keep propagating through standard JS DFG"
    );
}

#[test]
fn test_express_request_method_assignment_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const method = req.method;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "server-controlled req.method should not be synthesized as request-data taint"
    );
}

#[test]
fn test_express_request_method_stays_untainted_after_query_read() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const target = req.query.target_url;
  const method = req.method;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "reading allowed request data must not taint later server-controlled request fields"
    );
}

#[test]
fn test_express_request_alias_query_reaches_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req;
  const target = request.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "aliases of the request object should still expose allowed request-data accessors"
    );
}

#[test]
fn test_express_typescript_asserted_request_alias_query_reaches_ssrf() {
    let source = r#"import express, { Request } from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req as Request;
  const target = request.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "TypeScript assertion wrappers around request aliases should still expose request data"
    );
}

#[test]
fn test_express_request_alias_query_direct_sink_reaches_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req;
  return axios.get(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "request-object aliases should reach direct request-data sink dereferences"
    );
}

#[test]
fn test_express_block_scoped_request_alias_reaches_sink_inside_block() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  if (req.query.enabled) {
    const request = req;
    return axios.get(request.query.target_url);
  }
  return res.send("ok");
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "block-scoped request aliases should remain usable within their lexical scope"
    );
}

#[test]
fn test_express_multiline_direct_query_reaches_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  return axios.get(
    req
      .query
      .target_url
  );
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "multi-line request member accesses should still reach direct sinks"
    );
}

#[test]
fn test_express_multiline_request_alias_query_reaches_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req;
  return axios.get(
    request
      .query
      .target_url
  );
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "multi-line request alias member accesses should still reach direct sinks"
    );
}

#[test]
fn test_express_request_alias_same_line_reaches_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) { const request = req; return axios.get(request.query.target_url); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 6),
        "same-line request aliases should remain visible when the line dereferences allowed request data"
    );
}

#[test]
fn test_express_request_alias_method_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req;
  const method = request.method;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request-object aliases should not make server-controlled method fields tainted"
    );
}

#[test]
fn test_express_request_alias_same_line_definition_method_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req; return sequelize.query(`SELECT * FROM methods WHERE name = '${request.method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bare request-alias definitions must not taint server-owned same-line fields"
    );
}

#[test]
fn test_express_request_alias_method_stays_untainted_after_query_read() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req;
  const target = request.query.target_url;
  const method = request.method;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "reading allowed request data through an alias must not taint later server-owned alias fields"
    );
}

#[test]
fn test_express_block_scoped_request_alias_does_not_leak_to_outer_binding() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = { body: { term: "safe" } };
  if (req.query.enabled) {
    const request = req;
    const local = request.body.term;
  }
  return sequelize.query(`SELECT * FROM users WHERE name = '${request.body.term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases declared inside a block must not taint same-named outer bindings"
    );
}

#[test]
fn test_express_same_line_block_scoped_request_alias_does_not_leak_after_block() {
    let source = r#"import express from "express";
import axios from "axios";
const app = express();
app.get("/proxy", function(req, res) { const request = { query: { target_url: "https://safe.example" } }; if (req.query.enabled) { const request = req; } return fetch(request.query.target_url); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "byte-scoped request aliases should not leak past a same-line closing block"
    );
}

#[test]
fn test_express_same_line_block_scoped_alias_does_not_taint_later_assignment() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const request = { query: { target_url: "https://safe.example" } };
  let target = "https://safe.example";
  if (req.query.enabled) { const request = req; } target = request.query.target_url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "block-scoped request aliases should not taint later same-line code after the block closes"
    );
}

#[test]
fn test_express_same_line_block_scoped_target_does_not_taint_outer_target_after_block() {
    let source = r#"import express from "express";
const app = express();
app.get("/proxy", function(req, res) { let target = "https://safe.example"; if (req.query.enabled) { const request = req; const target = request.query.target_url; } return fetch(target); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "same-line block-scoped target seeds should not leak to same-named outer targets after the block"
    );
}

#[test]
fn test_express_same_line_request_alias_use_before_definition_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) { return fetch(request.query.target_url); const request = req; });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases should not be visible before their same-line definition"
    );
}

#[test]
fn test_express_block_scoped_assignment_alias_does_not_leak_to_outer_binding() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const request = { query: { target_url: "https://safe.example" } };
  if (req.query.enabled) {
    let request;
    request = req;
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases assigned to block-scoped bindings must not leak to same-named outer bindings"
    );
}

#[test]
fn test_express_request_alias_safe_reassignment_drops_request_taint() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  let request = req;
  request = { query: { target_url: "https://safe.example" } };
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "safe reassignments to request aliases should end the previous request alias"
    );
}

#[test]
fn test_express_request_alias_conditional_safe_reassignment_preserves_request_taint() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  let request = req;
  if (req.query.safe) {
    request = { query: { target_url: "https://safe.example" } };
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "conditional safe reassignments must not erase the request alias on paths that skip the branch"
    );
}

#[test]
fn test_express_request_alias_assignment_does_not_leak_to_else_branch() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  let request = {};
  if (Math.random() > 0.5) {
    request = req;
  } else {
    return fetch(request.query.target_url);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases assigned in one branch must not taint sibling branches"
    );
}

#[test]
fn test_express_request_alias_assignment_does_not_leak_to_ternary_sibling_arm() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  Math.random() > 0.5 ? (request = req) : fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases assigned in one ternary arm must not taint sibling arms"
    );
}

#[test]
fn test_express_request_data_assignment_does_not_leak_to_else_branch() {
    let source = r#"import express from "express";
const app = express();

app.get("/proxy", function(req, res) {
  let target = "https://safe.example";
  if (Math.random() > 0.5) {
    target = req.query.target_url;
  } else {
    return fetch(target);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request-data assignments in one branch must not taint sibling branches"
    );
}

#[test]
fn test_express_request_alias_assignment_in_branch_reaches_after_if() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  let request = {};
  if (Math.random() > 0.5) {
    request = req;
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 11),
        "request aliases assigned in a branch should conservatively taint later post-branch uses"
    );
}

#[test]
fn test_express_request_alias_assignment_in_returning_branch_does_not_reach_after_if() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  if (req.query.enabled) {
    request = req;
    return res.send("stopped");
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases from terminating branches should not taint post-branch code"
    );
}

#[test]
fn test_express_request_data_assignment_in_returning_branch_does_not_reach_after_if() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let target = "https://safe.example";
  if (req.query.enabled) {
    target = req.query.target_url;
    return res.send("stopped");
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request-data assignments in terminating branches should not taint post-branch code"
    );
}

#[test]
fn test_express_request_alias_short_circuit_safe_reassignment_preserves_request_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = req;
  req.query.safe && (request = { query: { target_url: "https://safe.example" } });
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "short-circuit safe reassignments must not erase request aliases on paths that skip the RHS"
    );
}

#[test]
fn test_express_destructured_request_alias_safe_reassignment_drops_request_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let { query } = req;
  ({ query } = { query: { target_url: "https://safe.example" } });
  return fetch(query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "safe destructuring reassignments should end previous destructured request aliases"
    );
}

#[test]
fn test_express_same_line_sink_before_safe_reassignment_still_taints() {
    let source = r#"import express from "express";
const app = express();
app.get("/proxy", function(req, res) { let target = req.query.target_url; fetch(target); target = "https://safe.example"; });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 3),
        "safe reassignments later on the same line must not erase earlier source-to-sink flow"
    );
}

#[test]
fn test_express_same_line_safe_reassignment_before_sink_drops_request_data() {
    let source = r#"import express from "express";
const app = express();
app.get("/search", function(req, res) { let term = req.query.term; term = "safe"; return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "safe reassignments earlier on the same line should clear later request-data uses"
    );
}

#[test]
fn test_express_same_line_request_data_alias_chain_reaches_sql() {
    let source = r#"import express from "express";
const app = express();
app.get("/search", function(req, res) { const a = req.query.term; const b = a; return sequelize.query(`SELECT * FROM users WHERE name = '${b}'`); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 3),
        "same-line aliases after request-data reads should remain tainted by byte order"
    );
}

#[test]
fn test_express_request_alias_reassignment_back_to_request_retaints() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  request = req;
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "request aliases should become active again after reassignment back to the request object"
    );
}

#[test]
fn test_express_for_initializer_request_alias_does_not_leak_after_loop() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const request = { query: { target_url: "https://safe.example" } };
  for (const request = req; req.query.enabled;) {
    break;
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases declared in for initializers should stop at the loop scope"
    );
}

#[test]
fn test_express_for_initializer_safe_alias_shadows_outer_request_alias_inside_loop() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req;
  for (const request = { query: { target_url: "https://safe.example" } }; req.query.enabled;) {
    return fetch(request.query.target_url);
  }
  return res.send("ok");
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "safe for-initializer declarations should shadow outer request aliases inside the loop"
    );
}

#[test]
fn test_express_switch_case_request_alias_does_not_leak_after_switch() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      const request = req;
      break;
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases declared in switch cases should not leak past the switch body"
    );
}

#[test]
fn test_express_switch_case_request_alias_assignment_does_not_leak_to_sibling_case() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      request = req;
      break;
    case "y":
      return fetch(request.query.target_url);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases assigned in one switch case must not taint sibling cases"
    );
}

#[test]
fn test_express_switch_case_request_alias_assignment_falls_through_to_later_case() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      request = req;
    case "y":
      return fetch(request.query.target_url);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 12),
        "request aliases assigned in a fallthrough switch case should taint later cases"
    );
}

#[test]
fn test_express_switch_default_middle_assignment_falls_through_to_later_case() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      break;
    default:
      request = req;
    case "y":
      return fetch(request.query.target_url);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 13),
        "request aliases assigned in a middle default case should taint later cases when default falls through"
    );
}

#[test]
fn test_express_switch_default_middle_assignment_break_does_not_reach_later_case() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      break;
    default:
      request = req;
      break;
    case "y":
      return fetch(request.query.target_url);
  }
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request aliases assigned in a middle default case must not taint later cases after break"
    );
}

#[test]
fn test_express_switch_case_request_alias_assignment_reaches_after_switch() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let request = { query: { target_url: "https://safe.example" } };
  switch (req.query.mode) {
    case "x":
      request = req;
      break;
  }
  return fetch(request.query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 12),
        "request aliases assigned in switch cases should conservatively taint post-switch uses"
    );
}

#[test]
fn test_express_request_alias_mixed_access_line_does_not_taint_method_sink_arg() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req;
  console.log(request.query.term); return sequelize.query(`SELECT * FROM methods WHERE name = '${request.method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "same-line request-data reads must not taint server-owned request fields in sink args"
    );
}

#[test]
fn test_express_request_alias_mixed_access_line_does_not_taint_render_method_arg() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req;
  console.log(request.query.term); return res.render(request.method);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "same-line request-data reads must not make unrelated flat call sinks line-wide"
    );
}

#[test]
fn test_express_request_alias_mixed_access_line_render_query_arg_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const request = req;
  console.log(request.method); return res.render(request.query.term);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "flat call sinks on source lines should still fire when their argument is request data"
    );
}

#[test]
fn test_express_multiple_request_aliases_same_line_do_not_cross_taint_fields() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const a = req;
  const b = req;
  console.log(a.query.term); return sequelize.query(`SELECT * FROM methods WHERE name = '${b.method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "a request-data read through one alias must not taint a server-owned field on another alias"
    );
}

#[test]
fn test_express_block_assignment_to_outer_target_reaches_sink_after_block() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let target = "safe";
  if (req.query.enabled) {
    target = req.query.target_url;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 10),
        "assignments to predeclared outer variables should retain the outer binding scope"
    );
}

#[test]
fn test_express_request_bracket_query_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req["query"].term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "bracket notation for allowed request-data fields should be tainted"
    );
}

#[test]
fn test_express_request_optional_chain_query_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req?.query?.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "optional chaining on allowed request-data fields should still be tainted"
    );
}

#[test]
fn test_express_request_query_sink_named_property_does_not_flat_sink() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const value = req.query.exec;
  return "ok";
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request subproperties named like sinks should not become flat sink findings on source lines"
    );
}

#[test]
fn test_express_request_bracket_method_does_not_taint() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const method = req["method"];
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bracket notation must still exclude server-controlled request fields"
    );
}

#[test]
fn test_express_nested_request_alias_does_not_leak_to_outer_scope() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  function inner() {
    const request = req;
  }
  const target = request.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request-object aliases from nested functions must not leak into the outer handler"
    );
}

#[test]
fn test_express_request_alias_use_inside_uncalled_nested_function_does_not_taint() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const request = req;
  function inner() {
    return axios.get(request.query.target_url);
  }
  return res.send("ok");
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "request-alias source scans should not cross into uncalled nested functions"
    );
}

#[test]
fn test_express_destructured_method_does_not_become_request_alias() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", function(req, res) {
  const { method } = req;
  const target = method.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "destructured server-owned fields must not become request-object aliases"
    );
}

#[test]
fn test_express_computed_destructure_key_does_not_assume_allowed_field() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const query = "method";
  const { [query]: value } = req;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${value}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "dynamic destructuring keys must not be treated as literal allowed request-data fields"
    );
}

#[test]
fn test_express_nested_computed_destructure_key_does_not_taint_key() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const safeKey = "safe";
  const { query: { [safeKey]: q } } = req;
  return sequelize.query(`SELECT * FROM users WHERE name = '${safeKey}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "computed destructuring keys under request-data fields are not bound request data"
    );
}

#[test]
fn test_express_destructured_query_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const { query } = req;
  return sequelize.query(`SELECT * FROM users WHERE name = '${query.term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "destructured allowed request-data fields should be tainted"
    );
}

#[test]
fn test_express_multiline_destructured_query_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const {
    query
  } = req;
  return sequelize.query(`SELECT * FROM users WHERE name = '${query.term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "multi-line destructured request-data fields should use the binding line as the source"
    );
}

#[test]
fn test_express_block_scoped_destructured_body_does_not_leak_to_outer_binding() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const payload = { term: "safe" };
  if (req.query.enabled) {
    const { body: payload } = req;
    const local = payload.term;
  }
  return sequelize.query(`SELECT * FROM users WHERE name = '${payload.term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 11),
        "block-scoped destructured request data must not taint same-named outer bindings"
    );
}

#[test]
fn test_express_block_scoped_destructured_assignment_does_not_leak_to_outer_binding() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  let query = { target_url: "https://safe.example" };
  if (req.query.enabled) {
    let query;
    ({ query } = req);
  }
  return fetch(query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "block-scoped destructuring assignments must not taint same-named outer bindings"
    );
}

#[test]
fn test_express_destructured_query_alias_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const { query: requestQuery } = req;
  return sequelize.query(`SELECT * FROM users WHERE name = '${requestQuery.term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "aliased destructured request-data fields should be tainted"
    );
}

#[test]
fn test_express_destructured_query_default_does_not_taint_default() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const { query: requestQuery = defaults } = req;
  return fetch(defaults.url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "destructuring defaults are fallback expressions and should not be marked as request data"
    );
}

#[test]
fn test_express_shorthand_destructured_query_default_reaches_ssrf() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const { query = {} } = req;
  return fetch(query.target_url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "shorthand destructuring defaults should taint the request field binding, not the fallback expression"
    );
}

#[test]
fn test_express_request_url_reaches_fetch_ssrf() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const target = req.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "req.url remains an allowed client-controlled request-data accessor"
    );
}

#[test]
fn test_express_request_path_reaches_send_file() {
    let source = r#"import express from "express";

const app = express();

app.get("/download", function(req, res) {
  const filename = req.path;
  return res.sendFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Express req.path is derived from the client URL and should remain tainted"
    );
}

#[test]
fn test_express_request_original_url_reaches_fetch_ssrf() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const target = req.originalUrl;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Express req.originalUrl is derived from the client URL and should remain tainted"
    );
}

#[test]
fn test_express_request_hostname_reaches_fetch_ssrf() {
    let source = r#"import express from "express";

const app = express();

app.get("/proxy", function(req, res) {
  const target = req.hostname;
  return fetch(`http://${target}/status`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "host-derived request fields should remain client-controlled SSRF inputs"
    );
}

#[test]
fn test_express_request_query_reaches_mongoose_where() {
    let source = r#"import express from "express";
import mongoose from "mongoose";

const app = express();
const User = mongoose.model("User");

app.get("/search", function(req, res) {
  const predicate = req.query.predicate;
  return User.$where(predicate);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "Mongoose $where should fire when Express query data reaches the JS predicate"
    );
}

#[test]
fn test_express_request_query_reaches_prisma_raw_unsafe() {
    let source = r#"import express from "express";
import { PrismaClient } from "@prisma/client";

const app = express();
const prisma = new PrismaClient();

app.get("/search", async function(req, res) {
  const term = req.query.term;
  return prisma.$queryRawUnsafe(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "Prisma $queryRawUnsafe should fire for tainted raw SQL"
    );
}

#[test]
fn test_express_mixed_template_sql_with_bind_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  const id = 1;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}' AND id = $1`, { bind: [id] });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpolated template SQL must not be suppressed just because another placeholder is bound"
    );
}

#[test]
fn test_express_json_parse_does_not_fire_deserialization() {
    let source = r#"import express from "express";

const app = express();

app.post("/json", (req, res) => {
  const payload = req.body.payload;
  const parsed = JSON.parse(payload);
  res.json(parsed);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 7),
        "JSON.parse is not a CWE-502 sink"
    );
}

#[test]
fn test_express_yaml_unsafe_schema_name_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "UNSAFE_SCHEMA must not be treated as SAFE_SCHEMA by substring matching"
    );
}

#[test]
fn test_express_destructured_yaml_load_still_fires() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "destructured js-yaml load imports should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_destructured_yaml_load_safe_schema_suppresses() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bare js-yaml load imports should honor imported safe schema constants"
    );
}

#[test]
fn test_express_same_line_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => { const payload = req.body.payload; const JSON_SCHEMA = req.body.schema; return load(payload, { schema: JSON_SCHEMA }); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "same-line local schema bindings must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_block_var_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  if (req.body.useSchema) {
    var JSON_SCHEMA = req.body.schema;
  }
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "block-contained var schema bindings are function-scoped and must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_later_var_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const parsed = load(payload, { schema: JSON_SCHEMA });
  var JSON_SCHEMA = req.body.schema;
  return parsed;
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "later var schema declarations are hoisted and must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_uninitialized_yaml_schema_shadow_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  let JSON_SCHEMA;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "uninitialized local declarations must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_nested_imported_yaml_schema_does_not_suppress_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const JSON_SCHEMA = {};
const app = express();

function helper() {
  const { JSON_SCHEMA } = require("js-yaml");
  return JSON_SCHEMA;
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper js-yaml schema imports must not make outer JSON_SCHEMA references trusted"
    );
}

#[test]
fn test_express_commonjs_aliased_yaml_load_still_fires() {
    let source = r#"import express from "express";
const { load: yamlLoad } = require("js-yaml");

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yamlLoad(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "CommonJS destructuring aliases from js-yaml should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_late_top_level_commonjs_yaml_load_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.post("/yaml", (req, res) => {
  return load(req.body.payload);
});

const { load } = require("js-yaml");
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level CommonJS bindings declared after a route callback are visible when the handler runs"
    );
}

#[test]
fn test_express_commonjs_require_member_yaml_load_still_fires() {
    let source = r#"import express from "express";
const yamlLoad = require("js-yaml").load;

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yamlLoad(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "CommonJS require(\"js-yaml\").load aliases should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_yaml_dump_import_does_not_fire() {
    let source = r#"import express from "express";
import { dump } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return dump(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "js-yaml dump serializes and must not be treated as the bare load sink"
    );
}

#[test]
fn test_express_shadowed_yaml_load_import_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const load = (value) => value;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "local handler bindings should shadow imported js-yaml load"
    );
}

#[test]
fn test_express_nested_block_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  {
    const load = (value) => value;
    load("safe");
  }
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "block-local load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_function_shadowed_yaml_load_import_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function load(value) {
    return value;
  }
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "local function declarations should shadow imported js-yaml load"
    );
}

#[test]
fn test_express_nested_function_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function helper() {
    const load = (value) => value;
    return load("safe");
  }
  helper();
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested-function load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_nested_function_var_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function helper() {
    var load = (value) => value;
    return load("safe");
  }
  helper();
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested-function var load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_nested_imported_yaml_load_does_not_fire_outer_call() {
    let source = r#"import express from "express";

const app = express();

function helper() {
  const { load } = require("js-yaml");
  return load("safe");
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "nested helper js-yaml load imports must not make outer bare load calls sinks"
    );
}

#[test]
fn test_express_unrelated_bare_load_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "./local-loader";

const app = express();

app.post("/load", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bare load should only be a deserialization sink when it resolves to js-yaml"
    );
}

#[test]
fn test_express_yaml_unrelated_safe_schema_token_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA, label: "SAFE_SCHEMA" });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe-schema tokens outside the schema option must not suppress unsafe yaml.load"
    );
}

#[test]
fn test_express_yaml_exact_safe_schema_suppresses() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "exact js-yaml safe schema constants should still suppress unsafe-load findings"
    );
}

#[test]
fn test_express_nested_module_alias_yaml_schema_does_not_suppress_outer_call() {
    let source = r#"import express from "express";

const yaml = { load: (value) => value, JSON_SCHEMA: {} };
const app = express();

function helper() {
  const yaml = require("js-yaml");
  return yaml.JSON_SCHEMA;
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper js-yaml module aliases must not make outer yaml.JSON_SCHEMA references trusted"
    );
}

#[test]
fn test_express_yaml_duplicate_schema_override_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA, schema: yaml.UNSAFE_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "duplicate schema properties can override safe schema constants and must fail closed"
    );
}

#[test]
fn test_express_yaml_spread_schema_override_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const opts = { schema: yaml.UNSAFE_SCHEMA };
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA, ...opts });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "spread schema options can override safe schema constants and must fail closed"
    );
}

#[test]
fn test_express_yaml_ignored_third_safe_schema_arg_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA }, yaml.JSON_SCHEMA);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe-looking ignored args after the options argument must not suppress unsafe yaml.load"
    );
}

#[test]
fn test_express_yaml_request_controlled_schema_holder_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const schemaHolder = { JSON_SCHEMA: req.body.schema };
  return yaml.load(payload, { schema: schemaHolder.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe schema suppression must not trust arbitrary objects ending in JSON_SCHEMA"
    );
}

#[test]
fn test_express_body_reaches_new_function_rce_bucket() {
    let source = r#"import express from "express";

const app = express();

app.post("/compile", (req, res) => {
  const code = req.body.code;
  const fn = new Function(code);
  return fn();
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "new Function is intentionally bucketed with CWE-502/RCE sinks for Phase 3"
    );
}

#[test]
fn test_express_node_serialize_unserialize_fires() {
    let source = r#"import express from "express";
import serialize from "node-serialize";

const app = express();

app.post("/payload", (req, res) => {
  const payload = req.body.payload;
  return serialize.unserialize(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "node-serialize unserialize should fire for Express request body data"
    );
}

#[test]
fn test_koa_request_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Koa ctx.request.body URL should reach fetch SSRF sink"
    );
}

#[test]
fn test_koa_optional_request_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx?.request?.body.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "optional chaining before ctx.request should still expose Koa request data"
    );
}

#[test]
fn test_koa_request_object_alias_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const request = ctx.request;
  const target = request.body.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Koa ctx.request aliases should expose request-object data fields"
    );
}

#[test]
fn test_koa_request_object_alias_shorthand_body_default_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const request = ctx.request;
  const { body = {} } = request;
  return fetch(body.url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "defaulted shorthand destructuring from Koa request aliases should expose body data"
    );
}

#[test]
fn test_koa_destructured_request_object_alias_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const { request } = ctx;
  const target = request.body.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "destructured Koa ctx.request aliases should expose request-object data fields"
    );
}

#[test]
fn test_koa_nested_request_object_destructure_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const { request: { body } } = ctx;
  return fetch(body.url);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Koa nested request destructuring should expose request body as user-controlled"
    );
}

#[test]
fn test_koa_request_object_alias_method_stays_untainted_after_body_read() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const request = ctx.request;
  const target = request.body.url;
  const method = request.method;
  return sequelize.query(`SELECT * FROM methods WHERE name = '${method}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "reading Koa request body through an alias must not taint later server-owned method fields"
    );
}

#[test]
fn test_koa_context_alias_response_body_does_not_taint_request_data() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const context = ctx;
  context.body = "ok";
  return fetch(context.body);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "aliases of the Koa context must not make response body look like request data"
    );
}

#[test]
fn test_koa_response_body_does_not_taint_request_data() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  ctx.body = "ok";
  return fetch(ctx.body);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "Koa ctx.body is response data and must not be treated like ctx.request.body"
    );
}

#[test]
fn test_koa_context_body_same_line_as_query_does_not_taint_response_body() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  console.log(ctx.query.enabled); return fetch(ctx.body);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "Koa ctx.body must not become request data just because ctx.query appears on the same line"
    );
}

#[test]
fn test_koa_import_unregistered_ctx_shape_does_not_taint() {
    let source = r#"import Koa from "koa";

const app = new Koa();

async function helper(ctx, next) {
  const target = ctx.request.body.url;
  return fetch(target);
}
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "Koa import plus ctx-shaped helper must not seed request taint without app.use registration"
    );
}

#[test]
fn test_express_query_reaches_axios_get_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express req.query URL should reach axios.get SSRF sink"
    );
}

#[test]
fn test_express_query_reaches_aliased_axios_get_ssrf() {
    let source = r#"import express from "express";
import httpClient from "axios";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return httpClient.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "aliased axios default imports should be recognized as SSRF receivers"
    );
}

#[test]
fn test_express_query_reaches_imported_axios_get_member_ssrf() {
    let source = r#"import express from "express";
import { get as axiosGet } from "axios";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return axiosGet(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "bare get aliases imported from axios should be recognized as SSRF sinks"
    );
}

#[test]
fn test_express_query_reaches_axios_create_client_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();
const client = axios.create({ timeout: 1000 });

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return client.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "axios.create-bound clients should be recognized as SSRF receivers"
    );
}

#[test]
fn test_express_query_reaches_commonjs_axios_create_client_ssrf() {
    let source = r#"const express = require("express");
const axios = require("axios");

const app = express();
const client = axios.create({ timeout: 1000 });

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return client.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "CommonJS axios.create-bound clients should be recognized as SSRF receivers"
    );
}

#[test]
fn test_express_query_reaches_inline_axios_create_client_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return axios.create({ timeout: 1000 }).post(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "inline axios.create() clients should be recognized as SSRF receivers"
    );
}

#[test]
fn test_local_object_named_axios_does_not_fire_ssrf() {
    let source = r#"import express from "express";

const app = express();
const axios = { get: (key) => key };

app.get("/cache", (req, res) => {
  const target = req.query.key;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "objects named axios should not be SSRF sinks unless they bind the axios module"
    );
}

#[test]
fn test_js_ts_ssrf_unrelated_url_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL("https://example.com");
  if (!allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allowlist check for an unrelated URL must not suppress fetch(target)"
    );
}

#[test]
fn test_js_ts_ssrf_inverted_url_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allow-on-pass guard whose branch returns must not suppress the later fetch"
    );
}

#[test]
fn test_js_ts_ssrf_denylist_guard_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const blockedHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!blockedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "denylist-shaped URL guards must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_disallowed_name_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const disallowedHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!disallowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "negative receiver names containing allow must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_unsafe_name_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const unsafeHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!unsafeHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "negative receiver names containing safe must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_request_derived_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const allowedHosts = new Set(ctx.query.allowedHosts);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "request-derived allowlist collections must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_mutated_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = new Set(["example.com"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  allowedHosts.add(ctx.query.allowedHost);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "allowlist collections mutated with non-literal values must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_alias_mutated_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = new Set(["example.com"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const hosts = allowedHosts;
  hosts.add(ctx.query.allowedHost);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "allowlist collections mutated through aliases must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_object_assign_array_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  Object.assign(allowedHosts, [ctx.query.allowedHost]);
  const parsed = new URL(target);
  if (!allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Object.assign array mutations can replace literal allowlist entries and must fail closed"
    );
}

#[test]
fn test_js_ts_ssrf_nested_allowlist_literal_does_not_suppress_outer_guard() {
    let source = r#"import Koa from "koa";

const app = new Koa();

function helper() {
  const allowedHosts = new Set(["example.com"]);
  return allowedHosts;
}

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper allowlists must not prove trust for an outer SSRF guard"
    );
}

#[test]
fn test_js_ts_ssrf_late_top_level_allowlist_mutation_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});

const allowedHosts = new Set(["example.com"]);
allowedHosts.add(process.env.ALLOWED_HOST);
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level allowlist mutations after handler registration run before requests and must fail closed"
    );
}

#[test]
fn test_express_query_reaches_superagent_get_ssrf() {
    let source = r#"import express from "express";
import superagent from "superagent";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return superagent.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "superagent.get should be covered by the Phase 3 SSRF sink list"
    );
}

#[test]
fn test_unrelated_get_method_does_not_fire_ssrf() {
    let source = r#"import express from "express";

const app = express();
const cache = new Map();

app.get("/cache", (req, res) => {
  const key = req.query.key;
  return cache.get(key);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "non-HTTP .get methods should not be treated as SSRF sinks"
    );
}

#[test]
fn test_fastify_request_body_reaches_child_process_exec() {
    let source = r#"import fastify from "fastify";
import { exec } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return exec(`psql -c ${arg}`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Fastify request.body should reach child_process.exec command sink"
    );
}

#[test]
fn test_fastify_import_unregistered_request_shape_does_not_taint() {
    let source = r#"import fastify from "fastify";
import { exec } from "child_process";

const app = fastify();

async function helper(request, reply) {
  const arg = request.body.shellArg;
  return exec(`psql -c ${arg}`);
}
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "Fastify import plus request/reply-shaped helper must not seed request taint without route registration"
    );
}

#[test]
fn test_exec_file_literal_binary_does_not_flat_leak() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("psql", ["-c", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "literal-binary execFile should suppress the broad flat execFile fallback"
    );
}

#[test]
fn test_exec_file_literal_binary_shell_option_identifier_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: true };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "execFile with identifier-bound shell:true options must not be suppressed as literal-binary safe"
    );
}

#[test]
fn test_exec_file_literal_binary_uninspectable_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const useShell = request.body.useShell;
  const opts = { shell: useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "execFile with non-literal shell option state must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_false_or_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const useShell = request.body.useShell;
  const opts = { shell: false || useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "shell option expressions that start with false are still uninspectable and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_uninspectable_options_node_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = buildOptions(request.body.useShell);
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "identifier options bound to non-object expressions must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_spread_shell_override_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const override = { shell: request.body.useShell };
  const opts = { shell: false, ...override };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "options spreads can override shell:false at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_computed_shell_override_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const key = "shell";
  const opts = { shell: false, [key]: request.body.useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "computed option keys can override shell:false at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_getter_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { get shell() { return request.body.useShell; } };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "accessor option properties can enable shell at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_alias_shell_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: false };
  const alias = opts;
  alias.shell = request.body.useShell;
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "alias mutation can enable shell after an inspectably safe options binding"
    );
}

#[test]
fn test_exec_file_literal_binary_object_assign_shell_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: false };
  Object.assign(opts, { shell: request.body.useShell });
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Object.assign can enable shell after an inspectably safe options binding"
    );
}

#[test]
fn test_exec_file_nested_safe_options_do_not_suppress_outer_call() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

function helper() {
  const opts = { shell: false };
  return opts;
}

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper options bindings must not make outer execFile options inspectably safe"
    );
}

#[test]
fn test_exec_file_late_top_level_options_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.get("/run", async (request, reply) => {
  const arg = request.query.arg;
  return execFile("git", ["status", arg], opts);
});

const opts = { shell: false };
Object.assign(opts, { shell: process.env.SHELL });
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level options mutations after handler registration run before requests and must fail closed"
    );
}

#[test]
fn test_exec_file_shell_wrapper_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("sh", ["-c", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "literal shell-wrapper execFile must not be suppressed as a safe literal-binary form"
    );
}

#[test]
fn test_exec_file_node_args_variable_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const args = ["-e", arg];
  return execFile("node", args);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpreter execFile argv passed through a variable must fail closed"
    );
}

#[test]
fn test_exec_file_node_long_eval_flag_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("node", ["--eval", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpreter execFile long eval flags must not be suppressed as literal-binary safe forms"
    );
}

#[test]
fn test_express_send_file_path_traversal_fires() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  return res.sendFile(path.join("/uploads", filename));
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express req.params filename should reach res.sendFile path traversal sink"
    );
}

#[test]
fn test_express_same_line_handler_reference_reaches_send_file() {
    let source = r#"import express from "express";

const app = express();

app.get("/download/:name", (req, res) => res.sendFile(req.params.name));
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([5]));
    assert!(
        has_taint_sink_on(&result, 5),
        "same-line JS/TS handler param references should remain visible to sink matching"
    );
}

#[test]
fn test_express_request_param_alias_chain_reaches_send_file() {
    let source = r#"import express from "express";

const app = express();

app.get("/download/:name", (req, res) => {
  const file = req.params.name;
  const candidate = file;
  const finalPath = candidate;
  return res.sendFile(finalPath);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "JS/TS target-seed alias synthesis should follow assignment chains beyond one hop"
    );
}

#[test]
fn test_js_ts_path_inverted_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allow-prefix branch that returns must not suppress the later sendFile"
    );
}

#[test]
fn test_js_ts_path_unrelated_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const other = path.resolve(uploadsDir, "static.txt");
  if (!other.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "a prefix check for an unrelated path variable must not suppress sendFile(candidate)"
    );
}

#[test]
fn test_js_ts_path_attacker_controlled_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(filename);
  if (!resolved.startsWith(filename)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "path prefix guards must prove the prefix is trusted, not attacker-controlled"
    );
}

#[test]
fn test_js_ts_path_non_boundary_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "startsWith(base) without a path boundary must not suppress path traversal sinks"
    );
}

#[test]
fn test_js_ts_path_boundary_prefix_guard_suppresses() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads/";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "startsWith(base) may suppress only when the trusted base preserves a path boundary"
    );
}

#[test]
fn test_js_ts_path_nested_prefix_literal_does_not_suppress_outer_guard() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

function helper() {
  const uploadsDir = "/uploads/";
  return uploadsDir;
}

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper path-prefix constants must not prove containment for an outer guard"
    );
}

#[test]
fn test_js_ts_path_root_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "root prefixes prove no containment and must not suppress path traversal sinks"
    );
}

#[test]
fn test_express_query_reaches_fs_promises_read_file() {
    let source = r#"import express from "express";
import { readFile } from "fs/promises";

const app = express();

app.get("/file", async (req, res) => {
  const filename = req.query.file;
  return readFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "destructured fs/promises readFile should fire as a modern Node path traversal sink"
    );
}

#[test]
fn test_nestjs_namespace_yaml_load_direct_dto_field_fires() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";
import * as yaml from "js-yaml";

class ConfigDto {
  yamlContent: string;
}

@Controller("/config")
export class ConfigController {
  @Post("/parse")
  parseConfig(@Body() body: ConfigDto) {
    const config = yaml.load(body.yamlContent);
    return { parsed: typeof config };
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        has_taint_sink(&result),
        "NestJS DTO field access should reach namespace-imported js-yaml yaml.load"
    );
}

#[test]
fn test_nestjs_default_yaml_load_direct_dto_field_fires() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";
import yaml from "js-yaml";

class ConfigDto {
  yamlContent: string;
}

@Controller("/config")
export class ConfigController {
  @Post("/parse")
  parseConfig(@Body() body: ConfigDto) {
    return yaml.load(body.yamlContent);
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        has_taint_sink(&result),
        "NestJS DTO field access should reach default-imported js-yaml yaml.load"
    );
}

#[test]
fn test_nestjs_default_yaml_assigned_config_direct_dto_field_fires() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";
import yaml from "js-yaml";

class ConfigDto {
  yamlContent: string;
}

@Controller("/config")
export class ConfigController {
  @Post("/parse")
  parseConfig(@Body() body: ConfigDto) {
    const config = yaml.load(body.yamlContent);
    return { parsed: typeof config };
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        has_taint_sink(&result),
        "NestJS DTO field access should reach assigned default-imported js-yaml yaml.load"
    );
}

#[test]
fn test_nestjs_destructured_body_field_reaches_yaml_load() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";
import yaml from "js-yaml";

@Controller("/config")
export class ConfigController {
  @Post("/parse")
  parseConfig(@Body() body: ConfigDto) {
    const { yamlContent } = body;
    return yaml.load(yamlContent);
  }
}
"#;
    let result = run_taint_js_ts_single(
        source,
        "config.ts",
        Language::TypeScript,
        BTreeSet::from([1]),
    );
    assert!(
        has_taint_sink_on(&result, 9),
        "NestJS destructured DTO fields should remain taint sources"
    );
}

#[test]
fn test_fastify_commonjs_factory_destructured_exec_promise_fires() {
    let source = r#"const fastify = require("fastify")();
const { exec } = require("child_process");

fastify.post("/api/db/connect", async (request, reply) => {
  const shellArg = request.body.shellArg;
  return new Promise((resolve, reject) => {
    exec(`psql -h localhost -U ${shellArg}`, (err, stdout) => {
      if (err) return reject(err);
      resolve({ output: stdout });
    });
  });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Fastify body taint should reach CommonJS destructured child_process.exec inside Promise"
    );
}

#[test]
fn test_fastify_commonjs_factory_destructured_exec_direct_fires() {
    let source = r#"const fastify = require("fastify")();
const { exec } = require("child_process");

fastify.post("/api/db/connect", async (request, reply) => {
  const shellArg = request.body.shellArg;
  return exec(`psql -h localhost -U ${shellArg}`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Fastify body taint should reach CommonJS destructured child_process.exec"
    );
}
